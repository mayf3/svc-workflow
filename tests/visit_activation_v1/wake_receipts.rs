//! CTR-VAI-008/014: real PostgreSQL receipt identity, replay and atomicity.
//! Future eligibility is a test fixture, not a claim that SCHEDULER_DEFER
//! is a production command. Ordinary ALREADY_DUE uses a natural fresh visit.

use super::*;
use svc_workflow::application::workflow_instance::wake::wake_dispatch_intent;
use svc_workflow::domain::workflow_instance::commands::WakeDispatchIntentCommand;
use svc_workflow::domain::workflow_instance::errors::WakeDispatchIntentError;

struct Fixture {
    pool: PgPool,
    owner: Uuid,
    creator: Uuid,
    scheduler: Uuid,
    instance: Uuid,
    visit: Uuid,
    version: i32,
    advance: Uuid,
}

impl Fixture {
    async fn new() -> Self {
        let pool = common::create_pool().await;
        let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
        let creator = seed_agent(&pool).await;
        seed_domain_member(&pool, domain, creator).await;
        let scheduler = seed_agent(&pool).await;
        grant_scheduler_read(&pool, scheduler).await;
        let (definition, _, _, _, _, advance, _, _) =
            seed_v1_definition(&pool, domain, creator).await;
        let (instance, visit, version) =
            create_v1_instance(&pool, creator, domain, definition).await;
        Self {
            pool,
            owner,
            creator,
            scheduler,
            instance,
            visit,
            version,
            advance,
        }
    }

    fn command(&self, key: &str) -> WakeDispatchIntentCommand {
        WakeDispatchIntentCommand {
            principal_id: PrincipalId::from_uuid(self.scheduler),
            idempotency_key: key.to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(self.instance),
            node_visit_id: NodeVisitId::from_uuid(self.visit),
            expected_workflow_state_version: self.version,
            cause: None,
        }
    }

    fn path(&self) -> String {
        format!(
            "/internal/v1/workflow-instances/{}/node-visits/{}/wake",
            self.instance, self.visit
        )
    }

    async fn defer_fixture(&self) {
        sqlx::query(
            "INSERT INTO workflow_dispatch_eligibility_events
                (eligibility_event_id, activation_id, previous_next_eligible_at,
                 new_next_eligible_at, cause_class, command_id)
             SELECT $1, activation_id, initial_next_eligible_at,
                    initial_next_eligible_at + interval '1 hour', 'SCHEDULER_DEFER', $2
               FROM workflow_activations WHERE node_visit_id = $3",
        )
        .bind(Uuid::new_v4())
        .bind(Uuid::new_v4())
        .bind(self.visit)
        .execute(&self.pool)
        .await
        .expect("future-eligibility fixture");
    }

    /// Only workflow business effects; receipts and audits are checked separately.
    async fn business(&self) -> (i32, i64, i64) {
        sqlx::query_as(
            "SELECT workflow_state_version,
                    (SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1),
                    (SELECT count(*) FROM workflow_dispatch_eligibility_events e
                       JOIN workflow_activations a USING (activation_id)
                      WHERE a.workflow_instance_id=$1)
               FROM workflow_instances WHERE workflow_instance_id=$1",
        )
        .bind(self.instance)
        .fetch_one(&self.pool)
        .await
        .expect("business snapshot")
    }

    async fn receipt(&self, key: &str, status: i32, body: &Value) -> Uuid {
        let (id, state, actual_status, actual_body, digest): (Uuid, String, i32, Value, String) =
            sqlx::query_as(
                "SELECT command_id, receipt_status::text, response_status, response_body, response_digest
                   FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2",
            )
            .bind(self.scheduler).bind(key).fetch_one(&self.pool).await.expect("durable receipt");
        assert_eq!(state, "COMPLETED");
        assert_eq!(actual_status, status);
        assert_eq!(&actual_body, body);
        assert_eq!(
            digest,
            svc_workflow::domain::definition::digest::compute_json_digest(body).unwrap()
        );
        id
    }

    async fn audit(&self, receipt: Uuid) -> Vec<(String, Option<String>)> {
        sqlx::query_as(
            "SELECT attempt_type, failure_reason FROM workflow_command_attempt_audits
              WHERE command_id=$1 ORDER BY created_at, audit_id",
        )
        .bind(receipt)
        .fetch_all(&self.pool)
        .await
        .expect("attempt audit")
    }
}

#[tokio::test]
async fn wake_noop_replay_preserves_original_after_real_transition() {
    let f = Fixture::new().await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(f.pool.clone(), &mock.url, vec![]);
    let token = direct_token(f.scheduler, "workflow.execute", &mock.key_pair);
    let request = json!({"expectedWorkflowStateVersion": f.version});
    let key = format!("wake-natural-replay-{}", Uuid::new_v4());
    let before = f.business().await;

    let (status, original) = do_post(app.clone(), &f.path(), &token, request.clone(), &key).await;
    assert_eq!(status, 200, "{original}");
    assert_eq!(original["reason"], "ALREADY_DUE");
    assert_eq!(original["wakeApplied"], false);
    let receipt = f.receipt(&key, 200, &original).await;
    assert_eq!(f.business().await, before);
    assert_eq!(
        do_post(app.clone(), &f.path(), &token, request.clone(), &key).await,
        (200, original.clone())
    );

    transition_v1(&f.pool, f.creator, f.instance, f.version, f.advance).await;
    let after_transition = f.business().await;
    assert_eq!(
        do_post(app.clone(), &f.path(), &token, request.clone(), &key).await,
        (200, original)
    );
    assert_eq!(f.business().await, after_transition);
    assert_eq!(
        f.audit(receipt).await,
        vec![("WAKE_NO_OP".to_string(), Some("ALREADY_DUE".to_string()))]
    );

    // A new key observes the real closed source activation; that result also replays.
    let closed_key = format!("wake-closed-replay-{}", Uuid::new_v4());
    let closed = do_post(app.clone(), &f.path(), &token, request.clone(), &closed_key).await;
    assert_eq!(closed.0, 200);
    assert_eq!(closed.1["reason"], "ACTIVATION_CLOSED");
    assert_eq!(
        do_post(app, &f.path(), &token, request, &closed_key).await,
        closed
    );
    assert_eq!(f.business().await, after_transition);
}

#[tokio::test]
async fn wake_version_mismatch_and_cancelled_noops_replay() {
    let f = Fixture::new().await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(f.pool.clone(), &mock.url, vec![]);
    let token = direct_token(f.scheduler, "workflow.execute", &mock.key_pair);
    let request = json!({"expectedWorkflowStateVersion": f.version + 1});
    let key = format!("wake-version-replay-{}", Uuid::new_v4());
    let before = f.business().await;
    let original = do_post(app.clone(), &f.path(), &token, request.clone(), &key).await;
    assert_eq!(original.0, 200);
    assert_eq!(original.1["reason"], "VERSION_MISMATCH");
    assert_eq!(
        do_post(app.clone(), &f.path(), &token, request.clone(), &key).await,
        original
    );
    assert_eq!(f.business().await, before);

    let owner_token = direct_token(f.owner, "workflow.execute", &mock.key_pair);
    let cancelled = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{}/cancel", f.instance),
        &owner_token,
        json!({"reason": "receipt regression"}),
        &format!("cancel-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(cancelled.0, 200, "cancel fixture: {}", cancelled.1);
    let after_cancel = f.business().await;
    assert_eq!(
        do_post(app.clone(), &f.path(), &token, request.clone(), &key).await,
        original
    );

    let closed_key = format!("wake-instance-closed-{}", Uuid::new_v4());
    let closed = do_post(app.clone(), &f.path(), &token, request.clone(), &closed_key).await;
    assert_eq!(closed.0, 200);
    assert_eq!(closed.1["reason"], "INSTANCE_CLOSED");
    assert_eq!(
        do_post(app, &f.path(), &token, request, &closed_key).await,
        closed
    );
    assert_eq!(f.business().await, after_cancel);
}

#[tokio::test]
async fn wake_applied_replay_and_conflict_preserve_exact_effects() {
    let f = Fixture::new().await;
    f.defer_fixture().await;
    let before = f.business().await;
    let key = format!("wake-apply-exact-{}", Uuid::new_v4());
    let command = f.command(&key);
    let original = wake_dispatch_intent(&f.pool, command.clone())
        .await
        .unwrap();
    assert!(original.wake_applied && !original.replayed);
    assert_eq!(
        f.business().await,
        (before.0 + 1, before.1 + 1, before.2 + 1)
    );
    let result_body = json!({
        "wakeApplied": true, "workflowInstanceId": f.instance, "nodeVisitId": f.visit,
        "workflowStateVersion": original.workflow_state_version,
        "eventSequence": original.event_sequence,
        "nextEligibleAt": original.next_eligible_at.unwrap().to_rfc3339(),
    });
    let receipt = f.receipt(&key, 200, &result_body).await;
    let replay = wake_dispatch_intent(&f.pool, command.clone())
        .await
        .unwrap();
    assert!(replay.replayed);
    assert_eq!(
        serde_json::to_value(&replay).unwrap(),
        serde_json::to_value(&original).unwrap()
    );
    assert_eq!(
        f.business().await,
        (before.0 + 1, before.1 + 1, before.2 + 1)
    );
    assert_eq!(
        f.audit(receipt).await,
        vec![("WAKE_APPLIED".to_string(), None)]
    );

    transition_v1(&f.pool, f.creator, f.instance, before.0 + 1, f.advance).await;
    let after_transition = f.business().await;
    let later_replay = wake_dispatch_intent(&f.pool, command.clone())
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(later_replay).unwrap(),
        serde_json::to_value(original).unwrap()
    );
    let mut changed = command;
    changed.cause = Some("different request".to_string());
    assert!(matches!(wake_dispatch_intent(&f.pool, changed).await,
        Err(WakeDispatchIntentError::IdempotencyConflict { original_command_id, .. }) if original_command_id == receipt));
    assert_eq!(f.business().await, after_transition);
    assert_eq!(f.receipt(&key, 200, &result_body).await, receipt);
}

#[tokio::test]
async fn wake_deterministic_rejections_are_durable_and_replay_exact_http_errors() {
    let f = Fixture::new().await;
    let mock = common::MockJwksServer::start().await;
    let app = build_app(f.pool.clone(), &mock.url, vec![]);
    let token = direct_token(f.scheduler, "workflow.execute", &mock.key_pair);
    let before = f.business().await;
    for (path, request, status, code) in [
        (
            f.path(),
            json!({"expectedWorkflowStateVersion": f.version, "cause": "bad\ncause"}),
            422,
            "invalid_cause",
        ),
        (
            format!(
                "/internal/v1/workflow-instances/{}/node-visits/{}/wake",
                Uuid::new_v4(),
                f.visit
            ),
            json!({"expectedWorkflowStateVersion": f.version}),
            404,
            "instance_not_found",
        ),
        (
            format!(
                "/internal/v1/workflow-instances/{}/node-visits/{}/wake",
                f.instance,
                Uuid::new_v4()
            ),
            json!({"expectedWorkflowStateVersion": f.version}),
            404,
            "dispatch_intent_not_found",
        ),
    ] {
        let key = format!("wake-rejection-{}", Uuid::new_v4());
        let first = do_post(app.clone(), &path, &token, request.clone(), &key).await;
        assert_eq!(first.0, status, "{}", first.1);
        assert_eq!(first.1["error"]["code"], code);
        if code == "invalid_cause" {
            assert_eq!(first.1["error"]["details"]["detail"], "bad\ncause");
        }
        let receipt = f
            .receipt(&key, i32::from(status), &json!({"error": code}))
            .await;
        assert_eq!(
            do_post(app.clone(), &path, &token, request, &key).await,
            first
        );
        assert_eq!(f.business().await, before);
        assert_eq!(
            f.audit(receipt).await,
            vec![("WAKE_REJECTED".to_string(), Some(code.to_string()))]
        );
        let (events, eligibility): (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM workflow_events WHERE command_id=$1),
                    (SELECT count(*) FROM workflow_dispatch_eligibility_events WHERE command_id=$1)",
        ).bind(receipt).fetch_one(&f.pool).await.unwrap();
        assert_eq!((events, eligibility), (0, 0));
    }
}

#[tokio::test]
async fn wake_disabled_actor_rejection_replays_after_actor_is_enabled() {
    let f = Fixture::new().await;
    let key = format!("wake-disabled-{}", Uuid::new_v4());
    let command = f.command(&key);
    sqlx::query("UPDATE principals SET enabled=FALSE WHERE principal_id=$1")
        .bind(f.scheduler)
        .execute(&f.pool)
        .await
        .unwrap();
    // Application entry intentionally isolates the storage guard. The HTTP
    // scheduler gate remains unchanged and can reject before receipt acquisition.
    assert!(matches!(
        wake_dispatch_intent(&f.pool, command.clone()).await,
        Err(WakeDispatchIntentError::PrincipalDisabled)
    ));
    let receipt = f
        .receipt(&key, 403, &json!({"error": "principal_disabled"}))
        .await;
    sqlx::query("UPDATE principals SET enabled=TRUE WHERE principal_id=$1")
        .bind(f.scheduler)
        .execute(&f.pool)
        .await
        .unwrap();
    let before = f.business().await;
    assert!(matches!(
        wake_dispatch_intent(&f.pool, command).await,
        Err(WakeDispatchIntentError::PrincipalDisabled)
    ));
    assert_eq!(f.business().await, before);
    assert_eq!(
        f.audit(receipt).await,
        vec![(
            "WAKE_REJECTED".to_string(),
            Some("principal_disabled".to_string())
        )]
    );
}

#[tokio::test]
async fn wake_concurrent_same_key_has_one_committed_effect() {
    let f = Fixture::new().await;
    f.defer_fixture().await;
    let before = f.business().await;
    let key = format!("wake-concurrent-same-{}", Uuid::new_v4());
    let command = f.command(&key);
    let (a, b) = tokio::join!(
        wake_dispatch_intent(&f.pool, command.clone()),
        wake_dispatch_intent(&f.pool, command),
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert!(a.wake_applied && b.wake_applied);
    assert_ne!(
        a.replayed, b.replayed,
        "exactly one transaction owns the receipt"
    );
    assert_eq!(
        serde_json::to_value(a).unwrap(),
        serde_json::to_value(b).unwrap()
    );
    assert_eq!(
        f.business().await,
        (before.0 + 1, before.1 + 1, before.2 + 1)
    );
    let (receipts, audits): (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2),
                (SELECT count(*) FROM workflow_command_attempt_audits WHERE principal_id=$1 AND idempotency_key=$2)",
    ).bind(f.scheduler).bind(&key).fetch_one(&f.pool).await.unwrap();
    assert_eq!((receipts, audits), (1, 1));
}

#[tokio::test]
async fn wake_concurrent_distinct_keys_cas_allows_one_applied() {
    let f = Fixture::new().await;
    f.defer_fixture().await;
    let before = f.business().await;
    let key_a = format!("wake-concurrent-a-{}", Uuid::new_v4());
    let key_b = format!("wake-concurrent-b-{}", Uuid::new_v4());
    let (a, b) = tokio::join!(
        wake_dispatch_intent(&f.pool, f.command(&key_a)),
        wake_dispatch_intent(&f.pool, f.command(&key_b)),
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_ne!(a.wake_applied, b.wake_applied);
    let no_op = if a.wake_applied { &b } else { &a };
    assert_eq!(no_op.reason.as_deref(), Some("VERSION_MISMATCH"));
    assert!(!a.replayed && !b.replayed);
    assert_eq!(
        f.business().await,
        (before.0 + 1, before.1 + 1, before.2 + 1)
    );
}

#[tokio::test]
async fn wake_commit_rejection_rolls_back_receipt_and_audit() {
    let f = Fixture::new().await;
    f.defer_fixture().await;
    let suffix = Uuid::new_v4().simple().to_string();
    let key = format!("wake-fail-commit-{suffix}");
    let trigger = format!("wake_receipt_commit_{suffix}");
    // A key-scoped deferred trigger injects a known database commit rejection;
    // it does not simulate lost commit acknowledgement or network UNKNOWN.
    sqlx::query(&format!(
        "CREATE FUNCTION {trigger}() RETURNS trigger LANGUAGE plpgsql AS $$
           BEGIN RAISE EXCEPTION 'test-only deferred receipt commit rejection'; END $$"
    ))
    .execute(&f.pool)
    .await
    .unwrap();
    sqlx::query(&format!(
        "CREATE CONSTRAINT TRIGGER {trigger} AFTER INSERT OR UPDATE ON workflow_command_receipts
         DEFERRABLE INITIALLY DEFERRED FOR EACH ROW
         WHEN (NEW.idempotency_key = '{key}' AND NEW.receipt_status = 'COMPLETED')
         EXECUTE FUNCTION {trigger}()"
    ))
    .execute(&f.pool)
    .await
    .unwrap();
    let before = f.business().await;
    let mut observations = Vec::new();
    for cause in [Some(String::new()), None] {
        let mut command = f.command(&key);
        command.cause = cause;
        // First reject invalid input, then execute a valid future-intent wake.
        // Both transactions reach the same deferred commit rejection. The
        // first known rollback leaves no receipt, so the second may own the key.
        let result = wake_dispatch_intent(&f.pool, command).await;
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2),
                    (SELECT count(*) FROM workflow_command_attempt_audits WHERE principal_id=$1 AND idempotency_key=$2)",
        ).bind(f.scheduler).bind(&key).fetch_one(&f.pool).await.unwrap();
        observations.push((result, counts, f.business().await));
    }
    sqlx::query(&format!(
        "DROP TRIGGER {trigger} ON workflow_command_receipts"
    ))
    .execute(&f.pool)
    .await
    .unwrap();
    sqlx::query(&format!("DROP FUNCTION {trigger}()"))
        .execute(&f.pool)
        .await
        .unwrap();
    for (result, counts, business) in observations {
        assert!(
            matches!(result, Err(WakeDispatchIntentError::StorageError(_))),
            "commit rejection must not be reported as InvalidCause or Applied"
        );
        assert_eq!(counts, (0, 0));
        assert_eq!(business, before, "version/Event/eligibility also roll back");
    }
}
