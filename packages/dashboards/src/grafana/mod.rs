// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Grafana dashboards, built on [`mzmon_lib::grafana`].
//!
//! One module per dashboard. Each owns its own `theme` so the colour assignments
//! for its tabs live in one file rather than being spread across them.

pub mod env_top;
