//! Read-only, provenance-checked Owner Assistance resume evidence.

use serde::Serialize;
use sqlx::FromRow;
use uuid::Uuid;

use crate::application::workflow_instance::query_types::WorkflowQueryError;

use super::query_visibility;

#[derive(Debug, Clone, FromRow, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnerAssistanceResumeEvidence {
    pub dispatch_intent_id: Uuid,
    pub workflow_instance_id: Uuid,
    pub node_visit_id: Uuid,
    pub owner_principal_id: Uuid,
    pub activation_state: String,
    pub workflow_state_version: i32,
    pub assistance_case_id: Uuid,
    pub request_event_sequence: i32,
    pub resolution_event_sequence: i32,
    pub requester_principal_id: Uuid,
    pub resolver_principal_id: Uuid,
    pub ingress: String,
    pub reason: Option<String>,
    pub last_attempt_id: Option<String>,
}

fn storage(error: sqlx::Error) -> WorkflowQueryError {
    WorkflowQueryError::StorageError(error.to_string())
}

/// Query one exact active dispatch intent and return its earliest resolved
/// Assistance case whose request, resolution, command receipts, and events
/// prove the current visit's lifecycle after the caller's durable baseline.
/// The scheduler role check and projection share one REPEATABLE READ snapshot.
pub(crate) async fn get_owner_assistance_resume_evidence(
    pool: &sqlx::PgPool,
    actor: Uuid,
    dispatch_intent_id: Uuid,
    previous_attempt_id: &str,
    pre_delivery_workflow_state_version: i32,
) -> Result<Option<OwnerAssistanceResumeEvidence>, WorkflowQueryError> {
    let mut tx = query_visibility::begin_snapshot(pool).await?;
    if !query_visibility::check_global_scheduler_read(&mut tx, actor).await? {
        tx.commit().await.map_err(storage)?;
        return Err(WorkflowQueryError::SchedulerReadRoleRequired);
    }

    let evidence = sqlx::query_as::<_, OwnerAssistanceResumeEvidence>(
        r#"
        WITH active_intent AS (
            SELECT a.activation_id AS dispatch_intent_id,
                   a.workflow_instance_id,
                   a.node_visit_id,
                   a.owner_principal_id,
                   wi.workflow_state_version,
                   nv.assignee_principal_id
              FROM workflow_activations a
              JOIN workflow_instances wi
                ON wi.workflow_instance_id = a.workflow_instance_id
              JOIN workflow_node_visits nv
                ON nv.node_visit_id = a.node_visit_id
               AND nv.workflow_instance_id = a.workflow_instance_id
              JOIN workflow_node_definitions nd ON nd.node_id = nv.node_id
              LEFT JOIN workflow_activation_closures ac
                ON ac.activation_id = a.activation_id
             WHERE a.activation_id = $1
               AND a.activation_kind = 'DISPATCH_INTENT'
               AND ac.activation_id IS NULL
               AND wi.execution_class = 'BUSINESS'
               AND wi.cancelled = FALSE
               AND wi.archived_at IS NULL
               AND wi.current_node_visit_id = a.node_visit_id
               AND nv.assignee_principal_id = a.owner_principal_id
               AND nd.node_type <> 'TERMINAL'
               AND NOT EXISTS (
                   SELECT 1
                     FROM workflow_assistance_cases open_case
                    WHERE open_case.workflow_instance_id = a.workflow_instance_id
                      AND open_case.node_visit_id = a.node_visit_id
                      AND open_case.status IN ('OWNER_PENDING', 'HUMAN_REQUIRED')
               )
        ),
        qualifying AS (
            SELECT ai.dispatch_intent_id,
                   ai.workflow_instance_id,
                   ai.node_visit_id,
                   ai.owner_principal_id,
                   ai.workflow_state_version,
                   c.assistance_case_id,
                   request_event.event_sequence AS request_event_sequence,
                   resolution_event.event_sequence AS resolution_event_sequence,
                   c.requested_by_principal_id AS requester_principal_id,
                   c.resolved_by_principal_id AS resolver_principal_id,
                   CASE
                       WHEN request_receipt.response_body = jsonb_build_object('source', 'execution_policy')
                           THEN 'execution_policy'
                       ELSE 'assignee'
                   END AS ingress,
                   CASE
                       WHEN request_receipt.response_body = jsonb_build_object('source', 'execution_policy')
                           THEN c.request_payload->'supportingPayload'->>'reason'
                       ELSE NULL
                   END AS reason,
                   CASE
                       WHEN request_receipt.response_body = jsonb_build_object('source', 'execution_policy')
                           THEN c.request_payload->'supportingPayload'->>'lastAttemptId'
                       ELSE NULL
                   END AS last_attempt_id
              FROM active_intent ai
              JOIN workflow_assistance_cases c
                ON c.workflow_instance_id = ai.workflow_instance_id
               AND c.node_visit_id = ai.node_visit_id
               AND c.status = 'RESOLVED'
              JOIN workflow_command_receipts request_receipt
                ON request_receipt.command_id = c.request_command_id
               AND request_receipt.command_type = 'REQUEST_WORKFLOW_ASSISTANCE'
               AND request_receipt.receipt_status = 'COMPLETED'
               AND request_receipt.response_status BETWEEN 200 AND 299
               AND request_receipt.principal_id = c.requested_by_principal_id
              JOIN workflow_command_receipts resolution_receipt
                ON resolution_receipt.command_id = c.resolution_command_id
               AND resolution_receipt.command_type = 'RESOLVE_WORKFLOW_ASSISTANCE'
               AND resolution_receipt.receipt_status = 'COMPLETED'
               AND resolution_receipt.response_status BETWEEN 200 AND 299
               AND resolution_receipt.principal_id = c.resolved_by_principal_id
              JOIN workflow_events request_event
                ON request_event.command_id = c.request_command_id
               AND request_event.workflow_instance_id = c.workflow_instance_id
               AND request_event.event_type = 'ASSISTANCE_REQUESTED'
               AND request_event.source_node_visit_id = c.node_visit_id
               AND request_event.target_node_visit_id = c.node_visit_id
               AND request_event.actor_principal_id = c.requested_by_principal_id
               AND request_event.event_data->>'assistanceCaseId' = c.assistance_case_id::text
               AND request_event.event_data->>'newStatus' = 'OWNER_PENDING'
               AND request_event.event_sequence = request_event.new_workflow_state_version
              JOIN workflow_events resolution_event
                ON resolution_event.command_id = c.resolution_command_id
               AND resolution_event.workflow_instance_id = c.workflow_instance_id
               AND resolution_event.event_type = 'ASSISTANCE_RESOLVED'
               AND resolution_event.source_node_visit_id = c.node_visit_id
               AND resolution_event.target_node_visit_id = c.node_visit_id
               AND resolution_event.actor_principal_id = c.resolved_by_principal_id
               AND resolution_event.event_data->>'assistanceCaseId' = c.assistance_case_id::text
               AND resolution_event.event_data->>'newStatus' = 'RESOLVED'
               AND resolution_event.event_sequence = resolution_event.new_workflow_state_version
             WHERE request_event.event_sequence > $3
               AND resolution_event.event_sequence > request_event.event_sequence
               AND resolution_event.event_sequence <= ai.workflow_state_version
               AND (
                   (
                       request_receipt.response_body->>'source' IS NULL
                       AND request_receipt.response_body->>'assistanceCaseId' = c.assistance_case_id::text
                       AND c.requested_by_principal_id = ai.assignee_principal_id
                       AND c.request_payload->'supportingPayload'->>'source' IS DISTINCT FROM 'execution_policy'
                   )
                   OR
                   (
                       request_receipt.response_body = jsonb_build_object('source', 'execution_policy')
                       AND EXISTS (
                           SELECT 1
                             FROM workflow_command_receipts system_receipt
                            WHERE request_receipt.idempotency_key = 'policy-req:' || system_receipt.command_id::text
                              AND system_receipt.command_type = 'SYSTEM_EXECUTION_ESCALATION'
                              AND system_receipt.receipt_status = 'COMPLETED'
                              AND system_receipt.response_status BETWEEN 200 AND 299
                              AND system_receipt.principal_id = request_receipt.principal_id
                              AND system_receipt.response_body->>'assistanceCaseId' = c.assistance_case_id::text
                       )
                       AND c.request_payload->'supportingPayload'->>'source' = 'execution_policy'
                       AND c.request_payload->'supportingPayload'->>'reason'
                           IN ('ATTEMPTS_EXHAUSTED', 'STALE_LOOP_EXHAUSTED')
                       AND NULLIF(c.request_payload->'supportingPayload'->>'lastAttemptId', '') IS NOT NULL
                       AND c.request_payload->'supportingPayload'->>'lastAttemptId' = $2
                       AND c.request_payload->'supportingPayload'->>'dispatchIntentId' = ai.dispatch_intent_id::text
                   )
               )
        )
        SELECT dispatch_intent_id,
               workflow_instance_id,
               node_visit_id,
               owner_principal_id,
               'ACTIVE'::text AS activation_state,
               workflow_state_version,
               assistance_case_id,
               request_event_sequence,
               resolution_event_sequence,
               requester_principal_id,
               resolver_principal_id,
               ingress,
               reason,
               last_attempt_id
          FROM qualifying
         ORDER BY resolution_event_sequence, assistance_case_id
         LIMIT 1
        "#,
    )
    .bind(dispatch_intent_id)
    .bind(previous_attempt_id)
    .bind(pre_delivery_workflow_state_version)
    .fetch_optional(&mut *tx)
    .await
    .map_err(storage)?;

    tx.commit().await.map_err(storage)?;
    Ok(evidence)
}
