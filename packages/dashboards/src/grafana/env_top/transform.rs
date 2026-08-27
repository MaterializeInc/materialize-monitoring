// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Panel transformations.
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
pub fn labels_to_fields(keep: &[&str]) -> dashboardv2::TransformationKind {
    raw("labelsToFields", serde_json::json!({ "keepLabels": keep }))
}

/// Merge the frames a multi-series query produces into one table.
pub fn merge() -> dashboardv2::TransformationKind {
    raw("merge", serde_json::json!({}))
}

/// Reduce each series to one row of calculations.
pub fn reduce(calcs: &[&str]) -> dashboardv2::TransformationKind {
    raw(
        "reduce",
        serde_json::json!({ "reducers": calcs, "includeTimeField": false }),
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

/// Extract structured fields out of one column.
pub fn extract_fields(source: &str) -> dashboardv2::TransformationKind {
    raw(
        "extractFields",
        serde_json::json!({ "source": source, "replace": false }),
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
    fn sort_by_omits_desc_when_ascending() {
        let asc = serde_json::to_value(&sort_by("name", false).spec.options).expect("serialize");
        assert!(asc["sort"][0].get("desc").is_none());
        let desc = serde_json::to_value(&sort_by("name", true).spec.options).expect("serialize");
        assert_eq!(desc["sort"][0]["desc"], true);
    }
}
