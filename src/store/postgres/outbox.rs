//! WORKFLOW_EXECUTION_CONTROL_V1 (CTR-SWEC-001/002/007/008) — the durable
//! outbox store and the canonical forum binding fact.
//!
//! Discipline: rows are queued INSIDE the committing business transaction
//! (same-tx durable facts) and drained by the background reconciler; a
//! forum/agent-core outage can delay delivery but can never lose a queued
//! fact and can never roll back a business transaction. Delivery is
//! at-least-once; dedupe rides `uq_outbox_kind_event_key` (queuing) and the
//! per-service idempotency protocols (draining).

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

/// Queue one FORUM_EVENT row (idempotent on `(outbox_kind, event_key)`).
pub async fn queue_forum_event(
    tx: &mut Transaction<'_, Postgres>,
    workflow_instance_id: Uuid,
    event_key: &str,
    payload: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO workflow_outbox
             (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
         VALUES ($1, $2, 'FORUM_EVENT', $3, $4)
         ON CONFLICT (outbox_kind, event_key) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(workflow_instance_id)
    .bind(event_key)
    .bind(payload)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Queue one EXECUTION_KICK row for a freshly created DISPATCH_INTENT
/// activation (CTR-SWEC-007). Idempotent per activation.
pub(crate) async fn queue_execution_kick(
    tx: &mut Transaction<'_, Postgres>,
    workflow_instance_id: Uuid,
    node_visit_id: Uuid,
    activation_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO workflow_outbox
             (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
         VALUES ($1, $2, 'EXECUTION_KICK', $3, $4)
         ON CONFLICT (outbox_kind, event_key) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(workflow_instance_id)
    .bind(format!("kick:{activation_id}"))
    .bind(serde_json::json!({
        "dispatchIntentId": activation_id,
        "nodeVisitId": node_visit_id,
        "workflowInstanceId": workflow_instance_id,
    }))
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// The PENDING canonical binding fact, written in the create transaction for
/// BUSINESS-class instances (CTR-SWEC-001). Non-BUSINESS instances get none
/// (no forum surface for canaries).
pub(crate) async fn queue_forum_binding(
    tx: &mut Transaction<'_, Postgres>,
    workflow_instance_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO workflow_forum_bindings (workflow_instance_id, binding_state)
         VALUES ($1, 'PENDING')
         ON CONFLICT (workflow_instance_id) DO NOTHING",
    )
    .bind(workflow_instance_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// One due outbox row (reconciler read model).
#[derive(Debug, sqlx::FromRow)]
pub(crate) struct OutboxRow {
    pub outbox_id: Uuid,
    pub workflow_instance_id: Uuid,
    pub outbox_kind: String,
    pub event_key: String,
    pub payload: serde_json::Value,
    pub attempt_count: i32,
}

/// The due batch, oldest first. `delivered_at IS NULL AND next_attempt_at <=
/// now()` mirrors the partial index.
pub(crate) async fn next_pending_batch(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<OutboxRow>, sqlx::Error> {
    sqlx::query_as::<_, OutboxRow>(
        "SELECT outbox_id, workflow_instance_id, outbox_kind, event_key, payload, attempt_count
           FROM workflow_outbox
          WHERE delivered_at IS NULL AND next_attempt_at <= now()
          ORDER BY created_at, outbox_id
          LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}

pub(crate) async fn mark_delivered(
    pool: &sqlx::PgPool,
    outbox_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE workflow_outbox SET delivered_at = now(), last_error = NULL WHERE outbox_id = $1",
    )
    .bind(outbox_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// Exponential backoff capped at 10 minutes; the row is never deleted or
/// dead-lettered (no silent loss).
pub(crate) async fn mark_attempt_failed(
    pool: &sqlx::PgPool,
    outbox_id: Uuid,
    error: &str,
) -> Result<(), sqlx::Error> {
    let sanitized: String = error
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(500)
        .collect();
    sqlx::query(
        "UPDATE workflow_outbox
            SET attempt_count = attempt_count + 1,
                next_attempt_at = now() + make_interval(secs => LEAST(POWER(2, attempt_count + 1), 600)),
                last_error = $2
          WHERE outbox_id = $1",
    )
    .bind(outbox_id)
    .bind(sanitized)
    .execute(pool)
    .await?;
    Ok(())
}

// ── binding lifecycle (drained by the reconciler, never by business tx) ────

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct ForumBindingRow {
    pub workflow_instance_id: Uuid,
    pub forum_thread_id: Option<String>,
    pub binding_state: String,
}

pub(crate) async fn get_binding(
    pool: &sqlx::PgPool,
    workflow_instance_id: Uuid,
) -> Result<Option<ForumBindingRow>, sqlx::Error> {
    sqlx::query_as::<_, ForumBindingRow>(
        "SELECT workflow_instance_id, forum_thread_id, binding_state
           FROM workflow_forum_bindings
          WHERE workflow_instance_id = $1",
    )
    .bind(workflow_instance_id)
    .fetch_optional(pool)
    .await
}

pub(crate) async fn bind_thread(
    pool: &sqlx::PgPool,
    workflow_instance_id: Uuid,
    forum_thread_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE workflow_forum_bindings
            SET forum_thread_id = $2, binding_state = 'BOUND', updated_at = now()
          WHERE workflow_instance_id = $1 AND binding_state = 'PENDING'",
    )
    .bind(workflow_instance_id)
    .bind(forum_thread_id)
    .execute(pool)
    .await?;
    Ok(())
}

/// The oldest PENDING bindings (thread ensure pass).
pub(crate) async fn pending_bindings(
    pool: &sqlx::PgPool,
    limit: i64,
) -> Result<Vec<Uuid>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT b.workflow_instance_id
           FROM workflow_forum_bindings b
          WHERE b.binding_state = 'PENDING'
          ORDER BY b.created_at, b.workflow_instance_id
          LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await
}
