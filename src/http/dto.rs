//! Strict transport DTOs for the internal API.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::workflow_instance::create::CreateWorkflowInstanceResult;
use crate::application::workflow_instance::execute_transition::ExecuteWorkflowTransitionResult;
use crate::application::workflow_instance::query_types::{
    WorkflowEventItem, WorkflowInstanceDetail,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateWorkflowInstanceRequest {
    pub domain_id: Uuid,
    pub definition_version_id: Uuid,
    pub external_reference: Option<String>,
    pub external_url: Option<String>,
    pub metadata: serde_json::Value,
    pub context_payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWorkflowInstanceResponse {
    pub workflow_instance_id: Uuid,
    pub workflow_state_version: i32,
    pub current_context_revision_id: Uuid,
    pub current_node_visit_id: Uuid,
    pub event_sequence: i32,
}

impl From<CreateWorkflowInstanceResult> for CreateWorkflowInstanceResponse {
    fn from(value: CreateWorkflowInstanceResult) -> Self {
        Self {
            workflow_instance_id: value.workflow_instance_id,
            workflow_state_version: value.workflow_state_version,
            current_context_revision_id: value.current_context_revision_id,
            current_node_visit_id: value.current_node_visit_id,
            event_sequence: value.event_sequence,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExecuteWorkflowTransitionRequest {
    pub transition_definition_id: Uuid,
    pub expected_workflow_state_version: i32,
    pub submission_payload: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteWorkflowTransitionResponse {
    pub workflow_instance_id: Uuid,
    pub workflow_state_version: i32,
    pub current_context_revision_id: Uuid,
    pub source_node_visit_id: Uuid,
    pub current_node_visit_id: Uuid,
    pub submission_id: Option<Uuid>,
    pub event_sequence: i32,
}

impl From<ExecuteWorkflowTransitionResult> for ExecuteWorkflowTransitionResponse {
    fn from(value: ExecuteWorkflowTransitionResult) -> Self {
        Self {
            workflow_instance_id: value.workflow_instance_id,
            workflow_state_version: value.workflow_state_version,
            current_context_revision_id: value.current_context_revision_id,
            source_node_visit_id: value.source_node_visit_id,
            current_node_visit_id: value.current_node_visit_id,
            submission_id: value.submission_id,
            event_sequence: value.event_sequence,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TimelineQuery {
    pub after: Option<i32>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WorklistQuery {
    pub before_created_at: Option<String>,
    pub before_id: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SubmissionHistoryQuery {
    pub after_created_at: Option<String>,
    pub after_id: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DomainInstanceQuery {
    pub domain_id: Uuid,
    pub before_created_at: Option<String>,
    pub before_id: Option<String>,
    pub limit: Option<u32>,
    pub definition_key: Option<String>,
    /// One of `active`, `terminal`, `all`. Invalid values produce a 422
    /// at the handler layer.
    pub lifecycle: Option<String>,
    pub current_node_key: Option<String>,
    pub assignee_principal_id: Option<Uuid>,
    /// One of `active`, `cancelled`, `archived`, `all`. Invalid values
    /// produce a 422 at the handler layer.
    pub status: Option<String>,
}

/// Query DTO for the global (cross-domain) instance list. Same filters as
/// `DomainInstanceQuery` minus `domainId` — the caller's coordinator role
/// replaces the domain scope.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GlobalInstanceQuery {
    pub before_created_at: Option<String>,
    pub before_id: Option<String>,
    pub limit: Option<u32>,
    pub definition_key: Option<String>,
    /// One of `active`, `terminal`, `all`. Invalid values produce a 422
    /// at the handler layer.
    pub lifecycle: Option<String>,
    pub current_node_key: Option<String>,
    pub assignee_principal_id: Option<Uuid>,
    /// One of `active`, `cancelled`, `archived`, `all`. Invalid values
    /// produce a 422 at the handler layer.
    pub status: Option<String>,
}

/// Validate and convert a lifecycle string to the strong type.
/// Returns an error code + message tuple for invalid values.
pub(crate) fn parse_lifecycle_param(
    lifecycle: &Option<String>,
) -> Result<
    Option<crate::application::workflow_instance::query_types::LifecycleFilter>,
    (&'static str, &'static str),
> {
    match lifecycle.as_deref() {
        None => Ok(None),
        Some("active") => Ok(Some(
            crate::application::workflow_instance::query_types::LifecycleFilter::Active,
        )),
        Some("terminal") => Ok(Some(
            crate::application::workflow_instance::query_types::LifecycleFilter::Terminal,
        )),
        Some("all") => Ok(Some(
            crate::application::workflow_instance::query_types::LifecycleFilter::All,
        )),
        Some(_) => Err((
            "invalid_lifecycle",
            "lifecycle must be 'active', 'terminal', or 'all'",
        )),
    }
}

/// Validate and convert a status string to the strong type.
/// Returns an error code + message tuple for invalid values.
pub(crate) fn parse_status_param(
    status: &Option<String>,
) -> Result<
    Option<crate::application::workflow_instance::query_types::StatusFilter>,
    (&'static str, &'static str),
> {
    match status.as_deref() {
        None => Ok(None),
        Some("active") => Ok(Some(
            crate::application::workflow_instance::query_types::StatusFilter::Active,
        )),
        Some("cancelled") => Ok(Some(
            crate::application::workflow_instance::query_types::StatusFilter::Cancelled,
        )),
        Some("archived") => Ok(Some(
            crate::application::workflow_instance::query_types::StatusFilter::Archived,
        )),
        Some("all") => Ok(Some(
            crate::application::workflow_instance::query_types::StatusFilter::All,
        )),
        Some(_) => Err((
            "invalid_status",
            "status must be 'active', 'cancelled', 'archived', or 'all'",
        )),
    }
}

impl DomainInstanceQuery {
    /// Validate and convert the lifecycle string to the strong type.
    /// Returns an error code + message tuple for invalid values.
    pub(crate) fn parse_lifecycle(
        &self,
    ) -> Result<
        Option<crate::application::workflow_instance::query_types::LifecycleFilter>,
        (&'static str, &'static str),
    > {
        parse_lifecycle_param(&self.lifecycle)
    }

    /// Validate and convert the status string to the strong type.
    /// Returns an error code + message tuple for invalid values.
    pub(crate) fn parse_status(
        &self,
    ) -> Result<
        Option<crate::application::workflow_instance::query_types::StatusFilter>,
        (&'static str, &'static str),
    > {
        parse_status_param(&self.status)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineResponse {
    pub items: Vec<WorkflowEventItem>,
    pub next_cursor: Option<i32>,
}

pub fn detail_response(detail: WorkflowInstanceDetail) -> serde_json::Value {
    match detail {
        WorkflowInstanceDetail::Full(detail) => {
            serde_json::json!({ "visibility": "full", "detail": detail })
        }
        WorkflowInstanceDetail::HistoricalParticipant(detail) => serde_json::json!({
            "visibility": "historical_participant",
            "detail": detail
        }),
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionResponse {
    pub service: &'static str,
    pub version: &'static str,
    pub git_sha: &'static str,
    /// "clean" | "dirty" | "unknown" — "clean" only if the binary was built
    /// from a Git tree with no uncommitted/untracked entries.
    pub git_tree_state: &'static str,
    /// UTC ISO-8601 build timestamp.
    pub build_timestamp: &'static str,
    pub schema_version: &'static str,
    pub api_contract_version: &'static str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actor_fields_are_rejected() {
        let value = serde_json::json!({
            "domainId": Uuid::new_v4(),
            "definitionVersionId": Uuid::new_v4(),
            "metadata": {},
            "contextPayload": {},
            "principalId": Uuid::new_v4()
        });
        assert!(serde_json::from_value::<CreateWorkflowInstanceRequest>(value).is_err());
    }
}

// ---------------------------------------------------------------------------
// Provisioning DTOs
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvisionPrincipalRequest {
    pub principal_id: Uuid,
    pub principal_type: String,
    pub enabled: bool,
    pub source: String,
    pub source_revision: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionPrincipalResponse {
    pub principal_id: Uuid,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvisionDomainRequest {
    pub domain_id: Uuid,
    pub domain_key: String,
    pub display_name: Option<String>,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProvisionDomainResponse {
    pub domain_id: Uuid,
    pub domain_key: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvisionRoleBindingRequest {
    pub role_key: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProvisionGlobalRoleBindingRequest {
    pub role_key: String,
    pub enabled: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevokeGlobalRoleBindingRequest {
    pub role_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReplaceOwnerRequest {
    pub new_owner_principal_id: Uuid,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevokeRoleBindingRequest {
    pub role_key: String,
}
