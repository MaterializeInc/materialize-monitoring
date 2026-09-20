// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Overview tab: is the cluster's networking healthy, and what is it made
//! of.
//!
//! Two rows, answering the two questions in that order. The verdict row is
//! throughput beside losses, because a number is only alarming relative to the
//! traffic it happened during. The inventory row says what the dashboard found
//! — which dataplane, how many Services, how many policies — and is what makes
//! the rest of the tabs legible before any of them is opened.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::generated::stat::BigValueTextMode;
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::theme;
use crate::grafana::queries::Queries;

const SHADE: &str = theme::OVERVIEW.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![health(q), inventory(q), talkers(q)]
}

fn health(q: &Queries) -> Row {
    Row::new("Traffic and Losses").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("overview-throughput", throughput(q))
            .panel("overview-errors", errors(q)),
    )
}

fn inventory(q: &Queries) -> Row {
    Row::new("What This Cluster Runs").grid(
        AutoGrid::new(3)
            .column_width(ColumnWidth::Narrow)
            .panel("overview-dataplane", dataplane(q))
            .panel("overview-services", services(q))
            .panel("overview-load-balancers", load_balancers(q)),
    )
}

fn talkers(q: &Queries) -> Row {
    Row::new("Busiest Pods").grid(AutoGrid::new(1).panel("overview-top-talkers", top_talkers(q)))
}

fn throughput(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Pod Throughput")
        .query(q.legended(
            "infra.net.overview.throughput",
            &["received", "transmitted"],
        ))
        .unit("Bps")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}

fn errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Errors and Drops")
        .query(q.legended("infra.net.overview.errors", &["errors", "drops"]))
        .unit("cps")
        .min(0.0)
        // Zero is the expected reading for both series, so an empty panel is the
        // healthy one and has to say so rather than looking unloaded.
        .no_value(NoValue::Custom(
            "No interface errors or dropped packets.".to_string(),
        ))
        .build(0)
}

/// What the dashboard detected, which is the tab's whole orientation.
///
/// A stat rather than a table: the value is a short string and there are
/// normally one or two of them. `VALUE_AND_NAME` because the query returns one
/// series per component, so each tile has to label itself.
fn dataplane(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Dataplane")
        .query(
            q.get("infra.net.overview.dataplane")
                .legend("{{network_component}}"),
        )
        .shade(SHADE)
        .text_mode(BigValueTextMode::ValueAndName)
        // Not an error, and the CNI tab explains which clusters land here, so
        // this points there rather than repeating the explanation.
        .no_value(NoValue::Custom(
            "No networking exporter detected — see the CNI tab.".to_string(),
        ))
        .build(0)
}

fn services(q: &Queries) -> dashboardv2::PanelKind {
    Panel::piechart("Services by Type")
        .query(q.get("infra.net.k8s.services.by_type").legend("{{type}}"))
        .unit("short")
        .no_value(NoValue::RequiresKubeStateMetrics)
        .build(0)
}

fn load_balancers(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Load Balancers")
        .query(
            q.get("infra.net.cloud.load_balancers")
                .legend("{{service}}"),
        )
        .shade(SHADE)
        .min(0.0)
        .text_mode(BigValueTextMode::Name)
        .no_value(NoValue::Custom(
            "No LoadBalancer Services have been given an address.".to_string(),
        ))
        .build(0)
}

fn top_talkers(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Busiest Pods")
        .query(
            q.get("infra.net.overview.top_talkers")
                .legend("{{namespace}}/{{pod}}"),
        )
        .unit("Bps")
        .min(0.0)
        .no_value(NoValue::RequiresCAdvisor)
        .build(0)
}
