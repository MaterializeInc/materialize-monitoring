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
//!
//! [`ALL`] is the registry the renderer walks, so adding a dashboard is a module
//! plus one entry — nothing in the CLI needs to know their names.

pub mod env_logs;
pub mod env_top;
pub mod env_upgrade;
pub mod infra_logs;
pub mod queries;
pub mod render;
pub mod transform;
pub mod volume_guard;

use mzmon_lib::grafana::dashboard::Resource;
use mzmon_lib::query::QueryRegistry;

/// Everything a dashboard needs from its deployment.
#[derive(Debug, Clone)]
pub struct Options {
    /// `mz_` on self-managed, `v2_mz_` on Materialize Cloud. Reaches the
    /// cluster-discovery variable, which reads a SQL-derived metric.
    pub sql_metric_prefix: String,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            sql_metric_prefix: "mz_".to_string(),
        }
    }
}

/// A dashboard the renderer can emit.
pub struct Renderable {
    /// Artifact stem: the chart and the docsite both key off it, so it is a stable
    /// identifier rather than a title.
    pub name: &'static str,
    /// One-line summary, for `--list`.
    pub summary: &'static str,
    pub render: fn(&Options, &QueryRegistry) -> render::Result<Resource>,
}

/// Every dashboard, in artifact order.
pub const ALL: &[Renderable] = &[
    Renderable {
        name: env_top::NAME_STEM,
        summary: "High-level overview of one Materialize environment",
        render: env_top::render,
    },
    Renderable {
        name: env_logs::NAME_STEM,
        summary: "Logs and Kubernetes events for a Materialize deployment",
        render: env_logs::render,
    },
    Renderable {
        name: infra_logs::NAME_STEM,
        summary: "Logs and Kubernetes events for the platform underneath Materialize",
        render: infra_logs::render,
    },
    Renderable {
        name: env_upgrade::NAME_STEM,
        summary: "What happened during a Materialize upgrade",
        render: env_upgrade::render,
    },
];

/// Look one up by artifact stem.
pub fn find(name: &str) -> Option<&'static Renderable> {
    ALL.iter().find(|d| d.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every row title in a rendered dashboard.
    fn row_titles(spec: &mzmon_lib::grafana::generated::dashboardv2::Dashboard) -> Vec<String> {
        fn collect(value: &serde_json::Value, out: &mut Vec<String>) {
            match value {
                serde_json::Value::Object(map) => {
                    if map.get("kind").and_then(|k| k.as_str()) == Some("RowsLayoutRow")
                        && let Some(title) = map["spec"].get("title").and_then(|t| t.as_str())
                    {
                        out.push(title.to_string());
                    }
                    for v in map.values() {
                        collect(v, out);
                    }
                }
                serde_json::Value::Array(items) => {
                    for v in items {
                        collect(v, out);
                    }
                }
                _ => {}
            }
        }
        let json = serde_json::to_value(&spec.layout).unwrap_or(serde_json::Value::Null);
        let mut out = Vec::new();
        collect(&json, &mut out);
        out
    }

    /// Every tab title in a rendered dashboard.
    fn tab_titles(spec: &mzmon_lib::grafana::generated::dashboardv2::Dashboard) -> Vec<String> {
        fn collect(value: &serde_json::Value, out: &mut Vec<String>) {
            match value {
                serde_json::Value::Object(map) => {
                    if map.get("kind").and_then(|k| k.as_str()) == Some("TabsLayoutTab")
                        && let Some(title) = map["spec"].get("title").and_then(|t| t.as_str())
                    {
                        out.push(title.to_string());
                    }
                    for v in map.values() {
                        collect(v, out);
                    }
                }
                serde_json::Value::Array(items) => {
                    for v in items {
                        collect(v, out);
                    }
                }
                _ => {}
            }
        }
        let json = serde_json::to_value(&spec.layout).unwrap_or(serde_json::Value::Null);
        let mut out = Vec::new();
        collect(&json, &mut out);
        out
    }

    /// The left-hand side of every `_Where -> What_` italic reference.
    ///
    /// Only the arrow form is checked. A bare `_Currently Hydrating_` names a
    /// *panel*, and validating those would need panel titles from dashboards a
    /// reference may legitimately point across.
    fn arrow_references(description: &str) -> Vec<String> {
        // Code spans first: `mz_compute_cluster_status` is full of underscores
        // that would otherwise parse as italic delimiters.
        let prose: String = description
            .split('`')
            .step_by(2)
            .collect::<Vec<_>>()
            .join(" ");

        let mut out = Vec::new();
        let mut rest = prose.as_str();
        while let Some(start) = rest.find('_') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('_') else { break };
            let italic = &after[..end];
            if let Some((where_, _what)) = italic.split_once("->") {
                out.push(where_.trim().to_string());
            }
            rest = &after[end + 1..];
        }
        out
    }

    #[test]
    fn no_description_points_somewhere_that_does_not_exist() {
        // Descriptions navigate by naming a destination as `_Where -> What_`, and
        // `Where` is either a tab or a row. Either way it has to exist somewhere
        // in the dashboard set -- a reference may cross dashboards, since the
        // upgrade dashboard legitimately points at the overview's tabs.
        //
        // One test over the registry rather than a copy per dashboard: this was
        // three near-identical tests and their three helper triples before the
        // third dashboard made that untenable.
        let options = Options::default();
        let registry = queries::test_registry();

        let rendered: Vec<Resource> = ALL
            .iter()
            .map(|d| (d.render)(&options, registry).expect("render"))
            .collect();

        let mut destinations = Vec::new();
        for resource in &rendered {
            destinations.extend(tab_titles(&resource.spec));
            destinations.extend(row_titles(&resource.spec));
        }
        destinations.push(env_top::theme::SUMMARY_TITLE.to_string());

        let mut broken = Vec::new();
        for (dashboard, resource) in ALL.iter().zip(&rendered) {
            for (name, element) in &resource.spec.elements {
                let mzmon_lib::grafana::generated::dashboardv2::Element::PanelKind(panel) = element
                else {
                    continue;
                };
                for destination in arrow_references(&panel.spec.description) {
                    if !destinations.contains(&destination) {
                        broken.push(format!(
                            "{}/{name}: _{destination} -> …_ is neither a tab nor a row",
                            dashboard.name
                        ));
                    }
                }
            }
        }
        broken.sort();
        assert!(
            broken.is_empty(),
            "{} broken cross-reference(s):\n  {}",
            broken.len(),
            broken.join("\n  ")
        );
    }

    #[test]
    fn every_dashboard_is_findable_by_its_stem() {
        for dashboard in ALL {
            assert!(find(dashboard.name).is_some(), "{}", dashboard.name);
        }
        assert!(find("nope").is_none());
    }

    #[test]
    fn stems_are_unique_and_filename_safe() {
        // The stem becomes a filename and a Helm `Files.Glob` key.
        let mut seen = std::collections::HashSet::new();
        for dashboard in ALL {
            assert!(
                seen.insert(dashboard.name),
                "duplicate stem {}",
                dashboard.name
            );
            assert!(
                dashboard
                    .name
                    .bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'),
                "{} is not a safe filename stem",
                dashboard.name
            );
        }
    }
}
