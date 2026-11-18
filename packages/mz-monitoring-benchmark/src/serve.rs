// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The metrics endpoint.
//!
//! Uncompressed, like Materialize's own, which does not compress its responses
//! yet. Every scrape renders afresh on a blocking thread; nothing is cached,
//! because a second scraper should see the second's values, not the first's.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Result};
use bytes::Bytes;
use http::{Request, Response, StatusCode};
use http_body_util::Full;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::logs::unix_now;
use crate::model::Model;

pub async fn serve(addr: SocketAddr, model: Arc<Model>) -> Result<()> {
    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("binding {addr}"))?;
    loop {
        let (stream, _) = listener.accept().await?;
        let model = Arc::clone(&model);
        tokio::spawn(async move {
            let service = service_fn(move |req| handle(req, Arc::clone(&model)));
            let _ = http1::Builder::new()
                .serve_connection(TokioIo::new(stream), service)
                .await;
        });
    }
}

async fn handle(
    req: Request<Incoming>,
    model: Arc<Model>,
) -> Result<Response<Full<Bytes>>, std::convert::Infallible> {
    let path = req.uri().path().to_string();
    if path == "/healthz" {
        return Ok(text(StatusCode::OK, "ok\n".into()));
    }
    let body = tokio::task::spawn_blocking(move || model.render(&path, unix_now()))
        .await
        .ok()
        .flatten();
    Ok(match body {
        Some(body) => {
            let mut resp = text(StatusCode::OK, body);
            resp.headers_mut().insert(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("text/plain; version=0.0.4; charset=utf-8"),
            );
            resp
        }
        None => text(StatusCode::NOT_FOUND, "not found\n".into()),
    })
}

fn text(status: StatusCode, body: String) -> Response<Full<Bytes>> {
    let mut resp = Response::new(Full::new(Bytes::from(body)));
    *resp.status_mut() = status;
    resp
}
