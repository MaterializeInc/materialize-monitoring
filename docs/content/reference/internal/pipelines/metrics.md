---
title: "Metrics"
weight: 10
---

# Metrics Pipelines

Metrics pipelines are alloy pipelines built from the `prometheus.*` component family.
They are authored the same way as log pipelines — see [Authoring]({{< relref "authoring.md" >}}) for the pipeline model, the strict-attributes policy, and the `raw:` escape hatch.

<!-- This page tracks the metrics-side component coverage. The logs-side conventions (label families, retention) will get their own analogous section here as metrics pipelines land. -->

## Available components

The following `prometheus.*` components are typed and validated (schema: `packages/mzmon-lib/schemas/alloy/prometheus.schema.yaml`, sugar: `packages/mzmon-lib/src/alloy/components/prometheus.rs`):

| Component | Purpose |
|---|---|
| `prometheus.echo` | Debug sink — prints samples to stdout (`format: text \| openmetrics`). |
| `prometheus.scrape` | Scrapes `targets` and forwards; typed `basic_auth` / `tls_config` / `clustering` sub-blocks and `bearer_token` / `bearer_token_file`. |
| `prometheus.operator.podmonitors` | Discovers `PodMonitor` CRs and scrapes them; typed `clustering` / `selector` (match_labels) / `scrape` / `rule` sub-blocks. |
| `prometheus.operator.servicemonitors` | Same for `ServiceMonitor` CRs; adds `kubernetes_role`. |
| `prometheus.relabel` | Rewrites metric labels via shared `rule` blocks; forwards downstream. |
| `prometheus.receive_http` | Serves a remote-write endpoint; typed `http` server sub-block. |
| `prometheus.remote_write` | Delivers metrics to remote-write endpoints; typed `endpoint` (`url` + scalars). |

Metrics receivers (`forward_to`, and the `receiver` exported by echo/relabel/remote_write/receive_http) are the `MetricsReceiver` capsule — the metrics analog of the logs-side `LogsReceiver`.
They render as **bare refs**, e.g. `forward_to = [prometheus.remote_write.default.receiver]`.

## Deferred to the `raw:` escape

Scope follows the "only stable, in-cluster, most-likely-used" guidance.
These are reachable today via a `raw:` block and can be graduated to typed schema as usage demands:

- **remote_write endpoint auth/TLS/tuning** — `basic_auth`, `bearer_token`, `oauth2`, `authorization`, `sigv4`, `azuread`, `tls_config`, `queue_config`, `metadata_config`, `wal`, `write_relabel_config`.
  These nest inside the typed `endpoint` via its `blocks:` list, so adding auth does **not** require rewriting the whole endpoint as `raw:`.
- **operator `client` block** — the Kubernetes API/auth client.
  Not needed in-cluster (alloy uses the pod's service account); reachable via `raw:` if ever required.
- **scrape `oauth2` / `authorization`** — only `basic_auth` and `tls_config` are typed on `prometheus.scrape`.
- **operator `selector` `match_expression`** — only `match_labels` is typed; set-based selectors use a nested `raw:` block.
- **receive_http `tls`** — the server-side TLS block.

> Load-testing note: `alloy validate` does not catch capsule-type mismatches (see [Authoring]({{< relref "authoring.md" >}})).
> The first committed metrics pipeline should be load-tested with a real `alloy run`.
