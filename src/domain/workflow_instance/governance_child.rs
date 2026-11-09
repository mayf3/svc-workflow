//! Closed GCRS commands and acceptance references. No business-state authority.
use crate::domain::ids::{AssistanceCaseId, PrincipalId, TransitionId, WorkflowInstanceId};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct BindGovernanceChildCommand {
    pub principal_id: PrincipalId,
    pub idempotency_key: String,
    pub command_schema_version: String,
    pub assistance_case_id: AssistanceCaseId,
    pub governance_child_instance_id: WorkflowInstanceId,
    pub acceptance_transition_id: TransitionId,
    pub expected_workflow_state_version: i32,
}
#[derive(Debug, Clone)]
pub struct CreateAndBindGovernanceChildCommand {
    pub principal_id: PrincipalId,
    pub idempotency_key: String,
    pub command_schema_version: String,
    pub assistance_case_id: AssistanceCaseId,
    pub definition_version_id: crate::domain::ids::DefinitionVersionId,
    pub context_payload: Value,
    pub acceptance_transition_id: TransitionId,
    pub expected_workflow_state_version: i32,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceAcceptance {
    pub binding_id: Uuid,
    pub acceptance_contract_digest: String,
    pub source_request_payload_digest: String,
    pub verdict: Verdict,
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Accepted,
    NotAccepted,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GovernanceChildReview {
    pub binding_id: Uuid,
    pub terminal_event_id: Uuid,
    pub submission_id: Uuid,
    pub submission_digest: String,
    pub acceptance_contract_digest: String,
}
pub fn declares_acceptance(schema: &Value) -> bool {
    let names = [
        "bindingId",
        "acceptanceContractDigest",
        "sourceRequestPayloadDigest",
        "verdict",
    ];
    let e = &schema["properties"]["governanceAcceptance"];
    schema["type"] == "object"
        && schema["required"]
            .as_array()
            .is_some_and(|a| a.contains(&Value::from("governanceAcceptance")))
        && e["type"] == "object"
        && e["additionalProperties"] == false
        && e["required"].as_array().is_some_and(|a| {
            a.len() == names.len() && names.iter().all(|k| a.contains(&Value::from(*k)))
        })
        && e["properties"].as_object().is_some_and(|m| {
            m.len() == names.len()
                && names
                    .iter()
                    .all(|k| m.get(*k).is_some_and(|v| v["type"] == "string"))
        })
        && e["properties"]["verdict"]["enum"]
            .as_array()
            .is_some_and(|a| {
                a.len() == 2
                    && a.contains(&Value::from("ACCEPTED"))
                    && a.contains(&Value::from("NOT_ACCEPTED"))
            })
}
pub fn digest_shape(d: &str) -> bool {
    d.len() == 64
        && d.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct GovernanceReviewWake {
    pub phase: GovernanceReviewPhase,
    pub workflow_instance_id: Uuid,
    pub node_visit_id: Uuid,
    pub assistance_case_id: Uuid,
    pub owner_principal_id: Uuid,
    pub owner_binding_id: Uuid,
    pub review_wake_id: Uuid,
    pub binding_id: Uuid,
    pub governance_child_instance_id: Uuid,
    pub child_node_visit_id: Uuid,
    pub child_definition_version_id: Uuid,
    pub child_terminal_event_id: Uuid,
    pub acceptance_transition_id: Uuid,
    pub acceptance_contract_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_submission_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub child_submission_digest: Option<String>,
}
#[derive(Debug, Deserialize, Serialize)]
pub enum GovernanceReviewPhase {
    #[serde(rename = "GOVERNANCE_CHILD_REVIEW")]
    GovernanceChildReview,
}
impl GovernanceReviewWake {
    pub fn parse(value: &Value) -> Result<Self, String> {
        let p: Self = serde_json::from_value(value.clone())
            .map_err(|_| "invalid closed governance review wake".to_string())?;
        if !digest_shape(&p.acceptance_contract_digest)
            || p.child_submission_id.is_some() != p.child_submission_digest.is_some()
            || p.child_submission_digest
                .as_ref()
                .is_some_and(|d| !digest_shape(d))
            || value.get("childSubmissionId").is_some_and(Value::is_null)
            || value
                .get("childSubmissionDigest")
                .is_some_and(Value::is_null)
        {
            return Err("invalid conditional review submission reference".into());
        }
        Ok(p)
    }
}

/// Preserve reserved-object duplicates at the HTTP boundary, only for linked
/// cases. Ordinary supportingPayload members keep their existing opaque form.
pub fn validate_review_json(raw: &str) -> Result<(), serde_json::Error> {
    struct ReviewMap;
    impl<'de> serde::de::Visitor<'de> for ReviewMap {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("supportingPayload object")
        }
        fn visit_map<M: serde::de::MapAccess<'de>>(self, mut map: M) -> Result<(), M::Error> {
            let mut seen = false;
            while let Some(key) = map.next_key::<String>()? {
                if key == "governanceChildReview" {
                    if seen {
                        return Err(serde::de::Error::duplicate_field("governanceChildReview"));
                    }
                    seen = true;
                    map.next_value::<GovernanceChildReview>()?;
                } else {
                    map.next_value::<serde::de::IgnoredAny>()?;
                }
            }
            Ok(())
        }
    }
    use serde::Deserializer;
    let mut d = serde_json::Deserializer::from_str(raw);
    d.deserialize_map(ReviewMap)?;
    d.end()
}
