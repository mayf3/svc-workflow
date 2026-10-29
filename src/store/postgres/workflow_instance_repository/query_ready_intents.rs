//! READY-only due poll. Every evaluation and final pre-LIMIT filter share one snapshot.
use super::{
    prerequisite,
    query_dispatch_intents::{due_rows, parse_due_limit, DueDispatchIntent},
    query_visibility,
};
use crate::application::workflow_instance::query_types::WorkflowQueryError;
use sqlx::PgPool;
use uuid::Uuid;
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadyDispatchIntent {
    #[serde(flatten)]
    pub due: DueDispatchIntent,
    pub readiness_status: &'static str,
}
fn storage(e: sqlx::Error) -> WorkflowQueryError {
    WorkflowQueryError::StorageError(e.to_string())
}
pub async fn list(
    pool: &PgPool,
    actor: Uuid,
    limit: i64,
    cursor: Option<(chrono::DateTime<chrono::Utc>, Uuid)>,
) -> Result<Vec<ReadyDispatchIntent>, WorkflowQueryError> {
    parse_due_limit(Some(limit))?;
    let mut tx = query_visibility::begin_snapshot(pool).await?;
    if !query_visibility::check_global_scheduler_read(&mut tx, actor).await? {
        return Err(WorkflowQueryError::SchedulerReadRoleRequired);
    }
    // No capped raw page: a blocked prefix of any size consumes zero output capacity.
    // JCS digests are verified in the shared Rust evaluator; the final database query
    // filters the resulting IDs BEFORE ordering/keyset LIMIT. No SQL function/migration.
    let candidates:Vec<(Uuid,Uuid)>=sqlx::query_as("SELECT a.activation_id,a.workflow_instance_id FROM workflow_activations a JOIN workflow_instances wi USING(workflow_instance_id) JOIN domains d USING(domain_id) WHERE a.activation_kind='DISPATCH_INTENT' AND wi.execution_class='BUSINESS' AND NOT wi.cancelled AND wi.archived_at IS NULL AND d.enabled AND NOT EXISTS(SELECT 1 FROM workflow_activation_closures c WHERE c.activation_id=a.activation_id)").fetch_all(&mut *tx).await.map_err(storage)?;
    let mut eligible = Vec::new();
    for (id, instance) in candidates {
        if prerequisite::project(&mut tx, instance)
            .await?
            .is_none_or(|v| v["status"] == "READY")
        {
            eligible.push(id)
        }
    }
    let due = due_rows(&mut tx, limit, cursor, Some(&eligible)).await?;
    tx.commit().await.map_err(storage)?;
    Ok(due
        .into_iter()
        .map(|due| ReadyDispatchIntent {
            due,
            readiness_status: "READY",
        })
        .collect())
}
