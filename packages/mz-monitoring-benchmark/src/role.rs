// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The Materialize processes a pod can impersonate, and the embedded fixtures
//! for each.

use anyhow::{Context, Result};

use crate::fixture::Fixture;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Role {
    Environmentd,
    Clusterd,
    Balancerd,
    Orchestratord,
    Console,
}

/// How a role's process writes its logs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogFormat {
    /// tracing's JSON layer: environmentd and clusterd.
    TracingJson,
    /// tracing's default text layer: balancerd and the operator.
    TracingText,
    /// nginx access and error logs: the console.
    Nginx,
}

impl Role {
    pub const ALL: [Role; 5] = [
        Role::Environmentd,
        Role::Clusterd,
        Role::Balancerd,
        Role::Orchestratord,
        Role::Console,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Role::Environmentd => "environmentd",
            Role::Clusterd => "clusterd",
            Role::Balancerd => "balancerd",
            Role::Orchestratord => "orchestratord",
            Role::Console => "console",
        }
    }

    pub fn parse(s: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|r| r.as_str() == s)
    }

    /// The endpoints the real process serves and the chart scrapes.
    pub fn paths(self) -> &'static [&'static str] {
        match self {
            Role::Environmentd => &[
                "/metrics",
                "/metrics/mz_compute",
                "/metrics/mz_frontier",
                "/metrics/mz_storage",
                "/metrics/mz_usage",
            ],
            _ => &["/metrics"],
        }
    }

    pub fn log_format(self) -> LogFormat {
        match self {
            Role::Environmentd | Role::Clusterd => LogFormat::TracingJson,
            Role::Balancerd | Role::Orchestratord => LogFormat::TracingText,
            Role::Console => LogFormat::Nginx,
        }
    }

    /// The container name the real process runs under. The gateway chooses
    /// its log parsing (multiline panics, the JSON and text formats) by
    /// container name, so a benchmark pod's container takes the same one.
    pub fn container_name(self) -> &'static str {
        match self {
            Role::Orchestratord => "materialize-operator",
            r => r.as_str(),
        }
    }

    /// Whether the process is Rust, and so can panic.
    pub fn is_rust(self) -> bool {
        !matches!(self, Role::Console)
    }

    /// The thread a panic names, as the real process's panics do.
    pub fn panic_thread(self) -> &'static str {
        match self {
            Role::Environmentd => "coordinator",
            Role::Clusterd => "timely:work-0",
            _ => "tokio-runtime-worker",
        }
    }

    /// The pod label `fixture-gen` finds the real process by.
    pub fn selector(self) -> &'static str {
        match self {
            Role::Environmentd => "app.kubernetes.io/name=environmentd",
            Role::Clusterd => "app.kubernetes.io/name=clusterd",
            Role::Balancerd => "app.kubernetes.io/name=balancerd",
            Role::Orchestratord => "app.kubernetes.io/name=materialize-operator",
            Role::Console => "app.kubernetes.io/name=console",
        }
    }

    /// The container port the real process serves metrics on.
    pub fn port_name(self) -> &'static str {
        match self {
            Role::Orchestratord => "metrics",
            _ => "internal-http",
        }
    }

    /// Whether the role lives in the environment's namespace, rather than the
    /// operator's.
    pub fn in_environment(self) -> bool {
        !matches!(self, Role::Orchestratord)
    }
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

macro_rules! embedded {
    ($($role:literal, $file:literal;)*) => {
        &[$(($role, $file, include_str!(concat!("../fixtures/", $role, "/", $file)))),*]
    };
}

/// The fixtures compiled into the binary: `(role, file name, text)`.
const EMBEDDED: &[(&str, &str, &str)] = embedded! {
    "environmentd", "metrics.tmpl";
    "environmentd", "metrics_mz_compute.tmpl";
    "environmentd", "metrics_mz_frontier.tmpl";
    "environmentd", "metrics_mz_storage.tmpl";
    "environmentd", "metrics_mz_usage.tmpl";
    "clusterd", "metrics.tmpl";
    "balancerd", "metrics.tmpl";
    "orchestratord", "metrics.tmpl";
    "console", "metrics.tmpl";
};

/// A role's fixtures, from `dir` when given, else the embedded copies.
pub fn load_fixtures(role: Role, dir: Option<&std::path::Path>) -> Result<Vec<Fixture>> {
    role.paths()
        .iter()
        .map(|path| {
            let file = Fixture::file_name(path);
            let text = match dir {
                Some(dir) => {
                    let p = dir.join(role.as_str()).join(&file);
                    std::fs::read_to_string(&p)
                        .with_context(|| format!("reading {}", p.display()))?
                }
                None => EMBEDDED
                    .iter()
                    .find(|(r, f, _)| *r == role.as_str() && *f == file)
                    .map(|(_, _, t)| (*t).to_string())
                    .with_context(|| format!("no embedded fixture {}/{file}", role.as_str()))?,
            };
            let fixture = Fixture::parse(&text)
                .with_context(|| format!("parsing fixture {}/{file}", role.as_str()))?;
            anyhow::ensure!(
                fixture.role == role.as_str() && fixture.path == *path,
                "fixture {}/{file} says it is {} {}",
                role.as_str(),
                fixture.role,
                fixture.path
            );
            Ok(fixture)
        })
        .collect()
}
