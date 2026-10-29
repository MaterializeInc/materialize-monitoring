//! Parity ledger between each Datadog dashboard and its Grafana counterpart.
//!
//! Both backends name each query by its registry id, so the registry keeps the
//! two sets meaning the same thing. What can still drift is everything around
//! the queries: which panels exist, their titles, which queries each reads, and
//! what kind of panel each is. This compares the two dashboards' manifests,
//! panel by panel, by slug.
//!
//! A difference has to be listed, with its reason, in the Datadog dashboard's
//! `DIFFERENCES`. A listing that no longer matches a difference fails too, so
//! the list can't rot into a stale excuse. Keep it short: a long list means the
//! ledger has stopped describing the two sets.

use std::path::PathBuf;

use mz_dashboards::manifest::{Difference, Gap, Panel};
use mz_dashboards::{datadog, grafana};
use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::query::QueryRegistry;

fn registry() -> QueryRegistry {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../queries");
    QueryRegistry::from_directory(&dir).expect("load the query registry")
}

/// Every unlisted difference between the two manifests, and every listing that
/// matches none.
fn compare(grafana: &[Panel], datadog: &[Panel], listed: &[Difference]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut used = vec![false; listed.len()];
    let mut allow =
        |slug: &str, gap: Gap| match listed.iter().position(|d| d.slug == slug && d.gap == gap) {
            Some(index) => {
                used[index] = true;
                true
            }
            None => false,
        };

    for g in grafana {
        let Some(d) = datadog.iter().find(|d| d.slug == g.slug) else {
            if !allow(&g.slug, Gap::Missing) {
                problems.push(format!(
                    "{}: no Datadog counterpart, and none listed",
                    g.slug
                ));
            }
            continue;
        };
        if d.title != g.title {
            problems.push(format!(
                "{}: title {:?} != Grafana's {:?}",
                g.slug, d.title, g.title
            ));
        }
        if d.queries != g.queries {
            problems.push(format!(
                "{}: reads {:?} where Grafana reads {:?}",
                g.slug, d.queries, g.queries
            ));
        }
        if d.kind != g.kind && !allow(&g.slug, Gap::Kind) {
            problems.push(format!(
                "{}: kind {:?} != Grafana's {:?}, and not listed",
                g.slug, d.kind, g.kind
            ));
        }
        if d.unit != g.unit && !allow(&g.slug, Gap::Unit) {
            problems.push(format!(
                "{}: unit {:?} != Grafana's {:?}, and not listed",
                g.slug, d.unit, g.unit
            ));
        }
    }
    for d in datadog {
        if !grafana.iter().any(|g| g.slug == d.slug) {
            problems.push(format!("{}: no Grafana panel by that slug", d.slug));
        }
    }
    for (difference, used) in listed.iter().zip(used) {
        if !used {
            problems.push(format!(
                "{} {:?}: listed, but the two dashboards don't differ that way",
                difference.slug, difference.gap
            ));
        }
    }
    problems
}

#[test]
fn every_datadog_dashboard_matches_its_grafana_counterpart() {
    let registry = registry();
    let grafana_options = grafana::Options::default();
    let datadog_options = datadog::Options::default();
    for dashboard in datadog::ALL {
        let twin = grafana::find(dashboard.name).expect("a Grafana counterpart");
        let rendered = (twin.render)(&grafana_options, &registry).expect("render Grafana");
        let scope = DashboardScope::for_prefix(&grafana_options.sql_metric_prefix);
        let theirs = grafana::manifest::manifest(&rendered.spec, &registry, &scope);
        let ours = (dashboard.manifest)(&datadog_options, &registry).expect("build Datadog");
        let problems = compare(&theirs, &ours, dashboard.differences);
        assert!(
            problems.is_empty(),
            "{}: {} difference(s) from Grafana:\n  {}",
            dashboard.name,
            problems.len(),
            problems.join("\n  ")
        );
    }
}

#[test]
fn an_unlisted_difference_fails() {
    let panel = |slug: &str, title: &str| Panel {
        slug: slug.to_string(),
        title: title.to_string(),
        queries: vec!["q".to_string()],
        kind: mz_dashboards::manifest::Kind::Value,
        unit: mz_dashboards::manifest::Unit::None,
    };
    let grafana = [panel("a", "A"), panel("b", "B")];
    let datadog = [panel("a", "A, retitled")];
    let problems = compare(&grafana, &datadog, &[]);
    assert_eq!(problems.len(), 2, "{problems:?}");

    let listed = [Difference {
        slug: "b",
        gap: Gap::Missing,
        reason: "a fixture",
    }];
    let problems = compare(&grafana, &[panel("a", "A"), panel("b", "B")], &listed);
    assert_eq!(problems.len(), 1, "a stale listing fails: {problems:?}");
}
