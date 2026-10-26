// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The ingress dashboard: how users reach a Materialize environment.
//!
//! Every SQL client, HTTP API call and console session arrives through a cloud
//! load balancer and enters balancerd, and the console's own API calls pass
//! through its nginx first. Both forward to environmentd, which serves the SQL
//! and HTTP APIs. This dashboard measures each hop with rate, errors and
//! duration, so that "users cannot connect" or "the console is slow" can be
//! placed on one of them.
//!
//! # Who it is for
//!
//! An operator hearing that users cannot connect, or that the console is slow,
//! deciding which hop is at fault. The Overview's verdict answers whether the
//! front door is serving; the component tabs say which hop and why.
//!
//! # Scope
//!
//! The environment picker and nothing narrower. balancerd and the console belong
//! to no cluster, so the cluster and replica pickers would filter nothing here —
//! see [`variable::dependency_scoped`].
//!
//! # What older Materialize versions publish
//!
//! balancerd's connection counts and capacity gauges are old. Its latency
//! histograms, the meaning of its `error` status, and every console metric are
//! not, so on older versions those panels are empty and their empty-state text
//! says why. The SQL and HTTP API tab reads environmentd alone, and is complete
//! on every supported version.
//!
//! # The load balancer is optional
//!
//! Its rows read the gateway's provider pulls, which are opt-in, so they render
//! on `infra-net`'s pull detection with a fallback that says how to turn one
//! on. See [`load_balancer`].

pub mod api;
pub mod balancer;
pub mod console;
pub mod load_balancer;
pub mod overview;
pub mod theme;

use mzmon_lib::grafana::context::DashboardScope;
use mzmon_lib::grafana::dashboard::{CursorSync, Dashboard, Resource};
use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{Layout, Tab};
use mzmon_lib::grafana::panel::NoValue;
use mzmon_lib::grafana::{dashboard, folder::Folder, tags, variable};
use mzmon_lib::query::QueryRegistry;

use crate::grafana::queries::Queries;

/// Resource name. Stable independently of the title, since it is what permalinks
/// and the chart's manifest key are built from.
pub const NAME: &str = "mz-mon-env-ingress";

/// Artifact filename stem, which is *not* the resource name.
pub const NAME_STEM: &str = "env-ingress";

/// Dashboard title.
pub const TITLE: &str = "Materialize Ingress (Balancer, Console)";

/// Minimum Materialize version this dashboard requires.
///
/// The balancerd and console PodMonitors select on `app.kubernetes.io/name`,
/// which the operator sets from this version on.
pub const MIN_MZ_VERSION: &str = "v26.24.0";
/// Recommended Materialize version.
// TODO: the first release carrying MaterializeInc/materialize#40192 (balancerd
// timings and fault accounting) and #40193 (console metrics), once both ship.
pub const REC_MZ_VERSION: &str = "v26.24.0";

/// What a balancerd panel shows when this environment reports nothing.
pub(crate) fn no_balancer() -> NoValue {
    NoValue::Custom(
        "No metrics from balancerd in this environment. Check the environment picker, and that \
         the balancerd PodMonitor is installed."
            .to_string(),
    )
}

/// What a panel on one of balancerd's timing histograms shows without them.
pub(crate) fn no_timings() -> NoValue {
    NoValue::Custom(
        "No connection timings from balancerd. Older Materialize versions don't publish them."
            .to_string(),
    )
}

/// What a console panel shows when the console reports nothing.
pub(crate) fn no_console() -> NoValue {
    NoValue::Custom(
        "No metrics from the console. Older Materialize versions don't publish them, and the \
         console PodMonitor must be installed."
            .to_string(),
    )
}

/// What an environmentd panel shows when this environment reports nothing.
pub(crate) fn no_environmentd() -> NoValue {
    NoValue::Custom(
        "No metrics from environmentd in this environment. Check the environment picker, and \
         that Materialize is running."
            .to_string(),
    )
}

/// The tabs, in order.
///
/// Overview answers "can users get in". The rest follow a connection from the
/// outside in: the cloud load balancer, then balancerd, which every connection
/// passes through, the console's included. Console is the browser's side, and
/// the SQL and HTTP API tab is where both lead.
fn tabs(q: &Queries) -> Vec<Tab> {
    vec![
        Tab::new(theme::OVERVIEW.title).rows(overview::rows(q)),
        Tab::new(theme::LOAD_BALANCER.title).rows(load_balancer::rows(q)),
        Tab::new(theme::BALANCER.title).rows(balancer::rows(q)),
        Tab::new(theme::CONSOLE.title).rows(console::rows(q)),
        Tab::new(theme::API.title).rows(api::rows(q)),
    ]
}

/// [`variable::dependency_scoped`], plus `infra-net`'s hidden detection of the
/// gateway's load balancer pulls, which the Load Balancer tab's rows render on.
fn variables() -> Vec<dashboardv2::VariableKind> {
    let mut variables = variable::dependency_scoped();
    variables.push(variable::cloud_network_pulls());
    variables
}

/// The export target this crate produces.
const TARGET_EXPORT: &str = "generic";

/// Build the dashboard for a deployment.
pub fn build(sql_metric_prefix: &str, registry: &QueryRegistry) -> dashboard::Result<Resource> {
    let scope = DashboardScope::for_prefix(sql_metric_prefix);
    let queries = Queries::new(registry, &scope);
    let layout = Layout::tabs(tabs(&queries));

    let failures = queries.failures();
    if !failures.is_empty() {
        return Err(dashboard::Error::Registry {
            dashboard: NAME_STEM,
            failures,
        });
    }

    Dashboard::new(NAME, TITLE)
        .description(
            "How users reach a Materialize environment.\n\n\
             Rate, errors and latency at each hop: the cloud load balancer, \
             where its metrics are pulled; balancerd, which every SQL and HTTPS \
             connection passes through; the console, which serves the web app \
             and forwards its API calls; and environmentd's SQL and HTTP APIs \
             behind them both.",
        )
        .tags([tags::MATERIALIZE, tags::MZMON, tags::content::NETWORK])
        .folder(Folder::Materialize)
        .cursor_sync(CursorSync::Crosshair)
        .variables(variables())
        .metadata_annotation(
            "monitoring.materialize.cloud/min-mz-version",
            MIN_MZ_VERSION,
        )
        .metadata_annotation(
            "monitoring.materialize.cloud/rec-mz-version",
            REC_MZ_VERSION,
        )
        .metadata_annotation(
            "monitoring.materialize.cloud/sql-metric-prefix",
            sql_metric_prefix,
        )
        .metadata_annotation("monitoring.materialize.cloud/target-export", TARGET_EXPORT)
        .layout(layout)
        .build()
}

/// Render for the registry.
pub fn render(
    options: &crate::grafana::Options,
    registry: &QueryRegistry,
) -> crate::grafana::render::Result<Resource> {
    use crate::grafana::render::Error;

    build(&options.sql_metric_prefix, registry).map_err(|source| Error::Build {
        name: NAME_STEM,
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_registry;
    use crate::grafana::test_support;

    fn built() -> Resource {
        build("mz_", test_registry()).expect("build")
    }

    fn elements_json() -> String {
        serde_json::to_string(&built().spec.elements).expect("serialize")
    }

    #[test]
    fn it_builds() {
        let resource = built();
        assert_eq!(resource.metadata.name, NAME);
        assert_eq!(resource.spec.title, TITLE);
    }

    #[test]
    fn every_tab_is_present() {
        let resource = built();
        let json = serde_json::to_string(&resource.spec.layout).expect("serialize");
        for theme in theme::THEMED {
            assert!(json.contains(theme.title), "missing tab {}", theme.title);
        }
    }

    #[test]
    fn it_is_filed_under_materialize() {
        let resource = built();
        let json = serde_json::to_string(&resource.metadata).expect("serialize");
        assert!(json.contains("\"materialize\""), "{json}");
    }

    #[test]
    fn every_query_references_only_variables_the_dashboard_defines() {
        test_support::assert_variables_defined(&built());
    }

    #[test]
    fn no_panel_leaves_its_empty_state_to_the_default() {
        // Half of what this dashboard reads is absent on older Materialize
        // versions, so an empty panel has to say which half it is.
        test_support::assert_every_panel_has_no_value(&built());
    }

    #[test]
    fn multi_series_panels_are_not_shaded() {
        test_support::assert_multi_series_panels_unshaded(&built());
    }

    #[test]
    fn every_metric_query_is_scoped_to_the_environment() {
        // balancerd and the console run in every environment's namespace on a
        // shared cluster, and their metric names carry no environment of their
        // own.
        let json = elements_json();
        for prefix in ["mz_balancer_", "mz_console_", "mz_http_"] {
            let selectors = test_support::selectors_of(&json, prefix);
            assert!(!selectors.is_empty(), "no {prefix} queries found");
            for selector in selectors {
                assert!(
                    selector.contains("$environmentNameList"),
                    "unscoped: {selector}"
                );
            }
        }
    }

    #[test]
    fn the_http_api_panels_read_only_what_balancerd_forwards() {
        // environmentd's HTTP server also answers the operator and the metric
        // scrapes, which would swamp the client traffic.
        let json = elements_json();
        let selectors = test_support::selectors_of(&json, "mz_http_");
        for selector in selectors {
            assert!(selector.contains(r#"source=\"external\""#), "{selector}");
        }
    }

    #[test]
    fn the_console_logs_leave_out_health_checks() {
        // On a reference install, load balancer health checks are nearly every
        // line the console logs, and half of them are answered with a 400.
        let q = Queries::new(test_registry(), &DashboardScope::default());
        let logs = q.logs("materialize.ingress.console.logs.errors");
        let json = serde_json::to_string(&logs.query_group).expect("serialize");
        assert!(json.contains("HealthChecker"), "{json}");
        assert!(json.contains("kube-probe"), "{json}");
        assert!(q.failures().is_empty(), "{:?}", q.failures());
    }

    /// Every PromQL expression on the dashboard.
    fn expressions() -> Vec<String> {
        let json = serde_json::to_value(&built().spec.elements).expect("serialize");
        let mut out = Vec::new();
        for element in json.as_object().expect("elements").values() {
            let Some(queries) = element
                .pointer("/spec/data/spec/queries")
                .and_then(serde_json::Value::as_array)
            else {
                continue;
            };
            for query in queries {
                if let Some(expr) = query
                    .pointer("/spec/query/spec/expr")
                    .and_then(serde_json::Value::as_str)
                {
                    out.push(expr.to_string());
                }
            }
        }
        out
    }

    #[test]
    fn each_provider_row_renders_on_its_own_pull() {
        use crate::grafana::infra_networking::cloud::{AWS_LB, AZURE_LB, GCP_LB};
        let conditions = test_support::variable_conditioned_rows(&built());
        let mut when: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "matches")
            .map(|(_, _, value)| value.as_str())
            .collect();
        when.sort();
        let mut expected = vec![AWS_LB, AZURE_LB, GCP_LB];
        expected.sort();
        assert_eq!(when, expected, "{conditions:?}");
    }

    #[test]
    fn exactly_one_fallback_covers_every_load_balancer_pull() {
        // A fallback that missed a pull would render its note beside that
        // pull's rows.
        let conditions = test_support::variable_conditioned_rows(&built());
        let fallbacks: Vec<_> = conditions
            .iter()
            .filter(|(_, op, _)| op == "notMatches")
            .collect();
        assert_eq!(fallbacks.len(), 1, "{fallbacks:?}");
        assert_eq!(fallbacks[0].2, load_balancer::load_balancer_pulls());
    }

    #[test]
    fn the_pull_rows_render_when_every_pull_is_selected() {
        test_support::assert_row_conditions_can_read_all(&built());
    }

    #[test]
    fn every_load_balancer_query_is_tied_to_the_environment() {
        // The pulled series carry no environment, and the pull may name
        // Grafana's load balancer too. Each provider reaches the environment
        // through its own join: target group names on AWS, Services in the
        // environment's namespace on Google Cloud and Azure.
        let mut found = 0;
        for expr in expressions() {
            if expr.contains("aws_networkelb_") {
                found += 1;
                assert!(
                    expr.contains(
                        r#"dimension_TargetGroup=~"targetgroup/.+-${environmentNameList}-"#
                    ),
                    "{expr}"
                );
            }
            if expr.contains("stackdriver_") || expr.contains("azure_microsoft_network_") {
                found += 1;
                assert!(expr.contains("$mzNamespaceList"), "{expr}");
            }
        }
        assert!(found > 0, "no load balancer queries found");
    }

    #[test]
    fn the_fallback_says_how_to_turn_each_pull_on() {
        for key in [
            "cloudwatch.nlb.loadBalancers",
            "loadBalancing.forwardingRules",
            "azure.loadBalancer.clusters",
            "provider_metrics.network_load_balancers",
            "provider_metrics.forwarding_rules",
            "provider_metrics.load_balancer_clusters",
        ] {
            assert!(load_balancer::NO_LOAD_BALANCER.contains(key), "{key}");
        }
    }
}
