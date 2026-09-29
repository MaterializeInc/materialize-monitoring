// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Collection tab: whether the pulls are working, and what they cost.
//!
//! Last rather than first, unlike the meta-monitoring dashboards, because an
//! empty provider row already says most of what this tab would: the rows render
//! on `up`, so a pull that is configured but returning nothing leaves its rows
//! on screen and empty. This tab is where that is confirmed — and where the one
//! trap is spelled out, since `up` reads 1 for a pull whose provider call
//! failed.
//!
//! The Pulls row is not rendered on the provider variable: with nothing pulled,
//! its panels say so in their own empty state, and the other two tabs carry the
//! fallback. The Cloud Monitoring row is, since nothing else publishes its two
//! signals.

use mzmon_lib::grafana::generated::{dashboardv2, stat::BigValueTextMode};
use mzmon_lib::grafana::layout::{AutoGrid, ColumnWidth, Row, RowHeight};
use mzmon_lib::grafana::panel::{NoValue, Panel};

use super::{GCP, PROVIDERS, not_pulled, theme};
use crate::grafana::queries::Queries;
use crate::grafana::transform;

const SHADE: &str = theme::COLLECTION.shade;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![pulls(q), gcp(q), about()]
}

fn pulls(q: &Queries) -> Row {
    Row::new("Pulls").grid(
        AutoGrid::new(2)
            .column_width(ColumnWidth::Wide)
            .panel("collection-pulls", pull_table(q))
            .panel("collection-resources", resources(q)),
    )
}

/// GCP only: CloudWatch and Azure Monitor publish no equivalent of either.
fn gcp(q: &Queries) -> Row {
    Row::new("Cloud Monitoring")
        .only_when_variable(PROVIDERS, GCP)
        .grid(
            AutoGrid::new(2)
                .column_width(ColumnWidth::Wide)
                .panel("collection-gcp-errors", gcp_errors(q))
                .panel("collection-gcp-calls", gcp_calls(q)),
        )
}

fn about() -> Row {
    Row::new("About Provider Data").collapsed().grid(
        AutoGrid::new(1).row_height(RowHeight::Tall).panel(
            "collection-about",
            Panel::text("About Provider Data", ABOUT)
                .description("How old provider data is, and what that means for reading it.")
                .build(0),
        ),
    )
}

fn pull_table(q: &Queries) -> dashboardv2::PanelKind {
    Panel::table("Provider Pulls")
        .query(q.get("infra.cloud.collection.pulls").table_format())
        .transformations(vec![transform::organize_full(
            &["Time"],
            &["job", "instance", "Value"],
            &[("job", "provider"), ("instance", "pull"), ("Value", "answering")],
        )])
        .unit("short")
        .no_value(NoValue::Custom(
            "No provider pulls are configured. See pipeline.metrics.provider in the chart's values."
                .to_string(),
        ))
        .build(0)
}

fn resources(q: &Queries) -> dashboardv2::PanelKind {
    Panel::stat("Resources Returning Data")
        .query(q.legended(
            "infra.cloud.collection.resources",
            &[
                "RDS instances",
                "S3 buckets",
                "Cloud SQL instances",
                "GCS buckets",
                "Flexible Servers",
                "storage accounts",
            ],
        ))
        .shade(SHADE)
        .text_mode(BigValueTextMode::ValueAndName)
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .no_value(not_pulled())
        .build(0)
}

fn gcp_errors(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("Failed Pulls")
        .query(
            q.get("infra.cloud.collection.gcp_errors")
                .legend("{{instance}}"),
        )
        .unit("short")
        .decimals(0.0)
        .min(0.0)
        .max(1.0)
        .no_value(NoValue::Custom(
            "Not pulling from Cloud Monitoring.".to_string(),
        ))
        .build(0)
}

fn gcp_calls(q: &Queries) -> dashboardv2::PanelKind {
    Panel::timeseries("API Calls")
        .query(
            q.get("infra.cloud.collection.gcp_calls")
                .legend("{{instance}}"),
        )
        .unit("suffix:calls/min")
        .min(0.0)
        .no_value(NoValue::Custom(
            "Not pulling from Cloud Monitoring.".to_string(),
        ))
        .build(0)
}

/// Why the data on this dashboard is old, and what not to conclude from it.
const ABOUT: &str = "**Everything on this dashboard is minutes old, and some of it a day old.**\n\n\
     | Source | Age on arrival |\n\
     |---|---|\n\
     | RDS, Azure Flexible Server, Azure Blob requests | A few minutes |\n\
     | Cloud SQL | About three minutes |\n\
     | GCS | Over ten minutes |\n\
     | S3 size and object count, Azure Blob capacity | Up to a day |\n\n\
     Read it beside Materialize's own view — Materialize Consensus and Materialize Persist — \
     which is current to the scrape. When the two disagree during an incident, the difference \
     is usually the provider's delay rather than a contradiction.\n\n\
     - **A restart of the gateway leaves a gap** in the Cloud SQL and GCS series as long as \
       their delay, because samples stamped before the restart are never sent. CloudWatch and \
       Azure samples are stamped when they are pulled and have no gap.\n\
     - **A pull that answers is not a pull that worked.** _Provider Pulls_ reads 1 for an \
       exporter whose call to the provider failed; _Resources Returning Data_ is the check.\n\
     - **Each pull is billed by the provider**, at a cost set by how many resources are \
       listed and how often they are pulled, not by how often this dashboard is viewed.";
