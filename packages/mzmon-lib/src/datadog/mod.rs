// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Building blocks for Datadog dashboards, on the models in [`datadog_api_client`].
//!
//! The Datadog counterpart of [`crate::grafana`]: what any dashboard built from
//! the query registry needs, whichever dashboard it is. A registry query renders
//! in the [`context`], [`query`] splits it into the shape a widget request
//! carries, through [`formula`], and [`widget`] draws it. [`variable`] holds the
//! template variables the context's parameters name.
//!
//! Behind the `datadog` feature, because `datadog-api-client`'s models are a
//! long compile that the tools with no Datadog output shouldn't pay for.

pub mod context;
pub mod formula;
pub mod query;
pub mod variable;
pub mod widget;
