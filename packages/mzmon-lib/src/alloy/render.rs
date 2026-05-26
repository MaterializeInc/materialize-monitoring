// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use crate::alloy::ast::{AttributeValue, Block, Identifier};
use crate::alloy::error::{Error, Result};

use std::fmt;

const INDENT: &str = "\t";

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

/// Pre-format the keys of an IndexMap of attributes
/// This is used in both block-level attributes and object values
/// since key alignment determines how they are rendered
///
/// This aggregates errors and reports a single error
fn preformat_attribute_keys<'a>(
    attributes: &'a indexmap::IndexMap<Identifier, AttributeValue>,
) -> Result<Vec<(String, &'a AttributeValue)>> {
    let mut formatted_key_map: Vec<(String, &'a AttributeValue)> = Vec::new();
    let mut formatting_errors: Vec<Error> = Vec::new();
    for (key, value) in attributes.iter() {
        match format_key(key) {
            Ok(formatted_key) => formatted_key_map.push((formatted_key, value)),
            Err(e) => formatting_errors.push(e),
        }
    }
    if !formatting_errors.is_empty() {
        return Err(Error::Multiple(formatting_errors));
    }
    Ok(formatted_key_map)
}

/// Calculate the length of the longest pre-formatted key, for alignment purposes
fn longest_preformatted_key(formatted_attribs: &[(String, &AttributeValue)]) -> usize {
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
        self.write_to(&mut str_buff, 0)?;
        Ok(str_buff)
    }

    pub fn write_to(&self, out: &mut impl fmt::Write, indent: usize) -> Result<()> {
        // write header
        if let Some(label) = &self.label {
            // `<indent>ComponentName "Label" {`
            // component names are never quoted, labels are always quoted
            write!(
                out,
                "{}{} \"{}\" {{",
                INDENT.repeat(indent),
                self.component,
                label
            )?;
        } else {
            // `<indent>ComponentName {`
            write!(out, "{}{} {{", INDENT.repeat(indent), self.component)?;
        }

        if self.blocks.is_empty() && self.attributes.is_empty() {
            // if no content, collapse to `<indent>ComponentName {}` on same line
            write!(out, "}}")?;
        } else {
            // We have content, write a newline and start building the body
            // We have exactly one level of indentation for our body
            writeln!(out)?;

            if !self.attributes.is_empty() {
                let inner_prefix = INDENT.repeat(indent + 1);
                // Write attributes one per line
                // Note that attributes do not use trailing commas (except within their values-- arrays or objects)
                // need to precalculate formatting for alignment
                let formatted_attribs = preformat_attribute_keys(&self.attributes)?;
                // calculate longest key for alignment
                let longest_key_len = longest_preformatted_key(&formatted_attribs);
                // finally output formatted attributes
                for (formatted_key, value) in &formatted_attribs {
                    let ljust = longest_key_len - formatted_key.len();
                    // NOTE: no trailing newline (yet) or trailing comma
                    write!(
                        out,
                        "{}{}{} = ",
                        inner_prefix,
                        formatted_key,
                        " ".repeat(ljust),
                    )?;
                    value.write_to(out, indent + 1)?;
                    writeln!(out)?; // newline after each attribute
                }
                if !self.blocks.is_empty() {
                    // blank line between attributes and blocks
                    writeln!(out)?;
                }
            }

            if !self.blocks.is_empty() {
                // Write blocks, separated by a blank line
                for (i, block) in self.blocks.iter().enumerate() {
                    if i > 0 {
                        writeln!(out)?; // blank line between blocks
                    }
                    // Render the block and indent all lines by one level
                    block.write_to(out, indent + 1)?;
                    writeln!(out)?; // newline after each block
                }
            }
            // write footer
            write!(out, "{}}}", INDENT.repeat(indent))?;
        }
        Ok(())
    }
}

/// Quote a string attribute value, escaping as needed in config.alloy syntax.
///
/// https://grafana.com/docs/alloy/latest/get-started/expressions/types_and_values/#strings
/// https://grafana.com/docs/alloy/latest/get-started/expressions/types_and_values/#raw-strings
fn format_value_string(s: &str) -> String {
    if s.contains('\n') && !s.contains('`') {
        // If the string contains newlines, prefer backtick raw-string syntax with no escapes.
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
    out.push('"');
    out
}

/// Render an attribute array value
fn write_value_array(
    out: &mut impl fmt::Write,
    arr: &[AttributeValue],
    indent: usize,
) -> Result<()> {
    // caller is responsible for leading indents
    if arr.is_empty() {
        // empty arrays are rendered inline as `[]`
        write!(out, "[]")?;
        return Ok(());
    }
    writeln!(out, "[")?;
    let inner_prefix = INDENT.repeat(indent + 1);
    for item in arr.iter() {
        write!(out, "{}", inner_prefix)?;
        item.write_to(out, indent + 1)?;
        writeln!(out, ",")?; // trailing comma on every item
    }
    write!(out, "{}]", INDENT.repeat(indent))?;
    Ok(())
}

/// Render an attribute object value
///
/// These render kinda like block attributes, except they have trailing commas
fn write_value_object(
    out: &mut impl fmt::Write,
    obj: &indexmap::IndexMap<String, AttributeValue>,
    indent: usize,
) -> Result<()> {
    // caller is responsible for leading indents
    if obj.is_empty() {
        // empty objects are rendered inline as `{}`
        write!(out, "{{}}")?;
        return Ok(());
    }
    writeln!(out, "{{")?;
    let inner_prefix = INDENT.repeat(indent + 1);
    let formatted_items = preformat_attribute_keys(obj)?;
    let longest_key_len = longest_preformatted_key(&formatted_items);

    for (formatted_key, value) in formatted_items {
        let ljust = longest_key_len - formatted_key.len();
        write!(
            out,
            "{}{}{} = ",
            inner_prefix,
            formatted_key,
            " ".repeat(ljust)
        )?;
        value.write_to(out, indent + 1)?;
        writeln!(out, ",")?; // trailing comma on every item
    }
    write!(out, "{}}}", INDENT.repeat(indent))?;
    Ok(())
}

impl AttributeValue {
    /// Render a top-level attribute value (RHS) of a block.
    /// This does not include a trailing newline or comma.
    pub fn render(&self) -> Result<String> {
        let mut out = String::new();
        self.write_to(&mut out, 0)?;
        Ok(out)
    }

    /// Write the RHS of an attribute assignment
    /// There is going to be a preceding `= ` that is responsible for the caller to write
    /// This also doesn't write the trailing newline or comma
    pub fn write_to(&self, out: &mut impl fmt::Write, indent: usize) -> Result<()> {
        match self {
            AttributeValue::Null => write!(out, "null")?,
            AttributeValue::Bool(b) => write!(out, "{}", b)?,
            AttributeValue::Number(n) => write!(out, "{}", n)?,
            AttributeValue::String(s) => write!(out, "{}", format_value_string(s))?, // quote strings
            AttributeValue::Array(arr) => write_value_array(out, arr, indent)?,
            AttributeValue::Object(obj) => write_value_object(out, obj, indent)?,
            // TODO: expression
        }
        Ok(())
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

    // -------- 7. Nested blocks --------

    #[test]
    fn nested_block_indents_by_one_tab() {
        let inner = Block {
            attributes: attrs(&[(
                "drop_counter_reason",
                AttributeValue::String("backlog > 12hr".into()),
            )]),
            ..block("stage.drop")
        };
        let outer = Block {
            blocks: vec![inner],
            ..block("loki.process")
        };
        assert_eq!(
            outer.render().unwrap(),
            concat!(
                "loki.process {\n",
                "\tstage.drop {\n",
                "\t\tdrop_counter_reason = \"backlog > 12hr\"\n",
                "\t}\n",
                "}",
            ),
        );
    }

    #[test]
    fn attributes_and_nested_blocks_are_separated_by_blank_line() {
        // This is the one place a newline acts as a separator — between
        // the attributes group and the nested-blocks group inside a block.
        // Inside the nested-blocks group, adjacent siblings also get a
        // blank line between them.
        let stage = Block {
            attributes: attrs(&[("older_than", AttributeValue::String("12h".into()))]),
            ..block("stage.drop")
        };
        let outer = Block {
            attributes: attrs(&[("forward_to", AttributeValue::String("x".into()))]),
            blocks: vec![stage.clone(), stage],
            ..block("loki.process")
        };
        assert_eq!(
            outer.render().unwrap(),
            concat!(
                "loki.process {\n",
                "\tforward_to = \"x\"\n",
                "\n", // blank line: attrs → blocks
                "\tstage.drop {\n",
                "\t\tolder_than = \"12h\"\n",
                "\t}\n",
                "\n", // blank line: block → block
                "\tstage.drop {\n",
                "\t\tolder_than = \"12h\"\n",
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
