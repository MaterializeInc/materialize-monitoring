// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Per-field config overrides.
//!
//! A panel's `fieldConfig.defaults` applies to every field; an override applies to
//! a matched subset. Only two panels in the baseline use one, both to
//! threshold-colour a single table column, so this stays deliberately narrow.

use mzmon_lib::grafana::generated::dashboardv2;

/// Match one field by its (possibly renamed) column name.
///
/// Note that Grafana's `byName` matcher takes the field name as a **bare string**,
/// not an object — the schema types `MatcherConfig::options` as `object`, which is
/// wrong, and `bin/gen-grafana-models.sh` widens it for exactly this reason.
pub fn by_name(field: &str) -> Builder {
    Builder {
        matcher: dashboardv2::MatcherConfig {
            id: "byName".to_string(),
            options: Some(serde_json::json!(field)),
            scope: None,
        },
        properties: Vec::new(),
    }
}

/// An override under construction.
pub struct Builder {
    matcher: dashboardv2::MatcherConfig,
    properties: Vec<dashboardv2::DynamicConfigValue>,
}

impl Builder {
    /// Set an arbitrary field property.
    pub fn property(mut self, id: &str, value: serde_json::Value) -> Self {
        self.properties.push(dashboardv2::DynamicConfigValue {
            id: id.to_string(),
            value: match value {
                serde_json::Value::Object(map) => map,
                // A scalar property still has to arrive as the schema's map type;
                // wrapping it keeps the generated type honest.
                other => {
                    let mut map = serde_json::Map::new();
                    map.insert("value".to_string(), other);
                    map
                }
            },
        });
        self
    }

    /// Threshold-colour this field independently of the panel's own thresholds.
    pub fn thresholds(self, thresholds: dashboardv2::ThresholdsConfig) -> Self {
        let value = serde_json::to_value(thresholds).unwrap_or(serde_json::Value::Null);
        self.property("thresholds", value)
    }

    /// Fill the cell background with the threshold colour, rather than the text.
    ///
    /// On a table of counts this is what makes a bad column visible at a glance
    /// instead of needing to be read.
    pub fn color_background_cells(self) -> Self {
        self.property(
            "custom.cellOptions",
            serde_json::json!({ "type": "color-background" }),
        )
    }

    pub fn build(self) -> dashboardv2::FieldConfigSourceOverridesItem {
        dashboardv2::FieldConfigSourceOverridesItem {
            matcher: self.matcher,
            properties: self.properties,
            system_ref: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mzmon_lib::grafana::threshold;

    #[test]
    fn by_name_matches_on_a_bare_string() {
        // The schema says `options` is an object; Grafana wants a string, which is
        // the discrepancy the generated models are patched for.
        let built = by_name("Errors").build();
        assert_eq!(built.matcher.id, "byName");
        assert_eq!(built.matcher.options, Some(serde_json::json!("Errors")));
    }

    #[test]
    fn thresholds_and_cell_colouring_land_as_properties() {
        let built = by_name("Errors")
            .thresholds(threshold::errors(1.0, 10.0).build())
            .color_background_cells()
            .build();
        let ids: Vec<&str> = built.properties.iter().map(|p| p.id.as_str()).collect();
        assert_eq!(ids, vec!["thresholds", "custom.cellOptions"]);
        let thresholds = serde_json::to_value(&built.properties[0].value).expect("serialize");
        // Carries the explicit base step, like every other ladder.
        assert!(thresholds["steps"][0]["value"].is_null());
    }
}
