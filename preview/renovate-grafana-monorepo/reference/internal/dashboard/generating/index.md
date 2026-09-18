# Generating and Pushing Dashboards




# Generating and Pushing Dashboards

How dashboard code is structured, how we keep generation deterministic, and how dashboards flow from source code into a live Grafana instance.

## Code structure

Dashboards live in `packages/dashboards` (the `mz-dashboards` crate), one module per backend under `src/`.
`grafana/` is the only backend today; a Datadog or Google Cloud Monitoring backend belongs beside it rather than inside
it, since little is genuinely shared — the queries differ per engine and each SDK has its own panel and layout model.

Within `grafana/`, each dashboard is a module named for its artifact stem:

```text
packages/dashboards/src/grafana/<dashboard_stem>/mod.rs
```

`mod.rs` holds the dashboard shell (title, variables, annotations, tab list) and one module per tab beside it.
Each dashboard also owns:

- `theme.rs` — the per-tab colours, coordinated in one place rather than spread across the tabs.
- `selector.rs` — the PromQL selector fragments the tab modules share.
- `field_override.rs` — field-override helpers.

`grafana/transform.rs` is shared rather than per-dashboard: it builds Grafana transformation JSON and knows nothing
about Materialize, so a second dashboard uses it directly instead of copying it.

`grafana/mod.rs` is the registry of what can be rendered; `grafana/render.rs` serializes.

**Panels write no PromQL and no prose.** Both come from the query registry — see
[SDKs and Schemas](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/internal/dashboard/sdks/#panels-do-not-write-promql).
What stays with a panel is presentation: legend, unit, panel type, thresholds, transformations.

Sharing panels, rows, or tabs between dashboards is fine, but **prefer the code to live in the most appropriate
module** and have others use it directly.
The Currently Hydrating panel is the worked example: one definition in the dashboard's `mod.rs`, placed on two tabs,
with the shade as the only parameter.

## Code quality

Rust tooling, gated by exit code:

- `cargo fmt --all` — formatting.
- `cargo clippy --workspace --all-targets -- -D warnings` — lints, warnings denied.
- `cargo test --workspace` — tests, including the parity suite under `packages/dashboards/tests/`.

`make -B dashboards` re-renders the checked-in artifacts; the `dashboards` workflow runs it and asserts `git status` is
clean, so a change to a panel or the generator cannot merge with stale output.

## Determinism in dashboards

We try to maximize deterministic and idempotent behavior of dashboards.
It is acceptable for a dashboard to be "upgraded" on import into Grafana, but we want to target a minimal diff.

### UID selection and behavior

UIDs should be selected consistently based on the name of the dashboard.
UIDs are not required to be random, but must be unique.
Upgraded dashboards should continue using the same UIDs unless they break workflows.

Even though we have different Grafana targets, we should **not encode the Grafana version in the UID** (since dashboards
may be upgraded across versions).

UIDs must follow the
[strict UID format introduced in Grafana 11.2](https://grafana.com/whats-new/2025-05-05-enforcing-stricter-data-source-uid-format/)
: Latin alphanumeric with dashes and underscores, 40 characters max.
We use the `mz-mon-` prefix for all UIDs.

**Dashboard v2 caveat:** in v2 the UID is *not* part of the dashboard spec — it lives in the surrounding
Kubernetes-style `metadata.name` on the `dashboard.grafana.app/v2` resource.
The `MzDashboard.UID` value (with the `mz-mon-` prefix) is what we want as the canonical resource name, but Grafana will
happily auto-generate a UID at first upload if one isn't supplied.
Once a dashboard exists, **its UID becomes immutable**; the way to "fix" a mismatched UID is to delete the existing
dashboard and re-upload.

### Element key stability

In a v2 dashboard, panels are referenced by string keys in `spec.elements{}` and in `spec.layout.…ElementReference.name`
.
The Rust source uses human-readable keys (e.g. `"pod-cpu-percent"`); Grafana may rewrite them to `"panel-<id>"` form
on some save paths and leave them alone on others.
**Both forms are valid and the round-trip is non-destructive** — do not rely on a specific naming convention when
reading dashboards back.

## Generating dashboards

`mz-monitoring-build gen-dashboards` renders every dashboard in the registry:

```bash
mz-monitoring-build gen-dashboards --output-dir <dir> --format yaml
```

`--list` enumerates what is available, `--dashboard <stem>` renders one, `--format json` emits the docsite shape, and
### Two copies, one review

Every dashboard is written twice: `charts/…/pre-rendered/dashboards/grafana/<stem>.yaml` for the chart, and
`docs/assets/dashboards/grafana/<stem>.json` for the docsite's download.
Same content, two serializations — so a one-line panel change shows up as two diffs and only one of them is worth
reading.

`.gitattributes` marks the docsite copies `linguist-generated=true`, which collapses them in a GitHub pull request and
drops them from the repository's language statistics.
**The chart's copy is the reviewable one**, since it is what a release installs.
The docsite's copies of the rendered scrapers carry the same mark for the same reason; those are byte-identical to the
chart's.

This changes the *view*, not the content.
`git diff` locally is unaffected, so verifying a render still works normally, and the freshness check below still runs
over both trees — collapsing a diff cannot hide a stale file.

`docs/assets/metrics/metrics.yaml` is deliberately left out: it is generated too, but it is the only copy of the
metric-to-usage index rather than a second one, so its diff is the only place a tier change is visible.

`--check` compares against what is on disk without writing (exiting non-zero if they differ).
`make dashboards` wires the two shipped output trees; see
[SDKs and Schemas](/materialize-monitoring/preview/renovate-grafana-monorepo/reference/internal/dashboard/sdks/#rendering) for the determinism guarantees.

## Pushing dashboards to Grafana

The canonical production path is **`gcx dashboards update`**, which handles the wrapping and the API call.
The notes below cover the ad-hoc / verification path when iterating from a Claude Code session against the Grafana MCP.

### Use the v2 API directly

`mcp-grafana` 's built-in `get_dashboard_by_uid` and `update_dashboard` tools convert dashboards to the v1
representation on the way out, which strips queries from v2-only panel/layout features.
For anything that must round-trip a v2 dashboard, hit the v2 resource API via `grafana_api_request`:

```text
GET /apis/dashboard.grafana.app/v2/namespaces/default/dashboards/<uid>
PUT /apis/dashboard.grafana.app/v2/namespaces/default/dashboards/<uid>
```

PATCH is generally unavailable in our deployments (service accounts only receive the `update` verb, not `patch`); use the full PUT.

### PUT body shape

PUTs must wrap the dashboard spec in the Kubernetes-style envelope:

```jsonc
{
  "apiVersion": "dashboard.grafana.app/v2",
  "kind": "Dashboard",
  "metadata": {
    "name": "<uid>",
    "namespace": "default",
    "resourceVersion": "<rv from current GET>",
    "annotations": {
      "grafana.app/folder": "<folder uid from current GET>",
      "grafana.app/message": "<one-line summary of this change>"
    }
  },
  "spec": { /* JSONEncoder output of MyDashboard() */ }
}
```

Gotchas:

- **Folder annotation is required on update.** Without `metadata.annotations["grafana.app/folder"]`, Grafana treats the
  PUT as a move-to-root and returns `403 "not allowed to create resource in the destination folder"`.
  Always fetch the current resource first and carry the folder annotation forward.
- **Always set `grafana.app/message`.** This is the dashboard's version history entry — populate it with a one-line
  summary describing the change in this revision (same role as a git commit message).
- **`resourceVersion` enables optimistic concurrency.** Fetch + PUT, not fire-and-forget; otherwise concurrent saves can clobber each other.

### Service account permissions

Reads work with a Viewer-scoped token, but PUT requires Edit on the destination folder. The clearest error tells you which:

- `"not allowed to update resource in the source folder"` = no edit on the existing folder.
- `"not allowed to create resource in the destination folder"` = missing folder annotation or no edit on the target folder.

