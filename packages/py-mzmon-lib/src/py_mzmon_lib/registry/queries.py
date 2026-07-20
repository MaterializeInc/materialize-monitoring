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
import typing

import yaml


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
# TODO: other types of templates
TemplateString = str


class QueryDef(typing.TypedDict):
    """YAML definition from mzmon-query.schema.yaml for a query."""

    id: QueryId
    description: str
    stability: Stability
    dependencies: typing.NotRequired[list[_DEPENDENCY_DEF]]
    promQL: typing.NotRequired[TemplateString]
    datadogSQL: typing.NotRequired[TemplateString]
    honeycombSQL: typing.NotRequired[TemplateString]
    logQL: typing.NotRequired[TemplateString]
    instant: typing.NotRequired[bool]


@dataclasses.dataclass(frozen=True)
class Query:
    """A concrete query definition in the registry."""

    id: QueryId
    """Stable identifier for this query."""
    description: str
    """Human-readable description of this query."""
    stability: Stability
    """Stability level of this query."""
    dependencies: list[QueryId] = dataclasses.field(default_factory=list)
    """List of query IDs that this query depends on."""

    promql: TemplateString | None = None
    """PromQL query string for this query."""
    datadog_sql: TemplateString | None = None
    """Datadog SQL query string for this query."""
    honeycomb_sql: TemplateString | None = None
    """Honeycomb SQL query string for this query."""
    logql: TemplateString | None = None
    """LogQL query string for this query."""
    instant: bool | None = None
    """Whether this query is an instant query."""

    def is_metric_query(self) -> bool:
        """Check if this query has a metric definition."""
        return (
            self.promql is not None
            or self.datadog_sql is not None
            or self.honeycomb_sql is not None
        )

    def is_log_query(self) -> bool:
        """Check if this query has a LogQL definition."""
        return self.logql is not None

    def extract_metrics(self, context) -> collections.abc.Iterable:
        """Extract all metrics used from this query."""
        _ = context
        raise NotImplementedError("TODO: implement semantic extraction")


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
        self._queries[query_def["id"]] = Query(
            id=query_def["id"],
            description=query_def["description"],
            stability=query_def["stability"],
            dependencies=deps,
            promql=query_def.get("promQL"),
            datadog_sql=query_def.get("datadogSQL"),
            honeycomb_sql=query_def.get("honeycombSQL"),
            logql=query_def.get("logQL"),
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
    for query in registry.iter_metric_queries():
        print(f"{query.id}: {query.stability}")  # noqa: T201
    for query in registry.iter_log_queries():
        print(f"{query.id}: {query.stability}")  # noqa: T201
