// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! The assertions, one module per component.
//!
//! The rule that keeps this suite honest: **assert query success everywhere,
//! assert non-empty results only on self-monitoring series.** Materialize
//! scrapers stay off here — they are integration-tested downstream — so the only
//! data guaranteed to exist is what the stack produces about itself. Getting this
//! backwards yields either a suite that passes while blind, or one that flakes on
//! empty Materialize series forever.

pub mod alloy;
pub mod grafana;
pub mod loki;
pub mod thanos;
