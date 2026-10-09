// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Assertions over a rendered dashboard that several dashboards make.
//!
//! Each of these started as a test inside one dashboard's module and was copied
//! into the next. The external-dependency dashboards are the first three to share
//! them from here; the older copies are left where they are.

use mzmon_lib::grafana::dashboard::Resource;
use mzmon_lib::grafana::generated::dashboardv2::Element;
use mzmon_lib::grafana::variable;
use serde_json::Value;

/// Grafana's own variables, which no dashboard defines.
const BUILTINS: &[&str] = &["__rate_interval", "__range", "__interval", "__auto"];

/// Every query expression on every panel, with the panel's name.
fn expressions(resource: &Resource) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for (name, element) in &resource.spec.elements {
        let Element::PanelKind(panel) = element else {
            continue;
        };
        for query in &panel.spec.data.spec.queries {
            let Some(spec) = query.spec.query.spec.as_ref() else {
                continue;
            };
            if let Some(expr) = spec.get("expr").and_then(Value::as_str) {
                out.push((name.clone(), expr.to_string()));
            }
        }
    }
    out
}

/// Every `$name` and `${name…}` in `expr`, skipping `label_replace` captures.
fn dollar_references(expr: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = expr;
    while let Some(pos) = rest.find('$') {
        let after = rest[pos + 1..].trim_start_matches('{');
        let len = after
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        if len > 0 && !after[..len].bytes().all(|b| b.is_ascii_digit()) {
            out.push(after[..len].to_string());
        }
        rest = &after[len..];
    }
    out
}

/// Assert that every variable a query names is one the dashboard defines.
///
/// An undefined Grafana variable interpolates to nothing, the selector matches
/// no series, and the panel renders empty and correct-looking.
///
/// A variable declared on a row or tab counts only for the panels inside it.
pub(crate) fn assert_variables_defined(resource: &Resource) {
    let defined: Vec<&str> = resource
        .spec
        .variables
        .iter()
        .map(variable::name_of)
        .collect();
    let sections = mzmon_lib::grafana::layout::section_variables(&resource.spec.layout);
    let mut missing = Vec::new();
    for (panel, expr) in expressions(resource) {
        let scoped = sections.get(&panel).map(Vec::as_slice).unwrap_or_default();
        for reference in dollar_references(&expr) {
            if !BUILTINS.contains(&reference.as_str())
                && !defined.contains(&reference.as_str())
                && !scoped.contains(&reference)
            {
                missing.push(format!("{panel}: ${reference}"));
            }
        }
    }
    missing.sort();
    missing.dedup();
    assert!(
        missing.is_empty(),
        "undefined variables (defined: {defined:?}):\n  {}",
        missing.join("\n  ")
    );
}

/// Assert that every panel with a query sets its own empty-state text.
pub(crate) fn assert_every_panel_has_no_value(resource: &Resource) {
    for (name, element) in &resource.spec.elements {
        let Element::PanelKind(panel) = element else {
            continue;
        };
        // Text panels carry no query and no field config to put it on.
        if panel.spec.data.spec.queries.is_empty() {
            continue;
        }
        let json = serde_json::to_string(&panel.spec.viz_config).expect("serialize");
        assert!(json.contains("noValue"), "{name} has no empty-state text");
    }
}

/// Assert that no timeseries drawing several lines pins them to one shade.
///
/// `shade` derives every series from one hue, which makes the lines of a
/// multi-series graph indistinguishable.
pub(crate) fn assert_multi_series_panels_unshaded(resource: &Resource) {
    let json = serde_json::to_value(&resource.spec.elements).expect("serialize");
    for (name, element) in json.as_object().expect("elements") {
        let Some(panel) = element.get("spec") else {
            continue;
        };
        if panel.pointer("/vizConfig/group").and_then(Value::as_str) != Some("timeseries") {
            continue;
        }
        let Some(queries) = panel
            .pointer("/data/spec/queries")
            .and_then(Value::as_array)
        else {
            continue;
        };
        let templated = queries.iter().any(|q| {
            q.pointer("/spec/query/spec/legendFormat")
                .and_then(Value::as_str)
                .is_some_and(|l| l.contains("{{"))
        });
        if queries.len() <= 1 && !templated {
            continue;
        }
        let mode = panel.pointer("/vizConfig/spec/fieldConfig/defaults/color/mode");
        assert_ne!(
            mode.and_then(Value::as_str),
            Some("shades"),
            "{name} draws several series but pins them to one shade"
        );
    }
}

/// The title and condition of every row rendered on a variable.
///
/// `(title, operator, value)` for each `RowsLayoutRow` whose conditional
/// rendering names a variable. Rows guarded by a time range are skipped.
pub(crate) fn variable_conditioned_rows(resource: &Resource) -> Vec<(String, String, String)> {
    fn collect(value: &Value, out: &mut Vec<Value>) {
        match value {
            Value::Object(map) => {
                if map.get("kind").and_then(Value::as_str) == Some("RowsLayoutRow") {
                    out.push(value.clone());
                }
                for v in map.values() {
                    collect(v, out);
                }
            }
            Value::Array(items) => {
                for v in items {
                    collect(v, out);
                }
            }
            _ => {}
        }
    }
    let json = serde_json::to_value(&resource.spec.layout).expect("serialize");
    let mut rows = Vec::new();
    collect(&json, &mut rows);
    rows.iter()
        .filter_map(|row| {
            let spec = row.get("spec")?;
            let title = spec.get("title")?.as_str()?.to_string();
            let item = spec
                .pointer("/conditionalRendering/spec/items/0")?
                .get("spec")?;
            item.get("variable")?;
            Some((
                title,
                item.get("operator")?.as_str()?.to_string(),
                item.get("value")?.as_str()?.to_string(),
            ))
        })
        .collect()
}

/// Assert that no variable a row condition reads has a custom "All" value.
///
/// Grafana evaluates a row condition against the variable's `getValue()`. For a
/// multi-value variable on "All" with a custom value, that is a `CustomAllValue`
/// object with no `toString()`, so a `matches` condition tests its regex against
/// `[object Object]`: every conditional row hides and the negated fallback
/// shows, on exactly the selection that should show everything. Without a
/// custom value, "All" is the array of discovered values, which the condition
/// tests one by one.
pub(crate) fn assert_row_conditions_can_read_all(resource: &Resource) {
    use mzmon_lib::grafana::generated::dashboardv2::VariableKind;

    let conditioned: Vec<String> = {
        let json = serde_json::to_value(&resource.spec.layout).expect("serialize");
        let mut names = Vec::new();
        fn collect(value: &Value, out: &mut Vec<String>) {
            match value {
                Value::Object(map) => {
                    if map.get("kind").and_then(Value::as_str)
                        == Some("ConditionalRenderingVariable")
                        && let Some(name) = map
                            .get("spec")
                            .and_then(|s| s.get("variable"))
                            .and_then(Value::as_str)
                    {
                        out.push(name.to_string());
                    }
                    for v in map.values() {
                        collect(v, out);
                    }
                }
                Value::Array(items) => {
                    for v in items {
                        collect(v, out);
                    }
                }
                _ => {}
            }
        }
        collect(&json, &mut names);
        names.sort();
        names.dedup();
        names
    };
    assert!(!conditioned.is_empty(), "no row is rendered on a variable");
    for variable in &resource.spec.variables {
        if let VariableKind::QueryVariableKind(v) = variable
            && conditioned.contains(&v.spec.name)
        {
            assert!(
                v.spec.all_value.is_none(),
                "rows are rendered on ${}, whose custom \"All\" value a row condition reads as [object Object]",
                v.spec.name
            );
        }
    }
}

/// Every metric selector in `json` whose metric name starts with `prefix`, from
/// the name to its closing brace.
///
/// Only a name followed directly by `{` is a selector; the same name in a
/// panel description is prose, and is skipped.
pub(crate) fn selectors_of<'a>(json: &'a str, prefix: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(found) = json[from..].find(prefix) {
        let start = from + found;
        let name_len = json[start..]
            .bytes()
            .take_while(|b| b.is_ascii_alphanumeric() || *b == b'_')
            .count();
        let after = start + name_len;
        if json[after..].starts_with('{')
            && let Some(close) = json[after..].find('}')
        {
            out.push(&json[start..=after + close]);
        }
        from = start + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dollar_references_read_both_spellings_and_skip_captures() {
        assert_eq!(
            dollar_references(r#"x{a=~"$one", b=~"${two:regex}"} "$1""#),
            vec!["one".to_string(), "two".to_string()]
        );
    }
}
