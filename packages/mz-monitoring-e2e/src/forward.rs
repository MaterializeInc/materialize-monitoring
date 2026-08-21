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
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use http_body_util::BodyExt;
use hyper_util::rt::TokioIo;
use k8s_openapi::api::core::v1::{Pod, Service};
use kube::Client;
use kube::api::{Api, ListParams};
use rustls::ClientConfig;
use rustls::pki_types::ServerName;
use tokio_rustls::TlsConnector;

/// How to speak TLS over a forwarded stream.
///
/// `server_name` is asserted in the handshake and verified against the
/// certificate's SANs. It is **`localhost`** for everything this suite dials,
/// and that is the honest name rather than a convenient one: a port-forward is a
/// loopback tunnel straight to one pod, so no Service DNS name is ever resolved
/// and asserting one would verify a binding the transport did not use. The chart
/// puts `localhost` and `127.0.0.1` on every certificate for exactly this case.
///
/// What the handshake therefore proves is the chain — that the server presents a
/// certificate the internal CA signed. Whether the SAN ladder covers the Service
/// names the *chart* dials is a render-time property, checked by
/// `mzmon.certificates.validate` before anything is applied.
#[derive(Clone)]
pub struct TlsDial {
    pub config: Arc<ClientConfig>,
    pub server_name: String,
}

impl TlsDial {
    pub fn new(config: Arc<ClientConfig>) -> Self {
        Self {
            config,
            server_name: "localhost".to_string(),
        }
    }
}

/// Marks an error as coming from `TlsConnector::connect` itself.
///
/// A marker in the error chain rather than a string to match on: the
/// distinction carries an assertion, and telling the cases apart by grepping a
/// message would silently start passing the day the message is reworded.
#[derive(Debug)]
pub struct HandshakeFailed;

impl std::fmt::Display for HandshakeFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("TLS handshake failed")
    }
}

impl std::error::Error for HandshakeFailed {}

/// Whether the peer turned us away at the TLS layer, rather than answering over
/// it.
///
/// **Not the same as "`connect` returned an error", and under TLS 1.3 it usually
/// is not.** The client sends its Certificate and Finished in the same flight as
/// its first application data, so `TlsConnector::connect` resolves *before* the
/// server has looked at the certificate — or at its absence. A listener set to
/// `RequireAndVerifyClientCert` therefore accepts the connection, then sends
/// `CertificateRequired` as a fatal alert, and the client meets it on the first
/// read. Measured against a phase-3 gateway, where it arrives as
/// `connection error: received fatal alert: CertificateRequired` from hyper.
///
/// So the predicate is "a `rustls::Error` appears in the chain" — refused at the
/// handshake and refused immediately after it are the same fact about the
/// server's policy, and the difference between them is a protocol-version
/// detail. An HTTP status, by contrast, never produces one: it means the request
/// got past the policy.
pub fn is_tls_refusal(err: &anyhow::Error) -> bool {
    err.chain().any(|e| {
        if e.is::<HandshakeFailed>() || e.is::<rustls::Error>() {
            return true;
        }
        // A `rustls::Error` that travelled inside an `io::Error` is invisible to
        // a plain chain walk, and this is the usual case rather than an edge
        // one: `io::Error::source` delegates to the *inner* error's source
        // instead of yielding the inner error, so the walk steps straight past
        // it and ends. `get_ref` is the only way back to it.
        e.downcast_ref::<std::io::Error>()
            .and_then(|io| io.get_ref())
            .is_some_and(|inner| inner.is::<rustls::Error>())
    })
}

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
    tls: Option<&TlsDial>,
) -> Result<Vec<u8>> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let mut forwarder = pods
        .portforward(&backend.pod, &[backend.port])
        .await
        .with_context(|| format!("port-forwarding {}:{}", backend.pod, backend.port))?;

    let stream = forwarder
        .take_stream(backend.port)
        .ok_or_else(|| anyhow!("port-forward to {} yielded no stream", backend.pod))?;

    match tls {
        None => speak_http(TokioIo::new(stream), backend, path, headers).await,
        Some(dial) => {
            let name = ServerName::try_from(dial.server_name.clone()).with_context(|| {
                format!("{:?} is not a usable TLS server name", dial.server_name)
            })?;
            let stream = TlsConnector::from(Arc::clone(&dial.config))
                .connect(name, stream)
                .await
                .map_err(|err| anyhow::Error::new(err).context(HandshakeFailed))
                .with_context(|| {
                    format!(
                        "TLS handshake with {} on port {}, asserting the name {:?}. \
                         `CertificateRequired` here means the listener requires a client \
                         certificate and this client presented none; `UnknownIssuer` means the \
                         server's certificate did not chain to the CA in the Secret the suite \
                         read; `NotValidForName` means the SAN ladder does not cover that name.",
                        backend.pod, backend.port, dial.server_name,
                    )
                })?;
            speak_http(TokioIo::new(stream), backend, path, headers).await
        }
    }
}

/// One request over an already-established stream, plaintext or TLS alike.
///
/// Generic over the stream so the TLS branch above is the only thing that knows
/// TLS happened: everything from the HTTP handshake down is identical.
async fn speak_http<S>(
    io: S,
    backend: &Backend,
    path: &str,
    headers: &[(String, String)],
) -> Result<Vec<u8>>
where
    S: hyper::rt::Read + hyper::rt::Write + Unpin + Send + 'static,
{
    let (mut sender, connection) = hyper::client::conn::http1::handshake(io)
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
