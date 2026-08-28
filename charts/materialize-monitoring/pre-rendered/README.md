# pre-rendered/

**Generated content. Do not edit by hand.**

Everything under this directory is produced by the build pipeline from
sources under `packages/`:

| Subdirectory                  | Generated from                                         | By                                      |
|-------------------------------|--------------------------------------------------------|-----------------------------------------|
| `dashboards/grafana/`         | `packages/dashboards/` + `packages/queries/`           | `mz-monitoring-build gen-dashboards`    |
| `pipelines/`                  | `packages/alloy-pipelines/`                            | `mz-monitoring-build gen-pipelines`     |
| `scrapers/`                   | `packages/prometheus-scrapers/`                        | `mz-monitoring-build gen-scrape-configs`|
| `metrics/`                    | `packages/queries/`                                    | `mz-monitoring-build gen-metric-tiers`  |

Empty placeholders, kept so the chart's `.Files.Get` globs resolve: `dashboards/datadog/`, `rules/prometheus/`,
`rules/loki/`, `rules/thanos/`. Nothing generates into them yet.

The chart's templates load these files via `{{ .Files.Get }}` because
Helm restricts that directive to the chart directory itself.

If you think you need to edit a file under `pre-rendered/`, you instead
need to edit the corresponding source under `packages/` and regenerate.
CI rejects any manual edit under this directory, even one that happens to
match what regeneration would produce.

To regenerate locally:

```bash
make charts/materialize-monitoring
```
