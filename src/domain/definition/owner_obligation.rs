//! Closed Owner-obligation contracts. No interpretation of names or model text.
use super::{error::GraphValidationError, model::WorkflowGraph};
use crate::domain::enums::{NodeType, TransitionEffect};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
pub const POLICY: &str = "x-workflow-owner-obligation-v1";
pub const ROLE: &str = "x-workflow-owner-obligation-role-v1";
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Policy {
    pub version: u32,
    pub intake_definition_version_id: Uuid,
    pub intake_acceptance_transition_id: Uuid,
    pub review_definition_version_id: Uuid,
    pub review_completion_transition_id: Uuid,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Descriptor {
    pub key: String,
    pub version: u32,
    pub hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Producer {
    pub node_id: Uuid,
    pub slot_key: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Operation {
    pub version: u32,
    pub slot_key: String,
    pub allowed_operations: Vec<Descriptor>,
    pub required_for_advance: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_definition_version_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_definition_digest: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prerequisite_producer: Option<Producer>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Gate {
    pub version: u32,
    pub kind: String,
    pub producer: Producer,
}
pub fn policy(s: Option<&Value>) -> Result<Option<Policy>, String> {
    let Some(v) = s.and_then(|s| s.get(POLICY)) else {
        return Ok(None);
    };
    let p: Policy = serde_json::from_value(v.clone()).map_err(|_| "owner_policy_invalid_shape")?;
    if p.version != 1 || p.intake_definition_version_id == p.review_definition_version_id {
        return Err("owner_policy_invalid_shape".into());
    }
    Ok(Some(p))
}
pub fn role(s: Option<&Value>) -> Result<Option<&str>, String> {
    match s.and_then(|s| s.get(ROLE)) {
        None => Ok(None),
        Some(Value::String(v)) if v == "INTAKE" || v == "REVIEW" => Ok(Some(v)),
        _ => Err("owner_policy_invalid_shape".into()),
    }
}
pub fn operation(m: Option<&Value>) -> Result<Option<Operation>, String> {
    let Some(v) = m.and_then(|m| m.get("workflowOperation")) else {
        return Ok(None);
    };
    if [
        "targetDefinitionVersionId",
        "targetDefinitionDigest",
        "prerequisiteProducer",
    ]
    .iter()
    .any(|key| v.get(*key).is_some_and(Value::is_null))
    {
        return Err("owner_operation_target_invalid".into());
    }
    let op: Operation =
        serde_json::from_value(v.clone()).map_err(|_| "owner_operation_target_invalid")?;
    let mut seen = HashSet::new();
    if op.version != 1
        || op.slot_key.is_empty()
        || !op.required_for_advance
        || op.allowed_operations.is_empty()
        || op.allowed_operations.iter().any(|d| {
            d.version != 1
                || !registered(d)
                || !valid_digest(&d.hash)
                || !matches!(
                    d.key.as_str(),
                    "workflow.create_instance"
                        | "workflow.assistance.resolve_current_review"
                        | "workflow.assistance.escalate_to_human"
                )
                || !seen.insert(d.key.clone())
        })
    {
        return Err("owner_operation_target_invalid".into());
    }
    let create = op
        .allowed_operations
        .iter()
        .any(|d| d.key == "workflow.create_instance");
    if create != op.target_definition_version_id.is_some()
        || create
            != op
                .target_definition_digest
                .as_deref()
                .is_some_and(valid_digest)
    {
        return Err("owner_operation_target_invalid".into());
    }
    Ok(Some(op))
}
pub fn gate(m: Option<&Value>) -> Result<Option<Gate>, String> {
    let Some(v) = m.and_then(|m| m.get("workflowPrerequisiteGate")) else {
        return Ok(None);
    };
    let g: Gate =
        serde_json::from_value(v.clone()).map_err(|_| "owner_node_gate_producer_invalid")?;
    if g.version != 1 || g.kind != "TERMINAL_SUBMISSION" || g.producer.slot_key.is_empty() {
        return Err("owner_node_gate_producer_invalid".into());
    }
    Ok(Some(g))
}
pub fn valid_digest(v: &str) -> bool {
    v.len() == 64
        && v.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn reaches(
    edges: &HashMap<Uuid, Vec<Uuid>>,
    from: Uuid,
    to: Uuid,
    seen: &mut HashSet<Uuid>,
) -> bool {
    if from == to {
        return true;
    }
    seen.insert(from)
        && edges
            .get(&from)
            .is_some_and(|ns| ns.iter().any(|n| reaches(edges, *n, to, seen)))
}
pub fn validate(model: i16, g: &WorkflowGraph) -> Vec<GraphValidationError> {
    let mut out = Vec::new();
    let mut err = |code: &str| out.push(GraphValidationError::new(code, code));
    let p = match policy(g.context_schema.as_ref()) {
        Ok(p) => p,
        Err(e) => {
            err(&e);
            None
        }
    };
    let r = match role(g.context_schema.as_ref()) {
        Ok(r) => r,
        Err(e) => {
            err(&e);
            None
        }
    };
    if (p.is_some() || r.is_some()) && model != 3 {
        err("owner_policy_target_role_mismatch")
    }
    if p.is_some() && r.is_some() {
        err("owner_policy_recursive_target")
    }
    if p.is_some()
        && !["ownerIntakeContext", "ownerReviewContext"]
            .iter()
            .all(|k| {
                g.context_schema
                    .as_ref()
                    .and_then(|s| s.pointer(&format!("/properties/{k}/type")))
                    .is_some_and(|v| v == "object")
            })
    {
        err("owner_policy_invalid_shape")
    }
    let mut edges: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for t in &g.transitions {
        if t.transition_effect == TransitionEffect::Advance {
            edges
                .entry(t.source_node_id.into_uuid())
                .or_default()
                .push(t.target_node_id.into_uuid());
        }
    }
    if r.is_some() {
        for t in &g.transitions {
            if t.transition_effect == TransitionEffect::Advance
                && reaches(
                    &edges,
                    t.target_node_id.into_uuid(),
                    t.source_node_id.into_uuid(),
                    &mut HashSet::new(),
                )
            {
                err("owner_policy_recursive_target");
                break;
            }
        }
        let terminals: Vec<_> = g
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::TERMINAL)
            .map(|n| n.node_id.into_uuid())
            .collect();
        for n in &g.nodes {
            if n.node_type != NodeType::TERMINAL
                && !terminals
                    .iter()
                    .any(|t| reaches(&edges, n.node_id.into_uuid(), *t, &mut HashSet::new()))
            {
                err("owner_policy_recursive_target");
                break;
            }
        }
    }
    for n in &g.nodes {
        let op = match operation(n.metadata.as_ref()) {
            Ok(o) => o,
            Err(e) => {
                err(&e);
                None
            }
        };
        let gate = match gate(n.metadata.as_ref()) {
            Ok(o) => o,
            Err(e) => {
                err(&e);
                None
            }
        };
        if (op.is_some() || gate.is_some()) && (r.is_none() || n.node_type != NodeType::TASK) {
            err("owner_operation_target_invalid")
        }
        let producers = op
            .as_ref()
            .and_then(|o| o.prerequisite_producer.as_ref())
            .into_iter()
            .chain(gate.as_ref().map(|g| &g.producer));
        for pr in producers {
            let producer = g.nodes.iter().find(|x| x.node_id.into_uuid() == pr.node_id);
            if !producer.is_some_and(|pn| {
                pn.order_index < n.order_index
                    && operation(pn.metadata.as_ref())
                        .ok()
                        .flatten()
                        .is_some_and(|o| {
                            o.slot_key == pr.slot_key
                                && o.allowed_operations
                                    .iter()
                                    .any(|d| d.key == "workflow.create_instance")
                        })
                    && reaches(
                        &edges,
                        pr.node_id,
                        n.node_id.into_uuid(),
                        &mut HashSet::new(),
                    )
            }) {
                err("owner_node_gate_producer_invalid")
            }
        }
        if gate.is_some()
            && (r != Some("INTAKE")
                || g.context_schema
                    .as_ref()
                    .is_some_and(|s| s.get(super::prerequisite::MARKER).is_some())
                || g.context_schema
                    .as_ref()
                    .and_then(|s| s.pointer("/properties/workflowPrerequisite"))
                    != Some(&super::prerequisite::property_schema()))
        {
            err("owner_node_gate_producer_invalid")
        }
    }
    out
}

pub fn registry() -> Vec<Value> {
    serde_json::from_str(include_str!("owner_operation_registry.json"))
        .expect("compiled registry JSON")
}
pub fn registered(d: &Descriptor) -> bool {
    registry()
        .iter()
        .any(|r| r["descriptor"] == serde_json::to_value(d).expect("descriptor JSON"))
}
pub fn validate_arguments(d: &Descriptor, args: &Value) -> bool {
    registry()
        .iter()
        .find(|r| r["descriptor"] == serde_json::to_value(d).expect("descriptor JSON"))
        .is_some_and(|r| {
            jsonschema::validator_for(&r["canonicalInput"]["inputSchema"])
                .is_ok_and(|v| v.is_valid(args))
        })
}
