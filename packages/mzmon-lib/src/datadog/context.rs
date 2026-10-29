// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The parameters a Datadog dashboard gives the registry's templates.
//!
//! The Datadog counterpart of [`crate::grafana::context`]. Parameters are mapped
//! as dashboards need them. A template that uses one this context doesn't set
//! fails to render, rather than reaching Datadog as a literal `%%{…}`, so porting
//! a panel is also the moment its parameters get a Datadog meaning.

use crate::query::{QueryEngine, QueryRegistry, TemplateContext};

use super::variable;

/// The render context for a Datadog dashboard.
///
/// `sql_metric_prefix` is `mz_` on self-managed and `v2_mz_` on Materialize
/// Cloud, and reaches every query that reads a SQL-derived metric.
pub fn dashboard_context<'a>(
    registry: &'a QueryRegistry,
    sql_metric_prefix: &str,
) -> TemplateContext<'a> {
    let mut context = TemplateContext::new(QueryEngine::Datadog).with_registry(registry);
    let namespace = variable::value(variable::NAMESPACE);
    for (name, value) in [
        ("mzSqlPrefix", sql_metric_prefix.to_string()),
        // `$environment` expands to the environment tag and the picked value, or
        // to nothing when the picker is `*`.
        ("mzEnvironmentFilter", format!("${}", variable::ENVIRONMENT)),
        // The templates write the tag, as in `compute_cluster_id:%%{mzClusterList}`
        // or `pod:*-cluster-%%{mzClusterList}-…`, so these are values alone.
        ("mzClusterList", variable::value(variable::CLUSTER)),
        ("mzReplicaList", variable::value(variable::REPLICA)),
        ("mzNamespaceList", namespace.clone()),
        // `container!=""` and `container!="POD"`: the pod-level cgroup roll-up and
        // the pause container aren't containers.
        (
            "cAdvisorFilter",
            format!("namespace:{namespace} AND container:* AND NOT container:POD"),
        ),
    ] {
        context.parameters.insert(name.to_string(), value);
    }
    context
}
