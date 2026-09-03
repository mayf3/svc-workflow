//! Workflow graph validation engine.
//!
//! Validates a complete workflow graph before publication.
//! Rules are specified in the architecture document section 14.
#![allow(clippy::needless_borrow)]

mod assignee_validation;
mod minimal_validator;
mod transition_validation;
mod validation;
mod visit_activation_validator;

#[cfg(test)]
mod minimal_validator_tests;

#[cfg(test)]
mod visit_activation_validator_tests;

use std::collections::HashMap;

use super::error::GraphValidationError;
use super::model::{NodeDefinition, TransitionDefinition, ValidationResult, WorkflowGraph};
use crate::domain::ids::TransitionId;

/// Validate a complete workflow graph against all publication rules.
///
/// Returns a [`ValidationResult`] summarizing all errors, warnings,
/// and optionally a computed digest.
pub fn validate_graph(graph: &WorkflowGraph) -> ValidationResult {
    let mut errors: Vec<GraphValidationError> = Vec::new();
    let warnings: Vec<String> = Vec::new();

    // Build lookup maps
    let nodes_by_id: HashMap<_, &NodeDefinition> =
        graph.nodes.iter().map(|n| (n.node_id, n)).collect();
    let transitions_by_id: HashMap<TransitionId, &TransitionDefinition> = graph
        .transitions
        .iter()
        .map(|t| (t.transition_id, t))
        .collect();

    // ---
    // 14.1 Node rules
    // ---
    let (draft_nodes, _terminal_nodes) = validation::validate_node_rules(graph, &mut errors);

    // ---
    // H-2: Assignee rules
    // ---
    assignee_validation::validate_assignee_rules(graph, &nodes_by_id, &mut errors);

    // ---
    // H-2b: Instance-input assignee keys must be covered by context_schema.required
    // ---
    assignee_validation::validate_instance_input_schema_coverage(graph, &mut errors);

    // ---
    // Transition uniqueness + reference checks (14.5)
    // ---
    transition_validation::validate_transition_references(graph, &nodes_by_id, &mut errors);

    // ---
    // 14.2 Primary trunk rules
    // ---
    let (_primary_targets, _nodes_with_primary) = transition_validation::validate_primary_trunk(
        graph,
        &nodes_by_id,
        &transitions_by_id,
        &draft_nodes,
        &mut errors,
    );

    // 7. All nodes must be reachable from DRAFT (H-1)
    validation::validate_directed_reachability(graph, &draft_nodes, &nodes_by_id, &mut errors);

    // ---
    // 14.3 RETURN rules
    // ---
    transition_validation::validate_return_rules(graph, &nodes_by_id, &mut errors);

    // ---
    // 14.4 TERMINATE rules
    // ---
    transition_validation::validate_terminate_rules(graph, &nodes_by_id, &mut errors);

    // Terminal nodes must have no outgoing transitions
    transition_validation::validate_terminal_outgoing(graph, &mut errors);

    let valid = errors.is_empty();
    ValidationResult {
        valid,
        errors,
        warnings,
        computed_digest: None,
    }
}

/// Validate a workflow graph under the V2 Minimal semantic model.
///
/// Fully separated from [`validate_graph`] (V1 Legacy rules); shares only
/// the pure structural reference checks. See `minimal_validator` for the
/// frozen V2 contract.
pub fn validate_minimal_graph(graph: &WorkflowGraph) -> ValidationResult {
    minimal_validator::validate_minimal_graph(graph)
}

/// Validate a workflow graph under the VISIT_ACTIVATION_V1 (model 3)
/// semantic model. Fully separated from the Legacy and Minimal validators;
/// see `visit_activation_validator` for the frozen V1-new contract
/// (v0.4.0 §5.3 / CTR-ARCH-006).
pub fn validate_visit_activation_graph(graph: &WorkflowGraph) -> ValidationResult {
    visit_activation_validator::validate_visit_activation_graph(graph)
}
