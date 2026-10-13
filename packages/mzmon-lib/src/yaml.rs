// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The workspace's one YAML entry point, so every crate parses and emits YAML
//! with the same `serde-saphyr` settings.
//!
//! Generated YAML is committed under `charts/*/pre-rendered/` and `docs/assets/`,
//! so [`to_string`] is tuned for stable, reviewable diffs rather than for the
//! library's defaults.

use serde::{Deserialize, Serialize};
pub use serde_saphyr::{Error, SerializeError};

/// Consecutive comment lines the parser may buffer before a collection entry.
///
/// The library default of 32 is a denial-of-service guard for untrusted input.
/// Every document parsed here is checked into this repository, and the alloy
/// pipelines carry long comment blocks above single stages.
const MAX_BUFFERED_COMMENT_EVENTS: usize = 1024;

/// Parse one YAML document.
pub fn from_str<'de, T: Deserialize<'de>>(input: &'de str) -> Result<T, Error> {
    let mut budget = serde_saphyr::Budget::default();
    budget.max_buffered_comment_events = MAX_BUFFERED_COMMENT_EVENTS;
    let mut options = serde_saphyr::Options::default();
    options.budget = Some(budget);
    serde_saphyr::from_str_with_options(input, options)
}

/// Serialize `value` as one YAML document.
///
/// Two departures from the library's output, both for readable committed files:
///
/// * Long single-line strings stay on one line. Left on, the library folds any
///   string over 80 characters into a `>` block, which splits PromQL and panel
///   text across lines and breaks grepping for them.
/// * Blank lines inside `|` blocks carry no indentation. The library pads them to
///   the block's indent, which is trailing whitespace that `.editorconfig` asks
///   editors to trim.
pub fn to_string<T: Serialize>(value: &T) -> Result<String, SerializeError> {
    let mut options = serde_saphyr::SerializerOptions::default();
    options.folded_wrap_chars = usize::MAX;
    let out = serde_saphyr::to_string_with_options(value, options)?;
    Ok(strip_blank_line_indent(out))
}

/// Empty the whitespace-only lines of `yaml`, unless that changes what it parses to.
///
/// A whitespace-only line inside a block scalar is an empty line when it is no
/// deeper than the block's indentation, which is all the serializer pads it with.
/// It is content only when the string itself has a line of spaces; the re-parse
/// catches that case, and the padded output is kept for it.
fn strip_blank_line_indent(yaml: String) -> String {
    let mut stripped = String::with_capacity(yaml.len());
    for line in yaml.split_inclusive('\n') {
        let body = line.trim_end_matches('\n');
        if !body.is_empty() && body.bytes().all(|b| b == b' ') {
            stripped.push_str(&line[body.len()..]);
        } else {
            stripped.push_str(line);
        }
    }
    if stripped.len() == yaml.len() {
        return yaml;
    }
    let same = matches!(
        (
            from_str::<serde_json::Value>(&yaml),
            from_str::<serde_json::Value>(&stripped),
        ),
        (Ok(a), Ok(b)) if a == b
    );
    if same { stripped } else { yaml }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn blank_lines_in_literal_blocks_carry_no_indent() {
        let value = BTreeMap::from([("text", "first\n\nsecond")]);
        let out = to_string(&value).unwrap();
        assert_eq!(out, "text: |-\n  first\n\n  second\n");
        let back: BTreeMap<String, String> = from_str(&out).unwrap();
        assert_eq!(back["text"], "first\n\nsecond");
    }

    #[test]
    fn whitespace_content_lines_keep_their_spaces() {
        // A line of spaces that is part of the string must survive the strip.
        let value = BTreeMap::from([("text", "first\n    \nsecond")]);
        let out = to_string(&value).unwrap();
        let back: BTreeMap<String, String> = from_str(&out).unwrap();
        assert_eq!(back["text"], "first\n    \nsecond");
    }

    #[test]
    fn long_strings_stay_on_one_line() {
        let long = "word ".repeat(40);
        let long = long.trim_end();
        let value = BTreeMap::from([("text", long)]);
        assert_eq!(to_string(&value).unwrap(), format!("text: {long}\n"));
    }

    #[test]
    fn long_comment_runs_parse() {
        let mut yaml = String::from("list:\n");
        for _ in 0..100 {
            yaml.push_str("  # comment\n");
        }
        yaml.push_str("  - item\n");
        let parsed: BTreeMap<String, Vec<String>> = from_str(&yaml).unwrap();
        assert_eq!(parsed["list"], ["item"]);
    }
}
