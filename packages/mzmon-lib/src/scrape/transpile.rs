// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Parse a prometheus-operator Monitor and transpile it to classic
//! `scrape_configs`.
//!
//! IMPLEMENTATION STATUS (TDD scaffold): the parse/validate plumbing
//! (`Monitor::from_yaml_str`) and the document assembly
//! (`ScrapeConfigDocument::from_monitors`) are implemented. The per-kind
//! `transpile_*` bodies are `todo!()` — they are the implementation target
//! driven by the tests at the bottom of this file.

use serde_json::Value;

use crate::scrape::classic::config::RelabelConfig as ClassicRelabelConfig;
use crate::scrape::classic::config::{
    GlobalConfig, KubernetesSdConfig, Namespaces, ScrapeConfigDocument, ScrapeJob,
};
use crate::scrape::error::{Error, Result};
use crate::scrape::operator::common::{NamespaceSelector, RelabelConfig as OperatorRelabelConfig};
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

// ============================================================
// Per-kind transpilation — IMPLEMENTATION TARGET (currently `todo!()`).
//
// Faithful operator-style semantics (see the tests below for the exact shape):
//   * the selector's `matchLabels` become an ordered `keep` relabel per label on
//     `(__meta_kubernetes_<scope>_label_<k>, __meta_kubernetes_<scope>_labelpresent_<k>)`
//     with `regex: (<v>);true`, where `<scope>` is `pod` (PodMonitor) or
//     `service` (ServiceMonitor); dotted/slashed label names are sanitized to
//     underscores;
//   * each endpoint yields one job with a `keep` on the port-name meta label and
//     the standard namespace / pod / container (or service) metadata relabels;
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
        let sanitized_k = k.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
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
    for (idx, endpoint) in pod_monitor.spec.pod_metrics_endpoints.iter().enumerate() {
        let job_name = format!("{}/{}", prefix, idx);
        let mut job = ScrapeJob {
            job_name,
            honor_labels: endpoint.honor_labels,
            scheme: endpoint.scheme.clone(),
            metrics_path: endpoint.path.clone(),
            scrape_interval: endpoint.interval.clone(),
            scrape_timeout: endpoint.scrape_timeout.clone(),
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
        let sanitized_k = k.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
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
    // This is uncomfortably close to podmonitor endpoints
    for (idx, endpoint) in service_monitor.spec.endpoints.iter().enumerate() {
        let job_name = format!("{}/{}", prefix, idx);
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
    // Transpilation goldens — the IMPLEMENTATION TARGET.
    //
    // These are RED until `transpile_*` is implemented (`todo!()` panics).
    // Goldens are compared structurally, so YAML key order / quoting don't
    // matter — but `relabel_configs` is an array, so RELABEL ORDER is asserted.
    //
    // Conventions encoded below (the implementer may revise these, but must
    // update the goldens in lockstep):
    //   * job_name = "podMonitor"/"serviceMonitor" + "/<name>/<endpointIdx>";
    //     for a ScrapeConfig, job_name = metadata.name.
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
            - job_name: podMonitor/mini/0
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
            - job_name: podMonitor/environmentd/0
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
              metrics_path: /metrics
            "#,
        );
    }

    /// A PodMonitor with N endpoints yields N jobs, one per endpoint, each with
    /// its own `metrics_path` and a `/<index>` job-name suffix. (Real
    /// `podmonitor-sql.yaml` fixture: 4 SQL subsystem endpoints.)
    #[test]
    fn pod_monitor_sql_fixture_one_job_per_endpoint() {
        let monitor = Monitor::from_yaml_str(fixture("podmonitor-sql")).unwrap();
        let jobs = monitor.transpile().expect("transpile");
        let names: Vec<&str> = jobs.iter().map(|j| j.job_name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                "podMonitor/materialize-sql/0",
                "podMonitor/materialize-sql/1",
                "podMonitor/materialize-sql/2",
                "podMonitor/materialize-sql/3",
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
            - job_name: serviceMonitor/example/0
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
}
