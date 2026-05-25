// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(String),

    #[error("formatter error")]
    Fmt(#[from] fmt::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
