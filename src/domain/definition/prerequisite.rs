//! Closed immutable Context opt-in (CTR-SWPO-001). No implicit field inference.
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
pub const MARKER: &str = "x-workflow-prerequisite-v1";
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Reference {
    pub kind: Kind,
    pub workflow_instance_id: Uuid,
    pub definition_version_id: Uuid,
    pub transition_id: Uuid,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Kind {
    #[serde(rename = "TERMINAL_SUBMISSION")]
    TerminalSubmission,
}
pub fn property_schema() -> Value {
    json!({"type":"object","required":["kind","workflowInstanceId","definitionVersionId","transitionId"],"properties":{"kind":{"const":"TERMINAL_SUBMISSION"},"workflowInstanceId":{"type":"string","format":"uuid"},"definitionVersionId":{"type":"string","format":"uuid"},"transitionId":{"type":"string","format":"uuid"}},"additionalProperties":false})
}
pub(crate) fn contains_marker(v: &Value) -> bool {
    match v {
        Value::Object(m) => m.contains_key(MARKER) || m.values().any(contains_marker),
        Value::Array(a) => a.iter().any(contains_marker),
        _ => false,
    }
}
/// Annotation keys are reserved at every depth, including other schema kinds.
pub fn validate_schema(schema: &Value) -> Result<bool, String> {
    let Some(root) = schema.as_object() else {
        return Ok(false);
    };
    if root.iter().any(|(k, v)| k != MARKER && contains_marker(v)) {
        return Err("prerequisite annotation is permitted only at Context root".into());
    }
    let Some(marker) = root.get(MARKER) else {
        return Ok(false);
    };
    if marker != &Value::Bool(true)
        || root.get("type") != Some(&json!("object"))
        || !root
            .get("required")
            .and_then(Value::as_array)
            .is_some_and(|a| a.contains(&json!("workflowPrerequisite")))
        || schema.pointer("/properties/workflowPrerequisite") != Some(&property_schema())
    {
        return Err(
            "prerequisite annotation requires the exact closed required Context property".into(),
        );
    }
    Ok(true)
}
pub fn parse(schema: Option<&Value>, context: &Value) -> Result<Option<Reference>, String> {
    let Some(schema) = schema else {
        return Ok(None);
    };
    if !validate_schema(schema)? {
        return Ok(None);
    }
    serde_json::from_value(
        context
            .get("workflowPrerequisite")
            .cloned()
            .ok_or("missing workflowPrerequisite")?,
    )
    .map(Some)
    .map_err(|_| "invalid closed workflowPrerequisite".into())
}
#[cfg(test)]
mod tests;
