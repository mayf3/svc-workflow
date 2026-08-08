//! Tests for the V2 Minimal semantic model validator.
//!
//! V2 decides LEGALITY only; no runtime behavior is tested here.

use uuid::Uuid;

use super::minimal_validator::validate_minimal_graph;
use crate::domain::definition::model::{
    AssigneeRef, NodeDefinition, TransitionDefinition, WorkflowGraph,
};
use crate::domain::enums::{AssigneeRefType, NodeType, TransitionEffect};
use crate::domain::ids::{DefinitionVersionId, NodeId, TransitionId};

fn node(id: u32, key: &str, node_type: NodeType, assignee: Option<AssigneeRef>) -> NodeDefinition {
    NodeDefinition {
        node_id: NodeId::from_uuid(Uuid::from_u128(id as u128)),
        definition_version_id: DefinitionVersionId::from_uuid(Uuid::from_u128(999)),
        node_key: key.to_string(),
        display_name: key.to_string(),
        order_index: 0,
        node_type,
        assignee_ref: assignee,
        instructions: None,
        primary_advance_transition_id: None,
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

fn creator() -> Option<AssigneeRef> {
    Some(AssigneeRef {
        ref_type: AssigneeRefType::WorkflowCreator,
        fixed_principal_id: None,
        assignee_input_key: None,
    })
}

fn fixed(principal: &str) -> Option<AssigneeRef> {
    Some(AssigneeRef {
        ref_type: AssigneeRefType::FixedPrincipal,
        fixed_principal_id: Some(
            crate::domain::ids::PrincipalId::from_uuid(
                Uuid::parse_str(principal).unwrap(),
            ),
        ),
        assignee_input_key: None,
    })
}

fn context_principal(key: &str) -> Option<AssigneeRef> {
    Some(AssigneeRef {
        ref_type: AssigneeRefType::InstanceInputPrincipal,
        fixed_principal_id: None,
        assignee_input_key: Some(key.to_string()),
    })
}

fn domain_owner() -> Option<AssigneeRef> {
    Some(AssigneeRef {
        ref_type: AssigneeRefType::DomainOwner,
        fixed_principal_id: None,
        assignee_input_key: None,
    })
}

fn error_codes(result: &GraphValidationResult) -> Vec<String> {
    result
        .errors
        .iter()
        .map(|e| e.code.clone())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

type GraphValidationResult = crate::domain::definition::model::ValidationResult;

// Legal V2 definitions
// ---------------------------------------------------------------------------

#[test]
fn single_task_to_terminal_is_legal() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    let result = validate_minimal_graph(&g);
    assert!(result.valid, "expected legal V2 graph: {:?}", error_codes(&result));
}

#[test]
fn creator_assignee_is_legal() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    assert!(validate_minimal_graph(&g).valid);
}

#[test]
fn fixed_principal_assignee_is_legal() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, fixed("10000000-0000-0000-0000-000000000001")),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    assert!(validate_minimal_graph(&g).valid);
}

#[test]
fn context_principal_single_segment_is_legal() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, context_principal("reviewer")),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    assert!(validate_minimal_graph(&g).valid);
}

#[test]
fn multi_outgoing_advance_branch_is_legal() {
    // review -> approved -> published (TERMINAL)
    //        -> rejected  -> archived (TERMINAL)
    let g = graph(
        vec![
            node(1, "review", NodeType::NORMAL, creator()),
            node(2, "approved", NodeType::NORMAL, fixed("10000000-0000-0000-0000-000000000002")),
            node(3, "rejected", NodeType::NORMAL, fixed("10000000-0000-0000-0000-000000000003")),
            node(4, "published", NodeType::TERMINAL, None),
            node(5, "archived", NodeType::TERMINAL, None),
        ],
        vec![
            transition(1, "approve", 1, 2, TransitionEffect::Advance),
            transition(2, "reject", 1, 3, TransitionEffect::Advance),
            transition(3, "to_published", 2, 4, TransitionEffect::Advance),
            transition(4, "to_archived", 3, 5, TransitionEffect::Advance),
        ],
    );
    let result = validate_minimal_graph(&g);
    assert!(
        result.valid,
        "multi-outgoing ADVANCE branch must be legal: {:?}",
        error_codes(&result)
    );
}

#[test]
fn return_to_strict_advance_ancestor_is_legal() {
    // start -> work -> review; review RETURN start / RETURN work
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "work", NodeType::NORMAL, creator()),
            node(3, "review", NodeType::NORMAL, creator()),
            node(4, "done", NodeType::TERMINAL, None),
        ],
        vec![
            transition(1, "start_to_work", 1, 2, TransitionEffect::Advance),
            transition(2, "work_to_review", 2, 3, TransitionEffect::Advance),
            transition(3, "back_to_start", 3, 1, TransitionEffect::Return),
            transition(4, "back_to_work", 3, 2, TransitionEffect::Return),
            transition(5, "review_to_done", 3, 4, TransitionEffect::Advance),
        ],
    );
    let result = validate_minimal_graph(&g);
    assert!(
        result.valid,
        "RETURN to strict ADVANCE ancestor must be legal: {:?}",
        error_codes(&result)
    );
}

#[test]
fn order_index_is_irrelevant_to_v2_legality() {
    // Two graphs differing only in order_index must validate identically.
    let mut a = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    a.nodes[0].order_index = 0;
    a.nodes[1].order_index = 1;

    let mut b = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    b.nodes[0].order_index = 42;
    b.nodes[1].order_index = -7;

    assert_eq!(
        validate_minimal_graph(&a).valid,
        validate_minimal_graph(&b).valid,
        "V2 legality must not depend on orderIndex"
    );
}

// ---------------------------------------------------------------------------
// Illegal V2 definitions
// ---------------------------------------------------------------------------

#[test]
fn draft_node_type_rejected() {
    let g = graph(
        vec![
            node(1, "legacy_draft", NodeType::DRAFT, creator()),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_node_draft_forbidden".to_string()));
}

#[test]
fn domain_owner_assignee_rejected() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, domain_owner()),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_assignee_domain_owner_forbidden".to_string()));
}

#[test]
fn primary_advance_transition_rejected() {
    let mut g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "next", NodeType::NORMAL, creator()),
            node(3, "done", NodeType::TERMINAL, None),
        ],
        vec![
            transition(1, "a", 1, 2, TransitionEffect::Advance),
            transition(2, "b", 2, 3, TransitionEffect::Advance),
        ],
    );
    g.nodes[0].primary_advance_transition_id = Some(g.transitions[0].transition_id);
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_primary_advance_forbidden".to_string()));
}

#[test]
fn two_entry_tasks_rejected() {
    let g = graph(
        vec![
            node(1, "entry_a", NodeType::NORMAL, creator()),
            node(2, "entry_b", NodeType::NORMAL, creator()),
            node(3, "done", NodeType::TERMINAL, None),
        ],
        vec![
            transition(1, "a", 1, 3, TransitionEffect::Advance),
            transition(2, "b", 2, 3, TransitionEffect::Advance),
        ],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_multiple_entry_tasks".to_string()));
}

#[test]
fn no_entry_task_rejected() {
    // Only a TERMINAL node with no TASK at all.
    let g = graph(
        vec![node(1, "done", NodeType::TERMINAL, None)],
        vec![],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_entry_task_required".to_string()));
}

#[test]
fn advance_cycle_rejected() {
    let g = graph(
        vec![
            node(1, "a", NodeType::NORMAL, creator()),
            node(2, "b", NodeType::NORMAL, creator()),
            node(3, "done", NodeType::TERMINAL, None),
        ],
        vec![
            transition(1, "a_to_b", 1, 2, TransitionEffect::Advance),
            transition(2, "b_to_a", 2, 1, TransitionEffect::Advance),
            transition(3, "a_to_done", 1, 3, TransitionEffect::Advance),
        ],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_advance_cycle".to_string()));
}

#[test]
fn isolated_task_rejected() {
    // 'orphan' has no edges at all: it is not reachable from entry, and it
    // surfaces as a second ADVANCE root (entry candidates = entry + orphan).
    // With a single entry, every TASK is mathematically reachable, so an
    // isolated TASK is rejected as an extra entry root.
    let g = graph(
        vec![
            node(1, "entry", NodeType::NORMAL, creator()),
            node(2, "done", NodeType::TERMINAL, None),
            node(3, "orphan", NodeType::NORMAL, creator()),
        ],
        vec![transition(1, "entry_to_done", 1, 2, TransitionEffect::Advance)],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid, "isolated TASK must be rejected");
    let codes = error_codes(&result);
    assert!(
        codes.contains(&"v2_multiple_entry_tasks".to_string()),
        "isolated TASK must be flagged as an extra entry root: {codes:?}"
    );
}

#[test]
fn return_to_descendant_rejected() {
    // start -> work; work RETURN start is legal, but start RETURN work is not.
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "work", NodeType::NORMAL, creator()),
            node(3, "done", NodeType::TERMINAL, None),
        ],
        vec![
            transition(1, "start_to_work", 1, 2, TransitionEffect::Advance),
            transition(2, "work_to_done", 2, 3, TransitionEffect::Advance),
            transition(3, "start_return_work", 1, 2, TransitionEffect::Return),
        ],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_return_target_not_strict_ancestor".to_string()));
}

#[test]
fn return_to_self_rejected() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![
            transition(1, "start_to_done", 1, 2, TransitionEffect::Advance),
            transition(2, "start_return_self", 1, 1, TransitionEffect::Return),
        ],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_return_target_not_strict_ancestor".to_string()));
}

#[test]
fn multi_segment_context_principal_path_rejected() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, context_principal("a.b")),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_context_principal_key_must_be_single_segment".to_string()));
}

#[test]
fn context_principal_without_key_rejected() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, context_principal("")),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_context_principal_key_must_be_single_segment".to_string()));
}

#[test]
fn task_without_assignee_rejected() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, None),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_task_assignee_required".to_string()));
}

#[test]
fn terminal_with_assignee_rejected() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "done", NodeType::TERMINAL, creator()),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_terminal_assignee_forbidden".to_string()));
}

#[test]
fn terminate_to_non_terminal_rejected() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "mid", NodeType::NORMAL, creator()),
            node(3, "done", NodeType::TERMINAL, None),
        ],
        vec![
            transition(1, "start_to_mid", 1, 2, TransitionEffect::Advance),
            transition(2, "terminate_to_mid", 1, 2, TransitionEffect::Terminate),
            transition(3, "mid_to_done", 2, 3, TransitionEffect::Advance),
        ],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_terminate_target_not_terminal".to_string()));
}

#[test]
fn terminal_with_outgoing_rejected() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, creator()),
            node(2, "done", NodeType::TERMINAL, None),
            node(3, "after", NodeType::NORMAL, creator()),
        ],
        vec![
            transition(1, "start_to_done", 1, 2, TransitionEffect::Advance),
            transition(2, "done_to_after", 2, 3, TransitionEffect::Advance),
        ],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_terminal_outgoing_forbidden".to_string()));
}

#[test]
fn fixed_principal_without_id_rejected() {
    let g = graph(
        vec![
            node(1, "start", NodeType::NORMAL, Some(AssigneeRef {
                ref_type: AssigneeRefType::FixedPrincipal,
                fixed_principal_id: None,
                assignee_input_key: None,
            })),
            node(2, "done", NodeType::TERMINAL, None),
        ],
        vec![transition(1, "finish", 1, 2, TransitionEffect::Advance)],
    );
    let result = validate_minimal_graph(&g);
    assert!(!result.valid);
    assert!(error_codes(&result).contains(&"v2_fixed_principal_missing".to_string()));
}
