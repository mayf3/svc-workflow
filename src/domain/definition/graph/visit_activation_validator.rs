//! VISIT_ACTIVATION_V1 (semantic model 3) graph validator.
//!
//! Implements the graph rules frozen by accepted
//! SVC_WORKFLOW_ARCHITECTURE_V0_4_0 §5.3 / CTR-ARCH-006:
//!
//! - node kinds are exactly TASK and TERMINAL (Legacy DRAFT/NORMAL are
//!   forbidden in new-model graphs);
//! - exactly one entry TASK exists;
//! - every TASK has exactly one primary ADVANCE Transition;
//! - primary ADVANCE edges form one acyclic deterministic path ending at a
//!   TERMINAL node;
//! - RETURN targets only an earlier reachable TASK;
//! - non-primary TERMINATE targets only a TERMINAL node;
//! - TERMINAL has no owner and no outgoing edge;
//! - all nodes are reachable from the entry TASK;
//! - TASK owner references come from the closed set
//!   WORKFLOW_CREATOR | DOMAIN_OWNER | FIXED_PRINCIPAL
//!   (INSTANCE_INPUT_PRINCIPAL is not part of the new-model owner set).

use std::collections::{HashMap, HashSet, VecDeque};

use super::super::error::GraphValidationError;
use super::super::model::{NodeDefinition, ValidationResult, WorkflowGraph};
use super::transition_validation::validate_transition_references;
use crate::domain::enums::{AssigneeRefType, NodeType, TransitionEffect};

fn err(code: &str, message: impl Into<String>) -> GraphValidationError {
    GraphValidationError::new(code, message)
}

fn node_kind(node: &NodeDefinition) -> Result<V1NodeKind, GraphValidationError> {
    match node.node_type {
        NodeType::TASK => Ok(V1NodeKind::Task),
        NodeType::TERMINAL => Ok(V1NodeKind::Terminal),
        other => Err(err(
            "v1_node_kind_forbidden",
            format!(
                "V1 node '{}' uses forbidden node type '{}'; VISIT_ACTIVATION_V1 allows exactly TASK | TERMINAL",
                node.node_key, other
            ),
        )),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum V1NodeKind {
    Task,
    Terminal,
}

/// Validate a workflow graph under the VISIT_ACTIVATION_V1 semantic model.
pub fn validate_visit_activation_graph(graph: &WorkflowGraph) -> ValidationResult {
    let mut errors: Vec<GraphValidationError> = Vec::new();

    let nodes_by_id: HashMap<uuid::Uuid, &NodeDefinition> =
        graph.nodes.iter().map(|n| (*n.node_id.as_uuid(), n)).collect();
    let nodes_by_node_id: HashMap<crate::domain::ids::NodeId, &NodeDefinition> =
        graph.nodes.iter().map(|n| (n.node_id, n)).collect();

    // ---
    // 0. Pure structural reference checks (shared helper).
    // ---
    validate_transition_references(graph, &nodes_by_node_id, &mut errors);

    // ---
    // 1. Node rules: kind, owner reference, primary ADVANCE, terminal.
    // ---
    let mut tasks: Vec<&NodeDefinition> = Vec::new();
    for node in &graph.nodes {
        let kind = match node_kind(node) {
            Ok(kind) => kind,
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        match kind {
            V1NodeKind::Task => {
                tasks.push(node);
                // TASK carries exactly one owner reference from the closed set.
                let Some(assignee) = &node.assignee_ref else {
                    errors.push(err(
                        "v1_task_owner_required",
                        format!("V1 TASK '{}' must declare an owner reference", node.node_key),
                    ));
                    continue;
                };
                match assignee.ref_type {
                    AssigneeRefType::WorkflowCreator => {}
                    AssigneeRefType::DomainOwner => {}
                    AssigneeRefType::FixedPrincipal => {
                        if assignee.fixed_principal_id.is_none() {
                            errors.push(err(
                                "v1_fixed_principal_missing",
                                format!(
                                    "V1 node '{}' uses FIXED_PRINCIPAL without a principalId",
                                    node.node_key
                                ),
                            ));
                        }
                    }
                    AssigneeRefType::InstanceInputPrincipal => {
                        errors.push(err(
                            "v1_owner_ref_forbidden",
                            format!(
                                "V1 node '{}' uses INSTANCE_INPUT_PRINCIPAL; the new-model \
                                 owner set is exactly WORKFLOW_CREATOR | DOMAIN_OWNER | \
                                 FIXED_PRINCIPAL",
                                node.node_key
                            ),
                        ));
                    }
                }
                // Every TASK has exactly one primary ADVANCE.
                if node.primary_advance_transition_id.is_none() {
                    errors.push(err(
                        "v1_primary_advance_required",
                        format!(
                            "V1 TASK '{}' must declare exactly one primary ADVANCE transition",
                            node.node_key
                        ),
                    ));
                }
            }
            V1NodeKind::Terminal => {
                if node.assignee_ref.is_some() {
                    errors.push(err(
                        "v1_terminal_owner_forbidden",
                        format!("V1 TERMINAL '{}' must not declare an owner", node.node_key),
                    ));
                }
                if node.primary_advance_transition_id.is_some() {
                    errors.push(err(
                        "v1_terminal_primary_advance_forbidden",
                        format!(
                            "V1 TERMINAL '{}' must not set a primary ADVANCE transition",
                            node.node_key
                        ),
                    ));
                }
            }
        }
    }

    // ---
    // 2. Transition edges.
    // ---
    let mut advance_out: HashMap<uuid::Uuid, Vec<uuid::Uuid>> = HashMap::new();
    let mut returns: Vec<(uuid::Uuid, uuid::Uuid)> = Vec::new();
    let mut terminates: Vec<(uuid::Uuid, uuid::Uuid)> = Vec::new();

    for node in &graph.nodes {
        advance_out.insert(*node.node_id.as_uuid(), Vec::new());
    }
    for trans in &graph.transitions {
        let source = *trans.source_node_id.as_uuid();
        let target = *trans.target_node_id.as_uuid();
        let source_node = nodes_by_id.get(&source);
        match trans.transition_effect {
            TransitionEffect::Advance => {
                // Exactly one ADVANCE out per TASK: the primary one. A second
                // ADVANCE edge from the same TASK is a dynamic branch and is
                // forbidden.
                if let Some(node) = source_node {
                    if node.node_type != NodeType::TERMINAL {
                        let already = advance_out.get(&source).map_or(0, |v| v.len());
                        if already >= 1 {
                            errors.push(err(
                                "v1_multiple_advance_forbidden",
                                format!(
                                    "V1 TASK '{}' has more than one ADVANCE transition; the \
                                     normal path is exactly one primary ADVANCE",
                                    node.node_key
                                ),
                            ));
                        }
                    }
                }
                advance_out.entry(source).or_default().push(target);
            }
            TransitionEffect::Return => returns.push((source, target)),
            TransitionEffect::Terminate => terminates.push((source, target)),
        }
    }

    // Primary ADVANCE edge must exist for each declared primary and must be
    // the TASK's single ADVANCE edge.
    for node in &graph.nodes {
        if let Some(primary_id) = node.primary_advance_transition_id {
            let matching = graph
                .transitions
                .iter()
                .any(|t| t.transition_id == primary_id && t.transition_effect == TransitionEffect::Advance);
            if !matching {
                errors.push(err(
                    "v1_primary_advance_invalid",
                    format!(
                        "V1 TASK '{}' declares a primary ADVANCE that is missing or not an ADVANCE",
                        node.node_key
                    ),
                ));
            }
        }
    }

    // ---
    // 3. Primary path: acyclic, deterministic, ends at TERMINAL.
    // ---
    // Walk each TASK's primary chain: from the current node, follow the
    // edge declared as that node's own primary ADVANCE. The chain must
    // terminate at a TERMINAL node without revisiting a node.
    for task in &tasks {
        let mut current = *task.node_id.as_uuid();
        let mut seen: HashSet<uuid::Uuid> = HashSet::new();
        loop {
            if !seen.insert(current) {
                errors.push(err(
                    "v1_primary_path_cycle",
                    format!(
                        "V1 primary path from TASK '{}' revisits a node; the primary path \
                         must be acyclic and deterministic",
                        task.node_key
                    ),
                ));
                break;
            }
            let primary_id = match nodes_by_id.get(&current).and_then(|n| n.primary_advance_transition_id) {
                Some(id) => id,
                None => break, // missing primary already reported in section 1
            };
            let next = graph
                .transitions
                .iter()
                .find(|t| t.source_node_id.as_uuid() == &current && t.transition_id == primary_id)
                .map(|t| *t.target_node_id.as_uuid());
            let Some(next) = next else {
                // This task's primary edge is missing/unmatched — reported
                // in section 1. Stop walking here.
                break;
            };
            match nodes_by_id.get(&next) {
                None => break, // dangling reference already reported
                Some(node) if node.node_type == NodeType::TERMINAL => break,
                Some(_) => {
                    current = next;
                }
            }
        }
    }

    // ---
    // 4. Entry: exactly one TASK with no incoming ADVANCE.
    // ---
    let mut advance_in_degree: HashMap<uuid::Uuid, usize> = HashMap::new();
    for node in &graph.nodes {
        advance_in_degree.insert(*node.node_id.as_uuid(), 0);
    }
    for trans in &graph.transitions {
        if trans.transition_effect == TransitionEffect::Advance {
            *advance_in_degree.entry(*trans.target_node_id.as_uuid()).or_insert(0) += 1;
        }
    }
    let task_ids: HashSet<uuid::Uuid> = tasks.iter().map(|n| *n.node_id.as_uuid()).collect();
    let entry_candidates: Vec<uuid::Uuid> = task_ids
        .iter()
        .copied()
        .filter(|id| advance_in_degree.get(id).copied().unwrap_or(0) == 0)
        .collect();
    let entry = match entry_candidates.as_slice() {
        [] => {
            errors.push(err(
                "v1_entry_task_required",
                "V1 graph must have exactly one entry TASK (a TASK with no incoming ADVANCE)",
            ));
            None
        }
        [single] => Some(*single),
        many => {
            errors.push(err(
                "v1_multiple_entry_tasks",
                format!(
                    "V1 graph has {} entry TASKs ({:?}); exactly one is required",
                    many.len(),
                    many
                ),
            ));
            None
        }
    };

    // ---
    // 5. Reachability from entry over ADVANCE + TERMINATE edges.
    // ---
    let mut combined_out = advance_out.clone();
    for (source, target) in &terminates {
        combined_out.entry(*source).or_default().push(*target);
    }
    if let Some(entry) = entry {
        let reachable = reachable_from(entry, &combined_out);
        for node in &graph.nodes {
            if !reachable.contains(node.node_id.as_uuid()) {
                errors.push(err(
                    "v1_unreachable_node",
                    format!(
                        "V1 node '{}' is not reachable from the entry TASK",
                        node.node_key
                    ),
                ));
            }
        }
    }

    // ---
    // 6. RETURN: target is an earlier reachable TASK.
    // ---
    for (source, target) in &returns {
        let Some(target_node) = nodes_by_id.get(target) else {
            continue; // dangling reference already reported
        };
        if target_node.node_type != NodeType::TASK {
            errors.push(err(
                "v1_return_target_not_task",
                format!(
                    "V1 RETURN target '{}' must be a TASK node",
                    target_node.node_key
                ),
            ));
            continue;
        }
        let Some(source_node) = nodes_by_id.get(source) else {
            continue;
        };
        if target_node.order_index >= source_node.order_index {
            errors.push(err(
                "v1_return_target_not_earlier",
                format!(
                    "V1 RETURN from '{}' to '{}' must target a strictly earlier (lower \
                     order_index) TASK",
                    source_node.node_key, target_node.node_key
                ),
            ));
        }
    }

    // ---
    // 7. TERMINATE: target must be TERMINAL.
    // ---
    for (source, target) in &terminates {
        let Some(target_node) = nodes_by_id.get(target) else {
            continue;
        };
        if target_node.node_type != NodeType::TERMINAL {
            errors.push(err(
                "v1_terminate_target_not_terminal",
                format!(
                    "V1 TERMINATE from '{source}' must target a TERMINAL node",
                ),
            ));
        }
        // TERMINATE must not be the source's primary ADVANCE.
        if let Some(source_node) = nodes_by_id.get(source) {
            if let Some(primary_id) = source_node.primary_advance_transition_id {
                let is_primary = graph.transitions.iter().any(|t| {
                    t.transition_id == primary_id
                        && t.source_node_id.as_uuid() == source
                        && t.target_node_id.as_uuid() == target
                        && t.transition_effect == TransitionEffect::Terminate
                });
                if is_primary {
                    errors.push(err(
                        "v1_terminate_primary_forbidden",
                        format!(
                            "V1 TERMINATE from '{}' must use a non-primary edge",
                            source_node.node_key
                        ),
                    ));
                }
            }
        }
    }

    // ---
    // 8. TERMINAL: no outgoing edges.
    // ---
    for node in &graph.nodes {
        if node.node_type == NodeType::TERMINAL {
            let has_outgoing = graph
                .transitions
                .iter()
                .any(|t| t.source_node_id == node.node_id);
            if has_outgoing {
                errors.push(err(
                    "v1_terminal_outgoing_forbidden",
                    format!("V1 TERMINAL '{}' must not have outgoing transitions", node.node_key),
                ));
            }
        }
    }

    let valid = errors.is_empty();
    ValidationResult {
        valid,
        errors,
        warnings: Vec::new(),
        computed_digest: None,
    }
}

/// Nodes reachable from `start` following the given edges (BFS).
fn reachable_from(
    start: uuid::Uuid,
    out: &HashMap<uuid::Uuid, Vec<uuid::Uuid>>,
) -> HashSet<uuid::Uuid> {
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);
    seen.insert(start);
    while let Some(node) = queue.pop_front() {
        if let Some(next) = out.get(&node) {
            for target in next {
                if seen.insert(*target) {
                    queue.push_back(*target);
                }
            }
        }
    }
    seen
}
