// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Everything an assertion needs, built once at startup and shared by all of
//! them.

use std::time::Duration;

use crate::cluster::Cluster;
use crate::features::Features;

pub struct Ctx {
    pub cluster: Cluster,
    pub features: Features,
    /// How long any single assertion may retry before it is a failure.
    pub deadline: Duration,
    /// Gap between retries.
    pub interval: Duration,
    /// How recent a log line has to be to count as proof the write path is live.
    pub recent_window: Duration,
}
