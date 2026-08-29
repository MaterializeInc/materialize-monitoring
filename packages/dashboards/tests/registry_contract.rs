//! The dashboards' contract with the query registry.
//!
//! There is no checked-in list of the ids a dashboard names — the ids are written
//! at the panels that use them. The guarantee comes from `build()` instead: it
//! collects every lookup that failed and refuses to return a dashboard, so a
//! renamed or deleted registry query is a build failure naming every affected
//! panel rather than a panel that silently renders nothing.
//!
//! These tests pin that mechanism, since it is what makes the inline ids safe.

use std::path::PathBuf;

use mz_dashboards::grafana::env_top;
use mz_dashboards::grafana::queries::Queries;
use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::query::QueryRegistry;

fn registry() -> QueryRegistry {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../queries");
    QueryRegistry::from_directory(&dir).expect("load the query registry")
}

#[test]
fn every_dashboard_builds_against_the_real_registry() {
    // The check that replaces a hand-maintained id list: if any panel names a
    // query that is gone, this fails and the error names them all.
    let registry = registry();
    for prefix in ["mz_", "v2_mz_"] {
        env_top::build(prefix, &registry)
            .unwrap_or_else(|e| panic!("env-top does not build with prefix {prefix:?}: {e}"));
    }
}

#[test]
fn an_unknown_id_is_reported_rather_than_rendered() {
    // The failure mode the inline ids depend on being loud.
    let registry = registry();
    let scope = DashboardScope::default();
    let queries = Queries::new(&registry, &scope);

    let panel = queries.get("materialize.not.a.real.query");
    assert!(
        panel.query_group.spec.queries.is_empty(),
        "a failed lookup must not produce a usable query"
    );
    let failures = queries.failures();
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(
        failures[0].contains("materialize.not.a.real.query"),
        "the failure names the id: {}",
        failures[0]
    );
}

#[test]
fn a_legend_count_mismatch_is_reported() {
    // Legends are positional against the query's `promQL` list, so a mismatch
    // would silently label the wrong series.
    let registry = registry();
    let scope = DashboardScope::default();
    let queries = Queries::new(&registry, &scope);

    // A single-expression query given two legends.
    queries.legended(
        "materialize.health.environment.availability.percentage",
        &["one", "two"],
    );
    let failures = queries.failures();
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains("legend"), "{}", failures[0]);
}
