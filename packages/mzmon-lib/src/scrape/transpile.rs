// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Parse a prometheus-operator Monitor and convert it to a consumer format.
//!
//! [`Monitor::from_yaml_str`] parses + validates an input (kind auto-detected).
//! From there:
//! * [`Monitor::transpile`] → classic Prometheus `scrape_configs` jobs, and
//!   [`ScrapeConfigDocument::from_monitors`] assembles the combined document;
//! * [`Monitor::to_gmp`] → a GMP `PodMonitoring` / `ClusterPodMonitoring`
//!   (`None` for kinds with no GMP equivalent).
//!
//! Tests at the bottom of this file pin the per-kind output shapes (compared
//! structurally, so YAML key order / quoting don't matter — but relabel order
//! does).

use serde_json::Value;

use crate::scrape::classic::config::RelabelConfig as ClassicRelabelConfig;
use crate::scrape::classic::config::{
    BasicAuth as ClassicBasicAuth, GlobalConfig, KubernetesSdConfig, Namespaces,
    ScrapeConfigDocument, ScrapeJob,
};
use crate::scrape::error::{Error, Result};
use crate::scrape::gmp::config as gmp;
use crate::scrape::operator::common::{
    BasicAuth as OperatorBasicAuth, NamespaceSelector, ObjectMeta,
    RelabelConfig as OperatorRelabelConfig,
};
use crate::scrape::operator::pod_monitor::PodMonitor;
use crate::scrape::operator::scrape_config::ScrapeConfigCrd;
use crate::scrape::operator::service_monitor::ServiceMonitor;
use crate::scrape::validate::{self, MonitorKind};

/// A parsed prometheus-operator Monitor of one of the supported kinds.
#[derive(Debug, Clone)]
pub enum Monitor {
    PodMonitor(PodMonitor),
    ServiceMonitor(ServiceMonitor),
    ScrapeConfig(ScrapeConfigCrd),
}

impl Monitor {
    /// Parse a Monitor YAML document: route on `kind`, validate against the
    /// upstream CRD schema, then deserialize into the typed subset.
    ///
    /// Mirrors `alloy::pipeline::Pipeline::from_yaml_str`.
    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        let value: Value = serde_yaml_ng::from_str(yaml)?;
        let kind = value
            .get("kind")
            .and_then(Value::as_str)
            .map(str::to_string);

        match kind.as_deref() {
            Some("PodMonitor") => {
                validate::validate(MonitorKind::PodMonitor, &value)?;
                Ok(Monitor::PodMonitor(serde_json::from_value(value)?))
            }
            Some("ServiceMonitor") => {
                validate::validate(MonitorKind::ServiceMonitor, &value)?;
                Ok(Monitor::ServiceMonitor(serde_json::from_value(value)?))
            }
            Some("ScrapeConfig") => {
                validate::validate(MonitorKind::ScrapeConfig, &value)?;
                Ok(Monitor::ScrapeConfig(serde_json::from_value(value)?))
            }
            _ => Err(Error::UnknownKind {
                api_version: value
                    .get("apiVersion")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                kind,
            }),
        }
    }

    /// Transpile this Monitor into one classic scrape job per endpoint.
    pub fn transpile(&self) -> Result<Vec<ScrapeJob>> {
        match self {
            Monitor::PodMonitor(pm) => transpile_pod_monitor(pm),
            Monitor::ServiceMonitor(sm) => transpile_service_monitor(sm),
            Monitor::ScrapeConfig(sc) => transpile_scrape_config(sc),
        }
    }

    /// Convert this Monitor to GMP `PodMonitoring` / `ClusterPodMonitoring`
    /// resources.
    ///
    /// Returns one resource **per endpoint**: GMP's `unique-ports` admission
    /// policy forbids two endpoints sharing a port within one resource, and
    /// Materialize commonly scrapes several paths on the same port. A
    /// single-endpoint PodMonitor keeps the base name; a multi-endpoint one fans
    /// out to `<name>-<endpoint-suffix>`.
    ///
    /// Returns an empty `Vec` for kinds with no GMP equivalent — GMP scrapes
    /// pods only, so `ServiceMonitor` (service-based) and `ScrapeConfig`
    /// (node/static SD) are skipped. Best-effort: a PodMonitor's target
    /// `relabelings` are dropped (GMP has no target-relabeling surface) and a
    /// cluster-wide `namespaceSelector` becomes a `ClusterPodMonitoring`.
    pub fn to_gmp(&self) -> Result<Vec<gmp::PodMonitoring>> {
        match self {
            Monitor::PodMonitor(pm) => pod_monitor_to_gmp(pm),
            Monitor::ServiceMonitor(_) | Monitor::ScrapeConfig(_) => Ok(Vec::new()),
        }
    }
}

/// GMP requires a scrape `interval` on every endpoint; prometheus-operator
/// leaves it optional (inheriting `global`). Fill this when the source omits it.
const GMP_DEFAULT_INTERVAL: &str = "60s";

const BASIC_AUTH_USERNAME: &str = "mz_support";

/// Map an operator `basicAuth` onto the classic `basic_auth` block
fn basic_auth_to_classic(_auth: &OperatorBasicAuth) -> ClassicBasicAuth {
    ClassicBasicAuth {
        username: Some(BASIC_AUTH_USERNAME.to_string()),
    }
}

/// Map an operator `basicAuth` onto the GMP `basicAuth` block
fn basic_auth_to_gmp(_auth: &OperatorBasicAuth) -> gmp::BasicAuth {
    gmp::BasicAuth {
        username: Some(BASIC_AUTH_USERNAME.to_string()),
    }
}

/// PodMonitor → one GMP `PodMonitoring` (namespaced) or `ClusterPodMonitoring`
/// (cluster-wide) **per endpoint** (GMP requires unique ports within a
/// resource). See [`Monitor::to_gmp`] for the naming and lossiness contract.
fn pod_monitor_to_gmp(pod_monitor: &PodMonitor) -> Result<Vec<gmp::PodMonitoring>> {
    let selector = &pod_monitor.spec.selector;
    if selector.match_labels.is_empty() && selector.match_expressions.is_empty() {
        return Err(Error::Unsupported(
            "PodMonitor selector is empty (would select all pods)".into(),
        ));
    }

    // Scope: a cluster-wide namespaceSelector (`any: true`, or one naming
    // namespaces other than the resource's own) maps to ClusterPodMonitoring.
    // GMP's cluster variant can't restrict to specific namespaces, so explicit
    // `matchNames` are dropped (lossy, best-effort).
    let cluster_scoped = matches!(
        &pod_monitor.spec.namespace_selector,
        Some(ns) if ns.any == Some(true) || !ns.match_names.is_empty()
    );
    let (kind, namespace) = if cluster_scoped {
        ("ClusterPodMonitoring".to_string(), None)
    } else {
        (
            "PodMonitoring".to_string(),
            pod_monitor.metadata.namespace.clone(),
        )
    };

    let base_name = pod_monitor
        .metadata
        .name
        .clone()
        .unwrap_or_else(|| "default".into());

    // `podTargetLabels` → `targetLabels.fromPod`. `to` is the sanitized label
    // name (GMP requires a valid Prometheus label there); `metadata` is left to
    // the GMP default by omitting it.
    let target_labels = if pod_monitor.spec.pod_target_labels.is_empty() {
        None
    } else {
        Some(gmp::TargetLabels {
            from_pod: pod_monitor
                .spec
                .pod_target_labels
                .iter()
                .map(|label| gmp::LabelMapping {
                    from: label.clone(),
                    to: Some(sanitize_label_name(label)),
                })
                .collect(),
        })
    };

    let endpoints = &pod_monitor.spec.pod_metrics_endpoints;
    let multi = endpoints.len() > 1;
    let suffixes = disambiguate_suffixes(
        endpoints
            .iter()
            .enumerate()
            .map(|(i, e)| endpoint_suffix(e.path.as_deref(), e.port.as_deref(), e.port_number, i))
            .collect(),
    );

    endpoints
        .iter()
        .enumerate()
        .map(|(idx, endpoint)| {
            let port = match (&endpoint.port, endpoint.port_number) {
                (Some(name), None) => gmp::IntOrString::Str(name.clone()),
                (None, Some(number)) => gmp::IntOrString::Int(number as i64),
                (Some(_), Some(_)) => {
                    return Err(Error::Unsupported(
                        "endpoint cannot specify both port and portNumber".into(),
                    ));
                }
                (None, None) => {
                    return Err(Error::Unsupported(
                        "endpoint must specify either port or portNumber".into(),
                    ));
                }
            };
            // Single-endpoint PodMonitors keep the base name; multi-endpoint ones
            // fan out to `<name>-<suffix>` so each resource has one (unique) port.
            let name = if multi {
                format!("{base_name}-{}", suffixes[idx])
            } else {
                base_name.clone()
            };
            Ok(gmp::PodMonitoring {
                api_version: gmp::API_VERSION.to_string(),
                kind: kind.clone(),
                metadata: ObjectMeta {
                    name: Some(name),
                    namespace: namespace.clone(),
                    labels: pod_monitor.metadata.labels.clone(),
                },
                spec: gmp::PodMonitoringSpec {
                    selector: selector.clone(),
                    endpoints: vec![gmp::ScrapeEndpoint {
                        port,
                        interval: endpoint
                            .interval
                            .clone()
                            .unwrap_or_else(|| GMP_DEFAULT_INTERVAL.to_string()),
                        path: endpoint.path.clone(),
                        scheme: endpoint.scheme.clone(),
                        timeout: endpoint.scrape_timeout.clone(),
                        basic_auth: endpoint.basic_auth.as_ref().map(basic_auth_to_gmp),
                        // operator `relabelings` (target relabeling) have no GMP
                        // equivalent and are dropped; only metric relabeling carries over.
                        metric_relabeling: endpoint.metric_relabelings.clone(),
                    }],
                    target_labels: target_labels.clone(),
                },
            })
        })
        .collect()
}

impl OperatorRelabelConfig {
    pub fn to_classic_relabel_config(&self) -> ClassicRelabelConfig {
        ClassicRelabelConfig {
            source_labels: self.source_labels.clone(),
            target_label: self.target_label.clone(),
            regex: self.regex.clone(),
            replacement: self.replacement.clone(),
            action: self.action.clone(),
            modulus: self.modulus,
            separator: self.separator.clone(),
        }
    }
}

impl NamespaceSelector {
    /// Convert NamespaceSelector to classic Namespaces within KubernetesSdConfig
    // NB: Result<> is probably YAGNI but we may reject any+matchNames later
    pub fn to_classic_sd_namespaces(&self) -> Result<Option<Namespaces>> {
        match self.any {
            // any:true is highest precedence -> all namespaces
            // (matchNames is ignored)
            Some(true) => Ok(None),
            // Explicit any:false -> current + explicit matchNames
            Some(false) => Ok(Some(Namespaces {
                own_namespace: true,
                names: self.match_names.clone(),
            })),
            // any: not set
            None => {
                if self.match_names.is_empty() {
                    // No namespaces selected → cluster-wide discovery (no `namespaces:` block).
                    Ok(None)
                } else {
                    Ok(Some(Namespaces {
                        own_namespace: false,
                        names: self.match_names.clone(),
                    }))
                }
            }
        }
    }
}

/// Implicit relabelings that prometheus-operator normally applies to every job to every podmonitor job
fn get_implicit_pod_relabels() -> Vec<ClassicRelabelConfig> {
    vec![
        ClassicRelabelConfig {
            source_labels: vec!["__meta_kubernetes_namespace".into()],
            target_label: Some("namespace".into()),
            ..Default::default()
        },
        ClassicRelabelConfig {
            source_labels: vec!["__meta_kubernetes_pod_container_name".into()],
            target_label: Some("container".into()),
            ..Default::default()
        },
        ClassicRelabelConfig {
            source_labels: vec!["__meta_kubernetes_pod_name".into()],
            target_label: Some("pod".into()),
            ..Default::default()
        },
    ]
}

/// Implicit relabelings that prometheus-operator normally applies to every servicemonitor job
fn get_implicit_service_relabels() -> Vec<ClassicRelabelConfig> {
    vec![
        ClassicRelabelConfig {
            source_labels: vec!["__meta_kubernetes_namespace".into()],
            target_label: Some("namespace".into()),
            ..Default::default()
        },
        ClassicRelabelConfig {
            source_labels: vec!["__meta_kubernetes_service_name".into()],
            target_label: Some("service".into()),
            ..Default::default()
        },
        ClassicRelabelConfig {
            source_labels: vec!["__meta_kubernetes_pod_name".into()],
            target_label: Some("pod".into()),
            ..Default::default()
        },
    ]
}

/// DNS-1123-style sanitization: lowercase, runs of non-alphanumerics collapse to
/// a single `-`, leading/trailing `-` trimmed. Keeps the result usable as both a
/// classic job-name segment and a Kubernetes (GMP) resource-name segment.
fn sanitize_name_segment(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_dash = false;
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash {
            out.push('-');
            prev_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Sanitize a Kubernetes label key into a Prometheus label name (a meta-label
/// suffix or a `target_label`): every non-alphanumeric becomes `_`. E.g.
/// `materialize.cloud/organization-name` → `materialize_cloud_organization_name`.
fn sanitize_label_name(key: &str) -> String {
    key.replace(|c: char| !c.is_ascii_alphanumeric(), "_")
}

/// A short, stable per-endpoint name suffix used to disambiguate the jobs /
/// resources produced from a multi-endpoint Monitor. Prefers the last path
/// segment (`/metrics/mz_compute` → `mz-compute`), then the port name, then the
/// numeric port, finally the endpoint index.
fn endpoint_suffix(
    path: Option<&str>,
    port: Option<&str>,
    port_number: Option<i32>,
    index: usize,
) -> String {
    if let Some(seg) = path.and_then(|p| p.rsplit('/').find(|s| !s.is_empty())) {
        let s = sanitize_name_segment(seg);
        if !s.is_empty() {
            return s;
        }
    }
    if let Some(name) = port {
        let s = sanitize_name_segment(name);
        if !s.is_empty() {
            return s;
        }
    }
    if let Some(n) = port_number {
        return n.to_string();
    }
    index.to_string()
}

/// Ensure per-endpoint suffixes are unique within a single Monitor by appending
/// the index to any that collide (rare — endpoints usually differ by path).
fn disambiguate_suffixes(mut suffixes: Vec<String>) -> Vec<String> {
    let mut counts = std::collections::HashMap::new();
    for s in &suffixes {
        *counts.entry(s.clone()).or_insert(0usize) += 1;
    }
    for (i, s) in suffixes.iter_mut().enumerate() {
        if counts[s] > 1 {
            *s = format!("{s}-{i}");
        }
    }
    suffixes
}

// ============================================================
// Per-kind transpilation.
//
// Faithful operator-style semantics (see the tests below for the exact shape):
//   * the selector's `matchLabels` become an ordered `keep` relabel per label on
//     `(__meta_kubernetes_<scope>_label_<k>, __meta_kubernetes_<scope>_labelpresent_<k>)`
//     with `regex: (<v>);true`, where `<scope>` is `pod` (PodMonitor) or
//     `service` (ServiceMonitor); dotted/slashed label names are sanitized to
//     underscores;
//   * each endpoint yields one job named `<prefix>/<name>/<endpoint-suffix>`,
//     with a `keep` on the port-name meta label and the standard namespace /
//     pod / container (or service) metadata relabels;
//   * endpoint `path`/`scheme`/`interval`/`scrapeTimeout` map to the job's
//     `metrics_path`/`scheme`/`scrape_interval`/`scrape_timeout`;
//   * operator `relabelings` / `metricRelabelings` append to the job's
//     `relabel_configs` / `metric_relabel_configs` (camelCase → snake_case).
// ============================================================

/// PodMonitor → one `role: pod` job per `podMetricsEndpoints` entry.
fn transpile_pod_monitor(pod_monitor: &PodMonitor) -> Result<Vec<ScrapeJob>> {
    let mut jobs = Vec::new();
    let prefix = format!(
        "podMonitor/{}",
        pod_monitor
            .metadata
            .name
            .clone()
            .unwrap_or("default".into())
    );
    if pod_monitor.spec.selector.match_labels.is_empty() {
        return Err(Error::Unsupported("matchLabels is required".into()));
    }
    if !pod_monitor.spec.selector.match_expressions.is_empty() {
        return Err(Error::Unsupported(
            "matchExpressions is not supported".into(),
        ));
    }
    let sd_namespaces = match &pod_monitor.spec.namespace_selector {
        Some(ns_selector) => ns_selector.to_classic_sd_namespaces()?,
        None => None,
    };
    let mut common_relabels: Vec<ClassicRelabelConfig> = Vec::new();
    // NB: operator sorts keys, but this is fine
    for (k, v) in &pod_monitor.spec.selector.match_labels {
        let sanitized_k = sanitize_label_name(k);
        common_relabels.push(ClassicRelabelConfig {
            // Same labelpresent logic as prometheus-operator (detects `label: ""` edge case specifically)
            source_labels: vec![
                format!("__meta_kubernetes_pod_label_{sanitized_k}"),
                format!("__meta_kubernetes_pod_labelpresent_{sanitized_k}"),
            ],
            regex: Some(format!("({v});true")),
            action: Some("keep".into()),
            ..Default::default()
        });
    }
    let suffixes = disambiguate_suffixes(
        pod_monitor
            .spec
            .pod_metrics_endpoints
            .iter()
            .enumerate()
            .map(|(i, e)| endpoint_suffix(e.path.as_deref(), e.port.as_deref(), e.port_number, i))
            .collect(),
    );
    for (idx, endpoint) in pod_monitor.spec.pod_metrics_endpoints.iter().enumerate() {
        let job_name = format!("{}/{}", prefix, suffixes[idx]);
        let mut job = ScrapeJob {
            job_name,
            honor_labels: endpoint.honor_labels,
            scheme: endpoint.scheme.clone(),
            metrics_path: endpoint.path.clone(),
            scrape_interval: endpoint.interval.clone(),
            scrape_timeout: endpoint.scrape_timeout.clone(),
            basic_auth: endpoint.basic_auth.as_ref().map(basic_auth_to_classic),
            kubernetes_sd_configs: vec![KubernetesSdConfig {
                role: "pod".into(),
                namespaces: sd_namespaces.clone(),
            }],
            ..Default::default()
        };
        job.relabel_configs.extend(common_relabels.clone());
        if let Some(port) = &endpoint.port {
            if endpoint.port_number.is_some() {
                return Err(Error::Unsupported(
                    "endpoint cannot specify both port and portNumber".into(),
                ));
            }
            job.relabel_configs.push(ClassicRelabelConfig {
                source_labels: vec!["__meta_kubernetes_pod_container_port_name".into()],
                regex: Some(port.clone()),
                action: Some("keep".into()),
                ..Default::default()
            });
        } else if let Some(port) = endpoint.port_number {
            job.relabel_configs.push(ClassicRelabelConfig {
                source_labels: vec!["__meta_kubernetes_pod_container_port_number".into()],
                regex: Some(port.to_string()),
                action: Some("keep".into()),
                ..Default::default()
            });
        } else {
            return Err(Error::Unsupported(
                "endpoint must specify either port or portNumber".into(),
            ));
        }
        job.relabel_configs.extend(get_implicit_pod_relabels());
        // `podTargetLabels`: copy each pod label onto the metric (`replace` from
        // the `__meta_kubernetes_pod_label_*` meta-label to the same name).
        for label in &pod_monitor.spec.pod_target_labels {
            let sanitized = sanitize_label_name(label);
            job.relabel_configs.push(ClassicRelabelConfig {
                source_labels: vec![format!("__meta_kubernetes_pod_label_{sanitized}")],
                target_label: Some(sanitized),
                ..Default::default()
            });
        }
        for relabel in &endpoint.relabelings {
            job.relabel_configs
                .push(relabel.to_classic_relabel_config());
        }
        for relabel in &endpoint.metric_relabelings {
            job.metric_relabel_configs
                .push(relabel.to_classic_relabel_config());
        }
        jobs.push(job);
    }
    Ok(jobs)
}

/// ServiceMonitor → one `role: endpoints` job per `endpoints` entry.
/// This varies from PodMonitors in the following ways:
///  * the selector applies to services/endpoints, so the `keep` relabels are on `__meta_kubernetes_service_label_*`
///  * the SD role is `endpoints`, not `pod` (but namespaces are handled the same);
///  * implicit relabels on namespace, service, and pod (not container)
///  * only port name on service (and targetPort against backing pod)
fn transpile_service_monitor(service_monitor: &ServiceMonitor) -> Result<Vec<ScrapeJob>> {
    let mut jobs = Vec::new();
    let prefix = format!(
        "serviceMonitor/{}",
        service_monitor
            .metadata
            .name
            .clone()
            .unwrap_or("default".into())
    );
    if service_monitor.spec.selector.match_labels.is_empty() {
        return Err(Error::Unsupported("matchLabels is required".into()));
    }
    if !service_monitor.spec.selector.match_expressions.is_empty() {
        return Err(Error::Unsupported(
            "matchExpressions is not supported".into(),
        ));
    }
    let sd_namespaces = match &service_monitor.spec.namespace_selector {
        Some(ns_selector) => ns_selector.to_classic_sd_namespaces()?,
        None => None,
    };
    let mut common_relabels: Vec<ClassicRelabelConfig> = Vec::new();
    // This is different from podmonitor by just the source_labels
    for (k, v) in &service_monitor.spec.selector.match_labels {
        let sanitized_k = sanitize_label_name(k);
        common_relabels.push(ClassicRelabelConfig {
            // Same labelpresent logic as prometheus-operator (detects `label: ""` edge case specifically)
            source_labels: vec![
                format!("__meta_kubernetes_service_label_{sanitized_k}"),
                format!("__meta_kubernetes_service_labelpresent_{sanitized_k}"),
            ],
            regex: Some(format!("({v});true")),
            action: Some("keep".into()),
            ..Default::default()
        });
    }
    let suffixes = disambiguate_suffixes(
        service_monitor
            .spec
            .endpoints
            .iter()
            .enumerate()
            .map(|(i, e)| endpoint_suffix(e.path.as_deref(), e.port.as_deref(), None, i))
            .collect(),
    );
    // This is uncomfortably close to podmonitor endpoints
    for (idx, endpoint) in service_monitor.spec.endpoints.iter().enumerate() {
        let job_name = format!("{}/{}", prefix, suffixes[idx]);
        let mut job = ScrapeJob {
            job_name,
            honor_labels: endpoint.honor_labels,
            scheme: endpoint.scheme.clone(),
            metrics_path: endpoint.path.clone(),
            scrape_interval: endpoint.interval.clone(),
            scrape_timeout: endpoint.scrape_timeout.clone(),
            kubernetes_sd_configs: vec![KubernetesSdConfig {
                role: "endpoints".into(),
                namespaces: sd_namespaces.clone(),
            }],
            ..Default::default()
        };
        job.relabel_configs.extend(common_relabels.clone());
        if let Some(port) = &endpoint.port {
            job.relabel_configs.push(ClassicRelabelConfig {
                source_labels: vec!["__meta_kubernetes_endpoint_port_name".into()],
                regex: Some(port.clone()),
                action: Some("keep".into()),
                ..Default::default()
            });
        } else {
            match &endpoint.target_port {
                Some(Value::String(s)) => job.relabel_configs.push(ClassicRelabelConfig {
                    source_labels: vec!["FIXME_target_port".into()],
                    regex: Some(s.clone()),
                    action: Some("keep".into()),
                    ..Default::default()
                }),
                Some(Value::Number(n)) => job.relabel_configs.push(ClassicRelabelConfig {
                    source_labels: vec!["FIXME_target_port".into()],
                    regex: Some(n.to_string()),
                    action: Some("keep".into()),
                    ..Default::default()
                }),
                Some(_) => {
                    return Err(Error::Unsupported(
                        "endpoint targetPort must be a string or number".into(),
                    ));
                }
                None => {
                    return Err(Error::Unsupported(
                        "endpoint must specify either port or targetPort".into(),
                    ));
                }
            }
        }
        job.relabel_configs.extend(get_implicit_service_relabels());
        for relabel in &endpoint.relabelings {
            job.relabel_configs
                .push(relabel.to_classic_relabel_config());
        }
        for relabel in &endpoint.metric_relabelings {
            job.metric_relabel_configs
                .push(relabel.to_classic_relabel_config());
        }
        jobs.push(job);
    }
    Ok(jobs)
}

/// ScrapeConfig → a single job; near 1:1 (lowercase `role`, passthrough
/// relabelings).
fn transpile_scrape_config(scrape_config: &ScrapeConfigCrd) -> Result<Vec<ScrapeJob>> {
    let job_name = scrape_config
        .metadata
        .name
        .clone()
        .unwrap_or("default".into());
    let mut job = ScrapeJob {
        job_name: job_name.clone(),
        scheme: scrape_config.spec.scheme.clone(),
        ..Default::default()
    };
    for sd_config in &scrape_config.spec.kubernetes_sd_configs {
        job.kubernetes_sd_configs.push(KubernetesSdConfig {
            role: sd_config.role.to_lowercase(),
            namespaces: None,
        });
    }
    for relabel in &scrape_config.spec.relabelings {
        job.relabel_configs
            .push(relabel.to_classic_relabel_config());
    }
    for relabel in &scrape_config.spec.metric_relabelings {
        job.metric_relabel_configs
            .push(relabel.to_classic_relabel_config());
    }
    Ok(vec![job])
}

impl ScrapeConfigDocument {
    /// Assemble a combined classic document from a set of Monitors, in order.
    pub fn from_monitors(global: GlobalConfig, monitors: &[Monitor]) -> Result<Self> {
        let mut scrape_configs = Vec::new();
        for monitor in monitors {
            scrape_configs.extend(monitor.transpile()?);
        }
        Ok(Self {
            global,
            scrape_configs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Monitor whose `kind` we do not transpile is rejected before any schema
    /// work — and the error carries the offending `kind`/`apiVersion`.
    #[test]
    fn unknown_kind_is_rejected() {
        let err = Monitor::from_yaml_str(
            r#"
            apiVersion: monitoring.coreos.com/v1
            kind: Probe
            spec: {}
            "#,
        )
        .unwrap_err();
        match err {
            Error::UnknownKind { kind, .. } => assert_eq!(kind.as_deref(), Some("Probe")),
            other => panic!("expected UnknownKind, got {other:?}"),
        }
    }

    /// Every fixture under `packages/prometheus-scrapers/` parses and validates
    /// against its CRD schema. This is the green half of the TDD scaffold: the
    /// input model + schema layer work today, before `transpile()` is filled in.
    #[test]
    fn all_fixtures_parse_and_validate() {
        for (name, yaml) in crate::scrape::test_support::FIXTURES {
            Monitor::from_yaml_str(yaml)
                .unwrap_or_else(|e| panic!("fixture {name} failed to parse/validate:\n{e}"));
        }
    }

    // ========================================================
    // Classic transpilation goldens.
    //
    // Goldens are compared structurally, so YAML key order / quoting don't
    // matter — but `relabel_configs` is an array, so RELABEL ORDER is asserted.
    //
    // Conventions:
    //   * job_name = "podMonitor"/"serviceMonitor" + "/<name>/<endpoint-suffix>",
    //     where the suffix is the endpoint's last path segment (else port name,
    //     else index); for a ScrapeConfig, job_name = metadata.name.
    //   * fixtures carry no namespace/namespaceSelector → cluster-wide discovery
    //     (no `namespaces:` on the SD config).
    // ========================================================

    use crate::scrape::test_support::{assert_jobs, fixture};

    /// Core PodMonitor spec: one label, one endpoint, no path. Pins the selector
    /// `keep` shape, the port `keep`, and the namespace/pod/container relabels.
    #[test]
    fn pod_monitor_minimal() {
        let monitor = Monitor::from_yaml_str(
            r#"
            apiVersion: monitoring.coreos.com/v1
            kind: PodMonitor
            metadata:
              name: mini
            spec:
              selector:
                matchLabels:
                  app: foo
              podMetricsEndpoints:
                - port: metrics
            "#,
        )
        .unwrap();
        assert_jobs(
            monitor.transpile(),
            r#"
            - job_name: podMonitor/mini/metrics
              kubernetes_sd_configs:
                - role: pod
              relabel_configs:
                - source_labels: [__meta_kubernetes_pod_label_app, __meta_kubernetes_pod_labelpresent_app]
                  regex: (foo);true
                  action: keep
                - source_labels: [__meta_kubernetes_pod_container_port_name]
                  regex: metrics
                  action: keep
                - source_labels: [__meta_kubernetes_namespace]
                  target_label: namespace
                - source_labels: [__meta_kubernetes_pod_container_name]
                  target_label: container
                - source_labels: [__meta_kubernetes_pod_name]
                  target_label: pod
            "#,
        );
    }

    /// Dotted/slashed label names in the selector are sanitized to underscores in
    /// the `__meta_kubernetes_pod_label_*` meta-label, and a static `path` maps to
    /// `metrics_path`. (Real `podmonitor-environmentd.yaml` fixture.)
    #[test]
    fn pod_monitor_environmentd_fixture() {
        let monitor = Monitor::from_yaml_str(fixture("podmonitor-environmentd")).unwrap();
        assert_jobs(
            monitor.transpile(),
            r#"
            - job_name: podMonitor/environmentd/metrics
              kubernetes_sd_configs:
                - role: pod
              relabel_configs:
                - source_labels: [__meta_kubernetes_pod_label_app_kubernetes_io_name, __meta_kubernetes_pod_labelpresent_app_kubernetes_io_name]
                  regex: (environmentd);true
                  action: keep
                - source_labels: [__meta_kubernetes_pod_container_port_name]
                  regex: internal-http
                  action: keep
                - source_labels: [__meta_kubernetes_namespace]
                  target_label: namespace
                - source_labels: [__meta_kubernetes_pod_container_name]
                  target_label: container
                - source_labels: [__meta_kubernetes_pod_name]
                  target_label: pod
                - source_labels: [__meta_kubernetes_pod_label_materialize_cloud_organization_name]
                  target_label: materialize_cloud_organization_name
                - source_labels: [__meta_kubernetes_pod_label_materialize_cloud_organization_namespace]
                  target_label: materialize_cloud_organization_namespace
                - source_labels: [__meta_kubernetes_pod_label_materialize_cloud_organization_id]
                  target_label: materialize_cloud_organization_id
                - source_labels: [__meta_kubernetes_pod_label_cluster_environmentd_materialize_cloud_cluster_id]
                  target_label: cluster_environmentd_materialize_cloud_cluster_id
                - source_labels: [__meta_kubernetes_pod_label_cluster_environmentd_materialize_cloud_replica_id]
                  target_label: cluster_environmentd_materialize_cloud_replica_id
              metrics_path: /metrics
            "#,
        );
    }

    /// A PodMonitor with N endpoints yields N jobs, one per endpoint, each with
    /// its own `metrics_path` and a path-derived job-name suffix. (Real
    /// `podmonitor-sql.yaml` fixture: 4 SQL subsystem endpoints on one port.)
    #[test]
    fn pod_monitor_sql_fixture_one_job_per_endpoint() {
        let monitor = Monitor::from_yaml_str(fixture("podmonitor-sql")).unwrap();
        let jobs = monitor.transpile().expect("transpile");
        let names: Vec<&str> = jobs.iter().map(|j| j.job_name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "podMonitor/materialize-sql/mz-compute",
                "podMonitor/materialize-sql/mz-frontier",
                "podMonitor/materialize-sql/mz-storage",
                "podMonitor/materialize-sql/mz-usage",
            ]
        );
        let paths: Vec<Option<&str>> = jobs.iter().map(|j| j.metrics_path.as_deref()).collect();
        assert_eq!(
            paths,
            vec![
                Some("/metrics/mz_compute"),
                Some("/metrics/mz_frontier"),
                Some("/metrics/mz_storage"),
                Some("/metrics/mz_usage"),
            ]
        );
    }

    /// ServiceMonitor → `role: endpoints`, selector `keep` on the *service*
    /// label family, port `keep` on the *endpoint* port name, and service/pod
    /// metadata relabels. (No fixture exists yet — authored target.)
    #[test]
    fn service_monitor_minimal() {
        let monitor = Monitor::from_yaml_str(
            r#"
            apiVersion: monitoring.coreos.com/v1
            kind: ServiceMonitor
            metadata:
              name: example
            spec:
              selector:
                matchLabels:
                  app: example
              endpoints:
                - port: http
                  path: /metrics
            "#,
        )
        .unwrap();
        assert_jobs(
            monitor.transpile(),
            r#"
            - job_name: serviceMonitor/example/metrics
              kubernetes_sd_configs:
                - role: endpoints
              relabel_configs:
                - source_labels: [__meta_kubernetes_service_label_app, __meta_kubernetes_service_labelpresent_app]
                  regex: (example);true
                  action: keep
                - source_labels: [__meta_kubernetes_endpoint_port_name]
                  regex: http
                  action: keep
                - source_labels: [__meta_kubernetes_namespace]
                  target_label: namespace
                - source_labels: [__meta_kubernetes_service_name]
                  target_label: service
                - source_labels: [__meta_kubernetes_pod_name]
                  target_label: pod
              metrics_path: /metrics
            "#,
        );
    }

    /// ScrapeConfig is near-identity: `role: Node` → `node`, `relabelings`
    /// passed through (camelCase → snake_case). Note this intentionally differs
    /// from the annotation-based `kubelet-cadvisor` job in
    /// `legacy_scrape_config.yaml`: the CRD carries no scheme/tls/bearer because
    /// it relies on the default in-cluster apiServer credentials.
    #[test]
    fn scrape_config_cadvisor_fixture() {
        let monitor = Monitor::from_yaml_str(fixture("scrapeconfig-cadvisor")).unwrap();
        assert_jobs(
            monitor.transpile(),
            r#"
            - job_name: mz-kubelet-cadvisor
              kubernetes_sd_configs:
                - role: node
              relabel_configs:
                - action: labelmap
                  regex: __meta_kubernetes_node_label_(.+)
                - target_label: __address__
                  replacement: kubernetes.default.svc:443
                - source_labels: [__meta_kubernetes_node_name]
                  regex: (.+)
                  target_label: __metrics_path__
                  replacement: /api/v1/nodes/${1}/proxy/metrics/cadvisor
            "#,
        );
    }

    /// `from_monitors` assembles a single combined document: the `global` block
    /// plus every monitor's jobs, in order. Also exercises `to_yaml` + the
    /// optional `promtool` oracle.
    #[test]
    fn document_assembles_global_and_jobs() {
        let monitors = vec![
            Monitor::from_yaml_str(fixture("podmonitor-environmentd")).unwrap(),
            Monitor::from_yaml_str(fixture("scrapeconfig-cadvisor")).unwrap(),
        ];
        let doc = ScrapeConfigDocument::from_monitors(GlobalConfig::default(), &monitors).unwrap();

        assert_eq!(doc.global.scrape_interval, "1m");
        assert_eq!(doc.global.scrape_timeout, "10s");
        assert_eq!(doc.global.evaluation_interval, "1m");
        // environmentd → 1 job, cadvisor → 1 job.
        assert_eq!(doc.scrape_configs.len(), 2);

        let yaml = doc.to_yaml().expect("serialize document");
        crate::scrape::test_support::assert_promtool_ok(&yaml);
    }

    // ========================================================
    // GMP output (`to_gmp`).
    // ========================================================

    use crate::scrape::test_support::assert_serializes_to;

    /// Core PodMonitor → PodMonitoring mapping: a single-endpoint monitor yields
    /// one resource keeping the base name; selector and endpoint carry over; a
    /// named port becomes a string `port`; the endpoint's `interval` is kept.
    #[test]
    fn pod_monitor_to_gmp_minimal() {
        let monitor = Monitor::from_yaml_str(
            r#"
            apiVersion: monitoring.coreos.com/v1
            kind: PodMonitor
            metadata:
              name: mini
            spec:
              selector:
                matchLabels:
                  app: foo
              podMetricsEndpoints:
                - port: metrics
                  interval: 30s
            "#,
        )
        .unwrap();
        let resources = monitor.to_gmp().unwrap();
        assert_eq!(resources.len(), 1);
        assert_serializes_to(
            &resources[0],
            r#"
            apiVersion: monitoring.googleapis.com/v1
            kind: PodMonitoring
            metadata:
              name: mini
            spec:
              selector:
                matchLabels:
                  app: foo
              endpoints:
                - port: metrics
                  interval: 30s
            "#,
        );
    }

    /// Real `podmonitor-environmentd.yaml`: labels carry through, the selector is
    /// preserved verbatim (GMP keeps the dotted label key — no sanitization), and
    /// the missing `interval` is defaulted.
    #[test]
    fn pod_monitor_environmentd_to_gmp() {
        let monitor = Monitor::from_yaml_str(fixture("podmonitor-environmentd")).unwrap();
        let resources = monitor.to_gmp().unwrap();
        assert_eq!(resources.len(), 1);
        assert_serializes_to(
            &resources[0],
            r#"
            apiVersion: monitoring.googleapis.com/v1
            kind: PodMonitoring
            metadata:
              name: environmentd
              labels:
                app.kubernetes.io/part-of: materialize
                app.kubernetes.io/name: environmentd
            spec:
              selector:
                matchLabels:
                  app.kubernetes.io/name: environmentd
              endpoints:
                - port: internal-http
                  interval: 60s
                  path: /metrics
              targetLabels:
                fromPod:
                  - from: materialize.cloud/organization-name
                    to: materialize_cloud_organization_name
                  - from: materialize.cloud/organization-namespace
                    to: materialize_cloud_organization_namespace
                  - from: materialize.cloud/organization-id
                    to: materialize_cloud_organization_id
                  - from: cluster.environmentd.materialize.cloud/cluster-id
                    to: cluster_environmentd_materialize_cloud_cluster_id
                  - from: cluster.environmentd.materialize.cloud/replica-id
                    to: cluster_environmentd_materialize_cloud_replica_id
            "#,
        );
    }

    /// The unique-ports fix: a PodMonitor whose endpoints share a port (the four
    /// SQL endpoints, all on `internal-http`) fans out to one PodMonitoring per
    /// endpoint — each `<name>-<suffix>` with a single endpoint — so GMP's
    /// `unique-ports` admission policy is satisfied.
    #[test]
    fn pod_monitor_sql_fans_out_one_gmp_resource_per_endpoint() {
        let monitor = Monitor::from_yaml_str(fixture("podmonitor-sql")).unwrap();
        let resources = monitor.to_gmp().unwrap();
        let names: Vec<&str> = resources
            .iter()
            .map(|r| r.metadata.name.as_deref().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "materialize-sql-mz-compute",
                "materialize-sql-mz-frontier",
                "materialize-sql-mz-storage",
                "materialize-sql-mz-usage",
            ]
        );
        for r in &resources {
            assert_eq!(r.spec.endpoints.len(), 1, "one endpoint per resource");
            assert!(
                matches!(&r.spec.endpoints[0].port, gmp::IntOrString::Str(s) if s == "internal-http"),
                "all SQL endpoints share the internal-http port",
            );
        }
    }

    /// A cluster-wide `namespaceSelector` (`any: true`) maps to the cluster-scoped
    /// kind, and the resource carries no namespace.
    #[test]
    fn cluster_wide_namespace_selector_yields_clusterpodmonitoring() {
        let monitor = Monitor::from_yaml_str(
            r#"
            apiVersion: monitoring.coreos.com/v1
            kind: PodMonitor
            metadata:
              name: wide
              namespace: mz
            spec:
              namespaceSelector:
                any: true
              selector:
                matchLabels:
                  app: foo
              podMetricsEndpoints:
                - port: metrics
            "#,
        )
        .unwrap();
        let resources = monitor.to_gmp().unwrap();
        assert_eq!(resources.len(), 1);
        assert_eq!(resources[0].kind, "ClusterPodMonitoring");
        assert!(resources[0].metadata.namespace.is_none());
    }

    /// ServiceMonitor and ScrapeConfig have no GMP equivalent (GMP scrapes pods
    /// only) — `to_gmp` returns an empty `Vec` so the caller can skip them.
    #[test]
    fn service_monitor_and_scrape_config_have_no_gmp_form() {
        let sm = Monitor::from_yaml_str(
            r#"
            apiVersion: monitoring.coreos.com/v1
            kind: ServiceMonitor
            metadata:
              name: example
            spec:
              selector:
                matchLabels:
                  app: example
              endpoints:
                - port: http
            "#,
        )
        .unwrap();
        assert!(sm.to_gmp().unwrap().is_empty());

        let sc = Monitor::from_yaml_str(fixture("scrapeconfig-cadvisor")).unwrap();
        assert!(sc.to_gmp().unwrap().is_empty());
    }

    const POD_MONITOR_WITH_TARGET_LABELS: &str = r#"
        apiVersion: monitoring.coreos.com/v1
        kind: PodMonitor
        metadata:
          name: mini
        spec:
          selector:
            matchLabels:
              app: foo
          podTargetLabels:
            - materialize.cloud/organization-name
          podMetricsEndpoints:
            - port: metrics
        "#;

    /// `podTargetLabels` → a classic `replace` relabel copying the sanitized
    /// `__meta_kubernetes_pod_label_*` onto the same-named metric label.
    #[test]
    fn pod_target_labels_become_classic_relabels() {
        let monitor = Monitor::from_yaml_str(POD_MONITOR_WITH_TARGET_LABELS).unwrap();
        let jobs = monitor.transpile().unwrap();
        let last = jobs[0].relabel_configs.last().unwrap();
        assert_eq!(
            last.source_labels,
            vec!["__meta_kubernetes_pod_label_materialize_cloud_organization_name"]
        );
        assert_eq!(
            last.target_label.as_deref(),
            Some("materialize_cloud_organization_name")
        );
    }

    /// `podTargetLabels` → GMP `targetLabels.fromPod` with `from` = raw pod label
    /// key and `to` = sanitized Prometheus label name.
    #[test]
    fn pod_target_labels_become_gmp_from_pod() {
        let monitor = Monitor::from_yaml_str(POD_MONITOR_WITH_TARGET_LABELS).unwrap();
        let resources = monitor.to_gmp().unwrap();
        let target_labels = resources[0]
            .spec
            .target_labels
            .as_ref()
            .expect("targetLabels populated");
        assert_eq!(target_labels.from_pod.len(), 1);
        assert_eq!(
            target_labels.from_pod[0].from,
            "materialize.cloud/organization-name"
        );
        assert_eq!(
            target_labels.from_pod[0].to.as_deref(),
            Some("materialize_cloud_organization_name")
        );
    }

    const POD_MONITOR_WITH_BASIC_AUTH: &str = r#"
        apiVersion: monitoring.coreos.com/v1
        kind: PodMonitor
        metadata:
          name: mini
        spec:
          selector:
            matchLabels:
              app: foo
          podMetricsEndpoints:
            - port: metrics
              basicAuth:
                username:
                  name: my-secret
                  key: username
                password:
                  name: my-secret
                  key: password
        "#;

    /// Operator `basicAuth` (Secret refs) → classic inline `mz_support` username
    /// with no password. Classic Prometheus can't read a Kubernetes Secret, so the
    /// Secret coordinates are dropped; the internal-http port doesn't check the
    /// password, so none is emitted.
    #[test]
    fn basic_auth_becomes_classic_inline_username() {
        let monitor = Monitor::from_yaml_str(POD_MONITOR_WITH_BASIC_AUTH).unwrap();
        let jobs = monitor.transpile().unwrap();
        let auth = jobs[0].basic_auth.as_ref().expect("basic_auth populated");
        assert_eq!(auth.username.as_deref(), Some("mz_support"));
    }

    /// Operator `basicAuth` → GMP `basicAuth`: an inline `mz_support` username and
    /// no password (the internal-http port doesn't check it). The source's Secret
    /// references have no GMP equivalent and are dropped.
    #[test]
    fn basic_auth_becomes_gmp_inline_username() {
        let monitor = Monitor::from_yaml_str(POD_MONITOR_WITH_BASIC_AUTH).unwrap();
        let resources = monitor.to_gmp().unwrap();
        let auth = resources[0].spec.endpoints[0]
            .basic_auth
            .as_ref()
            .expect("basicAuth populated");
        assert_eq!(auth.username.as_deref(), Some("mz_support"));
    }

    /// In the real `podmonitor-sql.yaml`, only the `mz_compute` endpoint declares
    /// `basicAuth`, so exactly that one job (classic) / resource (GMP) is
    /// authenticated and the other three SQL subsystems are left untouched.
    #[test]
    fn sql_fixture_authenticates_only_mz_compute() {
        let monitor = Monitor::from_yaml_str(fixture("podmonitor-sql")).unwrap();

        let jobs = monitor.transpile().unwrap();
        for job in &jobs {
            if job.job_name.ends_with("/mz-compute") {
                assert!(
                    job.basic_auth.is_some(),
                    "mz-compute job should carry basic_auth"
                );
            } else {
                assert!(
                    job.basic_auth.is_none(),
                    "{} should not carry basic_auth",
                    job.job_name
                );
            }
        }

        let resources = monitor.to_gmp().unwrap();
        for r in &resources {
            let name = r.metadata.name.as_deref().unwrap();
            let authenticated = r.spec.endpoints[0].basic_auth.is_some();
            if name == "materialize-sql-mz-compute" {
                assert!(authenticated, "{name} should be authenticated");
            } else {
                assert!(!authenticated, "{name} should not be authenticated");
            }
        }
    }
}
