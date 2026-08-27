// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Summary tab: is the environment healthy, and where do I go next.
//!
//! Every panel here is a pointer. It answers a question in one number and names
//! the tab that explains it, which is why the panels borrow their shades from the
//! tabs they point at rather than having one of their own — see
//! [`super::theme`].

use mzmon_lib::grafana::generated::{dashboardv2, stat};
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};
use mzmon_lib::grafana::query::{PromQuery, query_group};
use mzmon_lib::grafana::threshold;

use super::selector;
use super::theme;

/// One query, the common case on this tab.
fn data(query: PromQuery) -> dashboardv2::QueryGroupKind {
    query_group(vec![query.build()])
}

/// The Summary tab's rows.
pub fn rows() -> Vec<Row> {
    vec![environment_health(), environment_info()]
}

/// Health at a glance: is it up, has it been up, and is anything behind.
fn environment_health() -> Row {
    Row::new("Environment Health").grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("is-healthy", is_healthy())
            .panel("availability-percent", availability_percent())
            .panel("last-restart", last_restart())
            .panel("summary-currently-hydrating", currently_hydrating())
            .panel("summary-max-lag", max_lag())
            .panel("cpu-usage-current", cpu_usage_current())
            .panel("memory-usage-current", memory_usage_current()),
    )
}

/// What is running, and how much of it there is.
fn environment_info() -> Row {
    Row::new("Environment Info").grid(
        AutoGrid::new(3)
            .row_height(RowHeight::Short)
            .panel("materialize-version", materialize_version())
            .panel("summary-cpu-total", cpu_total())
            .panel("summary-memory-total", memory_total()),
    )
}

fn is_healthy() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
count(
    mz_compute_cluster_status{{{env}}} == 1
) / count(
    mz_compute_cluster_status{{{env}}}
) * 100
"#,
        env = selector::environment()
    );
    Panel::stat("Environment Status")
        .description(
            "**High-level environment health based on the fraction of clusters reporting \
             healthy.** Aggregates `mz_compute_cluster_status` across the env; the result is \
             mapped to text via thresholds: Healthy (100%), Degraded (80-100%), Unhealthy \
             (<80%). When this turns Degraded or Unhealthy, check _Kubernetes Workloads_ for \
             pod restart history and _Cluster Objects / Replicas_ to see which cluster(s) are \
             affected.",
        )
        .data(data(PromQuery::new(expr)))
        .color_background()
        // Mapped to words rather than threshold-coloured: "Degraded" says more
        // than "87".
        .mappings(threshold::health_mapping(80.0, 100.0))
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn availability_percent() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
avg by (materialize_cloud_organization_namespace) (
    avg_over_time(
        mz_compute_cluster_status{{{env}}}[$__range]
    ) * 100
)
"#,
        env = selector::environment()
    );
    Panel::stat("Environment Availability (Select Time Range)")
        .description(
            "**Fraction of time the environment was healthy over the dashboard's selected \
             time range** — computed from `mz_compute_cluster_status` averaged over `$__range`. \
             Effectively an SLO snapshot. Nominal: 100.0000% (note the four decimals — \
             five-nines = 99.999%). Sustained dips correlate with cluster restarts or outages; \
             widen the time range to find when they happened, then check _Last Restart Time_ \
             and _Kubernetes Workloads_ for pod restart context.",
        )
        .data(data(PromQuery::new(expr)))
        .color_background()
        .unit("percent")
        // Four decimals: five-nines and 100% are different stories.
        .decimals(4.0)
        .thresholds(threshold::health(95.0, 99.0).percentage().build())
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn last_restart() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
time()
- topk(1,
    container_start_time_seconds{{{containers}}}
)
"#,
        containers = selector::workload_containers()
    );
    Panel::stat("Last Restart Time")
        .description(
            "**Seconds since the most recent container restart in the environment.** \
             Threshold-colored from red (seconds ago — likely an active incident) through to \
             gray (days ago, fine). Nominal: hours-to-days. Recent restarts (red/orange) are \
             worth correlating with _Environment Availability_ and the _Kubernetes Workloads_ \
             tab's pod-health panels.",
        )
        .data(data(PromQuery::new(expr).instant().legend("{{pod}}")))
        .color_background()
        .justify_center()
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .value_size(25.0)
        .unit("s")
        // Low is bad here: a restart seconds ago is the alarming case, so the
        // palette runs alarming-to-calm as the duration grows.
        .thresholds(threshold::stability_days(2.0, false).build())
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn currently_hydrating() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
count(
    max by (instance_id, collection_id) (
        mz_dataflow_wallclock_lag_seconds{{
            {env},
            {cluster},
            instance_id!="",
            quantile="1"
        }} > 1e15
    )
) or vector(0)
"#,
        env = selector::environment(),
        cluster = selector::cluster()
    );
    Panel::stat("Currently Hydrating")
        .description(
            "**Collections still (re)building their in-memory state — a live hydration-queue \
             proxy.** Hydration rebuilds a dataflow's state from persisted storage after a \
             cluster/replica restart, replica creation, or some DDL; until it finishes, the \
             collection has no output frontier, which this counts (via the \
             `mz_dataflow_wallclock_lag_seconds` sentinel). **Nominal: 0, with brief spikes \
             right after a replica restart that drain back to 0 as dataflows catch up — that's \
             healthy.** A count that stays elevated means something can't finish hydrating \
             (e.g. a source whose `CREATE` didn't complete, or a wedged dataflow). Confirm what's \
             stuck with `SELECT * FROM mz_internal.mz_hydration_statuses WHERE NOT hydrated`; \
             watch _Compute Objects -> Freshness_ for the lag those collections accrue.",
        )
        .data(data(PromQuery::new(expr).legend("hydrating")))
        // Points at Compute Objects, so it wears that tab's colour.
        .shade(theme::COMPUTE.shade)
        .min(0.0)
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn max_lag() -> dashboardv2::PanelKind {
    // `[$__range:1m]` is a Grafana subquery, not a range vector, so the registry's
    // `%%{range}` parameter cannot express it -- see the FIXME on
    // `materialize.info.max_lag`. Written literally here, which is also why this
    // panel is Grafana-only.
    let expr = format!(
        r#"
max(
    max_over_time(
        (
            mz_dataflow_wallclock_lag_seconds{{
                {env}, instance_id!="", quantile="1"
            }} < 1e9
        )[$__range:1m]
    )
)
"#,
        env = selector::environment()
    );
    Panel::stat("Max Lag (Select Time Range)")
        .description(
            "**Worst frontier lag seen anywhere in the environment over the dashboard's \
             selected time range** — how far the most behind collection's output trailed real \
             time. A top-level freshness pointer: low (seconds) is fine and stays gray; if it \
             climbs toward an hour it turns red, meaning some collection is falling behind — \
             open _Compute Objects -> Freshness_ to see which one (and _Currently Hydrating_ / \
             _Sources and Sinks_ for why). Not-yet-hydrated collections are excluded here (they \
             show in _Currently Hydrating_).",
        )
        .data(data(PromQuery::new(expr).instant().legend("max lag")))
        .color_background()
        .unit("s")
        // High is bad: an hour of lag is the alarming end.
        .thresholds(threshold::stability(3600.0, true).build())
        .no_value(NoValue::FilterMismatch)
        .build(0)
}

fn cpu_usage_current() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
sum by (namespace, container) (
    rate(
        container_cpu_usage_seconds_total{{{containers}}}[5m]
    )
) / sum by (namespace, container) (
    kube_pod_container_resource_limits{{resource="cpu", {ns}}}
)
"#,
        containers = selector::containers(),
        ns = selector::namespace()
    );
    Panel::gauge("Current CPU Usage (5 min)")
        .description(
            "**Current CPU usage as a fraction of each container's limit, averaged over the \
             last 5 minutes.** Per-container gauge — shows the worst-loaded container types in \
             the env. Nominal: well below 1.0; sustained near 1.0 means a container type is \
             CPU-bound. For time-resolved per-pod view see _Kubernetes Workloads -> Pod CPU \
             Usage_; for the Materialize workload causing it see _Compute Objects -> Dataflow \
             Elapsed Rate_.",
        )
        .data(data(PromQuery::new(expr).instant().legend("{{container}}")))
        .unit("percentunit")
        .thresholds(threshold::load_default().build())
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn memory_usage_current() -> dashboardv2::PanelKind {
    // Averaged per pod before summing: a pod scraped by two jobs would otherwise
    // count twice on both sides of the ratio.
    let expr = format!(
        r#"
sum by (namespace, container) (
    avg by (namespace, pod, container) (
        container_memory_working_set_bytes{{{containers}}}
    )
) / sum by (namespace, container) (
    avg by (namespace, pod, container) (
        container_spec_memory_limit_bytes{{{containers}}}
    )
)
"#,
        containers = selector::workload_containers()
    );
    Panel::gauge("Current Memory Usage")
        .description(
            "**Current memory usage as a fraction of each container's limit.** Per-container \
             gauge — shows the worst-loaded container types. **Sustained near 1.0 is \
             dangerous** — OOM-kill triggers a hydration cycle (in-memory state has to be \
             rebuilt from persisted storage, taking minutes-to-hours depending on data size). \
             For time-resolved view see _Kubernetes Workloads -> Pod Memory Usage_; the \
             offending workload usually shows in _Compute Objects -> Arrangements_.",
        )
        .data(data(PromQuery::new(expr).instant().legend("{{container}}")))
        .unit("percentunit")
        .thresholds(threshold::load_default().build())
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn materialize_version() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
group by (mz_version) (
    mz_compute_cluster_status{{{env}}}
)
"#,
        env = selector::environment()
    );
    Panel::stat("Materialize Version")
        .description(
            "**The version of Materialize currently running in the environment.** A single \
             version is the steady state; multiple distinct values typically appear briefly \
             during a rolling upgrade. Click the value to open the corresponding commit on \
             GitHub. If the version doesn't match what you expect, check the env's manifest in \
             the deployment pipeline.",
        )
        .data(data(
            PromQuery::new(expr).instant().legend("{{mz_version}}"),
        ))
        // No sparkline: a version is not a time series.
        .graph_mode(stat::BigValueGraphMode::None)
        .value_size(20.0)
        // Reduce over the version label only, so the panel shows the version
        // string rather than the metric's value.
        .reduce_fields("/^mz_version$/")
        .links(vec![commit_link()])
        .build(0)
}

/// Link the displayed version to its commit on GitHub.
///
/// `${__data.fields.commit}` is a Grafana data link, interpolated from the row's
/// `commit` field rather than from a dashboard variable.
fn commit_link() -> serde_json::Map<String, serde_json::Value> {
    match serde_json::json!({
        "title": "View Materialize at Commit",
        "url": "https://github.com/MaterializeInc/materialize/commit/${__data.fields.commit}",
        "targetBlank": true,
    }) {
        serde_json::Value::Object(map) => map,
        _ => unreachable!("a json! object literal is an object"),
    }
}

fn cpu_total() -> dashboardv2::PanelKind {
    // cAdvisor reports a CPU limit as a quota over a period; their ratio is cores.
    let expr = format!(
        r#"
sum by (container) (
    container_spec_cpu_quota{{ {containers} }}
    / container_spec_cpu_period{{ {containers} }}
)
"#,
        containers = selector::workload_containers()
    );
    Panel::stat("Total CPU Capacity")
        .description(
            "**Total CPU cores configured across containers in the selected scope** (sum of \
             CPU limits from cAdvisor). Steps correlate with `ALTER CLUSTER REPLICA SIZE`, \
             `CREATE`/`DROP CLUSTER REPLICA`, or pod restarts. On the Summary tab the \
             monitoring exporter is excluded (so this reflects user-workload capacity); on the \
             Kubernetes Workloads tab it's included.",
        )
        .data(data(PromQuery::new(expr).legend("CPUs ({{container}})")))
        .text_mode(stat::BigValueTextMode::ValueAndName)
        // Points at Kubernetes Workloads.
        .shade(theme::KUBERNETES.shade)
        .unit("cores")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn memory_total() -> dashboardv2::PanelKind {
    let expr = format!(
        r#"
sum by (container) (
    container_spec_memory_limit_bytes{{ {containers} }}
)
"#,
        containers = selector::workload_containers()
    );
    Panel::stat("Total Memory")
        .description(
            "**Total memory configured across containers in the selected scope** (sum of \
             memory limits from cAdvisor). Memory is the dominant constraint on Materialize: \
             in-memory arrangements (see _Compute Objects -> Arrangements_) live in here. Steps \
             correlate with `ALTER CLUSTER REPLICA SIZE` or pod restarts.",
        )
        .data(data(PromQuery::new(expr).legend("Memory ({{container}})")))
        .text_mode(stat::BigValueTextMode::ValueAndName)
        .shade(theme::KUBERNETES.shade)
        .unit("bytes")
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tab_has_both_rows_and_all_ten_panels() {
        let rows = rows();
        assert_eq!(rows.len(), 2);
        // Panel count is checked end to end against the golden in
        // tests/env_top_parity.rs; this just guards the row split.
        let json = serde_json::to_value(
            mzmon_lib::grafana::layout::Layout::rows(rows)
                .assemble()
                .expect("assemble")
                .layout,
        )
        .expect("serialize");
        let text = json.to_string();
        assert_eq!(text.matches("ElementReference").count(), 10);
    }

    #[test]
    fn every_shade_is_a_theme_colour() {
        // Checked on the built output rather than the source text: what matters is
        // that no panel ends up with a colour outside the qualitative palette,
        // however it got there.
        use mzmon_lib::grafana::palette;
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows())
            .assemble()
            .expect("assemble");
        let mut shaded = 0usize;
        for element in assembled.elements.values() {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            let Some(colour) = &panel.spec.viz_config.spec.field_config.defaults.color else {
                continue;
            };
            let Some(hex) = &colour.fixed_color else {
                continue;
            };
            shaded += 1;
            assert!(
                palette::THEME.contains(&hex.as_str()),
                "{hex} is not a THEME colour"
            );
        }
        // Three panels borrow a shade in the baseline; if that drops to zero the
        // assertion above would pass vacuously.
        assert_eq!(shaded, 3, "expected three borrowed shades");
    }

    #[test]
    fn every_panel_query_is_environment_scoped_or_namespace_scoped() {
        // The property the selectors exist to guarantee: no panel queries the
        // whole fleet. Asserted on the rendered expressions, not the source.
        let assembled = mzmon_lib::grafana::layout::Layout::rows(rows())
            .assemble()
            .expect("assemble");
        for (name, element) in &assembled.elements {
            let dashboardv2::Element::PanelKind(panel) = element else {
                continue;
            };
            for query in &panel.spec.data.spec.queries {
                let expr = query.spec.query.spec.as_ref().and_then(|s| s.get("expr"));
                let expr = expr.and_then(|e| e.as_str()).unwrap_or_default();
                assert!(
                    expr.contains("$environmentNameList") || expr.contains("$mzNamespaceList"),
                    "{name} is not scoped to the environment:\n{expr}"
                );
            }
        }
    }
}
