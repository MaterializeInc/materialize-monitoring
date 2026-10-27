// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `env-top` for Datadog: a preview.
//!
//! The port of [`crate::grafana::env_top`] starts here, with the Cluster Objects /
//! Replicas tab's summary counts. They read `compute_cluster_status`, which every
//! self-managed install's SQL-exporter scrape publishes, and they carry the
//! machinery end to end: registry lookup, labeled templates, widgets, template
//! variable, render, and the docsite's download. The rest of the Grafana
//! dashboard's tabs are ported panel by panel, each mapping the registry
//! parameters it needs in [`super::queries`].

use datadog_api_client::datadogV1::model::{
    Dashboard, DashboardLayoutType, DashboardReflowType, Widget,
};
use mzmon_lib::query::QueryRegistry;

use super::queries::Queries;
use super::render::{Error, Result};
use super::{Options, variable, widget};
use crate::grafana::env_top as grafana;

/// Artifact stem, shared with the Grafana dashboard.
pub const NAME_STEM: &str = grafana::NAME_STEM;

/// The preview's note, shown above the panels.
///
/// The Grafana environment picker selects exactly one environment, and a Datadog
/// template variable always offers `*`. Environments reuse cluster and replica
/// ids, so a count grouped by id merges them when several are selected.
// TODO: settle how Datadog panels stay scoped to one environment (DEP-115), then
// drop the instruction to pick one.
const PREVIEW_NOTE: &str = "**Preview.** The start of the Datadog version of the Materialize \
Environment Overview, with the cluster and replica counts only. Pick one **environment**: \
environments reuse cluster and replica ids, so the counts merge them when several are selected. \
The Grafana version has the full set of panels.";

/// Build the dashboard.
pub fn render(options: &Options, registry: &QueryRegistry) -> Result<Dashboard> {
    let q = Queries::new(registry, options);
    let widgets = vec![widget::note(PREVIEW_NOTE), cluster_summary(&q)];

    let failures = q.failures();
    if !failures.is_empty() {
        return Err(Error::Registry {
            name: NAME_STEM,
            failures,
        });
    }

    Ok(Dashboard::new(
        DashboardLayoutType::ORDERED,
        format!("{} (Preview)", grafana::TITLE),
        widgets,
    )
    .description(Some(
        "Overview of a Materialize environment, ported from the Grafana dashboard of the same \
         name."
            .to_string(),
    ))
    .reflow_type(DashboardReflowType::AUTO)
    .template_variables(Some(vec![variable::environment()])))
}

/// The Cluster Objects / Replicas tab's summary: how many clusters and replicas
/// there are, and how many of each are the system's or redundancy.
fn cluster_summary(q: &Queries) -> Widget {
    const CLUSTERS: &[&str] = &["Total Clusters", "System Clusters"];
    const REPLICAS: &[&str] = &["Total Replicas", "Redundant Replicas"];

    let mut widgets =
        widget::query_values(CLUSTERS, &q.labeled("materialize.clusters.count", CLUSTERS));
    widgets.extend(widget::query_values(
        REPLICAS,
        &q.labeled("materialize.clusters.replicas.count", REPLICAS),
    ));
    widget::group("Cluster Objects / Replicas", widgets)
}
