// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Catalog-name enrichment for id-keyed metrics. Port of `py_mzmon_lib.enrich`.
//!
//! Most data-plane metrics carry only an id (`source_id`, `sink_id`,
//! `collection_id`, `parent_source_id`, …). These helpers attach the friendly
//! catalog `name` by joining against `mz_object_info` / `mz_cluster_info`, which
//! is what removes the "metrics only have ids; look the name up in SQL"
//! workaround.
//!
//! This lives beside the renderer rather than in [`crate::grafana`] because the
//! joins are plain PromQL: the query registry's template functions and the
//! dashboards share one implementation, and the environment scope is passed in
//! rather than imported, so nothing here depends on dashboard variables.
//!
//! Output is kept byte-identical to the Python, including its newline placement
//! — the two implementations render the same registry, and a formatting
//! difference would show up as spurious churn in every rendered expression.

/// Canonical id -> name catalog metric. Not SQL-prefixed (genuine in both
/// self-managed and cloud, value `1`).
pub const OBJECT_INFO: &str = "mz_object_info";

/// Cluster id -> name catalog metric.
pub const CLUSTER_INFO: &str = "mz_cluster_info";

/// Left-join `pulled` labels from `info_expr` onto `value_expr` on `id_label`.
///
/// `info_expr` is a `1`-valued info metric already `label_replace`d so it exposes
/// `id_label` (matching the value side) plus the `pulled` labels.
///
/// Robust to the two failure modes of a naive `* on(id) group_left(name)`:
///
/// 1. **Duplicate info series.** Two environmentd generations during a rolling
///    restart, or one `/metrics` endpoint scraped by several Prometheus jobs,
///    make the right-hand side non-unique per `id_label` — `group_left` then
///    errors with "many-to-many matching not allowed". Collapsing the info metric
///    with `max by (…)` drops the `job`/`instance`/`pod` identity labels so the
///    duplicate generations merge.
/// 2. **Missing info.** Over a range before the info metric was scraped, or for
///    an object dropped since, an inner join would drop the value series
///    entirely. Those rows are unioned back with `… or (value unless on(id)
///    info_keys)` and the raw id is `label_replace`d into the first pulled label
///    so the legend reads the id rather than going blank. The matched set and the
///    `unless` set are disjoint on `id_label`, so the union never double-counts.
///
/// Panics if `pulled` is empty; every caller passes at least one label.
fn left_join_labels(value_expr: &str, id_label: &str, info_expr: &str, pulled: &[&str]) -> String {
    let primary = pulled.first().expect("at least one pulled label");
    let keep = std::iter::once(id_label)
        .chain(pulled.iter().copied())
        .collect::<Vec<_>>()
        .join(", ");
    let pulled_list = pulled.join(", ");
    // One info series per (id + pulled labels): drops job/instance/pod so
    // concurrent envd generations collapse to a single row.
    let dedup = format!("max by ({keep}) (\n{info_expr}\n)");
    // One series per id that *has* catalog info -- the set excluded from the
    // fallback so matched rows are not duplicated.
    let keys = format!("group by ({id_label}) (\n{info_expr}\n)");
    format!(
        "(\n\
         (\n{value_expr}\n)\n\
         * on ({id_label}) group_left({pulled_list})\n\
         {dedup}\n\
         )\n\
         or\n\
         label_replace(\n\
         (\n{value_expr}\n)\n\
         unless on ({id_label}) (\n{keys}\n),\n\
         \"{primary}\", \"$1\", \"{id_label}\", \"(.*)\"\n\
         )"
    )
}

/// Attach catalog `name` to `value_expr` via `mz_object_info`.
///
/// `value_expr` must expose `id_label` (`source_id`, `sink_id`, `collection_id`,
/// `parent_source_id`, …). Those are **global** ids, so the join is on
/// `mz_object_info`'s `global_id` label, `label_replace`d onto `id_label`.
///
/// **Join on `global_id`, not `object_id`.** `object_id` is the catalog item id
/// and is *not unique* in `mz_object_info` — one object (a materialized view,
/// say) can own several collections, appearing once per `global_id` with the same
/// `object_id`. Joining on `object_id` yields multiple right-hand matches even in
/// steady state. In a trivial environment where every object has one collection
/// the two are equal, which is why this only surfaces against real workloads.
///
/// `extra` brings additional labels across (e.g. `type`). `env_filter` scopes the
/// info metric to the selected environment(s): ids are only unique within an
/// environment, so an unscoped join would match the same id across orgs in
/// multi-tenant cloud.
pub fn with_object_name(
    value_expr: &str,
    id_label: &str,
    extra: Option<&str>,
    env_filter: &str,
) -> String {
    let info = format!(
        "label_replace({OBJECT_INFO}{{{env_filter}}}, \"{id_label}\", \"$1\", \"global_id\", \"(.*)\")"
    );
    match extra {
        Some(extra) => left_join_labels(value_expr, id_label, &info, &["name", extra]),
        None => left_join_labels(value_expr, id_label, &info, &["name"]),
    }
}

/// Attach `cluster_name` from `mz_cluster_info` (keyed on `cluster_id`).
///
/// For panels legending on a cluster id under any label name — `instance_id` for
/// most compute metrics, `cluster_environmentd_materialize_cloud_cluster_id` for
/// arrangement / dataflow history — pass that label as `id_label`. The pulled
/// name is renamed to `cluster_name` (not `name`) so it does not collide when
/// composed with [`with_object_name`].
///
/// Replica names are deliberately not joined: they are all `r1`, so the more
/// informative `replica_id` is what belongs in a legend.
pub fn with_cluster_name(value_expr: &str, id_label: &str, env_filter: &str) -> String {
    let info = format!(
        "label_replace({CLUSTER_INFO}{{{env_filter}}}, \"{id_label}\", \"$1\", \"cluster_id\", \"(.*)\")"
    );
    let info = format!("label_replace({info}, \"cluster_name\", \"$1\", \"name\", \"(.*)\")");
    left_join_labels(value_expr, id_label, &info, &["cluster_name"])
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENV: &str = r#"materialize_cloud_organization_name=~"$environmentNameList""#;

    /// Captured from `py_mzmon_lib.enrich.with_cluster_name("VALUE",
    /// "instance_id", env_filter=ENV)`. Byte-identical output is the contract:
    /// both implementations render the same registry.
    const PY_CLUSTER_NAME: &str = r#"(
(
VALUE
)
* on (instance_id) group_left(cluster_name)
max by (instance_id, cluster_name) (
label_replace(label_replace(mz_cluster_info{materialize_cloud_organization_name=~"$environmentNameList"}, "instance_id", "$1", "cluster_id", "(.*)"), "cluster_name", "$1", "name", "(.*)")
)
)
or
label_replace(
(
VALUE
)
unless on (instance_id) (
group by (instance_id) (
label_replace(label_replace(mz_cluster_info{materialize_cloud_organization_name=~"$environmentNameList"}, "instance_id", "$1", "cluster_id", "(.*)"), "cluster_name", "$1", "name", "(.*)")
)
),
"cluster_name", "$1", "instance_id", "(.*)"
)"#;

    /// Captured from `py_mzmon_lib.enrich.with_object_name("VALUE", "sink_id",
    /// env_filter=ENV)`.
    const PY_OBJECT_NAME: &str = r#"(
(
VALUE
)
* on (sink_id) group_left(name)
max by (sink_id, name) (
label_replace(mz_object_info{materialize_cloud_organization_name=~"$environmentNameList"}, "sink_id", "$1", "global_id", "(.*)")
)
)
or
label_replace(
(
VALUE
)
unless on (sink_id) (
group by (sink_id) (
label_replace(mz_object_info{materialize_cloud_organization_name=~"$environmentNameList"}, "sink_id", "$1", "global_id", "(.*)")
)
),
"name", "$1", "sink_id", "(.*)"
)"#;

    /// Captured from `py_mzmon_lib.enrich.with_object_name("VALUE", "id",
    /// extra="type", env_filter=ENV)`.
    const PY_OBJECT_NAME_EXTRA: &str = r#"(
(
VALUE
)
* on (id) group_left(name, type)
max by (id, name, type) (
label_replace(mz_object_info{materialize_cloud_organization_name=~"$environmentNameList"}, "id", "$1", "global_id", "(.*)")
)
)
or
label_replace(
(
VALUE
)
unless on (id) (
group by (id) (
label_replace(mz_object_info{materialize_cloud_organization_name=~"$environmentNameList"}, "id", "$1", "global_id", "(.*)")
)
),
"name", "$1", "id", "(.*)"
)"#;

    #[test]
    fn cluster_name_matches_the_python() {
        assert_eq!(
            with_cluster_name("VALUE", "instance_id", ENV),
            PY_CLUSTER_NAME
        );
    }

    #[test]
    fn object_name_matches_the_python() {
        assert_eq!(
            with_object_name("VALUE", "sink_id", None, ENV),
            PY_OBJECT_NAME
        );
    }

    #[test]
    fn object_name_with_an_extra_label_matches_the_python() {
        assert_eq!(
            with_object_name("VALUE", "id", Some("type"), ENV),
            PY_OBJECT_NAME_EXTRA
        );
    }

    #[test]
    fn the_join_stays_parseable_promql() {
        // The whole point of the enrichment is a query Prometheus accepts; a
        // formatting slip that produced invalid PromQL would otherwise only show
        // up in a browser.
        let expr = with_cluster_name("up", "instance_id", ENV);
        promql_parser::parser::parse(&expr).expect("enriched expression must parse");

        let expr = with_object_name("up", "sink_id", Some("type"), ENV);
        promql_parser::parser::parse(&expr).expect("enriched expression must parse");
    }

    #[test]
    fn composing_both_joins_parses() {
        // Most-Lagged Collections applies object *and* cluster enrichment, which
        // is why cluster_name is not just called `name`.
        let expr = with_object_name("up", "collection_id", None, ENV);
        let expr = with_cluster_name(&expr, "instance_id", ENV);
        promql_parser::parser::parse(&expr).expect("composed enrichment must parse");
        assert!(expr.contains("cluster_name"));
        assert!(expr.contains("group_left(name)"));
    }
}
