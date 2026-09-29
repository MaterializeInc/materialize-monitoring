// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The alerting [`TemplateContext`]: how a registry query reads as a rule.
//!
//! A dashboard renders a query against Grafana variables the viewer picks, and
//! the extraction contexts render sentinels nobody evaluates. A rule is evaluated
//! unattended by a ruler that knows nothing of either, so this context differs in
//! two ways.
//!
//! **Deployment-specific values are placeholders.** Rules are rendered once, at
//! build time, into `pre-rendered/rules/`, but which namespaces hold a
//! Materialize environment, and whether SQL-backed metrics carry Cloud's `v2_`
//! prefix, are facts about one install. So those parameters render to a
//! [`Placeholder`] token that the chart replaces from values at install time.
//! Every token is a valid PromQL identifier or string in the position it
//! occupies, which keeps the pre-rendered files checkable by `promtool` as they
//! stand.
//!
//! **Selection parameters are absent.** `interval`, `range` and the log and
//! generation pickers mean "whatever the viewer chose", and a rule has no viewer.
//! Leaving them out makes a query that uses one fail to render with
//! [`MissingParameter`](crate::query::Error::MissingParameter), rather than
//! silently baking in a window nobody chose. That is the alerting design doc's
//! "range trap", enforced.

use std::collections::HashMap;

use crate::query::enrich;
use crate::query::model::QueryEngine;
use crate::query::registry::QueryRegistry;
use crate::query::render::{TemplateContext, TemplateFn, promql_or_zero};

/// A token the chart replaces with a deployment's own value at install time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Placeholder {
    /// `mz_` on self-managed, `v2_mz_` where `materialize.deploymentMode` is
    /// `cloud`. Occupies a metric-name prefix position.
    SqlPrefix,
    /// A regex alternation of the Materialize environment namespaces.
    EnvironmentNamespaces,
    /// A regex alternation of the Materialize operator's namespaces.
    OperatorNamespaces,
    /// A regex alternation of the namespaces a deployment excludes from alerting.
    ExcludedNamespaces,
}

impl Placeholder {
    /// Every placeholder, longest token first — the order the chart replaces
    /// them in, so no token is ever a prefix of one replaced before it.
    pub const ALL: &'static [Placeholder] = &[
        Placeholder::EnvironmentNamespaces,
        Placeholder::ExcludedNamespaces,
        Placeholder::OperatorNamespaces,
        Placeholder::SqlPrefix,
    ];

    /// The literal token written into a rendered rule.
    ///
    /// Distinct from metric-tiers' `__mz_sql_prefix__`
    /// ([`SQL_PREFIX_SENTINEL`](crate::query::render::SQL_PREFIX_SENTINEL)),
    /// which is rewritten into a regex rather than replaced at install time; a
    /// rule carrying that one is an error.
    pub fn token(self) -> &'static str {
        match self {
            Placeholder::SqlPrefix => "__mzmon_sql_prefix__",
            Placeholder::EnvironmentNamespaces => "__mzmon_environment_namespaces__",
            Placeholder::OperatorNamespaces => "__mzmon_operator_namespaces__",
            Placeholder::ExcludedNamespaces => "__mzmon_excluded_namespaces__",
        }
    }

    /// The token for `name`, if it is one.
    pub fn from_token(token: &str) -> Option<Placeholder> {
        Placeholder::ALL
            .iter()
            .copied()
            .find(|p| p.token() == token)
    }
}

/// The scrape jobs whose `up` series carry both `namespace` and
/// `materialize_cloud_organization_name`: the environmentd and clusterd
/// PodMonitors copy `materialize.cloud/organization-name` from the pod
/// (`packages/prometheus-scrapers/podmonitor-environmentd.yaml`). Job names are
/// `<monitor namespace>/<release>-materialize-<component>`, so only the suffix is
/// stable. Anchored, so the `-environmentd-sql` job is excluded.
pub const ENVIRONMENT_INFO_JOBS: &str = ".*/.*materialize-(environmentd|clusterd)";

/// Build the alerting [`TemplateContext`].
///
/// `identity_functions` renders every enrichment function as the identity. The
/// renderer uses that form to infer a rule's capabilities, so the join
/// [`mzEnvironmentName`](enrich::with_environment_name) adds — which reads `up`
/// on its own account — is not mistaken for something the rule itself needs.
pub fn alerting_context(
    registry: &QueryRegistry,
    engine: QueryEngine,
    identity_functions: bool,
) -> TemplateContext<'_> {
    let env = Placeholder::EnvironmentNamespaces.token();
    let operator = Placeholder::OperatorNamespaces.token();
    let excluded = Placeholder::ExcludedNamespaces.token();

    let environment_filter = format!(r#"namespace=~"{env}""#);
    let parameters = [
        ("mzSqlPrefix", Placeholder::SqlPrefix.token().to_string()),
        ("mzEnvironmentNamespaceFilter", environment_filter.clone()),
        // A bare value, not a matcher: queries write `namespace=~"%%{mzNamespaceList}"`.
        ("mzNamespaceList", env.to_string()),
        ("mzOperatorNamespaceFilter", format!(r#"namespace=~"{operator}""#)),
        (
            "mzDeploymentNamespaceFilter",
            format!(r#"namespace=~"{operator}|{env}""#),
        ),
        (
            "excludeMzDeploymentNamespaceFilter",
            format!(r#"namespace!~"{operator}|{env}""#),
        ),
        ("mzSystemNamespaceFilter", r#"namespace=~"kube-system""#.to_string()),
        // Cloud excludes its test organizations here. Self-managed has no org
        // taxonomy to key on, so the discriminator is the namespace; the chart
        // renders a regex that matches nothing when none are excluded, never an
        // empty string, which `!~` would read as "drop every series without a
        // namespace".
        (
            "excludeEnvironmentFilter",
            format!(r#"namespace!~"{excluded}""#),
        ),
        // Every environment. Only Materialize's own series carry the label, and
        // `.+` keeps it a real matcher rather than one that matches the empty
        // string.
        (
            "mzEnvironmentFilter",
            r#"materialize_cloud_organization_name=~".+""#.to_string(),
        ),
        (
            "cAdvisorFilter",
            format!(r#"{environment_filter},container!="",container!="POD""#),
        ),
        // A rule has no cluster or replica picker; it covers every one.
        ("mzClusterList", ".+".to_string()),
        ("mzReplicaList", ".+".to_string()),
        ("mzClusterListRegex", ".+".to_string()),
        ("mzReplicaListRegex", ".+".to_string()),
        (
            "excludeHostNetworkPods",
            r#"unless on (namespace, pod) count by (namespace, pod) (container_network_receive_bytes_total{interface!~"eth0|lo"})"#
                .to_string(),
        ),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();

    let mut functions: HashMap<String, TemplateFn> = HashMap::new();
    functions.insert("orZero".to_string(), Box::new(promql_or_zero));
    if identity_functions {
        functions.insert(
            "mzEnvironmentName".to_string(),
            Box::new(|base: &str, _args: &[String]| base.to_string()),
        );
    } else {
        functions.insert(
            "mzEnvironmentName".to_string(),
            Box::new(|base: &str, args: &[String]| {
                let key = args.first().map(String::as_str).unwrap_or("namespace");
                enrich::with_environment_name(base, key, ENVIRONMENT_INFO_JOBS)
            }),
        );
    }
    // `mzClusterName` / `mzObjectName` are deliberately absent. Their joins key on
    // a catalog id, which is unique only within one environment, and a rule has
    // no environment picker to scope them with — several environments would make
    // the join many-to-many and stop the rule evaluating. A rule using one fails
    // to render instead.

    TemplateContext {
        engine,
        parameters,
        functions,
        registry: Some(registry),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::model::TemplateExpr;
    use crate::query::{Description, Error, Importance, Query, Stability};

    fn query(template: &str) -> Query {
        Query {
            id: "q".to_string(),
            description: Description::default(),
            stability: Stability::BestEffort,
            importance: Importance::Essential,
            dependencies: vec![],
            promql: vec![TemplateExpr::template(template)],
            datadog_query: vec![],
            honeycomb_sql: vec![],
            logql: vec![],
            instant: None,
        }
    }

    #[test]
    fn deployment_values_render_as_placeholders() {
        let registry = QueryRegistry::new();
        let ctx = alerting_context(&registry, QueryEngine::PromQl, false);
        let rendered = query(
            "%%{mzSqlPrefix}compute_cluster_status{%%{mzEnvironmentNamespaceFilter}, %%{excludeEnvironmentFilter}}",
        )
        .render(&ctx)
        .unwrap();
        assert_eq!(
            rendered,
            vec![
                r#"__mzmon_sql_prefix__compute_cluster_status{namespace=~"__mzmon_environment_namespaces__", namespace!~"__mzmon_excluded_namespaces__"}"#
            ]
        );
    }

    #[test]
    fn selection_parameters_are_refused() {
        let registry = QueryRegistry::new();
        let ctx = alerting_context(&registry, QueryEngine::PromQl, false);
        for param in ["interval", "range", "rangeWindow", "mzLogNamespaceFilter"] {
            let err = query(&format!("rate(m{{}}%%{{{param}}})"))
                .render(&ctx)
                .unwrap_err();
            assert!(
                matches!(err, Error::MissingParameter { ref name, .. } if name == param),
                "{param}: {err:?}"
            );
        }
    }

    #[test]
    fn catalog_joins_are_refused() {
        let registry = QueryRegistry::new();
        let ctx = alerting_context(&registry, QueryEngine::PromQl, false);
        let mut q = query("m{}");
        q.promql[0].functions.push(crate::query::TemplateFunction {
            name: "mzObjectName".to_string(),
            args: vec![],
        });
        assert!(matches!(
            q.render(&ctx).unwrap_err(),
            Error::UnknownFunction { .. }
        ));
    }

    #[test]
    fn placeholders_are_not_prefixes_of_each_other() {
        // The chart replaces them in `ALL` order on the raw file text; a token
        // that were a prefix of a later one would corrupt it.
        for (i, a) in Placeholder::ALL.iter().enumerate() {
            for b in &Placeholder::ALL[i + 1..] {
                assert!(!b.token().contains(a.token()), "{a:?} inside {b:?}");
                assert!(!a.token().contains(b.token()), "{b:?} inside {a:?}");
            }
        }
        assert!(
            !Placeholder::SqlPrefix
                .token()
                .contains(crate::query::render::SQL_PREFIX_SENTINEL)
        );
    }
}
