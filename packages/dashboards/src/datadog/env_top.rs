// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `env-top` for Datadog: the port of [`crate::grafana::env_top`].
//!
//! # Layout
//!
//! Datadog's ordered layout has groups but no tabs, so each Grafana row becomes
//! a group, in tab order. A tab's leading summary rows, whose headers Grafana
//! hides, share one group named for the tab. Each tab's groups take one of
//! Datadog's group colors.
//!
//! # Parity
//!
//! Every panel is placed under its Grafana slug, and `tests/datadog_parity.rs`
//! compares the two dashboards' manifests panel by panel. [`DIFFERENCES`] lists
//! each Grafana panel this one draws differently or leaves out, with the reason.

use datadog_api_client::datadogV1::model::{
    Dashboard, DashboardLayoutType, DashboardReflowType, Widget,
};
use mzmon_lib::query::QueryRegistry;

use super::queries::Queries;
use super::render::{Error, Result};
use super::widget::{self, Column, Format, Look, Panel, Reduce};
use super::{Options, variable};
use crate::grafana::env_top as grafana;
use crate::manifest::{self, Difference, Gap};

/// Artifact stem, shared with the Grafana dashboard.
pub const NAME_STEM: &str = grafana::NAME_STEM;

/// Why a template renders only as a timeseries. See [`super::widget`].
const TWO_LAYERS: &str = "its Datadog template aggregates in two layers, which Datadog's scalar \
                          widgets can't draw, so it's a timeseries";

/// Grafana panels this dashboard draws differently, or not at all.
pub const DIFFERENCES: &[Difference] = &[
    Difference {
        slug: "summary-alerts-firing",
        gap: Gap::Missing,
        reason: "Datadog alerts are monitors, not a series to draw",
    },
    Difference {
        slug: "summary-alerts",
        gap: Gap::Missing,
        reason: "Datadog alerts are monitors, not a series to draw",
    },
    Difference {
        slug: "summary-alert-history",
        gap: Gap::Missing,
        reason: "Datadog alerts are monitors, not a series to draw",
    },
    Difference {
        slug: "last-restart",
        gap: Gap::Missing,
        reason: "Datadog has no `time()`, so the age of the newest container can't be derived \
                 from its start timestamp",
    },
    Difference {
        slug: "is-healthy",
        gap: Gap::Unit,
        reason: "Grafana maps the percentage to Healthy, Degraded, or Unhealthy; a query value \
                 shows the percentage, colored the same way",
    },
    Difference {
        slug: "summary-cpu-total",
        gap: Gap::Kind,
        reason: TWO_LAYERS,
    },
    Difference {
        slug: "k8s-res-cpu-total",
        gap: Gap::Kind,
        reason: TWO_LAYERS,
    },
    Difference {
        slug: "resource-pod-status",
        gap: Gap::Kind,
        reason: TWO_LAYERS,
    },
    Difference {
        slug: "resource-deployment-status",
        gap: Gap::Kind,
        reason: "ready and unavailable replicas are two templates, which a table shows side by \
                 side, one row per namespace",
    },
    Difference {
        slug: "replica-sizes",
        gap: Gap::Kind,
        reason: TWO_LAYERS,
    },
    Difference {
        slug: "freshness-top-collections",
        gap: Gap::Kind,
        reason: TWO_LAYERS,
    },
    Difference {
        slug: "sources-types",
        gap: Gap::Kind,
        reason: TWO_LAYERS,
    },
];

/// The note above the panels.
///
/// The Grafana environment picker selects exactly one environment, and a Datadog
/// template variable always offers `*`. Environments reuse cluster and replica
/// ids, so a value grouped by id merges them when several are selected.
// TODO: settle how Datadog panels stay scoped to one environment (DEP-115), then
// drop the instruction to pick one.
const NOTE: &str = "**Pick one environment, and the namespace it runs in.** Environments reuse \
cluster and replica ids, so the panels merge them when several are selected. The Kubernetes \
panels read the namespace picker, because pod metrics carry no environment tag. Pick one value \
or `*` in each picker. The peek and Iceberg commit latency panels need percentiles enabled for \
their metrics in Datadog.";

/// A group: a Grafana row, or a tab's summary rows.
struct Section {
    title: &'static str,
    color: &'static str,
    panels: Vec<Panel>,
}

impl Section {
    fn new(title: &'static str, color: &'static str, panels: Vec<Panel>) -> Self {
        Section {
            title,
            color,
            panels,
        }
    }
}

/// Datadog group colors, one per Grafana tab.
mod color {
    pub const SUMMARY: &str = "gray";
    pub const KUBERNETES: &str = "blue";
    pub const CONNECTIONS: &str = "purple";
    pub const CLUSTERS: &str = "green";
    pub const COMPUTE: &str = "orange";
    pub const SOURCES_SINKS: &str = "yellow";
}

/// Build the dashboard.
pub fn render(options: &Options, registry: &QueryRegistry) -> Result<Dashboard> {
    build(options, registry).map(|(dashboard, _)| dashboard)
}

/// The panels' manifest, for the parity ledger.
pub fn manifest(options: &Options, registry: &QueryRegistry) -> Result<Vec<manifest::Panel>> {
    build(options, registry).map(|(_, manifest)| manifest)
}

fn build(options: &Options, registry: &QueryRegistry) -> Result<(Dashboard, Vec<manifest::Panel>)> {
    let q = Queries::new(registry, options);
    let sections = sections(&q);

    let failures = q.failures();
    if !failures.is_empty() {
        return Err(Error::Registry {
            name: NAME_STEM,
            failures,
        });
    }

    let manifest = sections
        .iter()
        .flat_map(|s| s.panels.iter().map(|p| p.manifest.clone()))
        .collect();
    let mut widgets: Vec<Widget> = vec![widget::note(NOTE)];
    widgets.extend(
        sections
            .iter()
            .map(|s| widget::group(s.title, s.color, &s.panels)),
    );

    let dashboard = Dashboard::new(
        DashboardLayoutType::ORDERED,
        grafana::TITLE.to_string(),
        widgets,
    )
    .description(Some(
        "Overview of a Materialize environment, ported from the Grafana dashboard of the \
             same name."
            .to_string(),
    ))
    .reflow_type(DashboardReflowType::AUTO)
    .template_variables(Some(vec![
        variable::environment(),
        variable::namespace(),
        variable::cluster(),
        variable::replica(),
    ]));
    Ok((dashboard, manifest))
}

fn sections(q: &Queries) -> Vec<Section> {
    let mut sections = summary(q);
    sections.extend(kubernetes(q));
    sections.extend(connections(q));
    sections.extend(clusters(q));
    sections.extend(compute(q));
    sections.extend(sources_sinks(q));
    sections
}

/// The Summary tab: health at a glance, and what is running.
fn summary(q: &Queries) -> Vec<Section> {
    let health = vec![
        widget::query_value(
            "is-healthy",
            "Environment Status",
            &q.single("materialize.health.clusters.status.percentage"),
            &Look::new()
                .format(Format::Percent)
                .healthy_from(80.0, 100.0),
        ),
        widget::query_value(
            "availability-percent",
            "Environment Availability (Select Time Range)",
            &q.single("materialize.health.environment.availability.percentage"),
            // Four decimals: five-nines and 100% are different stories.
            &Look::new()
                .reduce(Reduce::Avg)
                .format(Format::Percent)
                .precision(4)
                .healthy_from(95.0, 99.0),
        ),
        currently_hydrating(q, "summary-currently-hydrating"),
        widget::query_value(
            "summary-max-lag",
            "Worst Freshness (Select Time Range)",
            &q.single("materialize.info.max_lag"),
            &Look::new()
                .reduce(Reduce::Max)
                .format(Format::Seconds)
                .unhealthy_from(60.0, 3600.0),
        ),
        widget::grouped_values(
            "cpu-usage-current",
            "Current CPU Usage (5 min)",
            &q.single("materialize.kubernetes.cpu.usage.percent"),
            &Look::new().format(Format::Fraction),
        ),
        widget::grouped_values(
            "memory-usage-current",
            "Current Memory Usage",
            &q.single("materialize.kubernetes.memory.usage.percent"),
            &Look::new().format(Format::Fraction),
        ),
    ];
    let info = vec![
        // One bar per version: a version is a tag, and a query value draws a
        // number.
        widget::grouped_values(
            "materialize-version",
            "Materialize Version",
            &q.single("materialize.info.version"),
            &Look::new(),
        ),
        widget::timeseries(
            "summary-cpu-total",
            "Total CPU Capacity",
            &q.single("materialize.kubernetes.cpu.capacity"),
            &[],
            &Look::new().format(Format::Cores).min_zero(),
        ),
        widget::grouped_values(
            "summary-memory-total",
            "Total Memory",
            &q.single("materialize.kubernetes.memory.capacity"),
            &Look::new().format(Format::Bytes),
        ),
    ];
    vec![
        Section::new("Environment Health", color::SUMMARY, health),
        Section::new("Environment Info", color::SUMMARY, info),
    ]
}

/// The Currently Hydrating number, which the Summary and Compute Objects tabs
/// both show.
fn currently_hydrating(q: &Queries, slug: &str) -> Panel {
    widget::query_value(
        slug,
        "Currently Hydrating",
        &q.single("materialize.compute.hydration.currently_hydrating"),
        &Look::new(),
    )
}

/// The Kubernetes Workloads tab: the pods underneath Materialize.
fn kubernetes(q: &Queries) -> Vec<Section> {
    let replicas_and_rest = &["Cluster replicas", "Other pods"];
    let by_direction = &[
        "Cluster replicas rx",
        "Other pods rx",
        "Cluster replicas tx",
        "Other pods tx",
    ];
    let summary = vec![
        widget::timeseries(
            "k8s-res-cpu-total",
            "Total CPU Capacity",
            &q.single("materialize.kubernetes.cpu.capacity.all_containers"),
            &[],
            &Look::new().format(Format::Cores).min_zero(),
        ),
        widget::grouped_values(
            "k8s-res-memory-total",
            "Total Memory",
            &q.single("materialize.kubernetes.memory.capacity.all_containers"),
            &Look::new().format(Format::Bytes),
        ),
        widget::timeseries(
            "resource-pod-status",
            "Pod Readiness",
            &q.single("materialize.kubernetes.pods.readiness"),
            &[],
            &Look::new().min_zero(),
        ),
        widget::pie(
            "resource-statefulset-status",
            "StatefulSet Readiness",
            &q.single("materialize.kubernetes.statefulsets.ready"),
            &Look::new(),
        ),
        widget::table(
            "resource-deployment-status",
            "Deployment Readiness",
            &q.labeled(
                "materialize.kubernetes.deployments.readiness",
                &["Ready", "Unavailable"],
            ),
            &[
                Column::nth("Ready", 0, Reduce::Last),
                Column::nth("Unavailable", 1, Reduce::Last),
            ],
            &Look::new(),
        ),
    ];
    let pods = vec![
        widget::timeseries(
            "pod-cpu-percent",
            "Pod CPU Usage",
            &q.labeled("materialize.kubernetes.pods.cpu_usage", replicas_and_rest),
            replicas_and_rest,
            &Look::new().format(Format::Fraction),
        ),
        widget::timeseries(
            "pod-memory-percent",
            "Pod Memory Usage",
            &q.labeled(
                "materialize.kubernetes.pods.memory_usage",
                replicas_and_rest,
            ),
            replicas_and_rest,
            &Look::new().format(Format::Fraction),
        ),
    ];
    let networking = vec![
        widget::timeseries(
            "pod-network-rx",
            "Pod Network Rx",
            &q.labeled("materialize.kubernetes.pods.network_rx", replicas_and_rest),
            replicas_and_rest,
            &Look::new().format(Format::BytesPerSecond),
        ),
        widget::timeseries(
            "pod-network-tx",
            "Pod Network Tx",
            &q.labeled("materialize.kubernetes.pods.network_tx", replicas_and_rest),
            replicas_and_rest,
            &Look::new().format(Format::BytesPerSecond),
        ),
        widget::timeseries(
            "pod-network-errors",
            "Pod Network Errors",
            &q.labeled("materialize.kubernetes.pods.network_errors", by_direction),
            by_direction,
            &Look::new().format(Format::PerSecond("error")),
        ),
        widget::timeseries(
            "pod-network-drops",
            "Pod Network Packet Drops",
            &q.labeled("materialize.kubernetes.pods.network_drops", by_direction),
            by_direction,
            &Look::new().format(Format::PerSecond("packet")),
        ),
    ];
    vec![
        Section::new("Kubernetes Workloads", color::KUBERNETES, summary),
        Section::new("Pod Metrics", color::KUBERNETES, pods),
        Section::new("Pod Networking", color::KUBERNETES, networking),
    ]
}

/// The Connections / Activity tab: who is talking to Materialize, and how fast.
fn connections(q: &Queries) -> Vec<Section> {
    let summary = vec![
        widget::grouped_values(
            "connections-active-sessions",
            "Active Sessions",
            &q.single("materialize.connections.sessions.active"),
            &Look::new(),
        ),
        widget::grouped_values(
            "connections-active-queries",
            "Active Queries",
            &q.single("materialize.connections.queries.rate"),
            &Look::new().format(Format::PerSecond("query")),
        ),
        widget::query_value(
            "connections-adapter-command-rate",
            "SQL Control Plane Command Rate",
            &q.single("materialize.connections.adapter.command_rate"),
            &Look::new()
                .format(Format::PerSecond("command"))
                .precision(2),
        ),
    ];
    let latency = |slug, title, id| {
        widget::timeseries(
            slug,
            title,
            &q.single(id),
            &[],
            // Log axis: peek latency spans milliseconds to seconds.
            &Look::new().format(Format::Seconds).log_scale(),
        )
    };
    let queries = vec![
        widget::pie(
            "queries-distribution",
            "Query Distribution (by statement_type)",
            &q.single("materialize.connections.queries.distribution"),
            &Look::new().reduce(Reduce::Sum),
        ),
        widget::timeseries(
            "queries-rate-by-statement",
            "Query Rate (by statement_type / session_type)",
            &q.single("materialize.connections.queries.rate_by_statement"),
            &[],
            &Look::new().format(Format::PerSecond("query")).min_zero(),
        ),
        latency(
            "queries-peek-latency-p50",
            "Peek Latency (p50)",
            "materialize.connections.peek_latency.p50",
        ),
        latency(
            "queries-peek-latency-p90",
            "Peek Latency (p90)",
            "materialize.connections.peek_latency.p90",
        ),
        latency(
            "queries-peek-latency-p99",
            "Peek Latency (p99)",
            "materialize.connections.peek_latency.p99",
        ),
    ];
    let commands = vec![widget::table(
        "adapter-commands-by-app",
        "SQL Control Plane Commands by Application",
        &q.single("materialize.connections.adapter.commands_by_application"),
        &[Column::of("Commands", Reduce::Sum)],
        &Look::new(),
    )];
    vec![
        Section::new("Connections / Activity", color::CONNECTIONS, summary),
        Section::new("Queries", color::CONNECTIONS, queries),
        Section::new("SQL Control Plane Commands", color::CONNECTIONS, commands),
    ]
}

/// The Cluster Objects / Replicas tab: what compute exists, and how it is shaped.
fn clusters(q: &Queries) -> Vec<Section> {
    const CLUSTERS: &[&str] = &["Total Clusters", "System Clusters"];
    const REPLICAS: &[&str] = &["Total Replicas", "Redundant Replicas"];
    let summary = vec![
        widget::query_values(
            "cluster-count",
            "Cluster Count",
            CLUSTERS,
            &q.labeled("materialize.clusters.count", CLUSTERS),
            &Look::new(),
        ),
        widget::query_values(
            "replica-count",
            "Replica Count",
            REPLICAS,
            &q.labeled("materialize.clusters.replicas.count", REPLICAS),
            &Look::new(),
        ),
    ];
    let replication = vec![widget::timeseries(
        "replica-sizes",
        "Replica Sizes",
        &q.single("materialize.clusters.replicas.sizes"),
        &[],
        &Look::new().min_zero(),
    )];
    // The status value says nothing here; the group tags are the content.
    let information = vec![widget::table(
        "cluster-table",
        "Cluster Information",
        &q.single("materialize.clusters.info"),
        &[Column::of("Status", Reduce::Last)],
        &Look::new(),
    )];
    vec![
        Section::new("Cluster Objects / Replicas", color::CLUSTERS, summary),
        Section::new("Replication / Availability", color::CLUSTERS, replication),
        Section::new("Cluster Information", color::CLUSTERS, information),
    ]
}

/// The Compute Objects tab: what the clusters are doing.
fn compute(q: &Queries) -> Vec<Section> {
    let count = |slug, title, id| widget::query_value(slug, title, &q.single(id), &Look::new());
    let summary = vec![
        count(
            "active-mzd-views",
            "Active Materialized Views",
            "materialize.compute.materialized_views.count",
        ),
        count(
            "active-indexes",
            "Active Indexes",
            "materialize.compute.indexes.count",
        ),
        count(
            "active-views",
            "Active Views",
            "materialize.compute.views.count",
        ),
        widget::pie(
            "active-subscribes",
            "Active Subscribes",
            &q.single("materialize.compute.subscribes.active"),
            &Look::new(),
        ),
        widget::pie(
            "index-types",
            "Index Relationship Types",
            &q.single("materialize.compute.indexes.by_type"),
            &Look::new(),
        ),
    ];
    let freshness = vec![
        widget::timeseries(
            "freshness-lag-by-cluster",
            "Worst Freshness by Cluster",
            &q.single("materialize.compute.freshness.lag_by_cluster"),
            &[],
            &Look::new().format(Format::Seconds).log_scale(),
        ),
        // No log scale, unlike its neighbor: it would flatten the decay that shows
        // rehydration finishing.
        widget::timeseries(
            "freshness-lag-total-by-cluster",
            "Total Freshness by Cluster",
            &q.single("materialize.compute.freshness.lag_total_by_cluster"),
            &[],
            &Look::new().format(Format::Seconds).min_zero(),
        ),
        widget::timeseries(
            "freshness-top-collections",
            "Least Fresh Objects",
            &q.single("materialize.compute.freshness.top_collections"),
            &[],
            &Look::new().format(Format::Seconds).log_scale(),
        ),
    ];
    let hydration = vec![
        currently_hydrating(q, "hydration-unhydrated-count"),
        widget::timeseries(
            "hydration-queue-size",
            "Hydration Queue Size",
            &q.single("materialize.compute.hydration.queue_size"),
            &[],
            &Look::new().min_zero(),
        ),
        widget::ranking(
            "hydration-slowest-collections",
            "Slowest Hydrating Objects",
            &q.single("materialize.compute.hydration.slowest_collections"),
            &Look::new().reduce(Reduce::Max).format(Format::Seconds),
            15,
        ),
    ];
    let dataflows = vec![
        widget::timeseries(
            "dataflow-count",
            "Dataflow Count",
            &q.single("materialize.compute.dataflows.count"),
            &[],
            &Look::new().min_zero(),
        ),
        widget::timeseries(
            "dataflow-count-by-worker",
            "Dataflow Count (per worker)",
            &q.single("materialize.compute.dataflows.count_by_worker"),
            &[],
            &Look::new().min_zero(),
        ),
        widget::timeseries(
            "dataflow-elapsed-rate",
            "Dataflow Elapsed Rate",
            &q.single("materialize.compute.dataflows.elapsed_rate"),
            &[],
            &Look::new().log_scale(),
        ),
    ];
    let records = |slug, title, id| {
        widget::table(
            slug,
            title,
            &q.single(id),
            &[
                Column::of("Min", Reduce::Min),
                Column::of("Max", Reduce::Max),
                Column::of("Last", Reduce::Last),
            ],
            &Look::new(),
        )
    };
    let arrangements = vec![
        widget::timeseries(
            "arrangement-rate",
            "Arrangement Maintenance Rate",
            &q.single("materialize.compute.arrangements.maintenance_rate"),
            &[],
            &Look::new(),
        ),
        widget::timeseries(
            "arrangement-rate-by-worker",
            "Arrangement Maintenance Rate (per worker)",
            &q.single("materialize.compute.arrangements.maintenance_rate_by_worker"),
            &[],
            &Look::new().format(Format::Fraction),
        ),
        records(
            "arrangement-records-system",
            "System Objects — Record Counts",
            "materialize.compute.arrangements.records.system",
        ),
        records(
            "arrangement-records-user",
            "User Objects — Record Counts",
            "materialize.compute.arrangements.records.user",
        ),
        records(
            "arrangement-records-transient",
            "Transient / Uncategorized — Record Counts",
            "materialize.compute.arrangements.records.transient",
        ),
    ];
    vec![
        Section::new("Compute Objects", color::COMPUTE, summary),
        Section::new("Freshness", color::COMPUTE, freshness),
        Section::new("Hydration", color::COMPUTE, hydration),
        Section::new("Dataflows", color::COMPUTE, dataflows),
        Section::new("Arrangements", color::COMPUTE, arrangements),
    ]
}

/// The Sources and Sinks tab: what flows in, and what flows out.
fn sources_sinks(q: &Queries) -> Vec<Section> {
    let count = |slug, title, id| widget::query_value(slug, title, &q.single(id), &Look::new());
    let line =
        |slug, title, id, look: Look| widget::timeseries(slug, title, &q.single(id), &[], &look);
    let lines = |slug, title, id, labels: &[&str], look: Look| {
        widget::timeseries(slug, title, &q.labeled(id, labels), labels, &look)
    };
    let summary = vec![
        count(
            "storage-active-sources",
            "Active Sources",
            "materialize.storage.sources.count",
        ),
        count(
            "storage-active-sinks",
            "Active Sinks",
            "materialize.storage.sinks.count",
        ),
        count(
            "storage-active-tables",
            "Active Tables",
            "materialize.storage.tables.count",
        ),
    ];
    let sources = vec![
        line(
            "sources-types",
            "Source Types",
            "materialize.storage.sources.by_type",
            Look::new().min_zero(),
        ),
        widget::table(
            "sources-status-table",
            "Sources",
            &q.single("materialize.storage.sources.catalog"),
            &[Column::of("Objects", Reduce::Last)],
            &Look::new(),
        ),
        line(
            "sources-bytes-received-rate",
            "Source Bytes Received (rate)",
            "materialize.storage.sources.bytes_received",
            Look::new().format(Format::BytesPerSecond).log_scale(),
        ),
        line(
            "sources-ingestion-by-replica",
            "Source Ingestion by Replica",
            "materialize.storage.sources.ingestion_by_replica",
            Look::new().format(Format::PerSecond("message")).min_zero(),
        ),
        lines(
            "sources-errors",
            "Source Upstream Errors",
            "materialize.storage.sources.upstream_errors",
            &["commit failures", "disconnected"],
            Look::new().min_zero(),
        ),
    ];
    let sinks = vec![
        widget::pie(
            "sinks-types",
            "Sink Types",
            &q.single("materialize.storage.sinks.by_type"),
            &Look::new(),
        ),
        line(
            "sinks-throughput",
            "Sink Throughput (committed)",
            "materialize.storage.sinks.throughput",
            Look::new().format(Format::BytesPerSecond).log_scale(),
        ),
        line(
            "sinks-lag",
            "Sink Lag (staged minus committed)",
            "materialize.storage.sinks.lag",
            Look::new().format(Format::Bytes).min_zero(),
        ),
    ];
    let iceberg = vec![
        lines(
            "sinks-iceberg-commit-latency",
            "Iceberg Commit Latency (p50 / p90 / p99)",
            "materialize.storage.sinks.iceberg.commit_latency",
            &["p50", "p90", "p99"],
            Look::new().format(Format::Seconds).log_scale(),
        ),
        lines(
            "sinks-iceberg-failures",
            "Iceberg Commit Failures & Conflicts",
            "materialize.storage.sinks.iceberg.commit_failures",
            &["failures", "conflicts"],
            Look::new().format(Format::PerSecond("error")).min_zero(),
        ),
        lines(
            "sinks-iceberg-files",
            "Iceberg File & Snapshot Rate",
            "materialize.storage.sinks.iceberg.file_rate",
            &["data", "deletes", "snapshots"],
            Look::new().format(Format::PerSecond("file")).min_zero(),
        ),
    ];
    let kafka = vec![
        line(
            "sinks-kafka-tx-errors",
            "Kafka TX Error Rate",
            "materialize.storage.sinks.kafka.tx_errors",
            Look::new().format(Format::PerSecond("error")).min_zero(),
        ),
        line(
            "sinks-kafka-outbuf",
            "Kafka Output Buffer (messages)",
            "materialize.storage.sinks.kafka.output_buffer",
            Look::new().min_zero(),
        ),
        lines(
            "sinks-kafka-connects",
            "Kafka Connect / Disconnect Rate",
            "materialize.storage.sinks.kafka.connect_rate",
            &["connects", "disconnects"],
            Look::new()
                .format(Format::PerSecond("connection"))
                .min_zero(),
        ),
    ];
    vec![
        Section::new("Sources and Sinks", color::SOURCES_SINKS, summary),
        Section::new("Sources", color::SOURCES_SINKS, sources),
        Section::new("Sinks", color::SOURCES_SINKS, sinks),
        Section::new("Iceberg Sinks", color::SOURCES_SINKS, iceberg),
        Section::new("Kafka Sinks", color::SOURCES_SINKS, kafka),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::queries::test_registry;

    #[test]
    fn every_slug_is_placed_once() {
        let manifest = manifest(&Options::default(), test_registry()).expect("build");
        let mut slugs: Vec<&str> = manifest.iter().map(|p| p.slug.as_str()).collect();
        let count = slugs.len();
        slugs.sort_unstable();
        slugs.dedup();
        assert_eq!(slugs.len(), count, "a slug is placed twice");
    }

    #[test]
    fn no_listed_difference_names_a_placed_panel_as_missing() {
        let manifest = manifest(&Options::default(), test_registry()).expect("build");
        for difference in DIFFERENCES.iter().filter(|d| d.gap == Gap::Missing) {
            assert!(
                manifest.iter().all(|p| p.slug != difference.slug),
                "{} is listed as missing and is placed",
                difference.slug
            );
        }
    }
}
