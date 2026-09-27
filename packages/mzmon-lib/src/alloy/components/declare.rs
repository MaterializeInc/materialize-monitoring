// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Custom components: `declare`, its `argument`s, and instances of it.
//!
//! A `declare` block defines a component whose body is ordinary pipeline
//! blocks, parameterised by `argument` blocks read as `argument.<name>.value`.
//! It is how a pre-rendered, schema-validated pipeline can hold the *shape* of
//! something whose *count* is decided at install: the body is fixed here, and
//! whatever renders the install (the chart) only instantiates it, once per
//! resource.
//!
//! A body cannot repeat or omit a block based on an argument. Anything that
//! varies in block structure stays with the caller, and an argument is only
//! ever a value.
//!
//! See: https://grafana.com/docs/alloy/latest/reference/config-blocks/declare/

use crate::alloy::ast::{AttributeValue, Block, Identifier, ToBlock};
use crate::alloy::error::Result;
use crate::alloy::pipeline::ComponentBlock;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A `declare "<name>"` block — defines the custom component `<name>`.
///
/// The body is a list of ordinary blocks plus `argument` blocks, and may
/// instantiate other custom components declared in the same file.
///
/// See: https://grafana.com/docs/alloy/latest/reference/config-blocks/declare/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclareBlock {
    /// The custom component's name. Required, and an identifier without dots,
    /// which is what keeps it apart from every built-in component name.
    pub label: Identifier,
    /// The body: `argument` blocks and ordinary components.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<ComponentBlock>,
}

impl ToBlock for DeclareBlock {
    fn to_block(&self) -> Result<Block> {
        Ok(Block {
            component: "declare".into(),
            label: Some(self.label.clone()),
            attributes: IndexMap::new(),
            blocks: self
                .blocks
                .iter()
                .map(ToBlock::to_block)
                .collect::<Result<Vec<_>>>()?,
        })
    }
}

/// An `argument "<name>"` block inside a `declare` body — one input of the
/// custom component, read in the body as `argument.<name>.value`.
///
/// See: https://grafana.com/docs/alloy/latest/reference/config-blocks/argument/
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArgumentBlock {
    /// The argument's name. Required.
    pub label: Identifier,
    /// Whether an instance may omit it. Defaults to false upstream.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub optional: Option<bool>,
    /// The value used when an optional argument is omitted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<AttributeValue>,
    /// What the argument is for, shown in Alloy's UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

impl ToBlock for ArgumentBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(v) = self.optional {
            attributes.insert("optional".into(), AttributeValue::Bool(v));
        }
        if let Some(v) = &self.default {
            attributes.insert("default".into(), v.clone());
        }
        if let Some(v) = &self.comment {
            attributes.insert("comment".into(), AttributeValue::String(v.clone()));
        }
        Ok(Block {
            component: "argument".into(),
            label: Some(self.label.clone()),
            attributes,
            blocks: Vec::new(),
        })
    }
}

/// An instance of a custom component: `<component> "<label>" { ... }`.
///
/// Unlike `raw:`, the component must be a custom one — an identifier with no
/// dots — so this cannot be used to emit an untyped built-in. Whether it names
/// a `declare` that exists, and whether the attributes match its arguments, is
/// checked by `alloy validate`, which resolves both.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomBlock {
    /// The `declare` label being instantiated.
    pub component: Identifier,
    /// The instance's label. Required: every instance is a component.
    pub label: Identifier,
    /// Argument values, by argument name.
    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
    pub attributes: IndexMap<Identifier, AttributeValue>,
}

impl ToBlock for CustomBlock {
    fn to_block(&self) -> Result<Block> {
        Ok(Block {
            component: self.component.clone(),
            label: Some(self.label.clone()),
            attributes: self.attributes.clone(),
            blocks: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::alloy::error::Error;
    use crate::alloy::pipeline::Pipeline;
    use crate::alloy::test_support::assert_renders;

    fn schema_violation_paths(err: Error) -> Vec<String> {
        match err {
            Error::Multiple(errs) => errs
                .iter()
                .filter_map(|e| match e {
                    Error::Schema { path, .. } => Some(path.clone()),
                    _ => None,
                })
                .collect(),
            other => panic!("expected Multiple([Schema, ...]), got {other:?}"),
        }
    }

    /// A declare carries its arguments and body in YAML order, and an argument
    /// read in the body renders as a bare reference.
    #[test]
    fn declare_with_arguments_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - declare:
                  label: pull
                  blocks:
                    - argument:
                        label: instance
                        comment: The instance label.
                    - argument:
                        label: interval
                        optional: true
                        default: 5m
                    - prometheus.echo:
                        label: out
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "declare \"pull\" {\n",
                "\targument \"instance\" {\n",
                "\t\tcomment = \"The instance label.\"\n",
                "\t}\n",
                "\n",
                "\targument \"interval\" {\n",
                "\t\toptional = true\n",
                "\t\tdefault  = \"5m\"\n",
                "\t}\n",
                "\n",
                "\tprometheus.echo \"out\" { }\n",
                "}\n",
            ),
        );
    }

    /// An instance is a flat block of arguments, which may be literals or
    /// references.
    #[test]
    fn custom_instance_round_trips() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - custom:
                  component: provider_cloudwatch_rds
                  label: rds_mz_db
                  attributes:
                    instance_id: mz-db
                    forward_to: {ref: otelcol.receiver.prometheus.inputBridge.receiver}
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "provider_cloudwatch_rds \"rds_mz_db\" {\n",
                "\tinstance_id = \"mz-db\"\n",
                "\tforward_to  = otelcol.receiver.prometheus.inputBridge.receiver\n",
                "}\n",
            ),
        );
    }

    /// `custom` must not become a second `raw:`. A dotted component name is a
    /// built-in, and belongs in its typed schema.
    #[test]
    fn custom_refuses_a_built_in_component() {
        let err = Pipeline::from_yaml_str(
            r#"
            blocks:
              - custom:
                  component: prometheus.scrape
                  label: sneaky
            "#,
        )
        .unwrap_err();
        assert!(
            schema_violation_paths(err)
                .iter()
                .any(|p| p.starts_with("/blocks/0"))
        );
    }

    /// An argument means nothing outside a declare body.
    #[test]
    fn argument_is_refused_at_top_level() {
        let err = Pipeline::from_yaml_str(
            r#"
            blocks:
              - argument:
                  label: stray
            "#,
        )
        .unwrap_err();
        assert!(
            schema_violation_paths(err)
                .iter()
                .any(|p| p.starts_with("/blocks/0"))
        );
    }

    /// A declare body holds typed components whose fields take arguments: a
    /// list member, a map value, and a scalar, each as a reference.
    #[test]
    fn typed_components_inside_a_declare_take_arguments() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - declare:
                  label: rds
                  blocks:
                    - argument: {label: region}
                    - argument: {label: instance_id}
                    - prometheus.exporter.cloudwatch:
                        label: rds
                        sts_region: {ref: argument.region.value}
                        blocks:
                          - static:
                              label: rds
                              regions: [{ref: argument.region.value}]
                              namespace: AWS/RDS
                              dimensions:
                                DBInstanceIdentifier: {ref: argument.instance_id.value}
            "#,
        )
        .unwrap();
        // Plain byte assert: alloy fmt aligns the scalars beside the multi-line
        // `regions` and `dimensions`, which the renderer does not.
        assert_eq!(
            pipeline.render().unwrap(),
            concat!(
                "declare \"rds\" {\n",
                "\targument \"region\" { }\n",
                "\n",
                "\targument \"instance_id\" { }\n",
                "\n",
                "\tprometheus.exporter.cloudwatch \"rds\" {\n",
                "\t\tsts_region = argument.region.value\n",
                "\n",
                "\t\tstatic \"rds\" {\n",
                "\t\t\tregions = [\n",
                "\t\t\t\targument.region.value,\n",
                "\t\t\t]\n",
                "\t\t\tnamespace = \"AWS/RDS\"\n",
                "\t\t\tdimensions = {\n",
                "\t\t\t\tDBInstanceIdentifier = argument.instance_id.value,\n",
                "\t\t\t}\n",
                "\t\t}\n",
                "\t}\n",
                "}\n",
            ),
        );
    }
}
