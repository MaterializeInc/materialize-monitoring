// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! A thin authenticated GitHub REST/GraphQL client shared by the commands that
//! talk to GitHub (`propose-bumps`, `publish-release`).

use anyhow::Context;
use reqwest::Client;
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, HeaderValue};
use serde_json::Value;

const GITHUB_API: &str = "https://api.github.com";

/// An authenticated client over the GitHub API.
pub(crate) struct Gh {
    client: Client,
}

impl Gh {
    pub(crate) fn new(token: &str) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert(
            ACCEPT,
            HeaderValue::from_static("application/vnd.github+json"),
        );
        headers.insert(
            "X-GitHub-Api-Version",
            HeaderValue::from_static("2022-11-28"),
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}"))?,
        );
        let client = Client::builder()
            .user_agent("mz-monitoring-build")
            .default_headers(headers)
            .build()?;
        Ok(Self { client })
    }

    pub(crate) async fn get(&self, path: &str) -> anyhow::Result<Value> {
        json_ok(
            self.client
                .get(format!("{GITHUB_API}{path}"))
                .send()
                .await?,
        )
        .await
    }

    pub(crate) async fn post(&self, path: &str, body: &Value) -> anyhow::Result<Value> {
        json_ok(
            self.client
                .post(format!("{GITHUB_API}{path}"))
                .json(body)
                .send()
                .await?,
        )
        .await
    }

    /// PATCH; returns whether the request succeeded (used for ref force-update,
    /// which 404s when the branch does not exist yet).
    pub(crate) async fn patch_ok(&self, path: &str, body: &Value) -> anyhow::Result<bool> {
        let resp = self
            .client
            .patch(format!("{GITHUB_API}{path}"))
            .json(body)
            .send()
            .await?;
        Ok(resp.status().is_success())
    }

    /// GET that returns whether the resource exists (2xx) vs is absent (404),
    /// erroring only on other failures.
    pub(crate) async fn exists(&self, path: &str) -> anyhow::Result<bool> {
        let resp = self
            .client
            .get(format!("{GITHUB_API}{path}"))
            .send()
            .await?;
        let status = resp.status();
        if status.is_success() {
            return Ok(true);
        }
        if status == reqwest::StatusCode::NOT_FOUND {
            return Ok(false);
        }
        let text = resp.text().await.unwrap_or_default();
        anyhow::bail!("github {status}: {text}");
    }

    /// Upload a release asset. `upload_url` is the release's `upload_url`
    /// template (the `{?name,label}` suffix is stripped); the upload targets
    /// `uploads.github.com`.
    pub(crate) async fn upload_asset(
        &self,
        upload_url: &str,
        name: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> anyhow::Result<()> {
        let base = upload_url.split_once('{').map_or(upload_url, |(b, _)| b);
        let resp = self
            .client
            .post(format!("{base}?name={name}"))
            .header(reqwest::header::CONTENT_TYPE, content_type)
            .body(bytes)
            .send()
            .await?;
        json_ok(resp).await?;
        Ok(())
    }

    pub(crate) async fn graphql(&self, body: &Value) -> anyhow::Result<Value> {
        json_ok(
            self.client
                .post(format!("{GITHUB_API}/graphql"))
                .json(body)
                .send()
                .await?,
        )
        .await
    }
}

/// Parse a JSON response, turning non-2xx into an error with the body.
async fn json_ok(resp: reqwest::Response) -> anyhow::Result<Value> {
    let status = resp.status();
    let text = resp.text().await?;
    if !status.is_success() {
        anyhow::bail!("github {status}: {text}");
    }
    if text.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).with_context(|| format!("parsing github response: {text}"))
}
