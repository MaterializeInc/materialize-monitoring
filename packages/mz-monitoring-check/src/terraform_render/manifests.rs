// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The rendered chart, parsed.
//!
//! Assertions read structure rather than lines wherever they can. The chart
//! emits its validators' warnings as YAML comments that quote the very settings
//! they warn about, and several components carry the same key name in different
//! places, so a line match passes on the warning that says a setting is absent,
//! or on the one component that happened to get it right. Parsing drops the
//! comments and keeps the places apart.
//!
//! The raw text is kept as well, for the one question it answers better: whether
//! a credential appears *anywhere* in the release, comments included.

use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow};
use base64::Engine;
use serde_json::Value;

use super::ctx::{lookup, parse_yaml};
use super::setup::RELEASE;

pub const WORKLOAD_KINDS: &[&str] = &["Deployment", "StatefulSet", "DaemonSet"];

pub struct Manifests {
    docs: Vec<Manifest>,
    text: String,
}

pub struct Manifest {
    pub value: Value,
    /// This document's source text, as `helm template` wrote it.
    pub text: String,
}

impl Manifests {
    /// Split `helm template` output on its document separators and parse each.
    pub fn parse(text: String) -> Result<Self> {
        let mut docs = Vec::new();
        let mut chunk = String::new();
        let mut flush = |chunk: &mut String| -> Result<()> {
            let source = std::mem::take(chunk);
            let value = parse_yaml(&source).with_context(|| {
                let origin = source
                    .lines()
                    .find(|line| line.starts_with("# Source:"))
                    .unwrap_or("a document with no `# Source:` line");
                format!("in {origin}")
            })?;
            if value.is_object() {
                docs.push(Manifest {
                    value,
                    text: source,
                });
            }
            Ok(())
        };
        for line in text.split_inclusive('\n') {
            if line.trim_end() == "---" {
                flush(&mut chunk)?;
            } else {
                chunk.push_str(line);
            }
        }
        flush(&mut chunk)?;
        Ok(Self { docs, text })
    }

    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Manifest> {
        self.docs.iter()
    }

    pub fn of_kind<'a>(&'a self, kind: &'a str) -> impl Iterator<Item = &'a Manifest> {
        self.iter().filter(move |doc| doc.kind() == kind)
    }

    pub fn find(&self, kind: &str, name: &str) -> Option<&Manifest> {
        self.iter()
            .find(|doc| doc.kind() == kind && doc.name() == name)
    }

    /// Every Deployment, StatefulSet and DaemonSet.
    pub fn workloads(&self) -> impl Iterator<Item = &Manifest> {
        self.iter()
            .filter(|doc| WORKLOAD_KINDS.contains(&doc.kind()))
    }

    /// The whole release as `helm template` wrote it, comments included.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// The entry `key` of the ConfigMap or Secret named `name`.
    pub fn config_entry(&self, name: &str, key: &str) -> Option<String> {
        self.iter()
            .filter(|doc| matches!(doc.kind(), "ConfigMap" | "Secret") && doc.name() == name)
            .find_map(|doc| doc.entry(key))
    }

    /// Loki's own configuration file.
    ///
    /// A ConfigMap by default, and a Secret when `configStorageType` moves it
    /// there to keep static credentials out of a ConfigMap.
    pub fn loki_config(&self) -> Result<Value> {
        let text = self
            .config_entry("loki", "config.yaml")
            .ok_or_else(|| anyhow!("no `loki` ConfigMap or Secret with a config.yaml rendered"))?;
        parse_yaml(&text).context("parsing Loki's config.yaml")
    }

    /// The Alloy gateway's pipeline.
    pub fn gateway_config(&self) -> Result<String> {
        let name = format!("{RELEASE}-alloy-gateway");
        self.config_entry(&name, "config.alloy")
            .ok_or_else(|| anyhow!("no {name} ConfigMap with a config.alloy rendered"))
    }

    /// The environment the gateway's pipeline reads its settings from with
    /// `sys.env`.
    pub fn gateway_env(&self) -> Result<BTreeMap<String, String>> {
        let name = format!("{RELEASE}-alloy-gateway-env");
        let doc = self
            .find("ConfigMap", &name)
            .ok_or_else(|| anyhow!("no {name} ConfigMap rendered"))?;
        Ok(doc.entries())
    }
}

impl Manifest {
    pub fn kind(&self) -> &str {
        self.str_at("kind").unwrap_or("")
    }

    pub fn name(&self) -> &str {
        self.str_at("metadata.name").unwrap_or("")
    }

    pub fn namespace(&self) -> Option<&str> {
        self.str_at("metadata.namespace")
    }

    pub fn get(&self, path: &str) -> Option<&Value> {
        lookup(&self.value, path)
    }

    pub fn str_at(&self, path: &str) -> Option<&str> {
        self.get(path).and_then(Value::as_str)
    }

    /// `Kind/name`, for messages.
    pub fn id(&self) -> String {
        format!("{}/{}", self.kind(), self.name())
    }

    /// A workload's pod spec.
    pub fn pod_spec(&self) -> Option<&Value> {
        self.get("spec.template.spec")
    }

    /// A workload's containers, init containers excluded.
    pub fn containers(&self) -> impl Iterator<Item = &Value> {
        self.pod_spec()
            .and_then(|spec| spec.get("containers"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
    }

    /// Every argument of every container.
    pub fn container_args(&self) -> impl Iterator<Item = &str> {
        self.containers()
            .filter_map(|container| container.get("args")?.as_array())
            .flatten()
            .filter_map(Value::as_str)
    }

    /// A ConfigMap's `data`, or a Secret's `stringData` and decoded `data`.
    pub fn entries(&self) -> BTreeMap<String, String> {
        let mut out = BTreeMap::new();
        let plain = |field: &str| {
            self.get(field)
                .and_then(Value::as_object)
                .into_iter()
                .flatten()
                .filter_map(|(key, value)| Some((key.clone(), value.as_str()?.to_owned())))
        };
        match self.kind() {
            "ConfigMap" => out.extend(plain("data")),
            "Secret" => {
                out.extend(
                    self.get("data")
                        .and_then(Value::as_object)
                        .into_iter()
                        .flatten()
                        .filter_map(|(key, value)| {
                            let decoded = base64::engine::general_purpose::STANDARD
                                .decode(value.as_str()?)
                                .ok()?;
                            Some((key.clone(), String::from_utf8(decoded).ok()?))
                        }),
                );
                out.extend(plain("stringData"));
            }
            _ => {}
        }
        out
    }

    pub fn entry(&self, key: &str) -> Option<String> {
        self.entries().remove(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RENDERED: &str = "\
---
# Source: chart/templates/a.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: loki
data:
  config.yaml: |
    server:
      http_listen_port: 3100
---
# Source: chart/templates/empty.yaml
# nothing rendered here
---
# Source: chart/templates/b.yaml
apiVersion: v1
kind: Secret
metadata:
  name: creds
data:
  key: aGVsbG8=
stringData:
  other: plain
";

    #[test]
    fn splits_documents_and_skips_empty_ones() {
        let manifests = Manifests::parse(RENDERED.to_owned()).unwrap();
        assert_eq!(manifests.len(), 2);
        assert_eq!(manifests.iter().next().unwrap().id(), "ConfigMap/loki");
        assert!(manifests.iter().nth(1).unwrap().text.contains("aGVsbG8="));
    }

    #[test]
    fn secret_entries_are_decoded() {
        let manifests = Manifests::parse(RENDERED.to_owned()).unwrap();
        assert_eq!(
            manifests.config_entry("creds", "key").as_deref(),
            Some("hello")
        );
        assert_eq!(
            manifests.config_entry("creds", "other").as_deref(),
            Some("plain")
        );
    }

    #[test]
    fn loki_config_is_parsed() {
        let manifests = Manifests::parse(RENDERED.to_owned()).unwrap();
        let config = manifests.loki_config().unwrap();
        assert_eq!(
            lookup(&config, "server.http_listen_port"),
            Some(&3100.into())
        );
    }
}
