//! WORKFLOW_EXECUTION_CONTROL_V1 (CTR-SWEC-001/002/007/008) — the durable
//! outbox store and the canonical forum binding fact.
//!
//! Discipline: rows are queued INSIDE the committing business transaction
//! (same-tx durable facts) and drained by the background reconciler; a
//! forum/agent-core outage can delay delivery but can never lose a queued
//! fact and can never roll back a business transaction. Delivery is
//! at-least-once; dedupe rides `uq_outbox_kind_event_key` (queuing) and the
//! per-service idempotency protocols (draining). The reconciler claims per
//! kind with `FOR UPDATE SKIP LOCKED` inside the delivery transaction, and
//! only the latency-only EXECUTION_KICK channel dead-letters at its retry
//! cap (payload and failure reason retained; recovery via the migrations/0032
//! requeue statement).

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

/// Queue one durable Domain Owner assistance wake. The outbox event key is
/// the assistance case identity, so retries/replays converge on one row. Only
/// BUSINESS workflows wake an Agent; NON_BUSINESS_TEST stays side-effect free.
pub(crate) async fn queue_owner_assistance_wake(
    tx: &mut Transaction<'_, Postgres>,
    workflow_instance_id: Uuid,
    node_visit_id: Uuid,
    assistance_case_id: Uuid,
    owner_principal_id: Uuid,
    reason: &str,
) -> Result<(), sqlx::Error> {
    let class: String = sqlx::query_scalar(
        "SELECT execution_class::text FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(workflow_instance_id)
    .fetch_one(&mut **tx)
    .await?;
    if class != "BUSINESS" {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO workflow_outbox
             (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
         VALUES ($1, $2, 'OWNER_ASSISTANCE_WAKE', $3, $4)
         ON CONFLICT (outbox_kind, event_key) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(workflow_instance_id)
    .bind(format!("owner-assistance:{assistance_case_id}"))
    .bind(serde_json::json!({
        "workflowInstanceId": workflow_instance_id,
        "nodeVisitId": node_visit_id,
        "assistanceCaseId": assistance_case_id,
        "ownerPrincipalId": owner_principal_id,
        "reason": reason,
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

/// Retry budget for the latency-only EXECUTION_KICK channel before a row
/// dead-letters (payload and last_error are retained; the recovery entry is
/// the requeue statement in migrations/0032_outbox_dead_letter.sql). The
/// correctness-path channels (FORUM_EVENT, OWNER_ASSISTANCE_WAKE) keep their
/// never-dead-letter discipline.
pub(crate) const EXECUTION_KICK_MAX_ATTEMPTS: i32 = 12;

/// The due batch of ONE kind, oldest first. `delivered_at IS NULL AND
/// dead_lettered_at IS NULL AND next_attempt_at <= now()` mirrors the partial
/// index; a channel that is not enabled never sees another kind's rows.
/// FORUM_EVENT rows additionally hold each other back per instance so the
/// thread projection posts in commit order.
pub(crate) async fn next_pending_batch(
    pool: &sqlx::PgPool,
    outbox_kind: &str,
    limit: i64,
) -> Result<Vec<OutboxRow>, sqlx::Error> {
    sqlx::query_as::<_, OutboxRow>(
        "SELECT o.outbox_id, o.workflow_instance_id, o.outbox_kind, o.event_key,
                o.payload, o.attempt_count
           FROM workflow_outbox o
          WHERE o.outbox_kind = $1
            AND o.delivered_at IS NULL AND o.dead_lettered_at IS NULL
            AND o.next_attempt_at <= now()
            AND (o.outbox_kind <> 'FORUM_EVENT' OR NOT EXISTS (
                SELECT 1 FROM workflow_outbox earlier
                 WHERE earlier.workflow_instance_id = o.workflow_instance_id
                   AND earlier.outbox_kind = 'FORUM_EVENT'
                   AND earlier.delivered_at IS NULL
                   AND (earlier.created_at, earlier.outbox_id) < (o.created_at, o.outbox_id)
            ))
          ORDER BY o.created_at, o.outbox_id
          LIMIT $2",
    )
    .bind(outbox_kind)
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Atomically claim one row for delivery inside the caller's transaction:
/// `FOR UPDATE SKIP LOCKED` makes concurrent workers skip a row that another
/// worker is already delivering, and re-checks every due predicate under the
/// lock. The row lock doubles as the lease — a crashed worker's transaction
/// dies with its connection, releasing the row for the next pass.
pub(crate) async fn claim_row(
    tx: &mut Transaction<'_, Postgres>,
    outbox_id: Uuid,
    outbox_kind: &str,
) -> Result<Option<OutboxRow>, sqlx::Error> {
    sqlx::query_as::<_, OutboxRow>(
        "SELECT o.outbox_id, o.workflow_instance_id, o.outbox_kind, o.event_key,
                o.payload, o.attempt_count
           FROM workflow_outbox o
          WHERE o.outbox_id = $1 AND o.outbox_kind = $2
            AND o.delivered_at IS NULL AND o.dead_lettered_at IS NULL
            AND o.next_attempt_at <= now()
          FOR UPDATE OF o SKIP LOCKED",
    )
    .bind(outbox_id)
    .bind(outbox_kind)
    .fetch_optional(&mut **tx)
    .await
}

pub(crate) async fn mark_delivered(
    tx: &mut Transaction<'_, Postgres>,
    outbox_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE workflow_outbox SET delivered_at = now(), last_error = NULL
          WHERE outbox_id = $1 AND delivered_at IS NULL",
    )
    .bind(outbox_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Exponential backoff capped at 10 minutes. When `dead_letter_after_attempts`
/// is Some(cap) and this attempt reaches it, the row dead-letters in the same
/// statement: payload, attempt_count and last_error are retained, the due
/// queries stop selecting it, and the recovery entry is the requeue statement
/// in migrations/0032_outbox_dead_letter.sql. `None` keeps the
/// never-dead-lettered correctness-path discipline (no silent loss).
pub(crate) async fn mark_attempt_failed(
    tx: &mut Transaction<'_, Postgres>,
    outbox_id: Uuid,
    error: &str,
    dead_letter_after_attempts: Option<i32>,
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
                last_error = $2,
                dead_lettered_at = CASE
                    WHEN $3::int IS NOT NULL AND attempt_count + 1 >= $3 THEN now()
                    ELSE dead_lettered_at END
          WHERE outbox_id = $1 AND delivered_at IS NULL",
    )
    .bind(outbox_id)
    .bind(sanitized)
    .bind(dead_letter_after_attempts)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Dead-letter a row immediately (a deterministically rejected delivery with
/// no retry value). The row, its payload and the failure reason are retained.
pub(crate) async fn mark_dead_lettered(
    tx: &mut Transaction<'_, Postgres>,
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
                dead_lettered_at = now(),
                last_error = $2
          WHERE outbox_id = $1 AND delivered_at IS NULL",
    )
    .bind(outbox_id)
    .bind(sanitized)
    .execute(&mut **tx)
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
                last_error TEXT,
                dead_lettered_at TIMESTAMPTZ,
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
        assert!(next_pending_batch(&pool, "FORUM_EVENT", 20)
            .await
            .unwrap()
            .is_empty());
        sqlx::query("UPDATE workflow_outbox SET next_attempt_at = now() WHERE outbox_id = $1")
            .bind(old)
            .execute(&pool)
            .await
            .unwrap();
        let rows = next_pending_batch(&pool, "FORUM_EVENT", 20).await.unwrap();
        assert_eq!(
            rows.iter().map(|r| r.outbox_id).collect::<Vec<_>>(),
            vec![old]
        );
        sqlx::query("UPDATE workflow_outbox SET delivered_at = now() WHERE outbox_id = $1")
            .bind(old)
            .execute(&pool)
            .await
            .unwrap();
        let rows = next_pending_batch(&pool, "FORUM_EVENT", 20).await.unwrap();
        assert_eq!(
            rows.iter().map(|r| r.outbox_id).collect::<Vec<_>>(),
            vec![later]
        );
    }

    #[tokio::test]
    async fn batch_is_kind_scoped_and_excludes_dead_lettered_rows() {
        let pool = fixture().await;
        let instance = Uuid::new_v4();
        let kick = Uuid::new_v4();
        let forum = Uuid::new_v4();
        let wake = Uuid::new_v4();
        let dead = Uuid::new_v4();
        for (id, kind, key) in [
            (kick, "EXECUTION_KICK", "kick:a"),
            (forum, "FORUM_EVENT", "wf_created:a"),
            (wake, "OWNER_ASSISTANCE_WAKE", "owner-assistance:a"),
            (dead, "EXECUTION_KICK", "kick:dead"),
        ] {
            sqlx::query(
                "INSERT INTO workflow_outbox
                 (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
                 VALUES ($1,$2,$3,$4,'{}'::jsonb)",
            )
            .bind(id)
            .bind(instance)
            .bind(kind)
            .bind(key)
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query("UPDATE workflow_outbox SET dead_lettered_at = now() WHERE outbox_id = $1")
            .bind(dead)
            .execute(&pool)
            .await
            .unwrap();

        let kicks: Vec<Uuid> = next_pending_batch(&pool, "EXECUTION_KICK", 20)
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.outbox_id)
            .collect();
        assert_eq!(
            kicks,
            vec![kick],
            "dead-lettered rows stay out of the batch"
        );
        assert_eq!(
            next_pending_batch(&pool, "FORUM_EVENT", 20)
                .await
                .unwrap()
                .len(),
            1
        );
        assert_eq!(
            next_pending_batch(&pool, "OWNER_ASSISTANCE_WAKE", 20)
                .await
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn claim_row_locks_and_rechecks_due_predicates() {
        let pool = fixture().await;
        let instance = Uuid::new_v4();
        let kick = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_outbox
             (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
             VALUES ($1,$2,'EXECUTION_KICK','kick:claim','{}'::jsonb)",
        )
        .bind(kick)
        .bind(instance)
        .execute(&pool)
        .await
        .unwrap();

        // A backed-off row is not claimable.
        sqlx::query(
            "UPDATE workflow_outbox SET next_attempt_at = now() + interval '10 minutes' WHERE outbox_id = $1",
        )
        .bind(kick)
        .execute(&pool)
        .await
        .unwrap();
        let mut tx = pool.begin().await.unwrap();
        assert!(claim_row(&mut tx, kick, "EXECUTION_KICK")
            .await
            .unwrap()
            .is_none());
        tx.commit().await.unwrap();

        // Due again: claimable, and a wrong kind never claims another
        // channel's row.
        sqlx::query("UPDATE workflow_outbox SET next_attempt_at = now() WHERE outbox_id = $1")
            .bind(kick)
            .execute(&pool)
            .await
            .unwrap();
        let mut tx = pool.begin().await.unwrap();
        assert!(claim_row(&mut tx, kick, "FORUM_EVENT")
            .await
            .unwrap()
            .is_none());
        let claimed = claim_row(&mut tx, kick, "EXECUTION_KICK")
            .await
            .unwrap()
            .expect("claimable while due");
        assert_eq!(claimed.outbox_id, kick);
        tx.commit().await.unwrap();
        // Cross-session SKIP LOCKED behavior (concurrent workers, crash
        // recovery) is covered by tests/48 on the real table.
    }

    #[tokio::test]
    async fn mark_attempt_failed_dead_letters_only_at_the_cap() {
        let pool = fixture().await;
        let instance = Uuid::new_v4();
        let kick = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_outbox
             (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
             VALUES ($1,$2,'EXECUTION_KICK','kick:cap','{}'::jsonb)",
        )
        .bind(kick)
        .bind(instance)
        .execute(&pool)
        .await
        .unwrap();

        for attempt in 1..EXECUTION_KICK_MAX_ATTEMPTS {
            let mut tx = pool.begin().await.unwrap();
            mark_attempt_failed(
                &mut tx,
                kick,
                "kick endpoint 5xx: 502",
                Some(EXECUTION_KICK_MAX_ATTEMPTS),
            )
            .await
            .unwrap();
            tx.commit().await.unwrap();
            let (attempts, dead): (
                i32,
                Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>,
            ) = sqlx::query_as(
                "SELECT attempt_count, dead_lettered_at FROM workflow_outbox WHERE outbox_id = $1",
            )
            .bind(kick)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(attempts, attempt);
            assert!(dead.is_none(), "no dead-letter before the cap");
        }
        let mut tx = pool.begin().await.unwrap();
        mark_attempt_failed(
            &mut tx,
            kick,
            "kick endpoint 5xx: 502",
            Some(EXECUTION_KICK_MAX_ATTEMPTS),
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let (attempts, dead): (
            i32,
            Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>,
        ) = sqlx::query_as(
            "SELECT attempt_count, dead_lettered_at FROM workflow_outbox WHERE outbox_id = $1",
        )
        .bind(kick)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(attempts, EXECUTION_KICK_MAX_ATTEMPTS);
        assert!(dead.is_some(), "the cap dead-letters the row");
        // Dead-lettered rows never come back through the due batch.
        assert!(next_pending_batch(&pool, "EXECUTION_KICK", 20)
            .await
            .unwrap()
            .is_empty());

        // Unbounded channels (cap = None) never dead-letter.
        let forum = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_outbox
             (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
             VALUES ($1,$2,'FORUM_EVENT','wf_created:cap','{}'::jsonb)",
        )
        .bind(forum)
        .bind(instance)
        .execute(&pool)
        .await
        .unwrap();
        for _ in 0..(EXECUTION_KICK_MAX_ATTEMPTS + 3) {
            let mut tx = pool.begin().await.unwrap();
            mark_attempt_failed(&mut tx, forum, "forum http 503", None)
                .await
                .unwrap();
            tx.commit().await.unwrap();
        }
        let dead: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>> =
            sqlx::query_scalar("SELECT dead_lettered_at FROM workflow_outbox WHERE outbox_id = $1")
                .bind(forum)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert!(
            dead.is_none(),
            "correctness-path rows are never dead-lettered"
        );
        let (delivered, attempts): (
            Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>,
            i32,
        ) = sqlx::query_as(
            "SELECT delivered_at, attempt_count FROM workflow_outbox WHERE outbox_id = $1",
        )
        .bind(forum)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(delivered.is_none(), "never silently delivered");
        assert_eq!(
            attempts,
            EXECUTION_KICK_MAX_ATTEMPTS + 3,
            "every attempt counted; the row stays recoverable"
        );
    }

    #[tokio::test]
    async fn mark_dead_lettered_retains_payload_and_reason() {
        let pool = fixture().await;
        let instance = Uuid::new_v4();
        let kick = Uuid::new_v4();
        let payload = serde_json::json!({"dispatchIntentId": instance});
        sqlx::query(
            "INSERT INTO workflow_outbox
             (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
             VALUES ($1,$2,'EXECUTION_KICK','kick:perm',$3)",
        )
        .bind(kick)
        .bind(instance)
        .bind(&payload)
        .execute(&pool)
        .await
        .unwrap();
        let mut tx = pool.begin().await.unwrap();
        mark_dead_lettered(&mut tx, kick, "kick endpoint 400: permanent rejection")
            .await
            .unwrap();
        tx.commit().await.unwrap();
        let (stored_payload, dead, error): (
            serde_json::Value,
            Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>,
            Option<String>,
        ) = sqlx::query_as(
            "SELECT payload, dead_lettered_at, last_error FROM workflow_outbox WHERE outbox_id = $1",
        )
        .bind(kick)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(stored_payload, payload, "original request retained");
        assert!(dead.is_some());
        assert_eq!(
            error.as_deref(),
            Some("kick endpoint 400: permanent rejection")
        );
        assert!(next_pending_batch(&pool, "EXECUTION_KICK", 20)
            .await
            .unwrap()
            .is_empty());
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

    /// Multi-backend regression on the REAL migrated `workflow_outbox` (the
    /// fixtures above shadow it with per-connection TEMP tables, so no test
    /// before this one ever exercised several PostgreSQL backends at once):
    /// concurrent queue dedupe, concurrent backoff arithmetic and the
    /// partitioned due-batch predicate must hold when multiple reconciler
    /// readers poll the same rows. Every row is keyed under a unique run
    /// prefix and deleted before any assertion can fail the test.
    #[tokio::test]
    async fn concurrent_backends_partition_kinds_dedupe_and_preserve_backoff() {
        let url = std::env::var("TEST_DATABASE_URL")
            .expect("TEST_DATABASE_URL must name an isolated test PostgreSQL instance");
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(&url)
            .await
            .expect("connect isolated test database");
        let migrated: bool =
            sqlx::query_scalar("SELECT to_regclass('workflow_outbox') IS NOT NULL")
                .fetch_one(&pool)
                .await
                .unwrap();
        if !migrated {
            sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
                .await
                .expect("load migrations")
                .run(&pool)
                .await
                .expect("apply migrations");
        }

        let run = Uuid::new_v4().simple().to_string();
        let key = |name: &str| format!("conc:{run}:{name}");

        // The real schema FKs every outbox row to a workflow instance; build
        // the minimal parent chain with the same INSERTs the integration
        // seeds use (tests/common).
        let principal_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled)
             VALUES ($1, 'HUMAN', 'Outbox Concurrency', 'outbox-conc@example.com', TRUE)",
        )
        .bind(principal_id)
        .execute(&pool)
        .await
        .unwrap();
        let domain_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO domains (domain_id, domain_key, display_name, enabled)
             VALUES ($1, $2, 'Outbox Concurrency', TRUE)",
        )
        .bind(domain_id)
        .bind(format!("outbox-conc-{run}"))
        .execute(&pool)
        .await
        .unwrap();
        let definition_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name)
             VALUES ($1, $2, $3, 'Outbox Concurrency')",
        )
        .bind(definition_id)
        .bind(domain_id)
        .bind(format!("outbox-conc-def-{run}"))
        .execute(&pool)
        .await
        .unwrap();
        let version_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_definition_versions
                 (definition_version_id, workflow_definition_id, version_number, version_status,
                  context_schema, submission_schema)
             VALUES ($1, $2, 1, 'DRAFT', '{\"type\":\"object\"}'::jsonb, '{\"type\":\"object\"}'::jsonb)",
        )
        .bind(version_id)
        .bind(definition_id)
        .execute(&pool)
        .await
        .unwrap();
        let instance_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_instances
                 (workflow_instance_id, domain_id, definition_version_id, created_by_principal_id)
             VALUES ($1, $2, $3, $4)",
        )
        .bind(instance_id)
        .bind(domain_id)
        .bind(version_id)
        .bind(principal_id)
        .execute(&pool)
        .await
        .unwrap();

        for name in ["wake:1", "wake:2", "wake:3"] {
            sqlx::query(
                "INSERT INTO workflow_outbox
                     (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
                 VALUES ($1,$2,'OWNER_ASSISTANCE_WAKE',$3,'{}'::jsonb)",
            )
            .bind(Uuid::new_v4())
            .bind(instance_id)
            .bind(key(name))
            .execute(&pool)
            .await
            .unwrap();
        }
        for name in ["kick:1", "kick:2", "kick:3"] {
            sqlx::query(
                "INSERT INTO workflow_outbox
                     (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
                 VALUES ($1,$2,'EXECUTION_KICK',$3,'{}'::jsonb)",
            )
            .bind(Uuid::new_v4())
            .bind(instance_id)
            .bind(key(name))
            .execute(&pool)
            .await
            .unwrap();
        }
        sqlx::query(
            "INSERT INTO workflow_outbox
                 (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
             VALUES ($1,$2,'FORUM_EVENT',$3,'{}'::jsonb)",
        )
        .bind(Uuid::new_v4())
        .bind(instance_id)
        .bind(key("forum:1"))
        .execute(&pool)
        .await
        .unwrap();

        // Eight backends race the same queue insert: the (outbox_kind,
        // event_key) dedupe must leave exactly one row.
        let dedupe_key = key("wake:dedupe");
        let racers = (0..8).map(|_| {
            let pool = pool.clone();
            let dedupe_key = dedupe_key.clone();
            tokio::spawn(async move {
                sqlx::query(
                    "INSERT INTO workflow_outbox
                         (outbox_id, workflow_instance_id, outbox_kind, event_key, payload)
                     VALUES ($1,$2,'OWNER_ASSISTANCE_WAKE',$3,'{}'::jsonb)
                     ON CONFLICT (outbox_kind, event_key) DO NOTHING",
                )
                .bind(Uuid::new_v4())
                .bind(instance_id)
                .bind(dedupe_key)
                .execute(&pool)
                .await
            })
        });
        for racer in racers {
            racer.await.unwrap().unwrap();
        }

        // Two backends record a failure for the same row: the atomic
        // attempt_count increment must land exactly twice.
        let backoff_row: Uuid =
            sqlx::query_scalar("SELECT outbox_id FROM workflow_outbox WHERE event_key = $1")
                .bind(key("kick:2"))
                .fetch_one(&pool)
                .await
                .unwrap();
        let failing = |pool: sqlx::PgPool, outbox_id: Uuid| {
            tokio::spawn(async move {
                let mut tx = pool.begin().await?;
                mark_attempt_failed(
                    &mut tx,
                    outbox_id,
                    "concurrent backoff probe",
                    Some(EXECUTION_KICK_MAX_ATTEMPTS),
                )
                .await?;
                tx.commit().await
            })
        };
        let (failure_a, failure_b) = tokio::join!(
            failing(pool.clone(), backoff_row),
            failing(pool.clone(), backoff_row)
        );
        failure_a.unwrap().unwrap();
        failure_b.unwrap().unwrap();

        // Four reconciler readers poll their own partition concurrently.
        // Wake readers may only see wakes; kick readers may only see kicks
        // (the real schema's CHECK constraint admits no other kind, so the
        // unsupported-kind arm stays covered by the TEMP-table tests above).
        let reader = |outbox_kind: &'static str, pool: sqlx::PgPool| async move {
            let mut violations = Vec::new();
            for _ in 0..10 {
                let rows = match next_pending_batch(&pool, outbox_kind, 20).await {
                    Ok(rows) => rows,
                    Err(error) => {
                        violations.push(format!("{outbox_kind} batch query failed: {error}"));
                        return violations;
                    }
                };
                if rows.is_empty() {
                    return violations;
                }
                for row in rows {
                    let allowed = match outbox_kind {
                        "OWNER_ASSISTANCE_WAKE" => row.outbox_kind == "OWNER_ASSISTANCE_WAKE",
                        "EXECUTION_KICK" => row.outbox_kind == "EXECUTION_KICK",
                        _ => false,
                    };
                    if !allowed {
                        violations.push(format!(
                            "{outbox_kind} batch returned foreign kind {}",
                            row.outbox_kind
                        ));
                        return violations;
                    }
                    let settled = async {
                        let mut tx = pool.begin().await?;
                        if claim_row(&mut tx, row.outbox_id, outbox_kind)
                            .await?
                            .is_some()
                        {
                            mark_delivered(&mut tx, row.outbox_id).await?;
                        }
                        tx.commit().await
                    }
                    .await;
                    if let Err(error) = settled {
                        violations.push(format!("claim/mark delivered failed: {error}"));
                        return violations;
                    }
                }
            }
            violations.push(format!("{outbox_kind} batch never drained"));
            violations
        };
        let (wake_a, wake_b, kick_a, kick_b) = tokio::join!(
            tokio::spawn(reader("OWNER_ASSISTANCE_WAKE", pool.clone())),
            tokio::spawn(reader("OWNER_ASSISTANCE_WAKE", pool.clone())),
            tokio::spawn(reader("EXECUTION_KICK", pool.clone())),
            tokio::spawn(reader("EXECUTION_KICK", pool.clone())),
        );
        let mut violations = Vec::new();
        for handle in [wake_a, wake_b, kick_a, kick_b] {
            violations.extend(handle.unwrap());
        }

        let wakes_total: i64 =
            sqlx::query_scalar("SELECT count(*) FROM workflow_outbox WHERE event_key LIKE $1")
                .bind(format!("conc:{run}:wake:%"))
                .fetch_one(&pool)
                .await
                .unwrap();
        let wakes_delivered: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_outbox
              WHERE event_key LIKE $1 AND delivered_at IS NOT NULL",
        )
        .bind(format!("conc:{run}:wake:%"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let healthy_kicks_delivered: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM workflow_outbox
              WHERE event_key IN ($1, $2) AND delivered_at IS NOT NULL",
        )
        .bind(key("kick:1"))
        .bind(key("kick:3"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let (backoff_attempts, backoff_pending): (i32, bool) = sqlx::query_as(
            "SELECT attempt_count, delivered_at IS NULL FROM workflow_outbox WHERE event_key = $1",
        )
        .bind(key("kick:2"))
        .fetch_one(&pool)
        .await
        .unwrap();
        let forum_untouched: (i32, bool) = sqlx::query_as(
            "SELECT attempt_count, delivered_at IS NULL FROM workflow_outbox WHERE event_key = $1",
        )
        .bind(key("forum:1"))
        .fetch_one(&pool)
        .await
        .unwrap();

        sqlx::query("DELETE FROM workflow_outbox WHERE event_key LIKE $1")
            .bind(format!("conc:{run}:%"))
            .execute(&pool)
            .await
            .unwrap();
        for (statement, id) in [
            (
                "DELETE FROM workflow_instances WHERE workflow_instance_id = $1",
                instance_id,
            ),
            (
                "DELETE FROM workflow_definition_versions WHERE definition_version_id = $1",
                version_id,
            ),
            (
                "DELETE FROM workflow_definitions WHERE workflow_definition_id = $1",
                definition_id,
            ),
            ("DELETE FROM domains WHERE domain_id = $1", domain_id),
            (
                "DELETE FROM principals WHERE principal_id = $1",
                principal_id,
            ),
        ] {
            sqlx::query(statement)
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        }

        assert!(
            violations.is_empty(),
            "partition violations: {violations:?}"
        );
        assert_eq!(
            wakes_total, 4,
            "dedupe must keep exactly one raced wake row"
        );
        assert_eq!(wakes_delivered, 4, "every wake row must stay drainable");
        assert_eq!(
            healthy_kicks_delivered, 2,
            "healthy kicks must deliver once"
        );
        assert_eq!(
            backoff_attempts, 2,
            "both concurrent failures must increment"
        );
        assert!(backoff_pending, "backoff must keep the row pending");
        assert_eq!(
            forum_untouched,
            (0, true),
            "forum rows must not leak into wake/kick batches"
        );
    }
}
