// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Synthetic Materialize metrics and logs, for load-testing the monitoring
//! stack's write path (the gateway) and its stores (Thanos and Loki).
//!
//! `fixture-gen` captures real scrapes and anonymizes them into fixtures;
//! `gen-load` serves the fixtures expanded to a requested size, and writes
//! logs in each role's format. The goal is realistic load, not realistic
//! graphs.

pub mod anonymize;
#[cfg(feature = "capture")]
pub mod capture;
pub mod expo;
pub mod fixture;
pub mod logs;
pub mod model;
pub mod role;
pub mod scenario;
pub mod seed;
pub mod serve;
