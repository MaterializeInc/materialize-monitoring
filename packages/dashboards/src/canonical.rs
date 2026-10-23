// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Canonical JSON for the checked-in artifacts, shared by every backend.
//!
//! Every rendered dashboard is checked in, so its bytes have to be a function of
//! its content and nothing else. Both backends build a `serde_json::Value` and
//! pass it through [`canonical`] before writing it out.

/// Put the tree in canonical form: keys sorted, whole numbers written as integers.
///
/// The artifacts are checked in, so their shape has to be a function of their
/// content and nothing else.
///
/// Sorting reaches further than choosing an ordered map for the generated structs:
/// the free-form `options` and `spec` blocks are `serde_json::Map`, whose order is
/// insertion order under this crate's `preserve_order`, and insertion order is
/// whatever the panel builder happened to do.
///
/// The number pass is about the Rust types rather than about determinism. Grafana's
/// schemas type most numerics as `number`, so typify gives them `f64`, and an `f64`
/// serializes as `3.0` even when it holds a count. JSON and YAML draw no
/// int/float distinction and Grafana reads either, but a `maxColumnCount: 3.0` reads
/// as a mistake, so an integral value is written as an integer.
pub fn canonical(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<(String, serde_json::Value)> = map.into_iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.cmp(b));
            serde_json::Value::Object(
                entries
                    .into_iter()
                    .map(|(key, value)| (key, canonical(value)))
                    .collect(),
            )
        }
        // Arrays keep their order: it is the panel and row order a reader sees.
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(canonical).collect())
        }
        serde_json::Value::Number(number) => serde_json::Value::Number(canonical_number(number)),
        scalar => scalar,
    }
}

/// Narrow a whole-valued float to an integer, leaving everything else alone.
///
/// Guarded on an exact round trip rather than on range checks: a float too large to
/// hold an integer exactly keeps its float form instead of being silently rewritten
/// to a different number.
fn canonical_number(number: serde_json::Number) -> serde_json::Number {
    let Some(float) = number.as_f64() else {
        // Already an integer.
        return number;
    };
    if float.fract() != 0.0 || !float.is_finite() {
        return number;
    }
    let narrowed = float as i64;
    if narrowed as f64 != float {
        return number;
    }
    serde_json::Number::from(narrowed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sorting_is_recursive_and_leaves_arrays_alone() {
        let input = serde_json::json!({
            "b": 1,
            "a": { "z": [3, 1, 2], "y": { "second": 2, "first": 1 } },
        });
        let out = serde_json::to_string(&canonical(input)).expect("serialize");
        assert_eq!(
            out, r#"{"a":{"y":{"first":1,"second":2},"z":[3,1,2]},"b":1}"#,
            "objects sort, arrays do not"
        );
    }

    #[test]
    fn whole_floats_narrow_to_integers_and_fractions_do_not() {
        // `maxColumnCount` is an f64 because the schema types it `number`, and a
        // column count written `3.0` reads as a mistake.
        let input = serde_json::json!({
            "count": 3.0,
            "ratio": 20.8,
            "zero": 0.0,
            "negative": -5.0,
            "already": 7,
            "huge": 1e300,
        });
        let out = serde_json::to_string(&canonical(input)).expect("serialize");
        assert_eq!(
            out,
            r#"{"already":7,"count":3,"huge":1e+300,"negative":-5,"ratio":20.8,"zero":0}"#
        );
    }

    #[test]
    fn narrowing_never_changes_a_value() {
        // The guard that matters: a float too large to hold an integer exactly must
        // keep its float form rather than being rewritten to a different number.
        for float in [
            0.0,
            -0.0,
            1.0,
            -1.0,
            20.8,
            i64::MAX as f64,
            i64::MIN as f64,
            1e300,
            f64::MIN_POSITIVE,
        ] {
            let input = serde_json::json!(float);
            let out = canonical(input);
            let round_tripped = out.as_f64().expect("still a number");
            assert_eq!(round_tripped, float, "narrowing changed {float}");
        }
    }
}
