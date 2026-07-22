// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The [`QueryRegistry`]: a keyed collection of queries, recording rules, and
//! alerts, loaded from the YAML files under `packages/queries/`.
//!
//! Ported from `py_mzmon_lib.registry.queries.QueryRegistry`. Insertion order is
//! preserved ([`IndexMap`]) so iteration is deterministic, though the
//! `extract-metrics` output is sorted by content and so does not depend on it.
//!
//! Registration is unique-by-id and promotes inline dependencies (a `query`
//! nested under another query's `dependencies`, or under a rule/alert) to
//! top-level entries — matching the Python contract that "registration
//! automatically promotes dependencies to top-level" while "dependencies are not
//! checked at registration time" (so load order is irrelevant).

use std::path::Path;

use indexmap::IndexMap;
use regex::Regex;

use crate::query::def::{
    AlertDef, DependencyDef, MetricOverrideDef, QueryDef, RegistryDoc, RuleDef,
    template_exprs_from_value,
};
use crate::query::error::{Error, Result};
use crate::query::importance::Importance;
use crate::query::model::{Alert, Query, QueryId, Rule};

/// A compiled metric-importance override: set every metric whose name matches
/// `pattern` to `importance` outright. `pattern` is anchored (the whole name must
/// match), mirroring Prometheus regex semantics.
#[derive(Debug, Clone)]
pub struct MetricOverride {
    pattern: Regex,
    raw_pattern: String,
    importance: Importance,
    priority: i64,
}

impl MetricOverride {
    /// Compile a raw override definition, anchoring its pattern.
    pub fn compile(def: MetricOverrideDef) -> Result<Self> {
        let pattern = Regex::new(&format!("^(?:{})$", def.metric_pattern)).map_err(|err| {
            Error::InvalidPattern {
                pattern: def.metric_pattern.clone(),
                message: err.to_string(),
            }
        })?;
        Ok(MetricOverride {
            pattern,
            raw_pattern: def.metric_pattern,
            importance: def.importance,
            priority: def.priority,
        })
    }

    /// The importance this override assigns.
    pub fn importance(&self) -> Importance {
        self.importance
    }

    /// The override's priority; higher wins when several overrides match.
    pub fn priority(&self) -> i64 {
        self.priority
    }

    /// The original (unanchored) pattern text.
    pub fn pattern(&self) -> &str {
        &self.raw_pattern
    }

    /// Whether this override matches `metric_name`.
    pub fn matches(&self, metric_name: &str) -> bool {
        self.pattern.is_match(metric_name)
    }
}

/// A registry of monitoring queries (plus recording rules, alerts, and
/// metric-importance overrides).
#[derive(Debug, Clone, Default)]
pub struct QueryRegistry {
    queries: IndexMap<QueryId, Query>,
    rules: IndexMap<String, Rule>,
    alerts: IndexMap<String, Alert>,
    metric_overrides: Vec<MetricOverride>,
}

impl QueryRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    // -- lookup ------------------------------------------------------------

    /// Get a query by id, or `None` if it is not registered.
    pub fn get(&self, id: &str) -> Option<&Query> {
        self.queries.get(id)
    }

    /// Get a recording rule by its `record` name.
    pub fn rule(&self, record: &str) -> Option<&Rule> {
        self.rules.get(record)
    }

    /// Get an alert by its `alert` name.
    pub fn alert(&self, name: &str) -> Option<&Alert> {
        self.alerts.get(name)
    }

    /// Number of registered queries (matches Python `len(registry)`).
    pub fn len(&self) -> usize {
        self.queries.len()
    }

    /// True if no queries are registered.
    pub fn is_empty(&self) -> bool {
        self.queries.is_empty()
    }

    // -- iteration ---------------------------------------------------------

    /// Iterate over all queries in registration order.
    pub fn queries(&self) -> impl Iterator<Item = &Query> {
        self.queries.values()
    }

    /// Iterate over all recording rules in registration order.
    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.rules.values()
    }

    /// Iterate over all alerts in registration order.
    pub fn alerts(&self) -> impl Iterator<Item = &Alert> {
        self.alerts.values()
    }

    /// The compiled metric-importance overrides, in load order.
    pub fn metric_overrides(&self) -> &[MetricOverride] {
        &self.metric_overrides
    }

    /// The override-assigned importance for `metric_name`, if any override
    /// matches. Among matches the highest [`priority`](MetricOverride::priority)
    /// wins; equal priorities resolve in load order (the later declaration wins).
    pub fn override_importance(&self, metric_name: &str) -> Option<Importance> {
        self.metric_overrides
            .iter()
            .filter(|ov| ov.matches(metric_name))
            // `max_by_key` returns the last maximal element, so a later
            // declaration wins a priority tie.
            .max_by_key(|ov| ov.priority)
            .map(|ov| ov.importance)
    }

    /// Iterate over queries that carry a metric (PromQL/Datadog/Honeycomb)
    /// definition.
    pub fn iter_metric_queries(&self) -> impl Iterator<Item = &Query> {
        self.queries.values().filter(|q| q.is_metric_query())
    }

    /// Iterate over queries that carry a LogQL definition. With
    /// `exclude_metric_queries`, skip those that also carry a metric definition.
    pub fn iter_log_queries(&self, exclude_metric_queries: bool) -> impl Iterator<Item = &Query> {
        self.queries
            .values()
            .filter(move |q| q.is_log_query() && (!exclude_metric_queries || !q.is_metric_query()))
    }

    // -- loading -----------------------------------------------------------

    /// Load every query/rule/alert/override from a parsed registry document.
    /// Each metric query is stamped with the file's `metricImportanceHint`.
    pub fn load(&mut self, doc: RegistryDoc) -> Result<()> {
        let hint = doc.metric_importance_hint;
        for query in doc.queries {
            self.register_query(query, hint)?;
        }
        for rule in doc.rules {
            self.register_rule(rule, hint)?;
        }
        for alert in doc.alerts {
            self.register_alert(alert, hint)?;
        }
        for override_def in doc.metric_overrides {
            self.metric_overrides
                .push(MetricOverride::compile(override_def)?);
        }
        Ok(())
    }

    /// Load a registry from every `*.yaml` file in `directory`, in sorted
    /// filename order (mirrors `QueryRegistry.from_directory`).
    pub fn from_directory(directory: &Path) -> Result<Self> {
        let mut registry = Self::new();
        let mut files: Vec<_> = std::fs::read_dir(directory)?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("yaml"))
            .collect();
        files.sort();
        for path in files {
            let yaml = std::fs::read_to_string(&path)?;
            let doc = RegistryDoc::from_yaml_str(&yaml).map_err(|err| {
                // Attach the offending file for a friendlier message.
                match err {
                    Error::Yaml(e) => Error::Schema {
                        path: path.display().to_string(),
                        message: e.to_string(),
                    },
                    other => other,
                }
            })?;
            registry.load(doc)?;
        }
        Ok(registry)
    }

    // -- registration ------------------------------------------------------

    /// Register a query, stamping `importance` from the source file's hint and
    /// promoting any inline dependencies (which inherit the same hint) to
    /// top-level entries. Returns the registered query's id; errors on a
    /// duplicate id.
    pub fn register_query(&mut self, def: QueryDef, importance: Importance) -> Result<QueryId> {
        let id = def.id.clone();
        if self.queries.contains_key(&id) {
            return Err(Error::DuplicateQuery(id));
        }

        // Resolve dependencies, registering inline definitions first so the edge
        // records a real id. (Consumes `def.dependencies`; other fields of `def`
        // are read afterwards via partial move.)
        let mut dependencies = Vec::new();
        for dep in def.dependencies {
            match dep {
                DependencyDef::Id(dep_id) => dependencies.push(dep_id),
                DependencyDef::Inline(dep_def) => {
                    dependencies.push(self.register_query(*dep_def, importance)?);
                }
            }
        }

        let query = Query {
            id: id.clone(),
            description: def.description.into(),
            stability: def.stability,
            importance,
            dependencies,
            promql: template_exprs_from_value(def.promql.as_ref())?,
            datadog_sql: template_exprs_from_value(def.datadog_sql.as_ref())?,
            honeycomb_sql: template_exprs_from_value(def.honeycomb_sql.as_ref())?,
            logql: template_exprs_from_value(def.logql.as_ref())?,
            instant: def.instant,
        };
        self.queries.insert(id.clone(), query);
        Ok(id)
    }

    /// Overwrite an existing query definition (the escape hatch to
    /// [`register_query`](Self::register_query)'s unique-id rule).
    pub fn overload_query(&mut self, query: Query) {
        self.queries.insert(query.id.clone(), query);
    }

    /// Register a recording rule, promoting an inline `query` (which inherits the
    /// file `importance` hint) if present.
    pub fn register_rule(&mut self, def: RuleDef, importance: Importance) -> Result<()> {
        if self.rules.contains_key(&def.record) {
            return Err(Error::DuplicateRule(def.record));
        }
        let query_id =
            self.resolve_required_dependency(def.query, def.query_id, &def.record, importance)?;
        let rule = Rule {
            record: def.record.clone(),
            description: def.description.into(),
            group: def.group,
            stability: def.stability,
            query_id,
            labels: def.labels,
        };
        self.rules.insert(rule.record.clone(), rule);
        Ok(())
    }

    /// Register an alert, promoting an inline `query` (which inherits the file
    /// `importance` hint) if present.
    pub fn register_alert(&mut self, def: AlertDef, importance: Importance) -> Result<()> {
        if self.alerts.contains_key(&def.alert) {
            return Err(Error::DuplicateAlert(def.alert));
        }
        let query_id =
            self.resolve_required_dependency(def.query, def.query_id, &def.alert, importance)?;
        let alert = Alert {
            alert: def.alert.clone(),
            description: def.description.into(),
            group: def.group,
            stability: def.stability,
            query_id,
            for_: def.for_,
            keep_firing_for: def.keep_firing_for,
            labels: def.labels,
            annotations: def.annotations,
        };
        self.alerts.insert(alert.alert.clone(), alert);
        Ok(())
    }

    /// Resolve a rule/alert's required dependency: an inline `query` (registered
    /// and promoted) or a `queryId` reference. Exactly one must be present.
    fn resolve_required_dependency(
        &mut self,
        query: Option<Box<QueryDef>>,
        query_id: Option<String>,
        owner: &str,
        importance: Importance,
    ) -> Result<QueryId> {
        match (query, query_id) {
            (Some(def), _) => self.register_query(*def, importance),
            (None, Some(id)) => Ok(id),
            (None, None) => Err(Error::Schema {
                path: owner.to_string(),
                message: "requires exactly one of `query` or `queryId`".to_string(),
            }),
        }
    }
}
