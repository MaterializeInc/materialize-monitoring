# Overview




# Queries as Code

Monitoring queries are authored once as YAML under `packages/queries/`, validated against a project-owned JSONSchema,
and rendered per backend query language.
A query is written for PromQL first; the other engines are translations of the same definition, carried in the same entry.

The audience for this section is **repo contributors** — the people adding a panel, an alert, or a metric to the registry.
The audience for the *rendered* queries is the operator reading a dashboard, so the descriptions attached to each query
are end-user voice; see [Dashboard Style Guidelines](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/dashboard/style-guidelines/) for that.

## In this section

- **[Datadog Translations](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/queries/datadog/)** — the PromQL→Datadog mapping, the OTLP naming assumptions it
  rests on, and the gaps where Datadog's language cannot express what the PromQL does.

## Where things live

| | |
|---|---|
| Sources | `packages/queries/*.yaml` — one registry per file |
| Schema | `packages/mzmon-lib/schemas/query/mzmon-query.schema.yaml` |
| Loader | `packages/mzmon-lib/src/query/` — model, rendering, metric extraction, metric tiers |

The loader is deliberately lenient; structural strictness is the schema validator's job.
The `check-queries` pre-commit hook runs it on every changed query file:

```bash
bin/mz-monitoring-check check-queries packages/queries/materialize-compute.yaml
```

## What a registry file holds

Each file carries a `description`, a `metricImportanceHint`, and at least one of four content branches — `queries`,
`rules` (recording rules), `alerts`, or `metricOverrides`.
Alerts and recording rules reference a query either by `queryId` or by defining one inline, which is then promoted to a
top-level registry entry at load time.

Every query carries a `stability` level, and that level is a contract: changes to `canonical` and `best-effort` queries
are breaking changes, and both must be deprecated before removal.
`experimental` and `playground` queries can be removed outright.

## The engines

A query's expression fields are the per-engine translations.
`promQL` is the source of truth; the rest are rendered from the same registry entry.

| Field | Engine | Notes |
|---|---|---|
| `promQL` | Prometheus | The canonical form. Metric extraction and metric tiers parse this and only this. |
| `datadogQuery` | Datadog | Metric query syntax, not DDSQL. See [Datadog Translations](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/queries/datadog/). |
| `honeycombSQL` | Honeycomb | Not yet populated. |
| `logQL` | Loki | Log queries rather than metric queries. |

`promQL` and `datadogQuery` accept either one expression or a list of them, where a list means several distinct series on one panel.

## Templating

Expressions are templates, not literals.
`%%{param}` placeholders are filled by a `TemplateContext`, which supplies both the parameter values and the template
*functions* (`orZero`, `mzClusterName`, `mzObjectName`) — so the same registry entry renders differently per engine.
The parameter names are shared across engines; the values are not.
A PromQL context supplies label matchers (`mzEnvironmentFilter` → `materialize_cloud_organization_name=~"…"`), a
Datadog context supplies tag matchers (`materialize_cloud_organization_name:…`).

The permitted parameter names are an enum in the schema, so a typo fails validation rather than rendering an empty string.
Every one of them must be implemented by the template engine.

## Consumers

Adding or changing a query has effects beyond the query itself:

- **Grafana dashboards** take both their PromQL *and* their panel descriptions from the registry, through
  `mz_dashboards::grafana::queries` — see
  [Dashboards → SDKs and Schemas](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/internal/dashboard/sdks/#panels-do-not-write-promql).
  A panel names a query id and gets the pair; nothing about a query is restated in the dashboard, so a change here
  reaches the rendered dashboards on the next `make dashboards`.
- **Metric extraction** (`mz-monitoring-build extract-metrics`) parses the PromQL to derive the metric set, which lands
  in `docs/assets/metrics/metrics.yaml` and backs
  [Reference Metrics](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/stable-metrics/list-metrics/).
- **Metric tiers** (`mz-monitoring-build gen-metric-tiers`) roll each query's `stability` and importance up into the
  per-destination allowlists in `charts/materialize-monitoring/pre-rendered/metrics/metric-tiers.yaml`.
  This is why a new query can change what a deployment ships to a metered backend.
- **The docs** read `packages/queries/` directly: Hugo mounts it at `assets/queries/`, and the `list-queries` shortcode
  renders [Common Queries](/materialize-monitoring/preview/heather-troubleshoot-skill/reference/stable-metrics/common-queries/) from it.
  There is no generated intermediate to refresh.

Both generated outputs declare `packages/queries/*.yaml` as a prerequisite, so `make metrics` rebuilds them after a query change.
Note that `make all` covers only the metric tiers — the `metrics.yaml` docs asset is not in the `synced` chain, so run
`make metrics` when you have changed which metrics a query names.

<!--
Metric extraction is PromQL-only by design — `--engine datadog` parses the
Datadog expressions as PromQL and finds nothing. Metric names are identical
across engines, so extract from the PromQL side.
-->

