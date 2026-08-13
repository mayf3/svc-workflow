//! Workflow Assistance V1 application service.

use sqlx::PgPool;
use uuid::Uuid;

pub use crate::store::postgres::workflow_instance_repository::assistance_transaction::{
    AssistanceCaseView, AssistanceCursor, AssistanceListView, AssistancePage,
    HumanRequiredAssistanceCaseView, HumanRequiredAssistancePage,
};

use crate::domain::workflow_instance::assistance::{
    AssistanceCaseStatus, AssistanceCommandResult, AssistanceError, EscalateAssistanceCommand,
    RequestAssistanceCommand, ResolveAssistanceCommand,
};
use crate::domain::workflow_instance::events::{
    COMMAND_TYPE_ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN, COMMAND_TYPE_REQUEST_WORKFLOW_ASSISTANCE,
    COMMAND_TYPE_RESOLVE_WORKFLOW_ASSISTANCE,
};
use crate::store::postgres::workflow_instance_repository::assistance_transaction;

async fn principal_exists(pool: &PgPool, principal_id: Uuid) -> Result<(), AssistanceError> {
    let exists: bool =
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM principals WHERE principal_id=$1)")
            .bind(principal_id)
            .fetch_one(pool)
            .await
            .map_err(|error| AssistanceError::StorageError(error.to_string()))?;
    if exists {
        Ok(())
    } else {
        Err(AssistanceError::PrincipalNotFound)
    }
}

fn command_hash(
    command_type: &str,
    schema_version: &str,
    principal_id: Uuid,
    route_parameters: serde_json::Value,
    request_body: serde_json::Value,
) -> Result<String, AssistanceError> {
    jcs_canonicalize::sha256_jcs_hex(&serde_json::json!({
        "commandSchemaVersion": schema_version,
        "commandType": command_type,
        "routeParameters": route_parameters,
        "requestBody": {
            "principalId": principal_id.to_string(),
            "body": request_body
        }
    }))
    .map_err(|error| AssistanceError::InternalConsistency(error.to_string()))
}

pub async fn request_assistance(
    pool: &PgPool,
    command: RequestAssistanceCommand,
) -> Result<AssistanceCommandResult, AssistanceError> {
    let actor = command.principal_id.into_uuid();
    principal_exists(pool, actor).await?;
    let hash = command_hash(
        COMMAND_TYPE_REQUEST_WORKFLOW_ASSISTANCE,
        &command.command_schema_version,
        actor,
        serde_json::json!({"workflowInstanceId": command.workflow_instance_id}),
        serde_json::json!({
            "currentNodeVisitId": command.current_node_visit_id,
            "expectedWorkflowStateVersion": command.expected_workflow_state_version,
            "request": command.request,
        }),
    )?;
    assistance_transaction::request_assistance(pool, command, &hash).await
}

pub async fn escalate_assistance_to_human(
    pool: &PgPool,
    command: EscalateAssistanceCommand,
) -> Result<AssistanceCommandResult, AssistanceError> {
    let actor = command.principal_id.into_uuid();
    principal_exists(pool, actor).await?;
    let hash = command_hash(
        COMMAND_TYPE_ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN,
        &command.command_schema_version,
        actor,
        serde_json::json!({"assistanceCaseId": command.assistance_case_id}),
        serde_json::json!({
            "expectedWorkflowStateVersion": command.expected_workflow_state_version,
            "escalation": command.escalation,
        }),
    )?;
    assistance_transaction::escalate_assistance(pool, command, &hash).await
}

pub async fn resolve_assistance(
    pool: &PgPool,
    command: ResolveAssistanceCommand,
) -> Result<AssistanceCommandResult, AssistanceError> {
    let actor = command.principal_id.into_uuid();
    principal_exists(pool, actor).await?;
    let hash = command_hash(
        COMMAND_TYPE_RESOLVE_WORKFLOW_ASSISTANCE,
        &command.command_schema_version,
        actor,
        serde_json::json!({"assistanceCaseId": command.assistance_case_id}),
        serde_json::json!({
            "expectedWorkflowStateVersion": command.expected_workflow_state_version,
            "resolution": command.resolution,
        }),
    )?;
    assistance_transaction::resolve_assistance(pool, command, &hash).await
}

pub async fn list_assistance(
    pool: &PgPool,
    actor: Uuid,
    view: AssistanceListView,
    status: Option<AssistanceCaseStatus>,
    before: Option<AssistanceCursor>,
    limit: u32,
) -> Result<AssistancePage, AssistanceError> {
    assistance_transaction::list_assistance(pool, actor, view, status, before, limit).await
}

pub async fn get_assistance_case(
    pool: &PgPool,
    actor: Uuid,
    case_id: Uuid,
) -> Result<AssistanceCaseView, AssistanceError> {
    assistance_transaction::get_assistance_case(pool, actor, case_id).await
}

pub async fn list_human_required_assistance(
    pool: &PgPool,
    actor: Uuid,
    before: Option<AssistanceCursor>,
    limit: u32,
) -> Result<HumanRequiredAssistancePage, AssistanceError> {
    assistance_transaction::list_human_required_assistance(pool, actor, before, limit).await
}

pub async fn get_human_required_assistance_case(
    pool: &PgPool,
    actor: Uuid,
    case_id: Uuid,
) -> Result<HumanRequiredAssistanceCaseView, AssistanceError> {
    assistance_transaction::get_human_required_assistance_case(pool, actor, case_id).await
}
