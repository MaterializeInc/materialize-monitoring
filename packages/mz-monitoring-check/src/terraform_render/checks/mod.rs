// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The assertions, one module per lever the Terraform module exposes.
//!
//! **When you add a lever, add an assertion that it lands.** Every one of these
//! exists because its failure is silent: the value is valid, the plan is clean,
//! the chart renders, and the setting does nothing.
//!
//! Each assertion collects every problem it finds rather than stopping at the
//! first, since a fan-out usually misses several workloads at once and the list
//! is the diagnosis.

use anyhow::{Result, bail};
use serde_json::Value;

pub mod alerting;
pub mod certificates;
pub mod cluster_name;
pub mod destinations;
pub mod gateway_credentials;
pub mod grafana;
pub mod internal_tls;
pub mod kube_state_metrics;
pub mod node_exporter;
pub mod object_storage;
pub mod scheduling;
pub mod storage_class;
pub mod workload_identity;

/// A string read back for a message: quoted when present, `unset` when not.
pub fn shown(value: Option<&str>) -> String {
    value.map_or_else(|| "unset".to_owned(), |value| format!("{value:?}"))
}

/// The same for any value, written as JSON.
pub fn shown_json(value: Option<&Value>) -> String {
    value.map_or_else(|| "unset".to_owned(), Value::to_string)
}

/// Fail with every problem, then where to look; pass when there are none.
pub fn report(problems: Vec<String>, hint: &str) -> Result<()> {
    if problems.is_empty() {
        return Ok(());
    }
    let mut message = problems.join("\n");
    if !hint.is_empty() {
        message.push('\n');
        message.push_str(hint);
    }
    bail!(message)
}
