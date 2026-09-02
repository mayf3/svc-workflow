//! Tests for the VISIT_ACTIVATION_V1 (semantic model 3) graph validator.
//!
//! Legality matrix for ACC-VAI-007 (SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1):
//! node kinds TASK | TERMINAL, one entry TASK, one primary ADVANCE per TASK,
//! acyclic deterministic primary path, RETURN to a strictly earlier TASK,
//! TERMINATE to a TERMINAL, closed owner set, reachability.

use uuid::Uuid;

use super::visit_activation_validator::validate_visit_activation_graph;
use crate::domain::definition::model::{
    AssigneeRef, NodeDefinition, TransitionDefinition, WorkflowGraph,
};
use crate::domain::enums::{AssigneeRefType, NodeType, TransitionEffect};
use crate::domain::ids::{DefinitionVersionId, NodeId, TransitionId};

fn node(
    id: u32,
    key: &str,
    node_type: NodeType,
    assignee: Option<AssigneeRef>,
    order_index: i32,
    primary: Option<u32>,
) -> NodeDefinition {
    NodeDefinition {
        node_id: NodeId::from_uuid(Uuid::from_u128(id as u128)),
        definition_version_id: DefinitionVersionId::from_uuid(Uuid::from_u128(999)),
        node_key: key.to_string(),
        display_name: key.to_string(),
        order_index,
        node_type,
        assignee_ref: assignee,
        instructions: None,
        primary_advance_transition_id: primary.map(|t| {
            TransitionId::from_uuid(Uuid::from_u128(t as u128))
        }),
        metadata: None,
        created_at: chrono::Utc::now(),
    }
}

fn transition(
    id: u32,
    key: &str,
    source: u32,
    target: u32,
    effect: TransitionEffect,
) -> TransitionDefinition {
    TransitionDefinition {
        transition_id: TransitionId::from_uuid(Uuid::from_u128(id as u128)),
        definition_version_id: DefinitionVersionId::from_uuid(Uuid::from_u128(999)),
        transition_key: key.to_string(),
        display_name: key.to_string(),
        source_node_id: NodeId::from_uuid(Uuid::from_u128(source as u128)),
        target_node_id: NodeId::from_uuid(Uuid::from_u128(target as u128)),
        transition_effect: effect,
        submission_schema: None,
        metadata: None,
        created_at: chrono::Utc::now(),
    }
}

fn graph(nodes: Vec<NodeDefinition>, transitions: Vec<TransitionDefinition>) -> WorkflowGraph {
    WorkflowGraph {
        nodes,
        transitions,
        context_schema: None,
    }
}

fn owner(ref_type: AssigneeRefType) -> Option<AssigneeRef> {
    Some(AssigneeRef {
        ref_type,
        fixed_principal_id: if ref_type == AssigneeRefType::FixedPrincipal {
            Some(crate::domain::ids::PrincipalId::from_uuid(
                Uuid::from_u128(0xDEAD),
            ))
        } else {
            None
        },
        assignee_input_key: None,
    })
}

fn terminal(id: u32, key: &str) -> NodeDefinition {
    node(id, key, NodeType::TERMINAL, None, 90, None)
}

fn error_codes(result: &crate::domain::definition::model::ValidationResult) -> Vec<String> {
    result.errors.iter().map(|e| e.code.clone()).collect()
}

/// Conformant graph: entry TASK -> primary ADVANCE -> TASK -> primary
/// ADVANCE -> TERMINAL, with a RETURN to the earlier TASK and a TERMINATE
/// to a second TERMINAL.
#[test]
fn v1_valid_graph_accepts() {
    let g = graph(
        vec![
            node(1, "start", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
            node(2, "work", NodeType::TASK, owner(AssigneeRefType::DomainOwner), 1, Some(11)),
            terminal(3, "done"),
            terminal(4, "failed"),
        ],
        vec![
            transition(10, "advance-1", 1, 2, TransitionEffect::Advance),
            transition(11, "advance-2", 2, 3, TransitionEffect::Advance),
            transition(12, "return", 2, 1, TransitionEffect::Return),
            transition(13, "terminate", 2, 4, TransitionEffect::Terminate),
        ],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(result.valid, "errors: {:?}", result.errors);
}

#[test]
fn v1_rejects_draft_and_normal_node_kinds() {
    for forbidden in [NodeType::DRAFT, NodeType::NORMAL] {
        let g = graph(
            vec![
                node(1, "start", forbidden, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
                terminal(3, "done"),
            ],
            vec![transition(10, "advance-1", 1, 3, TransitionEffect::Advance)],
        );
        let result = validate_visit_activation_graph(&g);
        assert!(!result.valid);
        assert!(error_codes(&result).contains(&"v1_node_kind_forbidden".to_string()));
    }
}

#[test]
fn v1_rejects_task_without_owner() {
    let g = graph(
        vec![
            node(1, "start", NodeType::TASK, None, 0, Some(10)),
            terminal(3, "done"),
        ],
        vec![transition(10, "advance-1", 1, 3, TransitionEffect::Advance)],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_task_owner_required".to_string()));
}

#[test]
fn v1_rejects_instance_input_principal_owner() {
    let g = graph(
        vec![
            node(
                1,
                "start",
                NodeType::TASK,
                owner(AssigneeRefType::InstanceInputPrincipal),
                0,
                Some(10),
            ),
            terminal(3, "done"),
        ],
        vec![transition(10, "advance-1", 1, 3, TransitionEffect::Advance)],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_owner_ref_forbidden".to_string()));
}

#[test]
fn v1_rejects_fixed_principal_without_id() {
    let g = graph(
        vec![
            node(
                1,
                "start",
                NodeType::TASK,
                Some(AssigneeRef {
                    ref_type: AssigneeRefType::FixedPrincipal,
                    fixed_principal_id: None,
                    assignee_input_key: None,
                }),
                0,
                Some(10),
            ),
            terminal(3, "done"),
        ],
        vec![transition(10, "advance-1", 1, 3, TransitionEffect::Advance)],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_fixed_principal_missing".to_string()));
}

#[test]
fn v1_rejects_task_without_primary_advance() {
    let g = graph(
        vec![
            node(1, "start", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, None),
            terminal(3, "done"),
        ],
        vec![],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_primary_advance_required".to_string()));
}

#[test]
fn v1_rejects_multiple_advance_edges() {
    let g = graph(
        vec![
            node(1, "start", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
            terminal(3, "done"),
            terminal(4, "other"),
        ],
        vec![
            transition(10, "advance-1", 1, 3, TransitionEffect::Advance),
            transition(11, "advance-2", 1, 4, TransitionEffect::Advance),
        ],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_multiple_advance_forbidden".to_string()));
}

#[test]
fn v1_rejects_two_entry_tasks() {
    let g = graph(
        vec![
            node(1, "a", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
            node(2, "b", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 1, Some(11)),
            terminal(3, "done"),
        ],
        vec![
            transition(10, "advance-1", 1, 3, TransitionEffect::Advance),
            transition(11, "advance-2", 2, 3, TransitionEffect::Advance),
        ],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_multiple_entry_tasks".to_string()));
}

#[test]
fn v1_rejects_return_to_later_task() {
    let g = graph(
        vec![
            node(1, "start", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
            node(2, "work", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 1, Some(11)),
            terminal(3, "done"),
        ],
        vec![
            transition(10, "advance-1", 1, 2, TransitionEffect::Advance),
            transition(11, "advance-2", 2, 3, TransitionEffect::Advance),
            transition(12, "return", 1, 2, TransitionEffect::Return),
        ],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_return_target_not_earlier".to_string()));
}

#[test]
fn v1_rejects_return_to_terminal() {
    let g = graph(
        vec![
            node(1, "start", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
            terminal(3, "done"),
        ],
        vec![
            transition(10, "advance-1", 1, 3, TransitionEffect::Advance),
            transition(12, "return", 1, 3, TransitionEffect::Return),
        ],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_return_target_not_task".to_string()));
}

#[test]
fn v1_rejects_terminate_to_task() {
    let g = graph(
        vec![
            node(1, "start", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
            node(2, "work", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 1, Some(11)),
            terminal(3, "done"),
        ],
        vec![
            transition(10, "advance-1", 1, 2, TransitionEffect::Advance),
            transition(11, "advance-2", 2, 3, TransitionEffect::Advance),
            transition(13, "terminate", 2, 1, TransitionEffect::Terminate),
        ],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_terminate_target_not_terminal".to_string()));
}

#[test]
fn v1_rejects_terminal_with_owner_or_outgoing() {
    // TERMINAL with an owner reference.
    let g = graph(
        vec![
            node(1, "start", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
            node(3, "done", NodeType::TERMINAL, owner(AssigneeRefType::WorkflowCreator), 1, None),
        ],
        vec![transition(10, "advance-1", 1, 3, TransitionEffect::Advance)],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_terminal_owner_forbidden".to_string()));

    // TERMINAL with an outgoing transition.
    let g2 = graph(
        vec![
            node(1, "start", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
            node(3, "done", NodeType::TERMINAL, None, 1, None),
            node(4, "after", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 2, Some(11)),
        ],
        vec![
            transition(10, "advance-1", 1, 3, TransitionEffect::Advance),
            transition(11, "outgoing", 3, 4, TransitionEffect::Advance),
        ],
    );
    let result2 = validate_visit_activation_graph(&g2);
    assert!(error_codes(&result2).contains(&"v1_terminal_outgoing_forbidden".to_string()));
}

#[test]
fn v1_rejects_unreachable_node() {
    // A TERMINAL node with no incoming edge is unreachable from the entry
    // TASK (a unique entry exists), so it must be reported.
    let g = graph(
        vec![
            node(1, "start", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
            terminal(3, "done"),
            terminal(4, "orphan-done"),
        ],
        vec![transition(10, "advance-1", 1, 3, TransitionEffect::Advance)],
    );
    let result = validate_visit_activation_graph(&g);
    assert!(error_codes(&result).contains(&"v1_unreachable_node".to_string()));
}

#[test]
fn v1_rejects_primary_path_cycle() {
    // Entry TASK a (primary -> b); b and c point their primary ADVANCE at
    // each other, so walking a's primary path revisits a node.
    let g = graph(
        vec![
            node(1, "a", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 0, Some(10)),
            node(2, "b", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 1, Some(11)),
            node(3, "c", NodeType::TASK, owner(AssigneeRefType::WorkflowCreator), 2, Some(12)),
        ],
        vec![
            transition(10, "advance-1", 1, 2, TransitionEffect::Advance),
            transition(11, "advance-2", 2, 3, TransitionEffect::Advance),
            transition(12, "advance-3", 3, 2, TransitionEffect::Advance),
        ],
    );
    let result = validate_visit_activation_graph(&g);
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"v1_primary_path_cycle".to_string()),
        "codes: {:?}",
        codes
    );
}
