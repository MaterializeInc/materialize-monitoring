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

OBJECT_INFO = "mz_object_info"
"""Canonical id->name catalog metric. Not SQL-prefixed (genuine, both envs)."""


def with_object_name(value_expr: str, id_label: str, *, extra: str = "") -> str:
    """Attach catalog `name` to `value_expr` via `mz_object_info`.

    `value_expr` must expose `id_label` (e.g. `source_id`, `sink_id`,
    `collection_id`, `parent_source_id`). `mz_object_info` is keyed on
    `object_id`, so we `label_replace` it onto `id_label` and pull `name` in via
    `group_left`. Pass `extra` (e.g. `"type"`) to also bring those labels across.

    Inner join: series whose id has no catalog entry are dropped. Real catalog
    objects are always present in `mz_object_info`; ids like the arrangement
    `none`/transient sentinel are not, so don't enrich those panels (or expect
    them to drop). Legend on `{{name}}`.

    `mz_object_info` is scoped by `$environmentFilter`: object ids (`u4`, …) are
    only unique within an environment, so an unscoped join would match the same
    id across orgs in multi-tenant cloud and break `group_left` (many matches on
    the right). Every panel using this already filters its value side by env.
    """
    pulled = f"name, {extra}" if extra else "name"
    return (
        f"(\n{value_expr}\n)\n"
        f"* on ({id_label}) group_left({pulled})\n"
        f'label_replace({OBJECT_INFO}{{$environmentFilter}}, "{id_label}", "$1", "object_id", "(.*)")'
    )
