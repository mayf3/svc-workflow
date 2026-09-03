//! Due DISPATCH_INTENT scheduler read (VISIT_ACTIVATION_V1).
//!
//! Implements CTR-VAI-009 of SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1:
//! the singular due predicate is `active Dispatch Intent AND
//! nextEligibleAt <= authoritative now`. The record contains exactly the
//! v0.4.0 §5.7 minimum projection and nothing else. The
//! `GLOBAL_SCHEDULER_READ` role check runs inside the same read snapshot as
//! the query (CTR-VAI-014).

use uuid::Uuid;

use super::query_visibility;
use crate::application::workflow_instance::query_types::WorkflowQueryError;

/// One due Dispatch Intent — the minimum Scheduler-facing projection
/// (v0.4.0 §5.7). Field names on the wire are camelCase.
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DueDispatchIntent {
    #[sqlx(rename = "dispatchIntentId")]
    pub dispatch_intent_id: Uuid,
    #[sqlx(rename = "nodeVisitId")]
    pub node_visit_id: Uuid,
    #[sqlx(rename = "workflowInstanceId")]
    pub workflow_instance_id: Uuid,
    #[sqlx(rename = "ownerPrincipalId")]
    pub owner_principal_id: Uuid,
    #[sqlx(rename = "nextEligibleAt")]
    pub next_eligible_at: chrono::DateTime<chrono::Utc>,
    #[sqlx(rename = "createdAt")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    #[sqlx(rename = "updatedAt")]
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn storage(error: sqlx::Error) -> WorkflowQueryError {
    WorkflowQueryError::StorageError(error.to_string())
}

/// List active due Dispatch Intents ordered by (nextEligibleAt, activation).
///
/// The caller's `GLOBAL_SCHEDULER_READ` binding is verified inside the same
/// REPEATABLE READ snapshot that performs the query; a missing binding
/// yields `WorkflowQueryError::SchedulerReadRoleRequired` (403
/// `scheduler_read_role_required` at the HTTP boundary).
pub(crate) async fn list_due_dispatch_intents(
    pool: &sqlx::PgPool,
    actor: Uuid,
    limit: i64,
) -> Result<Vec<DueDispatchIntent>, WorkflowQueryError> {
    let mut tx = query_visibility::begin_snapshot(pool).await?;

    let has_role =
        query_visibility::check_global_scheduler_read(&mut tx, actor).await?;
    if !has_role {
        tx.commit().await.map_err(storage)?;
        return Err(WorkflowQueryError::SchedulerReadRoleRequired);
    }

    let intents: Vec<DueDispatchIntent> = sqlx::query_as(
        r#"
        SELECT a.activation_id           AS "dispatchIntentId",
               a.node_visit_id           AS "nodeVisitId",
               a.workflow_instance_id    AS "workflowInstanceId",
               a.owner_principal_id      AS "ownerPrincipalId",
               COALESCE(
                   (SELECT e.new_next_eligible_at
                      FROM workflow_dispatch_eligibility_events e
                     WHERE e.activation_id = a.activation_id
                     ORDER BY e.created_at DESC, e.eligibility_event_id DESC
                     LIMIT 1),
                   a.initial_next_eligible_at
               )                         AS "nextEligibleAt",
               a.activation_at           AS "createdAt",
               COALESCE(
                   (SELECT e.created_at
                      FROM workflow_dispatch_eligibility_events e
                     WHERE e.activation_id = a.activation_id
                     ORDER BY e.created_at DESC, e.eligibility_event_id DESC
                     LIMIT 1),
                   a.created_at
               )                         AS "updatedAt"
          FROM workflow_activations a
          JOIN workflow_instances wi
            ON wi.workflow_instance_id = a.workflow_instance_id
          LEFT JOIN workflow_activation_closures c
            ON c.activation_id = a.activation_id
         WHERE a.activation_kind = 'DISPATCH_INTENT'
           AND c.activation_id IS NULL
           AND wi.cancelled = FALSE
           AND wi.archived_at IS NULL
           AND COALESCE(
                   (SELECT e.new_next_eligible_at
                      FROM workflow_dispatch_eligibility_events e
                     WHERE e.activation_id = a.activation_id
                     ORDER BY e.created_at DESC, e.eligibility_event_id DESC
                     LIMIT 1),
                   a.initial_next_eligible_at
               ) <= now()
         ORDER BY "nextEligibleAt", a.activation_id
         LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(&mut *tx)
    .await
    .map_err(storage)?;

    tx.commit().await.map_err(storage)?;
    Ok(intents)
}

/// Validate the limit parameter for the due poll (1..=100).
pub(crate) fn parse_due_limit(limit: Option<i64>) -> Result<i64, WorkflowQueryError> {
    match limit {
        None => Ok(50),
        Some(v) if (1..=100).contains(&v) => Ok(v),
        Some(_) => Err(WorkflowQueryError::InvalidPagination("limit must be 1-100".to_string())),
    }
}
