"""Registry of Queries for usage across monitoring subsystems.

Our standard metric format uses PromQL but we also need some support for
DataDog and Honeycomb.
For logs, we use LogQL.

These are used in Grafana dashboards and Google Cloud Monitoring dashboards
and AlertManager rules.

This registry also has a large output in our documentation.
"""

from __future__ import annotations

import collections.abc
import dataclasses
import enum
import functools
import pathlib
import re
import typing

import yaml

from py_mzmon_lib import enrich


@functools.total_ordering
class Stability(enum.StrEnum):
    """Enumeration of query stability levels."""

    UNUSED = "unused"
    """Query is not used anywhere.

    It is an error to use an UNUSED query (please promote to another level).

    Unused queries do not have user facing documentation.
    """
    PLAYGROUND = "playground"
    """Query is used for development and experimentation.

    Playground queries do not have user facing documentation.
    """
    EXPERIMENTAL = "experimental"
    """Query is experimental and may change or be removed without notice.

    Experimental queries have user-facing documentation.
    Experimental queries may change without notice.
    """
    BEST_EFFORT = "best-effort"
    """Query is expected to be fully supported.

    It is a breaking change to modify a best-effort query in an incompatible way.
    """
    CANONICAL = "canonical"
    """Query is fully supported and expected to be stable.

    It is a breaking change to modify a canonical query in an incompatible way.

    Canonical queries require test suite coverage.
    """
    DEPRECATED = "deprecated"
    """Query is deprecated and should not be used.

    Deprecated queries may still be available but are not recommended for use.
    Deprecated queries have user-facing documentation.
    It is a warning to use a deprecated query.
    """
    UNSUPPORTED = "unsupported"
    """Query is unsupported and should not be used.

    Unsupported queries may still be available but are not recommended for use.
    Unsupported queries do not have user-facing documentation.
    It is a warning to use an unsupported query.
    """

    def __lt__(self, other: object) -> bool:
        """Compare the maturity of two stability levels.

        See Also:
            :py:attr:`STABILITY_MATURITY_ORDER`
        """
        if not isinstance(other, Stability):
            return NotImplemented
        return STABILITY_MATURITY_ORDER.index(self) < STABILITY_MATURITY_ORDER.index(
            other
        )


STABILITY_MATURITY_ORDER = [
    Stability.UNUSED,
    Stability.PLAYGROUND,
    Stability.UNSUPPORTED,
    Stability.EXPERIMENTAL,
    Stability.BEST_EFFORT,
    Stability.DEPRECATED,
    Stability.CANONICAL,
]
"""Ordering of stability levels from least to most mature.

This is the normal flow of query development and promotion.

If a dashboard has a minimum stability level, it will use this ordering.
Note that Best-Effort and Deprecated are approximately the same level,
but Deprecated is placed higher so `>= BEST_EFFORT` will include Deprecated queries.
(This avoids implementing a BEST_EFFORT_DEPRECATED and CANONICAL_DEPRECATED separately.)
"""

STABILITY_DEPRECATION_ORDER = [
    Stability.CANONICAL,
    Stability.BEST_EFFORT,
    Stability.DEPRECATED,
    Stability.UNSUPPORTED,
    Stability.UNUSED,
]
"""Ordering of stability levels when being slated for removal.

Best-effort and canonical queries must be deprecated before removal.
Experimental queries can be removed without deprecation.

After a query has been deprecated, it can be removed.
If it is not removed, it can be marked as unsupported.
"""

QueryId = str
_DEPENDENCY_DEF = str | dict[str, "QueryDef"]


class QueryEngine(enum.StrEnum):
    """A backend query language a query can be rendered for.

    This is the *query engine* (Prometheus, Datadog, …), distinct from the
    *template engine* (this library) that renders a query for it.
    """

    PROMQL = "promql"
    DATADOG = "datadog"
    HONEYCOMB = "honeycomb"
    LOGQL = "logql"


# A template-engine transform: `fn(base, *rendered_args) -> str`. Supplied per
# query engine by a `TemplateContext`; NOT a query-engine function.
TemplateFn = collections.abc.Callable[..., str]


@dataclasses.dataclass(frozen=True)
class TemplateFunction:
    """A template-engine transform applied to a rendered template string.

    Names (`orZero`, `mzClusterName`, …) are resolved to implementations by the
    `TemplateContext`, so the same registry entry renders differently per query
    engine. `args` are themselves template strings, rendered before the call.
    """

    name: str
    args: list[TemplateExpr] = dataclasses.field(default_factory=list)

    @classmethod
    def from_entry(cls, entry: str | dict[str, typing.Any]) -> typing.Self:
        """Parse a YAML template string (raw string or object form)."""
        if isinstance(entry, str):
            return cls(name=entry)
        assert isinstance(entry, dict)
        return cls(
            name=entry["name"],
            args=TemplateExpr.from_entry(entry.get("args", [])),
        )


@dataclasses.dataclass(frozen=True)
class TemplateExpr:
    """The object form of a template string.

    Either an inline `template` (with `%%{param}` placeholders) or a reference to
    another query by `query_id`, optionally wrapped by template-engine
    `functions` (applied in order).
    """

    template: str | None = None
    query_id: QueryId | None = None
    functions: list[TemplateFunction] = dataclasses.field(default_factory=list)

    def __post_init__(self) -> None:
        """Enforce exactly one of `template` / `query_id`."""
        if (self.template is None) == (self.query_id is None):
            raise ValueError(
                "TemplateExpr requires exactly one of `template` or `query_id`"
            )

    @classmethod
    def from_entry(
        cls, entry: str | dict[str, typing.Any] | list[typing.Any] | None
    ) -> list[typing.Self]:
        """Parse a YAML template string (raw string or object form) into a consistent structure (list of TemplateExpr)."""
        if entry is None:
            return []
        if isinstance(entry, list):
            exprs = []
            for item in entry:
                exprs.extend(cls.from_entry(item))
            return exprs
        if isinstance(entry, str):
            return [cls(template=entry)]
        assert isinstance(entry, dict)
        return [
            cls(
                template=entry.get("template"),
                query_id=entry.get("queryId"),
                functions=[
                    TemplateFunction.from_entry(fn) for fn in entry.get("functions", [])
                ],
            )
        ]


class QueryDef(typing.TypedDict):
    """YAML definition from mzmon-query.schema.yaml for a query."""

    id: QueryId
    description: dict[str, str]
    stability: Stability
    dependencies: typing.NotRequired[list[_DEPENDENCY_DEF]]
    # Raw (unparsed) template values: str | {template|queryId, functions} | list.
    promQL: typing.NotRequired[typing.Any]
    datadogSQL: typing.NotRequired[typing.Any]
    honeycombSQL: typing.NotRequired[typing.Any]
    logQL: typing.NotRequired[typing.Any]
    instant: typing.NotRequired[bool]


@dataclasses.dataclass(frozen=True)
class Description:
    """Structured description for queries and rules."""

    summary: str
    """A brief summary of the query."""
    nominal: str | None = None
    """The nominal or expected behavior of the query."""
    degraded: str | None = None
    """The degraded behavior of the query and actions to take."""
    unhealthy: str | None = None
    """The unhealthy behavior of the query and actions to take."""
    notes: str | None = None
    """Additional notes about the query."""


@dataclasses.dataclass(frozen=True)
class TemplateContext:
    """Everything needed to render a query for one query engine.

    The template engine builds this per target: `parameters` maps knownParameter
    names to their rendered values (e.g. `interval` -> `[$__rate_interval]` for
    Grafana, `[5m]` for a static Google Cloud Monitoring dashboard), `functions`
    supplies the template-engine transforms (`orZero`, `mzClusterName`, …) for
    this engine, and `resolve_query` looks a query up by id for `queryId`
    references.
    """

    engine: QueryEngine
    parameters: collections.abc.Mapping[str, str] = dataclasses.field(
        default_factory=dict
    )
    functions: collections.abc.Mapping[str, TemplateFn] = dataclasses.field(
        default_factory=dict
    )
    resolve_query: collections.abc.Callable[[QueryId], Query] | None = None


_PLACEHOLDER = re.compile(r"%%\{([A-Za-z0-9_]+)\}")


def _substitute_params(template: str, context: TemplateContext) -> str:
    """Replace every `%%{name}` in `template` with its value from `context`."""

    def replace(match: re.Match[str]) -> str:
        name = match.group(1)
        try:
            return context.parameters[name]
        except KeyError:
            raise KeyError(
                f"template parameter %%{{{name}}} has no value in this "
                f"TemplateContext (known: {sorted(context.parameters)})"
            ) from None

    return _PLACEHOLDER.sub(replace, template)


def _render_template_string(ts: TemplateExpr, context: TemplateContext) -> str:
    """Render a single template string to a concrete query expression."""
    if isinstance(ts, str):
        return _substitute_params(ts, context)

    if ts.template is not None:
        base = _substitute_params(ts.template, context)
    else:
        assert ts.query_id is not None  # guaranteed by TemplateExpr.__post_init__
        if context.resolve_query is None:
            raise ValueError(
                f"template references query {ts.query_id!r} but the "
                f"TemplateContext has no resolve_query"
            )
        referenced = context.resolve_query(ts.query_id).render(context)
        if isinstance(referenced, list):
            raise ValueError(
                f"query {ts.query_id!r} renders multiple expressions and cannot "
                f"be embedded as a single template reference"
            )
        base = referenced

    for fn in ts.functions:
        impl = context.functions.get(fn.name)
        if impl is None:
            raise KeyError(
                f"template function {fn.name!r} is not implemented by this "
                f"TemplateContext (known: {sorted(context.functions)})"
            )
        rendered_args = [_render_template_string(arg, context) for arg in fn.args]
        base = impl(base, *rendered_args)
    return base


@dataclasses.dataclass(frozen=True)
class Query:
    """A concrete query definition in the registry."""

    id: QueryId
    """Stable identifier for this query."""
    description: Description
    """Human-readable description of this query."""
    stability: Stability
    """Stability level of this query."""
    dependencies: list[QueryId] = dataclasses.field(default_factory=list)
    """List of query IDs that this query depends on."""

    promql: list[TemplateExpr] = dataclasses.field(default_factory=list)
    """PromQL template(s) for this query (one, or several distinct series)."""
    datadog_sql: list[TemplateExpr] = dataclasses.field(default_factory=list)
    """Datadog SQL template for this query."""
    honeycomb_sql: list[TemplateExpr] = dataclasses.field(default_factory=list)
    """Honeycomb SQL template for this query."""
    logql: list[TemplateExpr] = dataclasses.field(default_factory=list)
    """LogQL template for this query."""
    instant: bool | None = None
    """Whether this query is an instant query."""

    def is_metric_query(self) -> bool:
        """Check if this query has a metric definition."""
        return any([self.promql, self.datadog_sql, self.honeycomb_sql])

    def is_log_query(self) -> bool:
        """Check if this query has a LogQL definition."""
        return bool(self.logql)

    def _value_for_engine(self, engine: QueryEngine) -> list[TemplateExpr]:
        """Return the (unrendered) template value for `engine`, if any."""
        return {
            QueryEngine.PROMQL: self.promql,
            QueryEngine.DATADOG: self.datadog_sql,
            QueryEngine.HONEYCOMB: self.honeycomb_sql,
            QueryEngine.LOGQL: self.logql,
        }[engine]

    def render(self, context: TemplateContext) -> list[str]:
        """Render this query for the context's query engine.

        Returns a single expression, or a list of expressions when the query
        defines several series (a list-valued PromQL). Raises if the query has
        no expression for the requested engine, if a `%%{param}` is unset, or if
        a referenced function/query is missing from the context.
        """
        value = self._value_for_engine(context.engine)
        if not value:
            raise ValueError(f"query {self.id!r} has no {context.engine} expression")
        return [_render_template_string(ts, context) for ts in value]

    def extract_metrics(self, context) -> collections.abc.Iterable:
        """Extract all metrics used from this query."""
        _ = context
        raise NotImplementedError("TODO: implement semantic extraction")


# ---------------------------------------------------------------------------
# PromQL template-engine functions.
#
# `mzClusterName` / `mzObjectName` delegate to the shared `py_mzmon_lib.enrich`
# left join (the same one the Grafana dashboards use), bound to the context's
# `mzEnvironmentFilter` so the info-metric join is scoped to the environment.
# ---------------------------------------------------------------------------


def _promql_or_zero(base: str) -> str:
    """Harden `base` to yield 0 rather than an empty result."""
    return f"({base}) or vector(0)"


def promql_context(
    parameters: collections.abc.Mapping[str, str],
    *,
    resolve_query: collections.abc.Callable[[QueryId], Query] | None = None,
) -> TemplateContext:
    """Build a PromQL :class:`TemplateContext` from parameter values.

    The id->name enrichment functions are scoped to the environment via the
    `mzEnvironmentFilter` parameter, so they must be built per context.
    """
    params = dict(parameters)
    env_filter = params.get("mzEnvironmentFilter", "")

    def with_cluster_name(base: str, id_label: str = "instance_id") -> str:
        return enrich.with_cluster_name(base, id_label, env_filter=env_filter)

    def with_object_name(base: str, id_label: str, extra: str = "") -> str:
        return enrich.with_object_name(
            base, id_label, extra=extra, env_filter=env_filter
        )

    functions: dict[str, TemplateFn] = {
        "orZero": _promql_or_zero,
        "mzClusterName": with_cluster_name,
        "mzObjectName": with_object_name,
    }
    return TemplateContext(
        engine=QueryEngine.PROMQL,
        parameters=params,
        functions=functions,
        resolve_query=resolve_query,
    )


class QueryRegistry:
    """Registry of queries across monitoring subsystems."""

    class _RegistryDef(typing.TypedDict):
        description: str
        queries: list[QueryDef]

    def __init__(self) -> None:
        """Initialize the query registry."""
        self._queries: dict[QueryId, Query] = {}

    def get(self, query_id: QueryId) -> Query:
        """Get the query definition for a given query ID."""
        return self._queries[query_id]

    def __getitem__(self, query_id: QueryId) -> Query:
        """Get the query definition for a given query ID."""
        return self.get(query_id)

    def iter_metric_queries(self) -> collections.abc.Iterator[Query]:
        """Iterate over all queries that have a metric definition."""
        for query in self._queries.values():
            if query.is_metric_query():
                yield query

    def iter_log_queries(
        self, *, exclude_metric_queries: bool = False
    ) -> collections.abc.Iterator[Query]:
        """Iterate over all queries that have a LogQL definition."""
        for query in self._queries.values():
            if query.is_log_query() and (
                not exclude_metric_queries or not query.is_metric_query()
            ):
                yield query

    def load(self, registry_data: _RegistryDef):
        """Load query definitions from a list of QueryDef dictionaries."""
        for query_def in registry_data["queries"]:
            self.register_query(query_def)

    @classmethod
    def from_directory(
        cls, directory: pathlib.Path, pattern: str = "*.yaml"
    ) -> typing.Self:
        """Load query definitions from a directory of YAML files."""
        registry = cls()
        for file_path in sorted(directory.glob(pattern)):
            with open(file_path, encoding="utf-8") as handle:
                registry_data = yaml.safe_load(handle)
                registry.load(registry_data)
        return registry

    def register_query(self, query_def: QueryDef | Query) -> Query:
        """Register a new query definition."""
        if isinstance(query_def, Query):
            if query_def.id in self._queries:
                raise ValueError(f"Query ID {query_def.id} is already registered.")
            self._queries[query_def.id] = query_def
            return query_def
        if query_def["id"] in self._queries:
            raise ValueError(f"Query ID {query_def['id']} is already registered.")
        deps = []
        for dependency in query_def.get("dependencies", []):
            if isinstance(dependency, str):
                deps.append(dependency)
            elif isinstance(dependency, dict):
                # Unfortunately, can't do an isinstance check
                dependency = typing.cast("QueryDef", dependency)
                dep_query = self.register_query(dependency)
                deps.append(dep_query.id)
            else:
                raise TypeError(
                    f"Dependency definition {dependency} must be a string or a dict."
                )
        description = Description(**query_def["description"])

        self._queries[query_def["id"]] = Query(
            id=query_def["id"],
            description=description,
            stability=query_def["stability"],
            dependencies=deps,
            promql=TemplateExpr.from_entry(query_def.get("promQL")),
            datadog_sql=TemplateExpr.from_entry(query_def.get("datadogSQL")),
            honeycomb_sql=TemplateExpr.from_entry(query_def.get("honeycombSQL")),
            logql=TemplateExpr.from_entry(query_def.get("logQL")),
            instant=query_def.get("instant"),
        )
        return self._queries[query_def["id"]]

    def overload_query(self, query_def: Query) -> None:
        """Overwrite an existing query definition."""
        self._queries[query_def.id] = query_def

    def register_rule(self, rule_def):
        """Load a rule definition."""
        _ = rule_def
        raise NotImplementedError("TODO")


if __name__ == "__main__":
    registry = QueryRegistry.from_directory(pathlib.Path("packages/queries"))

    # A Grafana-flavored PromQL context: parameters resolve to Grafana's
    # built-ins / dashboard variables. A Google Cloud Monitoring context would
    # map `interval` -> `[5m]`, `mzClusterList` -> `.*`, etc.
    ctx = promql_context(
        {
            "interval": "[$__rate_interval]",
            "range": "[$__range]",
            "mzSqlPrefix": "mz_",
            "mzEnvironmentFilter": 'materialize_cloud_organization_name=~"$environmentIdList"',
            "mzEnvironmentNamespaceFilter": 'namespace=~"$mzNamespaceList"',
            "mzOperatorNamespaceFilter": 'namespace=~"$mzOperatorNamespaceList"',
            "mzClusterList": "$mzClusterList",
            "mzReplicaList": "$mzReplicaList",
        },
        resolve_query=registry.get,
    )

    for query in registry.iter_metric_queries():
        print(f"{query.id}: {query.stability}")  # noqa: T201
        print(f"   {query.description.summary}")  # noqa: T201
        for expr in query.render(ctx):
            print("   ---")  # noqa: T201
            for line in expr.strip().splitlines():
                print(f"   {line}")  # noqa: T201
