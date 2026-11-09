//! Draft-only local producer resolution. Runtime decoders remain UUID-only.
use super::{error::GraphValidationError, model::NodeDefinition};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

/// Restrict the new interpretation to the two accepted authoring positions.
pub(crate) fn affected(nodes: &[NodeDefinition]) -> bool {
    nodes.iter().any(|n| metadata_affected(n.metadata.as_ref()))
}

pub(crate) fn metadata_affected(metadata: Option<&Value>) -> bool {
    metadata.is_some_and(|m| {
        m.get("workflowPrerequisiteGate").is_some()
            || m.get("workflowOperation")
                .and_then(|o| o.get("prerequisiteProducer"))
                .is_some()
    })
}

/// The caller supplies only rows from the current target DRAFT under its
/// replacement lock. Never look up a canonical input in history or globally.
pub(crate) fn normalize(
    nodes: &mut [NodeDefinition],
    old_keys: &[(Uuid, String)],
) -> Result<(), Vec<GraphValidationError>> {
    let mut fresh = HashMap::new();
    for node in nodes.iter() {
        if fresh
            .insert(node.node_key.clone(), node.node_id.into_uuid())
            .is_some()
        {
            return Err(vec![GraphValidationError::new(
                "DUPLICATE_NODE_KEY",
                "duplicate node key",
            )]);
        }
    }
    let old: HashMap<_, _> = old_keys
        .iter()
        .map(|(id, key)| (*id, key.as_str()))
        .collect();
    let mut errors = Vec::new();
    for node in nodes {
        let Some(metadata) = node.metadata.as_mut() else {
            continue;
        };
        for (declaration, member, code) in [
            (
                "workflowPrerequisiteGate",
                "producer",
                "owner_node_gate_producer_invalid",
            ),
            (
                "workflowOperation",
                "prerequisiteProducer",
                "owner_operation_target_invalid",
            ),
        ] {
            let Some(container) = metadata.get_mut(declaration) else {
                continue;
            };
            // A gate requires its producer; operation's producer is optional.
            if declaration == "workflowOperation" && container.get(member).is_none() {
                continue;
            }
            let resolved = container
                .get(member)
                .and_then(|reference| resolve(reference, &fresh, &old));
            match (container.as_object_mut(), resolved) {
                (Some(container), Some(reference)) => {
                    container.insert(member.into(), reference);
                }
                _ => errors.push(GraphValidationError::new(code, code)),
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn resolve(
    value: &Value,
    fresh: &HashMap<String, Uuid>,
    old: &HashMap<Uuid, &str>,
) -> Option<Value> {
    let fields = value.as_object()?;
    if fields.len() != 2 {
        return None;
    }
    let slot = fields.get("slotKey")?.as_str().filter(|s| !s.is_empty())?;
    let key = match (fields.get("nodeKey"), fields.get("nodeId")) {
        (Some(key), None) => key.as_str().filter(|s| !s.is_empty())?,
        (None, Some(id)) => {
            let id = Uuid::parse_str(id.as_str()?).ok()?;
            old.get(&id).copied()?
        }
        _ => return None,
    };
    let node_id = fresh.get(key)?;
    Some(json!({"nodeId": node_id, "slotKey": slot}))
}
