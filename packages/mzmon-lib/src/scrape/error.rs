// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Errors for the scrape transpiler. Mirrors `alloy::error` but scoped to this
//! subsystem so the two stay decoupled.

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("YAML error: {0}")]
    Yaml(#[from] serde_yaml_ng::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// The document's `apiVersion`/`kind` did not name a Monitor we transpile.
    #[error("unrecognized monitor: apiVersion={api_version:?} kind={kind:?}")]
    UnknownKind {
        api_version: Option<String>,
        kind: Option<String>,
    },

    #[error("schema violation at `{path}`: {message}")]
    Schema { path: String, message: String },

    /// A construct that is valid prometheus-operator input but that this
    /// best-effort transpiler does not (yet) translate.
    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("{}", format_multiple(.0))]
    Multiple(Vec<Error>),
}

/// Render a `Multiple` as a header plus an indented bullet per child error.
/// (Copied from `alloy::error` so the two subsystems stay independent.)
fn format_multiple(errs: &[Error]) -> String {
    let mut out = format!("{} errors:", errs.len());
    for e in errs {
        for (i, line) in e.to_string().lines().enumerate() {
            out.push_str(if i == 0 { "\n  - " } else { "\n    " });
            out.push_str(line);
        }
    }
    out
}

pub type Result<T> = std::result::Result<T, Error>;
