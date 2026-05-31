// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use crate::alloy::ast::{Block, ToBlock};
use crate::alloy::components::{discovery, loki, top};
use crate::alloy::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
#[serde(deny_unknown_fields)]
pub struct Pipeline {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logging: Option<top::LoggingBlock>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub livedebugging: Option<top::LiveDebuggingBlock>,
    #[serde(default)]
    pub blocks: Vec<ComponentBlock>,
}

// All-available blocks
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ComponentBlock {
    #[serde(rename = "raw")]
    Raw(Block),
    #[serde(rename = "loki.echo")]
    LokiEcho(loki::LokiEchoBlock),
    #[serde(rename = "loki.source.journal")]
    LokiSourceJournal(loki::LokiSourceJournalBlock),
    #[serde(rename = "discovery.kubernetes")]
    DiscoveryKubernetes(discovery::DiscoveryKubernetesBlock),
    #[serde(rename = "discovery.relabel")]
    DiscoveryRelabel(discovery::DiscoveryRelabelBlock),
}

impl ToBlock for ComponentBlock {
    fn to_block(&self) -> Result<Block> {
        match self {
            ComponentBlock::Raw(rb) => Ok(rb.clone()),
            ComponentBlock::LokiEcho(le) => le.to_block(),
            ComponentBlock::LokiSourceJournal(lsj) => lsj.to_block(),
            ComponentBlock::DiscoveryKubernetes(dk) => dk.to_block(),
            ComponentBlock::DiscoveryRelabel(dr) => dr.to_block(),
        }
    }
}

impl Pipeline {
    /// Render this pipeline as a string in config.alloy syntax.
    pub fn render(&self) -> Result<String> {
        let mut s = String::new();
        self.write_to(&mut s)?;
        Ok(s)
    }

    /// Write this pipeline to a formatter in config.alloy syntax.
    pub fn write_to(&self, out: &mut impl fmt::Write) -> Result<()> {
        // Collect all errors to be displayed simultaneously (instead of just first)
        let mut pipeline_errors: Vec<Error> = Vec::new();

        let mut blocks: Vec<Result<Block>> = Vec::new();
        if let Some(logging) = &self.logging {
            blocks.push(logging.to_block());
        }
        if let Some(ld) = &self.livedebugging {
            blocks.push(ld.to_block());
        }
        for block in self.blocks.iter() {
            blocks.push(block.to_block());
        }

        let mut needs_separator = false;
        if let Some(desc) = &self.description {
            write_description_comment(out, desc)?;
            needs_separator = true;
        }
        for block in blocks {
            if needs_separator {
                writeln!(out)?;
            }
            match block {
                Ok(block) => {
                    match block.write_to(out, 0) {
                        Ok(()) => (),
                        Err(e) => pipeline_errors.push(e),
                    };
                    writeln!(out)?;
                }
                Err(e) => {
                    writeln!(out, "// ERROR: {}", e)?;
                    pipeline_errors.push(e);
                }
            }
            needs_separator = true;
        }
        if pipeline_errors.is_empty() {
            Ok(())
        } else {
            Err(Error::Multiple(pipeline_errors))
        }
    }

    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        // 1. YAML → generic JSON value (structure only; no enum dispatch happens here)
        let value: serde_json::Value = serde_yaml_ng::from_str(yaml)?;
        // 2. Validate against the embedded JSONSchema, collecting *all* violations
        //    with their instance paths before we attempt to deserialize.
        crate::alloy::validate::validate(&value)?;
        // 3. JSON value → typed Pipeline (serde_json drives enum dispatch = map form)
        Ok(serde_json::from_value(value)?)
    }
}

/// Write each line of a description as a comment.
/// This is intended to go at the top of a config.alloy file.
fn write_description_comment(out: &mut impl fmt::Write, desc: &str) -> Result<()> {
    for line in desc.lines() {
        if line.is_empty() {
            writeln!(out, "//")?;
        } else {
            writeln!(out, "// {}", line)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloy::error::Error;
    use crate::alloy::test_support::assert_renders;

    /// Assert that `from_yaml_str` failed because of *schema* validation (an
    /// `Error::Multiple` containing at least one `Error::Schema`), not a serde
    /// deserialization error. Returns the schema-violation paths for inspection.
    fn assert_schema_rejected(yaml: &str) -> Vec<String> {
        match Pipeline::from_yaml_str(yaml) {
            Err(Error::Multiple(errs)) => {
                let paths: Vec<String> = errs
                    .iter()
                    .filter_map(|e| match e {
                        Error::Schema { path, .. } => Some(path.clone()),
                        _ => None,
                    })
                    .collect();
                assert!(
                    !paths.is_empty(),
                    "expected at least one schema violation, got {errs:?}"
                );
                paths
            }
            other => panic!("expected Multiple([Schema, ...]), got {other:?}"),
        }
    }

    #[test]
    fn pipeline_with_description_and_logging() {
        let yaml = r#"
            description: |
              first line

              third line
            logging:
              level: info
            blocks: []
        "#;
        let pipeline = Pipeline::from_yaml_str(yaml).unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "// first line\n",
                "//\n",
                "// third line\n",
                "\n", // blank line: description → logging
                "logging {\n",
                "\tlevel = \"info\"\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn pipeline_with_one_raw_block() {
        let yaml = r#"
            blocks:
              - raw:
                  component: loki.echo
                  label: stub
        "#;
        let pipeline = Pipeline::from_yaml_str(yaml).unwrap();
        assert_renders(pipeline.render(), "loki.echo \"stub\" { }\n");
    }

    /// Schema-only check (Rust sugar deserialization is Phase 3b): parse the YAML
    /// into a generic value tree and assert the validator accepts it.
    fn assert_schema_ok(yaml: &str) {
        let value: serde_json::Value = serde_yaml_ng::from_str(yaml).unwrap();
        if let Err(e) = crate::alloy::validate::validate(&value) {
            panic!("expected schema to accept the YAML, got {e:?}");
        }
    }

    #[test]
    fn pipeline_with_loki_echo_sugar_block() {
        // The `loki.echo` branch of the blocks oneOf validates against loki.schema.yaml.
        assert_schema_ok(
            r#"
            blocks:
              - loki.echo:
                  label: echo
        "#,
        );
    }

    #[test]
    fn pipeline_with_loki_process_and_nested_stages() {
        // Exercises the recursive case: loki.process body contains stage.match,
        // which itself nests further stages. Also confirms the `raw` escape inside
        // a stage list works alongside typed stages. Typed blocks are FLAT
        // (attributes live directly under the discriminator); only `raw:` keeps
        // the `attributes:` partition.
        assert_schema_ok(
            r#"
            blocks:
              - loki.process:
                  label: processor
                  forward_to: []
                  blocks:
                    - stage.drop:
                        older_than: "12h"
                        drop_counter_reason: "backlog > 12hr"
                    - stage.match:
                        selector: '{app="alloy"}'
                        blocks:
                          - stage.logfmt:
                              mapping:
                                msg: msg
                                level: level
                          - stage.timestamp:
                              source: ts
                              format: RFC3339Nano
                    - stage.labels:
                        values:
                          level: level
                    - raw:
                        component: stage.label_keep
                        attributes:
                          values: [level]
        "#,
        );
    }

    #[test]
    fn discovery_relabel_uses_rule_blocks_via_cross_file_ref() {
        // discovery.relabel's rule block lives in loki.schema.yaml ($defs/ruleBlock)
        // — this confirms the cross-file `$ref` resolves through the registry.
        assert_schema_ok(
            r#"
            blocks:
              - discovery.relabel:
                  targets: []
                  blocks:
                    - rule:
                        action: keep
                        source_labels: [__meta_kubernetes_pod_label_app]
                        regex: "loki|alloy"
                    - rule:
                        action: replace
                        source_labels: [__meta_kubernetes_namespace]
                        target_label: namespace
        "#,
        );
    }

    #[test]
    fn unknown_attribute_on_typed_block_is_rejected_by_schema() {
        // Typed blocks are strict: undocumented attributes must use `raw:` instead.
        // After the flatten, the undocumented attribute appears directly under the
        // typed block (no `attributes:` nesting in typed sugar).
        let yaml = r#"
            blocks:
              - loki.process:
                  forward_to: []
                  mystery_attr: 42
        "#;
        let paths = assert_schema_rejected(yaml);
        assert!(
            paths.iter().any(|p| p.starts_with("/blocks/0")),
            "expected a /blocks/0 violation, got {paths:?}"
        );
    }

    #[test]
    fn unknown_attribute_error_includes_raw_escape_hint() {
        // The `additional properties are not allowed` path attaches a project-specific
        // hint about the `raw:` escape vs. extending the schema. Without it, the bare
        // jsonschema message is correct but not actionable for a new contributor.
        let yaml = r#"
            blocks:
              - loki.process:
                  forward_to: []
                  mystery_attr: 42
        "#;
        let err = Pipeline::from_yaml_str(yaml).unwrap_err();
        let messages: Vec<String> = match err {
            Error::Multiple(errs) => errs
                .iter()
                .filter_map(|e| match e {
                    Error::Schema { message, .. } => Some(message.clone()),
                    _ => None,
                })
                .collect(),
            other => panic!("expected Multiple([Schema, ...]), got {other:?}"),
        };
        assert!(
            messages
                .iter()
                .any(|m| m.contains("hint:") && m.contains("`raw:`")),
            "expected a hint mentioning `raw:`, got messages:\n{}",
            messages.join("\n---\n"),
        );
    }

    #[test]
    fn unknown_top_field_is_rejected_by_schema() {
        // `unevaluatedProperties: false` in top.schema.yaml rejects this at the
        // schema layer (before serde would), pointing at the document root.
        let paths = assert_schema_rejected(
            r#"
            blocks: []
            mystery_field: 42
        "#,
        );
        assert!(
            paths.iter().any(|p| p.is_empty() || p == "/"),
            "expected a root-level violation, got {paths:?}"
        );
    }

    #[test]
    fn bad_logging_level_is_rejected_by_schema() {
        // `level` is constrained to an enum; "bogus" is not a member.
        let paths = assert_schema_rejected(
            r#"
            logging:
              level: bogus
            blocks: []
        "#,
        );
        assert!(
            paths.iter().any(|p| p == "/logging/level"),
            "expected /logging/level violation, got {paths:?}"
        );
    }

    #[test]
    fn raw_block_missing_component_is_rejected_by_schema() {
        // `component` is required on a raw block.
        let paths = assert_schema_rejected(
            r#"
            blocks:
              - raw:
                  label: stub
        "#,
        );
        assert!(
            paths.iter().any(|p| p.starts_with("/blocks/0")),
            "expected a /blocks/0 violation, got {paths:?}"
        );
    }

    #[test]
    fn loki_echo_sugar_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
        blocks:
          - loki.echo:
              label: stub
        "#,
        )
        .unwrap();
        assert_renders(pipeline.render(), "loki.echo \"stub\" { }\n");
    }

    #[test]
    fn loki_source_journal_sugar_renders_refs_and_attrs() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
        blocks:
          - loki.source.journal:
              forward_to: ["loki.write.gateway.receiver"]
              max_age: "7h"
              labels:
                job: "systemd-journal"
        "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "loki.source.journal {\n",
                "\tforward_to = [\n",
                "\t\tloki.write.gateway.receiver,\n", // bare ref, NOT quoted
                "\t]\n",
                "\tlabels = {\n",
                "\t\tjob = \"systemd-journal\",\n",
                "\t}\n",
                "\tmax_age = \"7h\"\n",
                "}\n",
            ),
        );
    }
}
