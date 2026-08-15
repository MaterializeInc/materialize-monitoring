// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Port-forward transport, for the one backend the Service proxy cannot reach.
//!
//! **The API server strips `Authorization` before proxying to a Service.** That
//! is deliberate on its part — it must not hand the caller's cluster credentials
//! to an arbitrary backend — but it means the proxy transport in
//! [`crate::cluster`] cannot authenticate to anything that wants that header.
//! Custom headers pass through fine, which is why Loki's `X-Scope-OrgID` works
//! there and Grafana's basic auth does not.
//!
//! Grafana is the only backend in the stack that needs it, so this is the
//! exception rather than the rule. Everything unauthenticated should keep using
//! the proxy, which has none of the machinery below.
//!
//! No local listener is involved: the forwarded stream is spoken to directly
//! with hyper. That removes port allocation, the bind race, and teardown — the
//! parts of a `kubectl port-forward` that actually flake. One connection per
//! request, because these checks issue a handful of small requests and a pooled
//! connection that goes stale between retries costs more than it saves.

use std::collections::BTreeMap;

use anyhow::{Context, Result, anyhow, bail};
use http_body_util::BodyExt;
use hyper_util::rt::TokioIo;
use k8s_openapi::api::core::v1::{Pod, Service};
use kube::Client;
use kube::api::{Api, ListParams};

/// A Service resolved down to a specific pod and container port.
pub struct Backend {
    pod: String,
    port: u16,
}

/// Resolve `service` to a ready pod behind it, and the container port its
/// `port` maps to.
///
/// Resolved per call rather than cached: a pod that is rolled between checks
/// would otherwise leave a stale name behind, and the failure reads as the
/// backend being down.
pub async fn resolve(
    client: &Client,
    namespace: &str,
    service: &str,
    port: u16,
) -> Result<Backend> {
    let services: Api<Service> = Api::namespaced(client.clone(), namespace);
    let svc = services
        .get(service)
        .await
        .with_context(|| format!("getting Service {service} in {namespace}"))?;
    let spec = svc
        .spec
        .ok_or_else(|| anyhow!("Service {service} has no spec"))?;

    let selector = spec
        .selector
        .filter(|s| !s.is_empty())
        .ok_or_else(|| anyhow!("Service {service} has no selector; nothing to forward to"))?;

    let service_port = spec
        .ports
        .unwrap_or_default()
        .into_iter()
        .find(|p| p.port == i32::from(port))
        .ok_or_else(|| anyhow!("Service {service} does not expose port {port}"))?;

    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let list = pods
        .list(&ListParams::default().labels(&label_selector(&selector)))
        .await
        .with_context(|| format!("listing pods behind Service {service}"))?;

    // Ready, not merely Running: a pod that is up but still failing its
    // readiness probe would refuse the connection, and the resulting error names
    // the port rather than the readiness state.
    let pod = list.items.iter().find(|p| is_ready(p)).ok_or_else(|| {
        anyhow!(
            "no ready pod behind Service {service} ({} pods matched {})",
            list.items.len(),
            label_selector(&selector),
        )
    })?;

    // `targetPort` may be a name, which is resolvable only against the pod's own
    // container ports — the Service does not carry the number.
    let target = match service_port.target_port {
        None => port,
        Some(ref t) => resolve_target_port(t, pod)?,
    };

    Ok(Backend {
        pod: pod
            .metadata
            .name
            .clone()
            .ok_or_else(|| anyhow!("pod behind Service {service} has no name"))?,
        port: target,
    })
}

/// Issue one GET over a fresh port-forward and return the body.
///
/// Non-2xx is an error carrying the status and the body, because the body is
/// where the reason is — Grafana answers an auth failure with a 401 and a JSON
/// message, and the status alone does not distinguish "wrong password" from
/// "this endpoint needs an admin role".
pub async fn get(
    client: &Client,
    namespace: &str,
    backend: &Backend,
    path: &str,
    headers: &[(String, String)],
) -> Result<Vec<u8>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let mut forwarder = pods
        .portforward(&backend.pod, &[backend.port])
        .await
        .with_context(|| format!("port-forwarding {}:{}", backend.pod, backend.port))?;

    let stream = forwarder
        .take_stream(backend.port)
        .ok_or_else(|| anyhow!("port-forward to {} yielded no stream", backend.pod))?;

    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .context("HTTP handshake over the forwarded stream")?;

    // The connection future drives the socket and must be polled for the
    // request to make progress. It ends when the response is complete; an error
    // here shows up as a failed request, which is where it is actionable.
    tokio::spawn(async move {
        let _ = connection.await;
    });

    let mut request = http::Request::get(format!("/{}", path.trim_start_matches('/')))
        // HTTP/1.1 requires it, and there is no real hostname here — the
        // connection is already pinned to one pod.
        .header("Host", "localhost");
    for (name, value) in headers {
        request = request.header(name, value);
    }
    let request = request
        .body(String::new())
        .with_context(|| format!("building request for {path}"))?;

    let response = sender
        .send_request(request)
        .await
        .with_context(|| format!("GET {path} on {}", backend.pod))?;

    let status = response.status();
    let body = response
        .into_body()
        .collect()
        .await
        .with_context(|| format!("reading response body from {path}"))?
        .to_bytes();
    if !status.is_success() {
        let head: String = String::from_utf8_lossy(&body).chars().take(400).collect();
        bail!("GET {path} returned {status}: {head}");
    }
    Ok(body.to_vec())
}

fn label_selector(selector: &BTreeMap<String, String>) -> String {
    selector
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join(",")
}

fn is_ready(pod: &Pod) -> bool {
    pod.status
        .as_ref()
        .and_then(|s| s.conditions.as_ref())
        .is_some_and(|conditions| {
            conditions
                .iter()
                .any(|c| c.type_ == "Ready" && c.status == "True")
        })
}

fn resolve_target_port(
    target: &k8s_openapi::apimachinery::pkg::util::intstr::IntOrString,
    pod: &Pod,
) -> Result<u16> {
    use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;

    match target {
        IntOrString::Int(n) => u16::try_from(*n).with_context(|| format!("target port {n}")),
        IntOrString::String(name) => pod
            .spec
            .as_ref()
            .map(|s| s.containers.as_slice())
            .unwrap_or_default()
            .iter()
            .filter_map(|c| c.ports.as_ref())
            .flatten()
            .find(|p| p.name.as_deref() == Some(name.as_str()))
            .and_then(|p| u16::try_from(p.container_port).ok())
            .ok_or_else(|| {
                anyhow!(
                    "named target port {name:?} is not declared by any container in {}",
                    pod.metadata.name.as_deref().unwrap_or("<unnamed>")
                )
            }),
    }
}
