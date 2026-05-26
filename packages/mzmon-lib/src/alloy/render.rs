// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use crate::alloy::ast::{AttributeValue, Block};
use crate::alloy::error::{Error, Result};

use std::fmt::Write;

const INDENT: &str = "\t";

/// Prefix each subsequent non-empty line of `s` with an indent
///
/// TODO: consider handling indents more closely to their renderers
fn indent_trailing_content(s: &str) -> Result<String> {
    let mut out = String::new();
    // First line does not get indented (and we want indents between each line)
    let mut first_line = true;
    // HACK: if we see a "= `", we want to leave content as is until we see another "`"
    let mut inside_raw_string = false;
    for line in s.lines() {
        if !first_line {
            out.push('\n');
        }
        // Do not indent blank lines (nor the first line)
        if !first_line && !line.is_empty() && !inside_raw_string {
            out.push_str(INDENT);
        }
        out.push_str(line);
        // Toggle raw string state if we encounter a backtick
        if inside_raw_string {
            if line.contains('`') {
                inside_raw_string = false;
            }
        } else {
            // if we contain a "= `", this is an RHS assignment to a raw string
            // if we start with a backtick, this is the RHS by itself
            if line.contains("= `") || line.starts_with('`') {
                // if we have two backticks, the raw string is terminated in the same line
                // (this shouldn't happen though, since we only use raw strings for multiline contexts)
                if line.chars().filter(|&c| c == '`').count() == 1 {
                    inside_raw_string = true;
                }
            }
        }
        first_line = false;
    }
    Ok(out)
}

/// Quote a key if needed
/// See: https://grafana.com/docs/alloy/latest/get-started/syntax/#identifiers
fn format_key(ident: &str) -> Result<String> {
    // Cannot be empty
    if ident.is_empty() {
        return Err(Error::InvalidIdentifier(
            "Identifier cannot be empty".into(),
        ));
    }
    // Cannot start with a digit
    if ident.starts_with(|c: char| c.is_ascii_digit()) {
        return Err(Error::InvalidIdentifier(format!(
            "Identifier {} cannot start with a digit",
            ident
        )));
    }
    // If it contains a space or period, it must be quoted
    Ok(if ident.contains(|c: char| c.is_whitespace() || c == '.') {
        format!("\"{}\"", ident)
    } else {
        ident.to_string()
    })
}

/// Pre-format an IndexMap of attributes
/// This is used in both block-level attributes and object values
/// since key alignment determines how they are rendered
///
/// This aggregates errors and reports a single error
fn preformat_attributes(
    attributes: &indexmap::IndexMap<String, AttributeValue>,
) -> Result<Vec<(String, String)>> {
    let mut formatted_attribs: Vec<(String, String)> = Vec::new();
    let mut formatting_errors: Vec<Error> = Vec::new();
    for (k, v) in attributes.iter() {
        let formatted_key = format_key(k);
        let rendered_value = v.render();
        match (formatted_key, rendered_value) {
            (Ok(k), Ok(v)) => formatted_attribs.push((k, v)),
            (Err(e), _) | (_, Err(e)) => formatting_errors.push(e),
        }
    }
    if !formatting_errors.is_empty() {
        return Err(Error::Multiple(formatting_errors));
    }
    Ok(formatted_attribs)
}

/// Calculate the length of the longest pre-formatted key, for alignment purposes
fn longest_preformatted_key(formatted_attribs: &[(String, String)]) -> usize {
    formatted_attribs
        .iter()
        .map(|(key, _)| key.len())
        .max()
        .unwrap_or(0)
}

impl Block {
    /// Render this block as a `config.alloy` snippet, starting at column 0.
    pub fn render(&self) -> Result<String> {
        let mut str_buff = String::new();
        // write header
        if let Some(label) = &self.label {
            // `<indent>ComponentName "Label" {`
            // component names are never quoted, labels are always quoted
            write!(str_buff, "{} \"{}\" {{", self.component, label)?;
        } else {
            // `<indent>ComponentName {`
            write!(str_buff, "{} {{", self.component)?;
        }
        if self.blocks.is_empty() && self.attributes.is_empty() {
            // if no content, collapse to `<indent>ComponentName {}`
            write!(str_buff, "}}")?;
        } else {
            // We have content, write a newline and start building the body
            // We have exactly one level of indentation for our body
            writeln!(str_buff)?;
            if !self.attributes.is_empty() {
                // Write attributes one per line
                // Note that attributes do not use trailing commas (except within their values-- arrays or objects)
                // need to precalculate formatting for alignment
                let formatted_attribs = preformat_attributes(&self.attributes)?;
                // calculate longest key for alignment
                let longest_key_len = longest_preformatted_key(&formatted_attribs);
                // finally output formatted attributes
                for (key, value) in &formatted_attribs {
                    let ljust = longest_key_len - key.len();
                    writeln!(
                        str_buff,
                        "{}{}{} = {}",
                        INDENT,
                        key,
                        " ".repeat(ljust),
                        indent_trailing_content(value)?
                    )?;
                }
                if !self.blocks.is_empty() {
                    // blank line between attributes and blocks
                    writeln!(str_buff)?;
                }
            }
            if !self.blocks.is_empty() {
                panic!("Nested blocks not implemented yet: {:?}", self.blocks);
            }
            // write footer
            write!(str_buff, "}}")?;
        }
        Ok(str_buff)
    }
}

/// Quote a string attribute value, escaping as needed in config.alloy syntax.
///
/// https://grafana.com/docs/alloy/latest/get-started/expressions/types_and_values/#strings
/// https://grafana.com/docs/alloy/latest/get-started/expressions/types_and_values/#raw-strings
fn format_value_string(s: &str) -> String {
    if s.contains('\n') {
        // If the string contains newlines, use backtick raw-string syntax with no escapes.
        return format!("`{s}`");
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str(r#"\""#),
            '\n' => out.push_str(r"\n"),
            '\t' => out.push_str(r"\t"),
            '\r' => out.push_str(r"\r"),
            _ => out.push(c),
        }
    }
    // TODO: replace escapes
    out.push('"');
    out
}

/// Render an attribute array value
fn format_value_array(arr: &[AttributeValue]) -> Result<String> {
    let mut out = String::new();
    out.push('[');
    if !arr.is_empty() {
        out.push('\n');
    }
    for item in arr.iter() {
        // We only write a single indent (indent_trailing_content will adjust later)
        out.push_str(INDENT);
        out.push_str(&item.render()?);
        // Always include a trailing comma
        out.push_str(",\n");
    }
    out.push(']');
    Ok(out)
}

/// Render an attribute object value
///
/// These render kinda like block attributes, except they have trailing commas
fn format_value_object(obj: &indexmap::IndexMap<String, AttributeValue>) -> Result<String> {
    let mut out = String::new();
    out.push('{');
    if !obj.is_empty() {
        out.push('\n');
    }
    let formatted_items = preformat_attributes(obj)?;
    let longest_key_len = longest_preformatted_key(&formatted_items);

    for (key, value) in formatted_items {
        let ljust = longest_key_len - key.len();
        out.push_str(INDENT);
        out.push_str(&key);
        out.push_str(&" ".repeat(ljust));
        out.push_str(" = ");
        out.push_str(&indent_trailing_content(&value)?);
        out.push_str(",\n");
    }
    out.push('}');
    Ok(out)
}

impl AttributeValue {
    /// Render a top-level attribute value (RHS) of a block.
    /// This does not include a trailing newline or comma.
    pub fn render(&self) -> Result<String> {
        match self {
            AttributeValue::Null => Ok("null".into()),
            AttributeValue::Bool(b) => Ok(b.to_string()),
            AttributeValue::Number(n) => Ok(n.to_string()),
            AttributeValue::String(s) => Ok(format_value_string(s)), // quote strings
            AttributeValue::Array(arr) => format_value_array(arr),
            AttributeValue::Object(obj) => format_value_object(obj),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::alloy::ast::{AttributeValue, Block};
    use indexmap::IndexMap;

    // small helper so tests read like data, not constructor noise
    fn block(component: &str) -> Block {
        Block {
            component: component.into(),
            label: None,
            attributes: IndexMap::new(),
            blocks: Vec::new(),
        }
    }

    fn attrs(pairs: &[(&str, AttributeValue)]) -> IndexMap<String, AttributeValue> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.clone()))
            .collect()
    }

    // -------- 1. The empty case --------

    #[test]
    fn empty_block_uses_compact_braces() {
        // A bare block with no content collapses to `component {}` on one line.
        assert_eq!(block("loki.echo").render().unwrap(), "loki.echo {}");
    }

    #[test]
    fn labeled_empty_block() {
        let b = Block {
            label: Some("stub".into()),
            ..block("loki.echo")
        };
        assert_eq!(b.render().unwrap(), r#"loki.echo "stub" {}"#);
    }

    // -------- 2. Statement-level attributes (no commas) --------

    #[test]
    fn single_string_attribute() {
        let b = Block {
            attributes: attrs(&[("source", AttributeValue::String("ts".into()))]),
            ..block("stage.timestamp")
        };
        assert_eq!(
            b.render().unwrap(),
            concat!("stage.timestamp {\n", "\tsource = \"ts\"\n", "}",),
        );
    }

    #[test]
    fn multiple_attributes_align_equals_signs() {
        // The widest key sets the column for all `=` signs in this group.
        // `forward_to` (10) is wider than `foo` (3), so `foo` is padded
        // to 10 characters with trailing spaces.
        let b = Block {
            attributes: attrs(&[
                ("forward_to", AttributeValue::String("x".into())),
                ("foo", AttributeValue::String("bar".into())),
            ]),
            ..block("loki.process")
        };
        assert_eq!(
            b.render().unwrap(),
            concat!(
                "loki.process {\n",
                "\tforward_to = \"x\"\n",
                "\tfoo        = \"bar\"\n",
                "}",
            ),
        );
    }

    // -------- 3. Primitive value rendering --------

    #[test]
    fn primitive_values_render_unquoted() {
        // Numbers, bools, and null are rendered as their literal alloy form.
        // Booleans are lowercase; null is the literal `null`.
        let b = Block {
            attributes: attrs(&[
                ("count", AttributeValue::Number(42.0)),
                ("rate", AttributeValue::Number(0.05)),
                ("drop", AttributeValue::Bool(true)),
                ("opt", AttributeValue::Bool(false)),
                ("zero", AttributeValue::Null),
            ]),
            ..block("stage.limit")
        };
        assert_eq!(
            b.render().unwrap(),
            concat!(
                "stage.limit {\n",
                "\tcount = 42\n", // not 42.0 — integers shouldn't get a .0 tail
                "\trate  = 0.05\n",
                "\tdrop  = true\n",
                "\topt   = false\n",
                "\tzero  = null\n",
                "}",
            ),
        );
        // ^ this one's spicy: f64 formatting in Rust defaults to "42" for
        // integral values, which is what we want. But `format!("{}", 1.0)`
        // also gives "1", and `format!("{}", 0.1)` gives "0.1". Verify
        // against alloy fmt — you may need to special-case the integer path.
    }
    // -------- 4. String escaping --------

    #[test]
    fn string_with_quotes_and_backslashes_is_escaped() {
        let b = Block {
            attributes: attrs(&[(
                "expression",
                AttributeValue::String(r#"hello "world" \n"#.into()),
            )]),
            ..block("stage.regex")
        };
        // Inner quotes become \", literal backslash becomes \\.
        // Note: the input contains literal `\` and `n`, NOT a newline.
        assert_eq!(
            b.render().unwrap(),
            concat!(
                "stage.regex {\n",
                "\texpression = \"hello \\\"world\\\" \\\\n\"\n",
                "}",
            ),
        );
    }

    #[test]
    fn multiline_string_uses_backtick_raw_form() {
        // Strings containing actual newlines use alloy's raw-string syntax (backticks),
        // which has no escapes. So the value goes in verbatim.
        let b = Block {
            attributes: attrs(&[(
                "template",
                AttributeValue::String("line one\nline two".into()),
            )]),
            ..block("stage.template")
        };
        assert_eq!(
            b.render().unwrap(),
            concat!(
                "stage.template {\n",
                "\ttemplate = `line one\nline two`\n",
                "}",
            ),
        );
    }

    // -------- 5. Array RHS (trailing commas, every line) --------

    #[test]
    fn empty_array_is_inline() {
        let b = Block {
            attributes: attrs(&[("forward_to", AttributeValue::Array(vec![]))]),
            ..block("loki.process")
        };
        assert_eq!(
            b.render().unwrap(),
            concat!("loki.process {\n", "\tforward_to = []\n", "}",),
        );
    }

    #[test]
    fn non_empty_array_breaks_lines_with_trailing_commas() {
        // Every item gets a trailing comma — including the last.
        let b = Block {
            attributes: attrs(&[(
                "forward_to",
                AttributeValue::Array(vec![
                    AttributeValue::String("a".into()),
                    AttributeValue::String("b".into()),
                ]),
            )]),
            ..block("loki.process")
        };
        assert_eq!(
            b.render().unwrap(),
            concat!(
                "loki.process {\n",
                "\tforward_to = [\n",
                "\t\t\"a\",\n",
                "\t\t\"b\",\n",
                "\t]\n",
                "}",
            ),
        );
    }

    // -------- 6. Object RHS (trailing commas, key alignment) --------

    #[test]
    fn empty_object_is_inline() {
        let b = Block {
            attributes: attrs(&[("values", AttributeValue::Object(IndexMap::new()))]),
            ..block("stage.labels")
        };
        assert_eq!(
            b.render().unwrap(),
            concat!("stage.labels {\n", "\tvalues = {}\n", "}",),
        );
    }

    #[test]
    fn non_empty_object_has_trailing_commas_and_aligned_keys() {
        let mut mapping = IndexMap::new();
        mapping.insert("msg".into(), AttributeValue::String("message".into()));
        mapping.insert("level".into(), AttributeValue::String("severity".into()));
        let b = Block {
            attributes: attrs(&[("mapping", AttributeValue::Object(mapping))]),
            ..block("stage.json")
        };
        // Object literals also align `=` and use trailing commas — same rules
        // as block-level attributes EXCEPT for the commas. Keys here are
        // `msg` (3) and `level` (5), so `msg` pads to 5.
        assert_eq!(
            b.render().unwrap(),
            concat!(
                "stage.json {\n",
                "\tmapping = {\n",
                "\t\tmsg   = \"message\",\n",
                "\t\tlevel = \"severity\",\n",
                "\t}\n",
                "}",
            ),
        );
    }

    // -------- 8. Identifiers that need quoting --------

    #[test]
    fn identifier_with_space_or_special_chars_is_quoted() {
        // A key like `service name` or `weird:key` isn't a bare alloy identifier
        // and must be quoted. Plain dots are OK unquoted (loki.echo, stage.drop).
        let b = Block {
            attributes: attrs(&[
                ("normal_key", AttributeValue::String("v".into())),
                ("weird key", AttributeValue::String("v".into())),
            ]),
            ..block("loki.process")
        };
        assert_eq!(
            b.render().unwrap(),
            concat!(
                "loki.process {\n",
                // The quoted key is 11 chars including quotes; normal_key is 10.
                // So `normal_key` pads to 11.
                "\tnormal_key  = \"v\"\n",
                "\t\"weird key\" = \"v\"\n",
                "}",
            ),
        );
    }
}
