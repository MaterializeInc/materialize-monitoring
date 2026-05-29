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

/// Returns true if `ident` is a bare alloy identifier: a non-empty run of
/// ASCII letters, digits, and underscores that does not start with a digit.
///
/// See: https://grafana.com/docs/alloy/latest/get-started/syntax/#identifiers
fn is_bare_identifier(ident: &str) -> bool {
    let mut chars = ident.chars();
    match chars.next() {
        // first char must be a letter or underscore (so: non-empty, no leading digit)
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Format a *block attribute* key — the LHS of `=` directly inside a block body.
///
/// These MUST be bare identifiers. Alloy rejects both quoted keys
/// (`"weird key" = ...` → "expected identifier, got STRING") and dotted keys
/// (`foo.bar = ...` → "attribute names may only consist of a single identifier").
/// Quoting cannot rescue an invalid key in this context, so we error instead.
fn format_attribute_key(ident: &str) -> Result<String> {
    if is_bare_identifier(ident) {
        Ok(ident.to_string())
    } else {
        Err(Error::InvalidIdentifier(format!(
            "attribute key {ident:?} must be a single bare identifier \
             (letters, digits, underscores; not starting with a digit); \
             alloy does not allow quoted or dotted attribute keys"
        )))
    }
}

/// Format an *object-literal* key — the LHS of `=` inside a `{ }` value.
///
/// Bare identifiers are emitted unquoted; anything else (spaces, dots, ...) is
/// quoted, which alloy accepts in object-literal context.
fn format_object_key(ident: &str) -> Result<String> {
    if ident.is_empty() {
        return Err(Error::InvalidIdentifier(
            "object key cannot be empty".into(),
        ));
    }
    Ok(if is_bare_identifier(ident) {
        ident.to_string()
    } else {
        format!("\"{ident}\"")
    })
}

/// Pre-format the keys of an IndexMap of attributes, using the supplied
/// context-specific key formatter (`format_attribute_key` for block bodies,
/// `format_object_key` for object literals — they have different syntax rules).
/// Key alignment is computed by the caller, so we return the formatted keys.
///
/// This aggregates errors and reports a single error
fn preformat_attribute_keys<'a>(
    attributes: &'a indexmap::IndexMap<Identifier, AttributeValue>,
    format_key: fn(&str) -> Result<String>,
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
            // if no content, collapse to `<indent>ComponentName { }` on same line
            write!(out, " }}")?;
        } else {
            // We have content, write a newline and start building the body
            // We have exactly one level of indentation for our body
            writeln!(out)?;

            if !self.attributes.is_empty() {
                let inner_prefix = INDENT.repeat(indent + 1);
                // Write attributes one per line
                // Note that attributes do not use trailing commas (except within their values-- arrays or objects)
                // need to precalculate formatting for alignment
                let formatted_attribs =
                    preformat_attribute_keys(&self.attributes, format_attribute_key)?;
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
    let formatted_items = preformat_attribute_keys(obj, format_object_key)?;
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

/// Render an expression value
fn write_expression(
    out: &mut impl fmt::Write,
    expr: &crate::alloy::ast::Expression,
    indent: usize,
) -> Result<()> {
    let mut rendered_expr = false;
    if let Some(env_var) = &expr.env {
        // TODO: validate characters?
        write!(out, "sys.env(\"{}\")", env_var)?;
        rendered_expr = true;
    }
    if let Some(raw) = &expr.raw {
        if rendered_expr {
            return Err(Error::Render("Too many expressions".into()));
        }
        write!(out, "{}", raw)?;
        rendered_expr = true;
    }
    if let Some(func) = &expr.function {
        if rendered_expr {
            return Err(Error::Render("Too many expressions".into()));
        }
        write!(out, "{}(", func)?;
        for (i, arg) in expr.arguments.iter().enumerate() {
            if i > 0 {
                write!(out, ", ")?;
            }
            // TODO: when do we break to multiple lines?
            arg.write_to(out, indent + 1)?;
        }
        write!(out, ")")?;
        rendered_expr = true;
    }
    if let Some(ref_name) = &expr.ref_name {
        if rendered_expr {
            return Err(Error::Render("Too many expressions".into()));
        }
        // TODO: validate ref exists on second pass?
        write!(out, "{}", ref_name)?;
        rendered_expr = true;
    }
    if let Some(oper) = &expr.operator {
        if rendered_expr {
            return Err(Error::Render("Too many expressions".into()));
        }
        // TODO: implement
        let _ = oper;
        return Err(Error::Render("Operator is not yet implemented".into()));
    }
    if !rendered_expr {
        return Err(Error::Render("Expression had no body".into()));
    }
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
            AttributeValue::Expression(expr) => write_expression(out, expr, indent)?,
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::alloy::ast::{AttributeValue, Block, Expression};
    use crate::alloy::error::Error;
    use crate::alloy::test_support::assert_renders;
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
        // A bare block with no content collapses to `component { }` on one line
        // (alloy fmt space-pads an empty block body).
        assert_renders(block("loki.echo").render(), "loki.echo { }");
    }

    #[test]
    fn labeled_empty_block() {
        let b = Block {
            label: Some("stub".into()),
            ..block("loki.echo")
        };
        assert_renders(b.render(), r#"loki.echo "stub" { }"#);
    }

    // -------- 2. Statement-level attributes (no commas) --------

    #[test]
    fn single_string_attribute() {
        let b = Block {
            attributes: attrs(&[("source", AttributeValue::String("ts".into()))]),
            ..block("stage.timestamp")
        };
        assert_renders(
            b.render(),
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
        assert_renders(
            b.render(),
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
        assert_renders(
            b.render(),
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
        assert_renders(
            b.render(),
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
        assert_renders(
            b.render(),
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
        assert_renders(
            b.render(),
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
        assert_renders(
            b.render(),
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
        assert_renders(
            b.render(),
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
        assert_renders(
            b.render(),
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
        assert_renders(
            outer.render(),
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
        assert_renders(
            outer.render(),
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
    fn quoted_key_in_object_literal() {
        // Inside an OBJECT literal, a key that isn't a bare identifier (space, dot)
        // is valid when quoted — `alloy fmt` accepts `"weird key" = ...` here.
        // The quoted key is 11 chars including quotes; normal_key is 10, so it
        // pads to 11 to align the `=` signs.
        //
        // NOTE: this is NOT true for block-level *attribute* keys, which must be
        // bare single identifiers (alloy rejects both `"weird key" =` and
        // `foo.bar =`). See the format_key context-split finding.
        let mut values = IndexMap::new();
        values.insert("normal_key".into(), AttributeValue::String("v".into()));
        values.insert("weird key".into(), AttributeValue::String("v".into()));
        let b = Block {
            attributes: attrs(&[("values", AttributeValue::Object(values))]),
            ..block("stage.labels")
        };
        assert_renders(
            b.render(),
            concat!(
                "stage.labels {\n",
                "\tvalues = {\n",
                "\t\tnormal_key  = \"v\",\n",
                "\t\t\"weird key\" = \"v\",\n",
                "\t}\n",
                "}",
            ),
        );
    }

    #[test]
    fn dotted_key_in_object_literal_is_quoted() {
        // A dotted key is also fine in object-literal context, quoted.
        let mut values = IndexMap::new();
        values.insert("foo.bar".into(), AttributeValue::String("v".into()));
        let b = Block {
            attributes: attrs(&[("values", AttributeValue::Object(values))]),
            ..block("stage.labels")
        };
        assert_renders(
            b.render(),
            concat!(
                "stage.labels {\n",
                "\tvalues = {\n",
                "\t\t\"foo.bar\" = \"v\",\n",
                "\t}\n",
                "}",
            ),
        );
    }

    // -------- 9. Attribute keys must be bare identifiers --------

    /// A render result should fail with a single InvalidIdentifier (wrapped in
    /// the error-collecting `Multiple`).
    fn assert_invalid_identifier(result: crate::alloy::error::Result<String>) {
        match result {
            Err(Error::Multiple(errs)) => {
                assert_eq!(errs.len(), 1, "expected exactly one error, got {errs:?}");
                assert!(
                    matches!(errs[0], Error::InvalidIdentifier(_)),
                    "expected InvalidIdentifier, got {:?}",
                    errs[0],
                );
            }
            other => panic!("expected Multiple([InvalidIdentifier]), got {other:?}"),
        }
    }

    #[test]
    fn attribute_key_with_space_is_rejected() {
        // alloy rejects `"weird key" = ...` as an attribute key — quoting can't
        // rescue it, so the renderer must error rather than emit invalid output.
        let b = Block {
            attributes: attrs(&[("weird key", AttributeValue::String("v".into()))]),
            ..block("loki.process")
        };
        assert_invalid_identifier(b.render());
    }

    #[test]
    fn attribute_key_with_dot_is_rejected() {
        // alloy: "attribute names may only consist of a single identifier with no '.'"
        let b = Block {
            attributes: attrs(&[("foo.bar", AttributeValue::String("v".into()))]),
            ..block("loki.process")
        };
        assert_invalid_identifier(b.render());
    }

    // -------- 10. Expressions --------

    /// Build a block `stage.expr { <key> = <expr> }` for exercising expression
    /// rendering as the RHS of an attribute.
    fn expr_block(key: &str, expr: Expression) -> Block {
        Block {
            attributes: attrs(&[(key, AttributeValue::Expression(expr))]),
            ..block("stage.expr")
        }
    }

    #[test]
    fn expression_env_renders_sys_env() {
        let b = expr_block(
            "target",
            Expression {
                env: Some("MZ_TARGET".into()),
                ..Default::default()
            },
        );
        assert_renders(
            b.render(),
            concat!("stage.expr {\n", "\ttarget = sys.env(\"MZ_TARGET\")\n", "}"),
        );
    }

    #[test]
    fn expression_raw_renders_verbatim() {
        // The primary tunable path: a free-form raw expression passed through as-is.
        let b = expr_block(
            "sum",
            Expression {
                raw: Some("1 + 2".into()),
                ..Default::default()
            },
        );
        assert_renders(
            b.render(),
            concat!("stage.expr {\n", "\tsum = 1 + 2\n", "}"),
        );
    }

    #[test]
    fn expression_ref_renders_verbatim() {
        // A component reference is emitted as a bare dotted path (unquoted).
        let b = expr_block(
            "forward_to",
            Expression {
                ref_name: Some("loki.write.default.receiver".into()),
                ..Default::default()
            },
        );
        assert_renders(
            b.render(),
            concat!(
                "stage.expr {\n",
                "\tforward_to = loki.write.default.receiver\n",
                "}",
            ),
        );
    }

    #[test]
    fn expression_function_with_args() {
        // function + arguments: args are comma-separated, string args quoted.
        let b = expr_block(
            "name",
            Expression {
                function: Some("concat".into()),
                arguments: vec![
                    AttributeValue::String("x".into()),
                    AttributeValue::String("y".into()),
                ],
                ..Default::default()
            },
        );
        assert_renders(
            b.render(),
            concat!("stage.expr {\n", "\tname = concat(\"x\", \"y\")\n", "}"),
        );
    }

    #[test]
    fn expression_function_without_args_uses_empty_parens() {
        let b = expr_block(
            "ts",
            Expression {
                function: Some("coalesce".into()),
                ..Default::default()
            },
        );
        assert_renders(
            b.render(),
            concat!("stage.expr {\n", "\tts = coalesce()\n", "}"),
        );
    }

    #[test]
    fn expression_function_with_env_arg() {
        // An argument can itself be an expression (here, a nested sys.env).
        let b = expr_block(
            "name",
            Expression {
                function: Some("concat".into()),
                arguments: vec![
                    AttributeValue::Expression(Expression {
                        env: Some("PREFIX".into()),
                        ..Default::default()
                    }),
                    AttributeValue::String("-suffix".into()),
                ],
                ..Default::default()
            },
        );
        assert_renders(
            b.render(),
            concat!(
                "stage.expr {\n",
                "\tname = concat(sys.env(\"PREFIX\"), \"-suffix\")\n",
                "}",
            ),
        );
    }

    #[test]
    fn expression_with_no_head_errors() {
        // An expression with none of raw/env/function/ref/operator set is invalid.
        let b = expr_block("x", Expression::default());
        assert!(
            matches!(b.render(), Err(Error::Render(_))),
            "empty expression should be a render error"
        );
    }

    #[test]
    fn expression_with_multiple_heads_errors() {
        // Exactly one head may be set; raw + env together is ambiguous.
        let b = expr_block(
            "x",
            Expression {
                env: Some("FOO".into()),
                raw: Some("1 + 2".into()),
                ..Default::default()
            },
        );
        assert!(
            matches!(b.render(), Err(Error::Render(_))),
            "multi-head expression should be a render error"
        );
    }

    #[test]
    fn expression_operator_is_unimplemented() {
        // operator rendering is intentionally not implemented yet; it errors.
        let b = expr_block(
            "x",
            Expression {
                operator: Some("+".into()),
                ..Default::default()
            },
        );
        assert!(
            matches!(b.render(), Err(Error::Render(_))),
            "operator expression should currently error"
        );
    }
}
