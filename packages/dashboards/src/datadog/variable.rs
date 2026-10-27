// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Dashboard template variables.
//!
//! A Datadog template variable carries a tag `prefix`, and a query that names it
//! as `$name` gets `prefix:value` spliced in, or nothing when it is `*`. Each
//! variable here backs one registry parameter in [`super::queries`].

use datadog_api_client::datadogV1::model::DashboardTemplateVariable;

/// The environment picker's name, as queries reference it (`$environment`).
pub const ENVIRONMENT: &str = "environment";

/// The tag an environment is scoped by: the label the Grafana dashboards'
/// `%%{mzEnvironmentFilter}` matches.
// TODO: move to `environment_id` with the Grafana fragment, in DEP-354.
pub const ENVIRONMENT_TAG: &str = "materialize_cloud_organization_name";

/// The environment picker, defaulting to every environment.
pub fn environment() -> DashboardTemplateVariable {
    DashboardTemplateVariable::new(ENVIRONMENT.to_string())
        .prefix(Some(ENVIRONMENT_TAG.to_string()))
        .defaults(vec!["*".to_string()])
}
