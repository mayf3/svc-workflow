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

/// One kind's due batch, oldest first. Owner wakes have their own bound so
/// kick/projection backlogs cannot consume their delivery batch. Unsupported
/// kinds remain in the kick batch for retry across forward deployment/rollback.
pub(crate) async fn next_pending_batch(
    pool: &sqlx::PgPool,
    limit: i64,
    outbox_kind: &str,
) -> Result<Vec<OutboxRow>, sqlx::Error> {
    sqlx::query_as::<_, OutboxRow>(
        "SELECT o.outbox_id, o.workflow_instance_id, o.outbox_kind, o.event_key,
                o.payload, o.attempt_count
           FROM workflow_outbox o
          WHERE o.delivered_at IS NULL AND o.next_attempt_at <= now()
            AND (o.outbox_kind = $2 OR ($2 = 'EXECUTION_KICK' AND
                 o.outbox_kind NOT IN ('FORUM_EVENT','EXECUTION_KICK','OWNER_ASSISTANCE_WAKE')))
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
    .bind(outbox_kind)
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
        assert!(next_pending_batch(&pool, 20, "FORUM_EVENT")
            .await
            .unwrap()
            .is_empty());
        sqlx::query("UPDATE workflow_outbox SET next_attempt_at = now() WHERE outbox_id = $1")
            .bind(old)
            .execute(&pool)
            .await
            .unwrap();
        let rows = next_pending_batch(&pool, 20, "FORUM_EVENT").await.unwrap();
        assert_eq!(
            rows.iter().map(|r| r.outbox_id).collect::<Vec<_>>(),
            vec![old]
        );
        sqlx::query("UPDATE workflow_outbox SET delivered_at = now() WHERE outbox_id = $1")
            .bind(old)
            .execute(&pool)
            .await
            .unwrap();
        let rows = next_pending_batch(&pool, 20, "FORUM_EVENT").await.unwrap();
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
                mark_attempt_failed(&pool, outbox_id, "concurrent backoff probe").await
            })
        };
        tokio::join!(
            failing(pool.clone(), backoff_row),
            failing(pool.clone(), backoff_row)
        );

        // Four reconciler readers poll their own partition concurrently.
        // Wake readers may only see wakes; kick readers may only see kicks
        // (the real schema's CHECK constraint admits no other kind, so the
        // unsupported-kind arm stays covered by the TEMP-table tests above).
        let reader = |outbox_kind: &'static str, pool: sqlx::PgPool| async move {
            let mut violations = Vec::new();
            for _ in 0..10 {
                let rows = match next_pending_batch(&pool, 20, outbox_kind).await {
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
                    if let Err(error) = mark_delivered(&pool, row.outbox_id).await {
                        violations.push(format!("mark delivered failed: {error}"));
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
