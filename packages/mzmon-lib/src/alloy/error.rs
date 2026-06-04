// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),

    #[error("formatter error")]
    Fmt(#[from] fmt::Error),

    #[error("YAML error: {0}")]
    YamlError(#[from] serde_yaml_ng::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("rendering error: {0}")]
    Render(String),

    #[error("schema violation at `{path}`: {message}")]
    Schema { path: String, message: String },

    #[error("{}", format_multiple(.0))]
    Multiple(Vec<Error>),
}

/// Render a `Multiple` as a header plus an indented bullet per child error.
/// Children that are themselves multi-line (e.g. nested `Multiple`) keep their
/// shape via continuation indentation.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_renders_children_indented() {
        // Previously `Multiple` displayed only "multiple errors", swallowing the
        // children. It now lists each child as a bullet, and a nested `Multiple`
        // keeps its shape under continuation indentation.
        let e = Error::Multiple(vec![
            Error::Render("first".into()),
            Error::Multiple(vec![Error::Render("nested".into())]),
        ]);
        let s = e.to_string();
        assert!(s.starts_with("2 errors:"), "got:\n{s}");
        assert!(s.contains("\n  - rendering error: first"), "got:\n{s}");
        assert!(s.contains("\n  - 1 errors:"), "got:\n{s}");
        // the nested child's own bullet is already 2-space indented, then the
        // 4-space continuation prefix is applied on top → 6 spaces.
        assert!(s.contains("\n      - rendering error: nested"), "got:\n{s}");
    }
}
