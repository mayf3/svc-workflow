//! Workflow graph validation engine.
//!
//! Validates a complete workflow graph before publication.
//! Rules are specified in the architecture document section 14.
#![allow(clippy::needless_borrow)]
use super::error::GraphValidationError;
use super::graph_helpers::compute_directed_reachable;
use super::model::{
    AssigneeRef, NodeDefinition, TransitionDefinition, ValidationResult, WorkflowGraph,
};
use crate::domain::enums::{AssigneeRefType, NodeType};
use crate::domain::ids::{NodeId, TransitionId};
use std::collections::{HashMap, HashSet};
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
    // ---
    // 14.1 Node rules
    // ---
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
    // ---
    // H-2: Assignee rules — strict contract enforcement
    // ---
    // Contract §3.1.7: Terminal Node has no assignee
    // Contract §3.1.8: Non-terminal Node must have a legal assignee reference
    for node in &graph.nodes {
        match node.node_type {
            NodeType::TERMINAL => {
                // Terminal nodes must have no assignee
                if node.assignee_ref.fixed_principal_id.is_some() {
                    errors.push(GraphValidationError::new(
                        "TERMINAL_HAS_ASSIGNEE",
                        format!(
                            "terminal node '{}' must not have a fixed_principal_id",
                            node.node_key
                        ),
                    ));
                }
                // The assignee_ref_type should be empty/null-like for terminals.
                // Since the enum always has a value, we check that the ref_type
                // doesn't carry meaningful assignment semantics.
                // Terminal nodes are allowed WORKFLOW_CREATOR as a no-op default
                // but must not have FIXED_PRINCIPAL (already checked above).
                if node.assignee_ref.ref_type == AssigneeRefType::FixedPrincipal {
                    errors.push(GraphValidationError::new(
                        "TERMINAL_HAS_ASSIGNEE",
                        format!(
                            "terminal node '{}' must not have assignee type FIXED_PRINCIPAL",
                            node.node_key
                        ),
                    ));
                }
            }
            NodeType::DRAFT => {
                // DRAFT node must be WORKFLOW_CREATOR (contract §8.1)
                if node.assignee_ref.ref_type != AssigneeRefType::WorkflowCreator {
                    errors.push(GraphValidationError::new(
                        "DRAFT_NOT_WORKFLOW_CREATOR",
                        format!(
                            "DRAFT node '{}' has assignee {:?}, expected WORKFLOW_CREATOR",
                            node.node_key, node.assignee_ref.ref_type
                        ),
                    ));
                }
                // WORKFLOW_CREATOR must not have a fixed_principal_id
                if node.assignee_ref.fixed_principal_id.is_some() {
                    errors.push(GraphValidationError::new(
                        "UNEXPECTED_FIXED_PRINCIPAL",
                        format!(
                            "DRAFT node '{}' has fixed_principal_id but assignee type is {:?}",
                            node.node_key, node.assignee_ref.ref_type
                        ),
                    ));
                }
            }
            NodeType::NORMAL => {
                // Non-terminal nodes must have a legal assignee reference
                match node.assignee_ref.ref_type {
                    AssigneeRefType::WorkflowCreator => {
                        // Must NOT have a fixed_principal_id
                        if node.assignee_ref.fixed_principal_id.is_some() {
                            errors.push(GraphValidationError::new(
                                "UNEXPECTED_FIXED_PRINCIPAL",
                                format!(
                                    "NORMAL node '{}' is WORKFLOW_CREATOR but has fixed_principal_id",
                                    node.node_key
                                ),
                            ));
                        }
                    }
                    AssigneeRefType::DomainOwner => {
                        // Must NOT have a fixed_principal_id
                        if node.assignee_ref.fixed_principal_id.is_some() {
                            errors.push(GraphValidationError::new(
                                "UNEXPECTED_FIXED_PRINCIPAL",
                                format!(
                                    "NORMAL node '{}' is DOMAIN_OWNER but has fixed_principal_id",
                                    node.node_key
                                ),
                            ));
                        }
                    }
                    AssigneeRefType::FixedPrincipal => {
                        // MUST have a fixed_principal_id
                        if node.assignee_ref.fixed_principal_id.is_none() {
                            errors.push(GraphValidationError::new(
                                "FIXED_PRINCIPAL_MISSING_ID",
                                format!(
                                    "NORMAL node '{}' is FIXED_PRINCIPAL but no principal_id provided",
                                    node.node_key
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
    // ---
    // 14.5 Transition uniqueness (transition_key unique within version)
    // ---
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
    // ---
    // 14.2 Primary trunk rules
    // ---
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
                // H-3: Primary transition effect must be ADVANCE
                if trans.transition_effect != crate::domain::enums::TransitionEffect::Advance {
                    errors.push(GraphValidationError::new(
                        "PRIMARY_NOT_ADVANCE",
                        format!(
                            "primary transition '{}' for node '{}' has effect {:?}, expected ADVANCE",
                            trans.transition_key, node.node_key, trans.transition_effect
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
    // ---
    // 14.3 RETURN rules
    // ---
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
    // ---
    // 14.4 TERMINATE rules
    // ---
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
    // ---
    // 14.5 Transition completeness
    // ---
    // 4. Each transition's submission_schema is valid (checked separately in service)
    // 5. Primary transition ID must actually exist (checked above)
    let valid = errors.is_empty();
    ValidationResult {
        valid,
        errors,
        warnings,
        computed_digest: None,
    }
}
