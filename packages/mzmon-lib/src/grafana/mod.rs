//! Grafana dashboard models.
//!
//! [`generated`] holds the generated types -- one module per upstream schema document,
//! produced by `bin/gen-grafana-models.sh` from the vendored draft-07 schemas
//! under `schemas/grafana/`. Do not edit those; regenerate them.
//!
//! Two shapes of the upstream schemas leak into any code built on them, and are
//! worth knowing before reading further:
//!
//! * Panel options are **not** typed in the dashboard document.
//!   `dashboardv2::VizConfigSpec::options` is a free-form map, so a typed
//!   [`generated::timeseries::Options`] has to be serialized into it. The typed value
//!   and the plugin id (`generated::TIMESERIES_PLUGIN_ID`, which fills
//!   `VizConfigKind::group`) travel together but are joined only by convention.
//!
//! * The `*Kind` unions carry no JSON Schema `discriminator`, even though every
//!   variant has a `kind` field with a `const` value. The generated enums are
//!   therefore `#[serde(untagged)]`: exact on the write path, but deserialization
//!   failures report only "data did not match any variant".
pub mod context;
pub mod generated;
pub mod layout;
pub mod panel;
pub mod query;
