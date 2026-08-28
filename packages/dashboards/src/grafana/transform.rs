// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Panel transformations, shared by every dashboard.
//!
//! These build Grafana transformation JSON and know nothing about Materialize, so
//! they sit beside the dashboards rather than inside one. They lived under
//! `env_top/` while it was the only dashboard; `upgrade`'s version table is the
//! second consumer, and copying them would have been the start of two divergent
//! copies of the same unschematized JSON.
//!
//! Grafana's transformation options are unschematized — the dashboard schema types
//! `TransformationSpec::options` as an open object, so there is nothing to
//! generate types from and every option set is hand-shaped JSON. These helpers
//! wrap the handful the dashboard uses so a panel reads as a pipeline rather than
//! a wall of `json!`.
//!
//! Only seven panels in the baseline transform at all, and four distinct chains
//! between them, so this stays small on purpose.

use mzmon_lib::grafana::generated::dashboardv2;

/// `TransformationKind::kind` discriminant.
const KIND: &str = "Transformation";

/// Build a transformation from its Grafana id and options.
pub fn raw(id: &str, options: serde_json::Value) -> dashboardv2::TransformationKind {
    dashboardv2::TransformationKind {
        kind: KIND.to_string(),
        group: id.to_string(),
        spec: dashboardv2::TransformationSpec {
            options: match options {
                serde_json::Value::Object(map) => map,
                _ => Default::default(),
            },
            disabled: None,
            filter: None,
            topic: None,
        },
    }
}

/// Promote the named labels to columns.
///
/// An empty `keep` promotes every label: Grafana reads a missing `keepLabels` as
/// "no restriction", so the option is omitted rather than sent as an empty list.
pub fn labels_to_fields(keep: &[&str]) -> dashboardv2::TransformationKind {
    if keep.is_empty() {
        return raw("labelsToFields", serde_json::json!({}));
    }
    raw("labelsToFields", serde_json::json!({ "keepLabels": keep }))
}

/// Merge the frames a multi-series query produces into one table.
pub fn merge() -> dashboardv2::TransformationKind {
    raw("merge", serde_json::json!({}))
}

/// Reduce each series to one row of calculations.
///
/// `seriesToRows` mode: one row per series, one column per reducer. The default
/// mode instead produces one row per *reducer*, which reads as a summary table
/// rather than a per-collection breakdown.
pub fn reduce(calcs: &[&str]) -> dashboardv2::TransformationKind {
    raw(
        "reduce",
        serde_json::json!({ "mode": "seriesToRows", "reducers": calcs }),
    )
}

/// Hide columns and set their order.
///
/// `order` is the column sequence; anything in `exclude` is dropped. `Time` and
/// the raw metric column are almost always excluded — a labels-to-fields table
/// carries them but they say nothing.
pub fn organize(exclude: &[&str], order: &[&str]) -> dashboardv2::TransformationKind {
    let exclude_by_name: serde_json::Map<String, serde_json::Value> = exclude
        .iter()
        .map(|name| (name.to_string(), serde_json::json!(true)))
        .collect();
    let index_by_name: serde_json::Map<String, serde_json::Value> = order
        .iter()
        .enumerate()
        .map(|(index, name)| (name.to_string(), serde_json::json!(index)))
        .collect();
    raw(
        "organize",
        serde_json::json!({ "excludeByName": exclude_by_name, "indexByName": index_by_name }),
    )
}

/// Hide, order, and rename columns in one transform.
///
/// Grafana models all three as one `organize` step, so a panel that needs two of
/// them cannot chain two calls — the second would replace the first.
pub fn organize_full(
    exclude: &[&str],
    order: &[&str],
    renames: &[(&str, &str)],
) -> dashboardv2::TransformationKind {
    let mut options = serde_json::Map::new();
    if !exclude.is_empty() {
        options.insert(
            "excludeByName".to_string(),
            serde_json::Value::Object(
                exclude
                    .iter()
                    .map(|name| (name.to_string(), serde_json::json!(true)))
                    .collect(),
            ),
        );
    }
    if !order.is_empty() {
        options.insert(
            "indexByName".to_string(),
            serde_json::Value::Object(
                order
                    .iter()
                    .enumerate()
                    .map(|(index, name)| (name.to_string(), serde_json::json!(index)))
                    .collect(),
            ),
        );
    }
    if !renames.is_empty() {
        options.insert(
            "renameByName".to_string(),
            serde_json::Value::Object(
                renames
                    .iter()
                    .map(|(from, to)| (from.to_string(), serde_json::json!(to)))
                    .collect(),
            ),
        );
    }
    raw("organize", serde_json::Value::Object(options))
}

/// [`organize`], also renaming columns.
///
/// A pivoted table names its row column `<row>\\<column>` — literally, with a
/// backslash — which is not something to show a reader, so the rename is usually
/// mandatory rather than cosmetic after a [`grouping_to_matrix`].
pub fn organize_renamed(
    order: &[&str],
    renames: &[(&str, &str)],
) -> dashboardv2::TransformationKind {
    organize_full(&[], order, renames)
}

/// Sort rows by one field.
pub fn sort_by(field: &str, descending: bool) -> dashboardv2::TransformationKind {
    let mut sort = serde_json::Map::new();
    sort.insert("field".to_string(), serde_json::json!(field));
    if descending {
        sort.insert("desc".to_string(), serde_json::json!(true));
    }
    raw(
        "sortBy",
        serde_json::json!({ "fields": {}, "sort": [serde_json::Value::Object(sort)] }),
    )
}

/// Pivot a label into columns, filling gaps with `empty_value`.
pub fn grouping_to_matrix(
    row: &str,
    column: &str,
    value: &str,
    empty_value: &str,
) -> dashboardv2::TransformationKind {
    raw(
        "groupingToMatrix",
        serde_json::json!({
            "rowField": row,
            "columnField": column,
            "valueField": value,
            "emptyValue": empty_value,
        }),
    )
}

/// Extract structured fields out of one column, letting Grafana sniff the format.
pub fn extract_fields(source: &str) -> dashboardv2::TransformationKind {
    raw(
        "extractFields",
        serde_json::json!({ "source": source, "replace": false }),
    )
}

/// Extract fields out of one column with an explicit regex.
///
/// Every named capture group becomes a field, which is how a value that is really
/// several values in a string — a version carrying its commit — gets split into
/// columns a panel or a data link can address. `regexp` is passed through
/// verbatim, so it needs the delimiting slashes Grafana's parser expects.
pub fn extract_fields_regex(source: &str, regexp: &str) -> dashboardv2::TransformationKind {
    raw(
        "extractFields",
        serde_json::json!({
            "source": source,
            "format": "regexp",
            "regExp": regexp,
            "replace": false,
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_transformation_carries_its_grafana_id_as_the_group() {
        let t = merge();
        assert_eq!(t.kind, "Transformation");
        assert_eq!(t.group, "merge");
        assert!(t.spec.options.is_empty());
    }

    #[test]
    fn organize_maps_order_to_indices_and_exclusions_to_flags() {
        let t = organize(&["Time", "value"], &["name", "size"]);
        let options = serde_json::to_value(&t.spec.options).expect("serialize");
        assert_eq!(options["indexByName"]["name"], 0);
        assert_eq!(options["indexByName"]["size"], 1);
        assert_eq!(options["excludeByName"]["Time"], true);
        assert_eq!(options["excludeByName"]["value"], true);
    }

    #[test]
    fn organize_renamed_carries_both_maps() {
        let t = organize_renamed(&["a", "b"], &[("a", "Application")]);
        let options = serde_json::to_value(&t.spec.options).expect("serialize");
        assert_eq!(options["indexByName"]["a"], 0);
        assert_eq!(options["renameByName"]["a"], "Application");
    }

    #[test]
    fn sort_by_omits_desc_when_ascending() {
        let asc = serde_json::to_value(&sort_by("name", false).spec.options).expect("serialize");
        assert!(asc["sort"][0].get("desc").is_none());
        let desc = serde_json::to_value(&sort_by("name", true).spec.options).expect("serialize");
        assert_eq!(desc["sort"][0]["desc"], true);
    }
}
