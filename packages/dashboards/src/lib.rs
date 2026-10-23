// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Materialize's dashboards.
//!
//! One module per backend: [`grafana`] and [`datadog`]. A Google Cloud Monitoring
//! backend would sit beside them rather than inside either, because very little is
//! genuinely shared: the queries differ per engine (that is what the query
//! registry's per-engine templates are for), and each backend's SDK has its own
//! panel and layout model.
//!
//! What *is* worth sharing lives in the query registry, not here — a query
//! defined once renders to PromQL, LogQL, or a Datadog metric query through
//! [`mzmon_lib::query`]. Shared structure grows out of the backends existing
//! rather than being designed ahead of them; [`canonical`], which both use to
//! write their artifacts, is the first piece.

pub mod canonical;
pub mod datadog;
pub mod grafana;
