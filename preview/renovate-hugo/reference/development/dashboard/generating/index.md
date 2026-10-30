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
[SDKs and Schemas](/materialize-monitoring/preview/renovate-hugo/reference/development/dashboard/sdks/#panels-do-not-write-promql).
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
[strict UID format introduced in Grafana 11.2](https://grafana.com/whats-new/2025-05-05-enforcing-stricter-data-source-uid-format/):
Latin alphanumeric with dashes and underscores, 40 characters max.
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

`mz-monitoring-build gen-dashboards` renders every dashboard of one backend.
`--flavor` picks the backend, and defaults to `grafana`:

```bash
mz-monitoring-build gen-dashboards --flavor grafana --output-dir <dir> --format yaml
mz-monitoring-build gen-dashboards --flavor datadog --output-dir <dir>
```

`--list` enumerates what a backend has, `--dashboard <stem>` renders one, and `--folder <name>` renders only the
Grafana dashboards filed in that folder.
For Grafana, `--format yaml` emits a chart's shape and `--format json` the docsite's.
Datadog renders only JSON, and refuses `--format yaml` and `--folder`, because no chart carries its dashboards.

### Two copies, one review

Every Grafana dashboard is written twice: `charts/<chart>/pre-rendered/dashboards/grafana/<stem>.yaml` for the dashboards chart that
carries its folder, and `docs/assets/dashboards/grafana/<stem>.json` for the docsite's download.
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
The Datadog renders under `docs/assets/dashboards/datadog/` are left out for the same reason.
No chart carries a Datadog dashboard, so its docsite file is the only copy, and its diff is the one to review.

`--check` compares against what is on disk without writing (exiting non-zero if they differ).
`make dashboards` wires the shipped output trees, one per dashboards chart plus the docsite's Grafana and Datadog trees; see
[SDKs and Schemas](/materialize-monitoring/preview/renovate-hugo/reference/development/dashboard/sdks/#rendering) for the determinism guarantees.

### Which chart a dashboard ships in

A dashboard's Grafana folder decides its chart.
Each `materialize-monitoring-dashboards*` chart carries one folder's dashboards, and `make dashboards` renders each with
`gen-dashboards --folder`, as the following table shows:

| Folder | `Folder` variant | Chart |
|---|---|---|
| `materialize` | `Folder::Materialize` | `materialize-monitoring-dashboards` |
| `infra` | `Folder::Infra` | `materialize-monitoring-dashboards-infra` |
| `meta-o11y` | `Folder::MetaO11y` | `materialize-monitoring-dashboards-meta` |

So you pick a new dashboard's chart by passing `.folder(...)` to its builder, and moving a dashboard between charts is a
one-line change.
A dashboard left at `Folder::Root` ships in no chart, and `every_dashboard_is_filed_in_a_folder` in
`packages/dashboards/src/grafana/mod.rs` fails on one.
The `DASHBOARDS_CHARTS` and `DASHBOARDS_FOLDER_*` variables in the `Makefile` hold the mapping.

The charts share their templates through `charts/_dashboards-common`, a library chart named `dashboards-common`.
Each declares it as a dependency and carries it as `charts/dashboards-common`, a symlink to that directory, so its own
`templates/` is a few one-line includes.
The README template the charts render their `README.md` from lives there too.
For the Helm behavior around the symlink, see the `chart-development` skill.

## The size ceiling on dashboard delivery

**The dashboards ship in charts of their own, one per folder, and this is why.**

Helm stores each release as a Kubernetes Secret, and a Secret's data may not exceed 1 MiB.
The rendered dashboards are the largest thing this repository produces by an order of magnitude — over a megabyte of
YAML before Helm's own encoding — and a seventh crossed the limit.
`helm upgrade` then failed with `Secret "sh.helm.release.v1.<release>.vN" is invalid: data: Too long`, raised before
anything was applied: safe and loud, and completely blocking.

Measured on a reference install, where the umbrella release had 36,202 bytes of headroom left:

| | one release | after the split |
|---|---|---|
| `materialize-monitoring` | 1,012,374 (97%) | **648,594** (62%) |
| `materialize-monitoring-dashboards` | — | **440,100** (42%) |

It is the same 1 MiB object ceiling the Terraform design doc
[flagged for the ConfigMap sidecar path](/materialize-monitoring/preview/renovate-hugo/reference/development/design-docs/20260803-terraform-modules/); that note
assumed the grafana-operator path had room, which stopped being true.
A `standalone` Grafana provisions dashboards through that sidecar, which is why the dashboards charts refuse that mode
rather than emitting resources nothing can carry.

**What the split did not fix was the trajectory.**
The dashboards release started with roughly 600 KB of headroom, and the rendered dashboards remain the
fastest-growing artifact here.

A dashboard's own size is worth knowing when adding one: `infra-alloy` is 344 KB, `env-top` 335 KB, `infra-nodes`
246 KB, `infra-loki` 167 KB, `env-logs` 37 KB.
Adding `infra-alloy` took the dashboards release from 441,572 to 551,100 bytes, 53% of the ceiling, estimated the way
Helm encodes a release: the manifest and the chart's files, JSON-encoded, gzipped, then base64-encoded.
The three external-dependency dashboards — `env-persist` (122 KB), `env-consensus` (137 KB) and `infra-cloud`
(122 KB) — took it to 683,928 bytes, 65%, measured the same way.
`infra-autoscaling` (153 KB) and `infra-karpenter` (137 KB) took it from 691,288 to 794,244 bytes, 76%.
By the time `infra-thanos` (235 KB) landed, other changes had taken the release to 840,860 bytes, and `infra-thanos` took it to 914,352, 87%, measured the same way.
Other changes then took it to 943,776, and the alerts rows on `infra-loki`, `infra-alloy`, and `env-top` took it to 966,728, 92%.
Filling `infra-net`'s Cloud Networking tab took `infra-net` from 143 KB to 230 KB and the release to 990,664 bytes, 94%, measured the same way.
That left about 58 KB of headroom, less than one more dashboard the size of `infra-thanos` needs.

So the dashboards split again, one chart per Grafana folder
([DEP-392](https://linear.app/materializeinc/issue/DEP-392)).
Estimated the same way, the following table gives each chart's release size after that split:

| Chart | Dashboards | Bytes | Share of the ceiling |
|---|---|---|---|
| `materialize-monitoring-dashboards` | `env-*` | 382,408 | 36% |
| `materialize-monitoring-dashboards-infra` | the platform | 373,500 | 36% |
| `materialize-monitoring-dashboards-meta` | Alloy, Loki, and Thanos | 282,320 | 27% |

A new dashboard lands in the chart for its folder, so watch that chart's size.
A meta-monitoring dashboard adds about 70 KB to `-meta`.
Shrinking the rendered output is the next lever, and the one that would stop the problem recurring is moving dashboards
out of the release payload altogether.
`selected` is what an operator narrows to hold one back.

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

