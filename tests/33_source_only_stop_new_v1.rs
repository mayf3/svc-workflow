//! Source-only stop-new deprecation provenance (CTR-CIR-003 final paragraph,
//! SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2).
//!
//! For exactly the two held versions of parent CTR-WACI-010 the reviewed plan
//! may deprecate without publishing a replacement, "atomically recording
//! SOURCE_IDENTITY_UNRESOLVED and preserving every task/history fact".
//!
//! Coverage (see docs/runbooks/SOURCE_ONLY_STOP_NEW_V1.md):
//!   STOP_NEW_DEPRECATION_RECORDS_REASON_ATOMICALLY
//!       (deprecation + reason in one transaction; reason readable back)
//!   STOP_NEW_CREATION_REFUSED_EXISTING_WORK_CONTINUES
//!       (new creation -> VersionNotPublished 409 rule; an existing instance
//!       still transitions on the DEPRECATED version)
//!   ORDINARY_DEPRECATION_WITHOUT_REASON_WRITES_NULL (no behavior change)
//!   EMPTY_REASON_REJECTED

#[allow(dead_code, unused_imports)]
#[path = "common/mod.rs"]
mod common;

use uuid::Uuid;

use svc_workflow::application::definition::commands::DeprecateVersion;
use svc_workflow::application::definition::{DefinitionService, SOURCE_IDENTITY_UNRESOLVED};
use svc_workflow::domain::definition::error::DefinitionError;
use svc_workflow::domain::ids::{DefinitionVersionId, DomainId, TransitionId, WorkflowInstanceId};
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
};
use svc_workflow::domain::workflow_instance::errors::CreateWorkflowInstanceError;
use svc_workflow::store::postgres::admission_gate::AdmissionGate;
use svc_workflow::store::postgres::definition_repository::PgDefinitionRepository;
use svc_workflow::application::workflow_instance::create::create_workflow_instance;
use svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition;

use serde_json::json;

/// A published legacy graph:
///   draft (WORKFLOW_CREATOR) --advance--> work (FIXED_PRINCIPAL agent)
///        --advance--> done (TERMINAL)
struct StopNewFixture {
    caller: Uuid,
    domain_id: Uuid,
    version_id: Uuid,
    draft_advance: Uuid,
}

async fn seed_published_version(pool: &sqlx::PgPool) -> StopNewFixture {
    let (caller, domain_id) = common::seed_principal_domain_with_owner(pool).await;
    let agent = common::seed_second_principal(pool).await;

    let def_id = Uuid::new_v4();
    let version_id = Uuid::new_v4();
    let def_key = format!("stop-new-{}", &Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) \
         VALUES ($1, $2, $3, 'Stop New Def')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(pool)
    .await
    .expect("insert definition");

    sqlx::query(
        "INSERT INTO workflow_definition_versions \
         (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) \
         VALUES ($1, $2, 1, 'DRAFT', NULL)",
    )
    .bind(version_id)
    .bind(def_id)
    .execute(pool)
    .await
    .expect("insert draft version");

    let draft = Uuid::new_v4();
    let work = Uuid::new_v4();
    let done = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) \
         VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')",
    )
    .bind(draft)
    .bind(version_id)
    .execute(pool)
    .await
    .expect("insert draft node");
    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) \
         VALUES ($1, $2, 'work', 'Work', 1, 'NORMAL', 'FIXED_PRINCIPAL', $3)",
    )
    .bind(work)
    .bind(version_id)
    .bind(agent)
    .execute(pool)
    .await
    .expect("insert work node");
    sqlx::query(
        "INSERT INTO workflow_node_definitions \
         (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) \
         VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)",
    )
    .bind(done)
    .bind(version_id)
    .execute(pool)
    .await
    .expect("insert done node");

    let draft_advance = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions \
         (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) \
         VALUES ($1, $2, 'advance-work', 'Advance', $3, $4, 'ADVANCE')",
    )
    .bind(draft_advance)
    .bind(version_id)
    .bind(draft)
    .bind(work)
    .execute(pool)
    .await
    .expect("insert draft advance");
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2",
    )
    .bind(draft_advance)
    .bind(draft)
    .execute(pool)
    .await
    .expect("set draft primary");

    let work_advance = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions \
         (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) \
         VALUES ($1, $2, 'advance-done', 'Complete', $3, $4, 'ADVANCE')",
    )
    .bind(work_advance)
    .bind(version_id)
    .bind(work)
    .bind(done)
    .execute(pool)
    .await
    .expect("insert work advance");
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2",
    )
    .bind(work_advance)
    .bind(work)
    .execute(pool)
    .await
    .expect("set work primary");

    sqlx::query(
        "UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' \
         WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .execute(pool)
    .await
    .expect("publish version");

    StopNewFixture {
        caller,
        domain_id,
        version_id,
        draft_advance,
    }
}

fn create_command(caller: Uuid, fixture: &StopNewFixture, key: &str) -> CreateWorkflowInstanceCommand {
    CreateWorkflowInstanceCommand {
        principal_id: svc_workflow::domain::ids::PrincipalId::from_uuid(caller),
        idempotency_key: key.to_string(),
        command_schema_version: "v1".to_string(),
        execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
        domain_id: DomainId::from_uuid(fixture.domain_id),
        definition_version_id: DefinitionVersionId::from_uuid(fixture.version_id),
        external_reference: None,
        external_url: None,
        metadata: json!({}),
        context_payload: json!({}),
    }
}

fn service(pool: &sqlx::PgPool) -> DefinitionService<PgDefinitionRepository> {
    DefinitionService::new(PgDefinitionRepository::new(pool.clone()))
}

async fn deprecation_row(
    pool: &sqlx::PgPool,
    version_id: Uuid,
) -> (String, Option<String>) {
    sqlx::query_as(
        "SELECT version_status::TEXT, deprecation_reason \
         FROM workflow_definition_versions WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .fetch_one(pool)
    .await
    .expect("deprecation readback")
}

/// The core source-only stop-new contract: the deprecation records
/// SOURCE_IDENTITY_UNRESOLVED atomically, new-instance creation on the
/// DEPRECATED version is refused by the existing 409 rule, and the existing
/// instance keeps transitioning (its work remains live business).
#[tokio::test]
async fn stop_new_deprecation_refuses_new_creation_but_existing_work_continues() {
    let pool = common::create_pool().await;
    let fixture = seed_published_version(&pool).await;

    // Existing instance created while the version is PUBLISHED.
    let created = create_workflow_instance(
        &pool,
        AdmissionGate::disabled(),
        create_command(fixture.caller, &fixture, "stop-new-existing"),
    )
    .await
    .expect("create on PUBLISHED version");
    assert_eq!(created.workflow_state_version, 1);

    // Governed stop-new deprecation with the SOURCE_IDENTITY_UNRESOLVED
    // reason: one transaction records status flip + provenance.
    let deprecated = service(&pool)
        .deprecate_version(DeprecateVersion {
            actor_principal_id: fixture.caller,
            definition_version_id: fixture.version_id,
            deprecation_reason: Some(SOURCE_IDENTITY_UNRESOLVED.to_string()),
        })
        .await
        .expect("stop-new deprecation");
    assert_eq!(deprecated.version_status.to_string(), "DEPRECATED");

    // Reason recorded (operator provenance is read at the database; see the
    // runbook — it is deliberately not projected through the read API).
    let (status, reason) = deprecation_row(&pool, fixture.version_id).await;
    assert_eq!(status, "DEPRECATED");
    assert_eq!(reason.as_deref(), Some(SOURCE_IDENTITY_UNRESOLVED));

    // NEW intake fails the existing deprecated-version rule (409 family).
    let error = create_workflow_instance(
        &pool,
        AdmissionGate::disabled(),
        create_command(fixture.caller, &fixture, "stop-new-late"),
    )
    .await
    .expect_err("new creation on DEPRECATED version must fail");
    assert!(
        matches!(error, CreateWorkflowInstanceError::VersionNotPublished),
        "expected VersionNotPublished, got {error:?}"
    );

    // EXISTING work continues: the created instance still transitions on the
    // DEPRECATED version (transition validation allows DEPRECATED).
    let advanced = execute_workflow_transition(
        &pool,
        AdmissionGate::disabled(),
        ExecuteWorkflowTransitionCommand {
            principal_id: svc_workflow::domain::ids::PrincipalId::from_uuid(fixture.caller),
            idempotency_key: "stop-new-transition".to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(created.workflow_instance_id),
            expected_workflow_state_version: 1,
            transition_definition_id: TransitionId::from_uuid(fixture.draft_advance),
            submission_payload: None,
        },
    )
    .await
    .expect("existing instance must keep transitioning on a DEPRECATED version");
    assert_eq!(advanced.workflow_state_version, 2);
}

/// A deprecation WITHOUT a reason keeps the pre-0024 behavior exactly:
/// DEPRECATED status, NULL reason. Also proves atomic_deprecate goes through
/// the ordinary publish lifecycle (PUBLISHED -> DEPRECATED only).
#[tokio::test]
async fn ordinary_deprecation_without_reason_writes_null() {
    let pool = common::create_pool().await;
    let fixture = seed_published_version(&pool).await;

    let deprecated = service(&pool)
        .deprecate_version(DeprecateVersion {
            actor_principal_id: fixture.caller,
            definition_version_id: fixture.version_id,
            deprecation_reason: None,
        })
        .await
        .expect("ordinary deprecation");
    assert_eq!(deprecated.version_status.to_string(), "DEPRECATED");

    let (status, reason) = deprecation_row(&pool, fixture.version_id).await;
    assert_eq!(status, "DEPRECATED");
    assert_eq!(reason, None, "no reason supplied -> NULL");
}

/// The recorded reason must be the governed token semantics: an empty or
/// oversized reason is rejected before any write (DB CHECK parity).
#[tokio::test]
async fn invalid_reason_is_rejected_without_writes() {
    let pool = common::create_pool().await;
    let fixture = seed_published_version(&pool).await;

    let error = service(&pool)
        .deprecate_version(DeprecateVersion {
            actor_principal_id: fixture.caller,
            definition_version_id: fixture.version_id,
            deprecation_reason: Some(String::new()),
        })
        .await
        .expect_err("empty reason must be rejected");
    assert!(
        matches!(error, DefinitionError::SchemaValidationFailed(_)),
        "expected SchemaValidationFailed, got {error:?}"
    );

    // A version >256 chars is rejected as well.
    let oversized = "x".repeat(257);
    let error = service(&pool)
        .deprecate_version(DeprecateVersion {
            actor_principal_id: fixture.caller,
            definition_version_id: fixture.version_id,
            deprecation_reason: Some(oversized),
        })
        .await
        .expect_err("oversized reason must be rejected");
    assert!(matches!(error, DefinitionError::SchemaValidationFailed(_)));

    // Zero writes: the version is still PUBLISHED with no reason.
    let (status, reason) = deprecation_row(&pool, fixture.version_id).await;
    assert_eq!(status, "PUBLISHED");
    assert_eq!(reason, None);

}
