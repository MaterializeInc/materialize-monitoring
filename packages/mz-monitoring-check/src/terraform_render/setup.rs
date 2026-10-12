// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Plan each example and render the chart against what it composed.
//!
//! Everything here runs before the harness starts, the way the E2E suite
//! connects to its cluster first. A plan or a render that fails is the example's
//! outcome and becomes a failed trial; a tool that cannot be started at all is a
//! setup error, since no example can get past it.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::thread;

use anyhow::{Context, Result, bail};
use serde_json::Value;

use super::TerraformRenderArgs;
use super::ctx::{Ctx, parse_yaml};
use super::manifests::Manifests;

/// The release name and namespace the chart is rendered as. Rendered object
/// names that the checks look up — `mzmon-alloy-gateway`, say — follow from
/// the release name.
pub const RELEASE: &str = "mzmon";
const NAMESPACE: &str = "monitoring";

/// One example and how far it got.
pub struct Example {
    pub name: String,
    pub outcome: Outcome,
}

pub enum Outcome {
    /// `terraform init`, `plan` or `show` failed, or the plan composed no
    /// values for the chart.
    PlanFailed(String),
    /// The chart refused the composed values. Its validators run at render time,
    /// which is the point: a backend mismatch the module introduces fails here
    /// with the same message an operator would get from `helm install`.
    RenderFailed(String),
    Rendered(Arc<Ctx>),
}

/// Plan and render every selected example.
///
/// `terraform init` runs one example at a time, because Terraform does not
/// promise a shared plugin cache survives concurrent installs. Planning and
/// rendering, which are most of the runtime, run in parallel.
pub fn plan_and_render(args: &TerraformRenderArgs) -> Result<Vec<Example>> {
    let examples = discover(&args.examples_dir, &args.examples)?;

    let chart_values_path = args.chart.join("values.yaml");
    let chart_values = Arc::new(parse_yaml(
        &std::fs::read_to_string(&chart_values_path)
            .with_context(|| format!("reading {}", chart_values_path.display()))?,
    )?);

    let work = tempfile::tempdir().context("creating a scratch directory")?;
    let tools = Tools {
        terraform: &args.terraform,
        helm: &args.helm,
        chart: &args.chart,
        work: work.path(),
    };

    let mut initialized = Vec::new();
    for (name, dir) in examples {
        eprintln!("==> terraform init {name}");
        let init = run(tools.terraform(&dir).args([
            "init",
            "-backend=false",
            "-input=false",
            "-no-color",
        ]))?;
        initialized.push((name, dir, init));
    }

    eprintln!("==> terraform plan and helm template, in parallel");
    thread::scope(|scope| {
        let handles: Vec<_> = initialized
            .into_iter()
            .map(|(name, dir, init)| {
                let tools = &tools;
                let chart_values = Arc::clone(&chart_values);
                scope.spawn(move || -> Result<Example> {
                    let outcome = match init {
                        Err(err) => Outcome::PlanFailed(err),
                        Ok(_) => tools.example(&name, &dir, chart_values)?,
                    };
                    match &outcome {
                        Outcome::Rendered(ctx) => eprintln!(
                            "    {name}: {} value documents, {} objects rendered",
                            ctx.value_documents().len(),
                            ctx.rendered.len(),
                        ),
                        Outcome::PlanFailed(_) => eprintln!("    {name}: plan failed"),
                        Outcome::RenderFailed(_) => eprintln!("    {name}: render failed"),
                    }
                    Ok(Example { name, outcome })
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().expect("an example's setup thread panicked"))
            .collect()
    })
}

/// The examples to plan, sorted by name: every subdirectory with a `main.tf`,
/// or only the ones named.
fn discover(dir: &Path, only: &[String]) -> Result<Vec<(String, PathBuf)>> {
    let mut examples: Vec<(String, PathBuf)> = std::fs::read_dir(dir)
        .with_context(|| format!("reading {}", dir.display()))?
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .filter(|path| path.join("main.tf").is_file())
        .filter_map(|path| {
            let name = path.file_name()?.to_str()?.to_owned();
            Some((name, path))
        })
        .collect();
    examples.sort();

    for name in only {
        if !examples.iter().any(|(found, _)| found == name) {
            bail!("no example named {name:?} under {}", dir.display());
        }
    }
    if !only.is_empty() {
        examples.retain(|(name, _)| only.contains(name));
    }
    if examples.is_empty() {
        bail!("no examples under {}", dir.display());
    }
    Ok(examples)
}

struct Tools<'a> {
    terraform: &'a str,
    helm: &'a str,
    chart: &'a Path,
    work: &'a Path,
}

impl Tools<'_> {
    fn terraform(&self, dir: &Path) -> Command {
        let mut cmd = Command::new(self.terraform);
        cmd.current_dir(dir);
        cmd
    }

    /// Plan one example, extract its composed values, and render the chart.
    fn example(&self, name: &str, dir: &Path, chart_values: Arc<Value>) -> Result<Outcome> {
        let plan_file = self.work.join(format!("{name}.tfplan"));
        // A kubeconfig path that cannot exist, so a stray provider connection
        // fails loudly here rather than silently reaching the operator's own
        // cluster.
        let kubeconfig = self.work.join("no-such-kubeconfig");
        if let Err(err) = run(self
            .terraform(dir)
            .args(["plan", "-input=false", "-no-color", "-var"])
            .arg(format!("kubeconfig_path={}", kubeconfig.display()))
            .arg("-out")
            .arg(&plan_file))?
        {
            return Ok(Outcome::PlanFailed(err));
        }

        let shown = match run(self
            .terraform(dir)
            .args(["show", "-json", "-no-color"])
            .arg(&plan_file))?
        {
            Ok(stdout) => stdout,
            Err(err) => return Ok(Outcome::PlanFailed(err)),
        };
        let plan: Value =
            serde_json::from_slice(&shown).context("parsing `terraform show -json` output")?;

        let documents = composed_documents(&plan);
        if documents.is_empty() {
            return Ok(Outcome::PlanFailed(
                "no helm_release values found in the plan".to_owned(),
            ));
        }

        // The module composes an ordered list of YAML documents, and Helm merges
        // them with later documents winning. Repeated `-f` has the same
        // semantics, so writing each out in order and passing them all
        // reproduces the release.
        let mut helm = Command::new(self.helm);
        helm.args(["template", RELEASE])
            .arg(self.chart)
            .args(["--namespace", NAMESPACE]);
        let mut values = Vec::with_capacity(documents.len());
        for (i, document) in documents.iter().enumerate() {
            let path = self.work.join(format!("{name}-{i}.yaml"));
            std::fs::write(&path, document)
                .with_context(|| format!("writing {}", path.display()))?;
            helm.arg("-f").arg(&path);
            match parse_yaml(document) {
                Ok(value) => values.push(value),
                Err(err) => {
                    return Ok(Outcome::PlanFailed(format!(
                        "value document {i} is not YAML: {err:#}"
                    )));
                }
            }
        }

        let rendered = match run(&mut helm)? {
            Ok(stdout) => String::from_utf8(stdout).context("`helm template` wrote non-UTF-8")?,
            Err(err) => return Ok(Outcome::RenderFailed(err)),
        };
        let rendered = match Manifests::parse(rendered) {
            Ok(rendered) => rendered,
            Err(err) => {
                return Ok(Outcome::RenderFailed(format!(
                    "the rendered chart did not parse: {err:#}"
                )));
            }
        };

        Ok(Outcome::Rendered(Arc::new(Ctx::new(
            plan,
            values,
            rendered,
            chart_values,
        ))))
    }
}

/// The values documents of the `monitoring` release, in the order the module
/// composed them.
fn composed_documents(plan: &Value) -> Vec<String> {
    plan.pointer("/planned_values/root_module/child_modules")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|module| module.get("resources")?.as_array())
        .flatten()
        .filter(|resource| {
            resource.get("type").and_then(Value::as_str) == Some("helm_release")
                && resource.get("name").and_then(Value::as_str) == Some("monitoring")
        })
        .filter_map(|resource| resource.pointer("/values/values")?.as_array())
        .flatten()
        .filter_map(|document| document.as_str().map(str::to_owned))
        .collect()
}

/// Run a command to completion and return its stdout.
///
/// Two layers, because they mean different things. The outer error is the tool
/// not starting at all — not on PATH, say — and stops the run. The inner one is
/// the tool exiting non-zero, which is something the example did, and is
/// returned for the caller to record as a failed trial with the tool's own
/// message.
fn run(cmd: &mut Command) -> Result<std::result::Result<Vec<u8>, String>> {
    let program = cmd.get_program().to_string_lossy().into_owned();
    let output = cmd
        .output()
        .with_context(|| format!("running `{program}` (is it on PATH?)"))?;
    if output.status.success() {
        return Ok(Ok(output.stdout));
    }
    // The subcommand only: the rest names files in a scratch directory that is
    // gone by the time anyone reads this.
    let subcommand = cmd
        .get_args()
        .next()
        .map(|arg| arg.to_string_lossy().into_owned())
        .unwrap_or_default();
    Ok(Err(format!(
        "`{program} {subcommand}` failed ({}):\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim_end()
    )))
}
