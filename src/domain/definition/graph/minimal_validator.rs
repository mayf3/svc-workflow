//! V2 Minimal semantic model validator — fully separated from the Legacy
//! (V1) validator. V1 rules live in `mod.rs`/`validation.rs`/
//! `assignee_validation.rs`/`transition_validation.rs` and are untouched.
//!
//! V2 semantic contract (frozen by WORKFLOW_MINIMAL_VALIDATOR_V1):
//!
//! Node:
//!   key, kind (TASK | TERMINAL), assignee?, instructions?, metadata?
//!   TASK    -> assignee required, one of Creator | FixedPrincipal | ContextPrincipal
//!   TERMINAL-> no assignee, no task-execution semantics, no outgoing edges
//!
//! Transition:
//!   key, source, target, effect (ADVANCE | RETURN only),
//!   submissionSchema?, metadata?
//!
//! V2 termination semantics: TERMINATE transitions are FORBIDDEN in V2.
//! A workflow ends by normal graph progression (ADVANCE) into a TERMINAL
//! node; instance cancel/archive is instance lifecycle, not a graph
//! transition. Legacy V1 keeps TERMINATE unchanged.
//!
//! Forbidden Legacy concepts in V2:
//!   DRAFT node type, DOMAIN_OWNER assignee, primary_advance_transition_id,
//!   orderIndex as execution semantics, AUTO/WAIT nodes, claim/pool/reassign
//!   semantics, parallel/fork/join.
//!
//! Graph rules:
//!   * ADVANCE edges must form a DAG with exactly one entry TASK (the unique
//!     ADVANCE root); every TASK must be reachable from entry via ADVANCE.
//!   * Multiple outgoing ADVANCE edges from a TASK are allowed (condition
//!     branches). No primary ADVANCE concept.
//!   * RETURN target must be a strict ADVANCE ancestor of the source; RETURN
//!     edges do not participate in cycle checks.

//! This module decides legality only. No V2 runtime, no production V2
//! creation path, no changes to V1 behavior.

use std::collections::{HashMap, HashSet, VecDeque};

use super::super::error::GraphValidationError;
use super::super::model::{NodeDefinition, ValidationResult, WorkflowGraph};
use super::transition_validation::validate_transition_references;
use crate::domain::enums::{AssigneeRefType, NodeType, TransitionEffect};

/// Minimal node kinds under the V2 semantic contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MinimalNodeKind {
    Task,
    Terminal,
}

fn node_kind(node: &NodeDefinition) -> Result<MinimalNodeKind, GraphValidationError> {
    match node.node_type {
        NodeType::DRAFT => Err(GraphValidationError::new(
            "v2_node_draft_forbidden",
            format!(
                "V2 node '{}' uses the forbidden Legacy DRAFT node type",
                node.node_key
            ),
        )),
        NodeType::NORMAL => Ok(MinimalNodeKind::Task),
        NodeType::TERMINAL => Ok(MinimalNodeKind::Terminal),
    }
}

fn err(code: &str, message: impl Into<String>) -> GraphValidationError {
    GraphValidationError::new(code, message)
}

/// Validate a workflow graph under the V2 Minimal semantic model.
pub fn validate_minimal_graph(graph: &WorkflowGraph) -> ValidationResult {
    let mut errors: Vec<GraphValidationError> = Vec::new();

    let nodes_by_id: HashMap<_, &NodeDefinition> =
        graph.nodes.iter().map(|n| (n.node_id, n)).collect();

    // ---
    // 0. Pure structural reference checks (shared helper, no V1 semantics)
    // ---
    validate_transition_references(graph, &nodes_by_id, &mut errors);

    // ---
    // 1. Node rules: kind, assignee, primary-advance, terminal constraints
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
            MinimalNodeKind::Task => {
                tasks.push(node);
                if node.assignee_ref.is_none() {
                    errors.push(err(
                        "v2_task_assignee_required",
                        format!("V2 TASK '{}' must declare an assignee", node.node_key),
                    ));
                }
            }
            MinimalNodeKind::Terminal => {
                if node.assignee_ref.is_some() {
                    errors.push(err(
                        "v2_terminal_assignee_forbidden",
                        format!(
                            "V2 TERMINAL '{}' must not declare an assignee",
                            node.node_key
                        ),
                    ));
                }
            }
        }
        if node.primary_advance_transition_id.is_some() {
            errors.push(err(
                "v2_primary_advance_forbidden",
                format!(
                    "V2 node '{}' sets primary_advance_transition_id; V2 uses plain ADVANCE edges",
                    node.node_key
                ),
            ));
        }
    }

    // ---
    // 2. Assignee selectors: only Creator | FixedPrincipal | ContextPrincipal
    // ---
    for node in &graph.nodes {
        let Some(assignee) = &node.assignee_ref else { continue };
        match assignee.ref_type {
            AssigneeRefType::WorkflowCreator => {}
            AssigneeRefType::FixedPrincipal => {
                if assignee.fixed_principal_id.is_none() {
                    errors.push(err(
                        "v2_fixed_principal_missing",
                        format!(
                            "V2 node '{}' uses FixedPrincipal without a principalId",
                            node.node_key
                        ),
                    ));
                }
            }
            AssigneeRefType::InstanceInputPrincipal => {
                // ContextPrincipal(singleSegmentPath): the context key is the
                // assignee_input_key and must be a single-segment path.
                match assignee.assignee_input_key.as_deref() {
                    None => errors.push(err(
                        "v2_context_principal_key_required",
                        format!(
                            "V2 node '{}' uses ContextPrincipal without a path",
                            node.node_key
                        ),
                    )),
                    Some(key) if !is_single_segment(key) => errors.push(err(
                        "v2_context_principal_key_must_be_single_segment",
                        format!(
                            "V2 node '{}' ContextPrincipal path '{}' must be a single segment \
                             (no '.', '/', '[', ']')",
                            node.node_key, key
                        ),
                    )),
                    Some(_) => {}
                }
            }
            AssigneeRefType::DomainOwner => errors.push(err(
                "v2_assignee_domain_owner_forbidden",
                format!(
                    "V2 node '{}' uses the forbidden Legacy DOMAIN_OWNER assignee",
                    node.node_key
                ),
            )),
        }
    }

    // ---
    // 3. ADVANCE graph analysis (V2 execution order comes from the graph,
    //    never from orderIndex)
    // ---
    let mut advance_out: HashMap<uuid::Uuid, Vec<uuid::Uuid>> = HashMap::new();
    let mut advance_in_degree: HashMap<uuid::Uuid, usize> = HashMap::new();
    let mut returns: Vec<(uuid::Uuid, uuid::Uuid)> = Vec::new();

    for node in &graph.nodes {
        advance_in_degree.insert(*node.node_id.as_uuid(), 0);
    }
    for trans in &graph.transitions {
        let source = *trans.source_node_id.as_uuid();
        let target = *trans.target_node_id.as_uuid();
        match trans.transition_effect {
            TransitionEffect::Advance => {
                advance_out.entry(source).or_default().push(target);
                *advance_in_degree.entry(target).or_default() += 1;
            }
            TransitionEffect::Return => returns.push((source, target)),
            // V2 termination semantics: TERMINATE transition is forbidden.
            // Workflows end by reaching a TERMINAL node via ADVANCE; instance
            // cancel/archive is instance lifecycle, not a graph transition.
            TransitionEffect::Terminate => errors.push(err(
                "v2_terminate_effect_forbidden",
                format!(
                    "V2 transition '{}' uses TERMINATE; V2 ends workflows by \
                     ADVANCE into a TERMINAL node (cancel/archive are instance \
                     lifecycle, not graph transitions)",
                    trans.transition_key
                ),
            )),
        }
    }

    // 3a. ADVANCE graph must be a DAG
    if let Some((from, to)) = find_advance_cycle(&advance_out) {
        errors.push(err(
            "v2_advance_cycle",
            format!("V2 ADVANCE graph contains a cycle: {from} -> ... -> {to}"),
        ));
    }

    // 3b. Exactly one entry TASK = the unique ADVANCE root among TASKs
    let task_ids: HashSet<uuid::Uuid> = tasks.iter().map(|n| *n.node_id.as_uuid()).collect();
    let entry_candidates: Vec<uuid::Uuid> = task_ids
        .iter()
        .copied()
        .filter(|id| advance_in_degree.get(id).copied().unwrap_or(0) == 0)
        .collect();
    let entry = match entry_candidates.as_slice() {
        [] => {
            errors.push(err(
                "v2_entry_task_required",
                "V2 graph must have exactly one entry TASK (a TASK with no incoming ADVANCE)",
            ));
            None
        }
        [single] => Some(*single),
        many => {
            errors.push(err(
                "v2_multiple_entry_tasks",
                format!(
                    "V2 graph has {} entry TASKs ({:?}); exactly one is required",
                    many.len(),
                    many
                ),
            ));
            None
        }
    };

    // 3c. Every TASK must be reachable from the entry via ADVANCE edges
    if let Some(entry) = entry {
        let reachable = advance_reachable(entry, &advance_out);
        for task in &tasks {
            if !reachable.contains(task.node_id.as_uuid()) {
                errors.push(err(
                    "v2_unreachable_task",
                    format!(
                        "V2 TASK '{}' is not reachable from entry via ADVANCE edges",
                        task.node_key
                    ),
                ));
            }
        }
    }

    // ---
    // 4. RETURN: target must be a strict ADVANCE ancestor of the source
    // ---
    for (source, target) in &returns {
        if source == target {
            errors.push(err(
                "v2_return_target_not_strict_ancestor",
                format!("V2 RETURN from '{source}' to itself is forbidden"),
            ));
            continue;
        }
        let ancestors_of_source = advance_reachable(*target, &advance_out);
        if !ancestors_of_source.contains(source) {
            errors.push(err(
                "v2_return_target_not_strict_ancestor",
                format!(
                    "V2 RETURN target '{target}' is not a strict ADVANCE ancestor of source '{source}'"
                ),
            ));
        }
    }

    // ---
    // 5. TERMINAL nodes: no assignee (checked above), no outgoing edges.
    //    Workflows end by ADVANCE into a TERMINAL node; TERMINATE itself is
    //    rejected during transition collection (v2_terminate_effect_forbidden).
    // ---
    for node in &graph.nodes {
        if node.node_type == NodeType::TERMINAL {
            let has_outgoing = graph
                .transitions
                .iter()
                .any(|t| t.source_node_id == node.node_id);
            if has_outgoing {
                errors.push(err(
                    "v2_terminal_outgoing_forbidden",
                    format!("V2 TERMINAL '{}' must not have outgoing transitions", node.node_key),
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

/// True when `key` is a single-segment context path (no JSONPath-ish
/// separators). Minimal by design; no expression language is built here.
fn is_single_segment(key: &str) -> bool {
    !key.is_empty()
        && !key.contains('.')
        && !key.contains('/')
        && !key.contains('[')
        && !key.contains(']')
        && !key.contains(' ')
}

/// Nodes reachable from `start` following ADVANCE edges (BFS).
fn advance_reachable(
    start: uuid::Uuid,
    advance_out: &HashMap<uuid::Uuid, Vec<uuid::Uuid>>,
) -> HashSet<uuid::Uuid> {
    let mut seen = HashSet::new();
    let mut queue = VecDeque::new();
    queue.push_back(start);
    seen.insert(start);
    while let Some(node) = queue.pop_front() {
        if let Some(next) = advance_out.get(&node) {
            for target in next {
                if seen.insert(*target) {
                    queue.push_back(*target);
                }
            }
        }
    }
    seen
}

/// Returns Some((start, end)) if the ADVANCE graph contains a cycle.
fn find_advance_cycle(
    advance_out: &HashMap<uuid::Uuid, Vec<uuid::Uuid>>,
) -> Option<(uuid::Uuid, uuid::Uuid)> {
    // Kahn's algorithm: nodes that cannot be removed form a cycle.
    let mut in_degree: HashMap<uuid::Uuid, usize> = HashMap::new();
    for (from, targets) in advance_out {
        in_degree.entry(*from).or_insert(0);
        for target in targets {
            *in_degree.entry(*target).or_insert(0) += 1;
        }
    }
    let mut queue: VecDeque<uuid::Uuid> = in_degree
        .iter()
        .filter(|(_, deg)| **deg == 0)
        .map(|(node, _)| *node)
        .collect();
    let mut removed = 0usize;
    while let Some(node) = queue.pop_front() {
        removed += 1;
        if let Some(targets) = advance_out.get(&node) {
            for target in targets {
                let degree = in_degree.get_mut(target).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(*target);
                }
            }
        }
    }
    if removed == in_degree.len() {
        None
    } else {
        let cycle_node = in_degree
            .iter()
            .find(|(_, deg)| **deg > 0)
            .map(|(node, _)| *node)
            .unwrap_or_default();
        Some((cycle_node, cycle_node))
    }
}
