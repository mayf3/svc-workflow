//! Integration tests for Workflow Instance Cancel and Archive V1.
//!
//! Covers:
//! - DOMAIN_OWNER_CANCEL_ACTIVE
//! - NON_OWNER_CANCEL_DENIED
//! - CROSS_DOMAIN_CANCEL_DENIED
//! - CANCEL_CLOSES_ACTIVE_WORK_ITEM
//! - CANCEL_PREVENTS_FURTHER_TRANSITION
//! - DOMAIN_OWNER_ARCHIVE_TERMINAL
//! - ARCHIVE_ACTIVE_INSTANCE_DENIED
//! - NON_OWNER_ARCHIVE_DENIED
//! - CROSS_DOMAIN_ARCHIVE_DENIED
//! - REPEATED_ARCHIVE_IDEMPOTENT
//! - ARCHIVED_DETAIL_STILL_READABLE
//! - ARCHIVED_TIMELINE_PRESERVED

#![allow(clippy::needless_borrow)]
#![allow(unused_imports, unused_variables)]

#[path = "common/mod.rs"]
mod common;

use common::{
    create_pool, seed_domain_owner, seed_principal_and_domain, seed_principal_domain_with_owner,
    seed_second_principal,
};

/// Generate a 64-char hex request hash.
fn req_hash(label: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(label.as_bytes());
    format!("{:x}", hasher.finalize())
}

use sqlx::PgPool;
use uuid::Uuid;

use svc_workflow::application::workflow_instance::archive::archive_workflow_instance;
use svc_workflow::application::workflow_instance::cancel::cancel_workflow_instance;
use svc_workflow::application::workflow_instance::create::create_workflow_instance;
use svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition;
use svc_workflow::application::workflow_instance::query_service::WorkflowQueryService;
use svc_workflow::application::workflow_instance::query_types::{
    GetWorkflowInstanceDetail, ListWorkflowTimeline,
};
use svc_workflow::application::workflow_instance::revise_and_transition::revise_context_and_transition;
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::combined_errors::ReviseContextAndTransitionError;
use svc_workflow::domain::workflow_instance::commands::{
    ArchiveWorkflowInstanceCommand, CancelWorkflowInstanceCommand, CreateWorkflowInstanceCommand,
    ExecuteWorkflowTransitionCommand, ReviseContextAndTransitionCommand,
};
use svc_workflow::domain::workflow_instance::errors::{
    ArchiveWorkflowInstanceError, CancelWorkflowInstanceError, ExecuteWorkflowTransitionError,
};

// ============================================================================
// Helpers
// ============================================================================

/// Seed a published definition with a DRAFT node (WORKFLOW_CREATOR assignee).
/// Returns (domain_id, definition_version_id).
async fn seed_published_definition(pool: &PgPool, domain_id: Uuid) -> (Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("cancel-test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Cancel Test Def')",
    )
    .bind(def_id).bind(domain_id).bind(&def_key)
    .execute(pool).await.expect("insert def");

    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1, $2, 1, 'DRAFT', NULL)",
    )
    .bind(ver_id).bind(def_id)
    .execute(pool).await.expect("insert version");

    let draft_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')",
    )
    .bind(draft_id).bind(ver_id)
    .execute(pool).await.expect("insert draft node");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', NULL)",
    )
    .bind(term_id).bind(ver_id)
    .execute(pool).await.expect("insert terminal node");

    let trans_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance', 'Advance', $3, $4, 'ADVANCE')",
    )
    .bind(trans_id).bind(ver_id).bind(draft_id).bind(term_id)
    .execute(pool).await.expect("insert transition");
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2",
    )
    .bind(trans_id).bind(draft_id)
    .execute(pool).await.expect("set primary");

    sqlx::query(
        "UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1",
    )
    .bind(ver_id)
    .execute(pool).await.expect("publish version");

    (domain_id, ver_id)
}

/// Create a workflow instance and return its ID + initial state version.
async fn create_instance(
    pool: &PgPool,
    creator_id: Uuid,
    domain_id: Uuid,
    definition_version_id: Uuid,
) -> (Uuid, i32) {
    let result = create_workflow_instance(
        pool,
        svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(creator_id),
            idempotency_key: format!("create-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(definition_version_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({}),
            context_payload: serde_json::json!({}),
        },
    )
    .await
    .expect("create instance should succeed");

    (result.workflow_instance_id, result.workflow_state_version)
}

/// Execute an ADVANCE transition to move an instance from DRAFT to TERMINAL.
async fn advance_to_terminal(
    pool: &PgPool,
    actor_id: Uuid,
    instance_id: Uuid,
    state_version: i32,
    definition_version_id: Uuid,
) -> i32 {
    // We need the transition definition ID from the definition graph
    let trans_id: Uuid = sqlx::query_scalar(
        "SELECT transition_id FROM workflow_transition_definitions WHERE definition_version_id = $1 ORDER BY transition_key LIMIT 1",
    )
    .bind(definition_version_id)
    .fetch_one(pool)
    .await
    .expect("find transition id");

    let result = execute_workflow_transition(
        pool,
        svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        ExecuteWorkflowTransitionCommand {
            principal_id: PrincipalId::from_uuid(actor_id),
            idempotency_key: format!("advance-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_version,
            transition_definition_id: TransitionId::from_uuid(trans_id),
            submission_payload: None,
        },
    )
    .await
    .expect("advance to terminal should succeed");

    result.workflow_state_version
}

// ============================================================================
// Cancel Tests
// ============================================================================

#[sqlx::test]
async fn domain_owner_cancel_active(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;

    let result = cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "cancel-1".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver,
            reason: "duplicate_instance".to_string(),
        },
        &req_hash("cancel-1"),
    )
    .await
    .expect("domain owner cancel active should succeed");

    assert_eq!(result.workflow_instance_id, instance_id);
    assert_eq!(result.workflow_state_version, state_ver + 1);
    assert!(!result.replayed);

    // Verify DB state
    let row: (bool,) =
        sqlx::query_as("SELECT cancelled FROM workflow_instances WHERE workflow_instance_id = $1")
            .bind(instance_id)
            .fetch_one(&pool)
            .await
            .expect("find instance");
    assert!(row.0, "instance should be marked cancelled");

    // Verify timeline event
    let query_service = WorkflowQueryService::new(pool.clone());
    let timeline = query_service
        .list_workflow_timeline(ListWorkflowTimeline {
            actor_principal_id: owner_id,
            workflow_instance_id: instance_id,
            after_event_sequence: None,
            limit: Some(100),
        })
        .await
        .expect("list timeline");
    let cancel_events: Vec<_> = timeline
        .items
        .iter()
        .filter(|e| e.event_type == "WORKFLOW_INSTANCE_CANCELLED")
        .collect();
    assert_eq!(
        cancel_events.len(),
        1,
        "should have exactly one cancel event"
    );
    assert_eq!(cancel_events[0].actor_principal_id, owner_id);
    let event_data = cancel_events[0].event_data.as_ref().expect("event data");
    assert_eq!(event_data["reason"], "duplicate_instance");
}

#[sqlx::test]
async fn non_owner_cancel_denied(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let non_owner_id = seed_second_principal(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;

    let err = cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(non_owner_id),
            idempotency_key: "cancel-2".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash2"),
    )
    .await
    .expect_err("non-owner cancel should be denied");

    assert!(matches!(err, CancelWorkflowInstanceError::NotDomainOwner));
}

#[sqlx::test]
async fn cross_domain_cancel_denied(pool: PgPool) {
    let (owner_a_id, domain_a_id) = seed_principal_domain_with_owner(&pool).await;
    let (other_id, domain_b_id) = seed_principal_and_domain(&pool).await;
    // Make other_id the domain owner of domain_b but NOT domain_a
    seed_domain_owner(&pool, domain_b_id, other_id).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_a_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_a_id, domain_a_id, ver_id).await;

    let err = cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(other_id),
            idempotency_key: "cancel-3".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash3"),
    )
    .await
    .expect_err("cross-domain cancel should be denied");

    assert!(matches!(err, CancelWorkflowInstanceError::NotDomainOwner));
}

#[sqlx::test]
async fn cancel_closes_active_work_item(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;

    // Verify assignee is set before cancel
    let assignee_before: Option<Uuid> = sqlx::query_scalar(
        "SELECT v.assignee_principal_id
         FROM workflow_instances wi
         JOIN workflow_node_visits v ON v.node_visit_id = wi.current_node_visit_id
         WHERE wi.workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("find assignee");
    assert_eq!(
        assignee_before,
        Some(owner_id),
        "assignee should be set before cancel"
    );

    // Cancel
    cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "cancel-4".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash4"),
    )
    .await
    .expect("cancel should succeed");

    // Verify cancelled flag is set (the cancelled flag + worklist filter +
    // transition block effectively close the work item; the DB CHECK constraint
    // prevents nullifying the assignee on a non-terminal node visit)
    let cancelled_after: bool = sqlx::query_scalar(
        "SELECT cancelled FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("find cancelled flag");
    assert!(
        cancelled_after,
        "instance should be marked cancelled after cancel"
    );

    // Verify the instance is excluded from worklists (by cancelled flag)
    let in_worklist: bool = sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM workflow_instances wi
           JOIN workflow_node_visits v ON v.node_visit_id = wi.current_node_visit_id
           JOIN workflow_node_definitions n ON n.node_id = v.node_id
           WHERE v.assignee_principal_id = $1 AND n.node_type <> 'TERMINAL'
             AND wi.cancelled = FALSE
             AND wi.workflow_instance_id = $2)",
    )
    .bind(owner_id)
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("check worklist");
    assert!(
        !in_worklist,
        "cancelled instance should not appear in worklist"
    );
}

#[sqlx::test]
async fn cancel_prevents_further_transition(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;

    // Cancel
    cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "cancel-5".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash5"),
    )
    .await
    .expect("cancel should succeed");

    // Try to transition — should fail because instance is cancelled
    let err = execute_workflow_transition(
        &pool,
        svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        ExecuteWorkflowTransitionCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "advance-after-cancel".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver + 1,
            transition_definition_id: TransitionId::from_uuid(Uuid::nil()),
            submission_payload: None,
        },
    )
    .await
    .expect_err("transition should be denied after cancel");

    assert!(
        matches!(err, ExecuteWorkflowTransitionError::SourceNodeTerminal),
        "expected SourceNodeTerminal or InstanceNotFound, got {:?}",
        err
    );
}

#[sqlx::test]
async fn cancel_terminal_instance_denied(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;

    // Advance to terminal
    let new_ver = advance_to_terminal(&pool, owner_id, instance_id, state_ver, ver_id).await;

    // Try to cancel terminal instance
    let err = cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "cancel-terminal".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: new_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-term"),
    )
    .await
    .expect_err("cancel terminal instance should be denied");

    assert!(matches!(
        err,
        CancelWorkflowInstanceError::SourceNodeTerminal
    ));
}

#[sqlx::test]
async fn cancel_reason_validation(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;

    // Empty reason
    let err = cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "cancel-empty".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver,
            reason: "".to_string(),
        },
        &req_hash("hash-empty"),
    )
    .await
    .expect_err("empty reason should be rejected");
    assert!(matches!(err, CancelWorkflowInstanceError::InvalidReason(_)));
}

// ============================================================================
// Archive Tests
// ============================================================================

#[sqlx::test]
async fn domain_owner_archive_terminated_instance(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;

    // Advance to terminal
    let new_ver = advance_to_terminal(&pool, owner_id, instance_id, state_ver, ver_id).await;

    // Archive
    let result = archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-1".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: new_ver,
            reason: "test_instance_cleanup".to_string(),
        },
        &req_hash("hash-arch1"),
    )
    .await
    .expect("domain owner archive terminal instance should succeed");

    assert_eq!(result.workflow_instance_id, instance_id);
    assert!(!result.replayed);

    // Verify DB state
    let row: (Option<Uuid>, Option<String>) = sqlx::query_as(
        "SELECT archived_by_principal_id, archive_reason FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("find instance");
    assert_eq!(row.0, Some(owner_id), "archived_by should be owner");
    assert_eq!(
        row.1.as_deref(),
        Some("test_instance_cleanup"),
        "archive reason should match"
    );
}

#[sqlx::test]
async fn domain_owner_archive_cancelled_instance(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;

    // Cancel first
    let cancel_result = cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "cancel-then-archive".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver,
            reason: "duplicate".to_string(),
        },
        &req_hash("hash-ca"),
    )
    .await
    .expect("cancel should succeed");

    // Archive
    let result = archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-cancelled".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: cancel_result.workflow_state_version,
            reason: "cleanup_cancelled".to_string(),
        },
        &req_hash("hash-arch2"),
    )
    .await
    .expect("domain owner archive cancelled instance should succeed");

    assert_eq!(result.workflow_instance_id, instance_id);

    // Verify timeline has both cancel and archive events
    let query_service = WorkflowQueryService::new(pool.clone());
    let timeline = query_service
        .list_workflow_timeline(ListWorkflowTimeline {
            actor_principal_id: owner_id,
            workflow_instance_id: instance_id,
            after_event_sequence: None,
            limit: Some(100),
        })
        .await
        .expect("list timeline");
    let archive_events: Vec<_> = timeline
        .items
        .iter()
        .filter(|e| e.event_type == "WORKFLOW_INSTANCE_ARCHIVED")
        .collect();
    assert_eq!(
        archive_events.len(),
        1,
        "should have exactly one archive event"
    );
}

#[sqlx::test]
async fn archive_active_instance_denied(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;

    // Try to archive active (non-terminal) instance
    let err = archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-active".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-act"),
    )
    .await
    .expect_err("archive active instance should be denied");

    assert!(
        matches!(err, ArchiveWorkflowInstanceError::InstanceNotTerminal),
        "expected InstanceNotTerminal, got {:?}",
        err
    );
}

#[sqlx::test]
async fn non_owner_archive_denied(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let non_owner_id = seed_second_principal(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    let new_ver = advance_to_terminal(&pool, owner_id, instance_id, state_ver, ver_id).await;

    let err = archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(non_owner_id),
            idempotency_key: "archive-nonowner".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: new_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-no"),
    )
    .await
    .expect_err("non-owner archive should be denied");

    assert!(matches!(err, ArchiveWorkflowInstanceError::NotDomainOwner));
}

#[sqlx::test]
async fn cross_domain_archive_denied(pool: PgPool) {
    let (owner_a_id, domain_a_id) = seed_principal_domain_with_owner(&pool).await;
    let (other_id, domain_b_id) = seed_principal_and_domain(&pool).await;
    // Make other_id the domain owner of domain_b but NOT domain_a
    seed_domain_owner(&pool, domain_b_id, other_id).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_a_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_a_id, domain_a_id, ver_id).await;
    let new_ver = advance_to_terminal(&pool, owner_a_id, instance_id, state_ver, ver_id).await;

    let err = archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(other_id),
            idempotency_key: "archive-cross".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: new_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-cross"),
    )
    .await
    .expect_err("cross-domain archive should be denied");

    assert!(matches!(err, ArchiveWorkflowInstanceError::NotDomainOwner));
}

#[sqlx::test]
async fn repeated_archive_idempotent(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    let new_ver = advance_to_terminal(&pool, owner_id, instance_id, state_ver, ver_id).await;

    // First archive
    archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-idem".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: new_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-idem"),
    )
    .await
    .expect("first archive should succeed");

    // But the state version has changed after archive, so we need to query the new version
    let (current_state_ver,): (i32,) = sqlx::query_as(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("find state version");
    assert_eq!(
        current_state_ver,
        new_ver + 1,
        "archive should increment state version"
    );

    // Second archive with same key (idempotent replay)
    let result = archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-idem".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: current_state_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-idem"),
    )
    .await
    .expect("repeated archive should succeed via idempotency");
    assert!(result.replayed, "should be a replayed result");

    // Verify only one archive event exists
    let query_service = WorkflowQueryService::new(pool.clone());
    let timeline = query_service
        .list_workflow_timeline(ListWorkflowTimeline {
            actor_principal_id: owner_id,
            workflow_instance_id: instance_id,
            after_event_sequence: None,
            limit: Some(100),
        })
        .await
        .expect("list timeline");
    let archive_events: Vec<_> = timeline
        .items
        .iter()
        .filter(|e| e.event_type == "WORKFLOW_INSTANCE_ARCHIVED")
        .collect();
    assert_eq!(
        archive_events.len(),
        1,
        "should still have exactly one archive event"
    );
}

#[sqlx::test]
async fn archived_detail_still_readable(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    let new_ver = advance_to_terminal(&pool, owner_id, instance_id, state_ver, ver_id).await;

    // Archive
    archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-read".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: new_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-read"),
    )
    .await
    .expect("archive should succeed");

    // Detail should still be readable via query service
    let query_service = WorkflowQueryService::new(pool.clone());
    let detail = query_service
        .get_workflow_instance_detail(GetWorkflowInstanceDetail {
            actor_principal_id: owner_id,
            workflow_instance_id: instance_id,
        })
        .await
        .expect("detail should still be readable after archive");

    // Basic detail checks
    match &detail {
        svc_workflow::application::workflow_instance::query_types::WorkflowInstanceDetail::Full(
            f,
        ) => {
            assert_eq!(f.instance.workflow_instance_id, instance_id);
        }
        _ => panic!("expected full detail for domain owner"),
    }
}

#[sqlx::test]
async fn archived_timeline_preserved(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    let new_ver = advance_to_terminal(&pool, owner_id, instance_id, state_ver, ver_id).await;

    // Check timeline count before archive
    let query_service = WorkflowQueryService::new(pool.clone());
    let timeline_before = query_service
        .list_workflow_timeline(ListWorkflowTimeline {
            actor_principal_id: owner_id,
            workflow_instance_id: instance_id,
            after_event_sequence: None,
            limit: Some(100),
        })
        .await
        .expect("list timeline before archive");
    let count_before = timeline_before.items.len();

    // Archive
    archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-tl".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: new_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-tl"),
    )
    .await
    .expect("archive should succeed");

    // Timeline should have exactly one more event (the archive event)
    let timeline_after = query_service
        .list_workflow_timeline(ListWorkflowTimeline {
            actor_principal_id: owner_id,
            workflow_instance_id: instance_id,
            after_event_sequence: None,
            limit: Some(100),
        })
        .await
        .expect("list timeline after archive");
    let count_after = timeline_after.items.len();
    assert_eq!(
        count_after,
        count_before + 1,
        "timeline should have one more event after archive"
    );

    // The last event should be the archive event
    let last_event = timeline_after.items.last().expect("last event");
    assert_eq!(last_event.event_type, "WORKFLOW_INSTANCE_ARCHIVED");
    assert_eq!(last_event.actor_principal_id, owner_id);
}

#[sqlx::test]
async fn archive_reason_validation(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    let new_ver = advance_to_terminal(&pool, owner_id, instance_id, state_ver, ver_id).await;

    // Empty reason
    let err = archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-empty".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: new_ver,
            reason: "".to_string(),
        },
        &req_hash("hash-empty"),
    )
    .await
    .expect_err("empty reason should be rejected");
    assert!(matches!(
        err,
        ArchiveWorkflowInstanceError::InvalidReason(_)
    ));
}

#[sqlx::test]
async fn cancelled_instance_cannot_advance_via_combined_path(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;

    // Cancel first
    let cancel_result = cancel_workflow_instance(
        &pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "cancel-before-combined".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver,
            reason: "duplicate".to_string(),
        },
        &req_hash("hash-combined"),
    )
    .await
    .expect("cancel should succeed");

    // The combined revise+advance command must not be able to move a cancelled
    // instance forward (regression guard for the combined path).
    let trans_id: Uuid = sqlx::query_scalar(
        "SELECT transition_id FROM workflow_transition_definitions WHERE definition_version_id = $1 ORDER BY transition_key LIMIT 1",
    )
    .bind(ver_id)
    .fetch_one(&pool)
    .await
    .expect("find transition id");

    let err = revise_context_and_transition(
        &pool,
        svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        ReviseContextAndTransitionCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "combined-after-cancel".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: cancel_result.workflow_state_version,
            transition_definition_id: TransitionId::from_uuid(trans_id),
            context_payload: serde_json::json!({}),
            submission_payload: serde_json::json!({}),
        },
    )
    .await
    .expect_err("combined revise+advance must be denied on a cancelled instance");

    assert!(
        matches!(err, ReviseContextAndTransitionError::CurrentNodeNotDraft),
        "expected CurrentNodeNotDraft, got {:?}",
        err
    );
}

#[sqlx::test]
async fn archive_new_key_already_archived_rejected(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    let new_ver = advance_to_terminal(&pool, owner_id, instance_id, state_ver, ver_id).await;

    // First archive with key A succeeds.
    archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-once-a".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: new_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-once-a"),
    )
    .await
    .expect("first archive should succeed");

    // Snapshot invariants after the first archive.
    let (archived_at_before, reason_before, state_after_first): (
        chrono::DateTime<chrono::Utc>,
        String,
        i32,
    ) = sqlx::query_as(
        "SELECT archived_at, archive_reason, workflow_state_version
             FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("read archive state");
    let event_count_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_events
         WHERE workflow_instance_id = $1 AND event_type = 'WORKFLOW_INSTANCE_ARCHIVED'",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("count archive events");

    // New idempotency key on the already-archived instance -> rejected.
    let err = archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-once-b".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: 0,
            reason: "another_cleanup".to_string(),
        },
        &req_hash("hash-once-b"),
    )
    .await
    .expect_err("second archive with a new key must be rejected");

    assert!(
        matches!(err, ArchiveWorkflowInstanceError::AlreadyArchived),
        "expected AlreadyArchived, got {:?}",
        err
    );

    // Invariants: nothing changed, no success receipt for key B.
    let (archived_at_after, reason_after, state_after): (
        chrono::DateTime<chrono::Utc>,
        String,
        i32,
    ) = sqlx::query_as(
        "SELECT archived_at, archive_reason, workflow_state_version
             FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("read archive state after rejection");
    assert_eq!(
        archived_at_after, archived_at_before,
        "archived_at must be unchanged"
    );
    assert_eq!(
        reason_after, reason_before,
        "archive_reason must be unchanged"
    );
    assert_eq!(
        state_after, state_after_first,
        "workflow_state_version must be unchanged"
    );

    let event_count_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_events
         WHERE workflow_instance_id = $1 AND event_type = 'WORKFLOW_INSTANCE_ARCHIVED'",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("count archive events after rejection");
    assert_eq!(
        event_count_after, event_count_before,
        "archive event count must be unchanged"
    );
    assert_eq!(event_count_after, 1, "exactly one archive event total");

    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_command_receipts
         WHERE principal_id = $1 AND idempotency_key = 'archive-once-b'",
    )
    .bind(owner_id)
    .fetch_one(&pool)
    .await
    .expect("count receipts for rejected key");
    assert_eq!(
        receipt_count, 0,
        "no success receipt may exist for the rejected key"
    );
}

#[sqlx::test]
async fn archive_same_key_different_request_conflicts(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    let new_ver = advance_to_terminal(&pool, owner_id, instance_id, state_ver, ver_id).await;

    // First archive with key X succeeds (request hash H1).
    archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-same-key".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: new_ver,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-same-1"),
    )
    .await
    .expect("first archive should succeed");

    // Same key but a different request hash -> idempotency conflict, not replay.
    let err = archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-same-key".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: 0,
            reason: "different_reason".to_string(),
        },
        &req_hash("hash-same-2"),
    )
    .await
    .expect_err("same key with a different request must conflict");

    assert!(
        matches!(
            err,
            ArchiveWorkflowInstanceError::IdempotencyConflict { .. }
        ),
        "expected IdempotencyConflict, got {:?}",
        err
    );

    // Same key with the ORIGINAL hash still replays.
    let replay = archive_workflow_instance(
        &pool,
        ArchiveWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: "archive-same-key".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: 0,
            reason: "cleanup".to_string(),
        },
        &req_hash("hash-same-1"),
    )
    .await
    .expect("same key with original request must replay");
    assert!(replay.replayed, "expected a replayed result");

    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_events
         WHERE workflow_instance_id = $1 AND event_type = 'WORKFLOW_INSTANCE_ARCHIVED'",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("count archive events");
    assert_eq!(
        event_count, 1,
        "exactly one archive event after conflict + replay"
    );
}

#[sqlx::test]
async fn concurrent_archive_different_keys_exactly_one_succeeds(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    let new_ver = advance_to_terminal(&pool, owner_id, instance_id, state_ver, ver_id).await;

    // Two different idempotency keys archive the same terminal instance
    // concurrently. Both use the HTTP adapter sentinel (expected version 0,
    // server-authoritative), so the loser is rejected by the in-transaction
    // archived guard rather than a stale-version conflict. The FOR UPDATE row
    // lock + guard guarantee exactly one success regardless of interleaving.
    let hash_a = req_hash("hash-concurrent-a");
    let hash_b = req_hash("hash-concurrent-b");
    let (result_a, result_b) = tokio::join!(
        archive_workflow_instance(
            &pool,
            ArchiveWorkflowInstanceCommand {
                principal_id: PrincipalId::from_uuid(owner_id),
                idempotency_key: "archive-concurrent-a".to_string(),
                command_schema_version: "v1".to_string(),
                workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
                expected_workflow_state_version: 0,
                reason: "cleanup_a".to_string(),
            },
            &hash_a,
        ),
        archive_workflow_instance(
            &pool,
            ArchiveWorkflowInstanceCommand {
                principal_id: PrincipalId::from_uuid(owner_id),
                idempotency_key: "archive-concurrent-b".to_string(),
                command_schema_version: "v1".to_string(),
                workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
                expected_workflow_state_version: 0,
                reason: "cleanup_b".to_string(),
            },
            &hash_b,
        ),
    );

    let success_count = match (&result_a, &result_b) {
        (Ok(_), Ok(_)) => 2,
        (Ok(_), Err(_)) | (Err(_), Ok(_)) => 1,
        (Err(_), Err(_)) => 0,
    };
    assert_eq!(
        success_count, 1,
        "exactly one concurrent archive must succeed"
    );
    for result in [&result_a, &result_b] {
        if let Err(err) = result {
            assert!(
                matches!(err, ArchiveWorkflowInstanceError::AlreadyArchived),
                "the loser must be AlreadyArchived, got {:?}",
                err
            );
        }
    }

    // Exactly one archive event and a single archived_at write.
    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_events
         WHERE workflow_instance_id = $1 AND event_type = 'WORKFLOW_INSTANCE_ARCHIVED'",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("count archive events");
    assert_eq!(
        event_count, 1,
        "exactly one archive event after concurrent archive"
    );

    let archived_at_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_instances
         WHERE workflow_instance_id = $1 AND archived_at IS NOT NULL",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .expect("count archived_at writes");
    assert_eq!(
        archived_at_count, 1,
        "archived_at must be written exactly once"
    );
}

// ============================================================================
// SVC_WORKFLOW_DANGLING_INSTANCE_CANCEL_V1 (M1B closure) — T1..T8
// ============================================================================

/// Force the dangling shape: `workflow_instances.current_node_visit_id IS NULL`.
/// Historical visit rows stay (visits are append-only); the shape keys on the
/// instance pointer exactly like the production F4 rows.
async fn make_dangling(pool: &PgPool, instance_id: Uuid) {
    sqlx::query("UPDATE workflow_instances SET current_node_visit_id = NULL WHERE workflow_instance_id = $1")
        .bind(instance_id)
        .execute(pool)
        .await
        .expect("detach current visit pointer");
}

/// Seed one OPEN runtime fact (workflow_activations row without closure) of
/// the given canonical kind, bound to one of the instance's visit rows.
async fn seed_open_activation(
    pool: &PgPool,
    instance_id: Uuid,
    owner_id: Uuid,
    kind: &str,
) -> Uuid {
    let visit_id: Uuid = sqlx::query_scalar(
        "SELECT node_visit_id FROM workflow_node_visits WHERE workflow_instance_id = $1 ORDER BY created_at LIMIT 1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .expect("instance has a historical visit row");

    let activation_id = Uuid::new_v4();
    let initial_next_eligible_at: Option<chrono::DateTime<chrono::Utc>> =
        if kind == "DISPATCH_INTENT" {
            Some(chrono::Utc::now())
        } else {
            None
        };
    sqlx::query(
        "INSERT INTO workflow_activations
           (activation_id, workflow_instance_id, node_visit_id, activation_kind,
            owner_principal_id, activation_at, initial_next_eligible_at, command_id)
         VALUES ($1, $2, $3, $4::activation_kind, $5, now(), $6, $7)",
    )
    .bind(activation_id)
    .bind(instance_id)
    .bind(visit_id)
    .bind(kind)
    .bind(owner_id)
    .bind(initial_next_eligible_at)
    .bind(Uuid::new_v4())
    .execute(pool)
    .await
    .expect("seed open activation");
    activation_id
}

async fn m1b_snapshot(pool: &PgPool, instance_id: Uuid) -> (bool, Option<Uuid>, i32, i64, i64) {
    let row: (bool, Option<Uuid>, i32) = sqlx::query_as(
        "SELECT cancelled, current_node_visit_id, workflow_state_version
           FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .expect("instance row");
    let cancel_events: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_events
          WHERE workflow_instance_id = $1 AND event_type = 'WORKFLOW_INSTANCE_CANCELLED'",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .expect("cancel event count");
    let open_facts: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_activations a
          LEFT JOIN workflow_activation_closures c ON c.activation_id = a.activation_id
          WHERE a.workflow_instance_id = $1 AND c.activation_id IS NULL",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .expect("open runtime fact count");
    (row.0, row.1, row.2, cancel_events, open_facts)
}

async fn m1b_cancel(
    pool: &PgPool,
    owner_id: Uuid,
    instance_id: Uuid,
    state_ver: i32,
    key: &str,
    hash: &str,
    reason: &str,
) -> Result<(), CancelWorkflowInstanceError> {
    cancel_workflow_instance(
        pool,
        CancelWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner_id),
            idempotency_key: key.to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: state_ver,
            reason: reason.to_string(),
        },
        &req_hash(hash),
    )
    .await
    .map(|_| ())
}

/// T1 — dangling + no open runtime fact: cancel succeeds; event
/// source_node_visit_id NULL; exactly one CANCELLED event; version +1; no
/// synthetic visit.
#[sqlx::test]
async fn m1b_t1_dangling_without_runtime_fact_cancels(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    make_dangling(&pool, instance_id).await;

    let visits_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_node_visits WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    m1b_cancel(
        &pool,
        owner_id,
        instance_id,
        state_ver,
        "m1b-t1",
        "m1b-t1",
        "m1b dangling cleanup",
    )
    .await
    .expect("dangling cancel without runtime facts must succeed");

    let (cancelled, current_visit, version, cancel_events, open_facts) =
        m1b_snapshot(&pool, instance_id).await;
    assert!(cancelled, "instance must be cancelled");
    assert!(
        current_visit.is_none(),
        "pointer must stay NULL (never had a current visit)"
    );
    assert_eq!(version, state_ver + 1, "exactly one version increment");
    assert_eq!(cancel_events, 1, "exactly one CANCELLED event");
    assert_eq!(open_facts, 0, "no runtime facts created");
    let source_visit: Option<Uuid> = sqlx::query_scalar(
        "SELECT source_node_visit_id FROM workflow_events
          WHERE workflow_instance_id = $1 AND event_type = 'WORKFLOW_INSTANCE_CANCELLED'",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(source_visit.is_none(), "event source visit must be NULL");
    let visits_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_node_visits WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        visits_before, visits_after,
        "no synthetic visit may be created"
    );
}

/// T2/T3/T4 — dangling + ANY open runtime fact (the canonical model's
/// complete activation-kind set) -> fail closed with zero mutation.
#[sqlx::test]
async fn m1b_t2_t3_t4_dangling_open_runtime_fact_fail_closed(pool: PgPool) {
    for kind in ["DISPATCH_INTENT", "HUMAN_WORK_ITEM"] {
        let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
        let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
        let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
        make_dangling(&pool, instance_id).await;
        seed_open_activation(&pool, instance_id, owner_id, kind).await;

        let before = m1b_snapshot(&pool, instance_id).await;
        let err = m1b_cancel(
            &pool,
            owner_id,
            instance_id,
            state_ver,
            "m1b-refuse",
            "m1b-refuse",
            "m1b refused",
        )
        .await
        .expect_err("open runtime fact must fail closed");
        assert!(
            matches!(err, CancelWorkflowInstanceError::InternalConsistency(_)),
            "{kind}: expected InternalConsistency, got {err:?}"
        );

        let after = m1b_snapshot(&pool, instance_id).await;
        assert_eq!(
            before, after,
            "{kind}: instance/runtime facts/version/events must be unchanged"
        );
        assert!(!after.0, "{kind}: instance must stay uncancelled");
        assert_eq!(after.3, 0, "{kind}: zero CANCELLED events");
        assert_eq!(
            after.4, 1,
            "{kind}: the open runtime fact must stay open (never auto-closed)"
        );
    }
}

/// T4b — a dangling instance whose only runtime fact is CLOSED is again
/// eligible (the invariant is about open facts, not their existence).
#[sqlx::test]
async fn m1b_t4b_dangling_closed_runtime_fact_is_eligible(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    make_dangling(&pool, instance_id).await;
    let activation_id = seed_open_activation(&pool, instance_id, owner_id, "DISPATCH_INTENT").await;
    sqlx::query(
        "INSERT INTO workflow_activation_closures
           (activation_id, closure_reason, closed_at, command_id)
         VALUES ($1, 'CANCELLED', now(), $2)",
    )
    .bind(activation_id)
    .bind(Uuid::new_v4())
    .execute(&pool)
    .await
    .expect("close the seeded activation");

    m1b_cancel(
        &pool,
        owner_id,
        instance_id,
        state_ver,
        "m1b-t4b",
        "m1b-t4b",
        "closed fact eligible",
    )
    .await
    .expect("dangling instance with only closed runtime facts must be cancellable");
    let (cancelled, _, _, cancel_events, open_facts) = m1b_snapshot(&pool, instance_id).await;
    assert!(cancelled);
    assert_eq!(cancel_events, 1);
    assert_eq!(open_facts, 0);
}

/// T5 — normal current-visit cancel is unregressed: event binds the source
/// visit, version +1, exactly one event.
#[sqlx::test]
async fn m1b_t5_normal_cancel_unregressed(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    let current_visit: Uuid = sqlx::query_scalar(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .unwrap();

    m1b_cancel(
        &pool,
        owner_id,
        instance_id,
        state_ver,
        "m1b-t5",
        "m1b-t5",
        "normal cleanup",
    )
    .await
    .expect("normal cancel must still succeed");

    let (cancelled, _, version, cancel_events, _) = m1b_snapshot(&pool, instance_id).await;
    assert!(cancelled);
    assert_eq!(version, state_ver + 1);
    assert_eq!(cancel_events, 1);
    let bound_visit: Option<Uuid> = sqlx::query_scalar(
        "SELECT source_node_visit_id FROM workflow_events
          WHERE workflow_instance_id = $1 AND event_type = 'WORKFLOW_INSTANCE_CANCELLED'",
    )
    .bind(instance_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        bound_visit,
        Some(current_visit),
        "non-dangling event must bind the current visit"
    );
}

/// T6 — dangling cancel: same key + same request replays; the operation is
/// applied exactly once.
#[sqlx::test]
async fn m1b_t6_dangling_cancel_replay_exactly_once(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    make_dangling(&pool, instance_id).await;

    m1b_cancel(
        &pool,
        owner_id,
        instance_id,
        state_ver,
        "m1b-t6",
        "m1b-t6",
        "replay cleanup",
    )
    .await
    .expect("first cancel succeeds");
    m1b_cancel(
        &pool,
        owner_id,
        instance_id,
        state_ver,
        "m1b-t6",
        "m1b-t6",
        "replay cleanup",
    )
    .await
    .expect("same-key same-request replay must succeed");

    let (_, _, version, cancel_events, _) = m1b_snapshot(&pool, instance_id).await;
    assert_eq!(
        cancel_events, 1,
        "exactly one CANCELLED event across replay"
    );
    assert_eq!(version, state_ver + 1, "version incremented exactly once");
}

/// T7 — dangling cancel: same key + different request conflicts without
/// mutation.
#[sqlx::test]
async fn m1b_t7_dangling_cancel_same_key_different_request_conflicts(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    make_dangling(&pool, instance_id).await;

    m1b_cancel(
        &pool,
        owner_id,
        instance_id,
        state_ver,
        "m1b-t7",
        "m1b-t7-a",
        "first reason",
    )
    .await
    .expect("first cancel succeeds");

    let before = m1b_snapshot(&pool, instance_id).await;
    let err = m1b_cancel(
        &pool,
        owner_id,
        instance_id,
        state_ver,
        "m1b-t7",
        "m1b-t7-b",
        "different reason",
    )
    .await
    .expect_err("same key with different request must conflict");
    assert!(
        matches!(err, CancelWorkflowInstanceError::IdempotencyConflict { .. }),
        "expected IdempotencyConflict, got {err:?}"
    );
    let after = m1b_snapshot(&pool, instance_id).await;
    assert_eq!(before, after, "conflict must not mutate anything");
}

/// T8 — fault inside the transaction rolls back completely: no partial
/// cancelled flag, no orphan CANCELLED event, no version change, runtime
/// facts untouched.
#[sqlx::test]
async fn m1b_t8_fault_rolls_back_completely(pool: PgPool) {
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id) = seed_published_definition(&pool, domain_id).await;
    let (instance_id, state_ver) = create_instance(&pool, owner_id, domain_id, ver_id).await;
    make_dangling(&pool, instance_id).await;

    // Pre-occupy the next event sequence slot so the Step 9 event insert
    // fails with a UNIQUE violation AFTER the guarded instance update.
    sqlx::query(
        "INSERT INTO workflow_events
           (event_id, workflow_instance_id, event_sequence, event_schema_version,
            event_type, actor_principal_id,
            old_workflow_state_version, new_workflow_state_version)
         VALUES ($1, $2, $3, 'v1', 'WORKFLOW_INSTANCE_CANCELLED', $4, $5, $6)",
    )
    .bind(Uuid::new_v4())
    .bind(instance_id)
    .bind(state_ver + 1)
    .bind(owner_id)
    .bind(state_ver)
    .bind(state_ver + 1)
    .execute(&pool)
    .await
    .expect("occupy the next event sequence slot");

    let before = m1b_snapshot(&pool, instance_id).await;
    let events_before: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_events WHERE workflow_instance_id = $1")
            .bind(instance_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let result = m1b_cancel(
        &pool,
        owner_id,
        instance_id,
        state_ver,
        "m1b-t8",
        "m1b-t8",
        "fault injection",
    )
    .await;
    assert!(
        result.is_err(),
        "event insert conflict must fail the cancel"
    );

    let (cancelled, current_visit, version, _, _) = m1b_snapshot(&pool, instance_id).await;
    assert!(!cancelled, "cancelled flag must roll back");
    assert!(current_visit.is_none(), "dangling shape unchanged");
    assert_eq!(version, state_ver, "state version must roll back");
    let total_events: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_events WHERE workflow_instance_id = $1")
            .bind(instance_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        total_events, events_before,
        "event set unchanged: no CANCELLED event persisted beyond the pre-seeded rows"
    );
}
