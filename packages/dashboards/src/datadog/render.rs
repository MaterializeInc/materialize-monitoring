// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Rendering Datadog dashboards to the JSON the docsite offers.
//!
//! One consumer, one format: `docs/assets/dashboards/datadog/`, which an operator
//! downloads and imports. The artifacts are checked in, so the render goes
//! through [`crate::canonical`] for the same byte-stability the Grafana render
//! has.

use datadog_api_client::datadogV1::model::Dashboard;
use mzmon_lib::query::QueryRegistry;

use super::{Options, Renderable};

/// The artifacts' filename suffix.
pub const EXTENSION: &str = "json";

/// The artifacts' filename prefix, before the stem the Grafana counterpart's file
/// shares.
///
/// A dashboards release uploads every checked-in JSON under its file name, so
/// `env-top.json` can't name both renders. It also keeps a downloaded Datadog
/// file apart from the Grafana one of the same stem.
pub const FILE_PREFIX: &str = "datadog-";

/// What went wrong rendering.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Panels named registry queries that did not render for Datadog.
    #[error("building {name}: {} registry lookup(s) failed:\n  {}", failures.len(), failures.join("\n  "))]
    Registry {
        name: &'static str,
        failures: Vec<String>,
    },

    #[error("serializing {name}: {source}")]
    Serialize {
        name: &'static str,
        #[source]
        source: serde_json::Error,
    },
}

/// Result of a render.
pub type Result<T> = std::result::Result<T, Error>;

/// Render one dashboard to a string.
pub fn render(
    dashboard: &Renderable,
    options: &Options,
    registry: &QueryRegistry,
) -> Result<String> {
    let built = (dashboard.render)(options, registry)?;
    serialize(dashboard.name, &built)
}

/// Serialize a built dashboard deterministically.
pub fn serialize(name: &'static str, dashboard: &Dashboard) -> Result<String> {
    let value =
        serde_json::to_value(dashboard).map_err(|source| Error::Serialize { name, source })?;
    let mut out = serde_json::to_string_pretty(&crate::canonical::canonical(value))
        .map_err(|source| Error::Serialize { name, source })?;
    // The repo's end-of-file hook wants a trailing newline.
    out.push('\n');
    Ok(out)
}

#[cfg(test)]
mod tests {
    use datadog_api_client::datadogV1::model::{
        FormulaAndFunctionQueryDefinition, Widget, WidgetDefinition, WidgetFormula,
    };

    use super::*;
    use crate::datadog::ALL;
    use crate::grafana::queries::test_registry;

    fn rendered() -> Vec<(&'static str, String)> {
        let options = Options::default();
        ALL.iter()
            .map(|d| {
                let json = render(d, &options, test_registry())
                    .unwrap_or_else(|e| panic!("{}: {e}", d.name));
                (d.name, json)
            })
            .collect()
    }

    /// One widget request's queries and formulas.
    type Request = (Vec<FormulaAndFunctionQueryDefinition>, Vec<WidgetFormula>);

    /// Every widget request in a widget tree, failing on any widget that
    /// deserialized as `UnparsedObject` rather than a typed variant.
    fn requests(widgets: &[Widget], name: &str, out: &mut Vec<Request>) {
        fn parts(
            queries: &Option<Vec<FormulaAndFunctionQueryDefinition>>,
            formulas: &Option<Vec<WidgetFormula>>,
        ) -> Request {
            (
                queries.clone().unwrap_or_default(),
                formulas.clone().unwrap_or_default(),
            )
        }
        for widget in widgets {
            match &widget.definition {
                WidgetDefinition::GroupWidgetDefinition(group) => {
                    requests(&group.widgets, name, out)
                }
                WidgetDefinition::QueryValueWidgetDefinition(w) => {
                    out.extend(w.requests.iter().map(|r| parts(&r.queries, &r.formulas)))
                }
                WidgetDefinition::TimeseriesWidgetDefinition(w) => {
                    out.extend(w.requests.iter().map(|r| parts(&r.queries, &r.formulas)))
                }
                WidgetDefinition::ToplistWidgetDefinition(w) => {
                    out.extend(w.requests.iter().map(|r| parts(&r.queries, &r.formulas)))
                }
                WidgetDefinition::SunburstWidgetDefinition(w) => {
                    out.extend(w.requests.iter().map(|r| parts(&r.queries, &r.formulas)))
                }
                WidgetDefinition::TableWidgetDefinition(w) => {
                    out.extend(w.requests.iter().map(|r| parts(&r.queries, &r.formulas)))
                }
                WidgetDefinition::NoteWidgetDefinition(_) => {}
                WidgetDefinition::UnparsedObject(raw) => {
                    panic!("{name}: a widget the models do not type: {raw:?}")
                }
                other => panic!("{name}: a widget kind these tests do not walk yet: {other:?}"),
            }
        }
    }

    /// Every metric query string in a widget tree, failing on any query that
    /// deserialized as anything but a typed metric query.
    fn metric_queries(widgets: &[Widget], name: &str, out: &mut Vec<String>) {
        let mut all = Vec::new();
        requests(widgets, name, &mut all);
        for (queries, _) in all {
            for query in queries {
                match query {
                    FormulaAndFunctionQueryDefinition::FormulaAndFunctionMetricQueryDefinition(
                        m,
                    ) => out.push(m.query.clone()),
                    other => panic!("{name}: an untyped or unexpected query: {other:?}"),
                }
            }
        }
    }

    #[test]
    fn every_dashboard_renders() {
        for (name, json) in rendered() {
            assert!(json.ends_with('\n'), "{name} lacks a trailing newline");
            assert!(!json.contains("%%{"), "{name} has an unrendered parameter");
        }
    }

    #[test]
    fn rendering_is_byte_stable() {
        assert_eq!(
            rendered(),
            rendered(),
            "two renders of the same source differ"
        );
    }

    #[test]
    fn every_dashboard_round_trips_through_the_typed_models() {
        // The artifact is what an operator imports, so it has to be a shape the
        // crate's models describe, not one that only survives as raw JSON.
        for (name, json) in rendered() {
            let parsed: Dashboard = serde_json::from_str(&json)
                .unwrap_or_else(|e| panic!("{name} does not parse as a Dashboard: {e}"));
            assert!(
                parsed.additional_properties.is_empty(),
                "{name} carries fields the model does not type: {:?}",
                parsed.additional_properties.keys().collect::<Vec<_>>()
            );
            let mut queries = Vec::new();
            metric_queries(&parsed.widgets, name, &mut queries);
            assert_eq!(
                serialize(name, &parsed).expect("serialize"),
                json,
                "{name} changed on a round trip"
            );
        }
    }

    #[test]
    fn every_variable_a_query_names_is_declared() {
        for (name, json) in rendered() {
            let parsed: Dashboard = serde_json::from_str(&json).expect("parse");
            let declared: Vec<String> = parsed
                .template_variables
                .flatten()
                .unwrap_or_default()
                .into_iter()
                .map(|v| v.name)
                .collect();
            let mut queries = Vec::new();
            metric_queries(&parsed.widgets, name, &mut queries);
            assert!(!queries.is_empty(), "{name} has no metric queries");
            for query in &queries {
                for variable in referenced_variables(query) {
                    assert!(
                        declared.contains(&variable),
                        "{name}: `{query}` names ${variable}, which the dashboard does not declare"
                    );
                }
            }
        }
    }

    #[test]
    fn every_widget_query_is_bare_and_every_formula_names_one() {
        // Datadog draws a widget whose query object carries a function as
        // "Invalid Formula in Query Object", although its query API evaluates
        // the same expression. Functions and operators belong in the formula.
        for (name, json) in rendered() {
            let parsed: Dashboard = serde_json::from_str(&json).expect("parse");
            let mut all = Vec::new();
            requests(&parsed.widgets, name, &mut all);
            assert!(!all.is_empty(), "{name} has no widget requests to check");
            for (queries, formulas) in all {
                let mut names = Vec::new();
                for query in &queries {
                    if let FormulaAndFunctionQueryDefinition::FormulaAndFunctionMetricQueryDefinition(m) = query {
                        assert!(
                            mzmon_lib::datadog::formula::is_bare_metric_query(&m.query),
                            "{name}: `{}` is not a bare metric query",
                            m.query
                        );
                        names.push(m.name.clone());
                    }
                }
                assert!(!formulas.is_empty(), "{name}: a request with no formula");
                for formula in formulas {
                    for used in formula
                        .formula
                        .split(|c: char| !c.is_ascii_alphanumeric())
                        .filter(|word| word.starts_with("query"))
                    {
                        assert!(
                            names.iter().any(|n| n == used),
                            "{name}: `{}` names {used}, which its request does not define",
                            formula.formula
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_scalar_widget_reduces_each_query_exactly_once() {
        // A scalar widget reduces over the window either in each query's
        // aggregator or, under a group-by function, in the formula's `reduce()`.
        // Datadog refuses an aggregator under a group-by function, and a query
        // with neither is "Missing aggregator".
        fn walk(widgets: &[Widget], name: &str) {
            for widget in widgets {
                let (requests, scalar) = match &widget.definition {
                    WidgetDefinition::GroupWidgetDefinition(group) => {
                        walk(&group.widgets, name);
                        continue;
                    }
                    WidgetDefinition::TimeseriesWidgetDefinition(_) => {
                        let mut all = Vec::new();
                        requests(std::slice::from_ref(widget), name, &mut all);
                        (all, false)
                    }
                    _ => {
                        let mut all = Vec::new();
                        requests(std::slice::from_ref(widget), name, &mut all);
                        (all, true)
                    }
                };
                for (queries, formulas) in requests {
                    for formula in &formulas {
                        let layered =
                            mzmon_lib::datadog::formula::aggregates_in_layers(&formula.formula);
                        let reduced = formula.formula.contains("reduce(");
                        assert_eq!(
                            scalar && layered,
                            reduced,
                            "{name}: `{}` in a {} widget",
                            formula.formula,
                            if scalar { "scalar" } else { "timeseries" }
                        );
                    }
                    for query in &queries {
                        let FormulaAndFunctionQueryDefinition::FormulaAndFunctionMetricQueryDefinition(m) = query else {
                            continue;
                        };
                        let named_by_a_layered_formula = formulas.iter().any(|f| {
                            mzmon_lib::datadog::formula::aggregates_in_layers(&f.formula)
                                && f.formula
                                    .split(|c: char| !c.is_ascii_alphanumeric())
                                    .any(|word| word == m.name)
                        });
                        assert_eq!(
                            m.aggregator.is_some(),
                            scalar && !named_by_a_layered_formula,
                            "{name}: {} `{}` has the wrong aggregator",
                            m.name,
                            m.query
                        );
                    }
                }
            }
        }
        for (name, json) in rendered() {
            let parsed: Dashboard = serde_json::from_str(&json).expect("parse");
            walk(&parsed.widgets, name);
        }
    }

    #[test]
    fn every_top_level_widget_sits_in_exactly_one_tab() {
        for (name, json) in rendered() {
            let parsed: Dashboard = serde_json::from_str(&json).expect("parse");
            let Some(Some(tabs)) = &parsed.tabs else {
                continue;
            };
            let mut placed: Vec<i64> = tabs.iter().flat_map(|t| t.widget_ids.clone()).collect();
            let count = placed.len();
            placed.sort_unstable();
            placed.dedup();
            assert_eq!(placed.len(), count, "{name}: a widget sits in two tabs");
            let mut ids: Vec<i64> = parsed
                .widgets
                .iter()
                .map(|w| {
                    w.id.unwrap_or_else(|| panic!("{name}: a top-level widget with no id"))
                })
                .collect();
            ids.sort_unstable();
            assert_eq!(
                ids, placed,
                "{name}: the tabs and the top-level widgets disagree"
            );
        }
    }

    /// The `$name` references in a Datadog metric query.
    fn referenced_variables(query: &str) -> Vec<String> {
        query
            .split('$')
            .skip(1)
            .map(|rest| {
                rest.chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect::<String>()
            })
            .filter(|name| !name.is_empty())
            .collect()
    }
}
