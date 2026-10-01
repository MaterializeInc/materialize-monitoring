// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The registry's alerts as rules for the Thanos and Loki rulers.
//!
//! - [`context`] renders a query for a ruler: install-time placeholders for the
//!   facts that differ per deployment, and no parameters for the choices a
//!   dashboard viewer makes.
//! - [`capability`] is the vocabulary rules are selected by, and the table that
//!   infers a rule's requirements from the metrics it reads.
//! - [`render`] validates and renders every alert, PromQL and LogQL, and
//!   serializes the rule files and the index the chart selects from.

pub mod capability;
pub mod context;
pub mod render;

pub use capability::Capability;
pub use context::{Placeholder, alerting_context};
pub use render::{RenderedRule, RuleEngine, RuleError, RuleSet, render_rules};
