// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Events tab: the record of every node added, removed or refused.
//!
//! Events are the one source every autoscaler writes to, so this is the tab
//! that reads the same on every cloud: Karpenter on EKS, the managed cluster
//! autoscaler on GKE and AKS, the node lifecycle, and the scheduler's
//! placement failures. Karpenter's two repeating reasons are left out, as the
//! query's own notes say.

use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::grafana::layout::{AutoGrid, Row, RowHeight};
use mzmon_lib::grafana::panel::Panel;

use super::{overview, quiet};
use crate::grafana::queries::Queries;

pub fn rows(q: &Queries) -> Vec<Row> {
    vec![activity(q), warnings(q), all(q)]
}

fn activity(q: &Queries) -> Row {
    Row::new("Activity")
        .grid(AutoGrid::new(1).panel("events-by-reason", overview::events_by_reason(q)))
}

fn warnings(q: &Queries) -> Row {
    Row::new("Warnings").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("events-warnings", overview::warnings(q)),
    )
}

fn all(q: &Queries) -> Row {
    Row::new("All Autoscaling Events").grid(
        AutoGrid::new(1)
            .row_height(RowHeight::Tall)
            .panel("events-all", stream(q)),
    )
}

fn stream(q: &Queries) -> dashboardv2::PanelKind {
    Panel::logs("All Autoscaling Events")
        .query(q.logs("infra.autoscaling.events.stream"))
        .dedup_by_signature()
        .no_value(quiet("autoscaling events"))
        .build(0)
}
