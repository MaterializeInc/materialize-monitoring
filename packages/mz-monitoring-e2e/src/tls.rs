// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The suite's own TLS client, and the certificate material behind it.
//!
//! Without this the suite cannot talk to a stack it is meant to be testing: a
//! Loki serving TLS on 3100 answers a plaintext GET with
//! `400 Client sent an HTTP request to an HTTPS server`, so every direct query
//! fails for a reason that has nothing to do with the thing being asserted.
//!
//! **The material is read out of the cluster rather than generated.** A
//! self-issued certificate would prove only that the suite can talk to itself;
//! reading a cert-manager Secret means the suite trusts the same CA the
//! components trust and presents an identity the same CA signed, which is
//! exactly the client every hop is configured for. It also means the assertions
//! fail when the *issuance* is wrong, not just when the serving is.
//!
//! Two configurations are built from one Secret, because the interesting
//! assertions need both:
//!
//! * [`ClientTls::authenticated`] trusts the CA and presents the keypair. This
//!   is a peer inside the trust domain, and it is what phase 3 admits.
//! * [`ClientTls::anonymous`] trusts the CA and presents nothing. This is what
//!   phase 3 must refuse and phase 2 must still serve — the difference between
//!   the two phases is only visible with a client that can do both.

use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, pem::PemObject};
use rustls::{ClientConfig, RootCertStore};

use crate::cluster::Cluster;

/// Which component's identity to borrow, in preference order.
///
/// Any certificate from the internal CA is admitted by every hop — these
/// components authenticate, they do not authorize, so one CA-signed identity is
/// as good as another. The order is therefore about *legibility*: a handshake
/// logged by Loki reads better as the gateway, which really is its client, than
/// as some unrelated component that happened to sort first.
///
/// Matched as a suffix against the Secret name so it is independent of the
/// release name.
const IDENTITY_PREFERENCE: &[&str] = &["-alloy-gateway-tls", "-alloy-agent-tls", "-loki-tls"];

/// A certificate that is deliberately not usable as an identity here.
///
/// The external Grafana certificate comes from a different issuer — the whole
/// point of it — so its `ca.crt` is the wrong trust root for in-cluster hops and
/// its keypair is signed by a CA nothing internal trusts. Picking it up by
/// accident would fail every handshake with a chain error that reads like a
/// broken internal CA.
const EXTERNAL_SUFFIX: &str = "-grafana-external-tls";

/// Client TLS material read from one cert-manager Secret.
pub struct ClientTls {
    authenticated: Arc<ClientConfig>,
    anonymous: Arc<ClientConfig>,
    /// The Secret it came from, named in errors and in the run header so a
    /// handshake failure can be traced back to the material without guessing.
    source: String,
}

impl ClientTls {
    /// Read `ca.crt`, `tls.crt` and `tls.key` from a Secret and build both
    /// configurations.
    ///
    /// `secret` overrides the automatic choice; without it the namespace is
    /// searched for a cert-manager-issued Secret using [`IDENTITY_PREFERENCE`].
    pub async fn load(cluster: &Cluster, secret: Option<&str>) -> Result<Self> {
        let source = match secret {
            Some(name) => name.to_string(),
            None => choose_identity(cluster).await?,
        };

        let ca_pem = cluster
            .secret_value(&source, "ca.crt")
            .await
            .with_context(|| {
                format!(
                    "reading the trust root out of Secret {source}. If that Secret exists but has \
                     no `ca.crt`, the issuer behind it is not a CA issuer — ACME and self-signed \
                     *leaf* issuers do not populate that key, and there is then no trust root here \
                     for the suite to use."
                )
            })?;

        let mut roots = RootCertStore::empty();
        let mut added = 0usize;
        for cert in CertificateDer::pem_slice_iter(ca_pem.as_bytes()) {
            let cert = cert.with_context(|| format!("parsing ca.crt from Secret {source}"))?;
            roots.add(cert).with_context(|| {
                format!("adding ca.crt from Secret {source} to the trust store")
            })?;
            added += 1;
        }
        if added == 0 {
            bail!("Secret {source} has a ca.crt with no PEM certificates in it");
        }

        let cert_pem = cluster.secret_value(&source, "tls.crt").await?;
        let key_pem = cluster.secret_value(&source, "tls.key").await?;
        let chain: Vec<CertificateDer<'static>> =
            CertificateDer::pem_slice_iter(cert_pem.as_bytes())
                .collect::<std::result::Result<_, _>>()
                .with_context(|| format!("parsing tls.crt from Secret {source}"))?;
        let key = PrivateKeyDer::from_pem_slice(key_pem.as_bytes())
            .with_context(|| format!("parsing tls.key from Secret {source}"))?;

        // Cloned rather than shared: `with_client_auth_cert` consumes the store,
        // and the two configurations differ only in whether they present.
        let anonymous = ClientConfig::builder()
            .with_root_certificates(roots.clone())
            .with_no_client_auth();
        let authenticated = ClientConfig::builder()
            .with_root_certificates(roots)
            .with_client_auth_cert(chain, key)
            .with_context(|| {
                format!(
                    "building a client identity from Secret {source}. A rejected keypair here is \
                     usually a certificate the issuer did not give `client auth` extended key \
                     usage."
                )
            })?;

        Ok(Self {
            authenticated: Arc::new(authenticated),
            anonymous: Arc::new(anonymous),
            source,
        })
    }

    /// Trusts the internal CA and presents a certificate it signed.
    pub fn authenticated(&self) -> Arc<ClientConfig> {
        Arc::clone(&self.authenticated)
    }

    /// Trusts the internal CA and presents nothing.
    pub fn anonymous(&self) -> Arc<ClientConfig> {
        Arc::clone(&self.anonymous)
    }

    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Pick a cert-manager Secret to borrow an identity from.
async fn choose_identity(cluster: &Cluster) -> Result<String> {
    let names = cluster.issued_tls_secrets().await?;
    if names.is_empty() {
        bail!(
            "no cert-manager-issued TLS Secret in namespace {}, so the suite has nothing to trust \
             or present. Certificates are enabled in the release values, so either the \
             Certificates have not become Ready yet — tls::certificates_ready reports that — or \
             they landed in another namespace.",
            cluster.namespace()
        );
    }

    let usable: Vec<&String> = names
        .iter()
        .filter(|n| !n.ends_with(EXTERNAL_SUFFIX))
        .collect();

    for suffix in IDENTITY_PREFERENCE {
        if let Some(name) = usable.iter().find(|n| n.ends_with(suffix)) {
            return Ok((*name).clone());
        }
    }

    usable
        .first()
        .map(|n| (*n).clone())
        .ok_or_else(|| anyhow!("the only issued TLS Secrets are external ones: {names:?}"))
}
