// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Best-effort transpiler from prometheus-operator Monitors into classic
//! Prometheus `scrape_configs`.
//!
//! `packages/prometheus-scrapers/` holds metric-collection definitions in the
//! modern prometheus-operator format (`PodMonitor` v1, `ServiceMonitor` v1,
//! `ScrapeConfig` v1alpha1). Some older Materialize deployments still run plain
//! Prometheus, which consumes classic `scrape_configs`. This module turns the
//! Monitors into that classic form so they can be documented and dropped into a
//! legacy Prometheus/Agent config.
//!
//! Support is **best-effort**: the typed input model (`monitor`) covers only the
//! subset of each CRD we transpile and ignores the rest, while validation leans
//! on the real upstream CRD schemas (`validate`, embedded from
//! `schemas/scrape/`). The output model (`config`) is plain serde structs
//! serialized straight to YAML — there is no custom renderer; field declaration
//! order is the emitted key order.
//!
//! Shape mirrors the `alloy` module: typed input + schema validation in
//! `from_yaml_str`, inline `#[cfg(test)]` tests per module.

pub mod error;
pub mod transpile;
pub mod validate;

pub mod classic {
    pub mod config;
}

pub mod gmp {
    pub mod config;
}

pub mod operator {
    pub mod common;
    pub mod pod_monitor;
    pub mod scrape_config;
    pub mod service_monitor;
}

#[cfg(test)]
pub(crate) mod test_support;
