# pre-rendered/

**Generated content. Do not edit by hand.**

Everything under this directory is produced by the build pipeline from
sources under `packages/`:

| Subdirectory                  | Generated from                                         | By                                      |
|-------------------------------|--------------------------------------------------------|-----------------------------------------|
| `pipelines/`                  | `packages/alloy-pipelines/`                            | `mz-monitoring-build gen-pipelines`     |
| `scrapers/`                   | `packages/prometheus-scrapers/`                        | `mz-monitoring-build gen-scrape-configs`|
| `metrics/`                    | `packages/queries/`                                    | `mz-monitoring-build gen-metric-tiers`  |
| `rules/prometheus/`           | `packages/queries/` (the `alerts:` entries)            | `mz-monitoring-build gen-rules`         |
| `rules/loki/`                 | `packages/queries/` (the LogQL `alerts:` entries)      | `mz-monitoring-build gen-rules`         |

`rules/prometheus/` holds one rule file per registry file plus `_index.yaml`, which records each rule's capabilities
and whether it is in the default set. Tokens of the form `__mzmon_*__` in the rule files are replaced by the chart at
install time (`templates/alerts/prometheusrules.yaml`).

`rules/loki/` holds the log-derived rules, one file per registry file, which `templates/alerts/lokirules.yaml` loads.
`rules/thanos/` is an empty placeholder, and nothing generates into it.
The rendered dashboards live in the dashboards charts, under `charts/materialize-monitoring-dashboards*/pre-rendered/`.

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

The rules alone regenerate with `make rules`.
