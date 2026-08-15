# E2E on kind

Two bases, both runnable locally with the same targets CI uses.

```bash
make e2e-cluster          # kind cluster + the namespaces a real install has
make e2e-tier1            # chart, hermetic shape
make e2e-verify-tier1     # assert the logging round trip
make e2e-generic-cloud    # rustfs + CNPG substrate
make e2e-cluster-down
```

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

**It talks to Services through the API server's proxy subresource**, not through a port-forward — `/api/v1/namespaces/<ns>/services/<svc>:<port>/proxy/<path>`.
Same destination, but with no local listener, no port allocation, no teardown, and no window where the tunnel is up but not yet forwarding.
It also behaves identically on kind, EKS and GKE, so tier 3 needs no separate transport. The cost is one RBAC verb, `services/proxy`.

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

`terraform/test/generic-cloud` provisions what a cloud wrapper provisions — S3-compatible storage with credentials, and Postgres — and stops there.
It does not call the monitoring module. The substrate has to be provable on its own, and a tier-2 root is the composition of the two.

rustfs stands in for S3 and CNPG for RDS/Cloud SQL. Outputs are shaped to line up with the module's `object_storage` object, so composing them is a copy rather than a mapping.

**What tier 2 cannot cover:** workload identity. rustfs takes static credentials and kind has no OIDC issuer an IAM provider trusts, so IRSA and GKE Workload Identity are only exercised at tier 3 — after we have already tagged. The `workload_identity_available` output states this so a caller cannot miss it.

## Notes for whoever extends this

**`make e2e-cluster` creates `materialize` and `materialize-environment`.** The chart renders scrape targets into them and Helm refuses to install objects into a namespace that does not exist. A real cluster has them from the operator module.

**Alloy needs a restart after any config change.** Its config arrives through `envFrom` ConfigMaps, and environment variables are fixed at container start — so neither Helm nor Alloy's `/-/reload` picks up a change. `e2e-tier1` does the rollout restart explicitly. This is the same gap the Terraform module closes with a values hash; see [Production Best Practices](../../docs/content/operating/production-best-practices.md#collection-alloy).

**Loki's Service names depend on `deploymentMode`.** SingleBinary renders one `loki` Service; the chart's defaults name `loki-query-frontend` (reads) and `loki-distributor` (writes). `loki-test` repoints all of them. The write path is the one that fails silently — the gateway retries DNS forever and no logs arrive, with nothing in Loki's own logs to say why.

**Assert on recent data, not on any data.** `loki::recent_query` bounds its query to a recent window, and it is the only load-bearing assertion in the tier. Verified by breaking the write path deliberately: an unbounded query and `loki_ingester_streams_created_total` both still passed, because Loki's filesystem store survives a pod restart and WAL-replayed streams count toward that counter. The `/ready`, streams and label checks are diagnostics — they narrow down *where* a failure is, they do not detect one.

Keep `--recent-window` meaningfully smaller than how long a broken stack would have been broken. Too wide and stale-but-in-window chunks satisfy it, which is how the first version of this check passed against a stack that had stopped ingesting.

**Breaking the write path to re-verify that check takes one extra step.** `kubectl scale deployment/alloy-gateway --replicas=0` returns immediately, but Alloy flushes its WAL on shutdown, so writes keep landing for as long as the pods take to terminate. Start the clock from `kubectl wait --for=delete pod -l app.kubernetes.io/instance=alloy-gateway`, not from the scale — otherwise the flush lands inside the window and the check passes against a stack you believe you have broken, which is the one result that teaches you the wrong thing.

## CI

`.github/workflows/e2e.yaml`, gated by `e2e-gate` — the check to require in branch protection.

Path filtering is per-job via a `changes` job rather than a trigger-level `paths:` filter, matching `pipelines.yaml`: a workflow skipped by a trigger filter never reports its checks, which leaves a required check pending on every unrelated PR.

Tier 2 is triggered by **chart** changes as well as Terraform ones. A change to Loki's or Thanos's storage wiring is exactly the kind that clears a filesystem-mode tier-1 gate and breaks against real object storage.

Both jobs upload a diagnostics artifact on failure. The cluster dies with the runner, so anything `dump-diagnostics.sh` does not capture is unrecoverable.
The assertion suite runs the same collector itself when `--diagnostics-dir` is set, into the same directory the job uploads — the collector is idempotent, so whichever step fails first, there is exactly one artifact and it is populated.

The tier-1 job installs a Rust toolchain and builds the suite *before* creating the cluster, so a compile error fails in seconds rather than after a ten-minute install.

**Still to build:** the rest of the assertions — Grafana (`/api/search`, dashboard UIDs, and queries through the datasource proxy, where the `no org id` failure surfaces), Thanos (`/api/v1/stores` and instant queries for `up` and `scrape_samples_scraped`), and the Alloy gateway support bundle.
Also the tier-2 root composing the substrate with the module, and `thanos-small` plus a kind resource-sizing profile, without which "small on PRs, medium on main" says nothing about Thanos.

`container_*` and `node_*` assertions are not writable until cAdvisor and node-exporter collection land. Worth encoding as tests expected to fail until then, so they convert to coverage the moment collection ships.
