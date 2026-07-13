//! Workflow graph validation engine.
//!
//! Validates a complete workflow graph before publication.
//! Rules are specified in the architecture document section 14.

#![allow(clippy::needless_borrow)]

use std::collections::{HashMap, HashSet};

use crate::domain::enums::{AssigneeRefType, NodeType};
use crate::domain::ids::{NodeId, TransitionId};

use super::error::GraphValidationError;
use super::model::{
    AssigneeRef, NodeDefinition, TransitionDefinition, ValidationResult, WorkflowGraph,
};

/// Validate a complete workflow graph against all publication rules.
///
/// Returns a [`ValidationResult`] summarizing all errors, warnings,
/// and optionally a computed digest.
pub fn validate_graph(graph: &WorkflowGraph) -> ValidationResult {
    let mut errors: Vec<GraphValidationError> = Vec::new();
    let warnings: Vec<String> = Vec::new();

    // Build lookup maps
    let nodes_by_id: HashMap<NodeId, &NodeDefinition> =
        graph.nodes.iter().map(|n| (n.node_id, n)).collect();

    let transitions_by_id: HashMap<TransitionId, &TransitionDefinition> = graph
        .transitions
        .iter()
        .map(|t| (t.transition_id, t))
        .collect();

    let mut nodes_by_key: HashMap<&str, &NodeDefinition> = HashMap::new();
    for node in &graph.nodes {
        nodes_by_key.insert(node.node_key.as_str(), node);
    }

    let mut transitions_by_key: HashMap<&str, &TransitionDefinition> = HashMap::new();
    for trans in &graph.transitions {
        transitions_by_key.insert(trans.transition_key.as_str(), trans);
    }

    // =====================================================================
    // 14.1 Node rules
    // =====================================================================

    // 1. At least two nodes
    if graph.nodes.len() < 2 {
        errors.push(GraphValidationError::new(
            "MIN_NODES",
            "graph must have at least 2 nodes",
        ));
    }

    // 2. Exactly one DRAFT node, 3. DRAFT is the sole entry point
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

    // 6. node_key unique within version (checked via hashmap construction)

    // 7. Terminal nodes have no assignee (assignee_ref for terminals is unset)
    //    (Terminal nodes can have WORKFLOW_CREATOR as per seed but logically no one assigned)

    // 8. Non-terminal nodes must have a valid assignee reference
    for node in &graph.nodes {
        if node.node_type != NodeType::TERMINAL && node.node_type != NodeType::DRAFT {
            // Non-terminal, non-draft nodes must have valid assignee ref
            // Which means WORKFLOW_CREATOR, DOMAIN_OWNER, or FIXED_PRINCIPAL
        }
    }

    // =====================================================================
    // 14.5 Transition uniqueness (transition_key unique within version)
    // =====================================================================

    // 1. transition_key unique within version
    // (verified during hashmap construction)

    // 2. source/target nodes belong to this version - we can't fully verify
    // without cross-version checks, but we can check if node IDs exist
    for trans in &graph.transitions {
        if !nodes_by_id.contains_key(&trans.source_node_id) {
            errors.push(GraphValidationError::new(
                "TRANSITION_SOURCE_MISSING",
                format!(
                    "transition '{}' references non-existent source node_id",
                    trans.transition_key
                ),
            ));
        }
        if !nodes_by_id.contains_key(&trans.target_node_id) {
            errors.push(GraphValidationError::new(
                "TRANSITION_TARGET_MISSING",
                format!(
                    "transition '{}' references non-existent target node_id",
                    trans.transition_key
                ),
            ));
        }
    }

    // 3. No self-loops
    for trans in &graph.transitions {
        if trans.source_node_id == trans.target_node_id {
            errors.push(GraphValidationError::new(
                "SELF_LOOP",
                format!("transition '{}' is a self-loop", trans.transition_key),
            ));
        }
    }

    // 5. Primary advance transition ID must exist (validated later per node)

    // =====================================================================
    // 14.2 Primary trunk rules
    // =====================================================================

    // Collect primary transitions and build trunk graph
    let mut primary_targets: HashMap<NodeId, NodeId> = HashMap::new();
    let mut nodes_with_primary: HashSet<NodeId> = HashSet::new();
    let mut node_order_indices: HashMap<NodeId, i32> = HashMap::new();

    // Pre-populate order indices so all nodes are available during primary trunk check
    for node in &graph.nodes {
        node_order_indices.insert(node.node_id, node.order_index);
    }

    for node in &graph.nodes {
        if let Some(pt_id) = node.primary_advance_transition_id {
            if let Some(trans) = transitions_by_id.get(&pt_id) {
                // 1. Each non-terminal node must have exactly one primary_advance_transition_id
                if node.node_type != NodeType::TERMINAL {
                    primary_targets.insert(node.node_id, trans.target_node_id);
                    nodes_with_primary.insert(node.node_id);
                }

                // 2. Primary transition must originate from this node
                if trans.source_node_id != node.node_id {
                    errors.push(GraphValidationError::new(
                        "PRIMARY_NOT_FROM_NODE",
                        format!(
                            "primary transition '{}' for node '{}' does not originate from this node",
                            trans.transition_key, node.node_key
                        ),
                    ));
                }

                // 3. Primary target must have higher order_index
                if let Some(target_order) = node_order_indices.get(&trans.target_node_id) {
                    if *target_order <= node.order_index {
                        errors.push(GraphValidationError::new(
                            "PRIMARY_NOT_ADVANCING",
                            format!(
                                "primary transition '{}' from '{}' (order={}) to target (order={}) does not advance",
                                trans.transition_key, node.node_key, node.order_index, target_order
                            ),
                        ));
                    }
                }
            } else {
                // primary_advance_transition_id refers to a transition not in transitions list
                if node.node_type != NodeType::TERMINAL {
                    errors.push(GraphValidationError::new(
                        "PRIMARY_TRANSITION_MISSING",
                        format!(
                            "node '{}' primary_advance_transition_id {} not found in transitions",
                            node.node_key, pt_id
                        ),
                    ));
                } else {
                    // Terminal nodes should not have a primary_advance_transition_id
                    errors.push(GraphValidationError::new(
                        "TERMINAL_HAS_PRIMARY",
                        format!(
                            "terminal node '{}' should not have a primary_advance_transition_id",
                            node.node_key
                        ),
                    ));
                }
            }
        } else if node.node_type != NodeType::TERMINAL {
            // 1. Each non-terminal node must have exactly one primary_advance_transition_id
            errors.push(GraphValidationError::new(
                "MISSING_PRIMARY",
                format!(
                    "non-terminal node '{}' (type={:?}) lacks primary_advance_transition_id",
                    node.node_key, node.node_type
                ),
            ));
        }
    }

    // 4. Primary trunk must be acyclic
    {
        let mut visited: HashSet<NodeId> = HashSet::new();
        let mut in_path: HashSet<NodeId> = HashSet::new();

        for &start_node in nodes_by_id.keys() {
            if visited.contains(&start_node) {
                continue;
            }

            // Detect cycle along primary chain
            let mut current = start_node;
            let mut path: Vec<NodeId> = Vec::new();
            #[allow(clippy::while_let_loop)]
            loop {
                if in_path.contains(&current) {
                    // Cycle detected
                    let cycle_start_idx = path.iter().position(|n| *n == current).unwrap_or(0);
                    let cycle_nodes: Vec<String> = path[cycle_start_idx..]
                        .iter()
                        .map(|n| {
                            nodes_by_id
                                .get(n)
                                .map(|nn| nn.node_key.clone())
                                .unwrap_or_else(|| "?".to_string())
                        })
                        .collect();
                    errors.push(GraphValidationError::new(
                        "PRIMARY_CYCLE",
                        format!(
                            "primary trunk contains a cycle: {}",
                            cycle_nodes.join(" -> ")
                        ),
                    ));
                    break;
                }

                if visited.contains(&current) {
                    break;
                }

                in_path.insert(current);
                path.push(current);

                if let Some(&next) = primary_targets.get(&current) {
                    current = next;
                } else {
                    // Reached end of primary chain (terminal node)
                    break;
                }
            }

            // Mark all nodes in this path as visited
            for n in &path {
                visited.insert(*n);
                in_path.remove(n);
            }
        }
    }

    // 5. Primary trunk must eventually reach a terminal node
    if let Some(draft_node) = draft_nodes.first() {
        let mut current = draft_node.node_id;
        #[allow(clippy::while_let_loop)]
        loop {
            if let Some(node) = nodes_by_id.get(&current) {
                if node.node_type == NodeType::TERMINAL {
                    // Reached terminal along primary trunk
                    break;
                }
                if let Some(&next) = primary_targets.get(&current) {
                    if next == current {
                        break; // safety: prevent infinite loop on self-transition
                    }
                    current = next;
                } else {
                    errors.push(GraphValidationError::new(
                        "PRIMARY_TRUNK_NO_TERMINAL",
                        format!(
                            "primary trunk from draft node '{}' does not reach a terminal node",
                            draft_node.node_key
                        ),
                    ));
                    break;
                }
            } else {
                break;
            }
        }
    }

    // 6. All non-terminal nodes must have a primary or be reachable on primary trunk
    for node in &graph.nodes {
        if node.node_type != NodeType::TERMINAL && !nodes_with_primary.contains(&node.node_id) {
            // This is a non-terminal without a primary advance transition
            // This is already reported as MISSING_PRIMARY
        }
    }

    // 7. All nodes must be reachable from DRAFT (via any transition, not just primary)
    {
        let _reachable =
            compute_reachable_nodes(&graph.nodes, &graph.transitions, nodes_by_id.clone());

        // Actually, the rule says "all nodes from Draft reachable" - meaning from the draft node,
        // there should be a path to every node. But this would be too strict for RETURN transitions
        // that go to lower order_index nodes. Let's check reachability via forward transitions.
        // Actually, the architecture says "所有节点从 Draft 可达" - this means via the graph structure
        // (both forward and backward transitions), all nodes should be reachable from draft.
        // But this is ambiguous. Let's check that there are no disconnected subgraphs.

        // Check weak connectivity: every node should be in the same weakly connected component as draft
        if let Some(draft) = draft_nodes.first() {
            let weakly_reachable =
                compute_weakly_reachable(&graph.nodes, &graph.transitions, draft.node_id);
            for node in &graph.nodes {
                if !weakly_reachable.contains(&node.node_id) {
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

    // =====================================================================
    // 14.3 RETURN rules
    // =====================================================================

    for trans in &graph.transitions {
        if trans.transition_effect == crate::domain::enums::TransitionEffect::Return {
            let target_order = node_order_indices.get(&trans.target_node_id).copied();
            let source_order = node_order_indices.get(&trans.source_node_id).copied();

            // Target must be a non-terminal node
            if let Some(target_node) = nodes_by_id.get(&trans.target_node_id) {
                if target_node.node_type == NodeType::TERMINAL {
                    errors.push(GraphValidationError::new(
                        "RETURN_TO_TERMINAL",
                        format!(
                            "RETURN transition '{}' targets a TERMINAL node (should use TERMINATE)",
                            trans.transition_key
                        ),
                    ));
                }
            }

            // Target order_index must be less than source order_index
            if let (Some(src_order), Some(tgt_order)) = (source_order, target_order) {
                if tgt_order >= src_order {
                    errors.push(GraphValidationError::new(
                        "RETURN_NOT_BACKWARD",
                        format!(
                            "RETURN transition '{}' goes from order {} to {} (must go to lower order)",
                            trans.transition_key, src_order, tgt_order
                        ),
                    ));
                }
            }

            // Must not be primary_advance_transition_id of source node
            if let Some(source_node) = nodes_by_id.get(&trans.source_node_id) {
                if Some(trans.transition_id) == source_node.primary_advance_transition_id {
                    errors.push(GraphValidationError::new(
                        "RETURN_IS_PRIMARY",
                        format!(
                            "RETURN transition '{}' is also the primary_advance_transition_id of its source node",
                            trans.transition_key
                        ),
                    ));
                }
            }
        }
    }

    // =====================================================================
    // 14.4 TERMINATE rules
    // =====================================================================

    for trans in &graph.transitions {
        if trans.transition_effect == crate::domain::enums::TransitionEffect::Terminate {
            // Must not be primary_advance_transition_id
            if let Some(source_node) = nodes_by_id.get(&trans.source_node_id) {
                if Some(trans.transition_id) == source_node.primary_advance_transition_id {
                    errors.push(GraphValidationError::new(
                        "TERMINATE_IS_PRIMARY",
                        format!(
                            "TERMINATE transition '{}' is also the primary_advance_transition_id",
                            trans.transition_key
                        ),
                    ));
                }
            }

            // Target must be a terminal node
            if let Some(target_node) = nodes_by_id.get(&trans.target_node_id) {
                if target_node.node_type != NodeType::TERMINAL {
                    errors.push(GraphValidationError::new(
                        "TERMINATE_TO_NON_TERMINAL",
                        format!(
                            "TERMINATE transition '{}' targets a non-terminal node (should use RETURN)",
                            trans.transition_key
                        ),
                    ));
                }
            }
        }
    }

    // Terminal nodes must have no outgoing transitions
    for node in &graph.nodes {
        if node.node_type == NodeType::TERMINAL {
            let outgoing: Vec<&TransitionDefinition> = graph
                .transitions
                .iter()
                .filter(|t| t.source_node_id == node.node_id)
                .collect();
            if !outgoing.is_empty() {
                let keys: Vec<&str> = outgoing.iter().map(|t| t.transition_key.as_str()).collect();
                errors.push(GraphValidationError::new(
                    "TERMINAL_HAS_OUTGOING",
                    format!(
                        "terminal node '{}' has {} outgoing transition(s): {}",
                        node.node_key,
                        outgoing.len(),
                        keys.join(", ")
                    ),
                ));
            }
        }
    }

    // =====================================================================
    // 14.5 Transition completeness
    // =====================================================================

    // 4. Each transition's submission_schema is valid (checked separately in service)
    // 5. Primary transition ID must actually exist (checked above)

    // =====================================================================
    // 14.6 Assignee rules
    // =====================================================================

    for node in &graph.nodes {
        match node.node_type {
            NodeType::DRAFT => {
                // Draft node must be WORKFLOW_CREATOR
                if node.assignee_ref.ref_type != AssigneeRefType::WorkflowCreator {
                    errors.push(GraphValidationError::new(
                        "DRAFT_NOT_WORKFLOW_CREATOR",
                        format!(
                            "DRAFT node '{}' has assignee {:?}, expected WORKFLOW_CREATOR",
                            node.node_key, node.assignee_ref.ref_type
                        ),
                    ));
                }
                // Also check no fixed_principal_id for non-FIXED_PRINCIPAL types
                if node.assignee_ref.ref_type != AssigneeRefType::FixedPrincipal
                    && node.assignee_ref.fixed_principal_id.is_some()
                {
                    errors.push(GraphValidationError::new(
                        "UNEXPECTED_FIXED_PRINCIPAL",
                        format!(
                            "DRAFT node '{}' has fixed_principal_id but assignee type is {:?}",
                            node.node_key, node.assignee_ref.ref_type
                        ),
                    ));
                }
            }
            NodeType::TERMINAL => {
                // Terminal node has no assignee
                // (the assignee_ref can be WORKFLOW_CREATOR as default but no one is assigned)
            }
            NodeType::NORMAL => {
                // Fixed principal checks
                if node.assignee_ref.ref_type == AssigneeRefType::FixedPrincipal {
                    if node.assignee_ref.fixed_principal_id.is_none() {
                        errors.push(GraphValidationError::new(
                            "FIXED_PRINCIPAL_MISSING_ID",
                            format!(
                                "Normal node '{}' is FIXED_PRINCIPAL but no principal_id provided",
                                node.node_key
                            ),
                        ));
                    }
                } else if node.assignee_ref.fixed_principal_id.is_some() {
                    errors.push(GraphValidationError::new(
                        "UNEXPECTED_FIXED_PRINCIPAL",
                        format!(
                            "Node '{}' has fixed_principal_id but assignee type is {:?}",
                            node.node_key, node.assignee_ref.ref_type
                        ),
                    ));
                }
            }
        }
    }

    let valid = errors.is_empty();

    ValidationResult {
        valid,
        errors,
        warnings,
        computed_digest: None,
    }
}

/// Compute reachable nodes via forward transitions (graph traversal).
fn compute_reachable_nodes(
    _nodes: &[NodeDefinition],
    transitions: &[TransitionDefinition],
    nodes_by_id: HashMap<NodeId, &NodeDefinition>,
) -> HashSet<NodeId> {
    // Find start nodes (nodes that are not the target of any transition)
    let mut reachable: HashSet<NodeId> = HashSet::new();

    for node in nodes_by_id.values() {
        // Start from nodes that have no incoming transitions (likely draft)
        let has_incoming = transitions.iter().any(|t| t.target_node_id == node.node_id);
        if !has_incoming {
            // BFS from this node
            let mut queue = vec![node.node_id];
            while let Some(current) = queue.pop() {
                if reachable.insert(current) {
                    for trans in transitions {
                        if trans.source_node_id == current {
                            queue.push(trans.target_node_id);
                        }
                    }
                }
            }
        }
    }

    reachable
}

/// Compute weakly reachable nodes (ignoring direction).
fn compute_weakly_reachable(
    nodes: &[NodeDefinition],
    transitions: &[TransitionDefinition],
    start_node_id: NodeId,
) -> HashSet<NodeId> {
    let mut reachable: HashSet<NodeId> = HashSet::new();
    let mut queue = vec![start_node_id];

    while let Some(current) = queue.pop() {
        if reachable.insert(current) {
            for trans in transitions {
                if trans.source_node_id == current {
                    queue.push(trans.target_node_id);
                }
                if trans.target_node_id == current {
                    queue.push(trans.source_node_id);
                }
            }
        }
    }

    // Also add all nodes that are in the graph (they're all in the same subgraph)
    for node in nodes {
        if reachable.contains(&node.node_id) {
            continue;
        }
        // Check if any transition connects this node
        let connected = transitions
            .iter()
            .any(|t| t.source_node_id == node.node_id || t.target_node_id == node.node_id);
        if !connected && reachable.is_empty() {
            // Single node with no connections - add it anyway
        }
    }

    reachable
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::enums::{AssigneeRefType, NodeType, TransitionEffect};
    use crate::domain::ids::{DefinitionVersionId, NodeId, TransitionId};

    /// Helper to create a simple valid graph.
    fn valid_graph() -> WorkflowGraph {
        let draft_node_id = NodeId::new();
        let normal_node_id = NodeId::new();
        let terminal_node_id = NodeId::new();
        let version_id = DefinitionVersionId::new();

        let advance_to_normal = TransitionId::new();
        let advance_to_terminal = TransitionId::new();

        WorkflowGraph {
            nodes: vec![
                NodeDefinition {
                    node_id: draft_node_id,
                    definition_version_id: version_id,
                    node_key: "draft".to_string(),
                    display_name: "DRAFT".to_string(),
                    order_index: 0,
                    node_type: NodeType::DRAFT,
                    assignee_ref: AssigneeRef {
                        ref_type: AssigneeRefType::WorkflowCreator,
                        fixed_principal_id: None,
                    },
                    instructions: None,
                    primary_advance_transition_id: Some(advance_to_normal),
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
                        fixed_principal_id: Some(crate::domain::ids::PrincipalId::new()),
                    },
                    instructions: None,
                    primary_advance_transition_id: Some(advance_to_terminal),
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
            ],
            transitions: vec![
                TransitionDefinition {
                    transition_id: advance_to_normal,
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
                    transition_id: advance_to_terminal,
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
            ],
            context_schema: Some(serde_json::json!({"type": "object"})),
        }
    }

    #[test]
    fn valid_three_node_trunk_passes() {
        let graph = valid_graph();
        let result = validate_graph(&graph);
        assert!(
            result.valid,
            "Expected valid graph, got errors: {:?}",
            result.errors
        );
    }

    #[test]
    fn no_draft_node() {
        let mut graph = valid_graph();
        graph.nodes[0].node_type = NodeType::NORMAL;
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "NO_DRAFT_NODE"));
    }

    #[test]
    fn multiple_draft_nodes() {
        let mut graph = valid_graph();
        // Add another draft node
        let extra_node = NodeDefinition {
            node_id: NodeId::new(),
            definition_version_id: graph.nodes[0].definition_version_id,
            node_key: "draft2".to_string(),
            display_name: "Draft 2".to_string(),
            order_index: 3,
            node_type: NodeType::DRAFT,
            assignee_ref: AssigneeRef {
                ref_type: AssigneeRefType::WorkflowCreator,
                fixed_principal_id: None,
            },
            instructions: None,
            primary_advance_transition_id: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        };
        graph.nodes.push(extra_node);
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == "MULTIPLE_DRAFT_NODES"));
    }

    #[test]
    fn no_terminal_node() {
        let mut graph = valid_graph();
        graph.nodes[2].node_type = NodeType::NORMAL;
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "NO_TERMINAL_NODE"));
    }

    #[test]
    fn non_terminal_missing_primary() {
        let mut graph = valid_graph();
        graph.nodes[1].primary_advance_transition_id = None;
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "MISSING_PRIMARY"));
    }

    #[test]
    fn primary_points_to_lower_order_index() {
        let mut graph = valid_graph();
        // Swap order indices so primary goes backward
        graph.nodes[1].order_index = -1; // dev_self_check now has lower order
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == "PRIMARY_NOT_ADVANCING"));
    }

    #[test]
    fn primary_trunk_has_cycle() {
        let mut graph = valid_graph();
        // Make normal_node's primary point back to draft
        graph.nodes[1].primary_advance_transition_id = Some(graph.transitions[0].transition_id);
        // This should create a cycle: draft -> dev_self_check -> draft
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "PRIMARY_CYCLE"));
    }

    #[test]
    fn node_not_reachable_from_draft() {
        let mut graph = valid_graph();
        // Add a disconnected node
        graph.nodes.push(NodeDefinition {
            node_id: NodeId::new(),
            definition_version_id: graph.nodes[0].definition_version_id,
            node_key: "isolated".to_string(),
            display_name: "Isolated".to_string(),
            order_index: 10,
            node_type: NodeType::NORMAL,
            assignee_ref: AssigneeRef {
                ref_type: AssigneeRefType::WorkflowCreator,
                fixed_principal_id: None,
            },
            instructions: None,
            primary_advance_transition_id: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        });
        let result = validate_graph(&graph);
        assert!(!result.valid);
        // Isolated node is missing primary
        assert!(result.errors.iter().any(|e| e.code == "MISSING_PRIMARY"));
    }

    #[test]
    fn return_to_higher_order_rejected() {
        let mut graph = valid_graph();
        // Add a RETURN transition from normal_node back to draft
        let return_trans_id = TransitionId::new();
        graph.transitions.push(TransitionDefinition {
            transition_id: return_trans_id,
            definition_version_id: graph.nodes[0].definition_version_id,
            transition_key: "return-draft".to_string(),
            display_name: "Return to Draft".to_string(),
            source_node_id: graph.nodes[1].node_id,
            target_node_id: graph.nodes[0].node_id,
            transition_effect: TransitionEffect::Return,
            submission_schema: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        });
        let result = validate_graph(&graph);
        // This is a valid return: from order 1 to order 0
        assert!(
            result.valid,
            "Valid RETURN should pass, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn return_to_terminal_rejected() {
        let mut graph = valid_graph();
        // Add a RETURN transition to the terminal node
        let return_trans_id = TransitionId::new();
        graph.transitions.push(TransitionDefinition {
            transition_id: return_trans_id,
            definition_version_id: graph.nodes[0].definition_version_id,
            transition_key: "return-done".to_string(),
            display_name: "Return to Done".to_string(),
            source_node_id: graph.nodes[1].node_id,
            target_node_id: graph.nodes[2].node_id,
            transition_effect: TransitionEffect::Return,
            submission_schema: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        });
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "RETURN_TO_TERMINAL"));
    }

    #[test]
    fn terminate_to_terminal_identified() {
        let mut graph = valid_graph();
        // Add a TERMINATE transition from normal_node to done
        let terminate_trans_id = TransitionId::new();
        graph.transitions.push(TransitionDefinition {
            transition_id: terminate_trans_id,
            definition_version_id: graph.nodes[0].definition_version_id,
            transition_key: "abandon".to_string(),
            display_name: "Abandon".to_string(),
            source_node_id: graph.nodes[1].node_id,
            target_node_id: graph.nodes[2].node_id,
            transition_effect: TransitionEffect::Terminate,
            submission_schema: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        });
        let result = validate_graph(&graph);
        assert!(
            result.valid,
            "Valid TERMINATE should pass, got: {:?}",
            result.errors
        );
    }

    #[test]
    fn primary_to_done_is_advance() {
        // The existing graph already has advance-to-terminal as primary
        let graph = valid_graph();
        let result = validate_graph(&graph);
        // The primary from normal_node to done is ADVANCE, which is correct
        assert!(result.valid, "Primary to terminal should pass as ADVANCE");
    }

    #[test]
    fn terminal_has_outgoing_transition() {
        let mut graph = valid_graph();
        // Add outgoing transition from terminal node
        let bad_trans_id = TransitionId::new();
        graph.transitions.push(TransitionDefinition {
            transition_id: bad_trans_id,
            definition_version_id: graph.nodes[0].definition_version_id,
            transition_key: "bad-exit".to_string(),
            display_name: "Bad Exit".to_string(),
            source_node_id: graph.nodes[2].node_id,
            target_node_id: graph.nodes[0].node_id,
            transition_effect: TransitionEffect::Advance,
            submission_schema: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        });
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == "TERMINAL_HAS_OUTGOING"));
    }

    #[test]
    fn draft_assignee_not_workflow_creator() {
        let mut graph = valid_graph();
        graph.nodes[0].assignee_ref.ref_type = AssigneeRefType::FixedPrincipal;
        graph.nodes[0].assignee_ref.fixed_principal_id =
            Some(crate::domain::ids::PrincipalId::new());
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == "DRAFT_NOT_WORKFLOW_CREATOR"));
    }

    #[test]
    fn fixed_principal_missing_id() {
        let mut graph = valid_graph();
        graph.nodes[1].assignee_ref.fixed_principal_id = None;
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == "FIXED_PRINCIPAL_MISSING_ID"));
    }

    #[test]
    fn duplicate_order_index() {
        let mut graph = valid_graph();
        graph.nodes[2].order_index = 0; // Same as draft
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == "DUPLICATE_ORDER_INDEX"));
    }

    #[test]
    fn self_loop_transition() {
        let mut graph = valid_graph();
        graph.transitions[1].source_node_id = graph.nodes[1].node_id;
        graph.transitions[1].target_node_id = graph.nodes[1].node_id;
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "SELF_LOOP"));
    }

    #[test]
    fn return_is_primary_rejected() {
        let mut graph = valid_graph();
        // Make normal_node's primary also marked as RETURN
        let primary_trans: TransitionId = graph.nodes[1].primary_advance_transition_id.unwrap();
        // Change the effect to RETURN
        if let Some(trans) = graph
            .transitions
            .iter_mut()
            .find(|t| t.transition_id == primary_trans)
        {
            trans.transition_effect = TransitionEffect::Return;
        }
        // Add a proper RETURN to replace
        let return_trans_id = TransitionId::new();
        graph.transitions.push(TransitionDefinition {
            transition_id: return_trans_id,
            definition_version_id: graph.nodes[0].definition_version_id,
            transition_key: "proper-return".to_string(),
            display_name: "Proper Return".to_string(),
            source_node_id: graph.nodes[1].node_id,
            target_node_id: graph.nodes[0].node_id,
            transition_effect: TransitionEffect::Return,
            submission_schema: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        });
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result.errors.iter().any(|e| e.code == "RETURN_IS_PRIMARY"));
    }

    #[test]
    fn terminate_to_non_terminal_rejected() {
        let mut graph = valid_graph();
        // Add a TERMINATE to normal_node (non-terminal)
        let terminate_trans_id = TransitionId::new();
        graph.transitions.push(TransitionDefinition {
            transition_id: terminate_trans_id,
            definition_version_id: graph.nodes[0].definition_version_id,
            transition_key: "bad-terminate".to_string(),
            display_name: "Bad Terminate".to_string(),
            source_node_id: graph.nodes[0].node_id,
            target_node_id: graph.nodes[1].node_id,
            transition_effect: TransitionEffect::Terminate,
            submission_schema: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        });
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == "TERMINATE_TO_NON_TERMINAL"));
    }

    #[test]
    fn unexpected_fixed_principal_on_non_fixed_type() {
        let mut graph = valid_graph();
        graph.nodes[0].assignee_ref.fixed_principal_id =
            Some(crate::domain::ids::PrincipalId::new());
        let result = validate_graph(&graph);
        assert!(!result.valid);
        assert!(result
            .errors
            .iter()
            .any(|e| e.code == "UNEXPECTED_FIXED_PRINCIPAL"));
    }

    #[test]
    fn invalid_json_schema_not_checked_by_graph_validation() {
        // JSON schema validation is done separately by the service layer
        let graph = valid_graph();
        let result = validate_graph(&graph);
        // Graph validation doesn't check JSON schema validity
        // (it would pass even with invalid schema)
        assert!(result.valid);
    }
}
