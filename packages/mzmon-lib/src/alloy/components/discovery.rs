// Copyright Materialize, Inc. and contributors. All rights reserved.
//
// Use of this software is governed by the Business Source License
// included in the LICENSE file.
//
// As of the Change Date specified in that file, in accordance with
// the Business Source License, use of this software will be governed
// by the Apache License, Version 2.0.

//! Typed sugar for discovery.* components.
//!
//! Mirrors the per-component schemas in `schemas/alloy/discovery.schema.yaml`.
//! Each block deserializes from the flat `{discovery.X: {label, attrs..., blocks}}`
//! form and converts to a generic [`Block`] via [`ToBlock`].

use crate::alloy::ast::{
    AttributeValue, Block, Expression, Identifier, ToBlock, impl_to_block_dispatch,
};
use crate::alloy::components::relabel::RelabelSubBlock;
use crate::alloy::error::Result;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A loki/discovery component output exported as a `targets` or `receiver`
/// expression — rendered as a bare ref (e.g. `discovery.kubernetes.pods.targets`),
/// NOT as a quoted string.
type TargetRef = String;

// ============================================================
// discovery.kubernetes
// ============================================================

/// A `discovery.kubernetes` block — discovers Kubernetes targets by role.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/discovery/discovery.kubernetes/
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryKubernetesBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Kubernetes object type to discover (`pod`, `service`, `endpoints`, ...).
    /// Required by the schema.
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kubeconfig_file: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_server: Option<String>,
    /// Optional sub-blocks (namespaces, selectors, attach_metadata, ...).
    /// Currently only `raw:` is typed; typed sugar can be added as new
    /// `KubernetesSubBlock` variants when needed.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<KubernetesSubBlock>,
}

/// Sub-block under a `discovery.kubernetes` body.
///
/// Externally-tagged: the YAML key picks the variant. Typed variants are
/// `selectors` and `attach_metadata`; everything else (e.g. `namespaces`)
/// falls through to the `raw:` escape.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum KubernetesSubBlock {
    #[serde(rename = "selectors")]
    Selectors(DiscoveryKubernetesSelector),
    #[serde(rename = "attach_metadata")]
    AttachMetadata(DiscoveryKubernetesAttachMetadata),
    #[serde(rename = "raw")]
    Raw(Block),
}
impl_to_block_dispatch!(KubernetesSubBlock {
    Selectors,
    AttachMetadata,
    Raw
});

/// A `selectors` sub-block of `discovery.kubernetes` — filters discovered
/// Kubernetes objects by label / field selectors. Scoped to a single `role`;
/// multiple `selectors` blocks may appear in one `discovery.kubernetes`.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/discovery/discovery.kubernetes/#selectors-block
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryKubernetesSelector {
    /// Role this selector applies to. Required.
    pub role: String,
    /// Kubernetes label selector expression (e.g. `app=alloy`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Kubernetes field selector expression (e.g. `status.phase=Running`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

impl ToBlock for DiscoveryKubernetesSelector {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("role".into(), AttributeValue::String(self.role.clone()));
        if let Some(label) = &self.label {
            attributes.insert("label".into(), AttributeValue::String(label.clone()));
        }
        if let Some(field) = &self.field {
            attributes.insert("field".into(), AttributeValue::String(field.clone()));
        }
        Ok(Block {
            component: "selectors".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

/// An `attach_metadata` sub-block of `discovery.kubernetes` — controls whether
/// discovered targets carry metadata from related Kubernetes objects.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/discovery/discovery.kubernetes/#attach_metadata-block
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryKubernetesAttachMetadata {
    /// When true, attach metadata from the pod's host node to each target.
    /// Defaults to false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node: Option<bool>,
}

impl ToBlock for DiscoveryKubernetesAttachMetadata {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        if let Some(node) = self.node {
            attributes.insert("node".into(), AttributeValue::Bool(node));
        }
        Ok(Block {
            component: "attach_metadata".into(),
            label: None,
            attributes,
            blocks: Vec::new(),
        })
    }
}

impl ToBlock for DiscoveryKubernetesBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        attributes.insert("role".into(), AttributeValue::String(self.role.clone()));
        if let Some(kc) = &self.kubeconfig_file {
            attributes.insert("kubeconfig_file".into(), AttributeValue::String(kc.clone()));
        }
        if let Some(api) = &self.api_server {
            attributes.insert("api_server".into(), AttributeValue::String(api.clone()));
        }

        let mut blocks: Vec<Block> = Vec::with_capacity(self.blocks.len());
        for sb in &self.blocks {
            blocks.push(sb.to_block()?);
        }

        Ok(Block {
            component: "discovery.kubernetes".into(),
            label: self.label.clone(),
            attributes,
            blocks,
        })
    }
}

// ============================================================
// discovery.relabel  (+ shared `rule` types)
// ============================================================

/// A `discovery.relabel` block — rewrites discovery target labels via
/// `rule` sub-blocks before they are consumed by a source component.
///
/// See: https://grafana.com/docs/alloy/latest/reference/components/discovery/discovery.relabel/
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryRelabelBlock {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<Identifier>,
    /// Discovery targets to relabel; usually a reference to another
    /// `discovery.*` component's `.targets` export. Required by the schema.
    pub targets: Vec<TargetRef>,
    /// Rule blocks applied in document order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocks: Vec<RelabelSubBlock>,
}

impl ToBlock for DiscoveryRelabelBlock {
    fn to_block(&self) -> Result<Block> {
        let mut attributes = IndexMap::new();
        // `targets` is a list of refs (e.g. `discovery.kubernetes.pods.targets`),
        // not quoted strings — wrap each as an Expression::ref_name so the
        // renderer emits a bare identifier.
        attributes.insert(
            "targets".into(),
            AttributeValue::Array(
                self.targets
                    .iter()
                    .map(|s| {
                        AttributeValue::Expression(Expression {
                            ref_name: Some(s.clone()),
                            ..Default::default()
                        })
                    })
                    .collect(),
            ),
        );

        let mut blocks: Vec<Block> = Vec::with_capacity(self.blocks.len());
        for sb in &self.blocks {
            blocks.push(sb.to_block()?);
        }

        Ok(Block {
            component: "discovery.relabel".into(),
            label: self.label.clone(),
            attributes,
            blocks,
        })
    }
}

// ============================================================
// tests
// ============================================================

#[cfg(test)]
mod tests {
    use crate::alloy::pipeline::Pipeline;
    use crate::alloy::test_support::assert_renders;

    #[test]
    fn discovery_kubernetes_renders_role_only() {
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - discovery.kubernetes:
                  label: pods
                  role: pod
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "discovery.kubernetes \"pods\" {\n",
                "\trole = \"pod\"\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn discovery_kubernetes_with_kubeconfig_and_api_server_aligns() {
        // Three single-line string attributes — alignment to widest key
        // (`kubeconfig_file` at 15 chars).
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - discovery.kubernetes:
                  role: service
                  kubeconfig_file: "/etc/kubernetes/admin.conf"
                  api_server: "https://kube.example:6443"
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "discovery.kubernetes {\n",
                "\trole            = \"service\"\n",
                "\tkubeconfig_file = \"/etc/kubernetes/admin.conf\"\n",
                "\tapi_server      = \"https://kube.example:6443\"\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn discovery_relabel_emits_targets_as_bare_refs() {
        // Verifies the critical property: `targets` is an array of refs
        // (bare identifiers, NOT quoted strings).
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - discovery.relabel:
                  label: k8s_filtered
                  targets: ["discovery.kubernetes.pods.targets"]
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "discovery.relabel \"k8s_filtered\" {\n",
                "\ttargets = [\n",
                "\t\tdiscovery.kubernetes.pods.targets,\n",
                "\t]\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn discovery_relabel_with_single_attribute_rule_blocks() {
        // Each rule has only one attribute to dodge the renderer's
        // multi-line-disables-alignment quirk (see follow-up task).
        // The structural assertions still verify: rule blocks deserialize,
        // render as `rule { ... }`, and stack with the right blank-line
        // separators.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - discovery.relabel:
                  targets: ["discovery.kubernetes.pods.targets"]
                  blocks:
                    - rule:
                        action: keep
                    - rule:
                        target_label: pod
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "discovery.relabel {\n",
                "\ttargets = [\n",
                "\t\tdiscovery.kubernetes.pods.targets,\n",
                "\t]\n",
                "\n",
                "\trule {\n",
                "\t\taction = \"keep\"\n",
                "\t}\n",
                "\n",
                "\trule {\n",
                "\t\ttarget_label = \"pod\"\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn discovery_kubernetes_with_raw_sub_block() {
        // Exercises the raw escape inside discovery.kubernetes.blocks.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - discovery.kubernetes:
                  role: pod
                  blocks:
                    - raw:
                        component: namespaces
                        attributes:
                          names: ["mz-system", "mz-environment"]
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "discovery.kubernetes {\n",
                "\trole = \"pod\"\n",
                "\n",
                "\tnamespaces {\n",
                "\t\tnames = [\n",
                "\t\t\t\"mz-system\",\n",
                "\t\t\t\"mz-environment\",\n",
                "\t\t]\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn discovery_kubernetes_with_selectors_sub_block() {
        // Typed `selectors` sub-block: filters by label/field, scoped to a role.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - discovery.kubernetes:
                  role: pod
                  blocks:
                    - selectors:
                        role: pod
                        label: "app=alloy"
                        field: "status.phase=Running"
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "discovery.kubernetes {\n",
                "\trole = \"pod\"\n",
                "\n",
                "\tselectors {\n",
                "\t\trole  = \"pod\"\n",
                "\t\tlabel = \"app=alloy\"\n",
                "\t\tfield = \"status.phase=Running\"\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn discovery_kubernetes_with_attach_metadata_sub_block() {
        // Typed `attach_metadata` sub-block: attach metadata from the host node.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - discovery.kubernetes:
                  role: pod
                  blocks:
                    - attach_metadata:
                        node: true
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "discovery.kubernetes {\n",
                "\trole = \"pod\"\n",
                "\n",
                "\tattach_metadata {\n",
                "\t\tnode = true\n",
                "\t}\n",
                "}\n",
            ),
        );
    }

    #[test]
    fn discovery_kubernetes_mixed_typed_and_raw_sub_blocks() {
        // Confirms typed and raw sub-blocks compose in one body, in source order.
        let pipeline = Pipeline::from_yaml_str(
            r#"
            blocks:
              - discovery.kubernetes:
                  role: pod
                  blocks:
                    - selectors:
                        role: pod
                        label: "tier=backend"
                    - attach_metadata:
                        node: true
                    - raw:
                        component: namespaces
                        attributes:
                          own_namespace: true
            "#,
        )
        .unwrap();
        assert_renders(
            pipeline.render(),
            concat!(
                "discovery.kubernetes {\n",
                "\trole = \"pod\"\n",
                "\n",
                "\tselectors {\n",
                "\t\trole  = \"pod\"\n",
                "\t\tlabel = \"tier=backend\"\n",
                "\t}\n",
                "\n",
                "\tattach_metadata {\n",
                "\t\tnode = true\n",
                "\t}\n",
                "\n",
                "\tnamespaces {\n",
                "\t\town_namespace = true\n",
                "\t}\n",
                "}\n",
            ),
        );
    }
}
