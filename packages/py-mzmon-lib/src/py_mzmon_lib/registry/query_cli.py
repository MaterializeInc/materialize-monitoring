"""Query Registry CLI."""

from __future__ import annotations

import argparse
import collections.abc
import logging
import pathlib
import typing

import yaml

import py_mzmon_lib.registry.queries as query_sdk

LOGGER = logging.getLogger("py_mzmon_lib.registry.query_cli")


def doc_context(
    *,
    engine: query_sdk.QueryEngine = query_sdk.QueryEngine.PROMQL,
    resolve_query: collections.abc.Callable[[query_sdk.QueryId], query_sdk.Query]
    | None = None,
) -> query_sdk.TemplateContext:
    """Build a documentation :class:`TemplateContext` from parameter values."""
    params = {
        "interval": "[51m]",
        "range": "[42m]",
        "mzSqlPrefix": "v2_mz_",
        "mzEnvironmentFilter": 'materialize_cloud_organization_name=~"your-env-name"',
        "mzEnvironmentNamespaceFilter": 'namespace=~"materialize-environment"',
        "mzOperatorNamespaceFilter": 'namespace=~"materialize"',
        "mzClusterList": ".+",
        "mzReplicaList": ".+",
        "mzNamespaceList": "materialize-environment",
        "cAdvisorFilter": 'container!="POD", container!="", namespace=~"materialize-environment"',
    }

    def identity(base: str, *args: str) -> str:
        """Identity function for documentation context."""
        return base

    functions: dict[str, query_sdk.TemplateFn] = {
        "orZero": query_sdk._promql_or_zero,  # noqa: SLF001
        "mzClusterName": identity,
        "mzObjectName": identity,
    }
    return query_sdk.TemplateContext(
        engine=engine,
        parameters=params,
        functions=functions,
        resolve_query=resolve_query,
    )


class MetricDoc(typing.TypedDict):
    """Documentation for a single metric."""

    name: str
    labels: list[str]
    usage: list[str]
    stability: str


def docgen(
    out_dir: pathlib.Path,
    registry: query_sdk.QueryRegistry,
    engine: query_sdk.QueryEngine,
) -> None:
    """Generate documentation for the query registry."""
    context = doc_context(engine=engine, resolve_query=registry.get)

    out_dir.mkdir(parents=True, exist_ok=True)
    metric_path = out_dir / "metrics.yaml"
    LOGGER.info("Generating metric documentation to %s", metric_path)

    metrics: dict[str, MetricDoc] = {}

    for query in registry.iter_metric_queries():
        try:
            for expr in query.render(context):
                LOGGER.debug("Rendered query %s: %s", query.id, expr.strip())
        except (ValueError, TypeError) as err:
            LOGGER.error("Failed to render query %s: %s", query.id, str(err))  # noqa: TRY400
        try:
            for metric in query.extract_metrics(context):
                stability = query.stability
                if metric.name.startswith(("kube_", "container_")):
                    stability = query_sdk.Stability.BEST_EFFORT.value
                if metric.name not in metrics:
                    metrics[metric.name] = {
                        "name": metric.name,
                        "labels": sorted(metric.labels),
                        "usage": [query.id],
                        "stability": stability,
                    }
                else:
                    metric_doc = metrics[metric.name]
                    # Merge labels and stability
                    metric_doc["labels"] = sorted(
                        set(metric_doc["labels"]) | set(metric.labels)
                    )
                    metric_doc["usage"] = sorted(set(metric_doc["usage"]) | {query.id})
                    metric_doc["stability"] = (
                        str(stability)
                        if query_sdk.Stability(stability)
                        > query_sdk.Stability(metric_doc["stability"])
                        else metric_doc["stability"]
                    )
        except ValueError as err:
            LOGGER.error(  # noqa: TRY400
                "Failed to extract metrics for query %s: %s", query.id, str(err)
            )

    def _sort_key(metric: MetricDoc):
        return (
            -query_sdk.STABILITY_MATURITY_ORDER.index(
                query_sdk.Stability(metric["stability"])
            ),
            -len(metric["usage"]),
            metric["name"],
        )

    metric_values = sorted(metrics.values(), key=_sort_key)
    LOGGER.info(
        "Extracted %d metrics from %d queries",
        len(metric_values),
        len(registry),
    )

    with open(metric_path, "w", encoding="utf-8") as handle:
        yaml.safe_dump(metric_values, handle)


def main() -> None:
    """Run the query registry CLI."""
    parser = argparse.ArgumentParser(description="Query Registry CLI")
    parser.add_argument(
        "-v",
        "--verbose",
        action="store_const",
        const="DEBUG",
        dest="log_level",
        help="Enable verbose logging",
    )
    parser.add_argument(
        "--source-dir",
        type=pathlib.Path,
        default=pathlib.Path("packages/queries"),
        help="Directory containing query YAML files",
    )
    parser.add_argument(
        "--engine",
        choices=query_sdk.QueryEngine,
        type=query_sdk.QueryEngine,
        help="Query engine to use",
        default=query_sdk.QueryEngine.PROMQL,
    )
    parser.add_argument(
        "--out-dir",
        type=pathlib.Path,
        required=True,
        help="Output directory for generated documentation",
    )
    parser.add_argument(
        "action",
        choices=["list", "docgen"],
        help="Action to perform on the query registry",
    )
    args = parser.parse_args()
    logging.basicConfig(level=args.log_level or logging.INFO)

    registry = query_sdk.QueryRegistry.from_directory(args.source_dir)
    LOGGER.info(
        "Loaded %d queries from %s",
        len(registry),
        args.source_dir,
    )

    if args.action == "docgen":
        docgen(out_dir=args.out_dir, registry=registry, engine=args.engine)
    else:
        raise RuntimeError(f"Unknown action: {args.action}")


if __name__ == "__main__":
    main()
