// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Rendering dashboards to the artifacts that ship.
//!
//! Two consumers, two formats. The Helm chart reads YAML out of
//! `charts/materialize-monitoring-dashboards/pre-rendered/dashboards/grafana/`,
//! and the docsite serves JSON out of `docs/assets/dashboards/grafana/`. Both are
//! checked in, which makes byte-stability the property that matters most here: an
//! artifact that reorders between two identical builds turns every regeneration
//! into an unreviewable diff.
//!
//! So both formats go through [`canonical`] first. `serde_json` is built with
//! `preserve_order` in this crate, so rebuilding every object with its keys
//! inserted in sorted order is what fixes the output — and it fixes the free-form
//! `options` and `spec` maps too, which no amount of choosing the right map type
//! for the generated structs would reach.

use mzmon_lib::grafana::dashboard::Resource;
use mzmon_lib::query::QueryRegistry;

pub use crate::canonical::canonical;

use super::{Options, Renderable};

/// What went wrong rendering.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A dashboard could not be built.
    #[error("building {name}: {source}")]
    Build {
        name: &'static str,
        #[source]
        source: mzmon_lib::grafana::dashboard::Error,
    },

    #[error("serializing {name} as {format}: {source}")]
    Serialize {
        name: &'static str,
        format: Format,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

/// Result of a render.
pub type Result<T> = std::result::Result<T, Error>;

/// Output format, which is really "which consumer is this for".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// YAML, for the Helm chart's `pre-rendered/` tree.
    Yaml,
    /// JSON, for the docsite's downloadable assets.
    Json,
}

impl Format {
    /// File extension, which is also the artifact's filename suffix.
    pub fn extension(self) -> &'static str {
        match self {
            Format::Yaml => "yaml",
            Format::Json => "json",
        }
    }
}

impl std::fmt::Display for Format {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.extension())
    }
}

/// Render one dashboard to a string.
pub fn render(
    dashboard: &Renderable,
    options: &Options,
    registry: &QueryRegistry,
    format: Format,
) -> Result<String> {
    let resource = (dashboard.render)(options, registry)?;
    serialize(dashboard.name, &resource, format)
}

/// Serialize a built dashboard deterministically.
pub fn serialize(name: &'static str, resource: &Resource, format: Format) -> Result<String> {
    let value = serde_json::to_value(resource).map_err(|source| Error::Serialize {
        name,
        format,
        source: Box::new(source),
    })?;
    let value = canonical(value);

    let mut out = match format {
        Format::Json => {
            serde_json::to_string_pretty(&value).map_err(|source| Error::Serialize {
                name,
                format,
                source: Box::new(source),
            })?
        }
        Format::Yaml => mzmon_lib::yaml::to_string(&value).map_err(|source| Error::Serialize {
            name,
            format,
            source: Box::new(source),
        })?,
    };
    // YAML from serde already ends in a newline; JSON does not, and a file without
    // a trailing newline trips the repo's end-of-file hook.
    if !out.ends_with('\n') {
        out.push('\n');
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana;

    #[test]
    fn no_rendered_number_is_written_as_a_whole_float() {
        // The property the artifacts depend on, checked against the real render
        // rather than a hand-built value.
        let options = Options::default();
        for dashboard in grafana::ALL {
            let json = render(
                dashboard,
                &options,
                crate::grafana::queries::test_registry(),
                Format::Json,
            )
            .expect("render");
            for (index, line) in json.lines().enumerate() {
                assert!(
                    !line.trim_end_matches(',').ends_with(".0"),
                    "{} line {}: whole float in the artifact: {}",
                    dashboard.name,
                    index + 1,
                    line.trim()
                );
            }
        }
    }

    #[test]
    fn every_dashboard_renders_in_both_formats() {
        let options = Options::default();
        for dashboard in grafana::ALL {
            for format in [Format::Yaml, Format::Json] {
                let out = render(
                    dashboard,
                    &options,
                    crate::grafana::queries::test_registry(),
                    format,
                )
                .unwrap_or_else(|e| panic!("{} as {format}: {e}", dashboard.name));
                assert!(
                    out.ends_with('\n'),
                    "{} as {format} lacks a trailing newline",
                    dashboard.name
                );
                assert!(
                    out.len() > 1000,
                    "{} as {format} looks empty",
                    dashboard.name
                );
            }
        }
    }

    #[test]
    fn rendering_is_byte_stable() {
        // The property the checked-in artifacts depend on. Two builds of the same
        // source must not differ, or every regeneration churns the diff.
        let options = Options::default();
        for dashboard in grafana::ALL {
            for format in [Format::Yaml, Format::Json] {
                let first = render(
                    dashboard,
                    &options,
                    crate::grafana::queries::test_registry(),
                    format,
                )
                .expect("render");
                let second = render(
                    dashboard,
                    &options,
                    crate::grafana::queries::test_registry(),
                    format,
                )
                .expect("render");
                assert_eq!(
                    first, second,
                    "{} as {format} is not stable",
                    dashboard.name
                );
            }
        }
    }

    #[test]
    fn the_rendered_keys_are_sorted() {
        let options = Options::default();
        let json = render(
            &grafana::ALL[0],
            &options,
            crate::grafana::queries::test_registry(),
            Format::Json,
        )
        .expect("render");
        let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
        assert_sorted(&value, "");
    }

    fn assert_sorted(value: &serde_json::Value, path: &str) {
        if let serde_json::Value::Object(map) = value {
            let keys: Vec<&String> = map.keys().collect();
            let mut want = keys.clone();
            want.sort();
            assert_eq!(keys, want, "keys out of order at {path}");
            for (key, child) in map {
                assert_sorted(child, &format!("{path}.{key}"));
            }
        } else if let serde_json::Value::Array(items) = value {
            for (index, child) in items.iter().enumerate() {
                assert_sorted(child, &format!("{path}[{index}]"));
            }
        }
    }

    #[test]
    fn every_dashboard_renders_and_records_no_target_cloud() {
        // One variant per dashboard now. The `cloud` option and its
        // `target-cloud` annotation are gone: the clouds stopped differing in
        // panel content once `alloy-gateway` began scraping the kubelet's
        // cAdvisor directly instead of consuming GKE's reduced subset, so the
        // second artifact recorded nothing but its own name.
        let options = Options::default();
        for dashboard in grafana::ALL {
            let json = render(
                dashboard,
                &options,
                crate::grafana::queries::test_registry(),
                Format::Json,
            )
            .unwrap_or_else(|e| panic!("{}: {e}", dashboard.name));
            let value: serde_json::Value = serde_json::from_str(&json).expect("parse");
            assert!(
                value["metadata"]["annotations"]
                    .get("monitoring.materialize.cloud/target-cloud")
                    .is_none(),
                "{} still records a target cloud",
                dashboard.name
            );
        }
    }

    #[test]
    fn the_yaml_and_json_renders_carry_the_same_content() {
        // Two consumers, one dashboard: the formats must not drift.
        let options = Options::default();
        let dashboard = &grafana::ALL[0];
        let yaml: serde_json::Value = mzmon_lib::yaml::from_str(
            &render(
                dashboard,
                &options,
                crate::grafana::queries::test_registry(),
                Format::Yaml,
            )
            .expect("yaml"),
        )
        .expect("parse yaml");
        let json: serde_json::Value = serde_json::from_str(
            &render(
                dashboard,
                &options,
                crate::grafana::queries::test_registry(),
                Format::Json,
            )
            .expect("json"),
        )
        .expect("parse json");
        assert_eq!(yaml, json);
    }
}
