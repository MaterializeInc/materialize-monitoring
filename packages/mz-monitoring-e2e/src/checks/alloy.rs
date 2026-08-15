// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Alloy, via its support bundle.
//!
//! `/-/support` collapses "is the rendered config what we meant" and "are the
//! components healthy" into a single fetch, and it is the same artifact support
//! would ask a customer for during an escalation. Exercising it in CI keeps that
//! path honest instead of discovering it is broken mid-incident.
//!
//! Confirmed against the pinned Alloy (v1.17.0): the endpoint is `/-/support`,
//! it needs no stability-level flag, and `duration` bounds the metrics sample it
//! takes — one second is plenty, since nothing here reads the sample.
//!
//! Deliberately *not* asserted: `prometheus_sd_discovered_targets`. It counts
//! candidates before relabelling drops them, so it is non-zero whether or not a
//! selector matches anything — a target-count assertion built on it would pass
//! against a monitor that selects nothing at all.

use std::io::{Cursor, Read};

use anyhow::{Context, Result, bail};
use serde_json::Value;

use crate::cluster::ServiceTarget;
use crate::ctx::Ctx;
use crate::retry::retry_until;

/// Alloy's HTTP listen port, the same for both roles.
const ALLOY_PORT: u16 = 12345;

/// Seconds of metrics the bundle samples. Nothing here reads the sample, and it
/// is dead time in CI, so take the minimum.
const SAMPLE_SECONDS: u32 = 1;

const CONFIG_ENTRY: &str = "alloy-support-bundle/sources/config.alloy";
const COMPONENTS_ENTRY: &str = "alloy-support-bundle/alloy-components.json";

pub async fn gateway_support_bundle(ctx: &Ctx) -> Result<()> {
    support_bundle(ctx, "alloy-gateway").await
}

pub async fn agent_support_bundle(ctx: &Ctx) -> Result<()> {
    support_bundle(ctx, "alloy-agent").await
}

/// Fetch, unpack and inspect one role's support bundle.
///
/// Three assertions in one artifact: the endpoint works at all, the rendered
/// config is present and non-trivial, and every component the config produced is
/// healthy. The last is the one that catches a config that loads but does not
/// run — a component that failed to start reports `unhealthy` here while the pod
/// stays Ready and the deployment looks fine.
async fn support_bundle(ctx: &Ctx, service: &str) -> Result<()> {
    let target = ServiceTarget::new(service, ALLOY_PORT);
    let path = format!("-/support?duration={SAMPLE_SECONDS}");
    let what = format!("{service} support bundle is fetchable and healthy");

    retry_until(&what, ctx.deadline, ctx.interval, || async {
        let bytes = ctx.cluster.get_bytes(&target, &path).await?;
        let mut bundle = Bundle::open(&bytes)
            .with_context(|| format!("unpacking the {service} support bundle"))?;

        let config = bundle.entry(CONFIG_ENTRY)?;
        if config.trim().is_empty() {
            bail!("{service} rendered an empty config");
        }

        let components = bundle.entry(COMPONENTS_ENTRY)?;
        let components: Vec<Value> = serde_json::from_str(&components)
            .with_context(|| format!("parsing {COMPONENTS_ENTRY} from the {service} bundle"))?;

        // A bundle reporting no components at all would otherwise satisfy the
        // health check vacuously — every component in an empty list is healthy.
        if components.is_empty() {
            bail!("{service} reports no components; its config produced nothing");
        }

        let (exempt, unhealthy): (Vec<_>, Vec<_>) = components
            .iter()
            .filter(|c| c.pointer("/health/state").and_then(Value::as_str) != Some("healthy"))
            // Exact match, never a prefix or a glob: an exemption that widens on
            // its own would start swallowing failures it was never meant to
            // cover, and the whole point of the flag is that what it hides is
            // legible.
            .partition(|c| {
                c.get("localID")
                    .and_then(Value::as_str)
                    .is_some_and(|id| ctx.allow_unhealthy.iter().any(|a| a == id))
            });

        // Announced rather than silent. An exemption nobody sees is
        // indistinguishable from a check that never ran.
        for c in &exempt {
            eprintln!(
                "\n    allowed to be unhealthy: {}",
                c.get("localID")
                    .and_then(Value::as_str)
                    .unwrap_or("<unnamed>")
            );
        }

        let unhealthy: Vec<String> = unhealthy
            .iter()
            .map(|c| {
                format!(
                    "{} is {} ({})",
                    c.get("localID")
                        .and_then(Value::as_str)
                        .unwrap_or("<unnamed>"),
                    c.pointer("/health/state")
                        .and_then(Value::as_str)
                        .unwrap_or("in an unknown state"),
                    c.pointer("/health/message")
                        .and_then(Value::as_str)
                        .unwrap_or("no message"),
                )
            })
            .collect();

        if unhealthy.is_empty() {
            Ok(())
        } else {
            bail!(
                "{}/{} {service} components are unhealthy: {}",
                unhealthy.len(),
                components.len(),
                unhealthy.join("; ")
            )
        }
    })
    .await
}

/// The support bundle, held in memory.
///
/// Never written to disk: nothing here needs a file, and a temp file that
/// outlives a failed run is one more thing to clean up. The pprof profiles and
/// metrics samples make it a few hundred kilobytes, which is cheap to hold.
struct Bundle {
    archive: zip::ZipArchive<Cursor<Vec<u8>>>,
}

impl Bundle {
    fn open(bytes: &[u8]) -> Result<Self> {
        let archive = zip::ZipArchive::new(Cursor::new(bytes.to_vec()))
            // A non-zip body here is almost always an error page from something
            // in front of Alloy, so quote it rather than only naming the parse
            // failure.
            .with_context(|| {
                let head: String = String::from_utf8_lossy(bytes).chars().take(200).collect();
                format!("response is not a zip archive (starts: {head})")
            })?;
        Ok(Self { archive })
    }

    fn entry(&mut self, name: &str) -> Result<String> {
        // Listed up front: `by_name` borrows the archive mutably, so the error
        // path cannot go back and ask what it does contain.
        let present: Vec<String> = self.archive.file_names().map(str::to_owned).collect();
        let mut file = self
            .archive
            .by_name(name)
            .with_context(|| format!("bundle has no {name} (contains: {})", present.join(", ")))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .with_context(|| format!("reading {name} out of the bundle"))?;
        Ok(contents)
    }
}
