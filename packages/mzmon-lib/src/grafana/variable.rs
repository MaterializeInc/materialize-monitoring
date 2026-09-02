// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Dashboard variables. Port of `dashboards.variables`.
//!
//! The set here is what [`crate::grafana::context`] requires plus the two
//! controls that shape them, chained so each narrows the last: pick an
//! environment, and the namespace / cluster / replica lists follow from it.
//!
//! Two conventions carried over, both documented in
//! [`crate::grafana::context::variables`]:
//!
//! * A `*List` name means the value is always interpolated as a *pattern* —
//!   `instance_id=~"$mzClusterList"`, never `=`. A single-select variable still
//!   renders as an alternation when "All" is chosen, and `=` against `a|b` matches
//!   nothing, silently.
//! * `Name` versus `Id` records which identifier is stable. [`environments`] holds
//!   organization *names* because self-managed Materialize has no organization id;
//!   [`clusters`] holds cluster *ids* with the name as display text, because
//!   cluster names are neither stable nor unique.
//!
//! Discovery queries read `mz_compute_commands_total` — genuine instrumentation
//! present in every deployment, so never SQL-prefixed.
//!
//! [`environment_scoped`] is that funnel. [`operator_scoped`] extends it for a
//! dashboard that also watches the operator reconciling those environments, which
//! needs two things the funnel has no place for: a second datasource, because
//! Kubernetes events are logs rather than metrics, and a namespace for the
//! operator itself, which is a cluster-wide singleton and so sits *outside* the
//! funnel rather than inside it.

use crate::grafana::context::variables;
use crate::grafana::generated::dashboardv2;
use crate::grafana::query::{
    LOGS_DATASOURCE_VAR, METRICS_DATASOURCE_VAR, logql_variable_query, promql_data_query,
};

/// The info metric the discovery queries read.
///
/// Genuine instrumentation (bare `mz_` in every deployment), so it is *not*
/// SQL-prefixed — unlike [`CLUSTER_STATUS_METRIC`].
const INFO_METRIC: &str = "mz_compute_commands_total";

/// The operator metric [`operator_namespace`] discovers from.
///
/// `orchestratord_is_leader` rather than `up{job=...}`: the `up` series carries
/// the operator's namespace only inside a `job` label that pins the *monitoring*
/// namespace's naming convention, which this repo deliberately does not require of
/// a deployment. A metric the operator itself exports carries `namespace`
/// directly, and every replica exports this one — the follower reports 0, not
/// nothing, so discovery does not depend on which replica holds the lease.
const OPERATOR_METRIC: &str = "orchestratord_is_leader";

/// The SQL-derived metric [`clusters`] reads, which *is* prefixed.
const CLUSTER_STATUS_SUFFIX: &str = "compute_cluster_status";

/// Grafana plugin id for the metrics datasource.
const PROMETHEUS_PLUGIN: &str = "prometheus";

/// Grafana plugin id for the logs datasource.
const LOKI_PLUGIN: &str = "loki";

/// Namespaces a Materialize deployment conventionally occupies.
///
/// Three conventions rather than one, because the naming differs by how the
/// deployment was installed: `materialize` / `materialize-environment` from this
/// repo's own charts, `mz-…` from the shorter-prefixed installs, and
/// `environment-<uuid>-0` from Materialize Cloud.
///
/// Used as a namespace picker's **opening selection**, not as a filter on what it
/// offers -- on a representative install the difference is 838 lines an hour
/// against 30,432, most of the remainder being `kube-system`. The distinction is the whole point: a gate would hide the
/// monitoring stack and `kube-system`, which are what you need when collection
/// itself is failing, whereas a default merely means you do not start buried in
/// them. Every namespace stays one click away, and an install matching none of
/// these conventions picks its own from a list that was never narrowed.
const MATERIALIZE_NAMESPACE_PATTERN: &str = ".*materialize.*|mz-.*|environment-.*";

/// Extra controls the baseline defines beyond what the render context requires.
pub mod extra {
    /// Whether cluster discovery includes Materialize's own system clusters.
    pub const INCLUDE_SYSTEM_CLUSTERS: &str = "includeSystemClusters";
    /// Free-form label filters applied to every metrics query.
    pub const METRIC_ADHOC: &str = "metricAdhoc";
    /// Free-form label filters applied to every logs query.
    pub const LOGS_ADHOC: &str = "logsAdhoc";
    /// Free-text line filter applied to every logs query.
    pub const LOG_SEARCH: &str = "logSearch";
    /// Whether an infrastructure view subtracts the Materialize namespaces.
    pub const EXCLUDE_MATERIALIZE: &str = "excludeMaterialize";
    /// The node a node-detail dashboard is scoped to, by Kubernetes node name.
    pub const NODE: &str = "node";
}

/// An empty current selection.
///
/// Grafana fills this in on load from the variable's own query; authoring a value
/// would just pin a stale selection into the dashboard.
fn no_selection() -> dashboardv2::VariableOption {
    dashboardv2::VariableOption {
        selected: None,
        text: dashboardv2::VariableOptionText::String(String::new()),
        value: dashboardv2::VariableOptionValue::String(String::new()),
        properties: Default::default(),
    }
}

/// Shared skeleton for the query variables, which differ only in a few fields.
struct QueryVariable {
    name: &'static str,
    label: &'static str,
    description: &'static str,
    expr: String,
    multi: bool,
    include_all: bool,
    all_value: Option<&'static str>,
    hide: dashboardv2::VariableHide,
    sort: dashboardv2::VariableSort,
    skip_url_sync: bool,
    /// Named-capture regex extracting `value` and `text` from a `query_result`.
    regex: String,
}

impl QueryVariable {
    fn build(self) -> dashboardv2::VariableKind {
        dashboardv2::VariableKind::QueryVariableKind(dashboardv2::QueryVariableKind {
            kind: "QueryVariable".to_string(),
            spec: dashboardv2::QueryVariableSpec {
                name: self.name.to_string(),
                label: Some(self.label.to_string()),
                description: Some(self.description.to_string()),
                // Grafana shows `definition` in the variable editor; it has to
                // repeat the query rather than being derived from it.
                definition: Some(self.expr.clone()),
                query: promql_data_query(&self.expr, METRICS_DATASOURCE_VAR, None),
                multi: self.multi,
                include_all: self.include_all,
                all_value: self.all_value.map(str::to_string),
                hide: self.hide,
                sort: self.sort,
                skip_url_sync: self.skip_url_sync,
                regex: self.regex,
                regex_apply_to: Some(dashboardv2::VariableRegexApplyTo::Value),
                // Custom values let an operator name an environment or cluster the
                // discovery query has not seen yet -- a new environment, or one
                // whose metrics have aged out of the window.
                allow_custom_value: true,
                // `Never`: these are chained, and refreshing on every time-range
                // change re-runs four queries for no new information.
                refresh: dashboardv2::VariableRefresh::Never,
                current: no_selection(),
                options: Vec::new(),
                static_options: Vec::new(),
                static_options_order: None,
                placeholder: None,
                origin: None,
            },
        })
    }
}

/// The metrics datasource, which every other variable's query runs against.
pub fn metrics_datasource() -> dashboardv2::VariableKind {
    dashboardv2::VariableKind::DatasourceVariableKind(dashboardv2::DatasourceVariableKind {
        kind: "DatasourceVariable".to_string(),
        spec: dashboardv2::DatasourceVariableSpec {
            name: METRICS_DATASOURCE_VAR.to_string(),
            label: Some("Metrics Datasource".to_string()),
            description: Some("Datasource for metrics queries".to_string()),
            plugin_id: PROMETHEUS_PLUGIN.to_string(),
            multi: false,
            include_all: false,
            all_value: None,
            allow_custom_value: false,
            hide: dashboardv2::VariableHide::DontHide,
            refresh: dashboardv2::VariableRefresh::Never,
            regex: String::new(),
            skip_url_sync: false,
            current: no_selection(),
            options: Vec::new(),
            origin: None,
        },
    })
}

/// The logs datasource, which every LogQL panel queries.
///
/// Separate from [`metrics_datasource`] rather than one datasource variable with
/// two uses: Grafana resolves a `DatasourceVariable` against a *plugin id*, so one
/// variable cannot offer both a Prometheus and a Loki datasource. A dashboard that
/// mixes engines therefore defines both, and each dataquery names the one matching
/// its engine — see [`crate::grafana::query::data_query`].
pub fn logs_datasource() -> dashboardv2::VariableKind {
    dashboardv2::VariableKind::DatasourceVariableKind(dashboardv2::DatasourceVariableKind {
        kind: "DatasourceVariable".to_string(),
        spec: dashboardv2::DatasourceVariableSpec {
            name: LOGS_DATASOURCE_VAR.to_string(),
            label: Some("Logs Datasource".to_string()),
            description: Some("Datasource for log and event queries".to_string()),
            plugin_id: LOKI_PLUGIN.to_string(),
            multi: false,
            include_all: false,
            all_value: None,
            allow_custom_value: false,
            hide: dashboardv2::VariableHide::DontHide,
            refresh: dashboardv2::VariableRefresh::Never,
            regex: String::new(),
            skip_url_sync: false,
            current: no_selection(),
            options: Vec::new(),
            origin: None,
        },
    })
}

/// Environment selector, by organization *name*.
///
/// **Single-select on purpose.** Object and cluster ids are only unique *within*
/// one environment, and both the registry's aggregations and
/// [`crate::query::enrich`]'s joins match on those ids alone — so selecting two
/// environments can merge unrelated series, or break the enrichment join with a
/// duplicate id on its right-hand side. Multi-select becomes safe once the
/// organization label survives those aggregations and is part of the enrichment
/// join keys; until then this deliberately does not offer it.
///
/// Queries must still treat the value as a *pattern* — hence the `*List` name —
/// because a custom value interpolates as an alternation.
pub fn environments() -> dashboardv2::VariableKind {
    QueryVariable {
        name: variables::ENVIRONMENT_NAME_LIST,
        label: "Environment",
        description: "The current environment to view",
        expr: format!("label_values({INFO_METRIC}, materialize_cloud_organization_name)"),
        multi: false,
        include_all: false,
        all_value: Some(".*"),
        hide: dashboardv2::VariableHide::DontHide,
        sort: dashboardv2::VariableSort::AlphabeticalAsc,
        skip_url_sync: false,
        regex: String::new(),
    }
    .build()
}

/// Namespaces the selected environments live in.
///
/// Hidden: it is derived from the environment selection, so exposing it invites
/// an operator to desync the two. `materialize_cloud_organization_namespace` is
/// the only namespace label present in both deployments — self-managed carries
/// `kubernetes_namespace`, Cloud carries `namespace`.
pub fn namespaces() -> dashboardv2::VariableKind {
    QueryVariable {
        name: variables::MZ_NAMESPACE_LIST,
        label: "Materialize Namespace",
        description: "The current materialize namespace where environments live",
        expr: format!(
            "label_values({INFO_METRIC}{{ {} }}, materialize_cloud_organization_namespace)",
            environment_selector()
        ),
        multi: true,
        include_all: true,
        all_value: None,
        hide: dashboardv2::VariableHide::HideVariable,
        sort: dashboardv2::VariableSort::AlphabeticalAsc,
        // Derived, so keeping it out of the URL avoids permalinks that pin a
        // namespace inconsistent with their own environment.
        skip_url_sync: true,
        regex: String::new(),
    }
    .build()
}

/// Toggle for whether [`clusters`] includes Materialize's system clusters.
///
/// The values are regexes spliced into the cluster query: system cluster ids
/// start with `s`, so excluding them is `^[^s].*`.
pub fn include_system_clusters() -> dashboardv2::VariableKind {
    dashboardv2::VariableKind::SwitchVariableKind(dashboardv2::SwitchVariableKind {
        kind: "SwitchVariable".to_string(),
        spec: dashboardv2::SwitchVariableSpec {
            name: extra::INCLUDE_SYSTEM_CLUSTERS.to_string(),
            label: Some("Include System Clusters".to_string()),
            description: Some(
                "Whether to include materialize system clusters in the cluster list.".to_string(),
            ),
            enabled_value: ".*".to_string(),
            disabled_value: "^[^s].*".to_string(),
            current: ".*".to_string(),
            hide: dashboardv2::VariableHide::InControlsMenu,
            skip_url_sync: true,
            origin: None,
        },
    })
}

/// Cluster selector within the selected environments.
///
/// Reads the SQL-derived `compute_cluster_status`, so `sql_metric_prefix` must
/// match what the panels use (`mz_` self-managed, `v2_mz_` Cloud).
///
/// Uses `query_result` rather than `label_values` so both the id and the name are
/// available: the regex puts `compute_cluster_id` in the value and
/// `compute_cluster_name` in the display text. Grafana sorts labels
/// alphabetically, and `compute_cluster_id` sorts before `compute_cluster_name`,
/// which is what makes that regex stable.
pub fn clusters(sql_metric_prefix: &str) -> dashboardv2::VariableKind {
    let metric = format!("{sql_metric_prefix}{CLUSTER_STATUS_SUFFIX}");
    QueryVariable {
        name: variables::MZ_CLUSTER_LIST,
        label: "Cluster",
        description: "The cluster within the current environment to filter to",
        expr: format!(
            "query_result({metric}{{{}, compute_cluster_id=~\"${}\" }})",
            environment_selector(),
            extra::INCLUDE_SYSTEM_CLUSTERS
        ),
        multi: true,
        include_all: true,
        all_value: None,
        hide: dashboardv2::VariableHide::InControlsMenu,
        // Natural, so u2 sorts before u11.
        sort: dashboardv2::VariableSort::NaturalAsc,
        skip_url_sync: false,
        regex: r#".*compute_cluster_id=\"(?<value>[^\"]+)\",.*compute_cluster_name=\"(?<text>[^\"]+)\",.*"#
            .to_string(),
    }
    .build()
}

/// Replica selector within the selected clusters.
///
/// Replica *ids*, not names: names are almost always `r1`, which says nothing
/// across clusters.
pub fn replicas() -> dashboardv2::VariableKind {
    QueryVariable {
        name: variables::MZ_REPLICA_LIST,
        label: "Replica",
        description: "The replica within the current cluster to filter to",
        expr: format!(
            "label_values({INFO_METRIC}{{{}, instance_id=~\"${}\"}}, replica_id)",
            environment_selector(),
            variables::MZ_CLUSTER_LIST
        ),
        multi: true,
        include_all: true,
        all_value: Some(".*"),
        hide: dashboardv2::VariableHide::InControlsMenu,
        sort: dashboardv2::VariableSort::NaturalAsc,
        skip_url_sync: false,
        regex: String::new(),
    }
    .build()
}

/// Namespace the Materialize operator runs in.
///
/// Not part of the environment funnel, and deliberately outside it: the operator
/// is a cluster-wide singleton that reconciles every environment, so narrowing it
/// by the selected environment would be backwards. It is the one namespace control
/// an operator sets directly, which is why it is visible where
/// [`namespaces`] is hidden.
///
/// **Single-select, hence no `*List` suffix.** One operator deployment reconciles
/// the cluster; a second is a migration in progress rather than a steady state, and
/// merging two operators' events into one feed would misattribute every line. The
/// value still reaches a `=~` matcher, which a bare value satisfies.
///
/// Discovery reads [`OPERATOR_METRIC`]. Custom values are allowed, so an operator
/// whose metrics are down — the case this dashboard exists for — can still be named
/// by hand.
pub fn operator_namespace() -> dashboardv2::VariableKind {
    QueryVariable {
        name: variables::OPERATOR_NAMESPACE,
        label: "Operator Namespace",
        description: "The namespace the Materialize operator (orchestratord) runs in",
        expr: format!("label_values({OPERATOR_METRIC}, namespace)"),
        multi: false,
        include_all: false,
        all_value: None,
        hide: dashboardv2::VariableHide::DontHide,
        sort: dashboardv2::VariableSort::AlphabeticalAsc,
        skip_url_sync: false,
        regex: String::new(),
    }
    .build()
}

/// Deployment generations of the selected environment, for blue/green.
///
/// A rollout stands a *new* generation of `environmentd` and its replicas up
/// beside the old one, rehydrates it, and only then promotes. Both generations
/// are live and scraped at once, which is the whole reason this control exists:
/// without it their series are summed together and the question the rollout
/// actually poses — is the new side caught up yet — cannot be asked.
///
/// **The generation is not a label on anything.** orchestratord records it as an
/// *annotation*, which no exporter surfaces. What it does reach is the object
/// *name*, in two shapes: `…-environmentd-<generation>-<ordinal>` for environmentd
/// and `…-gen-<generation>-<ordinal>` for a replica. Discovery therefore reads
/// pod names and captures the number out of one; the filters in
/// [`crate::grafana::context`] match both.
///
/// Refreshed on time-range change, unlike every other variable here. The set of
/// generations is a property of the *window* — an old generation is torn down
/// after promotion, so widening the range to cover a rollout is exactly how its
/// other side comes into view, and a variable that never refreshed would go on
/// offering only the generation that happened to be live at load.
pub fn generations() -> dashboardv2::VariableKind {
    let mut variable = QueryVariable {
        name: variables::MZ_GENERATION_LIST,
        label: "Generation",
        description: "The deployment generation(s) to filter to, for blue/green rollouts",
        expr: format!(
            "label_values({INFO_METRIC}{{{}}}, pod)",
            environment_selector()
        ),
        multi: true,
        include_all: true,
        // No `all_value`: on "All" Grafana expands to the discovered generations,
        // which under `:regex` is the alternation the filters splice in. A literal
        // like `[0-9]+` would be regex-*escaped* by that same format and match
        // nothing.
        all_value: None,
        hide: dashboardv2::VariableHide::DontHide,
        // Newest generation first: during a rollout the one being asked about is
        // the one that just appeared.
        sort: dashboardv2::VariableSort::NumericalDesc,
        skip_url_sync: false,
        regex: r"/.*-environmentd-(?<value>[0-9]+)-[0-9]+/".to_string(),
    }
    .build();
    if let dashboardv2::VariableKind::QueryVariableKind(v) = &mut variable {
        v.spec.refresh = dashboardv2::VariableRefresh::OnTimeRangeChanged;
    }
    variable
}

/// Shared skeleton for the Loki-backed query variables.
///
/// Separate from [`QueryVariable`] because the two datasources answer a variable
/// differently: Prometheus takes `label_values(...)` text, Loki takes a
/// `{label, stream}` object. See [`logql_variable_query`].
struct LogQueryVariable {
    name: &'static str,
    label: &'static str,
    description: &'static str,
    /// The Loki label to enumerate.
    loki_label: &'static str,
    /// Stream selector narrowing which streams are consulted. May reference other
    /// variables, which is what chains these together.
    stream: String,
    /// What "All" interpolates to.
    ///
    /// Always stated rather than left to expand into the discovered values. Two
    /// reasons, both learned the hard way: an expansion is empty when discovery
    /// has not run or has failed, and `label=~""` matches only streams *missing*
    /// that label rather than all of them — so a picker that fails to load takes
    /// the panel with it. And the choice between `.*` and `.+` is a real one per
    /// label, not a default: see [`log_apps`].
    all_value: &'static str,
    /// The selection the picker opens on, as a custom value.
    ///
    /// `None` leaves it to Grafana, which picks "All". A value here is a
    /// *default*, not a constraint — the picker still lists everything discovery
    /// found, and `allow_custom_value` is what lets a pattern stand in for a
    /// value that is not one of them.
    current: Option<&'static str>,
    hide: dashboardv2::VariableHide,
}

impl LogQueryVariable {
    fn build(self) -> dashboardv2::VariableKind {
        dashboardv2::VariableKind::QueryVariableKind(dashboardv2::QueryVariableKind {
            kind: "QueryVariable".to_string(),
            spec: dashboardv2::QueryVariableSpec {
                name: self.name.to_string(),
                label: Some(self.label.to_string()),
                description: Some(self.description.to_string()),
                definition: Some(format!("label_values({})", self.loki_label)),
                query: logql_variable_query(self.loki_label, &self.stream, LOGS_DATASOURCE_VAR),
                multi: true,
                include_all: true,
                all_value: Some(self.all_value.to_string()),
                hide: self.hide,
                sort: dashboardv2::VariableSort::AlphabeticalAsc,
                skip_url_sync: false,
                regex: String::new(),
                regex_apply_to: Some(dashboardv2::VariableRegexApplyTo::Value),
                allow_custom_value: true,
                // Log labels come and go with the workloads emitting them, so the
                // list is a property of the window rather than of the deployment.
                refresh: dashboardv2::VariableRefresh::OnTimeRangeChanged,
                current: match self.current {
                    // Arrays, because the variable is multi-select: that is the
                    // shape Grafana writes and reads back for one.
                    Some(value) => dashboardv2::VariableOption {
                        selected: Some(true),
                        text: dashboardv2::VariableOptionText::Array(vec![value.to_string()]),
                        value: dashboardv2::VariableOptionValue::Array(vec![value.to_string()]),
                        properties: Default::default(),
                    },
                    None => no_selection(),
                },
                options: Vec::new(),
                static_options: Vec::new(),
                static_options_order: None,
                placeholder: None,
                origin: None,
            },
        })
    }
}

/// Namespaces that have logs.
///
/// Discovered with **no stream selector at all**, which is Loki's "every value of
/// this label" mode. That is not just simpler than selecting on something — it is
/// the one form that cannot trip the empty-matcher rule, since there is no
/// selector to be empty-compatible.
/// `opens_on` is the selection the picker starts with, as a custom value. The
/// two dashboards that define this variable differ in exactly that: a
/// Materialize dashboard opens on the deployment's own namespaces, an
/// infrastructure one on all of them. Same variable, same name, so the queries
/// behind both are one set of definitions.
pub fn log_namespaces(opens_on: &'static str) -> dashboardv2::VariableKind {
    LogQueryVariable {
        name: variables::LOG_NAMESPACE_LIST,
        label: "Namespace",
        description: "The namespace(s) to read logs from",
        loki_label: "namespace",
        stream: String::new(),
        current: Some(opens_on),
        // `.+`, not `.*`: this value becomes the sole matcher of the app, level
        // and job discovery selectors, so an empty-compatible one would stop
        // those loading. Safe because every line carries a namespace — the
        // collection pipeline coerces cluster-scoped events to `kube-system`
        // rather than leaving the label off.
        all_value: ".+",
        hide: dashboardv2::VariableHide::DontHide,
    }
    .build()
}

/// Applications emitting logs in the selected namespaces.
pub fn log_apps() -> dashboardv2::VariableKind {
    LogQueryVariable {
        name: variables::LOG_APP_LIST,
        label: "App",
        description: "The application(s) to read logs from",
        loki_label: "app",
        stream: format!(r#"{{namespace=~"${}"}}"#, variables::LOG_NAMESPACE_LIST),
        current: None,
        // `.*`, deliberately. `app` is the one label here that is genuinely
        // absent from some streams, and `.+` silently drops them — on a
        // representative install that is 2,407 of 30,432 lines in half an hour,
        // most of `kube-system`. "All" has to mean all.
        all_value: ".*",
        hide: dashboardv2::VariableHide::DontHide,
    }
    .build()
}

/// Severity levels present in the selected namespaces.
///
/// Discovered rather than hard-coded: the pipeline normalizes what it can and
/// falls back to `UNKNOWN`, so which levels exist is a property of the workloads
/// running, not a fixed vocabulary.
pub fn log_levels() -> dashboardv2::VariableKind {
    LogQueryVariable {
        name: variables::LOG_LEVEL_LIST,
        label: "Level",
        description: "The severity level(s) to include",
        loki_label: "level",
        stream: format!(r#"{{namespace=~"${}"}}"#, variables::LOG_NAMESPACE_LIST),
        current: None,
        // `.*` for the same reason as `app`, even though every line carries a
        // level today: the inclusive form costs nothing and does not depend on
        // that staying true.
        all_value: ".*",
        hide: dashboardv2::VariableHide::DontHide,
    }
    .build()
}

/// Collection jobs, and the matcher that keeps every log selector parseable.
///
/// Two jobs in one. As a *filter* it is the most direct way to isolate a single
/// workload's logs, since `job` is `<namespace>/<container>` for container logs
/// and names the source outright for everything else.
///
/// As a *matcher* it is load-bearing. LogQL rejects a stream selector in which
/// every matcher can match the empty string — "queries require at least one
/// regexp or equality matcher that does not have an empty-compatible value" — and
/// a dashboard whose pickers are all `=~` is exactly that shape. `all_value` is
/// therefore `.+` rather than the discovered values: it always matches something
/// non-empty, so the selector parses even when every other picker is on "All" or
/// has discovered nothing at all. Without it the panels error rather than coming
/// back empty.
///
/// Only the *log* queries need it. The event queries pin
/// `job="loki.source.kubernetes_events"`, which is already a non-empty equality
/// matcher, and adding a second `job` matcher there would AND the two and zero
/// the panel the moment a container job was picked.
pub fn log_jobs() -> dashboardv2::VariableKind {
    LogQueryVariable {
        name: variables::LOG_JOB_LIST,
        label: "Job",
        description: "The collection job(s) to read logs from",
        loki_label: "job",
        stream: format!(r#"{{namespace=~"${}"}}"#, variables::LOG_NAMESPACE_LIST),
        current: None,
        // See the note above: this must never interpolate to something that can
        // match the empty string. Free here — `job` is present on every line, so
        // `.+` and `.*` select identically and only the parseability differs.
        all_value: ".+",
        hide: dashboardv2::VariableHide::InControlsMenu,
    }
    .build()
}

/// Sub-components within an application.
///
/// The dimension an infrastructure dashboard is built on and a Materialize one
/// has no use for: `loki` alone splits into `canary`, `querier`, `ingester`,
/// `query-frontend`, `index-gateway`, `compactor`, `distributor` and `ruler`, and
/// `thanos` into `compactor`, `receive` and `storegateway`. Without it, "why is
/// Loki slow" is a grep.
pub fn log_components() -> dashboardv2::VariableKind {
    LogQueryVariable {
        name: variables::LOG_COMPONENT_LIST,
        label: "Component",
        description: "The sub-component(s) to read logs from",
        loki_label: "component",
        stream: format!(r#"{{namespace=~"${}"}}"#, variables::LOG_NAMESPACE_LIST),
        current: None,
        // `.*`: plenty of workloads have no component at all, and `.+` would drop
        // them silently.
        all_value: ".*",
        hide: dashboardv2::VariableHide::DontHide,
    }
    .build()
}

/// Containers within the selected namespaces.
///
/// Carried because `app` is empty for much of the platform — every `kube-system`
/// workload on a representative install — so it is the only picker that reaches
/// `cilium-agent`, `kubedns`, `konnectivity-agent` and the rest. In the controls
/// menu rather than the main row: most readings never need it, but when they do
/// nothing else will serve.
pub fn log_containers() -> dashboardv2::VariableKind {
    LogQueryVariable {
        name: variables::LOG_CONTAINER_LIST,
        label: "Container",
        description: "The container(s) to read logs from",
        loki_label: "container",
        stream: format!(r#"{{namespace=~"${}"}}"#, variables::LOG_NAMESPACE_LIST),
        current: None,
        all_value: ".*",
        hide: dashboardv2::VariableHide::InControlsMenu,
    }
    .build()
}

/// systemd units, for the node journal.
///
/// A separate axis rather than another filter on the same one. Journal lines
/// carry `unit`, `component`, `job`, `level` and `service_name` — and **no
/// `namespace`, `app` or `container`** — because they come from the node rather
/// than from a pod. That is why they are unreachable from any dashboard whose
/// selectors require a namespace, and why they get a tab of their own.
pub fn log_units() -> dashboardv2::VariableKind {
    LogQueryVariable {
        name: variables::LOG_UNIT_LIST,
        label: "Unit",
        description: "The systemd unit(s) to read node journal logs from",
        loki_label: "unit",
        // No selector: `unit` streams carry no namespace, so chaining this off the
        // namespace picker would discover nothing.
        stream: String::new(),
        current: None,
        // The anchor for every journal selector, the way `job` is for container
        // logs -- a journal query has no namespace matcher to lean on.
        all_value: ".+",
        hide: dashboardv2::VariableHide::DontHide,
    }
    .build()
}

/// The node a node-detail dashboard is about, as Kubernetes names it.
///
/// Discovered from `kube_node_info` rather than from node-exporter, because
/// kube-state-metrics is the family that knows the node by *name*. Single-select
/// and no "All": this dashboard answers questions about one machine, and every
/// panel on it would be ambiguous averaged across several.
pub fn node() -> dashboardv2::VariableKind {
    QueryVariable {
        name: extra::NODE,
        label: "Node",
        description: "The node this dashboard describes",
        expr: "label_values(kube_node_info, node)".to_string(),
        multi: false,
        include_all: false,
        all_value: None,
        hide: dashboardv2::VariableHide::DontHide,
        sort: dashboardv2::VariableSort::AlphabeticalAsc,
        skip_url_sync: false,
        regex: String::new(),
    }
    .build()
}

/// The selected node's node-exporter address, derived from [`node`].
///
/// **Hidden, and chained rather than chosen.** The two families that describe a
/// node do not agree on how to name one: kube-state-metrics labels it `node`
/// with the Kubernetes name, while node-exporter labels it `instance` with
/// `<ip>:9100` and carries the name only as `nodename` on `node_uname_info`.
/// That metric is therefore the join, and this variable is that join expressed
/// as a lookup — the operator picks a name, and this resolves the address.
///
/// The name is `nodeList` because the node-exporter query families write
/// `instance=~"$nodeList"` literally, 220 times across `node-health.yaml` and
/// `node-debug.yaml` — a convention inherited from the Node Exporter Full
/// dashboard. Keeping the name lets every one of those queries back this
/// dashboard unchanged, at the cost of a name that says "list" while holding one
/// address. A fleet view is exactly where the plural earns itself.
pub fn node_instance() -> dashboardv2::VariableKind {
    QueryVariable {
        name: variables::NODE_LIST,
        label: "Node Instance",
        description: "node-exporter address of the selected node (derived)",
        expr: format!(
            "label_values(node_uname_info{{nodename=\"${}\"}}, instance)",
            extra::NODE
        ),
        multi: false,
        include_all: false,
        all_value: None,
        // Hidden: it is a lookup, not a control. Showing it would offer the
        // operator a second node picker that must agree with the first.
        hide: dashboardv2::VariableHide::HideVariable,
        sort: dashboardv2::VariableSort::AlphabeticalAsc,
        skip_url_sync: false,
        regex: String::new(),
    }
    .build()
}

/// Whether the Materialize namespaces are excluded from an infrastructure view.
///
/// **On by default**, which is the point: an infrastructure dashboard opens on
/// every namespace, and the deployment is about half of everything written. On a
/// reference install the Materialize namespaces are 49.9% of a week's lines, and
/// `materialize-environment` on its own is the single largest namespace — ahead
/// of the monitoring stack that ships this dashboard. Left in, it dominates every
/// volume panel and buries the platform's own signal, which is the one thing this
/// dashboard is for.
///
/// The two values are the same [`MATERIALIZE_NAMESPACE_PATTERN`] the Materialize
/// dashboard *opens on*, and a pattern that matches nothing. That symmetry is
/// deliberate: one dashboard selects the deployment, the other subtracts it, and
/// both agree on what "the deployment" means.
///
/// `a^` for the off position rather than an empty string. Empty would work only
/// because every container line happens to carry a namespace, and `!~""` reads as
/// "has a namespace" rather than as "exclude nothing"; `a^` is a pattern no value
/// can match, which is what is actually meant.
pub fn exclude_materialize() -> dashboardv2::VariableKind {
    dashboardv2::VariableKind::SwitchVariableKind(dashboardv2::SwitchVariableKind {
        kind: "SwitchVariable".to_string(),
        spec: dashboardv2::SwitchVariableSpec {
            name: extra::EXCLUDE_MATERIALIZE.to_string(),
            label: Some("Exclude Materialize".to_string()),
            description: Some(
                "Drop the Materialize namespaces, so the platform's own logs and events are not \
                 buried under the deployment's."
                    .to_string(),
            ),
            enabled_value: MATERIALIZE_NAMESPACE_PATTERN.to_string(),
            disabled_value: "a^".to_string(),
            current: MATERIALIZE_NAMESPACE_PATTERN.to_string(),
            hide: dashboardv2::VariableHide::DontHide,
            skip_url_sync: false,
            origin: None,
        },
    })
}

/// Free-text line filter applied to every log and event query.
///
/// Empty is the resting state and must stay harmless: the filter renders as
/// `|~ "(?i)$logSearch"`, and an empty pattern matches every line rather than
/// none. Verified against a live Loki, since the opposite would make the
/// dashboard blank until something is typed.
pub fn log_search() -> dashboardv2::VariableKind {
    dashboardv2::VariableKind::TextVariableKind(dashboardv2::TextVariableKind {
        kind: "TextVariable".to_string(),
        spec: dashboardv2::TextVariableSpec {
            name: extra::LOG_SEARCH.to_string(),
            label: Some("Search".to_string()),
            description: Some(
                "Case-insensitive text to match anywhere in the log line".to_string(),
            ),
            query: String::new(),
            current: no_selection(),
            hide: dashboardv2::VariableHide::DontHide,
            skip_url_sync: false,
            origin: None,
        },
    })
}

/// Free-form label filters applied to every metrics query.
///
/// Seeded with the namespace selector so an operator's ad-hoc filters compose
/// with the environment scope rather than escaping it.
pub fn metric_adhoc() -> dashboardv2::VariableKind {
    dashboardv2::VariableKind::AdhocVariableKind(dashboardv2::AdhocVariableKind {
        kind: "AdhocVariable".to_string(),
        // The ad-hoc filter picks its own label keys from the datasource, so it
        // carries a datasource ref of its own rather than borrowing a query's.
        datasource: Some(dashboardv2::AdhocVariableKindDatasource {
            name: Some(format!("${METRICS_DATASOURCE_VAR}")),
        }),
        // Empty in the baseline: `group` names a datasource *plugin* for
        // dataqueries, and an ad-hoc variable resolves its own through the
        // datasource ref above.
        group: String::new(),
        labels: Default::default(),
        spec: dashboardv2::AdhocVariableSpec {
            name: extra::METRIC_ADHOC.to_string(),
            label: Some("Advanced Metric Filter".to_string()),
            description: Some("Adhoc filters to apply to all metrics queries".to_string()),
            base_filters: vec![dashboardv2::AdHocFilterWithLabels {
                key: "namespace".to_string(),
                operator: "=~".to_string(),
                value: format!("${}", variables::MZ_NAMESPACE_LIST),
                // `FilterOrigin` is a newtype over the string Grafana expects.
                origin: Some(dashboardv2::FilterOrigin("dashboard".to_string())),
                condition: None,
                force_edit: None,
                key_label: None,
                values: Vec::new(),
                value_labels: Vec::new(),
            }],
            filters: Vec::new(),
            default_keys: Vec::new(),
            enable_group_by: false,
            allow_custom_value: true,
            hide: dashboardv2::VariableHide::InControlsMenu,
            skip_url_sync: false,
            origin: None,
        },
    })
}

/// The standard environment-scoped variable set, in dependency order.
///
/// Order matters to a reader, not to Grafana: each variable's query references
/// the one before it, so listing them in that order is what makes the chain
/// legible in the variable editor. It is also the order the controls appear in,
/// so the row reads as a narrowing funnel — environment, then namespace, then
/// cluster, then replica — with the system-cluster toggle sitting where it gates
/// the cluster list and the ad-hoc filter last, being a free-form escape hatch
/// rather than a step in the funnel.
///
/// This differs from the Python, which emitted the ad-hoc filter second because
/// it registered datasources before variables. That was an artifact of the call
/// order, not a choice about the controls row.
pub fn environment_scoped(sql_metric_prefix: &str) -> Vec<dashboardv2::VariableKind> {
    vec![
        metrics_datasource(),
        environments(),
        namespaces(),
        include_system_clusters(),
        clusters(sql_metric_prefix),
        replicas(),
        metric_adhoc(),
    ]
}

/// Free-form label filters applied to every logs query.
///
/// The counterpart to [`metric_adhoc`], on the logs datasource. Separate because
/// an ad-hoc variable resolves its label keys *from a datasource*, so one pointed
/// at Prometheus offers metric labels and cannot offer Loki's stream labels.
///
/// **No base filter**, unlike the metrics one. Grafana ANDs an ad-hoc filter into
/// the query's own selector, and the natural seed — the environment namespace —
/// would narrow a stream selector that deliberately spans two namespaces, silently
/// dropping every event the operator published. The log queries already carry
/// their own namespace scope; this is purely an escape hatch on top of it.
///
/// The keys on offer are Loki *stream* labels (`app`, `container`, `level`,
/// `namespace`, …). Structured metadata — `reason`, `kind`, `reportingcontroller`
/// on a Kubernetes event — is not a stream label, so it is filtered in the query
/// rather than here.
pub fn logs_adhoc() -> dashboardv2::VariableKind {
    dashboardv2::VariableKind::AdhocVariableKind(dashboardv2::AdhocVariableKind {
        kind: "AdhocVariable".to_string(),
        datasource: Some(dashboardv2::AdhocVariableKindDatasource {
            name: Some(format!("${LOGS_DATASOURCE_VAR}")),
        }),
        group: String::new(),
        labels: Default::default(),
        spec: dashboardv2::AdhocVariableSpec {
            name: extra::LOGS_ADHOC.to_string(),
            label: Some("Advanced Log Filter".to_string()),
            description: Some("Adhoc filters to apply to all logs queries".to_string()),
            base_filters: Vec::new(),
            filters: Vec::new(),
            default_keys: Vec::new(),
            enable_group_by: false,
            allow_custom_value: true,
            hide: dashboardv2::VariableHide::InControlsMenu,
            skip_url_sync: false,
            origin: None,
        },
    })
}

/// [`environment_scoped`], plus what a dashboard that also reads the operator and
/// its logs needs.
///
/// Three additions, in the order a reader meets them: the logs datasource beside
/// the metrics one, since a dashboard mixing engines picks both up front; and the
/// operator namespace at the head of the funnel, because it scopes a
/// cluster-wide singleton rather than narrowing the environment selection below
/// it.
///
/// A dashboard using this set must pair it with
/// [`DashboardScope::operator_variable`](crate::grafana::context::DashboardScope::operator_variable),
/// or the operator queries keep rendering the scope's pinned namespace and the
/// control does nothing.
pub fn operator_scoped(sql_metric_prefix: &str) -> Vec<dashboardv2::VariableKind> {
    let mut set = vec![
        metrics_datasource(),
        logs_datasource(),
        operator_namespace(),
    ];
    set.extend(
        environment_scoped(sql_metric_prefix)
            .into_iter()
            .filter(|v| name_of(v) != METRICS_DATASOURCE_VAR),
    );
    // After the environment funnel, whose selection it narrows, and before the
    // ad-hoc filters, which stay last as free-form escape hatches.
    let adhoc = set.len() - 1;
    set.insert(adhoc, generations());
    // Beside the metrics one, so the two escape hatches sit together at the end
    // of the controls row rather than one of them hiding mid-funnel.
    set.push(logs_adhoc());
    set
}

/// The variable set for a logs dashboard.
///
/// **Loki end to end, and deliberately disjoint from [`environment_scoped`].** A
/// logs dashboard is frequently the tool for working out why the *metrics*
/// pipeline is broken, so deriving its scope from Prometheus — as
/// `$mzNamespaceList` does — would make it depend on the thing being
/// investigated. Nothing here defines or reads a metrics variable, and the
/// dashboard needs no metrics datasource at all.
///
/// Ordered as a funnel like the others: what to look at, then how to narrow it,
/// with the free-text search and the ad-hoc filter last as escape hatches.
///
/// Unscoped by default. There was briefly a switch narrowing namespace discovery
/// to the ones a Materialize deployment conventionally occupies; it was dropped
/// because the naming it guessed at is not a fact, and because the namespace
/// picker on its own is a better answer than a filter in front of it.
pub fn logs_scoped() -> Vec<dashboardv2::VariableKind> {
    vec![
        logs_datasource(),
        log_namespaces(MATERIALIZE_NAMESPACE_PATTERN),
        log_apps(),
        log_levels(),
        log_jobs(),
        log_search(),
        logs_adhoc(),
    ]
}

/// The variable set for an infrastructure logs dashboard.
///
/// [`logs_scoped`] plus the three axes the platform needs and a Materialize
/// environment does not: `component` to tell one Loki or Thanos process from
/// another, `container` because `app` is empty across most of `kube-system`, and
/// `unit` for the node journal, which carries no namespace at all.
///
/// The variable *names* are shared with [`logs_scoped`] on purpose. The two
/// dashboards then differ only in which pickers they offer and where the
/// namespace one opens, so every query behind them is one set of definitions
/// rather than two that drift.
/// Controls for a single-node detail dashboard.
///
/// Mixed-datasource, like `env-upgrade`: the machine's measurements come from
/// Thanos and its own account of itself comes from Loki, so both datasource
/// variables are defined.
///
/// [`node`] is the only control that scopes the dashboard; [`node_instance`]
/// follows it invisibly. The log pickers narrow the journal feed *within* the
/// selected node and are the same ones `infra-logs` uses, so an operator moving
/// between the two dashboards meets the same controls.
pub fn node_scoped() -> Vec<dashboardv2::VariableKind> {
    vec![
        metrics_datasource(),
        logs_datasource(),
        node(),
        node_instance(),
        log_units(),
        log_levels(),
        log_search(),
        metric_adhoc(),
        logs_adhoc(),
    ]
}

pub fn logs_infra_scoped() -> Vec<dashboardv2::VariableKind> {
    vec![
        logs_datasource(),
        // Opens on everything: an infrastructure operator owns the cluster, and
        // the Materialize namespaces are a small part of what they are watching.
        log_namespaces(".+"),
        // Beside the namespace picker, since the two together are the scope.
        exclude_materialize(),
        log_apps(),
        log_components(),
        log_levels(),
        log_units(),
        log_containers(),
        log_jobs(),
        log_search(),
        logs_adhoc(),
    ]
}

/// The environment scope fragment the chained queries share.
fn environment_selector() -> String {
    format!(
        r#"materialize_cloud_organization_name=~"${}""#,
        variables::ENVIRONMENT_NAME_LIST
    )
}

/// The `name` of a variable, whatever its kind.
pub fn name_of(variable: &dashboardv2::VariableKind) -> &str {
    use dashboardv2::VariableKind as V;
    match variable {
        V::QueryVariableKind(v) => &v.spec.name,
        V::TextVariableKind(v) => &v.spec.name,
        V::ConstantVariableKind(v) => &v.spec.name,
        V::DatasourceVariableKind(v) => &v.spec.name,
        V::IntervalVariableKind(v) => &v.spec.name,
        V::CustomVariableKind(v) => &v.spec.name,
        V::GroupByVariableKind(v) => &v.spec.name,
        V::AdhocVariableKind(v) => &v.spec.name,
        V::SwitchVariableKind(v) => &v.spec.name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::context::REQUIRED_VARIABLES;

    fn names(variables: &[dashboardv2::VariableKind]) -> Vec<&str> {
        variables.iter().map(name_of).collect()
    }

    #[test]
    fn the_standard_set_defines_every_required_variable() {
        // The check that matters: an undefined variable interpolates to nothing
        // and every selector using it silently matches no series.
        let set = environment_scoped("mz_");
        let defined = names(&set);
        for required in REQUIRED_VARIABLES {
            assert!(
                defined.contains(required),
                "the standard set does not define ${required}"
            );
        }
    }

    #[test]
    fn the_standard_set_is_listed_in_dependency_order() {
        assert_eq!(
            names(&environment_scoped("mz_")),
            vec![
                "metricsDatasource",
                "environmentNameList",
                "mzNamespaceList",
                "includeSystemClusters",
                "mzClusterList",
                "mzReplicaList",
                "metricAdhoc",
            ]
        );
    }

    #[test]
    fn the_operator_set_adds_the_logs_datasource_and_the_operator_namespace() {
        let set = operator_scoped("mz_");
        let names = names(&set);
        assert!(names.contains(&"logsDatasource"));
        assert!(names.contains(&"operatorNamespace"));
        // Everything the environment set defines is still there: the operator set
        // is an extension, not a replacement.
        for required in REQUIRED_VARIABLES {
            assert!(names.contains(required), "operator set dropped ${required}");
        }
    }

    #[test]
    fn the_operator_set_defines_the_metrics_datasource_exactly_once() {
        // `operator_scoped` prepends its own datasources and then splices in the
        // environment set, which defines the metrics one too. Emitting it twice
        // would be a duplicate variable name rather than a merge.
        let set = operator_scoped("mz_");
        let names = names(&set);
        assert_eq!(
            names.iter().filter(|n| **n == "metricsDatasource").count(),
            1
        );
        let mut sorted = names.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), names.len(), "duplicate variable name");
    }

    #[test]
    fn the_operator_set_leads_with_its_datasources_and_the_operator_scope() {
        // The controls row reads top-down: pick where you are looking (both
        // datasources, then the operator), and only then narrow to an environment.
        let set = operator_scoped("mz_");
        assert_eq!(
            &names(&set)[..3],
            &["metricsDatasource", "logsDatasource", "operatorNamespace"]
        );
    }

    #[test]
    fn the_operator_namespace_is_visible_single_select_and_hand_editable() {
        match operator_namespace() {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                // Visible where `mzNamespaceList` is hidden: this one is not
                // derived from the environment selection, so there is nothing for
                // it to desync from.
                assert_eq!(v.spec.hide, dashboardv2::VariableHide::DontHide);
                assert!(!v.spec.multi, "two operators' events must not merge");
                assert!(!v.spec.include_all);
                // The dashboard's whole subject is an operator in trouble, whose
                // metrics may be exactly what is missing.
                assert!(v.spec.allow_custom_value);
                assert!(!v.spec.skip_url_sync, "a permalink should carry it");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_operator_namespace_discovers_from_a_metric_the_operator_exports() {
        // Not `up{job="..."}`: that job label pins the monitoring namespace's
        // naming convention, which no deployment is required to follow.
        let expr = expr_of(&operator_namespace()).expect("a discovery query");
        assert_eq!(expr, "label_values(orchestratord_is_leader, namespace)");
        assert!(!expr.contains("job="), "{expr}");
    }

    #[test]
    fn the_two_datasource_variables_name_different_plugins() {
        // One `DatasourceVariable` resolves against one plugin id, which is the
        // reason a mixed-engine dashboard needs two rather than one.
        let plugin = |variable| match variable {
            dashboardv2::VariableKind::DatasourceVariableKind(v) => v.spec.plugin_id,
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(plugin(metrics_datasource()), "prometheus");
        assert_eq!(plugin(logs_datasource()), "loki");
    }

    #[test]
    fn the_generation_variable_captures_the_number_out_of_a_pod_name() {
        // The generation is an annotation, which nothing exports. The pod name is
        // where it actually reaches a query, so discovery reads names and the
        // regex is what turns one into a generation.
        match generations() {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                let expr = v.spec.query.spec.as_ref().unwrap()["expr"]
                    .as_str()
                    .unwrap();
                assert!(expr.ends_with(", pod)"), "{expr}");
                assert!(expr.contains("$environmentNameList"), "{expr}");
                assert!(v.spec.regex.contains("(?<value>"), "{}", v.spec.regex);
                assert!(v.spec.regex.contains("environmentd"), "{}", v.spec.regex);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_generation_variable_refreshes_with_the_time_range() {
        // The only one here that does. Which generations exist is a property of
        // the window: the old side is torn down after promotion, so widening the
        // range to cover a rollout is how it comes back into view.
        match generations() {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                assert_eq!(
                    v.spec.refresh,
                    dashboardv2::VariableRefresh::OnTimeRangeChanged
                );
                assert!(v.spec.multi, "comparing two generations is the point");
                assert!(v.spec.include_all);
                assert!(
                    v.spec.all_value.is_none(),
                    "a literal all_value would be regex-escaped by the :regex format"
                );
                assert_eq!(v.spec.sort, dashboardv2::VariableSort::NumericalDesc);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_generation_variable_sits_after_the_funnel_and_before_the_escape_hatch() {
        let set = operator_scoped("mz_");
        let names = names(&set);
        let generation = names.iter().position(|n| *n == "mzGenerationList").unwrap();
        let replicas = names.iter().position(|n| *n == "mzReplicaList").unwrap();
        let adhoc = names.iter().position(|n| *n == "metricAdhoc").unwrap();
        assert!(replicas < generation, "{names:?}");
        assert!(generation < adhoc, "{names:?}");
        // The ad-hoc filters are the tail of the row -- see
        // `the_operator_set_ends_with_both_ad_hoc_filters`.
        assert_eq!(adhoc, names.len() - 2, "{names:?}");
    }

    #[test]
    fn the_operator_set_ends_with_both_ad_hoc_filters() {
        // They are escape hatches rather than steps in the funnel, so they belong
        // at the end of the controls row -- and there are two because an ad-hoc
        // variable resolves its keys from one datasource.
        let set = operator_scoped("mz_");
        let names = names(&set);
        assert_eq!(&names[names.len() - 2..], &["metricAdhoc", "logsAdhoc"]);
    }

    #[test]
    fn each_ad_hoc_filter_points_at_its_own_datasource() {
        let datasource = |variable| match variable {
            dashboardv2::VariableKind::AdhocVariableKind(v) => {
                v.datasource.and_then(|d| d.name).unwrap_or_default()
            }
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(datasource(metric_adhoc()), "$metricsDatasource");
        assert_eq!(datasource(logs_adhoc()), "$logsDatasource");
    }

    #[test]
    fn the_logs_filter_seeds_no_base_filter() {
        // Grafana ANDs a base filter into the query's own selector. The obvious
        // seed -- the environment namespace -- would narrow a stream selector that
        // deliberately spans the operator's namespace too, and every event the
        // operator published would silently disappear.
        match logs_adhoc() {
            dashboardv2::VariableKind::AdhocVariableKind(v) => {
                assert!(v.spec.base_filters.is_empty());
                assert!(v.spec.allow_custom_value);
                assert_eq!(v.spec.hide, dashboardv2::VariableHide::InControlsMenu);
            }
            other => panic!("unexpected {other:?}"),
        }
        // The metrics one does seed, and that difference is deliberate.
        match metric_adhoc() {
            dashboardv2::VariableKind::AdhocVariableKind(v) => {
                assert_eq!(v.spec.base_filters.len(), 1);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_logs_set_is_loki_end_to_end() {
        // The property that matters: a logs dashboard must not depend on the
        // metrics pipeline, since it is often how you find out why that pipeline
        // is broken.
        let set = logs_scoped();
        let names = names(&set);
        assert!(!names.contains(&"metricsDatasource"), "{names:?}");
        for metric_scoped in REQUIRED_VARIABLES {
            assert!(
                !names.contains(metric_scoped),
                "logs set pulled in the metric-side ${metric_scoped}"
            );
        }
        for variable in &set {
            if let dashboardv2::VariableKind::QueryVariableKind(v) = variable {
                let ds = v
                    .spec
                    .query
                    .datasource
                    .as_ref()
                    .and_then(|d| d.name.as_deref())
                    .unwrap_or_default();
                assert_eq!(
                    ds, "${logsDatasource}",
                    "{} is not a Loki query",
                    v.spec.name
                );
                assert_eq!(v.spec.query.group, "loki", "{}", v.spec.name);
            }
        }
    }

    #[test]
    fn the_logs_funnel_narrows_in_order() {
        let set = logs_scoped();
        assert_eq!(
            names(&set),
            vec![
                "logsDatasource",
                "logNamespaceList",
                "logAppList",
                "logLevelList",
                "logJobList",
                "logSearch",
                "logsAdhoc",
            ]
        );
    }

    /// Whether a pattern can match the empty string, which is what LogQL rejects
    /// a selector for when every matcher does.
    ///
    /// A branch matches empty only if it is made entirely of `.*`: strip those and
    /// anything left is a required literal or a `+` quantifier. So `.*` matches
    /// empty, while `.*materialize.*` and `.+` do not — a check on the leading
    /// characters alone gets that backwards.
    fn matches_empty(pattern: &str) -> bool {
        pattern
            .split('|')
            .any(|branch| branch.replace(".*", "").is_empty())
    }

    #[test]
    fn no_log_selector_can_become_empty_compatible() {
        // LogQL rejects a stream selector whose every matcher can match the empty
        // string. Two of these values are load-bearing for that: the switch is
        // the *sole* matcher of the namespace discovery query, and the namespace
        // "All" is the sole matcher of the app, level and job discovery queries.
        // With `.*` in either place the variable fails to load, every picker
        // below it empties, and the dashboard reads as selecting no log lines
        // rather than as the query error it is.
        let anchor = |variable| match variable {
            dashboardv2::VariableKind::QueryVariableKind(v) => v.spec.all_value.clone(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(
            anchor(log_namespaces(MATERIALIZE_NAMESPACE_PATTERN)).as_deref(),
            Some(".+")
        );
        assert_eq!(anchor(log_jobs()).as_deref(), Some(".+"));
        assert!(!matches_empty(".+"));
        assert!(matches_empty(".*"));

        // Namespace discovery selects on nothing at all, which is the one shape
        // that cannot be empty-compatible.
        match log_namespaces(MATERIALIZE_NAMESPACE_PATTERN) {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                let stream = v.spec.query.spec.as_ref().expect("spec")["stream"]
                    .as_str()
                    .expect("stream");
                assert!(stream.is_empty(), "expected no selector, got {stream}");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_namespace_picker_opens_on_materialize_but_offers_everything() {
        // The distinction the dropped switch got wrong. A *filter* hid the
        // monitoring stack and `kube-system`, which are exactly what you need
        // when collection itself is failing. A *default* only decides where you
        // start; every namespace stays one click away.
        match log_namespaces(MATERIALIZE_NAMESPACE_PATTERN) {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                let selected = match &v.spec.current.value {
                    dashboardv2::VariableOptionValue::Array(values) => values.clone(),
                    dashboardv2::VariableOptionValue::String(value) => vec![value.clone()],
                };
                assert_eq!(
                    selected,
                    vec![".*materialize.*|mz-.*|environment-.*".to_string()]
                );

                // Discovery is unnarrowed, so the picker lists every namespace.
                let stream = v.spec.query.spec.as_ref().expect("spec")["stream"]
                    .as_str()
                    .expect("stream");
                assert!(stream.is_empty(), "discovery was narrowed: {stream}");
                // A pattern is not one of the discovered values, so this is what
                // lets it be the opening selection at all.
                assert!(v.spec.allow_custom_value);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn only_the_namespace_picker_opens_on_anything() {
        // The pickers below it chain off the namespace selection, so a default
        // there would fight whatever the namespace picker just narrowed to.
        for picker in [log_apps(), log_levels(), log_jobs()] {
            match picker {
                dashboardv2::VariableKind::QueryVariableKind(v) => {
                    let empty = matches!(
                        &v.spec.current.value,
                        dashboardv2::VariableOptionValue::String(s) if s.is_empty()
                    );
                    assert!(empty, "{} opens on a pinned selection", v.spec.name);
                }
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn all_means_all_for_the_labels_that_are_not_universal() {
        // `app` is genuinely absent from some streams, and `.+` drops them --
        // 2,407 of 30,432 lines in half an hour on a representative install,
        // most of `kube-system`. These two are not the anchor, so they can and
        // must be the inclusive form.
        let all_value = |variable| match variable {
            dashboardv2::VariableKind::QueryVariableKind(v) => v.spec.all_value.clone(),
            other => panic!("unexpected {other:?}"),
        };
        assert_eq!(all_value(log_apps()).as_deref(), Some(".*"));
        assert_eq!(all_value(log_levels()).as_deref(), Some(".*"));
    }

    #[test]
    fn every_log_picker_states_what_all_means() {
        // Left to expand into the discovered values, "All" is *empty* whenever
        // discovery has not run or has failed -- and `label=~""` matches only the
        // streams missing that label, so a picker that fails to load takes the
        // panel down with it rather than widening it.
        for picker in [
            log_namespaces(MATERIALIZE_NAMESPACE_PATTERN),
            log_apps(),
            log_levels(),
            log_jobs(),
        ] {
            match picker {
                dashboardv2::VariableKind::QueryVariableKind(v) => {
                    assert!(
                        v.spec.all_value.is_some(),
                        "{} leaves \"All\" to the discovered values",
                        v.spec.name
                    );
                    assert!(v.spec.include_all, "{}", v.spec.name);
                }
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn the_job_picker_can_never_interpolate_to_an_empty_matcher() {
        // LogQL rejects a stream selector whose every matcher can match the empty
        // string, and a dashboard of `=~` pickers is exactly that shape. This
        // variable's "All" is what keeps every log panel parseable, so a change
        // to `None` here would turn every one of them into a query error rather
        // than a wide result.
        match log_jobs() {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                assert_eq!(v.spec.all_value.as_deref(), Some(".+"));
                assert!(v.spec.include_all);
                // Sits in the controls menu: it is an advanced filter that most
                // readings never touch, but it must always be present.
                assert_eq!(v.spec.hide, dashboardv2::VariableHide::InControlsMenu);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_log_variable_queries_carry_a_label_and_the_label_values_mode() {
        // Loki answers a variable from a `{label, stream, type}` object rather
        // than from an expression, so a Prometheus-shaped query here would
        // silently resolve to nothing.
        for variable in [
            log_namespaces(MATERIALIZE_NAMESPACE_PATTERN),
            log_apps(),
            log_levels(),
        ] {
            match variable {
                dashboardv2::VariableKind::QueryVariableKind(v) => {
                    let spec = v.spec.query.spec.as_ref().expect("spec");
                    assert!(spec.get("label").and_then(|l| l.as_str()).is_some());
                    assert_eq!(spec["type"], serde_json::json!(1));
                    assert!(spec.get("expr").is_none(), "{}", v.spec.name);
                }
                other => panic!("unexpected {other:?}"),
            }
        }
    }

    #[test]
    fn no_variable_is_defined_twice() {
        let set = environment_scoped("mz_");
        let mut seen = names(&set);
        let count = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(seen.len(), count, "duplicate variable name");
    }

    /// Extract each variable's discovery expression, where it has one.
    fn expr_of(variable: &dashboardv2::VariableKind) -> Option<String> {
        match variable {
            dashboardv2::VariableKind::QueryVariableKind(v) => v
                .spec
                .query
                .spec
                .as_ref()
                .and_then(|s| s.get("expr"))
                .and_then(|e| e.as_str())
                .map(str::to_string),
            _ => None,
        }
    }

    #[test]
    fn every_chained_query_references_only_variables_the_set_defines() {
        // The chain is the part that breaks quietly: `mzReplicaList` reads
        // `$mzClusterList`, which reads `$includeSystemClusters`, and a rename
        // anywhere in that line leaves an empty selector.
        let set = environment_scoped("mz_");
        let defined = names(&set);
        for variable in &set {
            let Some(expr) = expr_of(variable) else {
                continue;
            };
            for reference in dollar_references(&expr) {
                assert!(
                    defined.contains(&reference.as_str()),
                    "{} references undefined ${reference}",
                    name_of(variable)
                );
            }
        }
    }

    #[test]
    fn the_chain_narrows_in_order() {
        let set = environment_scoped("mz_");
        let expr = |name: &str| {
            set.iter()
                .find(|v| name_of(v) == name)
                .and_then(expr_of)
                .unwrap_or_default()
        };
        // Environment discovery is unscoped; everything after it is scoped by the
        // selection before it.
        assert!(!expr("environmentNameList").contains('$'));
        assert!(expr("mzNamespaceList").contains("$environmentNameList"));
        assert!(expr("mzClusterList").contains("$environmentNameList"));
        assert!(expr("mzClusterList").contains("$includeSystemClusters"));
        assert!(expr("mzReplicaList").contains("$mzClusterList"));
    }

    #[test]
    fn the_cluster_query_is_sql_prefixed_and_the_others_are_not() {
        // `compute_cluster_status` is SQL-derived and must match the panels'
        // prefix; `mz_compute_commands_total` is genuine instrumentation and never
        // prefixed.
        let cloud = environment_scoped("v2_mz_");
        let expr = |set: &[dashboardv2::VariableKind], name: &str| {
            set.iter()
                .find(|v| name_of(v) == name)
                .and_then(expr_of)
                .unwrap()
        };
        assert!(expr(&cloud, "mzClusterList").contains("v2_mz_compute_cluster_status"));
        assert!(expr(&cloud, "mzReplicaList").contains("mz_compute_commands_total"));
        assert!(!expr(&cloud, "mzReplicaList").contains("v2_mz_compute_commands_total"));

        let self_managed = environment_scoped("mz_");
        assert!(expr(&self_managed, "mzClusterList").contains("mz_compute_cluster_status"));
    }

    #[test]
    fn the_environment_selector_is_single_select() {
        // Multi-select would let two environments' id-keyed series merge, and would
        // break the enrichment joins, which match on ids alone. See `environments`.
        match environments() {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                assert!(!v.spec.multi, "multi-select is unsound with id-keyed joins");
                assert!(
                    !v.spec.include_all,
                    "\"All\" is multi-select by another name"
                );
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn derived_variables_are_hidden_and_kept_out_of_the_url() {
        // A permalink that pins a derived namespace inconsistent with its own
        // environment is worse than one that re-derives it.
        let set = environment_scoped("mz_");
        match set
            .iter()
            .find(|v| name_of(v) == "mzNamespaceList")
            .unwrap()
        {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                assert_eq!(v.spec.hide, dashboardv2::VariableHide::HideVariable);
                assert!(v.spec.skip_url_sync);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_cluster_regex_names_both_captures() {
        match environment_scoped("mz_")
            .into_iter()
            .find(|v| name_of(v) == "mzClusterList")
            .unwrap()
        {
            dashboardv2::VariableKind::QueryVariableKind(v) => {
                assert!(v.spec.regex.contains("(?<value>"), "{}", v.spec.regex);
                assert!(v.spec.regex.contains("(?<text>"), "{}", v.spec.regex);
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn the_adhoc_filter_is_seeded_with_the_namespace_scope() {
        match metric_adhoc() {
            dashboardv2::VariableKind::AdhocVariableKind(v) => {
                let base = &v.spec.base_filters[0];
                assert_eq!(base.key, "namespace");
                assert_eq!(base.operator, "=~");
                assert_eq!(base.value, "$mzNamespaceList");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    /// Every `$name`, skipping `label_replace` capture groups.
    fn dollar_references(value: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = value;
        while let Some(pos) = rest.find('$') {
            let after = &rest[pos + 1..];
            let len = after
                .bytes()
                .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
                .count();
            if len > 0 && !after[..len].bytes().all(|b| b.is_ascii_digit()) {
                out.push(after[..len].to_string());
            }
            rest = &after[len..];
        }
        out
    }
}
