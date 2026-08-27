// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! PromQL selector fragments the panels share.
//!
//! These exist so a panel's query never writes a dashboard variable name
//! directly. A variable rename then touches one function rather than sixty
//! expressions, and — more to the point — an undefined variable cannot creep in
//! through a typo, since the names come from
//! [`mzmon_lib::grafana::context::variables`].

use mzmon_lib::grafana::context::{self, variables};

/// Scope to the selected environment(s): `materialize_cloud_organization_name=~"$environmentNameList"`.
pub fn environment() -> String {
    context::environment_filter()
}

/// Scope to the Materialize namespaces: `namespace=~"$mzNamespaceList"`.
pub fn namespace() -> String {
    format!(r#"namespace=~"${}""#, variables::MZ_NAMESPACE_LIST)
}

/// Scope to the selected clusters: `instance_id=~"$mzClusterList"`.
pub fn cluster() -> String {
    format!(r#"instance_id=~"${}""#, variables::MZ_CLUSTER_LIST)
}

/// Scope to the selected replicas: `replica_id=~"$mzReplicaList"`.
pub fn replica() -> String {
    format!(r#"replica_id=~"${}""#, variables::MZ_REPLICA_LIST)
}

/// Scope to the selected clusters under the `compute_cluster_id` label name.
///
/// The label a cluster id arrives under varies by metric family — `instance_id`
/// on most compute metrics, `compute_cluster_id` on the SQL-derived ones, and
/// `cluster_environmentd_materialize_cloud_cluster_id` on arrangement history.
/// That is why the registry interpolates a cluster *value* and leaves the label
/// to the author; these functions are the author's side of that split.
pub fn compute_cluster() -> String {
    format!(r#"compute_cluster_id=~"${}""#, variables::MZ_CLUSTER_LIST)
}

/// Scope to the selected replicas under the `compute_replica_id` label name.
pub fn compute_replica() -> String {
    format!(r#"compute_replica_id=~"${}""#, variables::MZ_REPLICA_LIST)
}

/// Scope to the selected clusters under the replica-history label name.
///
/// The third spelling of a cluster id: arrangement and dataflow-history metrics
/// arrive from the replica's own scrape, which prefixes environmentd's labels.
pub fn history_cluster() -> String {
    format!(
        r#"cluster_environmentd_materialize_cloud_cluster_id=~"${}""#,
        variables::MZ_CLUSTER_LIST
    )
}

/// Scope to the selected replicas under the replica-history label name.
pub fn history_replica() -> String {
    format!(
        r#"cluster_environmentd_materialize_cloud_replica_id=~"${}""#,
        variables::MZ_REPLICA_LIST
    )
}

/// The label carrying a cluster id on replica-history metrics.
pub const HISTORY_CLUSTER_LABEL: &str = "cluster_environmentd_materialize_cloud_cluster_id";
/// The label carrying a replica id on replica-history metrics.
pub const HISTORY_REPLICA_LABEL: &str = "cluster_environmentd_materialize_cloud_replica_id";

/// System clusters, whose ids start with `s`.
pub const SYSTEM_CLUSTER_PATTERN: &str = "^s.*";

/// Collections Materialize maintains for itself: catalog tables, probes.
pub const SYSTEM_COLLECTION_PATTERN: &str = "s.*";
/// Collections created by the user: indexes and materialized views.
pub const USER_COLLECTION_PATTERN: &str = "u.*";
/// Short-lived intermediates, plus the sentinel for an unidentified collection.
pub const TRANSIENT_COLLECTION_PATTERN: &str = "t.*|none";

/// The sentinel a collection reports before it has an output frontier.
///
/// A not-yet-hydrated collection reports a lag far in the future rather than
/// nothing, so `> SENTINEL` counts what is hydrating and `< FINITE_LAG_CEILING`
/// excludes it from lag panels.
pub const HYDRATING_SENTINEL: &str = "1e15";
/// The ceiling separating a real lag from the hydration sentinel.
pub const FINITE_LAG_CEILING: &str = "1e9";

/// Match the pods of the *selected* cluster replicas, by name.
///
/// cAdvisor and kube-state-metrics know nothing about Materialize's catalog, so
/// the cluster and replica selection has to be applied to the pod *name* rather
/// than to a cluster-id label. Replica pods are named
/// `…-cluster-<cluster>-replica-<replica>-…`.
///
/// `${var:regex}` is Grafana's format modifier: it escapes the interpolated value
/// for use inside a regex, which a bare `$var` would not. Getting that wrong
/// gives a selector that quietly matches nothing when a value contains a regex
/// metacharacter.
pub fn replica_pods() -> String {
    format!(
        r#"pod=~".*-cluster-${{{cluster}:regex}}-replica-${{{replica}:regex}}-.*""#,
        cluster = variables::MZ_CLUSTER_LIST,
        replica = variables::MZ_REPLICA_LIST
    )
}

/// Match every pod that is *not* a cluster replica: environmentd, the balancer,
/// the exporter.
///
/// Deliberately unfiltered by the cluster selectors — those pods do not belong to
/// a cluster, so narrowing the selection should not make them vanish.
pub fn non_replica_pods() -> String {
    r#"pod!~".*-cluster-.*-replica-.*""#.to_string()
}

/// The monitoring stack's own SQL exporter.
///
/// Excluded from capacity and usage panels on the Summary tab so those read as
/// user-workload figures; the Kubernetes tab includes it, since there the
/// question is what is actually running.
pub const SQL_EXPORTER_CONTAINER: &str = "new-promsql-exporter";

/// cAdvisor container matchers, dropping the two series that are not containers.
///
/// `container=""` is the pod-level cgroup roll-up and `container="POD"` is the
/// pause container; counting either double-counts the pod.
pub fn containers() -> String {
    format!(r#"{},container!="",container!="POD""#, namespace())
}

/// [`containers`], also dropping the monitoring exporter.
pub fn workload_containers() -> String {
    format!(r#"{}, container!="{SQL_EXPORTER_CONTAINER}""#, containers())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mzmon_lib::grafana::context::REQUIRED_VARIABLES;

    #[test]
    fn every_selector_names_a_required_variable() {
        for (what, fragment) in [
            ("environment", environment()),
            ("namespace", namespace()),
            ("cluster", cluster()),
            ("replica", replica()),
        ] {
            let referenced: Vec<&str> = REQUIRED_VARIABLES
                .iter()
                .copied()
                .filter(|v| fragment.contains(&format!("${v}")))
                .collect();
            assert_eq!(
                referenced.len(),
                1,
                "{what} should reference exactly one required variable, got {referenced:?}"
            );
        }
    }

    #[test]
    fn the_container_matchers_drop_both_non_containers() {
        let c = containers();
        assert!(c.contains(r#"container!="""#), "{c}");
        assert!(c.contains(r#"container!="POD""#), "{c}");
        assert!(c.contains("$mzNamespaceList"), "{c}");
    }

    #[test]
    fn the_pod_split_is_exhaustive_and_disjoint() {
        // Every pod is either a cluster replica or not, and the two matchers are
        // the same pattern under `=~` and `!~` -- so a pod cannot fall through the
        // gap or be counted twice.
        let selected = replica_pods();
        let rest = non_replica_pods();
        assert!(selected.starts_with("pod=~"), "{selected}");
        assert!(rest.starts_with("pod!~"), "{rest}");
        assert!(selected.contains("-cluster-") && selected.contains("-replica-"));
        assert!(rest.contains("-cluster-") && rest.contains("-replica-"));
    }

    #[test]
    fn the_replica_pod_pattern_escapes_its_interpolations() {
        // A bare `$var` inside a regex breaks on any metacharacter in the value;
        // `${var:regex}` is what makes it safe.
        let pattern = replica_pods();
        assert!(pattern.contains("${mzClusterList:regex}"), "{pattern}");
        assert!(pattern.contains("${mzReplicaList:regex}"), "{pattern}");
    }

    #[test]
    fn workload_containers_also_drops_the_exporter() {
        let w = workload_containers();
        assert!(w.contains(SQL_EXPORTER_CONTAINER), "{w}");
        assert!(!containers().contains(SQL_EXPORTER_CONTAINER));
    }
}
