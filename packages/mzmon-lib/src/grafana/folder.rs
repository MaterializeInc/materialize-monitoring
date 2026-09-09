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
//! is the Grafana app-platform convention, so the operator applies the resource
//! as-is and needs no per-dashboard wiring in the chart.
//!
//! Grafana reads that annotation as a folder **UID**, and a UID is not something
//! this crate can know — it depends on the release the dashboard is installed
//! into. So what is rendered here is a **name, not a UID**: the chart's
//! `dashboards.config.grafana.folders` is keyed by exactly these strings, and
//! `templates/dashboards/grafana-operator/dashboards.yaml` rewrites the
//! annotation to the real UID on the way past, the same way it rewrites
//! `apiVersion`. A dashboard whose name matches no key has the annotation removed
//! rather than rewritten, and lands at the root.
//!
//! Adding a variant here therefore means adding the matching key to the chart's
//! `folders` map. `charts/materialize-monitoring/tests/folders_test.yaml` is what
//! notices if the two disagree.

use serde::{Deserialize, Serialize};

/// The annotation the Grafana app platform reads a dashboard's folder from.
pub const FOLDER_ANNOTATION: &str = "grafana.app/folder";

/// The folder a dashboard is filed under.
///
/// Each variant's [`Display`] output is the folder *name* written to
/// [`FOLDER_ANNOTATION`], which the chart resolves to a UID — see the module docs.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum Folder {
    /// Grafana's root folder. Emits no annotation at all.
    ///
    /// The default, so a dashboard that says nothing about placement keeps the
    /// one it had before folders existed. Filing a dashboard is a decision, and
    /// an unmade decision should not put it somewhere arbitrary.
    #[default]
    Root,
    /// The platform a Materialize deployment runs on.
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
            Folder::Infra => "infra",
            Folder::MetaO11y => "meta-o11y",
            Folder::Materialize => "materialize",
        })
    }
}
