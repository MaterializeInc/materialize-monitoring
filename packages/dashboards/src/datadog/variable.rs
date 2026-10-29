// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Dashboard template variables.
//!
//! A Datadog template variable carries a tag `prefix`, and a query that names it
//! as `$name` gets `prefix:value` spliced in, or nothing when it is `*`. A query
//! that writes the tag itself names `$name.value` instead, which is the picked
//! value alone, or `*`. Each variable here backs registry parameters in
//! [`super::queries`].

use datadog_api_client::datadogV1::model::DashboardTemplateVariable;

/// The environment picker's name, as queries reference it (`$environment`).
pub const ENVIRONMENT: &str = "environment";

/// The tag an environment is scoped by: the label the Grafana dashboards'
/// `%%{mzEnvironmentFilter}` matches.
// TODO: move to `environment_id` with the Grafana fragment, in DEP-354.
pub const ENVIRONMENT_TAG: &str = "materialize_cloud_organization_name";

/// The Kubernetes namespace picker's name (`$namespace`).
///
/// Grafana derives its namespace list from the environment picker. A Datadog
/// variable can't be derived from another, and the cAdvisor and
/// kube-state-metrics series carry no environment tag, so the namespace is a
/// picker of its own.
pub const NAMESPACE: &str = "namespace";

/// The cluster picker's name (`$cluster`).
pub const CLUSTER: &str = "cluster";

/// The replica picker's name (`$replica`).
pub const REPLICA: &str = "replica";

/// The environment picker, defaulting to every environment.
pub fn environment() -> DashboardTemplateVariable {
    picker(ENVIRONMENT, ENVIRONMENT_TAG)
}

/// The namespace picker, defaulting to every namespace.
pub fn namespace() -> DashboardTemplateVariable {
    picker(NAMESPACE, "namespace")
}

/// The cluster picker, listing cluster ids.
///
/// The canonical `cluster_id` lists them, because every Materialize series
/// carries it beside its own spelling. The registry's templates still filter on
/// their own spelling, such as `instance_id`, so they read `$cluster.value`.
pub fn cluster() -> DashboardTemplateVariable {
    picker(CLUSTER, "cluster_id")
}

/// The replica picker, listing replica ids from the canonical `replica_id`.
pub fn replica() -> DashboardTemplateVariable {
    picker(REPLICA, "replica_id")
}

/// `$name.value`: the picked value alone, for a template that names the tag.
pub fn value(name: &str) -> String {
    format!("${name}.value")
}

fn picker(name: &str, tag: &str) -> DashboardTemplateVariable {
    DashboardTemplateVariable::new(name.to_string())
        .prefix(Some(tag.to_string()))
        .defaults(vec!["*".to_string()])
}
