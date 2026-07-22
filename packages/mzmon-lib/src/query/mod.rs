// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Registry of monitoring queries, ported from `py_mzmon_lib.registry`.
//!
//! A [`registry::QueryRegistry`] is a set of [`model::Query`] definitions (plus
//! recording [`model::Rule`]s and [`model::Alert`]s) loaded from the YAML files
//! under `packages/queries/`, each validated against
//! `schemas/query/mzmon-query.schema.yaml`. A query carries one template per
//! *query engine* (PromQL, Datadog, Honeycomb, LogQL); the same registry entry
//! renders differently per engine because the concrete parameter values and
//! template-engine functions are supplied by a [`render::TemplateContext`] at
//! render time, not baked into the definition.
//!
//! The two things built on top of the model:
//!
//! * **Rendering** ([`render`]) turns a query's `%%{param}` templates into a
//!   concrete expression for one engine, applying template-engine functions
//!   (`orZero`, …) along the way.
//! * **Metric extraction** ([`extract`], [`docgen`]) parses rendered PromQL and
//!   collects the metric names + label matchers each query touches. This is the
//!   input to the `extract-metrics` CLI (the Rust equivalent of the Python
//!   `query_cli docgen`) and, later, to cardinality-reduction pipelines.
//!
//! Shape mirrors the sibling `scrape` / `alloy` modules: a typed model, schema
//! validation in [`validate`], and inline `#[cfg(test)]` tests per module.

pub mod def;
pub mod docgen;
pub mod error;
pub mod extract;
pub mod importance;
pub mod model;
pub mod registry;
pub mod render;
pub mod stability;
pub mod validate;

#[cfg(test)]
pub(crate) mod test_support;

pub use error::{Error, Result};
pub use importance::Importance;
pub use model::{Alert, Description, Query, QueryEngine, Rule, TemplateExpr, TemplateFunction};
pub use registry::{MetricOverride, QueryRegistry};
pub use render::TemplateContext;
pub use stability::Stability;
