"""Catalog-name enrichment for id-keyed metrics via `mz_object_info`.

`mz_object_info` is the canonical object lookup (converged: plain `mz_` in both
self-managed and cloud, value `1`):

    object_id -> name, schema_name, type

Most data-plane metrics carry only an id (`source_id`, `sink_id`, `collection_id`,
`parent_source_id`, …). These helpers attach the friendly `name` (and optionally
`type`) by joining against `mz_object_info`, removing the long-standing
"metrics only have ids; look the name up in SQL" workaround.

The join is a **robust left join** (see `_left_join_labels`): it tolerates both
duplicate info series (two envd generations during a restart, or one endpoint
scraped by several jobs) and missing info (historical ranges before the metric
was scraped, or objects dropped since). A naive `* on(id) group_left(name)`
breaks on the former ("many-to-many matching not allowed") and silently drops
series on the latter.

This module lives in py_mzmon_lib (not the dashboards package) so both the
Grafana dashboards and the query registry's PromQL template functions share one
implementation. The environment (org) scope for the info metric is passed in as
`env_filter` rather than imported, so this module has no dependency on the
dashboards' variables — the dashboards inject their `ENVIRONMENT_FILTER`, and the
registry injects its rendered `%%{mzEnvironmentFilter}` value.
"""

from __future__ import annotations

OBJECT_INFO = "mz_object_info"
"""Canonical id->name catalog metric. Not SQL-prefixed (genuine, both envs)."""


def _left_join_labels(
    value_expr: str, id_label: str, info_expr: str, pulled: list[str]
) -> str:
    """Left-join `pulled` labels from `info_expr` onto `value_expr` on `id_label`.

    `info_expr` is a `1`-valued info metric already `label_replace`d so it
    exposes `id_label` (matching the value side) plus the `pulled` labels.

    Robust to the two failure modes of a naive `* on(id) group_left(name)`:

    1. **Duplicate info series.** Two environmentd generations during a rolling
       restart (briefly two pods), or one `/metrics` endpoint scraped by several
       Prometheus jobs, make the right-hand side non-unique per `id_label` —
       `group_left` then errors with "found duplicate series for the match group
       … many-to-many matching not allowed". We collapse the info metric to one
       series per (`id_label` + `pulled`) with `max by (…)`, which drops the
       `job`/`instance`/`pod` identity labels so the duplicate generations merge.

    2. **Missing info.** Over a historical range before the info metric was
       scraped, or for an object dropped since, the info side has no series for
       that id and an inner join would drop the value series entirely. We union
       those rows back with `… or (value unless on(id) info_keys)` and
       `label_replace` the raw id into the first pulled label so the legend still
       reads something (the id) instead of going blank. The matched set and the
       `unless` set are disjoint on `id_label`, so the union never double-counts.

    Caveat: if two generations genuinely disagree on a pulled label for the same
    id (e.g. an object renamed mid-restart), that id briefly has two info series
    again and re-triggers (1) for that id alone until the old generation ages
    out. And because enrichment lives in a label, a series picks up / loses its
    `name` as info appears/disappears, which Grafana renders as a new series —
    expected, and far better than a broken query or vanished history.
    """
    primary = pulled[0]
    keep = ", ".join([id_label, *pulled])
    pulled_list = ", ".join(pulled)
    # One info series per (id + pulled labels): drops job/instance/pod so
    # concurrent envd generations collapse to a single row.
    dedup = f"max by ({keep}) (\n{info_expr}\n)"
    # One series per id that *has* catalog info — the set we exclude from the
    # fallback so matched rows aren't duplicated.
    keys = f"group by ({id_label}) (\n{info_expr}\n)"
    return (
        f"(\n"
        f"(\n{value_expr}\n)\n"
        f"* on ({id_label}) group_left({pulled_list})\n"
        f"{dedup}\n"
        f")\n"
        f"or\n"
        f"label_replace(\n"
        f"(\n{value_expr}\n)\n"
        f"unless on ({id_label}) (\n{keys}\n),\n"
        f'"{primary}", "$1", "{id_label}", "(.*)"\n'
        f")"
    )


def with_object_name(
    value_expr: str, id_label: str, *, extra: str = "", env_filter: str = ""
) -> str:
    """Attach catalog `name` to `value_expr` via `mz_object_info`.

    `value_expr` must expose `id_label` (e.g. `source_id`, `sink_id`,
    `collection_id`, `parent_source_id`). These are **global** ids, so we join
    `mz_object_info` on its **`global_id`** label (`label_replace`d onto
    `id_label`) and pull `name` in via `group_left`. Pass `extra` (e.g.
    `"type"`) to also bring those labels across.

    **Join on `global_id`, NOT `object_id`.** `object_id` is the catalog item id
    and is *not unique* in `mz_object_info` — a single object (e.g. a
    materialized view) can own several collections, so it appears once per
    `global_id` with the same `object_id`/`name`. Joining on `object_id` then
    yields multiple right-hand matches even in steady state. `global_id` is
    unique and is what the metrics' ids reference. (In a trivial env where every
    object has one collection, `object_id == global_id`, which is why this only
    surfaces against real workloads.)

    Left join (see `_left_join_labels`): series whose id has no catalog entry are
    kept with the raw id as their `name` rather than dropped, and concurrent
    envd generations don't break the join. Legend on `{{name}}`.

    `mz_object_info` must be scoped to the selected environment(s) via
    `env_filter`: ids are only unique within an environment, so an unscoped join
    would match the same id across orgs in multi-tenant cloud. Every caller
    already filters its value side by env and should pass the same filter here.
    """
    pulled = ["name", extra] if extra else ["name"]
    info = (
        f"label_replace({OBJECT_INFO}{{{env_filter}}}, "
        f'"{id_label}", "$1", "global_id", "(.*)")'
    )
    return _left_join_labels(value_expr, id_label, info, pulled)


def with_cluster_name(
    value_expr: str, id_label: str = "instance_id", *, env_filter: str = ""
) -> str:
    """Attach `cluster_name` from `mz_cluster_info` (keyed on `cluster_id`).

    For panels that legend on a cluster id under any label name —
    `instance_id` (most compute metrics) or
    `cluster_environmentd_materialize_cloud_cluster_id` (arrangement / dataflow
    history) — pass that label as `id_label`. The pulled name is renamed to
    `cluster_name` (not `name`) so it won't collide when composed with
    `with_object_name` (e.g. Most-Lagged Collections: cluster + object name).
    Legend on `{{cluster_name}}`. Replica names are intentionally not joined —
    they're all `r1`; keep the more informative `replica_id` in legends.

    Left join (see `_left_join_labels`): tolerant of concurrent envd generations
    and of clusters with no `mz_cluster_info` entry (kept with the raw id as
    `cluster_name`). Scope the info metric to the environment via `env_filter`.
    """
    info = (
        f"label_replace(mz_cluster_info{{{env_filter}}}, "
        f'"{id_label}", "$1", "cluster_id", "(.*)")'
    )
    info = f'label_replace({info}, "cluster_name", "$1", "name", "(.*)")'
    return _left_join_labels(value_expr, id_label, info, ["cluster_name"])
