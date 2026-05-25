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
fn indent_trailing_content(s: &str) -> Result<String> {
    let mut out = String::new();
    let mut lines = s.lines();
    // Write first line directly
    out.push_str(lines.next().unwrap_or(""));
    // Indent subsequent lines
    for line in lines {
        if line.is_empty() {
            // Do not indent blank lines
            out.push('\n');
        } else {
            writeln!(out, "{}{}", INDENT, line)?;
        }
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
                let formatted_attribs: Vec<(String, String)> = self
                    .attributes
                    .iter()
                    .map(|(k, v)| {
                        let formatted_key = format_key(k);
                        let rendered_value = v.render();
                        match (formatted_key, rendered_value) {
                            (Ok(k), Ok(v)) => Ok((k, v)),
                            (Err(e), _) | (_, Err(e)) => Err(e),
                        }
                    })
                    .collect::<Result<Vec<(String, String)>>>()?;
                // calculate longest key for alignment
                let mut longest_key_len = 0;
                for (key, _) in &formatted_attribs {
                    if key.len() > longest_key_len {
                        longest_key_len = key.len();
                    }
                }
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

impl AttributeValue {
    /// Render a top-level attribute value (RHS) of a block.
    /// This does not include a trailing newline or comma.
    pub fn render(&self) -> Result<String> {
        match self {
            AttributeValue::Null => Ok("null".into()),
            AttributeValue::Bool(b) => Ok(b.to_string()),
            AttributeValue::Number(n) => Ok(n.to_string()),
            AttributeValue::String(s) => Ok(format!("{:?}", s)), // quote strings
            AttributeValue::Array(arr) => {
                panic!("Array rendering not implemented yet: {:?}", arr)
            }
            AttributeValue::Object(obj) => {
                panic!("Object rendering not implemented yet: {:?}", obj)
            }
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
