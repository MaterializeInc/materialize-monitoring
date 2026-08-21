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
//! Two transports, because neither works everywhere.
//!
//! **Port-forward is the default.** It reaches pods through the kubelet, which
//! is a path every managed control plane already keeps open, so it works on kind
//! and on a real cloud alike. No local listener is involved — the forwarded
//! stream is spoken to directly, so there is no port allocation, no bind race
//! and no teardown. See [`crate::forward`].
//!
//! **The API server's Service proxy** —
//! `/api/v1/namespaces/<ns>/services/<svc>:<port>/proxy/<path>` — is lighter and
//! is available with `--transport proxy`, but it is not a general answer:
//!
//! - *It cannot carry `Authorization`.* The API server strips it before
//!   proxying, since it must not hand a caller's cluster credentials to an
//!   arbitrary Service. Custom headers pass through untouched, which is why
//!   Loki's `X-Scope-OrgID` works over it and Grafana's basic auth does not.
//!   [`Cluster::get_authenticated_json`] port-forwards regardless of the flag.
//! - *It needs control-plane-to-pod reachability on the target port.* On EKS the
//!   control plane sits in an AWS-managed VPC and the node security groups admit
//!   only a few ports, so a proxied request to Thanos on 9090 times out while a
//!   port-forward to the same pod succeeds. This was measured, not assumed.
//!
//! So the proxy is a kind-and-similar optimisation, and the default is the one
//! that is correct everywhere.

use std::path::PathBuf;
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use http_body_util::BodyExt;
use k8s_openapi::api::core::v1::{Secret, Service};
use kube::api::{Api, ApiResource, DynamicObject, GroupVersionKind, ListParams};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Client, Config};
use percent_encoding::{AsciiSet, NON_ALPHANUMERIC, utf8_percent_encode};

use crate::forward;

/// How to reach an in-cluster Service.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Transport {
    /// Through the kubelet. Works everywhere; the default.
    PortForward,
    /// Through the API server's Service proxy subresource. Lighter, but cannot
    /// carry `Authorization` and needs control-plane-to-pod reachability.
    Proxy,
}

/// A connected cluster, scoped to the namespace the release lives in.
pub struct Cluster {
    client: Client,
    namespace: String,
    transport: Transport,
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
        transport: Transport,
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
            transport,
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

    /// Read one key out of a Secret.
    ///
    /// Used for Grafana's admin credentials, which the chart generates rather
    /// than takes as input — so there is nothing to pass on the command line.
    pub async fn secret_value(&self, name: &str, key: &str) -> Result<String> {
        let api: Api<Secret> = Api::namespaced(self.client.clone(), &self.namespace);
        let secret = api
            .get(name)
            .await
            .with_context(|| format!("getting Secret {name} in {}", self.namespace))?;

        let data = secret
            .data
            .ok_or_else(|| anyhow!("Secret {name} has no data"))?;
        let value = data
            .get(key)
            .ok_or_else(|| {
                let keys: Vec<_> = data.keys().map(String::as_str).collect();
                anyhow!(
                    "Secret {name} has no key {key:?} (has: {})",
                    keys.join(", ")
                )
            })?
            .clone();

        String::from_utf8(value.0).with_context(|| format!("Secret {name}/{key} is not UTF-8"))
    }

    /// Delete a Secret, so cert-manager reissues the certificate behind it.
    ///
    /// The forced half of the rotation assertion. Deleting is used rather than a
    /// short `renewBefore` because provoking renewal through lifetimes short
    /// enough to fit a test run has been observed to livelock cert-manager —
    /// after which it stops issuing and reports the expired certificates as
    /// healthy, which is the failure the assertion is supposed to detect.
    pub async fn delete_secret(&self, name: &str) -> Result<()> {
        let api: Api<Secret> = Api::namespaced(self.client.clone(), &self.namespace);
        api.delete(name, &kube::api::DeleteParams::default())
            .await
            .with_context(|| format!("deleting Secret {name} in {}", self.namespace))?;
        Ok(())
    }

    /// List custom resources of one kind in the release namespace.
    ///
    /// Typed against `DynamicObject` rather than generated structs: the suite
    /// reads a handful of fields out of grafana-operator's CRs, and vendoring
    /// their full schemas would couple this to the operator's release cadence
    /// for no gain.
    pub async fn list_custom(
        &self,
        group: &str,
        version: &str,
        kind: &str,
    ) -> Result<Vec<DynamicObject>> {
        let gvk = GroupVersionKind::gvk(group, version, kind);
        let resource = ApiResource::from_gvk(&gvk);
        let api: Api<DynamicObject> =
            Api::namespaced_with(self.client.clone(), &self.namespace, &resource);

        let list = api
            .list(&ListParams::default())
            .await
            .with_context(|| format!("listing {kind} in {}", self.namespace))?;
        Ok(list.items)
    }

    /// GET `path` from a Service over the configured transport.
    ///
    /// `path` is everything after the service root, query string included —
    /// e.g. `loki/api/v1/labels?start=...`.
    pub async fn get(&self, target: &ServiceTarget, path: &str) -> Result<String> {
        let bytes = self.get_bytes(target, path).await?;
        String::from_utf8(bytes)
            .with_context(|| format!("{path} (svc {}) is not UTF-8", target.service))
    }

    /// [`Cluster::get`], parsed as JSON.
    pub async fn get_json(&self, target: &ServiceTarget, path: &str) -> Result<serde_json::Value> {
        let body = self.get(target, path).await?;
        parse_json(&body, path, &target.service)
    }

    /// GET `path` and return the raw bytes.
    ///
    /// The byte-level entry point, because Alloy's support bundle is a zip and
    /// decoding it as UTF-8 would corrupt the archive into an unhelpful parse
    /// error rather than a readable one.
    pub async fn get_bytes(&self, target: &ServiceTarget, path: &str) -> Result<Vec<u8>> {
        match self.transport {
            Transport::PortForward => self.forward_bytes(target, path).await,
            Transport::Proxy => self.proxy_bytes(target, path).await,
        }
    }

    /// GET `path` from a backend that authenticates with `Authorization`.
    ///
    /// Always port-forwards, whatever `--transport` says: the API server strips
    /// that header before proxying, so the proxy cannot carry it at all. Named
    /// for the caller's requirement rather than the mechanism, since the
    /// mechanism is not a choice here.
    pub async fn get_authenticated_json(
        &self,
        target: &ServiceTarget,
        path: &str,
    ) -> Result<serde_json::Value> {
        let bytes = self.forward_bytes(target, path).await?;
        let body = String::from_utf8(bytes)
            .with_context(|| format!("{path} (svc {}) is not UTF-8", target.service))?;
        parse_json(&body, path, &target.service)
    }

    async fn forward_bytes(&self, target: &ServiceTarget, path: &str) -> Result<Vec<u8>> {
        let backend =
            forward::resolve(&self.client, &self.namespace, &target.service, target.port).await?;
        forward::get(
            &self.client,
            &self.namespace,
            &backend,
            path,
            &target.headers,
        )
        .await
    }

    async fn proxy_bytes(&self, target: &ServiceTarget, path: &str) -> Result<Vec<u8>> {
        let uri = self.proxy_uri(target, path);
        let mut builder = http::Request::get(&uri);
        for (name, value) in &target.headers {
            builder = builder.header(name, value);
        }
        let request = builder
            .body(kube::client::Body::empty())
            .with_context(|| format!("building request for {uri}"))?;

        let response = self
            .client
            .send(request)
            .await
            .with_context(|| format!("GET {} (svc {})", path, target.service))?;

        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .with_context(|| format!("reading response body from {path}"))?
            .to_bytes();

        if !status.is_success() {
            let head: String = String::from_utf8_lossy(&body).chars().take(400).collect();
            bail!(
                "GET {path} (svc {}) returned {status}: {head}",
                target.service
            );
        }
        Ok(body.to_vec())
    }

    fn proxy_uri(&self, target: &ServiceTarget, path: &str) -> String {
        // The port is part of the subresource name (`<svc>:<port>`), not a
        // network port — the connection is to the API server.
        let ns = encode(&self.namespace);
        let svc = encode(&target.service);
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
/// Headers are owned rather than borrowed because some of them are computed:
/// Grafana's admin credentials come out of a Secret at runtime, so there is no
/// caller-side string to borrow from.
pub struct ServiceTarget {
    pub service: String,
    pub port: u16,
    pub headers: Vec<(String, String)>,
}

impl ServiceTarget {
    pub fn new(service: impl Into<String>, port: u16) -> Self {
        Self {
            service: service.into(),
            port,
            headers: Vec::new(),
        }
    }

    pub fn with_header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    /// The tenant header Loki reads as. Named rather than passed as a raw pair
    /// because forgetting it does not fail loudly: the bundled Loki runs
    /// `auth_enabled: true` and answers `no org id`, which reads like an empty
    /// stack rather than an auth problem.
    pub fn with_tenant(self, tenant: &str) -> Self {
        self.with_header("X-Scope-OrgID", tenant)
    }
}

/// Parse a response body, quoting the start of it when it is not JSON.
///
/// Truncated deliberately: a Loki or Grafana error page runs long, and the first
/// line is where the reason is.
fn parse_json(body: &str, path: &str, service: &str) -> Result<serde_json::Value> {
    serde_json::from_str(body).with_context(|| {
        let head: String = body.chars().take(400).collect();
        format!("parsing JSON from {path} (svc {service}): {head}")
    })
}

/// Percent-encode a query-string **value**.
///
/// Deliberately aggressive — a server URL-decodes a query value before reading
/// it, so over-encoding is free here and under-encoding is not.
pub fn encode(value: &str) -> String {
    utf8_percent_encode(value, NON_ALPHANUMERIC).to_string()
}

/// The unreserved set from RFC 3986: everything else in a path segment is
/// escaped.
///
/// Narrower than [`encode`] on purpose. Routers match path segments against the
/// *raw* path, so escaping a character that did not need it changes which route
/// matches: `mzmon-loki` encoded as `mzmon%2Dloki` is a 404 from Grafana even
/// though the two are the same string once decoded.
const PATH_SEGMENT: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'.')
    .remove(b'_')
    .remove(b'~');

/// Percent-encode one path segment.
pub fn encode_segment(value: &str) -> String {
    utf8_percent_encode(value, PATH_SEGMENT).to_string()
}

/// Nanoseconds since the epoch, which is the timestamp unit Loki's query API
/// takes.
pub fn unix_nanos_now() -> Result<u128> {
    unix_nanos_ago(Duration::ZERO)
}

/// Nanoseconds since the epoch, `window` ago.
pub fn unix_nanos_ago(window: Duration) -> Result<u128> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| anyhow!("system clock is before the epoch: {e}"))?;
    Ok(now.saturating_sub(window).as_nanos())
}

#[cfg(test)]
mod tests {
    use super::{encode, encode_segment};

    /// The bug this pins: `encode` escapes `-` to `%2D`, which is harmless in a
    /// query value but changes which route a server matches. Grafana answered
    /// `/api/datasources/proxy/uid/mzmon%2Dloki/...` with a 404.
    #[test]
    fn a_path_segment_keeps_the_unreserved_characters() {
        assert_eq!(encode_segment("mzmon-loki"), "mzmon-loki");
        assert_eq!(encode_segment("a.b_c~d"), "a.b_c~d");
        assert_eq!(encode("mzmon-loki"), "mzmon%2Dloki");
    }

    #[test]
    fn a_path_segment_still_escapes_separators() {
        assert_eq!(encode_segment("a/b"), "a%2Fb");
        assert_eq!(encode_segment("a?b"), "a%3Fb");
        assert_eq!(encode_segment("a b"), "a%20b");
    }

    /// A LogQL selector has to survive intact as a query value.
    #[test]
    fn a_query_value_escapes_logql_punctuation() {
        assert_eq!(
            encode("{namespace=\"monitoring\"}"),
            "%7Bnamespace%3D%22monitoring%22%7D"
        );
    }
}
