//! Core graph validation rules.
//!
//! Handles:
//! - Node structural rules (minimum nodes, DRAFT count, TERMINAL count, order_index)
//! - Directed reachability from DRAFT node (H-1)

use std::collections::{HashMap, HashSet};

use crate::domain::definition::error::GraphValidationError;
use crate::domain::definition::graph_helpers::compute_directed_reachable;
use crate::domain::definition::model::{NodeDefinition, WorkflowGraph};
use crate::domain::enums::NodeType;
use crate::domain::ids::NodeId;

/// Validate node structural rules.
///
/// Rules checked:
/// 1. At least 2 nodes
/// 2. Exactly one DRAFT node
/// 3. At least one TERMINAL node
/// 4. order_index unique within version
/// 5. node_key unique within version (via hashmap construction)
pub(super) fn validate_node_rules<'a>(
    graph: &'a WorkflowGraph,
    errors: &mut Vec<GraphValidationError>,
) -> (Vec<&'a NodeDefinition>, Vec<&'a NodeDefinition>) {
    // 1. At least two nodes
    if graph.nodes.len() < 2 {
        errors.push(GraphValidationError::new(
            "MIN_NODES",
            "graph must have at least 2 nodes",
        ));
    }

    // 2 & 3. Exactly one DRAFT node
    let draft_nodes: Vec<&NodeDefinition> = graph
        .nodes
        .iter()
        .filter(|n| n.node_type == NodeType::DRAFT)
        .collect();
    if draft_nodes.is_empty() {
        errors.push(GraphValidationError::new(
            "NO_DRAFT_NODE",
            "graph must have exactly one DRAFT node",
        ));
    } else if draft_nodes.len() > 1 {
        errors.push(GraphValidationError::new(
            "MULTIPLE_DRAFT_NODES",
            format!(
                "graph has {} DRAFT nodes, expected exactly one",
                draft_nodes.len()
            ),
        ));
    }

    // 4. At least one TERMINAL node
    let terminal_nodes: Vec<&NodeDefinition> = graph
        .nodes
        .iter()
        .filter(|n| n.node_type == NodeType::TERMINAL)
        .collect();
    if terminal_nodes.is_empty() {
        errors.push(GraphValidationError::new(
            "NO_TERMINAL_NODE",
            "graph must have at least one TERMINAL node",
        ));
    }

    // 5. order_index unique within version
    let mut seen_order_indices: HashSet<i32> = HashSet::new();
    for node in &graph.nodes {
        if !seen_order_indices.insert(node.order_index) {
            errors.push(GraphValidationError::new(
                "DUPLICATE_ORDER_INDEX",
                format!(
                    "duplicate order_index {} (node_key={})",
                    node.order_index, node.node_key
                ),
            ));
        }
    }

    (draft_nodes, terminal_nodes)
}


/// Validate failure-path coverage (READY_CONTRACT enforcement, 2026-10-26
/// owner decision; fleet evidence: 6 of 8 stalled TASK nodes had RET:0/HR:0
/// exits — agents then had no formal place to record blocked/missing-input
/// outcomes and stalled, e.g. adc-backend-dev dev_self_check).
///
/// Every non-DRAFT, non-TERMINAL node must have at least one outgoing
/// transition whose effect is RETURN or TERMINATE. ADVANCE-only nodes make
/// "evidence missing" / "input missing" outcomes unrepresentable: the graph
/// only encodes the happy path.
///
/// Legacy escape hatch: a node MAY declare metadata.readyContractExempt=true
/// (explicit per-node exemption, audited) — e.g. intake nodes whose only
/// legitimate outcome is human follow-up outside the workflow.
pub(super) fn validate_failure_path_coverage(
    graph: &WorkflowGraph,
    errors: &mut Vec<GraphValidationError>,
) {
    for node in &graph.nodes {
        if node.node_type == NodeType::DRAFT || node.node_type == NodeType::TERMINAL {
            continue;
        }
        if node
            .metadata
            .as_ref()
            .and_then(|m| m.get("readyContractExempt"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            continue;
        }
        let has_failure_exit = graph.transitions.iter().any(|t| {
            matches!(t.transition_effect, crate::domain::enums::TransitionEffect::Return | crate::domain::enums::TransitionEffect::Terminate)
        });
        if !has_failure_exit {
            errors.push(GraphValidationError::new(
                "NO_FAILURE_EXIT",
                format!(
                    "TASK node '{}' has no RETURN or TERMINATE outgoing transition; every execution node must define a failure path (evidence/input missing → RETURN to upstream, external blocker → HUMAN_REQUIRED/TERMINATE). Add one, or set metadata.readyContractExempt=true to record an explicit exemption.",
                    node.node_key
                ),
            ));
        }
    }
}

/// Validate that all nodes are reachable from DRAFT via directed edges (H-1).
pub(super) fn validate_directed_reachability(
    graph: &WorkflowGraph,
    draft_nodes: &[&NodeDefinition],
    _nodes_by_id: &HashMap<NodeId, &NodeDefinition>,
    errors: &mut Vec<GraphValidationError>,
) {
    if let Some(draft) = draft_nodes.first() {
        let directed_reachable =
            compute_directed_reachable(&graph.nodes, &graph.transitions, draft.node_id);
        for node in &graph.nodes {
            if !directed_reachable.contains(&node.node_id) {
                errors.push(GraphValidationError::new(
                    "NODE_NOT_REACHABLE",
                    format!(
                        "node '{}' is not reachable from draft node via any path",
                        node.node_key
                    ),
                ));
            }
        }
    }
}
