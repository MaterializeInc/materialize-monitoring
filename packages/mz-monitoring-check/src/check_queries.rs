// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! `check-queries`: validate query-registry YAML against
//! `mzmon-query.schema.yaml`.
//!
//! This replaces the `ajv-validate-query` pre-commit hook (and its Node.js
//! dependency) with the embedded validator in `mzmon_lib::query::validate` — the
//! same schema, so decisions match, but with no `npx`. As a pre-commit hook it is
//! passed the changed files as arguments; run directly with no arguments it
//! checks every `*.yaml` under `--source-dir`.

use std::path::PathBuf;

use anyhow::{Context, bail};
use mzmon_lib::query::validate::validate_yaml_str;

/// Arguments for the `check-queries` command.
#[derive(clap::Args)]
pub struct CheckQueriesArgs {
    /// Query files to validate. If none are given, every `*.yaml` under
    /// `--source-dir` is checked.
    files: Vec<PathBuf>,

    /// Directory scanned for `*.yaml` files when no explicit files are passed.
    #[arg(long, default_value = "packages/queries")]
    source_dir: PathBuf,
}

/// Entry point for `mz-monitoring-check check-queries`.
pub fn check_queries(args: CheckQueriesArgs) -> anyhow::Result<()> {
    let files = if args.files.is_empty() {
        discover(&args.source_dir)?
    } else {
        args.files
    };

    if files.is_empty() {
        bail!("no query files to check in {}", args.source_dir.display());
    }

    let mut failures = 0usize;
    for file in &files {
        let yaml =
            std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
        if let Err(err) = validate_yaml_str(&yaml) {
            failures += 1;
            eprintln!("{}: FAILED", file.display());
            // `Error::Multiple` renders as a bulleted list; indent it under the
            // filename so multi-file output stays readable.
            for line in err.to_string().lines() {
                eprintln!("  {line}");
            }
        }
    }

    if failures > 0 {
        bail!(
            "{failures} of {} query file(s) failed schema validation",
            files.len()
        );
    }
    eprintln!("checked {} query file(s): all valid", files.len());
    Ok(())
}

/// List the `*.yaml` files in `dir`, sorted for deterministic output.
fn discover(dir: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("yaml"))
        .collect();
    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Write `contents` to a `.yaml` file under a fresh temp dir; return both so
    /// the dir stays alive for the test's duration.
    fn temp_query_file(contents: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("q.yaml");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(contents.as_bytes()).unwrap();
        (dir, path)
    }

    const VALID: &str = r#"
description: test
metricImportanceHint: recommended
queries:
  - id: q
    stability: best-effort
    description: {summary: s}
    promQL: 'up{}'
"#;

    // Missing the required `id`.
    const INVALID: &str = r#"
description: test
metricImportanceHint: recommended
queries:
  - stability: best-effort
    description: {summary: s}
    promQL: 'up{}'
"#;

    #[test]
    fn valid_file_passes() {
        let (_dir, path) = temp_query_file(VALID);
        let args = CheckQueriesArgs {
            files: vec![path],
            source_dir: "packages/queries".into(),
        };
        assert!(check_queries(args).is_ok());
    }

    #[test]
    fn invalid_file_fails() {
        let (_dir, path) = temp_query_file(INVALID);
        let args = CheckQueriesArgs {
            files: vec![path],
            source_dir: "packages/queries".into(),
        };
        assert!(check_queries(args).is_err());
    }

    #[test]
    fn discovers_yaml_in_a_directory() {
        let (dir, _path) = temp_query_file(VALID);
        let args = CheckQueriesArgs {
            files: vec![],
            source_dir: dir.path().to_path_buf(),
        };
        assert!(check_queries(args).is_ok());
    }

    #[test]
    fn real_query_files_pass() {
        // The checked-in registry under packages/queries must validate.
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../queries");
        let args = CheckQueriesArgs {
            files: vec![],
            source_dir: dir,
        };
        check_queries(args).expect("packages/queries should pass schema validation");
    }
}
