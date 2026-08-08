//! Admin repair capability tests (operator CLI maintenance path).
//!
//! Coverage:
//!   DRY_RUN_WRITES_NOTHING
//!   APPLY_APPENDS_REVISION_EVENT_AUDIT
//!   NON_AUTHORIZED_OPERATOR_REJECTED
//!   DISABLED_OPERATOR_REJECTED
//!   PAYLOAD_ALTERING_EXISTING_VALUE_REJECTED
//!   PAYLOAD_ADDING_NON_REQUIRED_KEY_REJECTED
//!   PAYLOAD_STILL_MISSING_REQUIRED_KEY_REJECTED
//!   UNKNOWN_PRINCIPAL_IN_PAYLOAD_REJECTED
//!   CANCELLED_INSTANCE_REJECTED
//!   ARCHIVED_INSTANCE_REJECTED
//!
//! Note: the create-time invariant forbids creating a bad instance through
//! the API, so the bad instance is seeded directly via SQL — exactly the
//! shape of the historical half-legal instances this capability repairs.

use svc_workflow::application::workflow_instance::admin_repair::{
    apply_repair_context, plan_repair_context, RepairContextRequest,
};
use svc_workflow::domain::definition::digest;
use svc_workflow::store::postgres::workflow_instance_repository::repair_transaction::{
    RepairContextError, REPAIR_SECURITY_AUDIT_ACTION,
};

use common::{create_pool, seed_domain_owner, seed_principal_and_domain, seed_second_principal};
use sqlx::PgPool;
use uuid::Uuid;

mod common;

// ---------------------------------------------------------------------------
// Seeding: a published IIP definition + a directly inserted bad instance.
// ---------------------------------------------------------------------------

/// Seed a published definition with a NORMAL INSTANCE_INPUT_PRINCIPAL node
/// (key `assigneePrincipalId`). Returns (domain_id, definition_version_id).
async fn seed_iip_definition(pool: &PgPool, domain_id: Uuid) -> (Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("repair-test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Repair Test Def')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(pool)
    .await
    .expect("insert def");

    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1, $2, 1, 'DRAFT', $3)",
    )
    .bind(ver_id)
    .bind(def_id)
    .bind(serde_json::json!({"type": "object"}))
    .execute(pool)
    .await
    .expect("insert version");

    let draft_id = Uuid::new_v4();
    let normal_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id, assignee_input_key) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR', NULL, NULL)",
    )
    .bind(draft_id)
    .bind(ver_id)
    .execute(pool)
    .await
    .expect("insert draft node");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id, assignee_input_key) VALUES ($1, $2, 'do', 'Do', 1, 'NORMAL', 'INSTANCE_INPUT_PRINCIPAL', NULL, 'assigneePrincipalId')",
    )
    .bind(normal_id)
    .bind(ver_id)
    .execute(pool)
    .await
    .expect("insert normal node");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)",
    )
    .bind(term_id)
    .bind(ver_id)
    .execute(pool)
    .await
    .expect("insert terminal node");

    sqlx::query(
        "UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1",
    )
    .bind(ver_id)
    .execute(pool)
    .await
    .expect("publish version");

    (domain_id, ver_id)
}

/// Seed a half-legal instance directly (bypassing create, which now rejects
/// it): context payload carries only `title`; the IIP key is missing.
/// Returns (instance_id, context_revision_id, node_visit_id).
async fn seed_bad_instance(
    pool: &PgPool,
    creator: Uuid,
    domain_id: Uuid,
    definition_version_id: Uuid,
) -> (Uuid, Uuid, Uuid) {
    let instance_id = Uuid::new_v4();
    let revision_id = Uuid::new_v4();
    let visit_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();

    let payload = serde_json::json!({"title": "half-legal instance"});
    let payload_digest = digest::compute_json_digest(&payload).expect("digest");

    let (draft_node_id,): (Uuid,) = sqlx::query_as(
        "SELECT node_id FROM workflow_node_definitions \
         WHERE definition_version_id = $1 AND node_type = 'DRAFT'",
    )
    .bind(definition_version_id)
    .fetch_one(pool)
    .await
    .expect("draft node");

    // The instance/revision/visit/event rows reference each other through
    // deferred composite FKs, so they must be inserted in ONE transaction.
    let mut tx = pool.begin().await.expect("begin seed transaction");

    sqlx::query(
        "INSERT INTO workflow_instances \
         (workflow_instance_id, domain_id, definition_version_id, created_by_principal_id, \
          workflow_state_version, current_context_revision_id, current_node_visit_id) \
         VALUES ($1, $2, $3, $4, 1, $5, $6)",
    )
    .bind(instance_id)
    .bind(domain_id)
    .bind(definition_version_id)
    .bind(creator)
    .bind(revision_id)
    .bind(visit_id)
    .execute(&mut *tx)
    .await
    .expect("insert instance");

    sqlx::query(
        "INSERT INTO workflow_context_revisions \
         (context_revision_id, workflow_instance_id, revision_number, previous_revision_id, \
          payload, payload_digest, created_by_principal_id) \
         VALUES ($1, $2, 1, NULL, $3, $4, $5)",
    )
    .bind(revision_id)
    .bind(instance_id)
    .bind(&payload)
    .bind(&payload_digest)
    .bind(creator)
    .execute(&mut *tx)
    .await
    .expect("insert revision");

    sqlx::query(
        "INSERT INTO workflow_node_visits \
         (node_visit_id, workflow_instance_id, node_id, visit_number, assignee_principal_id) \
         VALUES ($1, $2, $3, 1, $4)",
    )
    .bind(visit_id)
    .bind(instance_id)
    .bind(draft_node_id)
    .bind(creator)
    .execute(&mut *tx)
    .await
    .expect("insert visit");

    let event_data = serde_json::json!({
        "definitionVersionId": definition_version_id,
        "initialNodeId": draft_node_id,
        "assigneeResolutionType": "WORKFLOW_CREATOR",
    });
    let event_data_digest = digest::compute_json_digest(&event_data).expect("event digest");

    sqlx::query(
        "INSERT INTO workflow_events \
         (event_id, workflow_instance_id, event_sequence, event_schema_version, event_type, \
          source_node_visit_id, target_node_visit_id, context_revision_id, event_data, \
          event_data_digest, actor_principal_id, old_workflow_state_version, new_workflow_state_version) \
         VALUES ($1, $2, 1, 'v1', 'INSTANCE_CREATED', NULL, $3, $4, $5, $6, $7, 0, 1)",
    )
    .bind(event_id)
    .bind(instance_id)
    .bind(visit_id)
    .bind(revision_id)
    .bind(&event_data)
    .bind(&event_data_digest)
    .bind(creator)
    .execute(&mut *tx)
    .await
    .expect("insert event");

    tx.commit().await.expect("commit seed transaction");

    (instance_id, revision_id, visit_id)
}

async fn snapshot_counts(pool: &PgPool, instance_id: Uuid) -> (i64, i64, i64, i32) {
    let (revisions,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_context_revisions WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let (events,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM workflow_events WHERE workflow_instance_id = $1")
            .bind(instance_id)
            .fetch_one(pool)
            .await
            .unwrap();
    let (audits,): (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM workflow_security_audits WHERE resource_id = $1")
            .bind(instance_id.to_string())
            .fetch_one(pool)
            .await
            .unwrap();
    let (state_version,): (i32,) = sqlx::query_as(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .unwrap();
    (revisions, events, audits, state_version)
}

fn request(operator: Uuid, instance_id: Uuid, payload: serde_json::Value) -> RepairContextRequest {
    RepairContextRequest {
        operator_principal_id: operator,
        workflow_instance_id: instance_id,
        context_payload: payload,
        reason: "test: repair missing assignee keys".to_string(),
        repair_source: "tests/22_repair_context.rs".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dry_run_returns_plan_without_writing() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let creator = seed_second_principal(&pool).await;
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1, $2, $3, 'MEMBER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(creator)
    .execute(&pool)
    .await
    .unwrap();

    let (_d, ver_id) = seed_iip_definition(&pool, domain_id).await;
    let (instance_id, _, _) = seed_bad_instance(&pool, creator, domain_id, ver_id).await;

    let before = snapshot_counts(&pool, instance_id).await;

    let repaired = serde_json::json!({
        "title": "half-legal instance",
        "assigneePrincipalId": owner,
    });
    let outcome = plan_repair_context(&pool, request(owner, instance_id, repaired))
        .await
        .expect("dry-run plan");

    assert!(!outcome.applied, "dry-run must not apply");
    assert_eq!(outcome.plan.authorization_result, "ok (DOMAIN_OWNER)");
    assert_eq!(
        outcome.plan.missing_required_keys,
        vec!["assigneePrincipalId"]
    );
    assert_eq!(
        outcome.plan.proposed_added_keys,
        vec!["assigneePrincipalId"]
    );
    assert!(outcome.plan.modified_existing_keys.is_empty());
    assert_eq!(outcome.plan.schema_validation, "pass");
    assert_eq!(outcome.plan.post_repair_invariant_result, "pass");

    let after = snapshot_counts(&pool, instance_id).await;
    assert_eq!(before, after, "dry-run must write nothing");
}

#[tokio::test]
async fn apply_appends_revision_event_and_audit() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let creator = seed_second_principal(&pool).await;
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1, $2, $3, 'MEMBER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(creator)
    .execute(&pool)
    .await
    .unwrap();

    let (_d, ver_id) = seed_iip_definition(&pool, domain_id).await;
    let (instance_id, old_revision_id, visit_id) =
        seed_bad_instance(&pool, creator, domain_id, ver_id).await;

    let repaired = serde_json::json!({
        "title": "half-legal instance",
        "assigneePrincipalId": owner,
    });
    let outcome = apply_repair_context(&pool, request(owner, instance_id, repaired))
        .await
        .expect("apply repair");

    assert!(outcome.applied);
    let new_revision_id = outcome.new_context_revision_id.expect("new revision id");
    assert_eq!(outcome.new_revision_number, Some(2));
    assert_eq!(outcome.new_state_version, Some(2));
    assert_eq!(outcome.event_sequence, Some(2));
    assert_eq!(outcome.event_type.as_deref(), Some("CONTEXT_REVISED"));
    assert_eq!(
        outcome.security_audit_action.as_deref(),
        Some(REPAIR_SECURITY_AUDIT_ACTION)
    );

    // Revision appended with previous pointer + digest, payload is a superset.
    let (rev_number, prev, payload, digest_ok): (i32, Option<Uuid>, serde_json::Value, String) =
        sqlx::query_as(
            "SELECT revision_number, previous_revision_id, payload, payload_digest \
             FROM workflow_context_revisions WHERE context_revision_id = $1",
        )
        .bind(new_revision_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(rev_number, 2);
    assert_eq!(prev, Some(old_revision_id));
    assert_eq!(payload["title"], "half-legal instance");
    assert_eq!(payload["assigneePrincipalId"], owner.to_string());
    assert_eq!(
        digest_ok,
        digest::compute_json_digest(&payload).expect("digest"),
        "payload_digest must match the stored payload"
    );

    // Pointer + state version advanced; CONTEXT_REVISED event appended.
    let (current_ctx, state_version): (Uuid, i32) = sqlx::query_as(
        "SELECT current_context_revision_id, workflow_state_version \
         FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(current_ctx, new_revision_id);
    assert_eq!(state_version, 2);

    let (event_seq, event_type, actor): (i32, String, Uuid) = sqlx::query_as(
        "SELECT event_sequence, event_type, actor_principal_id \
         FROM workflow_events WHERE workflow_instance_id = $1 ORDER BY event_sequence DESC LIMIT 1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(event_seq, 2);
    assert_eq!(event_type, "CONTEXT_REVISED");
    assert_eq!(actor, owner);

    let (audit_action, details): (String, serde_json::Value) = sqlx::query_as(
        "SELECT action, details FROM workflow_security_audits WHERE resource_id = $1",
    )
    .bind(instance_id.to_string())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(audit_action, REPAIR_SECURITY_AUDIT_ACTION);
    assert_eq!(details["reason"], "test: repair missing assignee keys");
    assert_eq!(details["repairSource"], "tests/22_repair_context.rs");
    assert_eq!(details["operatorPrincipalId"], owner.to_string());
    assert_eq!(details["addedKeys"][0], "assigneePrincipalId");

    // History preserved: revision 1 + INSTANCE_CREATED event still present.
    let (revisions, events, audits, _) = snapshot_counts(&pool, instance_id).await;
    assert_eq!((revisions, events, audits), (2, 2, 1));
    let _ = visit_id;
}

#[tokio::test]
async fn non_authorized_operator_rejected() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let creator = seed_second_principal(&pool).await;
    let stranger = seed_second_principal(&pool).await; // no role bindings at all

    let (_d, ver_id) = seed_iip_definition(&pool, domain_id).await;
    let (instance_id, _, _) = seed_bad_instance(&pool, creator, domain_id, ver_id).await;

    let before = snapshot_counts(&pool, instance_id).await;
    let repaired = serde_json::json!({
        "title": "half-legal instance",
        "assigneePrincipalId": owner,
    });
    let err = apply_repair_context(&pool, request(stranger, instance_id, repaired))
        .await
        .unwrap_err();
    assert!(
        matches!(err, RepairContextError::OperatorNotAuthorized(_)),
        "expected OperatorNotAuthorized, got {err:?}"
    );
    let after = snapshot_counts(&pool, instance_id).await;
    assert_eq!(before, after, "denied repair must write nothing");
}

#[tokio::test]
async fn disabled_operator_rejected() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let creator = seed_second_principal(&pool).await;
    sqlx::query("UPDATE principals SET enabled = FALSE WHERE principal_id = $1")
        .bind(owner)
        .execute(&pool)
        .await
        .unwrap();

    let (_d, ver_id) = seed_iip_definition(&pool, domain_id).await;
    let (instance_id, _, _) = seed_bad_instance(&pool, creator, domain_id, ver_id).await;

    let repaired = serde_json::json!({
        "title": "half-legal instance",
        "assigneePrincipalId": owner,
    });
    let err = apply_repair_context(&pool, request(owner, instance_id, repaired))
        .await
        .unwrap_err();
    assert!(
        matches!(err, RepairContextError::OperatorPrincipalDisabled),
        "expected OperatorPrincipalDisabled, got {err:?}"
    );
}

#[tokio::test]
async fn payload_altering_existing_value_rejected() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let creator = seed_second_principal(&pool).await;

    let (_d, ver_id) = seed_iip_definition(&pool, domain_id).await;
    let (instance_id, _, _) = seed_bad_instance(&pool, creator, domain_id, ver_id).await;

    // Existing key `title` is CHANGED -> must be rejected.
    let repaired = serde_json::json!({
        "title": "tampered title",
        "assigneePrincipalId": owner,
    });
    let before = snapshot_counts(&pool, instance_id).await;
    let err = apply_repair_context(&pool, request(owner, instance_id, repaired))
        .await
        .unwrap_err();
    assert!(
        matches!(err, RepairContextError::PayloadNotSuperset(_)),
        "expected PayloadNotSuperset, got {err:?}"
    );
    let after = snapshot_counts(&pool, instance_id).await;
    assert_eq!(before, after);
}

#[tokio::test]
async fn payload_adding_non_required_key_rejected() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let creator = seed_second_principal(&pool).await;

    let (_d, ver_id) = seed_iip_definition(&pool, domain_id).await;
    let (instance_id, _, _) = seed_bad_instance(&pool, creator, domain_id, ver_id).await;

    // Adds a key that is NOT a missing required assignee key -> rejected.
    let repaired = serde_json::json!({
        "title": "half-legal instance",
        "assigneePrincipalId": owner,
        "bonusField": "not allowed",
    });
    let err = apply_repair_context(&pool, request(owner, instance_id, repaired))
        .await
        .unwrap_err();
    assert!(
        matches!(err, RepairContextError::PayloadAddsNonRequiredKeys(_)),
        "expected PayloadAddsNonRequiredKeys, got {err:?}"
    );
}

#[tokio::test]
async fn payload_still_missing_required_key_rejected_on_apply() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let creator = seed_second_principal(&pool).await;

    let (_d, ver_id) = seed_iip_definition(&pool, domain_id).await;
    let (instance_id, _, _) = seed_bad_instance(&pool, creator, domain_id, ver_id).await;

    // Payload identical to current: still missing the required key.
    let unchanged = serde_json::json!({"title": "half-legal instance"});
    let plan = plan_repair_context(&pool, request(owner, instance_id, unchanged.clone()))
        .await
        .expect("plan");
    assert_eq!(
        plan.plan.post_repair_invariant_result,
        "fail: missing required assignee key 'assigneePrincipalId'"
    );

    let err = apply_repair_context(&pool, request(owner, instance_id, unchanged))
        .await
        .unwrap_err();
    assert!(
        matches!(err, RepairContextError::InvariantViolation(_)),
        "expected InvariantViolation, got {err:?}"
    );
}

#[tokio::test]
async fn unknown_principal_in_payload_rejected() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let creator = seed_second_principal(&pool).await;

    let (_d, ver_id) = seed_iip_definition(&pool, domain_id).await;
    let (instance_id, _, _) = seed_bad_instance(&pool, creator, domain_id, ver_id).await;

    let ghost = Uuid::new_v4();
    let repaired = serde_json::json!({
        "title": "half-legal instance",
        "assigneePrincipalId": ghost,
    });
    let plan = plan_repair_context(&pool, request(owner, instance_id, repaired.clone()))
        .await
        .expect("plan");
    assert!(
        plan.plan
            .post_repair_invariant_result
            .contains(&format!("unknown principal '{ghost}'")),
        "plan must flag the unknown principal, got: {}",
        plan.plan.post_repair_invariant_result
    );

    let err = apply_repair_context(&pool, request(owner, instance_id, repaired))
        .await
        .unwrap_err();
    assert!(
        matches!(err, RepairContextError::InvariantViolation(_)),
        "expected InvariantViolation, got {err:?}"
    );
}

#[tokio::test]
async fn cancelled_instance_rejected() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let creator = seed_second_principal(&pool).await;

    let (_d, ver_id) = seed_iip_definition(&pool, domain_id).await;
    let (instance_id, _, _) = seed_bad_instance(&pool, creator, domain_id, ver_id).await;
    sqlx::query("UPDATE workflow_instances SET cancelled = TRUE WHERE workflow_instance_id = $1")
        .bind(instance_id)
        .execute(&pool)
        .await
        .unwrap();

    let repaired = serde_json::json!({
        "title": "half-legal instance",
        "assigneePrincipalId": owner,
    });
    let err = apply_repair_context(&pool, request(owner, instance_id, repaired))
        .await
        .unwrap_err();
    assert!(
        matches!(err, RepairContextError::InstanceCancelled),
        "expected InstanceCancelled, got {err:?}"
    );
}

#[tokio::test]
async fn archived_instance_rejected() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let creator = seed_second_principal(&pool).await;

    let (_d, ver_id) = seed_iip_definition(&pool, domain_id).await;
    let (instance_id, _, _) = seed_bad_instance(&pool, creator, domain_id, ver_id).await;
    sqlx::query(
        "UPDATE workflow_instances SET archived_at = now() WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .execute(&pool)
    .await
    .unwrap();

    let repaired = serde_json::json!({
        "title": "half-legal instance",
        "assigneePrincipalId": owner,
    });
    let err = apply_repair_context(&pool, request(owner, instance_id, repaired))
        .await
        .unwrap_err();
    assert!(
        matches!(err, RepairContextError::InstanceArchived),
        "expected InstanceArchived, got {err:?}"
    );
}
