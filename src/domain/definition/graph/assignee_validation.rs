//! H-2: Assignee rules for workflow graph validation.
//!
//! Enforces contract §3.1.7: Terminal nodes have no assignee.
//! Enforces contract §3.1.8: Non-terminal nodes must have a legal assignee reference.
//!
//! | Node type    | Assignee rule                                           |
//! |--------------|--------------------------------------------------------|
//! | TERMINAL     | No assignee reference                                     |
//! | DRAFT        | ref_type == WORKFLOW_CREATOR; no fixed_principal_id     |
//! | NORMAL       | Depends on ref_type per contract                        |

use std::collections::HashMap;

use crate::domain::definition::error::GraphValidationError;
use crate::domain::definition::model::{NodeDefinition, WorkflowGraph};
use crate::domain::enums::{AssigneeRefType, NodeType};
use crate::domain::ids::NodeId;

/// Validate assignee rules for all nodes in the graph.
///
/// Appends errors to the provided vector.
pub(super) fn validate_assignee_rules(
    graph: &WorkflowGraph,
    _nodes_by_id: &HashMap<NodeId, &NodeDefinition>,
    errors: &mut Vec<GraphValidationError>,
) {
    for node in &graph.nodes {
        match node.node_type {
            NodeType::TERMINAL => {
                if node.assignee_ref.is_some() {
                    errors.push(GraphValidationError::new(
                        "TERMINAL_HAS_ASSIGNEE",
                        format!(
                            "terminal node '{}' must not have an assignee reference",
                            node.node_key
                        ),
                    ));
                }
            }
            NodeType::DRAFT => {
                let Some(assignee_ref) = &node.assignee_ref else {
                    errors.push(GraphValidationError::new(
                        "ASSIGNEE_REQUIRED",
                        format!("DRAFT node '{}' requires an assignee", node.node_key),
                    ));
                    continue;
                };
                if assignee_ref.ref_type != AssigneeRefType::WorkflowCreator {
                    errors.push(GraphValidationError::new(
                        "DRAFT_NOT_WORKFLOW_CREATOR",
                        format!(
                            "DRAFT node '{}' has assignee {:?}, expected WORKFLOW_CREATOR",
                            node.node_key, assignee_ref.ref_type
                        ),
                    ));
                }
                if assignee_ref.fixed_principal_id.is_some() {
                    errors.push(GraphValidationError::new(
                        "UNEXPECTED_FIXED_PRINCIPAL",
                        format!(
                            "DRAFT node '{}' has fixed_principal_id but assignee type is {:?}",
                            node.node_key, assignee_ref.ref_type
                        ),
                    ));
                }
                if assignee_ref.assignee_input_key.is_some() {
                    errors.push(GraphValidationError::new(
                        "UNEXPECTED_ASSIGNEE_INPUT_KEY",
                        format!(
                            "DRAFT node '{}' has assignee_input_key but DRAFT must be WORKFLOW_CREATOR",
                            node.node_key
                        ),
                    ));
                }
            }
            NodeType::NORMAL => {
                let Some(assignee_ref) = &node.assignee_ref else {
                    errors.push(GraphValidationError::new(
                        "ASSIGNEE_REQUIRED",
                        format!("NORMAL node '{}' requires an assignee", node.node_key),
                    ));
                    continue;
                };
                match assignee_ref.ref_type {
                    AssigneeRefType::WorkflowCreator => {
                        if assignee_ref.fixed_principal_id.is_some() {
                            errors.push(GraphValidationError::new(
                                "UNEXPECTED_FIXED_PRINCIPAL",
                                format!(
                                "NORMAL node '{}' is WORKFLOW_CREATOR but has fixed_principal_id",
                                node.node_key
                            ),
                            ));
                        }
                        if assignee_ref.assignee_input_key.is_some() {
                            errors.push(GraphValidationError::new(
                                "UNEXPECTED_ASSIGNEE_INPUT_KEY",
                                format!(
                                    "NORMAL node '{}' is WORKFLOW_CREATOR but has assignee_input_key",
                                    node.node_key
                                ),
                            ));
                        }
                    }
                    AssigneeRefType::DomainOwner => {
                        if assignee_ref.fixed_principal_id.is_some() {
                            errors.push(GraphValidationError::new(
                                "UNEXPECTED_FIXED_PRINCIPAL",
                                format!(
                                    "NORMAL node '{}' is DOMAIN_OWNER but has fixed_principal_id",
                                    node.node_key
                                ),
                            ));
                        }
                        if assignee_ref.assignee_input_key.is_some() {
                            errors.push(GraphValidationError::new(
                                "UNEXPECTED_ASSIGNEE_INPUT_KEY",
                                format!(
                                    "NORMAL node '{}' is DOMAIN_OWNER but has assignee_input_key",
                                    node.node_key
                                ),
                            ));
                        }
                    }
                    AssigneeRefType::FixedPrincipal => {
                        if assignee_ref.fixed_principal_id.is_none() {
                            errors.push(GraphValidationError::new(
                                "FIXED_PRINCIPAL_MISSING_ID",
                                format!(
                                "NORMAL node '{}' is FIXED_PRINCIPAL but no principal_id provided",
                                node.node_key
                            ),
                            ));
                        }
                        if assignee_ref.assignee_input_key.is_some() {
                            errors.push(GraphValidationError::new(
                                "UNEXPECTED_ASSIGNEE_INPUT_KEY",
                                format!(
                                    "NORMAL node '{}' is FIXED_PRINCIPAL but has assignee_input_key",
                                    node.node_key
                                ),
                            ));
                        }
                    }
                    AssigneeRefType::InstanceInputPrincipal => {
                        if assignee_ref.fixed_principal_id.is_some() {
                            errors.push(GraphValidationError::new(
                                "UNEXPECTED_FIXED_PRINCIPAL",
                                format!(
                                    "NORMAL node '{}' is INSTANCE_INPUT_PRINCIPAL but has fixed_principal_id",
                                    node.node_key
                                ),
                            ));
                        }
                        match &assignee_ref.assignee_input_key {
                            None => {
                                errors.push(GraphValidationError::new(
                                    "INSTANCE_INPUT_PRINCIPAL_MISSING_KEY",
                                    format!(
                                        "NORMAL node '{}' is INSTANCE_INPUT_PRINCIPAL but no assignee_input_key provided",
                                        node.node_key
                                    ),
                                ));
                            }
                            Some(key) if !is_valid_input_key(key) => {
                                errors.push(GraphValidationError::new(
                                    "INSTANCE_INPUT_PRINCIPAL_INVALID_KEY",
                                    format!(
                                        "NORMAL node '{}' has invalid assignee_input_key '{}': must match ^[A-Za-z_][A-Za-z0-9_]*$ (1-128 chars)",
                                        node.node_key, key
                                    ),
                                ));
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}

/// Validate that every INSTANCE_INPUT_PRINCIPAL assignee key is covered by
/// `context_schema.required`.
///
/// A node that resolves its assignee from the instance context is a hard
/// dependency of every read/transition path: if the schema does not require
/// the key, a half-legal instance could be created and fail later with an
/// internal consistency error. The definition must therefore be
/// self-consistent already at draft/publish time. Keys are derived
/// generically from the graph's real node definitions (never hardcoded).
pub(super) fn validate_instance_input_schema_coverage(
    graph: &WorkflowGraph,
    errors: &mut Vec<GraphValidationError>,
) {
    let input_keys: Vec<(&str, &str)> = graph
        .nodes
        .iter()
        .filter_map(|node| {
            if node.node_type != NodeType::NORMAL {
                return None;
            }
            let assignee_ref = node.assignee_ref.as_ref()?;
            if assignee_ref.ref_type != AssigneeRefType::InstanceInputPrincipal {
                return None;
            }
            Some((
                node.node_key.as_str(),
                assignee_ref.assignee_input_key.as_deref()?,
            ))
        })
        .collect();

    if input_keys.is_empty() {
        return;
    }

    let Some(schema) = &graph.context_schema else {
        for (node_key, input_key) in &input_keys {
            errors.push(GraphValidationError::new(
                "CONTEXT_SCHEMA_REQUIRED_FOR_INSTANCE_INPUT",
                format!(
                    "NORMAL node '{}' is INSTANCE_INPUT_PRINCIPAL (key '{}') but the graph declares \
                     no context_schema; context_schema.required must cover every assignee_input_key",
                    node_key, input_key
                ),
            ));
        }
        return;
    };

    let required_keys: Vec<&str> = schema
        .get("required")
        .and_then(|required| required.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
        .unwrap_or_default();

    for (node_key, input_key) in &input_keys {
        if !required_keys.contains(input_key) {
            errors.push(GraphValidationError::new(
                "CONTEXT_SCHEMA_REQUIRED_MISSING_ASSIGNEE_KEY",
                format!(
                    "NORMAL node '{}' is INSTANCE_INPUT_PRINCIPAL but context_schema.required \
                     does not include '{}'",
                    node_key, input_key
                ),
            ));
        }
    }
}

/// Validate that an assignee input key is a safe JSON object property name.
///
/// Mirrors the database CHECK constraint: ASCII identifier starting with a
/// letter or underscore, 1-128 chars. This rejects path traversal-style keys
/// and keeps the key a flat, single-level lookup into the context payload.
fn is_valid_input_key(key: &str) -> bool {
    let len = key.len();
    if !(1..=128).contains(&len) {
        return false;
    }
    let mut chars = key.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
