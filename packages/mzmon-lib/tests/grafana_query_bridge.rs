//! The query bridge against the real registry under `packages/queries/`.
//!
//! The unit tests in `grafana::query` use synthetic queries; these run the whole
//! trip on a real registry entry, so a change to the registry schema, the
//! renderer, or the generated dashboard models shows up here.

use std::path::PathBuf;

use mzmon_lib::grafana::query::{METRICS_DATASOURCE_VAR, panel_query};
use mzmon_lib::query::render::doc_context;
use mzmon_lib::query::{QueryEngine, QueryRegistry};

/// The query id the dashboards' availability panel is built from.
const AVAILABILITY: &str = "materialize.health.environment.availability.percentage";

fn registry() -> QueryRegistry {
    // tests/ -> mzmon-lib -> packages -> repo root
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../packages/queries");
    QueryRegistry::from_directory(&dir).expect("load query registry")
}

#[test]
fn availability_query_becomes_a_prometheus_query_group() {
    let registry = registry();
    let ctx = doc_context(&registry, QueryEngine::PromQl, "mz_");

    let panel = panel_query(&registry, AVAILABILITY, &ctx).expect("bridge the query");

    assert_eq!(panel.query_group.kind, "QueryGroup");
    assert_eq!(
        panel.query_group.spec.queries.len(),
        1,
        "availability is a single-expression query"
    );

    let data = &panel.query_group.spec.queries[0].spec.query;
    assert_eq!(data.group, "prometheus");
    assert_eq!(data.version, "v0");
    assert_eq!(
        data.datasource.as_ref().and_then(|d| d.name.as_deref()),
        Some(format!("${{{METRICS_DATASOURCE_VAR}}}").as_str())
    );

    let spec = data.spec.as_ref().expect("dataquery spec");
    let expr = spec["expr"].as_str().expect("expr is a string");
    // The parameters the doc context supplies must all have been substituted.
    assert!(
        !expr.contains("%%{"),
        "unsubstituted template parameter in:\n{expr}"
    );
    assert!(
        expr.contains("mz_compute_cluster_status"),
        "expected the prefixed metric in:\n{expr}"
    );
    // Grafana's Prometheus datasource reads these, not `expr`/`queryType`.
    assert_eq!(spec["query"], spec["expr"]);
    assert_eq!(spec["qryType"], 1);
}

#[test]
fn availability_description_carries_the_registry_prose() {
    let registry = registry();
    let ctx = doc_context(&registry, QueryEngine::PromQl, "mz_");

    let panel = panel_query(&registry, AVAILABILITY, &ctx).expect("bridge the query");
    let description = &panel.description;

    // Summary comes first, in bold, matching how the hand-written panels open.
    assert!(
        description.starts_with("**An SLO-style snapshot:"),
        "description did not open with the bold summary:\n{description}"
    );
    // Structured fields become labeled paragraphs.
    assert!(description.contains("**Nominal:** At or near full availability."));
    assert!(description.contains("**Degraded:** Availability slipped below target"));
    // This query defines no `unhealthy`, so no such paragraph should appear.
    assert!(!description.contains("**Unhealthy:**"));
    // Notes land unlabeled at the end.
    assert!(description.contains("Four-nines is about the ceiling"));
    // Registry prose is hard-wrapped in YAML; the panel gets unwrapped paragraphs.
    assert!(
        !description.contains("how much of the selected window the\n"),
        "description kept the YAML hard wrapping:\n{description}"
    );
}

/// Every metric query in the registry should bridge cleanly. This is the sweep
/// that catches a query whose templates the bridge cannot render, rather than
/// waiting for whichever dashboard happens to use it.
#[test]
fn every_metric_query_bridges() {
    let registry = registry();
    let ctx = doc_context(&registry, QueryEngine::PromQl, "mz_");

    let mut failures = Vec::new();
    let mut bridged = 0usize;
    for query in registry.iter_metric_queries() {
        // Datadog/Honeycomb-only queries have no PromQL to render; the bridge is
        // right to refuse them, so they are not failures here.
        if query.promql.is_empty() {
            continue;
        }
        match panel_query(&registry, &query.id, &ctx) {
            Ok(_) => bridged += 1,
            Err(e) => failures.push(format!("{}: {e}", query.id)),
        }
    }

    assert!(bridged > 0, "registry produced no PromQL queries to bridge");
    assert!(
        failures.is_empty(),
        "{} of {} PromQL queries failed to bridge:\n  {}",
        failures.len(),
        failures.len() + bridged,
        failures.join("\n  ")
    );
}

/// Log queries bridge to Loki. The dashboards do not use these yet, so this is
/// the only coverage the LogQL path has against real registry content.
#[test]
fn log_queries_bridge_to_loki() {
    let registry = registry();
    let ctx = doc_context(&registry, QueryEngine::LogQl, "mz_");

    let mut bridged = 0usize;
    let mut failures = Vec::new();
    for query in registry.iter_log_queries(false) {
        if query.logql.is_empty() {
            continue;
        }
        match panel_query(&registry, &query.id, &ctx) {
            Ok(panel) => {
                for panel_query in &panel.query_group.spec.queries {
                    assert_eq!(
                        panel_query.spec.query.group, "loki",
                        "{} did not target loki",
                        query.id
                    );
                }
                bridged += 1;
            }
            Err(e) => failures.push(format!("{}: {e}", query.id)),
        }
    }

    assert!(
        failures.is_empty(),
        "{} log queries failed to bridge:\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
    // Not an assertion on the count -- the registry may legitimately have none
    // yet -- but the LogQL path is untested if it does.
    if bridged == 0 {
        eprintln!("note: registry has no LogQL queries; the Loki path is unexercised");
    }
}
