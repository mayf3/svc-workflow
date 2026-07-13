//! Helper function for graph validation.

use std::collections::{HashMap, HashSet};

use crate::domain::ids::NodeId;

use super::model::{NodeDefinition, TransitionDefinition};

/// Compute weakly reachable nodes (ignoring direction).
pub fn compute_weakly_reachable(
    _nodes: &[NodeDefinition],
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
    reachable
}
