// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! A Grafana dashboard's [`manifest`](crate::manifest), read back from its render.
//!
//! A panel's builder never sees the slug it is placed under, and a rendered
//! panel doesn't carry the registry ids it was built from. So both come from the
//! render: slugs from the layout's element references, in layout order, and ids
//! by rendering every registry query in the dashboard's own context and matching
//! each panel's expressions against them.

use std::collections::HashMap;

use mzmon_lib::grafana::context::{DashboardScope, dashboard_context};
use mzmon_lib::grafana::generated::dashboardv2;
use mzmon_lib::query::{QueryEngine, QueryRegistry};
use serde_json::Value;

use crate::manifest::{Kind, Panel, Unit};

/// The panels of `dashboard`, in layout order.
///
/// `scope` has to be the one the dashboard was built in, or no panel's
/// expressions match their registry query and every panel reads no queries.
pub fn manifest(
    dashboard: &dashboardv2::Dashboard,
    registry: &QueryRegistry,
    scope: &DashboardScope,
) -> Vec<Panel> {
    let by_expressions = rendered_queries(registry, scope);
    let spec = serde_json::to_value(dashboard).expect("a dashboard serializes");
    let mut names = Vec::new();
    element_names(&spec["layout"], &mut names);
    names
        .iter()
        .filter_map(|name| {
            let element = &spec["elements"][name.as_str()];
            (element["kind"] == "Panel").then(|| panel(name, &element["spec"], &by_expressions))
        })
        .collect()
}

/// Every registry query that renders in the dashboard's contexts, keyed by its
/// normalized expressions.
fn rendered_queries(
    registry: &QueryRegistry,
    scope: &DashboardScope,
) -> HashMap<Vec<String>, String> {
    let mut out = HashMap::new();
    for engine in [QueryEngine::PromQl, QueryEngine::LogQl] {
        let ctx = dashboard_context(registry, engine, scope);
        for query in registry.queries() {
            if let Ok(exprs) = query.render(&ctx) {
                let key = exprs.iter().map(|e| normalize(e)).collect();
                out.entry(key).or_insert_with(|| query.id.clone());
            }
        }
    }
    out
}

fn panel(slug: &str, spec: &Value, by_expressions: &HashMap<Vec<String>, String>) -> Panel {
    let exprs: Vec<String> = spec["data"]["spec"]["queries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|q| q["spec"]["query"]["spec"]["expr"].as_str())
        .map(normalize)
        .collect();
    let queries = match by_expressions.get(&exprs) {
        Some(id) => vec![id.clone()],
        // Not one registry query: name each expression's own, where it has one.
        None => {
            let mut ids: Vec<String> = Vec::new();
            for expr in &exprs {
                if let Some(id) = by_expressions.get(std::slice::from_ref(expr))
                    && !ids.contains(id)
                {
                    ids.push(id.clone());
                }
            }
            ids
        }
    };
    let viz = &spec["vizConfig"];
    Panel {
        slug: slug.to_string(),
        title: spec["title"].as_str().unwrap_or_default().to_string(),
        queries,
        kind: kind(viz["group"].as_str().unwrap_or_default()),
        unit: unit(viz["spec"]["fieldConfig"]["defaults"]["unit"].as_str()),
    }
}

/// Element references, depth first, which is layout order.
fn element_names(layout: &Value, out: &mut Vec<String>) {
    match layout {
        Value::Object(map) => {
            if map.get("kind").and_then(Value::as_str) == Some("ElementReference")
                && let Some(name) = map.get("name").and_then(Value::as_str)
            {
                out.push(name.to_string());
                return;
            }
            for value in map.values() {
                element_names(value, out);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| element_names(item, out)),
        _ => {}
    }
}

fn kind(plugin: &str) -> Kind {
    match plugin {
        "timeseries" => Kind::TimeSeries,
        "stat" | "gauge" => Kind::Value,
        "table" => Kind::Table,
        "piechart" => Kind::Pie,
        "barchart" => Kind::Ranking,
        other => Kind::Other(other.to_string()),
    }
}

fn unit(unit: Option<&str>) -> Unit {
    match unit.unwrap_or_default() {
        "" | "short" | "none" => Unit::None,
        "s" => Unit::Seconds,
        "bytes" => Unit::Bytes,
        "Bps" => Unit::BytesPerSecond,
        "percent" | "percentunit" => Unit::Percent,
        "cores" => Unit::Cores,
        "cps" | "pps" | "ops" | "reqps" => Unit::PerSecond,
        other => Unit::Other(other.to_string()),
    }
}

/// Whitespace isn't significant in a query, and the registry wraps long ones.
fn normalize(expr: &str) -> String {
    expr.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grafana::env_top;
    use crate::grafana::queries::test_registry;

    #[test]
    fn every_env_top_panel_resolves_to_its_registry_query() {
        let dashboard = env_top::build("mz_", test_registry()).expect("build").spec;
        let panels = manifest(
            &dashboard,
            test_registry(),
            &DashboardScope::for_prefix("mz_"),
        );
        let lag = panels
            .iter()
            .find(|p| p.slug == "freshness-lag-by-cluster")
            .expect("placed");
        assert_eq!(
            lag.queries,
            vec!["materialize.compute.freshness.lag_by_cluster"]
        );
        assert_eq!(lag.kind, Kind::TimeSeries);
        assert_eq!(lag.unit, Unit::Seconds);
        let unresolved: Vec<&str> = panels
            .iter()
            .filter(|p| p.queries.is_empty())
            .map(|p| p.slug.as_str())
            .collect();
        assert!(
            unresolved.is_empty(),
            "no registry query for {unresolved:?}"
        );
    }
}
