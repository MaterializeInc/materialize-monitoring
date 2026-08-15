// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! What the release under test actually turned on.
//!
//! Read from `helm get values --all`, and **`--all` is load-bearing**: plain
//! `helm get values` returns only what the caller supplied, so a default install
//! answers `null` and nothing is inferable from it. `--all` returns the
//! coalesced values, which is where the `tags:` block that drives enablement
//! lives.
//!
//! Values are read as *intent*, not as a description of the cluster. A component
//! the values enable but the cluster does not have is a **failure** — that is
//! precisely the bug an E2E suite exists to catch. Only a component the values
//! genuinely disable is skipped.

use std::process::Command;

use anyhow::{Context, Result, bail};
use serde_json::Value;

/// A subchart, its values key, and the tags that enable it.
///
/// Transcribed from `dependencies:` in the chart's `Chart.yaml`. Helm's rule is
/// that tags are OR'd, and that a `condition:` pointing at an existing values
/// path overrides them entirely — the chart leaves each `<key>.enabled` key
/// commented out so the default path is tag-driven, and uncommenting it force-
/// includes or force-excludes that subchart.
///
/// Keep this in step with `Chart.yaml`. A dependency added there and missed here
/// is not a broken build, it is an assertion that silently never runs.
pub struct Component {
    /// Values key, which is also the `condition:` prefix. For an aliased
    /// dependency this is the alias, not the upstream chart name.
    pub key: &'static str,
    pub tags: &'static [&'static str],
}

pub const COMPONENTS: &[Component] = &[
    Component {
        key: "alloy-agent",
        tags: &["default", "pipeline", "alloy-agent"],
    },
    Component {
        key: "alloy-gateway",
        tags: &["default", "pipeline", "alloy-gateway"],
    },
    Component {
        key: "loki",
        tags: &["default", "bundled-backends", "loki"],
    },
    Component {
        key: "thanos",
        tags: &["default", "bundled-backends", "thanos"],
    },
    Component {
        key: "grafana",
        tags: &["default", "managed-grafana", "grafana-standalone"],
    },
    Component {
        key: "grafana-operator",
        tags: &["default", "managed-grafana", "grafana-operator"],
    },
    Component {
        key: "alertmanager",
        tags: &["default", "bundled-backends", "alertmanager"],
    },
    Component {
        key: "kube-state-metrics",
        tags: &["default", "cluster-metrics", "kube-state-metrics"],
    },
    Component {
        key: "node-exporter",
        tags: &["default", "cluster-metrics", "node-exporter"],
    },
    // No `default` tag, deliberately: metrics-server is commonly already
    // installed cluster-wide, and two of them fight over the same APIService.
    Component {
        key: "metrics-server",
        tags: &["cluster-metrics", "metrics-server"],
    },
];

/// The coalesced values of the release under test.
pub struct Features {
    values: Value,
}

impl Features {
    /// Shell out to `helm get values --all` for `release` in `namespace`.
    ///
    /// Shelling out rather than decoding the release Secret directly: the Secret
    /// format is a Helm implementation detail, and every environment that runs
    /// this suite already has the CLI.
    pub fn load(namespace: &str, release: &str, context: Option<&str>) -> Result<Self> {
        let mut cmd = Command::new("helm");
        cmd.args([
            "get", "values", "--all", "-o", "json", "-n", namespace, release,
        ]);
        if let Some(context) = context {
            cmd.args(["--kube-context", context]);
        }

        let output = cmd
            .output()
            .context("running `helm get values` (is helm on PATH?)")?;
        if !output.status.success() {
            bail!(
                "`helm get values --all -n {namespace} {release}` failed: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }

        let values: Value = serde_json::from_slice(&output.stdout)
            .context("parsing `helm get values --all -o json` output")?;
        Ok(Self::from_values(values))
    }

    pub fn from_values(values: Value) -> Self {
        Self { values }
    }

    /// Look up a dotted values path. Chart keys contain `-`, never `.`, so a
    /// plain split is unambiguous here.
    pub fn get(&self, path: &str) -> Option<&Value> {
        path.split('.')
            .try_fold(&self.values, |node, key| node.get(key))
    }

    pub fn string(&self, path: &str) -> Option<&str> {
        self.get(path).and_then(Value::as_str)
    }

    /// Whether `key` names a subchart this release enabled.
    ///
    /// Mirrors Helm: an explicit `<key>.enabled` boolean wins outright,
    /// otherwise any true tag enables it.
    pub fn enabled(&self, key: &str) -> bool {
        let Some(component) = COMPONENTS.iter().find(|c| c.key == key) else {
            // A caller asking about a component not in the table is a bug in
            // this suite, not a disabled component — say so rather than
            // silently skipping the assertions that depend on it.
            panic!("no component named {key:?} in COMPONENTS; add it from Chart.yaml");
        };

        if let Some(explicit) = self.get(&format!("{key}.enabled")).and_then(Value::as_bool) {
            return explicit;
        }

        component.tags.iter().any(|tag| self.tag(tag))
    }

    fn tag(&self, tag: &str) -> bool {
        self.get(&format!("tags.{tag}"))
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// The tenant the logging pipeline writes to, sent as `X-Scope-OrgID` on
    /// reads.
    ///
    /// The bundled Loki runs `auth_enabled: true`, so a read without this header
    /// fails with `no org id` no matter what was ingested — a failure that looks
    /// like an empty stack unless you know to look for it.
    pub fn loki_tenant(&self) -> &str {
        self.string("pipeline.logging.tenancy.staticTenant")
            .unwrap_or("loki")
    }

    /// Loki's deployment mode, which decides what its Services are called.
    pub fn loki_deployment_mode(&self) -> &str {
        self.string("loki.deploymentMode").unwrap_or("Distributed")
    }

    /// Read, write and ingester Service name candidates for the current
    /// deployment mode, most likely first.
    ///
    /// SingleBinary renders exactly one `loki` Service; the microservice modes
    /// split reads and writes across several. Resolved by probing this list
    /// rather than trusting the mode alone, because a profile can repoint the
    /// Services independently of it — `loki-test` does exactly that.
    pub fn loki_service_candidates(&self) -> LokiServices {
        match self.loki_deployment_mode() {
            "SingleBinary" => LokiServices {
                read: vec!["loki"],
                ingester: vec!["loki"],
            },
            "SimpleScalable" => LokiServices {
                read: vec!["loki-read", "loki-query-frontend", "loki"],
                ingester: vec!["loki-write", "loki"],
            },
            // Distributed, and anything unrecognized: the chart defaults.
            _ => LokiServices {
                read: vec!["loki-query-frontend", "loki-read", "loki"],
                ingester: vec!["loki-ingester", "loki-write", "loki"],
            },
        }
    }
}

/// Loki Service names by role. Reads and the ingester are separate Services in
/// every mode but SingleBinary, and `loki_ingester_streams_created_total` is
/// only exposed by the ingester.
pub struct LokiServices {
    pub read: Vec<&'static str>,
    pub ingester: Vec<&'static str>,
}

#[cfg(test)]
mod tests {
    use super::{COMPONENTS, Features};
    use serde_json::json;

    fn with_tags(tags: serde_json::Value) -> Features {
        Features::from_values(json!({ "tags": tags }))
    }

    /// The values the tier-1 profiles actually produce, so this test fails if
    /// the OR-logic drifts from what that cluster is known to run.
    #[test]
    fn tier1_tags_resolve_to_the_tier1_stack() {
        let features = with_tags(json!({
            "default": false,
            "pipeline": true,
            "loki": true,
            "managed-grafana": true,
            "kube-state-metrics": true,
            "bundled-backends": false,
            "cluster-metrics": false,
        }));

        for on in [
            "loki",
            "alloy-agent",
            "alloy-gateway",
            "grafana",
            "grafana-operator",
        ] {
            assert!(features.enabled(on), "{on} should be enabled");
        }
        for off in ["thanos", "alertmanager", "node-exporter", "metrics-server"] {
            assert!(!features.enabled(off), "{off} should be disabled");
        }
    }

    /// `tags.default` reaches every dependency that carries it — which is all of
    /// them but `metrics-server`, deliberately: two metrics-servers in a cluster
    /// fight over the same APIService.
    #[test]
    fn default_tag_enables_everything_but_metrics_server() {
        let features = with_tags(json!({ "default": true }));

        for component in COMPONENTS {
            let expected = component.key != "metrics-server";
            assert_eq!(
                features.enabled(component.key),
                expected,
                "{} under tags.default alone",
                component.key
            );
        }
    }

    /// Helm evaluates `condition:` before `tags:`, and the chart points each one
    /// at `<key>.enabled` as a circuit breaker. Force-*off* is the direction
    /// that matters here: read wrong, the suite asserts against a component the
    /// operator deliberately excluded.
    #[test]
    fn explicit_enabled_overrides_the_tags() {
        let forced_off = Features::from_values(json!({
            "tags": { "default": true },
            "thanos": { "enabled": false },
        }));
        assert!(!forced_off.enabled("thanos"));

        let forced_on = Features::from_values(json!({
            "tags": { "default": false },
            "thanos": { "enabled": true },
        }));
        assert!(forced_on.enabled("thanos"));
    }

    /// A `thanos:` block with no `enabled` key is the normal case — the chart
    /// leaves it commented out — and must fall through to the tags rather than
    /// reading as disabled.
    #[test]
    fn a_values_block_without_enabled_falls_through_to_tags() {
        let features = Features::from_values(json!({
            "tags": { "default": true },
            "thanos": { "deploymentMode": "whatever" },
        }));
        assert!(features.enabled("thanos"));
    }

    /// Absent tags are false, not an error: `helm get values` without `--all`
    /// returns only what the caller supplied, and this is what that degrades to.
    #[test]
    fn missing_tags_block_disables_everything() {
        let features = Features::from_values(json!({}));
        for component in COMPONENTS {
            assert!(!features.enabled(component.key), "{}", component.key);
        }
    }

    #[test]
    fn loki_services_follow_the_deployment_mode() {
        let single = Features::from_values(json!({ "loki": { "deploymentMode": "SingleBinary" } }));
        assert_eq!(single.loki_service_candidates().read, vec!["loki"]);

        // The chart default, and what an unset mode has to fall back to.
        let distributed = Features::from_values(json!({}));
        assert_eq!(
            distributed.loki_service_candidates().read.first(),
            Some(&"loki-query-frontend")
        );
    }

    #[test]
    fn loki_tenant_follows_the_pipeline() {
        let features = Features::from_values(json!({
            "pipeline": { "logging": { "tenancy": { "staticTenant": "audit" } } },
        }));
        assert_eq!(features.loki_tenant(), "audit");
    }
}
