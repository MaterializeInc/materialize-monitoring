// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Cluster connectivity, and HTTP to in-cluster Services.
//!
//! Requests to Loki, Thanos, Grafana and Alloy go through the API server's
//! Service proxy subresource:
//!
//! ```text
//! /api/v1/namespaces/<ns>/services/<svc>:<port>/proxy/<path>
//! ```
//!
//! rather than through a port-forward. Both reach the same place; the proxy has
//! no local listener, no port allocation, no teardown, and no window in which
//! the tunnel is up but not yet forwarding — which is the whole flake surface of
//! the port-forward approach. It also behaves identically against kind, EKS and
//! GKE, so the tier-3 variants need no separate transport.
//!
//! The cost is one RBAC verb (`services/proxy`), which every kubeconfig these
//! tiers run under already has. If a stack ever needs to be reached from a
//! service account that deliberately lacks it, a port-forward transport can be
//! added behind the same [`Cluster`] API.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use k8s_openapi::api::core::v1::Service;
use kube::api::{Api, ListParams};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Client, Config};
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};

/// A connected cluster, scoped to the namespace the release lives in.
pub struct Cluster {
    client: Client,
    namespace: String,
    /// What we were asked to target, echoed in output. Not
    /// `config current-context`: that reports the *configured* context and
    /// ignores an explicit override, so it would name the cluster we are not
    /// talking to.
    context: String,
}

impl Cluster {
    /// Connect using `context` from `kubeconfig`, or the ambient defaults when
    /// either is unset.
    pub async fn connect(
        kubeconfig: Option<&PathBuf>,
        context: Option<&str>,
        namespace: &str,
    ) -> Result<Self> {
        let kubeconfig = match kubeconfig {
            Some(path) => Kubeconfig::read_from(path)
                .with_context(|| format!("reading kubeconfig {}", path.display()))?,
            None => Kubeconfig::read().context("reading kubeconfig")?,
        };

        // Checked up front. An unknown context otherwise surfaces later as a
        // connection failure, which every assertion then spends its full
        // deadline retrying and reports as a timeout — indistinguishable from a
        // broken stack, when the real fault is a typo in the invocation.
        if let Some(context) = context
            && !kubeconfig.contexts.iter().any(|c| c.name == context)
        {
            let known: Vec<_> = kubeconfig
                .contexts
                .iter()
                .map(|c| c.name.as_str())
                .collect();
            bail!(
                "kubeconfig has no context {context:?} (known: {})",
                known.join(", ")
            );
        }

        let options = KubeConfigOptions {
            context: context.map(str::to_owned),
            ..Default::default()
        };
        let config = Config::from_custom_kubeconfig(kubeconfig, &options)
            .await
            .context("building client config from kubeconfig")?;

        let resolved = context
            .map(str::to_owned)
            .unwrap_or_else(|| config.cluster_url.to_string());
        let client = Client::try_from(config).context("building Kubernetes client")?;

        Ok(Self {
            client,
            namespace: namespace.to_owned(),
            context: resolved,
        })
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn context(&self) -> &str {
        &self.context
    }

    /// Confirm the API server answers at all, so a bad kubeconfig fails once at
    /// startup instead of once per assertion.
    pub async fn preflight(&self) -> Result<()> {
        let api: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);
        api.list(&ListParams::default().limit(1))
            .await
            .with_context(|| {
                format!(
                    "listing Services in namespace {} (context {})",
                    self.namespace, self.context
                )
            })?;
        Ok(())
    }

    /// Whether a Service of this name exists in the release namespace.
    pub async fn service_exists(&self, name: &str) -> Result<bool> {
        let api: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);
        match api.get_opt(name).await {
            Ok(found) => Ok(found.is_some()),
            Err(err) => Err(err).with_context(|| format!("getting Service {name}")),
        }
    }

    /// The first of `candidates` that exists, for names that depend on a
    /// deployment shape we would otherwise have to infer from values.
    pub async fn first_existing_service(&self, candidates: &[&str]) -> Result<String> {
        for name in candidates {
            if self.service_exists(name).await? {
                return Ok((*name).to_owned());
            }
        }
        bail!(
            "none of the Services {:?} exist in namespace {}",
            candidates,
            self.namespace
        )
    }

    /// GET `path` from a Service, returning the raw body.
    ///
    /// `path` is everything after the proxy prefix, query string included —
    /// e.g. `loki/api/v1/labels?start=...`.
    pub async fn get(&self, target: &ServiceTarget<'_>, path: &str) -> Result<String> {
        let uri = self.proxy_uri(target, path);
        let mut builder = http::Request::get(&uri);
        for (name, value) in target.headers {
            builder = builder.header(*name, *value);
        }
        let request = builder
            .body(Vec::new())
            .with_context(|| format!("building request for {uri}"))?;

        self.client
            .request_text(request)
            .await
            .with_context(|| format!("GET {} (svc {})", path, target.service))
    }

    /// GET `path` and parse the body as JSON.
    pub async fn get_json(
        &self,
        target: &ServiceTarget<'_>,
        path: &str,
    ) -> Result<serde_json::Value> {
        let body = self.get(target, path).await?;
        serde_json::from_str(&body).with_context(|| {
            // Truncated: a Loki or Grafana error page is long, and the first
            // line is where the reason is.
            let head: String = body.chars().take(400).collect();
            format!("parsing JSON from {path} (svc {}): {head}", target.service)
        })
    }

    fn proxy_uri(&self, target: &ServiceTarget<'_>, path: &str) -> String {
        // The port is part of the subresource name (`<svc>:<port>`), not a
        // network port — the connection is to the API server.
        let ns = encode(&self.namespace);
        let svc = encode(target.service);
        let path = path.trim_start_matches('/');
        format!(
            "/api/v1/namespaces/{ns}/services/{svc}:{}/proxy/{path}",
            target.port
        )
    }
}

/// A Service to talk to, plus the headers every request to it needs.
///
/// The headers are attached here rather than per call because the ones that
/// matter are per-backend and easy to forget: Loki's bundled deployment runs
/// `auth_enabled: true`, so a read without `X-Scope-OrgID` fails with `no org
/// id` regardless of whether anything was ingested.
pub struct ServiceTarget<'a> {
    pub service: &'a str,
    pub port: u16,
    pub headers: &'a [(&'a str, &'a str)],
}

impl<'a> ServiceTarget<'a> {
    pub fn new(service: &'a str, port: u16) -> Self {
        Self {
            service,
            port,
            headers: &[],
        }
    }

    pub fn with_headers(mut self, headers: &'a [(&'a str, &'a str)]) -> Self {
        self.headers = headers;
        self
    }
}

/// Percent-encode a query-string value.
pub fn encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

/// Nanoseconds since the epoch, which is the timestamp unit Loki's query API
/// takes.
pub fn unix_nanos_ago(window: Duration) -> Result<u128> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow!("system clock is before the epoch: {e}"))?;
    Ok(now.saturating_sub(window).as_nanos())
}
