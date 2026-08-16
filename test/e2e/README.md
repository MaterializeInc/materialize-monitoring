# E2E on kind

Two bases, both runnable locally with the same targets CI uses.

```bash
make e2e-cluster          # kind cluster + the namespaces a real install has
make e2e-tier1            # chart, hermetic shape
make e2e-verify-tier1     # assert the logging round trip
make e2e-tier1-down       # remove tier 1 so the same cluster can host tier 2
make e2e-tier2            # substrate + the module composed onto it
make e2e-verify-tier2     # same assertions, with Thanos live
make e2e-cluster-down
```

**The tiers collide rather than coexist.** Both name their CRDs release `mzmon-crds` in `monitoring`, and Helm refuses a name already in use, so a tier-2 apply against a tier-1 cluster fails on the first release it creates. `make e2e-tier1-down` is the switch; recreating the cluster also works and takes minutes longer.

Every one of these names its target cluster explicitly (`KIND_CONTEXT`, default `kind-mzmon-e2e`) rather than inheriting the current kubeconfig context.
These targets install, restart, and delete things; without that, `make e2e-tier1` would run against whatever cluster you last used — a production one, if that is what it was.
An unknown context is an immediate error rather than a fallback, and the Terraform substrate pins its providers the same way.

Override `KIND_CONTEXT` to point at a different cluster, or `KUBE_CONTEXT` when running the scripts directly.

Tier definitions live in the [Terraform modules design doc](../../docs/content/reference/internal/design-docs/20260803-terraform-modules.md#tiers).
The short version: tier 0 is `make terraform-check` (no cluster), tier 1 is the chart's own hermetic shape, tier 2 is the chart against real object storage, tier 3 is real clouds and lives downstream.

## The assertion suite

Assertions live in [`packages/mz-monitoring-e2e`](../../packages/mz-monitoring-e2e), a Rust workspace member.
There is one binary for every tier, and no tier flag: it takes a context, a namespace and a release name, reads that release's own coalesced Helm values, and runs the assertions those values imply.
The tier is a property of the cluster, not of the invocation, which is what lets the same binary gate a kind job and answer questions about a live cluster.

```bash
make e2e-verify-tier1                                    # the tier-1 kind cluster
make e2e-verify E2E_CONTEXT=<ctx> E2E_NAMESPACE=<ns>     # anything else, including tier 3
cargo run -p mz-monitoring-e2e -- --context <ctx> --list  # which assertions apply there
```

**Values are read as intent.** A component the values enable but the cluster does not have is a *failure* — that is the bug the suite exists to catch. Only a component the values genuinely disable is skipped, and a skip is reported as an ignored test rather than as no test at all: a suite whose list silently shrinks looks exactly like one that passed.

Use `--all` if you ever read those values by hand. Plain `helm get values` returns only what the caller supplied, so a default install answers `null` and nothing is inferable from it.

**It reaches Services by port-forward, and there is a second transport it deliberately does not default to.**
`src/forward.rs` speaks HTTP straight over the forwarded stream — no local listener, so no port allocation, no bind race and no teardown, which is where a `kubectl port-forward` actually flakes.
The API server's Service proxy (`/api/v1/namespaces/<ns>/services/<svc>:<port>/proxy/<path>`) is lighter and available with `--transport proxy`, but it is not a general answer, for two measured reasons:

- **It cannot carry `Authorization`.** The API server strips it before proxying — correctly, since it must not hand a caller's cluster credentials to an arbitrary backend. Custom headers pass through untouched, which is why Loki's `X-Scope-OrgID` works over the proxy and Grafana's basic auth returns `Unauthorized` no matter what you send. Grafana port-forwards regardless of the flag.
- **It needs control-plane-to-pod reachability on the target port.** On EKS the control plane sits in an AWS-managed VPC and the node security groups admit only a few ports: a proxied request to Thanos on 9090 times out, while a port-forward to the same pod answers immediately. The proxy is a kind-and-similar optimisation, not the portable choice.

**Path segments and query values need different escaping.** `encode` is aggressive and right for a query value, which the server URL-decodes before reading. A path segment is matched *raw* by the router, so escaping a character that did not need it changes which route matches — `mzmon-loki` written as `mzmon%2Dloki` is a 404 from Grafana even though the two decode to the same string. Use `encode_segment` there.

**Every assertion retries against a deadline** (`--deadline`, 180s) rather than checking once, and the last error is carried into the timeout message.
Ingestion is asynchronous; a bare check here is the classic E2E flake, and a timeout that does not say how the final attempt failed is the least actionable line a CI log can contain.

## Tier 1 — chart base

`loki-test` + `kind-tier1`: SingleBinary Loki on local filesystem, both Alloy roles, Grafana and its operator, kube-state-metrics.
No Thanos — it needs object storage in every shape it supports, so a hermetic run cannot include it.

What this proves that a render cannot: pods start, the agent discovers pods and ships their logs, the gateway relabels and forwards them, Loki ingests and indexes, and the datasource resolves.
The verified round trip on a fresh cluster:

```
loki_ingester_streams_created_total{tenant="loki"} 59
labels: app, component, container, job, k8s_app, k8s_container, k8s_namespace, k8s_pod, level, namespace, service_name
```

Three warnings are expected here and are not failures: SingleBinary deployment mode, NetworkPolicy disabled, and every gateway metric destination disabled.

## Tier 2 — generic-cloud base

Two roots, composed in one direction.

`terraform/test/generic-cloud` provisions what a cloud wrapper provisions — S3-compatible storage with credentials, and Postgres — and stops there.
rustfs stands in for S3 and CNPG for RDS/Cloud SQL. It does not call the monitoring module: the substrate has to be provable on its own.

`terraform/test/tier2` is the composition. It reads the substrate's outputs and installs the module against them — both backends on rustfs, Grafana's state in the Postgres, `sizing = "small"` so it fits a kind node.

It composes through the substrate's **state file** rather than instantiating it as a child module, because the substrate configures its own providers to stay applyable alone, and a child module carrying provider blocks cannot be cleanly removed. Hence two applies; `make e2e-tier2` runs both.

**What this proves that tier 1 cannot:** the object-storage paths. Tier 1 runs Loki on a local filesystem and no Thanos at all, so the entire storage surface — Loki's S3 chunk client, the Thanos objstore config, the static-credential wiring — is unexercised there. It is also the only tier where the suite's five Thanos assertions run rather than report `ignored`.

Credentials go in through `object_storage_access_key_id` / `object_storage_secret_access_key`, the module's first-class static-credential path — not `additional_values`. That is deliberate: a tier-2 root wired through the escape hatch would validate a composition no customer uses, while this exercises exactly the path a deployment without workload identity takes in production.

**What tier 2 cannot cover:** workload identity. rustfs takes static credentials and kind has no OIDC issuer an IAM provider trusts, so IRSA and GKE Workload Identity are only exercised at tier 3 — after we have already tagged. The `workload_identity_available` output states this so a caller cannot miss it.

## Notes for whoever extends this

**`make e2e-cluster` creates `materialize` and `materialize-environment`.** The chart renders scrape targets into them and Helm refuses to install objects into a namespace that does not exist. A real cluster has them from the operator module.

**Alloy needs a restart after any config change.** Its config arrives through `envFrom` ConfigMaps, and environment variables are fixed at container start — so neither Helm nor Alloy's `/-/reload` picks up a change. `e2e-tier1` does the rollout restart explicitly. This is the same gap the Terraform module closes with a values hash; see [Production Best Practices](../../docs/content/operating/production-best-practices.md#collection-alloy).

**Loki's Service names depend on `deploymentMode`.** SingleBinary renders one `loki` Service; the chart's defaults name `loki-query-frontend` (reads) and `loki-distributor` (writes). `loki-test` repoints all of them. The write path is the one that fails silently — the gateway retries DNS forever and no logs arrive, with nothing in Loki's own logs to say why.

**Assert on recent data, not on any data.** `loki::recent_query` bounds its query to a recent window, and it is the only load-bearing assertion in the tier. Verified by breaking the write path deliberately: an unbounded query and `loki_ingester_streams_created_total` both still passed, because Loki's filesystem store survives a pod restart and WAL-replayed streams count toward that counter. The `/ready`, streams and label checks are diagnostics — they narrow down *where* a failure is, they do not detect one.

Keep `--recent-window` meaningfully smaller than how long a broken stack would have been broken. Too wide and stale-but-in-window chunks satisfy it, which is how the first version of this check passed against a stack that had stopped ingesting.

**The support bundle is the highest-yield single request in the suite.** One fetch of `/-/support` carries the rendered config and every component's health, and the health half found a live bug the first time it ran: `loki.source.journal.node_logs` was unhealthy on every cluster, kind and EKS alike, so journal collection had never run ([DEP-230](https://linear.app/materializeinc/issue/DEP-230), internal). Two causes, both now fixed — the distroless Alloy image was missing `libcap.so.2`, which is what failed libsystemd's dlopen, and the agent did not mount `/run/log/journal`, where a volatile journal lives on both `kindest/node` and Bottlerocket.

That is the shape of failure this assertion exists for: the pod was Ready, the DaemonSet was healthy, `helm install` exited zero, and a whole signal was missing. Nothing short of reading component health would have caught it.

`--allow-unhealthy <component-id>` exists for the window between finding such a thing and fixing it. It takes exact IDs, never globs, and every exemption honoured is printed in the run output — so it cannot quietly become the reason a real failure goes unnoticed. There are none today; adding one should come with a ticket and a condition for removing it.

**Loki's egress NetworkPolicy has to name the port object storage is really on, *and* keep 443.** The module derives the store's port from the endpoint and always adds 443, because the same rule is what grants egress to **STS** for workload identity — always 443, wherever the bucket lives. Getting it wrong in either direction breaks something: hardcoded at 443, which is what it was since real S3 is HTTPS, a self-hosted store on any other port is unreachable; narrowed to only the store's port, IRSA breaks instead, with the credential fetch hanging at pod start rather than failing as a config error. It is a superset of the old rule, so no existing deployment loses access. The symptom is worth memorising because nothing in it mentions a policy: Loki's index gateway logs a bare `dial tcp ...: i/o timeout`, every query that needs the index hangs, and the query frontend eventually answers `504 request timed out, decrease the duration of the request` — which reads as "your query is too expensive" and sends you off narrowing selectors. A 10-second window failed exactly like a 2-hour one, which is the tell: cost-related timeouts scale with the range, and this did not. Thanos hides it further by working perfectly, because the chart writes no policy for it.

**Bound every Loki label query by time.** Unbounded, `/loki/api/v1/labels` spans every index period the schema knows about. SingleBinary answers instantly on a small store, so tier 1 is happy; a distributed Loki fans the request across queriers and returns `504: request timed out, decrease the duration of the request`, which Grafana passes through as a `502`. Two assertions shipped with that bug and only tier 2 found it — which is the clearest argument for the tier there is. Bound it wider than the write-path window, though: a label query asks *which labels exist*, not whether data is arriving, so it should not fail on a brief lull.

**Querying a backend *through Grafana* is not redundant with querying it directly.** `loki::gateway_labels` sends `X-Scope-OrgID` itself, so it passes against a stack whose *datasource* never sends it — and the bundled Loki runs `auth_enabled: true`, so that stack has empty Loki panels and a perfectly healthy Loki. `grafana::loki_datasource_query` is the only assertion covering that gap. Verified by pointing the datasource's `httpHeaderName1` at a header Loki ignores: the direct checks stayed green and this one failed with `Authentication to data source failed`, which is how Grafana surfaces `no org id`.

**Expected UIDs come from the operator's own custom resources, never a list in the suite.** A hardcoded list goes stale the moment a dashboard is added, and it goes stale in the direction that passes: the suite keeps asserting the dashboards it knows about and never notices the new one failing to land. An empty declared set is a failure for the same reason — every member of an empty set is present in Grafana.

**An interrupted tier-2 apply leaves two kinds of debris, and neither error says so.** Terraform records a `helm_release` only once it completes, so a cancelled apply leaves the release live in the cluster and absent from state; the next apply fails with `cannot re-use a name that is still in use`. `helm uninstall mzmon` clears it and the apply recreates it. Separately, VS Code's Terraform extension holds an flock on the state file, which surfaces as `Error acquiring the state lock: resource temporarily unavailable` even with no apply running — `terraform force-unlock` reports the state as *not locked*, because the lock is an flock rather than the lock file. Closing the editor or passing `-lock=false` (safe only when no other apply is running) gets past it.

**Breaking the write path to re-verify that check takes one extra step.** `kubectl scale deployment/alloy-gateway --replicas=0` returns immediately, but Alloy flushes its WAL on shutdown, so writes keep landing for as long as the pods take to terminate. Start the clock from `kubectl wait --for=delete pod -l app.kubernetes.io/instance=alloy-gateway`, not from the scale — otherwise the flush lands inside the window and the check passes against a stack you believe you have broken, which is the one result that teaches you the wrong thing.

## CI

`.github/workflows/e2e.yaml`, gated by `e2e-gate` — the check to require in branch protection.

Path filtering is per-job via a `changes` job rather than a trigger-level `paths:` filter, matching `pipelines.yaml`: a workflow skipped by a trigger filter never reports its checks, which leaves a required check pending on every unrelated PR.

Tier 2 is triggered by **chart** changes as well as Terraform ones. A change to Loki's or Thanos's storage wiring is exactly the kind that clears a filesystem-mode tier-1 gate and breaks against real object storage.

Both jobs upload a diagnostics artifact on failure. The cluster dies with the runner, so anything `dump-diagnostics.sh` does not capture is unrecoverable.
The assertion suite runs the same collector itself when `--diagnostics-dir` is set, into the same directory the job uploads — the collector is idempotent, so whichever step fails first, there is exactly one artifact and it is populated.

The tier-1 job installs a Rust toolchain and builds the suite *before* creating the cluster, so a compile error fails in seconds rather than after a ten-minute install.

**Still to build:** WAL durability across a gateway outage; NetworkPolicy and in-cluster mTLS, both of which the design doc assigns to tier 2 and neither of which is asserted yet; and a kind resource-sizing profile, without which "medium on main" has nowhere to run.

`container_*` and `node_*` assertions are now writable — the gateway scrapes `/metrics/cadvisor` on every kubelet, and node-exporter installs behind `tags.node-exporter`. They belong at tier 2, where a metrics backend exists to query them out of; tier 1 discards metrics entirely.
