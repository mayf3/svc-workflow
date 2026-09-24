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
    // Older BUSINESS instances have no 0027 binding row. The first new event
    // establishes it in the same transaction; test-class instances have no
    // Forum projection and must not leave undrainable outbox rows.
    let class: String = sqlx::query_scalar(
        "SELECT execution_class::text FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(workflow_instance_id)
    .fetch_one(&mut **tx)
    .await?;
    if class != "BUSINESS" {
        return Ok(());
    }
    queue_forum_binding(tx, workflow_instance_id).await?;
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
        "SELECT o.outbox_id, o.workflow_instance_id, o.outbox_kind, o.event_key,
                o.payload, o.attempt_count
           FROM workflow_outbox o
          WHERE o.delivered_at IS NULL AND o.next_attempt_at <= now()
            AND (o.outbox_kind <> 'FORUM_EVENT' OR NOT EXISTS (
                SELECT 1 FROM workflow_outbox earlier
                 WHERE earlier.workflow_instance_id = o.workflow_instance_id
                   AND earlier.outbox_kind = 'FORUM_EVENT'
                   AND earlier.delivered_at IS NULL
                   AND (earlier.created_at, earlier.outbox_id) < (o.created_at, o.outbox_id)
            ))
          ORDER BY o.created_at, o.outbox_id
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

#[cfg(test)]
mod regression_tests {
    use super::*;
    use sqlx::postgres::PgPoolOptions;

    async fn fixture() -> sqlx::PgPool {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must name an isolated test PostgreSQL instance");
        let pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .expect("connect isolated test database");
        sqlx::query(
            "CREATE TEMP TABLE workflow_instances (
                workflow_instance_id UUID PRIMARY KEY,
                execution_class TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TEMP TABLE workflow_forum_bindings (
                workflow_instance_id UUID PRIMARY KEY,
                binding_state TEXT NOT NULL
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        sqlx::query(
            "CREATE TEMP TABLE workflow_outbox (
                outbox_id UUID PRIMARY KEY,
                workflow_instance_id UUID NOT NULL,
                outbox_kind TEXT NOT NULL,
                event_key TEXT NOT NULL,
                payload JSONB NOT NULL,
                attempt_count INT NOT NULL DEFAULT 0,
                next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                delivered_at TIMESTAMPTZ,
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                UNIQUE (outbox_kind, event_key)
            )",
        )
        .execute(&pool)
        .await
        .unwrap();
        pool
    }

    #[tokio::test]
    async fn backed_off_forum_event_holds_later_event_for_same_instance() {
        let pool = fixture().await;
        let instance = Uuid::new_v4();
        let old = Uuid::new_v4();
        let later = Uuid::new_v4();
        for (id, key, created_at, next_attempt_at) in [
            (
                old,
                "old",
                "2026-01-01 00:00:00+00",
                "2099-01-01 00:00:00+00",
            ),
            (
                later,
                "later",
                "2026-01-01 00:00:01+00",
                "2026-01-01 00:00:01+00",
            ),
        ] {
            sqlx::query(
                "INSERT INTO workflow_outbox
                 (outbox_id, workflow_instance_id, outbox_kind, event_key, payload,
                  created_at, next_attempt_at)
                 VALUES ($1,$2,'FORUM_EVENT',$3,'{}'::jsonb,$4::timestamptz,$5::timestamptz)",
            )
            .bind(id)
            .bind(instance)
            .bind(key)
            .bind(created_at)
            .bind(next_attempt_at)
            .execute(&pool)
            .await
            .unwrap();
        }
        assert!(next_pending_batch(&pool, 20).await.unwrap().is_empty());
        sqlx::query("UPDATE workflow_outbox SET next_attempt_at = now() WHERE outbox_id = $1")
            .bind(old)
            .execute(&pool)
            .await
            .unwrap();
        let rows = next_pending_batch(&pool, 20).await.unwrap();
        assert_eq!(
            rows.iter().map(|r| r.outbox_id).collect::<Vec<_>>(),
            vec![old]
        );
        sqlx::query("UPDATE workflow_outbox SET delivered_at = now() WHERE outbox_id = $1")
            .bind(old)
            .execute(&pool)
            .await
            .unwrap();
        let rows = next_pending_batch(&pool, 20).await.unwrap();
        assert_eq!(
            rows.iter().map(|r| r.outbox_id).collect::<Vec<_>>(),
            vec![later]
        );
    }

    #[tokio::test]
    async fn first_event_for_old_business_instance_creates_binding_but_nonbusiness_does_not() {
        let pool = fixture().await;
        let business = Uuid::new_v4();
        let nonbusiness = Uuid::new_v4();
        for (id, class) in [(business, "BUSINESS"), (nonbusiness, "NON_BUSINESS_TEST")] {
            sqlx::query(
                "INSERT INTO workflow_instances (workflow_instance_id, execution_class) VALUES ($1,$2)",
            )
            .bind(id)
            .bind(class)
            .execute(&pool)
            .await
            .unwrap();
            let mut tx = pool.begin().await.unwrap();
            queue_forum_event(&mut tx, id, &format!("event:{id}"), serde_json::json!({}))
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
        let bindings: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_forum_bindings WHERE workflow_instance_id = $1",
        )
        .bind(business)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(bindings, 1);
        let nonbusiness_rows: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_outbox WHERE workflow_instance_id = $1",
        )
        .bind(nonbusiness)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(nonbusiness_rows, 0);
    }
}
