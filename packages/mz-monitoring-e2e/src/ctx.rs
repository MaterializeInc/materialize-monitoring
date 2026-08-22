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

use std::sync::Arc;

use anyhow::{Result, anyhow};
use rustls::ClientConfig;

use crate::cluster::Cluster;
use crate::features::Features;
use crate::tls::ClientTls;

pub struct Ctx {
    pub cluster: Cluster,
    pub features: Features,
    /// How long any single assertion may retry before it is a failure.
    pub deadline: Duration,
    /// Gap between retries.
    pub interval: Duration,
    /// How recent a log line has to be to count as proof the write path is live.
    pub recent_window: Duration,
    /// Alloy component IDs allowed to be unhealthy; see `--allow-unhealthy`.
    pub allow_unhealthy: Vec<String>,
    /// Whether assertions that change cluster state may run. See the CLI flag.
    pub allow_disruptive: bool,
    /// The suite's own certificate material, loaded once at startup when the
    /// release issues certificates. `None` on a plaintext stack, where nothing
    /// needs it.
    pub tls: Option<ClientTls>,
}

impl Ctx {
    /// The client configuration for dialling a hop that serves TLS.
    ///
    /// An error rather than an `Option`, because every caller reaches this only
    /// after deciding the hop *is* TLS — at which point missing material is a
    /// setup problem to report, not a branch to take.
    pub fn client_tls(&self) -> Result<Arc<ClientConfig>> {
        self.tls.as_ref().map(ClientTls::authenticated).ok_or_else(|| {
            anyhow!(
                "this hop serves TLS but the suite has no certificate material. That combination \
                 means the release configured a TLS listener without `certificates.enabled`, so \
                 the material was mounted from somewhere the chart did not issue — point the \
                 suite at it with --client-cert-secret."
            )
        })
    }

    /// The same trust root, presenting no client certificate.
    pub fn anonymous_tls(&self) -> Result<Arc<ClientConfig>> {
        self.tls
            .as_ref()
            .map(ClientTls::anonymous)
            .ok_or_else(|| anyhow!("no certificate material loaded; see --client-cert-secret"))
    }
}
