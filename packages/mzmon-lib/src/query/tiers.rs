// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The `metric-tiers.yaml` projection: registry metrics grouped by importance.
//!
//! This is the artifact the Helm gateway consumes to build per-destination
//! allowlists (`mzmon.alloyGateway.metricFilter`). Each importance level lists
//! the metrics **at exactly that level** (disjoint) — the chart does the
//! cumulative "this tier and more important" union, and treats the `all`
//! destination tier as `.*` without consulting this file.
//!
//! Names are emitted as **anchored-regex fragments**, not plain strings: a
//! SQL-exporter metric authored as `%%{mzSqlPrefix}foo` is rendered through
//! [`tier_context`](crate::query::render::tier_context) with a sentinel prefix
//! and rewritten here to `(?:v2_)?mz_foo`, so it matches under either the
//! converged (`mz_`) or legacy (`v2_mz_`) prefix. Literal `mz_…` and infra
//! (`kube_…`, `container_…`, `up`) names pass through verbatim (and remain valid
//! one-element alternations).

use serde::Serialize;

use crate::query::docgen::MetricDoc;
use crate::query::error::Result;
use crate::query::importance::Importance;
use crate::query::render::{SQL_PREFIX_REGEX, SQL_PREFIX_SENTINEL};

/// Metric names grouped by importance. Field order (most → least important)
/// is the serialized key order.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct MetricTiers {
    pub essential: Vec<String>,
    pub recommended: Vec<String>,
    pub extended: Vec<String>,
    pub diagnostic: Vec<String>,
}

impl MetricTiers {
    /// Group the aggregated metric docs by their resolved importance, rewriting
    /// the SQL-prefix sentinel to its prefix-agnostic regex. Names within each
    /// tier are sorted for deterministic output.
    pub fn from_docs(docs: &[MetricDoc]) -> Self {
        let mut tiers = MetricTiers::default();
        for doc in docs {
            let name = rewrite_sql_prefix(&doc.name);
            // `doc.importance` is always one of our four kebab values.
            match doc.importance.parse::<Importance>() {
                Ok(Importance::Essential) => tiers.essential.push(name),
                Ok(Importance::Recommended) => tiers.recommended.push(name),
                Ok(Importance::Extended) => tiers.extended.push(name),
                Ok(Importance::Diagnostic) => tiers.diagnostic.push(name),
                Err(_) => {}
            }
        }
        for bucket in [
            &mut tiers.essential,
            &mut tiers.recommended,
            &mut tiers.extended,
            &mut tiers.diagnostic,
        ] {
            bucket.sort();
            bucket.dedup();
        }
        tiers
    }

    /// Total number of metrics across all tiers.
    pub fn len(&self) -> usize {
        self.essential.len() + self.recommended.len() + self.extended.len() + self.diagnostic.len()
    }

    /// True if no metrics are present in any tier.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Serialize to the `metric-tiers.yaml` body (no header comment).
    pub fn to_yaml(&self) -> Result<String> {
        Ok(serde_yaml_ng::to_string(self)?)
    }
}

/// Rewrite a leading [`SQL_PREFIX_SENTINEL`] to the prefix-agnostic
/// [`SQL_PREFIX_REGEX`]; other names pass through unchanged.
fn rewrite_sql_prefix(name: &str) -> String {
    match name.strip_prefix(SQL_PREFIX_SENTINEL) {
        Some(rest) => format!("{SQL_PREFIX_REGEX}{rest}"),
        None => name.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn doc(name: &str, importance: &str) -> MetricDoc {
        MetricDoc {
            importance: importance.to_string(),
            labels: vec![],
            name: name.to_string(),
            usage: vec![],
        }
    }

    #[test]
    fn groups_by_importance_disjoint_and_sorted() {
        let tiers = MetricTiers::from_docs(&[
            doc("mz_b", "essential"),
            doc("mz_a", "essential"),
            doc("mz_c", "recommended"),
            doc("kube_pod_info", "extended"),
        ]);
        assert_eq!(tiers.essential, vec!["mz_a", "mz_b"]);
        assert_eq!(tiers.recommended, vec!["mz_c"]);
        assert_eq!(tiers.extended, vec!["kube_pod_info"]);
        assert!(tiers.diagnostic.is_empty());
        assert_eq!(tiers.len(), 4);
    }

    #[test]
    fn sql_prefix_sentinel_becomes_prefix_agnostic() {
        let sentinel_name = format!("{SQL_PREFIX_SENTINEL}compute_cluster_status");
        let tiers = MetricTiers::from_docs(&[doc(&sentinel_name, "essential")]);
        assert_eq!(
            tiers.essential,
            vec!["(?:v2_)?mz_compute_cluster_status".to_string()]
        );
    }

    #[test]
    fn literal_and_infra_names_pass_through() {
        let tiers = MetricTiers::from_docs(&[
            doc("mz_dataflow_wallclock_lag_seconds", "recommended"),
            doc("container_cpu_usage_seconds_total", "essential"),
            doc("up", "essential"),
        ]);
        assert_eq!(
            tiers.recommended,
            vec!["mz_dataflow_wallclock_lag_seconds".to_string()]
        );
        assert_eq!(
            tiers.essential,
            vec![
                "container_cpu_usage_seconds_total".to_string(),
                "up".to_string()
            ]
        );
    }

    #[test]
    fn serializes_all_four_keys_in_importance_order() {
        let yaml = MetricTiers::default().to_yaml().unwrap();
        let essential = yaml.find("essential").unwrap();
        let recommended = yaml.find("recommended").unwrap();
        let extended = yaml.find("extended").unwrap();
        let diagnostic = yaml.find("diagnostic").unwrap();
        assert!(essential < recommended && recommended < extended && extended < diagnostic);
    }
}
