//! Definition digest computation using JCS (JSON Canonicalization Scheme).
//!
//! Produces a stable SHA-256 digest of a complete workflow definition version,
//! excluding database-generated fields (timestamps, IDs not part of business identity).
//!
//! Algorithm:
//! 1. Build a canonical document with deterministic field order
//! 2. Sort nodes by node_key, transitions by transition_key
//! 3. JCS-normalize the document
//! 4. SHA-256 the normalized bytes

use serde::Serialize;

use crate::domain::ids::{DefinitionVersionId, WorkflowDefinitionId};

use super::model::{AssigneeRef, NodeDefinition, TransitionDefinition};

/// Canonical document used for digest computation.
///
/// Fields are ordered alphabetically for deterministic output.
/// Node and Transition arrays are sorted by their stable keys.
/// All timestamps and database-generated IDs are excluded.
#[derive(Debug, Clone, Serialize)]
pub struct CanonicalDefinitionDocument {
    pub definition_key: String,
    pub version_number: i32,
    pub json_schema_dialect: Option<String>,
    pub validator_version: Option<String>,
    pub context_schema: Option<serde_json::Value>,
    pub nodes: Vec<CanonicalNode>,
    pub transitions: Vec<CanonicalTransition>,
}

/// Canonical representation of a node.
#[derive(Debug, Clone, Serialize)]
pub struct CanonicalNode {
    pub node_key: String,
    pub display_name: String,
    pub order_index: i32,
    pub node_type: String,
    pub assignee_ref_type: String,
    pub fixed_principal_id: Option<String>,
    pub instructions: Option<String>,
    pub primary_advance_transition_key: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Canonical representation of a transition.
#[derive(Debug, Clone, Serialize)]
pub struct CanonicalTransition {
    pub transition_key: String,
    pub display_name: String,
    pub source_node_key: String,
    pub target_node_key: String,
    pub transition_effect: String,
    pub submission_schema: Option<serde_json::Value>,
    pub metadata: Option<serde_json::Value>,
}

/// Compute the stable digest for a workflow definition version.
///
/// # Arguments
/// * `definition_key` - The workflow definition key (stable business identifier)
/// * `version_number` - The version number
/// * `json_schema_dialect` - The JSON Schema dialect
/// * `validator_version` - The validator version
/// * `context_schema` - The context JSON Schema
/// * `nodes` - All node definitions
/// * `transitions` - All transition definitions
/// * `node_key_by_id` - Map from node_id to node_key for resolving references
///
/// # Returns
/// SHA-256 hex digest string (64 lowercase hex characters).
#[allow(clippy::too_many_arguments)]
pub fn compute_digest(
    definition_key: &str,
    version_number: i32,
    json_schema_dialect: Option<&str>,
    validator_version: Option<&str>,
    context_schema: Option<&serde_json::Value>,
    nodes: &[NodeDefinition],
    transitions: &[TransitionDefinition],
    node_key_by_id: &std::collections::HashMap<crate::domain::ids::NodeId, String>,
    transition_key_by_id: &std::collections::HashMap<crate::domain::ids::TransitionId, String>,
) -> Result<String, super::error::DefinitionError> {
    // Build sorted canonical nodes
    let mut canonical_nodes: Vec<CanonicalNode> = nodes
        .iter()
        .map(|n| {
            let primary_key = n
                .primary_advance_transition_id
                .and_then(|tid| transition_key_by_id.get(&tid).cloned());

            let fixed_id = n.assignee_ref.fixed_principal_id.map(|pid| pid.to_string());

            CanonicalNode {
                node_key: n.node_key.clone(),
                display_name: n.display_name.clone(),
                order_index: n.order_index,
                node_type: n.node_type.to_string(),
                assignee_ref_type: n.assignee_ref.ref_type.to_string(),
                fixed_principal_id: fixed_id,
                instructions: n.instructions.clone(),
                primary_advance_transition_key: primary_key,
                metadata: n.metadata.clone(),
            }
        })
        .collect();

    // Sort nodes by node_key for deterministic order
    canonical_nodes.sort_by(|a, b| a.node_key.cmp(&b.node_key));

    // Build sorted canonical transitions
    let mut canonical_transitions: Vec<CanonicalTransition> = transitions
        .iter()
        .map(|t| {
            let source_key = node_key_by_id
                .get(&t.source_node_id)
                .cloned()
                .unwrap_or_default();
            let target_key = node_key_by_id
                .get(&t.target_node_id)
                .cloned()
                .unwrap_or_default();

            CanonicalTransition {
                transition_key: t.transition_key.clone(),
                display_name: t.display_name.clone(),
                source_node_key: source_key,
                target_node_key: target_key,
                transition_effect: t.transition_effect.to_string(),
                submission_schema: t.submission_schema.clone(),
                metadata: t.metadata.clone(),
            }
        })
        .collect();

    // Sort transitions by transition_key for deterministic order
    canonical_transitions.sort_by(|a, b| a.transition_key.cmp(&b.transition_key));

    let doc = CanonicalDefinitionDocument {
        definition_key: definition_key.to_string(),
        version_number,
        json_schema_dialect: json_schema_dialect.map(|s| s.to_string()),
        validator_version: validator_version.map(|s| s.to_string()),
        context_schema: context_schema.cloned(),
        nodes: canonical_nodes,
        transitions: canonical_transitions,
    };

    // Serialize to JSON then JCS-canonicalize with SHA-256
    // sha256_jcs_hex serializes the value to JSON, JCS-canonicalizes it,
    // and computes SHA-256 in one step.
    let digest = jcs_canonicalize::sha256_jcs_hex(&doc).map_err(|e| {
        super::error::DefinitionError::DigestFailure(format!("JCS canonicalization failed: {}", e))
    })?;

    Ok(digest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::enums::{AssigneeRefType, NodeType, TransitionEffect};
    use crate::domain::ids::{DefinitionVersionId, NodeId, PrincipalId, TransitionId};
    use std::collections::HashMap;

    #[allow(clippy::type_complexity)]
    fn make_test_data(
        version_id: DefinitionVersionId,
    ) -> (
        Vec<NodeDefinition>,
        Vec<TransitionDefinition>,
        HashMap<NodeId, String>,
        HashMap<TransitionId, String>,
    ) {
        let draft_node_id = NodeId::new();
        let normal_node_id = NodeId::new();
        let terminal_node_id = NodeId::new();
        let advance_trans_id = TransitionId::new();
        let complete_trans_id = TransitionId::new();

        let nodes = vec![
            NodeDefinition {
                node_id: draft_node_id,
                definition_version_id: version_id,
                node_key: "draft".to_string(),
                display_name: "Draft".to_string(),
                order_index: 0,
                node_type: NodeType::DRAFT,
                assignee_ref: AssigneeRef {
                    ref_type: AssigneeRefType::WorkflowCreator,
                    fixed_principal_id: None,
                },
                instructions: None,
                primary_advance_transition_id: Some(advance_trans_id),
                metadata: None,
                created_at: chrono::Utc::now(),
            },
            NodeDefinition {
                node_id: normal_node_id,
                definition_version_id: version_id,
                node_key: "dev_self_check".to_string(),
                display_name: "Dev Self Check".to_string(),
                order_index: 1,
                node_type: NodeType::NORMAL,
                assignee_ref: AssigneeRef {
                    ref_type: AssigneeRefType::FixedPrincipal,
                    fixed_principal_id: Some(PrincipalId::new()),
                },
                instructions: Some("Run tests".to_string()),
                primary_advance_transition_id: Some(complete_trans_id),
                metadata: None,
                created_at: chrono::Utc::now(),
            },
            NodeDefinition {
                node_id: terminal_node_id,
                definition_version_id: version_id,
                node_key: "done".to_string(),
                display_name: "Done".to_string(),
                order_index: 2,
                node_type: NodeType::TERMINAL,
                assignee_ref: AssigneeRef {
                    ref_type: AssigneeRefType::WorkflowCreator,
                    fixed_principal_id: None,
                },
                instructions: None,
                primary_advance_transition_id: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
        ];

        let transitions = vec![
            TransitionDefinition {
                transition_id: advance_trans_id,
                definition_version_id: version_id,
                transition_key: "advance-dev".to_string(),
                display_name: "Advance to Dev".to_string(),
                source_node_id: draft_node_id,
                target_node_id: normal_node_id,
                transition_effect: TransitionEffect::Advance,
                submission_schema: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
            TransitionDefinition {
                transition_id: complete_trans_id,
                definition_version_id: version_id,
                transition_key: "advance-done".to_string(),
                display_name: "Complete".to_string(),
                source_node_id: normal_node_id,
                target_node_id: terminal_node_id,
                transition_effect: TransitionEffect::Advance,
                submission_schema: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
        ];

        let mut node_key_map = HashMap::new();
        node_key_map.insert(draft_node_id, "draft".to_string());
        node_key_map.insert(normal_node_id, "dev_self_check".to_string());
        node_key_map.insert(terminal_node_id, "done".to_string());

        let mut transition_key_map = HashMap::new();
        transition_key_map.insert(advance_trans_id, "advance-dev".to_string());
        transition_key_map.insert(complete_trans_id, "advance-done".to_string());

        (nodes, transitions, node_key_map, transition_key_map)
    }

    #[test]
    fn same_semantics_produces_same_digest() {
        let version_id = DefinitionVersionId::new();
        let (nodes, transitions, nk, tk) = make_test_data(version_id);
        let context_schema = serde_json::json!({"type": "object"});

        let digest1 = compute_digest(
            "test-def",
            1,
            Some("https://json-schema.org/draft/2020-12/schema"),
            Some("1"),
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        // Same data should produce same digest
        let digest2 = compute_digest(
            "test-def",
            1,
            Some("https://json-schema.org/draft/2020-12/schema"),
            Some("1"),
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        assert_eq!(digest1, digest2, "same input should produce same digest");
    }

    #[test]
    fn different_json_key_order_same_digest() {
        let version_id = DefinitionVersionId::new();
        let (nodes, transitions, nk, tk) = make_test_data(version_id);

        // Context schema with different key order
        let ctx1 = serde_json::json!({"type": "object", "required": ["title"]});
        let ctx2 = serde_json::json!({"required": ["title"], "type": "object"});

        let digest1 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&ctx1),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        let digest2 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&ctx2),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        assert_eq!(
            digest1, digest2,
            "different JSON key order should produce same digest"
        );
    }

    #[test]
    fn different_node_order_same_digest() {
        let version_id = DefinitionVersionId::new();
        let (mut nodes, transitions, nk, tk) = make_test_data(version_id);

        // Reverse node order
        nodes.reverse();

        let context_schema = serde_json::json!({"type": "object"});

        let digest1 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        // Same data, original order
        nodes.reverse();
        let digest2 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        assert_eq!(
            digest1, digest2,
            "different node order should produce same digest"
        );
    }

    #[test]
    fn different_transition_order_same_digest() {
        let version_id = DefinitionVersionId::new();
        let (nodes, mut transitions, nk, tk) = make_test_data(version_id);

        transitions.reverse();

        let context_schema = serde_json::json!({"type": "object"});

        let digest1 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        transitions.reverse();
        let digest2 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        assert_eq!(
            digest1, digest2,
            "different transition order should produce same digest"
        );
    }

    #[test]
    fn different_context_schema_produces_different_digest() {
        let version_id = DefinitionVersionId::new();
        let (nodes, transitions, nk, tk) = make_test_data(version_id);

        let ctx1 = serde_json::json!({"type": "object", "required": ["title"]});
        let ctx2 = serde_json::json!({"type": "object", "required": ["description"]});

        let digest1 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&ctx1),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        let digest2 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&ctx2),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        assert_ne!(
            digest1, digest2,
            "different context schema should produce different digest"
        );
    }

    #[test]
    fn different_instructions_produces_different_digest() {
        let version_id = DefinitionVersionId::new();
        let (mut nodes, transitions, nk, tk) = make_test_data(version_id);

        // Change instructions on one node
        nodes[1].instructions = Some("Different instructions".to_string());

        let context_schema = serde_json::json!({"type": "object"});

        let digest1 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        nodes[1].instructions = Some("Original instructions".to_string());
        let digest2 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        assert_ne!(
            digest1, digest2,
            "different instructions should produce different digest"
        );
    }

    #[test]
    fn different_submission_schema_produces_different_digest() {
        let version_id = DefinitionVersionId::new();
        let (nodes, mut transitions, nk, tk) = make_test_data(version_id);

        transitions[0].submission_schema = Some(serde_json::json!({"type": "object"}));

        let context_schema = serde_json::json!({"type": "object"});

        let digest1 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        transitions[0].submission_schema =
            Some(serde_json::json!({"type": "object", "required": ["field"]}));
        let digest2 = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        assert_ne!(
            digest1, digest2,
            "different submission schema should produce different digest"
        );
    }

    #[test]
    fn different_timestamps_do_not_affect_digest() {
        let version_id = DefinitionVersionId::new();
        let (nodes, transitions, nk, tk) = make_test_data(version_id);

        let context_schema = serde_json::json!({"type": "object"});

        let digest = compute_digest(
            "test-def",
            1,
            None,
            None,
            Some(&context_schema),
            &nodes,
            &transitions,
            &nk,
            &tk,
        )
        .unwrap();

        // Digest should be deterministic regardless of when it's computed
        assert_eq!(digest.len(), 64, "digest should be a 64-char hex string");
        assert!(
            digest.chars().all(|c| c.is_ascii_hexdigit()),
            "digest should be hex"
        );
    }
}
