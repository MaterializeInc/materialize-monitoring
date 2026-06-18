"""Catalog-name enrichment for id-keyed metrics via `mz_object_info`.

`mz_object_info` is the canonical object lookup (converged: plain `mz_` in both
self-managed and cloud, value `1`):

    object_id -> name, schema_name, type

Most data-plane metrics carry only an id (`source_id`, `sink_id`, `collection_id`,
`parent_source_id`, …). These helpers attach the friendly `name` (and optionally
`type`) by joining against `mz_object_info`, removing the long-standing
"metrics only have ids; look the name up in SQL" workaround.
"""

from __future__ import annotations

from dashboards import variables

OBJECT_INFO = "mz_object_info"
"""Canonical id->name catalog metric. Not SQL-prefixed (genuine, both envs)."""


def with_object_name(value_expr: str, id_label: str, *, extra: str = "") -> str:
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
    yields multiple right-hand matches and PromQL errors with
    "found duplicate series for the match group … many-to-many matching not
    allowed". `global_id` is unique and is what the metrics' ids reference. (In a
    trivial env where every object has one collection, `object_id == global_id`,
    which is why this only surfaces against real workloads.)

    Inner join: series whose id has no catalog entry are dropped. Real catalog
    objects are always present in `mz_object_info`; ids like the arrangement
    `none`/transient sentinel are not, so don't enrich those panels (or expect
    them to drop). Legend on `{{name}}`.

    `mz_object_info` is scoped to the selected environment(s): ids are only unique within
    an environment, so an unscoped join would match the same id across orgs in
    multi-tenant cloud and break `group_left`. Every panel using this already
    filters its value side by env.
    """
    pulled = f"name, {extra}" if extra else "name"
    return (
        f"(\n{value_expr}\n)\n"
        f"* on ({id_label}) group_left({pulled})\n"
        f'label_replace({OBJECT_INFO}{{{variables.ENVIRONMENT_FILTER}}}, "{id_label}", "$1", "global_id", "(.*)")'
    )


def with_cluster_name(value_expr: str, id_label: str = "instance_id") -> str:
    """Attach `cluster_name` from `mz_cluster_info` (keyed on `cluster_id`).

    For panels that legend on a cluster id under any label name —
    `instance_id` (most compute metrics) or
    `cluster_environmentd_materialize_cloud_cluster_id` (arrangement / dataflow
    history) — pass that label as `id_label`. The pulled name is renamed to
    `cluster_name` (not `name`) so it won't collide when composed with
    `with_object_name` (e.g. Most-Lagged Collections: cluster + object name).
    Legend on `{{cluster_name}}`. Replica names are intentionally not joined —
    they're all `r1`; keep the more informative `replica_id` in legends.
    """
    info = (
        f"label_replace(mz_cluster_info{{{variables.ENVIRONMENT_FILTER}}}, "
        f'"{id_label}", "$1", "cluster_id", "(.*)")'
    )
    info = f'label_replace({info}, "cluster_name", "$1", "name", "(.*)")'
    return f"(\n{value_expr}\n)\n* on ({id_label}) group_left(cluster_name)\n{info}"
