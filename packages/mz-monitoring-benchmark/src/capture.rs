// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Live capture for `fixture-gen`: find one pod per role, scrape each of its
//! endpoints twice through a port-forward, and hand the bodies to the
//! anonymizer.
//!
//! The port-forward is spoken to directly with hyper, as in
//! `mz-monitoring-e2e`, so no local listener is bound and nothing is left
//! running. Only reads are issued: pods, one Secret, and GETs on the metrics
//! endpoints.

use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use base64::Engine as _;
use http_body_util::BodyExt;
use hyper_util::rt::TokioIo;
use k8s_openapi::api::core::v1::{Pod, Secret};
use kube::api::{Api, ListParams};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Client, Config};

use crate::anonymize::Capture;
use crate::logs::unix_now;
use crate::role::Role;

pub struct Options {
    pub context: Option<String>,
    /// The environment's namespace; required when the cluster runs several.
    pub namespace: Option<String>,
    /// `namespace/name` of the Secret holding the SQL exporter's basic-auth
    /// `username` and `password`.
    pub sql_auth_secret: String,
    pub window: Duration,
    pub roles: Vec<Role>,
}

pub struct RoleCapture {
    pub role: Role,
    pub pod: String,
    pub namespace: String,
    pub materialize: Option<String>,
    pub captures: Vec<Capture>,
}

struct Target {
    role: Role,
    namespace: String,
    pod: String,
    port: u16,
    image_tag: Option<String>,
}

pub async fn capture(opts: &Options) -> Result<Vec<RoleCapture>> {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .ok();
    let kubeconfig = Kubeconfig::read().context("reading kubeconfig")?;
    let config = Config::from_custom_kubeconfig(
        kubeconfig,
        &KubeConfigOptions {
            context: opts.context.clone(),
            ..KubeConfigOptions::default()
        },
    )
    .await
    .context("loading kube context")?;
    let client = Client::try_from(config).context("building kube client")?;

    let mut targets = Vec::new();
    for role in &opts.roles {
        targets.push(find(&client, *role, opts.namespace.as_deref()).await?);
    }

    let auth = if opts.roles.contains(&Role::Environmentd) {
        Some(sql_auth(&client, &opts.sql_auth_secret).await?)
    } else {
        None
    };

    // Every endpoint once, then every endpoint again, so the window between a
    // pair is the same for all of them.
    let mut firsts = Vec::new();
    for t in &targets {
        for path in t.role.paths() {
            let headers = header_for(path, auth.as_deref());
            let at = unix_now();
            let body = get(&client, t, path, headers.as_ref())
                .await
                .with_context(|| {
                    format!("scraping {} {path} on {}/{}", t.role, t.namespace, t.pod)
                })?;
            firsts.push((at, body));
        }
    }
    tokio::time::sleep(opts.window).await;

    let mut out = Vec::new();
    let mut firsts = firsts.into_iter();
    for t in targets {
        let mut captures = Vec::new();
        for path in t.role.paths() {
            let (at, first) = firsts.next().expect("one first scrape per path");
            let headers = header_for(path, auth.as_deref());
            let second = get(&client, &t, path, headers.as_ref()).await?;
            captures.push(Capture {
                path: (*path).to_string(),
                first,
                second: Some(second),
                window_secs: (unix_now() - at).max(1.0),
                at_unix_ms: at * 1000.0,
            });
        }
        out.push(RoleCapture {
            role: t.role,
            pod: t.pod,
            namespace: t.namespace,
            materialize: t.image_tag,
            captures,
        });
    }
    Ok(out)
}

fn header_for(path: &str, auth: Option<&str>) -> Option<(String, String)> {
    let sql = path.starts_with("/metrics/");
    auth.filter(|_| sql)
        .map(|a| ("Authorization".to_string(), format!("Basic {a}")))
}

async fn find(client: &Client, role: Role, namespace: Option<&str>) -> Result<Target> {
    let pods: Api<Pod> = match namespace.filter(|_| role.in_environment()) {
        Some(ns) => Api::namespaced(client.clone(), ns),
        None => Api::all(client.clone()),
    };
    let list = pods
        .list(&ListParams::default().labels(role.selector()))
        .await
        .with_context(|| format!("listing {role} pods"))?;
    let mut ready: Vec<&Pod> = list.items.iter().filter(|p| is_ready(p)).collect();
    // A user replica, not a system one: that is what `gen-load --role
    // clusterd` stands in for.
    if role == Role::Clusterd {
        let user: Vec<&Pod> = ready
            .iter()
            .copied()
            .filter(|p| {
                label(p, "cluster.environmentd.materialize.cloud/cluster-id")
                    .is_some_and(|c| c.starts_with('u'))
            })
            .collect();
        if !user.is_empty() {
            ready = user;
        }
    }
    ready.sort_by_key(|p| (p.metadata.namespace.clone(), p.metadata.name.clone()));
    let namespaces: std::collections::BTreeSet<_> = ready
        .iter()
        .filter_map(|p| p.metadata.namespace.clone())
        .collect();
    if role.in_environment() && namespaces.len() > 1 {
        bail!(
            "{role} pods are ready in several namespaces ({}); pick one with --namespace",
            namespaces.into_iter().collect::<Vec<_>>().join(", ")
        );
    }
    let pod = ready
        .first()
        .ok_or_else(|| anyhow!("no ready pod matches {} for {role}", role.selector()))?;
    let containers = pod
        .spec
        .as_ref()
        .map(|s| s.containers.as_slice())
        .unwrap_or_default();
    let port = containers
        .iter()
        .filter_map(|c| c.ports.as_ref())
        .flatten()
        .find(|p| p.name.as_deref() == Some(role.port_name()))
        .and_then(|p| u16::try_from(p.container_port).ok())
        .with_context(|| format!("{role} pod declares no port named {}", role.port_name()))?;
    let image_tag = containers
        .first()
        .and_then(|c| c.image.as_deref())
        .and_then(|i| i.rsplit_once(':'))
        .map(|(_, tag)| tag.to_string());
    Ok(Target {
        role,
        namespace: pod.metadata.namespace.clone().unwrap_or_default(),
        pod: pod.metadata.name.clone().unwrap_or_default(),
        port,
        image_tag: (role == Role::Environmentd).then_some(image_tag).flatten(),
    })
}

async fn sql_auth(client: &Client, secret: &str) -> Result<String> {
    let (ns, name) = secret
        .split_once('/')
        .with_context(|| format!("--sql-auth-secret wants namespace/name, got {secret:?}"))?;
    let secrets: Api<Secret> = Api::namespaced(client.clone(), ns);
    let s = secrets.get(name).await.with_context(|| {
        format!(
            "reading Secret {secret} for the SQL exporter endpoints' basic auth; \
             the chart creates it as monitoring/materialize-sql-monitor"
        )
    })?;
    let data = s.data.unwrap_or_default();
    let field = |k: &str| -> Result<String> {
        let v = data
            .get(k)
            .with_context(|| format!("Secret {secret} has no {k}"))?;
        Ok(String::from_utf8_lossy(&v.0).into_owned())
    };
    let pair = format!("{}:{}", field("username")?, field("password")?);
    Ok(base64::engine::general_purpose::STANDARD.encode(pair))
}

async fn get(
    client: &Client,
    t: &Target,
    path: &str,
    header: Option<&(String, String)>,
) -> Result<String> {
    let pods: Api<Pod> = Api::namespaced(client.clone(), &t.namespace);
    let mut forwarder = pods
        .portforward(&t.pod, &[t.port])
        .await
        .with_context(|| format!("port-forwarding {}:{}", t.pod, t.port))?;
    let stream = forwarder
        .take_stream(t.port)
        .ok_or_else(|| anyhow!("port-forward to {} yielded no stream", t.pod))?;
    let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
        .await
        .context("HTTP handshake over the forwarded stream")?;
    tokio::spawn(async move {
        let _ = connection.await;
    });
    let mut req = http::Request::get(path).header("Host", "localhost");
    if let Some((k, v)) = header {
        req = req.header(k, v);
    }
    let resp = sender
        .send_request(req.body(String::new())?)
        .await
        .with_context(|| format!("GET {path}"))?;
    let status = resp.status();
    let body = resp.into_body().collect().await?.to_bytes();
    if !status.is_success() {
        bail!("GET {path} returned {status}");
    }
    String::from_utf8(body.to_vec()).context("metrics body is not UTF-8")
}

fn is_ready(pod: &Pod) -> bool {
    pod.status
        .as_ref()
        .and_then(|s| s.conditions.as_ref())
        .is_some_and(|c| c.iter().any(|c| c.type_ == "Ready" && c.status == "True"))
}

fn label<'a>(pod: &'a Pod, key: &str) -> Option<&'a str> {
    pod.metadata.labels.as_ref()?.get(key).map(String::as_str)
}
