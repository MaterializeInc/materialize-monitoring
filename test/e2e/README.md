# E2E on kind

Two bases, both runnable locally with the same targets CI uses.

```bash
make e2e-cluster          # kind cluster + the namespaces a real install has
make e2e-tier1            # chart, hermetic shape
make e2e-generic-cloud    # rustfs + CNPG substrate
make e2e-cluster-down
```

Tier definitions live in the [Terraform modules design doc](../../docs/content/reference/internal/design-docs/20260803-terraform-modules.md#tiers).
The short version: tier 0 is `make terraform-check` (no cluster), tier 1 is the chart's own hermetic shape, tier 2 is the chart against real object storage, tier 3 is real clouds and lives downstream.

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

**Assert on recent data, not on any data.** `verify-tier1.sh` bounds its query to a recent window, and that is the only load-bearing assertion in it. Verified by breaking the write path deliberately: an unbounded query and `loki_ingester_streams_created_total` both still passed, because Loki's filesystem store survives a pod restart and WAL-replayed streams count toward that counter. The `/ready` and label checks are diagnostics — they narrow down *where* a failure is, they do not detect one.

Keep `RECENT_WINDOW_SECONDS` meaningfully smaller than how long a broken stack would have been broken. Too wide and stale-but-in-window chunks satisfy it, which is how the first version of this script passed against a stack that had stopped ingesting.

## CI

`.github/workflows/e2e.yaml`, gated by `e2e-gate` — the check to require in branch protection.

Path filtering is per-job via a `changes` job rather than a trigger-level `paths:` filter, matching `pipelines.yaml`: a workflow skipped by a trigger filter never reports its checks, which leaves a required check pending on every unrelated PR.

Tier 2 is triggered by **chart** changes as well as Terraform ones. A change to Loki's or Thanos's storage wiring is exactly the kind that clears a filesystem-mode tier-1 gate and breaks against real object storage.

Both jobs upload a diagnostics artifact on failure. The cluster dies with the runner, so anything `dump-diagnostics.sh` does not capture is unrecoverable.

**Still to build:** the Rust assertion suite (`packages/mz-monitoring-e2e`), which replaces `verify-tier1.sh`; the tier-2 root composing the substrate with the module; and `thanos-small` plus a kind resource-sizing profile, without which "small on PRs, medium on main" says nothing about Thanos.
