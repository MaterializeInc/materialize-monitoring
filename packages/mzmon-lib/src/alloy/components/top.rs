// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use crate::alloy::ast::{AttributeValue, Block, ToBlock};
use crate::alloy::error::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

type LogLevel = String;
type LogFormat = String;

/// Top-level "logging" block, which configures logging for alloy itself.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LoggingBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub level: Option<LogLevel>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub format: Option<LogFormat>,
}

/// Top-level "livedebugging" block, which allows alloy UI to show live data.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct LiveDebuggingBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

impl ToBlock for LoggingBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(level) = &self.level {
            attributes.insert("level".into(), AttributeValue::String(level.clone()));
        }
        if let Some(format) = &self.format {
            attributes.insert("format".into(), AttributeValue::String(format.clone()));
        }
        Ok(Block {
            attributes,
            ..Block::new("logging")
        })
    }
}

impl ToBlock for LiveDebuggingBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(enabled) = self.enabled {
            attributes.insert("enabled".into(), AttributeValue::Bool(enabled));
        }
        Ok(Block {
            attributes,
            ..Block::new("livedebugging")
        })
    }
}
