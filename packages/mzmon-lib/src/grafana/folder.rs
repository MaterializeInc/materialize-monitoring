// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Which Grafana folder a dashboard lands in.
//!
//! Placement travels with the dashboard rather than beside it: the folder is
//! encoded as a [`FOLDER_ANNOTATION`] annotation on the dashboard resource, which
//! is the Grafana app-platform convention, so the Grafana Operator applies the
//! resource as-is and needs no per-dashboard wiring in the chart.
//!
//! The annotation holds a folder **UID**, not a title, and the folder has to
//! already exist — a dashboard naming a UID nothing created lands in the root
//! folder. The chart creates them: `dashboards.config.grafana.folders` renders one
//! `GrafanaFolder` per entry with `uid: <fullname>-<key>`, and the chart pins
//! `fullnameOverride: "mzmon"`, which is what makes the UIDs below the ones that
//! exist on a default install.
//!
//! **That coupling is by name only.** These strings and the chart's folder keys
//! are two halves of the same contract with nothing asserting they agree, so an
//! install that overrides `fullnameOverride` moves the folders out from under
//! every dashboard here.

use serde::{Deserialize, Serialize};

/// The annotation the Grafana app platform reads a dashboard's folder UID from.
pub const FOLDER_ANNOTATION: &str = "grafana.app/folder";

/// The folder a dashboard is filed under.
///
/// Each variant's [`Display`] output is the folder UID written to
/// [`FOLDER_ANNOTATION`] — see the module docs for what has to exist on the other
/// end of that string.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum Folder {
    /// Grafana's root folder. Emits no annotation at all, which is the placement
    /// every dashboard had before folders existed.
    Root,
    /// The platform a Materialize deployment runs on.
    #[default]
    Infra,
    /// The monitoring stack watching itself.
    MetaO11y,
    /// Materialize itself.
    Materialize,
}

impl std::fmt::Display for Folder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Root is written out for completeness; `Dashboard::build` never asks for
        // it, because Root is the absence of the annotation rather than a value
        // for it.
        f.write_str(match self {
            Folder::Root => "root",
            Folder::Infra => "mzmon-infra",
            Folder::MetaO11y => "mzmon-meta-o11y",
            Folder::Materialize => "mzmon-materialize",
        })
    }
}
