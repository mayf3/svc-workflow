//! Integration tests for Workflow Runtime commands.
//!
//! Covers WorkflowInstance creation (PR 3A) and WorkflowContext revision (PR 3B).

#![allow(clippy::needless_borrow)]

#[path = "common/mod.rs"]
mod common;

use common::*;
use sqlx::PgPool;
use uuid::Uuid;

use svc_workflow::application::workflow_instance::create::create_workflow_instance;
use svc_workflow::application::workflow_instance::create::CreateWorkflowInstanceResult;
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use svc_workflow::domain::workflow_instance::errors::CreateWorkflowInstanceError;

// ---------------------------------------------------------------------------
// Seed helpers — Instance Create
// ---------------------------------------------------------------------------

/// Seed a published definition with a DRAFT node (WORKFLOW_CREATOR assignee).
pub(crate) async fn seed_published_definition_wf_creator(
    pool: &PgPool,
    domain_id: Uuid,
) -> (Uuid, Uuid) {
    seed_published_def_inner(pool, domain_id, None, "WORKFLOW_CREATOR", None).await
}

/// Seed a published definition with DOMAIN_OWNER assignee on the DRAFT node.
pub(crate) async fn seed_published_definition_domain_owner(
    pool: &PgPool,
    domain_id: Uuid,
) -> (Uuid, Uuid) {
    seed_published_def_inner(pool, domain_id, None, "DOMAIN_OWNER", None).await
}

/// Seed a published definition with FIXED_PRINCIPAL assignee.
pub(crate) async fn seed_published_definition_fixed_principal(
    pool: &PgPool,
    domain_id: Uuid,
    fixed_principal_id: Uuid,
) -> (Uuid, Uuid) {
    seed_published_def_inner(
        pool,
        domain_id,
        Some(fixed_principal_id),
        "FIXED_PRINCIPAL",
        None,
    )
    .await
}

/// Seed a published definition with a non-null context_schema.
pub(crate) async fn seed_published_definition_with_schema(
    pool: &PgPool,
    domain_id: Uuid,
    schema: &serde_json::Value,
) -> (Uuid, Uuid) {
    seed_published_def_inner(pool, domain_id, None, "WORKFLOW_CREATOR", Some(schema)).await
}

/// Core seed helper: create and publish a minimal definition with one DRAFT
/// node, one TERMINAL node, and one ADVANCE transition.
///
/// `context_schema` if provided is stored in the version's context_schema column.
async fn seed_published_def_inner(
    pool: &PgPool,
    domain_id: Uuid,
    fixed_principal_id: Option<Uuid>,
    assignee_type: &str,
    context_schema: Option<&serde_json::Value>,
) -> (Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query("INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Test Def')")
        .bind(def_id).bind(domain_id).bind(&def_key)
        .execute(pool).await.expect("insert def");

    if let Some(schema) = context_schema {
        sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1, $2, 1, 'DRAFT', $3)")
            .bind(ver_id).bind(def_id).bind(schema).execute(pool).await.expect("insert version with schema");
    } else {
        sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1, $2, 1, 'DRAFT', NULL)")
            .bind(ver_id).bind(def_id).execute(pool).await.expect("insert version");
    }

    let draft_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', $3::assignee_ref_type, $4)")
        .bind(draft_id).bind(ver_id).bind(assignee_type).bind(fixed_principal_id)
        .execute(pool).await.expect("insert draft node");
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', 'WORKFLOW_CREATOR')")
        .bind(term_id).bind(ver_id).execute(pool).await.expect("insert terminal node");

    let trans_id = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance', 'Advance', $3, $4, 'ADVANCE')")
        .bind(trans_id).bind(ver_id).bind(draft_id).bind(term_id)
        .execute(pool).await.expect("insert transition");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(trans_id).bind(draft_id).execute(pool).await.expect("set primary");

    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(pool).await.expect("publish version");

    (domain_id, ver_id)
}

/// Seed a published definition with a NORMAL (non-DRAFT) node.
/// Returns (domain_id, definition_version_id, node_id).
pub(crate) async fn seed_published_definition_normal_node(
    pool: &PgPool,
    domain_id: Uuid,
) -> (Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query("INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Test Def')")
        .bind(def_id).bind(domain_id).bind(&def_key)
        .execute(pool).await.expect("insert def");

    sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1, $2, 1, 'DRAFT', NULL)")
        .bind(ver_id).bind(def_id).execute(pool).await.expect("insert version");

    let draft_id = Uuid::new_v4();
    let normal_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();

    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')")
        .bind(draft_id).bind(ver_id).execute(pool).await.expect("insert draft node");
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'review', 'Review', 1, 'NORMAL', 'WORKFLOW_CREATOR')")
        .bind(normal_id).bind(ver_id).execute(pool).await.expect("insert normal node");
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', 'WORKFLOW_CREATOR')")
        .bind(term_id).bind(ver_id).execute(pool).await.expect("insert terminal node");

    // DRAFT → NORMAL
    let trans1_id = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance-draft', 'To Review', $3, $4, 'ADVANCE')")
        .bind(trans1_id).bind(ver_id).bind(draft_id).bind(normal_id)
        .execute(pool).await.expect("insert transition 1");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(trans1_id).bind(draft_id).execute(pool).await.expect("set primary on draft");

    // NORMAL → DONE
    let trans2_id = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance-review', 'To Done', $3, $4, 'ADVANCE')")
        .bind(trans2_id).bind(ver_id).bind(normal_id).bind(term_id)
        .execute(pool).await.expect("insert transition 2");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(trans2_id).bind(normal_id).execute(pool).await.expect("set primary on normal");

    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(pool).await.expect("publish version");

    (domain_id, ver_id, normal_id)
}

/// Seed a published definition with a TERMINAL node that directly follows DRAFT.
/// Returns (domain_id, definition_version_id, terminal_node_id).
pub(crate) async fn seed_published_definition_terminal_only(
    pool: &PgPool,
    domain_id: Uuid,
) -> (Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query("INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Test Def')")
        .bind(def_id).bind(domain_id).bind(&def_key)
        .execute(pool).await.expect("insert def");

    sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1, $2, 1, 'DRAFT', NULL)")
        .bind(ver_id).bind(def_id).execute(pool).await.expect("insert version");

    let draft_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();

    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')")
        .bind(draft_id).bind(ver_id).execute(pool).await.expect("insert draft node");
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', 'WORKFLOW_CREATOR')")
        .bind(term_id).bind(ver_id).execute(pool).await.expect("insert terminal node");

    let trans_id = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance', 'Advance', $3, $4, 'ADVANCE')")
        .bind(trans_id).bind(ver_id).bind(draft_id).bind(term_id)
        .execute(pool).await.expect("insert transition");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(trans_id).bind(draft_id).execute(pool).await.expect("set primary");

    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(pool).await.expect("publish version");

    (domain_id, ver_id, term_id)
}

// ---------------------------------------------------------------------------
// CreateWorkflowInstance helpers
// ---------------------------------------------------------------------------

/// Create a basic CreateWorkflowInstanceCommand.
pub(crate) fn make_command(
    principal_id: Uuid,
    domain_id: Uuid,
    definition_version_id: Uuid,
) -> CreateWorkflowInstanceCommand {
    CreateWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(principal_id),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        domain_id: DomainId::from_uuid(domain_id),
        definition_version_id: DefinitionVersionId::from_uuid(definition_version_id),
        external_reference: None,
        external_url: None,
        metadata: serde_json::json!({"source": "test"}),
        context_payload: serde_json::json!({"hello": "world"}),
    }
}

/// Verify a successful creation result structure.
pub(crate) async fn verify_creation(
    pool: &PgPool,
    result: &CreateWorkflowInstanceResult,
    _principal_id: Uuid,
    _domain_id: Uuid,
    _definition_version_id: Uuid,
) {
    assert_eq!(result.workflow_state_version, 1);
    assert_eq!(result.event_sequence, 1);

    let row: (i32, Uuid, Uuid) = sqlx::query_as(
        "SELECT workflow_state_version, current_context_revision_id, current_node_visit_id FROM workflow_instances WHERE workflow_instance_id = $1",
    ).bind(result.workflow_instance_id).fetch_one(pool).await.expect("instance");
    assert_eq!(row.0, 1);
    assert_eq!(row.1, result.current_context_revision_id);
    assert_eq!(row.2, result.current_node_visit_id);

    let ctx: (i32,) = sqlx::query_as(
        "SELECT revision_number FROM workflow_context_revisions WHERE context_revision_id = $1 AND workflow_instance_id = $2",
    ).bind(result.current_context_revision_id).bind(result.workflow_instance_id)
        .fetch_one(pool).await.expect("context");
    assert_eq!(ctx.0, 1);

    let visit: (i32,) = sqlx::query_as(
        "SELECT visit_number FROM workflow_node_visits WHERE node_visit_id = $1 AND workflow_instance_id = $2",
    ).bind(result.current_node_visit_id).bind(result.workflow_instance_id)
        .fetch_one(pool).await.expect("visit");
    assert_eq!(visit.0, 1);

    let ev: (String, i32, i32) = sqlx::query_as(
        "SELECT event_type, event_sequence, new_workflow_state_version FROM workflow_events WHERE workflow_instance_id = $1 ORDER BY event_sequence",
    ).bind(result.workflow_instance_id).fetch_one(pool).await.expect("event");
    assert_eq!(ev.0, "INSTANCE_CREATED");
    assert_eq!(ev.1, 1);
    assert_eq!(ev.2, 1);

    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM workflow_events WHERE workflow_instance_id = $1")
            .bind(result.workflow_instance_id)
            .fetch_one(pool)
            .await
            .expect("count");
    assert_eq!(count.0, 1);
}

// Sub-modules — Instance Create
#[path = "17_workflow_runtime/instance_create/atomicity.rs"]
mod atomicity;
#[path = "17_workflow_runtime/instance_create/authorization.rs"]
mod authorization;
#[path = "17_workflow_runtime/instance_create/context_validation.rs"]
mod context_validation;
#[path = "17_workflow_runtime/instance_create/definition_gates.rs"]
mod definition_gates;
#[path = "17_workflow_runtime/instance_create/idempotency.rs"]
mod idempotency;
#[path = "17_workflow_runtime/instance_create/normal_create.rs"]
mod normal_create;
#[path = "17_workflow_runtime/instance_create/request_hash_contract.rs"]
mod request_hash_contract;

// Sub-modules — Context Revision (enabled when domain types exist)
// #[path = "17_workflow_runtime/context_revision/success.rs"]
// mod context_revision_success;
// #[path = "17_workflow_runtime/context_revision/authorization.rs"]
// mod context_revision_authorization;
// #[path = "17_workflow_runtime/context_revision/concurrency.rs"]
// mod context_revision_concurrency;
// #[path = "17_workflow_runtime/context_revision/context_validation.rs"]
// mod context_revision_context_validation;
// #[path = "17_workflow_runtime/context_revision/idempotency.rs"]
// mod context_revision_idempotency;
// #[path = "17_workflow_runtime/context_revision/atomicity.rs"]
// mod context_revision_atomicity;
// #[path = "17_workflow_runtime/context_revision/request_hash_contract.rs"]
// mod context_revision_request_hash_contract;
