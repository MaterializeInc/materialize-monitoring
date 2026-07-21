"""Catalog-name enrichment for id-keyed metrics via `mz_object_info`.

The implementation now lives in `py_mzmon_lib.enrich` so the dashboards and the
query registry's PromQL template functions share one robust left join. This
module is a thin shim that binds the dashboards' environment scope
(`variables.ENVIRONMENT_FILTER`) so existing call sites don't change; see
`py_mzmon_lib.enrich` for the full docstrings and the join's failure-mode notes.
"""

from __future__ import annotations

from py_mzmon_lib import enrich as _enrich

from dashboards import variables

OBJECT_INFO = _enrich.OBJECT_INFO
_left_join_labels = _enrich._left_join_labels  # noqa: SLF001


def with_object_name(value_expr: str, id_label: str, *, extra: str = "") -> str:
    """Attach catalog `name` via `mz_object_info`, scoped to the environment.

    See :func:`py_mzmon_lib.enrich.with_object_name`.
    """
    return _enrich.with_object_name(
        value_expr, id_label, extra=extra, env_filter=variables.ENVIRONMENT_FILTER
    )


def with_cluster_name(value_expr: str, id_label: str = "instance_id") -> str:
    """Attach `cluster_name` from `mz_cluster_info`, scoped to the environment.

    See :func:`py_mzmon_lib.enrich.with_cluster_name`.
    """
    return _enrich.with_cluster_name(
        value_expr, id_label, env_filter=variables.ENVIRONMENT_FILTER
    )
