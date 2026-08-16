// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Deadline-bounded retry.
//!
//! Every assertion in this suite runs through here rather than checking once.
//! Ingestion is asynchronous and rollouts are not instantaneous, so a bare check
//! is the classic E2E flake — it fails on timing and reads as a broken stack.
//!
//! The deadline is a bound on *how long a failure takes to report*, not a
//! sleep: a healthy stack satisfies the first attempt and returns immediately.

use std::future::Future;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow};

/// Poll `f` until it succeeds or `deadline` elapses.
///
/// The last error is carried into the timeout message. Without it a timeout says
/// only that something did not happen, which is the least actionable failure a
/// CI log can contain — the interesting part is *how* the final attempt failed.
pub async fn retry_until<F, Fut, T>(
    what: &str,
    deadline: Duration,
    interval: Duration,
    mut f: F,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T>>,
{
    let started = Instant::now();
    let mut attempts = 0usize;
    let mut last_report = Instant::now();

    loop {
        attempts += 1;
        match f().await {
            Ok(value) => return Ok(value),
            Err(err) => {
                // The deadline is checked here rather than before the attempt,
                // so a deadline shorter than one round trip still produces a
                // real error instead of timing out having never tried.
                if started.elapsed() >= deadline {
                    return Err(anyhow!(
                        "{what} did not succeed within {}s ({attempts} attempts); \
                         last error: {err:#}",
                        deadline.as_secs(),
                    ));
                }

                // Nothing is printed on the fast path — a check that passes
                // first try should not decorate the harness's own output. But a
                // three-minute deadline spent in silence is indistinguishable
                // from a hang, so once a check has been failing for a while,
                // report *why* while it keeps trying rather than saving it for
                // the timeout.
                if last_report.elapsed() >= PROGRESS_AFTER {
                    last_report = Instant::now();
                    eprintln!(
                        "\n    still failing after {attempts} attempts ({:.0}s): {err:#}",
                        started.elapsed().as_secs_f64()
                    );
                }

                tokio::time::sleep(interval).await;
            }
        }
    }
}

/// How long a check may keep failing before it says so mid-flight.
const PROGRESS_AFTER: Duration = Duration::from_secs(20);
