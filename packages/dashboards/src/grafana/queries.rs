// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The query registry, as a dashboard sees it.
//!
//! Panels do not write PromQL. A panel names a registry query by id and gets back
//! both the expression *and* the prose describing it, so the two cannot drift:
//! the registry is where a query's semantics and its explanation are maintained
//! together, and a dashboard is one of several consumers of that pair.
//!
//! What stays with the panel is presentation — legend templates, units, panel
//! type, thresholds, transformations. A legend in particular is deliberately not a
//! registry field: the same query legitimately reads `{{name}}` on one panel and
//! `{{cluster_name}} / {{name}}` on another.
//!
//! ## Why lookups are infallible
//!
//! A bad query id is a bug in the dashboard, not a runtime condition, and it would
//! be found by the first render. Threading a `Result` through all sixty-odd panel
//! builders to carry a case that only fires on a typo would cost far more than it
//! buys.
//!
//! So [`Queries::get`] hands back a placeholder and records the failure, and
//! [`Queries::failures`] is checked once when the dashboard is assembled. The
//! result is that a typo reports *every* bad id at once rather than the first, and
//! a placeholder can never reach a rendered artifact because assembly fails first.

use std::cell::RefCell;

use mzmon_lib::grafana::context::{DashboardScope, dashboard_context};
use mzmon_lib::grafana::query::{PanelQuery, panel_query, query_group};
use mzmon_lib::query::{QueryEngine, QueryRegistry, render::TemplateContext};

/// A registry plus the contexts a dashboard renders it in.
///
/// Two contexts, not one. A query's engine is a property of the query — a
/// Kubernetes event has no PromQL form and a peek histogram has no LogQL one — so
/// the engine cannot be a per-dashboard setting. It is chosen per lookup instead:
/// [`Queries::get`] renders PromQL and [`Queries::logs`] renders LogQL, and each
/// carries the datasource variable its engine belongs to.
///
/// Both contexts share one scope, so a filter fragment resolves identically
/// whichever engine reads it. That is what lets a stream selector and a metric
/// selector agree on which namespaces they are looking at.
pub struct Queries<'a> {
    registry: &'a QueryRegistry,
    metrics: TemplateContext<'a>,
    logs: TemplateContext<'a>,
    failures: RefCell<Vec<String>>,
}

impl<'a> Queries<'a> {
    /// Bind a registry to the Grafana rendering contexts.
    pub fn new(registry: &'a QueryRegistry, scope: &DashboardScope) -> Self {
        Queries {
            registry,
            metrics: dashboard_context(registry, QueryEngine::PromQl, scope),
            logs: dashboard_context(registry, QueryEngine::LogQl, scope),
            failures: RefCell::new(Vec::new()),
        }
    }

    /// Bridge a registry query into a panel's query group and description.
    pub fn get(&self, id: &str) -> PanelQuery {
        self.bridge(id, &self.metrics)
    }

    /// [`Queries::get`] for a query written in LogQL, against the logs datasource.
    ///
    /// Asking for the wrong engine is recorded as a failure rather than falling
    /// back: a query with no LogQL form would otherwise render its PromQL against
    /// Loki, which parses far enough to produce an empty panel rather than an
    /// error.
    pub fn logs(&self, id: &str) -> PanelQuery {
        self.bridge(id, &self.logs)
    }

    /// Look `id` up in one of the contexts, recording a failure rather than
    /// propagating it.
    fn bridge(&self, id: &str, ctx: &TemplateContext) -> PanelQuery {
        match panel_query(self.registry, id, ctx) {
            Ok(panel) => panel,
            Err(e) => {
                self.failures.borrow_mut().push(format!("{id}: {e}"));
                self.placeholder(id)
            }
        }
    }

    /// [`Queries::get`], labeling each expression with its own legend template.
    ///
    /// The templates are positional against the query's `promQL` list, so a count
    /// mismatch means the panel and the query have drifted and the labels would
    /// land on the wrong series. That is recorded as a failure rather than
    /// silently truncated.
    pub fn legended(&self, id: &str, legends: &[&str]) -> PanelQuery {
        let panel = self.get(id);
        match panel.clone().legends(legends) {
            Ok(labeled) => labeled,
            Err(e) => {
                self.failures.borrow_mut().push(format!("{id}: {e}"));
                panel
            }
        }
    }

    /// Every lookup that failed, in the order they were attempted.
    pub fn failures(&self) -> Vec<String> {
        self.failures.borrow().clone()
    }

    /// Stand-in for a query that could not be bridged.
    ///
    /// Never reaches an artifact: the caller checks [`Queries::failures`] before
    /// returning a dashboard. It exists so panel builders stay infallible.
    fn placeholder(&self, id: &str) -> PanelQuery {
        PanelQuery {
            query_group: query_group(Vec::new()),
            description: format!("unresolved registry query `{id}`"),
        }
    }
}

/// A registry loaded once, for tests.
///
/// `Queries` borrows its registry, so a helper returning one needs the registry to
/// outlive it — hence a process-lifetime `OnceLock` rather than a local.
#[cfg(test)]
pub(crate) fn test_registry() -> &'static QueryRegistry {
    static REGISTRY: std::sync::OnceLock<QueryRegistry> = std::sync::OnceLock::new();
    REGISTRY.get_or_init(|| {
        let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../queries");
        QueryRegistry::from_directory(&dir).expect("load the query registry")
    })
}

/// [`Queries`] over [`test_registry`], with self-managed scope.
#[cfg(test)]
pub(crate) fn test_queries() -> Queries<'static> {
    // Leaked so the returned `Queries` is `'static`: one scope per test process is
    // all these tests need, and the alternative is threading a scope binding
    // through every call site.
    let scope: &'static DashboardScope = Box::leak(Box::new(DashboardScope::default()));
    Queries::new(test_registry(), scope)
}

/// [`test_queries`] in the scope a dashboard that defines `$operatorNamespace`
/// builds in.
///
/// A tab has to be tested in the scope its dashboard renders it in, or an
/// assertion about what a query resolves to is about a rendering that never
/// ships. The two differ in exactly one parameter, and it is the one the operator
/// queries are built on.
#[cfg(test)]
pub(crate) fn test_operator_queries() -> Queries<'static> {
    let scope: &'static DashboardScope =
        Box::leak(Box::new(DashboardScope::default().operator_variable()));
    Queries::new(test_registry(), scope)
}
