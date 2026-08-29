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

/// Every registry query must render through the dashboard context.
///
/// Note what is *not* checked: dashboard-flavored PromQL is deliberately not
/// parseable — `[$__rate_interval]` is not a duration — which is exactly why the
/// extraction contexts substitute `[42m]` sentinels instead. So this asserts the
/// three things that are checkable and that actually break dashboards.
#[test]
fn every_query_renders_through_the_dashboard_context() {
    use mzmon_lib::grafana::context::{
        DashboardScope, GENERATION_VARIABLES, NODE_VARIABLES, OPERATOR_VARIABLES,
        REQUIRED_VARIABLES, dashboard_context,
    };

    let registry = registry();
    let scope = DashboardScope::default();
    let ctx = dashboard_context(&registry, QueryEngine::PromQl, &scope);

    // Grafana's own built-ins, plus the variables a dashboard must define --
    // including the optional sets, which only some dashboards provide. A query
    // naming one of those is asserting that its dashboard defines it; that
    // pairing is checked on the dashboard side, in `mz-dashboards`.
    let builtins = ["__rate_interval", "__range", "__interval", "__auto"];

    let mut failures = Vec::new();
    let mut rendered = 0usize;

    for query in registry.iter_metric_queries() {
        if query.promql.is_empty() {
            continue;
        }
        let exprs = match query.render(&ctx) {
            Ok(exprs) => exprs,
            Err(e) => {
                failures.push(format!("{}: render failed: {e}", query.id));
                continue;
            }
        };
        for expr in exprs {
            rendered += 1;
            // 1. Nothing left unsubstituted.
            if expr.contains("%%{") {
                failures.push(format!("{}: unsubstituted parameter", query.id));
            }
            // 2. Every `$name` resolves to something the dashboard provides.
            //    An undefined variable interpolates to nothing and the selector
            //    matches no series -- a panel that looks fine and is empty.
            for reference in dollar_references(&expr) {
                if !builtins.contains(&reference.as_str())
                    && !REQUIRED_VARIABLES.contains(&reference.as_str())
                    && !NODE_VARIABLES.contains(&reference.as_str())
                    && !OPERATOR_VARIABLES.contains(&reference.as_str())
                    && !GENERATION_VARIABLES.contains(&reference.as_str())
                {
                    failures.push(format!("{}: references unknown ${reference}", query.id));
                }
            }
        }
    }

    assert!(rendered > 0, "no PromQL queries rendered");
    assert!(
        failures.is_empty(),
        "{} failure(s) across {rendered} rendered expression(s):\n  {}",
        failures.len(),
        failures.join("\n  ")
    );
}

/// Queries that ask for catalog enrichment must actually get the join.
#[test]
fn enrichment_reaches_the_rendered_expression() {
    use mzmon_lib::grafana::context::{DashboardScope, dashboard_context};

    let registry = registry();
    let ctx = dashboard_context(&registry, QueryEngine::PromQl, &DashboardScope::default());

    // A query the registry annotates with both enrichment functions.
    let id = "materialize.compute.hydration.slowest_collections";
    let query = registry.get(id).expect("query should exist");
    let exprs = query.render(&ctx).expect("render");
    let expr = exprs.first().expect("one expression");

    assert!(expr.contains("mz_object_info"), "no object join:\n{expr}");
    assert!(expr.contains("mz_cluster_info"), "no cluster join:\n{expr}");
    assert!(expr.contains("group_left(name)"), "no name pull:\n{expr}");
    assert!(
        expr.contains("group_left(cluster_name)"),
        "no cluster_name pull:\n{expr}"
    );

    // The same query through the doc context gets neither join, which is what
    // makes the two contexts worth having separately.
    let doc = mzmon_lib::query::render::doc_context(&registry, QueryEngine::PromQl, "mz_");
    let plain = query.render(&doc).expect("render").remove(0);
    assert!(
        !plain.contains("mz_object_info"),
        "doc context should not join"
    );
}

/// Every `$name` in a string.
fn dollar_references(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = value;
    while let Some(pos) = rest.find('$') {
        let after = &rest[pos + 1..];
        // `${name}` is as valid as `$name`; without stripping the brace the length
        // below is zero and an unknown braced variable slips through. Matches the
        // production validator in `grafana::dashboard`.
        let after = after.strip_prefix('{').unwrap_or(after);
        let len = after
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        // `$1` and friends are `label_replace` capture-group references --
        // PromQL syntax, not Grafana variables. The enrichment joins emit them,
        // and so do queries that do their own label_replace.
        if len > 0 && !after[..len].bytes().all(|b| b.is_ascii_digit()) {
            out.push(after[..len].to_string());
        }
        rest = &after[len..];
    }
    out
}
