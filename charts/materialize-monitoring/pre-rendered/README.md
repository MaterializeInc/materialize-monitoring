# pre-rendered/

**Generated content. Do not edit by hand.**

Everything under this directory is produced by the build pipeline from
sources under `packages/`:

| Subdirectory                  | Generated from                                         | By                                      |
|-------------------------------|--------------------------------------------------------|-----------------------------------------|
| `dashboards/grafana/`         | `packages/grafana-dashboards/`                         | `mz-monitoring-build` + Grafonnet/SDK   |
| `dashboards/datadog/`         | `packages/datadog-dashboards/`                         | `mz-monitoring-build` + Datadog SDK     |
| `rules/prometheus/`           | `packages/mz-monitoring/rules/`                        | `mz-monitoring-build` rule expander     |
| `rules/loki/`                 | `packages/mz-monitoring/rules/`                        | `mz-monitoring-build` rule expander     |
| `rules/thanos/`               | `packages/mz-monitoring/rules/`                        | `mz-monitoring-build` rule expander     |

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
