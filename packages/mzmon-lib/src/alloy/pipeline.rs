// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use crate::alloy::ast::{Block, ToBlock};
use crate::alloy::components::top;
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
#[serde(rename_all = "snake_case")]
pub enum ComponentBlock {
    Raw(Block),
}

impl ToBlock for ComponentBlock {
    fn to_block(&self) -> Result<Block> {
        match self {
            ComponentBlock::Raw(rb) => Ok(rb.clone()),
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
        // 2. JSON value → typed Pipeline (serde_json drives enum dispatch = map form)
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
        assert_eq!(
            pipeline.render().unwrap(),
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
        assert_eq!(pipeline.render().unwrap(), "loki.echo \"stub\" {}\n",);
    }

    #[test]
    fn unknown_top_field_is_rejected() {
        let yaml = r#"
            blocks: []
            mystery_field: 42
        "#;
        assert!(Pipeline::from_yaml_str(yaml).is_err());
    }
}
