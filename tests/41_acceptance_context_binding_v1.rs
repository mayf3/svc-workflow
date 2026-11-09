//! Integration tests for SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1
//! (CTR-ACB-001..006).
//!
//! Black-box coverage through the real application services and HTTP router:
//! - ACC-ACB-001 / CTR-001: publication/validate annotation matrix (opt-in
//!   placement, exact value, nested/malformed rejection, digest binding).
//! - ACC-ACB-002 / CTR-002: Create-time acceptanceCriteria catalog gate
//!   (zero workflow-fact delta, deterministic failure receipt, no metadata
//!   substitution).
//! - ACC-ACB-003 / CTR-003: successful-ADVANCE binding predicate (exact
//!   current Context UUID, exact ID set, literal PASS, cross-Instance
//!   isolation, structural-pass/QA-truth boundary).
//! - ACC-ACB-004 / CTR-004: combined fail-closed, RETURN/TERMINATE remain
//!   usable without all-PASS, model3 combined still rejected.
//! - ACC-ACB-005 / CTR-005: CAS, same-key success/failure replay, changed
//!   payload conflict — zero business delta on every rejection.
//! - ACC-ACB-006 / CTR-006: unannotated compatibility.
//!
//! Requires TEST_DATABASE_URL pointing at an isolated PostgreSQL 16 test
//! database. Run with `--test-threads=1` (fact-count deltas are global).
//!
//! NOTE (evidence discipline): publish-path assertions go through the real
//! DefinitionService::publish_version or the HTTP router. Draft graphs are
//! seeded as DRAFT rows via SQL fixtures before invoking the real publish
//! service (authoring fixture, not a publish bypass). Post-publish drift
//! fixtures are limited to the Spec-sanctioned ACC-ACB-004 combined
//! definition and the F1 drifted instances, which the independent review's
//! counterexamples themselves prescribe.

#![allow(clippy::needless_borrow)]
#![allow(unused_imports, unused_variables)]

#[path = "common/mod.rs"]
mod common;

use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use svc_workflow::application::definition::commands::{PublishVersion, ValidateDraftVersion};
use svc_workflow::application::definition::repository::DefinitionRepository;
use svc_workflow::application::definition::DefinitionService;
use svc_workflow::application::workflow_instance::revise_and_transition::revise_context_and_transition;
use svc_workflow::domain::definition::error::DefinitionError;
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::combined_errors::ReviseContextAndTransitionError;
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
    ReviseContextAndTransitionCommand,
};
use svc_workflow::domain::workflow_instance::errors::{
    CreateWorkflowInstanceError, ExecuteWorkflowTransitionError,
};
use svc_workflow::store::postgres::admission_gate::AdmissionGate;
use svc_workflow::store::postgres::definition_repository::PgDefinitionRepository;

// ============================================================================
// Constants (mirrors of the accepted contract's fixed literals)
// ============================================================================

const ANNOTATION_KEY: &str = "x-workflow-acceptance-binding-v1";
const CATALOG_POINTER: &str = "/acceptanceCriteria";
const REPORT_POINTER: &str = "/acceptanceReport";

// ============================================================================
// Fixture helpers
// ============================================================================

fn annotation_value() -> Value {
    json!({
        "contextCatalogPointer": CATALOG_POINTER,
        "submissionReportPointer": REPORT_POINTER,
    })
}

/// Submission schema carrying the root opt-in annotation on a model3
/// primary ADVANCE. The schema itself mirrors the acceptance report shape.
fn annotated_submission_schema() -> Value {
    let mut schema = plain_acceptance_report_schema();
    schema[ANNOTATION_KEY] = annotation_value();
    schema
}

fn plain_acceptance_report_schema() -> Value {
    json!({
        "type": "object",
        "required": ["acceptanceReport"],
        "properties": {
            "acceptanceReport": {
                "type": "object",
                "required": ["contextRevisionId", "results"],
                "properties": {
                    "contextRevisionId": {"type": "string"},
                    "results": {"type": "array"},
                },
            },
        },
    })
}

fn catalog_context(ids: &[&str]) -> Value {
    json!({
        "title": "acb-fixture",
        "acceptanceCriteria": ids
            .iter()
            .map(|id| json!({"id": id, "description": format!("criterion {id}")}))
            .collect::<Vec<_>>(),
    })
}

fn acceptance_report(context_revision_id: Uuid, entries: Value) -> Value {
    json!({
        "acceptanceReport": {
            "contextRevisionId": context_revision_id.to_string(),
            "results": entries,
        }
    })
}

fn pass_entries(ids: &[&str]) -> Value {
    Value::Array(
        ids.iter()
            .map(|id| json!({"id": id, "verdict": "PASS"}))
            .collect(),
    )
}

struct Model3Fixture {
    version_id: Uuid,
    advance_start_work: Uuid,
    /// The primary ADVANCE work -> done that carries the annotation.
    advance_work_done: Uuid,
    return_work_start: Uuid,
    terminate_work_failed: Uuid,
}

/// Seed a model3 DRAFT graph:
/// start(TASK, WORKFLOW_CREATOR) -ADVANCE-> work(TASK, FIXED agent)
/// -ADVANCE-> done(TERMINAL); RETURN work->start; TERMINATE work->failed.
/// `work_done_schema` replaces the work->done transition's submission_schema.
/// The version stays DRAFT — publish through the real service.
async fn seed_model3_draft(
    pool: &PgPool,
    domain_id: Uuid,
    agent_id: Uuid,
    work_done_schema: Option<Value>,
) -> Model3Fixture {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("acb-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'ACB Def')",
    )
    .bind(def_id).bind(domain_id).bind(&def_key)
    .execute(pool).await.expect("insert definition");

    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, semantic_model_version) VALUES ($1, $2, 1, 'DRAFT', 3)",
    )
    .bind(ver_id).bind(def_id)
    .execute(pool).await.expect("insert version");

    let start_id = Uuid::new_v4();
    let work_id = Uuid::new_v4();
    let done_id = Uuid::new_v4();
    let failed_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'start', 'Start', 0, 'TASK', 'WORKFLOW_CREATOR')",
    ).bind(start_id).bind(ver_id).execute(pool).await.expect("insert start");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) VALUES ($1, $2, 'work', 'Work', 1, 'TASK', 'FIXED_PRINCIPAL', $3)",
    ).bind(work_id).bind(ver_id).bind(agent_id).execute(pool).await.expect("insert work");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)",
    ).bind(done_id).bind(ver_id).execute(pool).await.expect("insert done");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'failed', 'Failed', 3, 'TERMINAL', NULL)",
    ).bind(failed_id).bind(ver_id).execute(pool).await.expect("insert failed");

    let t1 = Uuid::new_v4();
    let t2 = Uuid::new_v4();
    let t3 = Uuid::new_v4();
    let t4 = Uuid::new_v4();
    for (tid, key, src, dst, effect, schema) in [
        (t1, "advance-1", start_id, work_id, "ADVANCE", None),
        (t2, "advance-2", work_id, done_id, "ADVANCE", work_done_schema),
        (t3, "return", work_id, start_id, "RETURN", None),
        (t4, "terminate", work_id, failed_id, "TERMINATE", None),
    ] {
        sqlx::query(
            "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema) VALUES ($1, $2, $3, $3, $4, $5, $6::transition_effect, $7)",
        )
        .bind(tid).bind(ver_id).bind(key).bind(src).bind(dst).bind(effect).bind(schema)
        .execute(pool).await.expect("insert transition");
    }
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t1).bind(start_id).execute(pool).await.expect("primary start");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t2).bind(work_id).execute(pool).await.expect("primary work");

    Model3Fixture {
        version_id: ver_id,
        advance_start_work: t1,
        advance_work_done: t2,
        return_work_start: t3,
        terminate_work_failed: t4,
    }
}

/// Publish a DRAFT version through the real application service with an
/// enabled DOMAIN_OWNER actor and a dormant admission gate.
async fn publish_draft(
    pool: &PgPool,
    actor: Uuid,
    version_id: Uuid,
) -> Result<(), DefinitionError> {
    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);
    service
        .publish_version(
            PublishVersion {
                actor_principal_id: actor,
                definition_version_id: version_id,
                expected_revision: None,
            },
            crate::common::admitting_gate(),
        )
        .await
        .map(|_| ())
}

async fn validate_draft(
    pool: &PgPool,
    actor: Uuid,
    version_id: Uuid,
) -> Result<Vec<String>, Vec<String>> {
    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);
    let result = service
        .validate_draft_version(ValidateDraftVersion {
            actor_principal_id: actor,
            definition_version_id: version_id,
        })
        .await
        .expect("validate_draft_version service call");
    if result.valid {
        Ok(result.errors.into_iter().map(|e| e.code).collect())
    } else {
        Err(result.errors.into_iter().map(|e| e.code).collect())
    }
}

fn create_command(
    creator: Uuid,
    domain_id: Uuid,
    version_id: Uuid,
    context_payload: Value,
    metadata: Value,
) -> CreateWorkflowInstanceCommand {
    CreateWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(creator),
        idempotency_key: format!("create-{}", Uuid::new_v4()),
        command_schema_version: "v1".to_string(),
        execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
        domain_id: DomainId::from_uuid(domain_id),
        definition_version_id: DefinitionVersionId::from_uuid(version_id),
        external_reference: None,
        external_url: None,
        metadata,
        context_payload,
    }
}

async fn create_instance(
    pool: &PgPool,
    creator: Uuid,
    domain_id: Uuid,
    version_id: Uuid,
    catalog_ids: &[&str],
) -> (Uuid, Uuid, i32) {
    let result = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        pool,
        crate::common::admitting_gate(),
        create_command(
            creator,
            domain_id,
            version_id,
            catalog_context(catalog_ids),
            json!({}),
        ),
    )
    .await
    .expect("create should succeed");
    (
        result.workflow_instance_id,
        result.current_context_revision_id,
        result.workflow_state_version,
    )
}

fn transition_command(
    actor: Uuid,
    instance_id: Uuid,
    expected_version: i32,
    transition_id: Uuid,
    submission_payload: Option<Value>,
) -> ExecuteWorkflowTransitionCommand {
    ExecuteWorkflowTransitionCommand {
        principal_id: PrincipalId::from_uuid(actor),
        idempotency_key: format!("trans-{}", Uuid::new_v4()),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
        expected_workflow_state_version: expected_version,
        transition_definition_id: TransitionId::from_uuid(transition_id),
        submission_payload,
    }
}

#[allow(clippy::type_complexity)]
async fn fact_counts(pool: &PgPool) -> (i64, i64, i64, i64, i64, i64) {
    let one = |n: (i64,)| n.0;
    let instances = one(
        sqlx::query_as("SELECT count(*) FROM workflow_instances")
            .fetch_one(pool)
            .await
            .unwrap(),
    );
    let visits = one(
        sqlx::query_as("SELECT count(*) FROM workflow_node_visits")
            .fetch_one(pool)
            .await
            .unwrap(),
    );
    let submissions = one(
        sqlx::query_as("SELECT count(*) FROM workflow_submissions")
            .fetch_one(pool)
            .await
            .unwrap(),
    );
    let events = one(
        sqlx::query_as("SELECT count(*) FROM workflow_events")
            .fetch_one(pool)
            .await
            .unwrap(),
    );
    let contexts = one(
        sqlx::query_as("SELECT count(*) FROM workflow_context_revisions")
            .fetch_one(pool)
            .await
            .unwrap(),
    );
    let receipts = one(
        sqlx::query_as("SELECT count(*) FROM workflow_command_receipts")
            .fetch_one(pool)
            .await
            .unwrap(),
    );
    (instances, visits, submissions, events, contexts, receipts)
}

/// Standard two-instance rig: annotated published definition; instance A and
/// instance B share the same catalog IDs but hold different Context UUIDs.
/// Returns (domain, fixture, owner, agent, [instanceA, ctxA, verA], [instanceB, ctxB, verB]).
#[allow(clippy::type_complexity)]
async fn two_instance_rig(
    pool: &PgPool,
) -> (
    Uuid,
    Model3Fixture,
    Uuid,
    Uuid,
    (Uuid, Uuid, i32),
    (Uuid, Uuid, i32),
) {
    let (owner, domain) = common::seed_principal_domain_with_owner(pool).await;
    let agent = common::seed_second_principal(pool).await;
    let fx = seed_model3_draft(pool, domain, agent, Some(annotated_submission_schema())).await;
    publish_draft(pool, owner, fx.version_id)
        .await
        .expect("annotated model3 primary ADVANCE must publish");

    let a = create_instance(pool, owner, domain, fx.version_id, &["C1", "C2"]).await;
    let b = create_instance(pool, owner, domain, fx.version_id, &["C1", "C2"]).await;
    (domain, fx, owner, agent, a, b)
}

/// Advance instance to the `work` node (t1 from start, no schema).
async fn advance_to_work(pool: &PgPool, actor: Uuid, instance: &(Uuid, Uuid, i32), t1: Uuid) -> i32 {
    let result =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            pool,
            crate::common::admitting_gate(),
            transition_command(actor, instance.0, instance.2, t1, None),
        )
        .await
        .expect("advance start->work should succeed");
    result.workflow_state_version
}

// ============================================================================
// ACC-ACB-001 / CTR-ACB-001 — publication and validation matrix
// ============================================================================

async fn publish_matrix_case(
    pool: &PgPool,
    schema: Option<Value>,
    semantic_model: i16,
    graph_shape: GraphShape,
) -> Result<(), DefinitionError> {
    let (owner, domain) = common::seed_principal_domain_with_owner(pool).await;
    let agent = common::seed_second_principal(pool).await;
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'ACB Matrix')",
    )
    .bind(def_id).bind(domain).bind(format!("acb-m-{}", &Uuid::new_v4().to_string()[..8]))
    .execute(pool).await.expect("insert definition");
    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, semantic_model_version) VALUES ($1, $2, 1, 'DRAFT', $3)",
    )
    .bind(ver_id).bind(def_id).bind(semantic_model)
    .execute(pool).await.expect("insert version");

    match graph_shape {
        GraphShape::Model3 => {
            // same shape as seed_model3_draft
            let start_id = Uuid::new_v4();
            let work_id = Uuid::new_v4();
            let done_id = Uuid::new_v4();
            let failed_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'start', 'Start', 0, 'TASK', 'WORKFLOW_CREATOR')",
            ).bind(start_id).bind(ver_id).execute(pool).await.unwrap();
            sqlx::query(
                "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) VALUES ($1, $2, 'work', 'Work', 1, 'TASK', 'FIXED_PRINCIPAL', $3)",
            ).bind(work_id).bind(ver_id).bind(agent).execute(pool).await.unwrap();
            sqlx::query(
                "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)",
            ).bind(done_id).bind(ver_id).execute(pool).await.unwrap();
            sqlx::query(
                "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'failed', 'Failed', 3, 'TERMINAL', NULL)",
            ).bind(failed_id).bind(ver_id).execute(pool).await.unwrap();
            let t1 = Uuid::new_v4();
            let t2 = Uuid::new_v4();
            let t3 = Uuid::new_v4();
            let t4 = Uuid::new_v4();
            for (tid, key, src, dst, effect, sch) in [
                (t1, "advance-1", start_id, work_id, "ADVANCE", None),
                (t2, "advance-2", work_id, done_id, "ADVANCE", schema),
                (t3, "return", work_id, start_id, "RETURN", None),
                (t4, "terminate", work_id, failed_id, "TERMINATE", None),
            ] {
                sqlx::query(
                    "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema) VALUES ($1, $2, $3, $3, $4, $5, $6::transition_effect, $7)",
                ).bind(tid).bind(ver_id).bind(key).bind(src).bind(dst).bind(effect).bind(sch)
                .execute(pool).await.unwrap();
            }
            sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
                .bind(t1).bind(start_id).execute(pool).await.unwrap();
            sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
                .bind(t2).bind(work_id).execute(pool).await.unwrap();
        }
        GraphShape::LegacyDraftEntry => {
            let draft_id = Uuid::new_v4();
            let term_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')",
            ).bind(draft_id).bind(ver_id).execute(pool).await.unwrap();
            sqlx::query(
                "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', NULL)",
            ).bind(term_id).bind(ver_id).execute(pool).await.unwrap();
            let t1 = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema) VALUES ($1, $2, 'advance', 'advance', $3, $4, 'ADVANCE', $5)",
            ).bind(t1).bind(ver_id).bind(draft_id).bind(term_id).bind(schema)
            .execute(pool).await.unwrap();
            sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
                .bind(t1).bind(draft_id).execute(pool).await.unwrap();
        }
        GraphShape::Minimal => {
            let entry_id = Uuid::new_v4();
            let term_id = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'entry', 'Entry', 0, 'NORMAL', 'WORKFLOW_CREATOR')",
            ).bind(entry_id).bind(ver_id).execute(pool).await.unwrap();
            sqlx::query(
                "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', NULL)",
            ).bind(term_id).bind(ver_id).execute(pool).await.unwrap();
            let t1 = Uuid::new_v4();
            sqlx::query(
                "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema) VALUES ($1, $2, 'advance', 'advance', $3, $4, 'ADVANCE', $5)",
            ).bind(t1).bind(ver_id).bind(entry_id).bind(term_id).bind(schema)
            .execute(pool).await.unwrap();
            sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
                .bind(t1).bind(entry_id).execute(pool).await.unwrap();
        }
    }

    publish_draft(pool, owner, ver_id).await
}

enum GraphShape {
    Model3,
    LegacyDraftEntry,
    Minimal,
}

#[tokio::test]
async fn acc1_valid_root_annotation_on_model3_primary_advance_publishes() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;
    let fx = seed_model3_draft(&pool, domain, agent, Some(annotated_submission_schema())).await;

    publish_draft(&pool, owner, fx.version_id)
        .await
        .expect("valid root annotation on model3 primary ADVANCE must publish");

    let repo = PgDefinitionRepository::new(pool.clone());
    let version = repo
        .get_version(fx.version_id)
        .await
        .unwrap();
    assert_eq!(version.version_status.to_string(), "PUBLISHED");
    let digest = version.definition_digest.expect("published digest recorded");
    assert!(!digest.is_empty());
}

#[tokio::test]
async fn acc1_annotation_is_bound_into_canonical_digest() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;

    let fx_with =
        seed_model3_draft(&pool, domain, agent, Some(annotated_submission_schema())).await;
    let fx_without = seed_model3_draft(&pool, domain, agent, Some(plain_acceptance_report_schema()))
        .await;
    publish_draft(&pool, owner, fx_with.version_id).await.unwrap();
    publish_draft(&pool, owner, fx_without.version_id)
        .await
        .unwrap();

    let repo = PgDefinitionRepository::new(pool.clone());
    let d_with = repo
        .get_version(fx_with.version_id)
        .await
        .unwrap()
        .definition_digest
        .unwrap();
    let d_without = repo
        .get_version(fx_without.version_id)
        .await
        .unwrap()
        .definition_digest
        .unwrap();
    assert_ne!(
        d_with, d_without,
        "the annotation is part of the canonical digest"
    );
}

#[tokio::test]
async fn acc1_publish_rejects_changed_pointer_value() {
    let pool = common::create_pool().await;
    let mut schema = annotated_submission_schema();
    schema[ANNOTATION_KEY]["submissionReportPointer"] = json!("/acceptanceReportV2");
    let err = publish_matrix_case(&pool, Some(schema), 3, GraphShape::Model3)
        .await
        .expect_err("changed pointer value must fail publication");
    assert!(
        graph_errors_contain(&err, "ACCEPTANCE_BINDING"),
        "{err:?}"
    );
}

#[tokio::test]
async fn acc1_publish_rejects_extra_annotation_member() {
    let pool = common::create_pool().await;
    let mut schema = annotated_submission_schema();
    schema[ANNOTATION_KEY]["extraMember"] = json!("nope");
    let err = publish_matrix_case(&pool, Some(schema), 3, GraphShape::Model3)
        .await
        .expect_err("extra annotation member must fail publication");
    assert!(graph_errors_contain(&err, "ACCEPTANCE_BINDING"), "{err:?}");
}

#[tokio::test]
async fn acc1_publish_rejects_missing_annotation_member() {
    let pool = common::create_pool().await;
    let mut schema = annotated_submission_schema();
    schema[ANNOTATION_KEY]
        .as_object_mut()
        .unwrap()
        .remove("contextCatalogPointer");
    let err = publish_matrix_case(&pool, Some(schema), 3, GraphShape::Model3)
        .await
        .expect_err("missing annotation member must fail publication");
    assert!(graph_errors_contain(&err, "ACCEPTANCE_BINDING"), "{err:?}");
}

#[tokio::test]
async fn acc1_publish_rejects_nested_annotation_occurrence() {
    let pool = common::create_pool().await;
    let mut schema = annotated_submission_schema();
    // Valid root value, but the key also occurs nested inside a property.
    schema["properties"]["acceptanceReport"]["properties"][ANNOTATION_KEY] = annotation_value();
    let err = publish_matrix_case(&pool, Some(schema), 3, GraphShape::Model3)
        .await
        .expect_err("nested occurrence must fail publication");
    assert!(graph_errors_contain(&err, "ACCEPTANCE_BINDING"), "{err:?}");
}

#[tokio::test]
async fn acc1_publish_rejects_nested_only_annotation() {
    let pool = common::create_pool().await;
    let mut schema = plain_acceptance_report_schema();
    schema["properties"][ANNOTATION_KEY] = annotation_value();
    let err = publish_matrix_case(&pool, Some(schema), 3, GraphShape::Model3)
        .await
        .expect_err("nested-only occurrence must fail publication (never absence)");
    assert!(graph_errors_contain(&err, "ACCEPTANCE_BINDING"), "{err:?}");
}

#[tokio::test]
async fn acc1_publish_rejects_malformed_annotation_values() {
    for bad in [
        json!(null),
        json!(true),
        json!("yes"),
        json!([]),
        json!({"contextCatalogPointer": 1, "submissionReportPointer": "/acceptanceReport"}),
    ] {
        let pool = common::create_pool().await;
        let mut schema = plain_acceptance_report_schema();
        schema[ANNOTATION_KEY] = bad.clone();
        let err = publish_matrix_case(&pool, Some(schema), 3, GraphShape::Model3)
            .await
            .expect_err(&format!("malformed annotation {bad} must fail publication"));
        assert!(
            graph_errors_contain(&err, "ACCEPTANCE_BINDING"),
            "malformed {bad}: {err:?}"
        );
    }
}

#[tokio::test]
async fn acc1_publish_rejects_annotation_on_return_and_terminate() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;
    let fx = seed_model3_draft(&pool, domain, agent, None).await;
    // Move the annotation onto the RETURN transition (drift authoring on DRAFT).
    sqlx::query("UPDATE workflow_transition_definitions SET submission_schema = $1 WHERE transition_id = $2")
        .bind(annotated_submission_schema())
        .bind(fx.return_work_start)
        .execute(&pool)
        .await
        .unwrap();
    let err = publish_draft(&pool, owner, fx.version_id)
        .await
        .expect_err("RETURN opt-in must fail publication");
    assert!(graph_errors_contain(&err, "ACCEPTANCE_BINDING"), "{err:?}");

    // TERMINATE
    let pool2 = common::create_pool().await;
    let (owner2, domain2) = common::seed_principal_domain_with_owner(&pool2).await;
    let agent2 = common::seed_second_principal(&pool2).await;
    let fx2 = seed_model3_draft(&pool2, domain2, agent2, None).await;
    sqlx::query("UPDATE workflow_transition_definitions SET submission_schema = $1 WHERE transition_id = $2")
        .bind(annotated_submission_schema())
        .bind(fx2.terminate_work_failed)
        .execute(&pool2)
        .await
        .unwrap();
    let err2 = publish_draft(&pool2, owner2, fx2.version_id)
        .await
        .expect_err("TERMINATE opt-in must fail publication");
    assert!(graph_errors_contain(&err2, "ACCEPTANCE_BINDING"), "{err2:?}");
}

#[tokio::test]
async fn acc1_publish_rejects_annotation_on_legacy_draft_entry() {
    let pool = common::create_pool().await;
    let err = publish_matrix_case(
        &pool,
        Some(annotated_submission_schema()),
        1,
        GraphShape::LegacyDraftEntry,
    )
    .await
    .expect_err("Legacy (DRAFT source node kind) opt-in must fail publication");
    assert!(graph_errors_contain(&err, "ACCEPTANCE_BINDING"), "{err:?}");
}

#[tokio::test]
async fn acc1_publish_rejects_annotation_on_minimal_model() {
    let pool = common::create_pool().await;
    let err = publish_matrix_case(
        &pool,
        Some(annotated_submission_schema()),
        2,
        GraphShape::Minimal,
    )
    .await
    .expect_err("model2 opt-in must fail publication");
    assert!(graph_errors_contain(&err, "ACCEPTANCE_BINDING"), "{err:?}");
}

#[tokio::test]
async fn acc1_validate_draft_reports_binding_errors_like_publish() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;
    // RETURN-carried annotation: validate_draft must flag it too.
    let fx = seed_model3_draft(&pool, domain, agent, None).await;
    sqlx::query("UPDATE workflow_transition_definitions SET submission_schema = $1 WHERE transition_id = $2")
        .bind(annotated_submission_schema())
        .bind(fx.return_work_start)
        .execute(&pool)
        .await
        .unwrap();
    let codes = validate_draft(&pool, owner, fx.version_id)
        .await
        .expect_err("validate_draft_version must report the binding violation");
    assert!(
        codes.iter().any(|c| c.starts_with("ACCEPTANCE_BINDING")),
        "{codes:?}"
    );

    // Valid placement: no binding-specific findings (the draft validator's
    // pre-existing legacy-model findings on a model3 graph are out of scope).
    let pool2 = common::create_pool().await;
    let (owner2, domain2) = common::seed_principal_domain_with_owner(&pool2).await;
    let agent2 = common::seed_second_principal(&pool2).await;
    let fx2 = seed_model3_draft(&pool2, domain2, agent2, Some(annotated_submission_schema())).await;
    let codes2 = match validate_draft(&pool2, owner2, fx2.version_id).await {
        Ok(codes) => codes,
        Err(codes) => codes,
    };
    assert!(
        !codes2.iter().any(|c| c.starts_with("ACCEPTANCE_BINDING")),
        "{codes2:?}"
    );
}

// ============================================================================
// F2 (review ACB-SOURCE-SNAPSHOT-A1-INDEPENDENT-REVIEW) — the reserved
// annotation is rejected anywhere in the version's context schema; the sole
// legal location remains the root of a model3 TASK primary ADVANCE
// submission schema.
// ============================================================================

#[tokio::test]
async fn f2_publish_rejects_annotation_in_context_schema() {
    for (name, context_schema) in [
        (
            "root valid value",
            json!({"type": "object", ANNOTATION_KEY: annotation_value()}),
        ),
        (
            "nested null value",
            json!({"type": "object", "$defs": {"payload": {ANNOTATION_KEY: null}}}),
        ),
        (
            "nested inside array",
            json!({"anyOf": [true, {ANNOTATION_KEY: annotation_value()}]}),
        ),
    ] {
        let pool = common::create_pool().await;
        let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
        let agent = common::seed_second_principal(&pool).await;
        let fx = seed_model3_draft(&pool, domain, agent, None).await;
        sqlx::query(
            "UPDATE workflow_definition_versions SET context_schema = $1 \
             WHERE definition_version_id = $2",
        )
        .bind(context_schema)
        .bind(fx.version_id)
        .execute(&pool)
        .await
        .unwrap();
        let err = publish_draft(&pool, owner, fx.version_id)
            .await
            .err()
            .unwrap_or_else(|| panic!("context schema annotation ({name}) must fail publication"));
        assert!(
            graph_errors_contain(&err, "ACCEPTANCE_BINDING"),
            "{name}: {err:?}"
        );
        // Fail closed: version stays DRAFT.
        let (st,): (String,) = sqlx::query_as(
            "SELECT version_status::text FROM workflow_definition_versions \
             WHERE definition_version_id = $1",
        )
        .bind(fx.version_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(st, "DRAFT", "{name}");
    }
}

#[tokio::test]
async fn f2_validate_draft_flags_context_schema_annotation() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;
    let fx = seed_model3_draft(&pool, domain, agent, None).await;
    sqlx::query(
        "UPDATE workflow_definition_versions SET context_schema = $1 \
         WHERE definition_version_id = $2",
    )
    .bind(json!({"type": "object", ANNOTATION_KEY: annotation_value()}))
    .bind(fx.version_id)
    .execute(&pool)
    .await
    .unwrap();
    let codes = match validate_draft(&pool, owner, fx.version_id).await {
        Ok(codes) => codes,
        Err(codes) => codes,
    };
    assert!(
        codes.iter().any(|c| c.starts_with("ACCEPTANCE_BINDING")),
        "{codes:?}"
    );
}

fn graph_errors_contain(err: &DefinitionError, needle: &str) -> bool {
    match err {
        DefinitionError::GraphValidationFailed(errors) => errors
            .iter()
            .any(|e| e.code.starts_with(needle) || e.message.contains(needle)),
        other => panic!("expected GraphValidationFailed, got {other:?}"),
    }
}

// ============================================================================
// ACC-ACB-002 / CTR-ACB-002 — Create catalog gate
// ============================================================================

#[tokio::test]
async fn acc2_valid_catalog_creates_instance_with_immutable_context() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, _agent, _a, _b) = two_instance_rig(&pool).await;
    let (instance, ctx, _ver) =
        create_instance(&pool, owner, _domain, fx.version_id, &["OK1"]).await;
    let stored: (Value,) = sqlx::query_as(
        "SELECT payload FROM workflow_context_revisions WHERE context_revision_id = $1",
    )
    .bind(ctx)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        stored.0["acceptanceCriteria"].as_array().unwrap().len(),
        1,
        "formal Context stores the catalog"
    );
    let _ = instance;
}

#[tokio::test]
async fn acc2_missing_catalog_rejected_zero_delta_with_failure_receipt() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;
    let fx = seed_model3_draft(&pool, domain, agent, Some(annotated_submission_schema())).await;
    publish_draft(&pool, owner, fx.version_id).await.unwrap();

    let before = fact_counts(&pool).await;
    let err = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool,
        crate::common::admitting_gate(),
        create_command(owner, domain, fx.version_id, json!({"title": "no catalog"}), json!({})),
    )
    .await
    .expect_err("missing catalog must fail creation");
    assert!(
        matches!(err, CreateWorkflowInstanceError::ContextValidationFailed(_)),
        "{err:?}"
    );
    let after = fact_counts(&pool).await;
    assert_eq!(before.0, after.0, "zero instance delta");
    assert_eq!(before.1, after.1, "zero visit delta");
    assert_eq!(before.2, after.2, "zero submission delta");
    assert_eq!(before.3, after.3, "zero event delta");
    assert_eq!(before.4, after.4, "zero context delta");
    assert_eq!(
        after.5,
        before.5 + 1,
        "deterministic failure receipt persists"
    );
    let receipt: (i32, Value) = sqlx::query_as(
        "SELECT response_status, response_body FROM workflow_command_receipts \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(receipt.0, 422);
    assert_eq!(receipt.1["error"], "context_validation_failed");
}

#[tokio::test]
async fn acc2_create_rejects_malformed_catalog_shapes() {
    struct Case {
        name: &'static str,
        catalog: Value,
    }
    let cases = vec![
        Case {
            name: "empty array",
            catalog: json!({"acceptanceCriteria": []}),
        },
        Case {
            name: "duplicate ids",
            catalog: json!({"acceptanceCriteria": [
                {"id": "C1", "description": "a"},
                {"id": "C1", "description": "b"}
            ]}),
        },
        Case {
            name: "empty id",
            catalog: json!({"acceptanceCriteria": [{"id": "", "description": "a"}]}),
        },
        Case {
            name: "id starting with dash",
            catalog: json!({"acceptanceCriteria": [{"id": "-bad", "description": "a"}]}),
        },
        Case {
            name: "id with forbidden char",
            catalog: json!({"acceptanceCriteria": [{"id": "bad id!", "description": "a"}]}),
        },
        Case {
            name: "id over 64 chars",
            catalog: json!({"acceptanceCriteria": [
                {"id": "A12345678901234567890123456789012345678901234567890123456789012345", "description": "a"}
            ]}),
        },
        Case {
            name: "blank description",
            catalog: json!({"acceptanceCriteria": [{"id": "C1", "description": "   "},]}),
        },
        Case {
            name: "missing description",
            catalog: json!({"acceptanceCriteria": [{"id": "C1"}]}),
        },
        Case {
            name: "oversize description (4097 bytes)",
            catalog: json!({"acceptanceCriteria": [
                {"id": "C1", "description": "x".repeat(4097)}
            ]}),
        },
        Case {
            name: "extra catalog member",
            catalog: json!({"acceptanceCriteria": [
                {"id": "C1", "description": "a", "severity": "high"}
            ]}),
        },
        Case {
            name: "non-object entry",
            catalog: json!({"acceptanceCriteria": ["C1"]}),
        },
        Case {
            name: "catalog not an array",
            catalog: json!({"acceptanceCriteria": {"id": "C1", "description": "a"}}),
        },
        Case {
            name: "129 entries",
            catalog: json!({"acceptanceCriteria": (0..129)
                .map(|i| json!({"id": format!("C{i}"), "description": "d"}))
                .collect::<Vec<_>>()}),
        },
    ];

    for case in cases {
        let pool = common::create_pool().await;
        let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
        let agent = common::seed_second_principal(&pool).await;
        let fx =
            seed_model3_draft(&pool, domain, agent, Some(annotated_submission_schema())).await;
        publish_draft(&pool, owner, fx.version_id).await.unwrap();

        let mut context = json!({"title": "t"});
        context["acceptanceCriteria"] = case.catalog["acceptanceCriteria"].clone();
        let err =
            svc_workflow::application::workflow_instance::create::create_workflow_instance(
                &pool,
                crate::common::admitting_gate(),
                create_command(owner, domain, fx.version_id, context, json!({})),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("case '{}' must reject", case.name));
        assert!(
            matches!(err, CreateWorkflowInstanceError::ContextValidationFailed(_)),
            "case '{}': {err:?}",
            case.name
        );
        let instances: (i64,) =
            sqlx::query_as("SELECT count(*) FROM workflow_instances WHERE definition_version_id = $1")
                .bind(fx.version_id)
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(instances.0, 0, "case '{}': zero instance delta", case.name);

        // Boundary: 4096-byte description and 64-char id are legal; C1 and c1
        // are distinct case-sensitive values (uniqueness is exact-match).
        if case.name == "blank description" {
            let ok_ctx = json!({
                "acceptanceCriteria": [
                    {"id": "A123456789012345678901234567890123456789012345678901234567890123",
                     "description": "x".repeat(4096)}
                ]
            });
            svc_workflow::application::workflow_instance::create::create_workflow_instance(
                &pool,
                crate::common::admitting_gate(),
                create_command(owner, domain, fx.version_id, ok_ctx, json!({})),
            )
            .await
            .expect("64-char id and 4096-byte description are inside the limits");

            let case_sensitive = json!({
                "acceptanceCriteria": [
                    {"id": "C1", "description": "upper"},
                    {"id": "c1", "description": "lower"}
                ]
            });
            svc_workflow::application::workflow_instance::create::create_workflow_instance(
                &pool,
                crate::common::admitting_gate(),
                create_command(owner, domain, fx.version_id, case_sensitive, json!({})),
            )
            .await
            .expect("case-distinct ids are unique by exact case-sensitive value");
        }
    }
}

#[tokio::test]
async fn acc2_metadata_cannot_substitute_for_formal_catalog() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;
    let fx = seed_model3_draft(&pool, domain, agent, Some(annotated_submission_schema())).await;
    publish_draft(&pool, owner, fx.version_id).await.unwrap();

    // Perfect catalog in `metadata`, but nothing at the formal context path.
    let err = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool,
        crate::common::admitting_gate(),
        create_command(
            owner,
            domain,
            fx.version_id,
            json!({"title": "t"}),
            catalog_context(&["C1", "C2"]),
        ),
    )
    .await
    .expect_err("metadata catalog must not satisfy the formal Context gate");
    assert!(
        matches!(err, CreateWorkflowInstanceError::ContextValidationFailed(_)),
        "{err:?}"
    );
}

#[tokio::test]
async fn acc2_unannotated_definition_has_no_catalog_gate() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;
    let fx = seed_model3_draft(&pool, domain, agent, None).await;
    publish_draft(&pool, owner, fx.version_id).await.unwrap();

    // No acceptanceCriteria anywhere: creation is unaffected (CTR-ACB-006).
    let result = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool,
        crate::common::admitting_gate(),
        create_command(owner, domain, fx.version_id, json!({"title": "plain"}), json!({})),
    )
    .await
    .expect("unannotated definition keeps existing create behavior");
    let _ = result;
}

// ============================================================================
// ACC-ACB-003 / CTR-ACB-003 — successful-ADVANCE binding predicate
// ============================================================================

#[tokio::test]
async fn acc3_exact_current_uuid_and_full_pass_set_advances_once() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    let before = fact_counts(&pool).await;
    let result =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool,
            crate::common::admitting_gate(),
            transition_command(
                agent,
                a.0,
                ver,
                fx.advance_work_done,
                Some(acceptance_report(a.1, pass_entries(&["C1", "C2"]))),
            ),
        )
        .await
        .expect("exact report must advance");
    assert_eq!(result.workflow_state_version, ver + 1);
    let after = fact_counts(&pool).await;
    assert_eq!(after.0, before.0 + 0, "no new instance");
    assert_eq!(after.1, before.1 + 1, "exactly one new visit");
    assert_eq!(after.2, before.2 + 1, "exactly one submission");
    assert_eq!(after.3, before.3 + 1, "exactly one event");
    assert_eq!(after.4, before.4, "no context delta");
}

#[tokio::test]
async fn acc3_same_ids_other_instance_context_uuid_cannot_exchange_reports() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, b) = two_instance_rig(&pool).await;
    assert_ne!(a.1, b.1, "two instances hold different Context UUIDs");
    let ver_a = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;
    let ver_b = advance_to_work(&pool, owner, &b, fx.advance_start_work).await;

    // A's report used on B (same Definition, same IDs, different Context UUID).
    for (actor, instance, version, report, other) in [
        (
            agent,
            b.0,
            ver_b,
            acceptance_report(a.1, pass_entries(&["C1", "C2"])),
            "A",
        ),
        (
            agent,
            a.0,
            ver_a,
            acceptance_report(b.1, pass_entries(&["C1", "C2"])),
            "B",
        ),
    ] {
        let before = fact_counts(&pool).await;
        let err =
            svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
                &pool,
                crate::common::admitting_gate(),
                transition_command(actor, instance, version, fx.advance_work_done, Some(report)),
            )
            .await
            .expect_err(&format!("report from instance {other} must be rejected"));
        assert!(
            matches!(err, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)),
            "{err:?}"
        );
        let after = fact_counts(&pool).await;
        assert_eq!(
            (before.0, before.1, before.2, before.3, before.4),
            (after.0, after.1, after.2, after.3, after.4),
            "zero workflow-fact delta on rejection"
        );
    }
}

#[tokio::test]
async fn acc3_wrong_uuid_shapes_rejected() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    // Arbitrary cached UUID (valid v4, not the current Context).
    let arbitrary = Uuid::new_v4();
    // Wrong-but-plausible: instance B's UUID is covered acc3_; here use random.
    for (name, report) in [
        (
            "arbitrary uuid",
            acceptance_report(arbitrary, pass_entries(&["C1", "C2"])),
        ),
        (
            "omitted uuid",
            json!({"acceptanceReport": {"results": pass_entries(&["C1", "C2"])}}),
        ),
        (
            "non-string uuid",
            json!({"acceptanceReport": {"contextRevisionId": 42, "results": pass_entries(&["C1", "C2"])}}),
        ),
        (
            "uppercase re-encoding",
            json!({"acceptanceReport": {
                "contextRevisionId": a.1.to_string().to_uppercase(),
                "results": pass_entries(&["C1", "C2"])
            }}),
        ),
    ] {
        let err =
            svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
                &pool,
                crate::common::admitting_gate(),
                transition_command(agent, a.0, ver, fx.advance_work_done, Some(report)),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("{name} must be rejected"));
        assert!(
            matches!(err, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)),
            "{name}: {err:?}"
        );
    }
}

#[tokio::test]
async fn acc3_missing_unknown_substituted_and_duplicate_ids_rejected() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    for (name, entries) in [
        ("missing id", pass_entries(&["C1"])),
        ("unknown id", pass_entries(&["C1", "C2", "CX"])),
        ("substituted id", pass_entries(&["C1", "C9"])),
        ("duplicate id", pass_entries(&["C1", "C1"])),
        ("empty results", pass_entries(&[])),
    ] {
        let err =
            svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
                &pool,
                crate::common::admitting_gate(),
                transition_command(
                    agent,
                    a.0,
                    ver,
                    fx.advance_work_done,
                    Some(acceptance_report(a.1, entries)),
                ),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("{name} must be rejected"));
        assert!(
            matches!(err, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)),
            "{name}: {err:?}"
        );
    }
}

#[tokio::test]
async fn acc3_non_pass_verdicts_rejected() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    for (name, entries) in [
        (
            "FAIL verdict",
            json!([
                {"id": "C1", "verdict": "PASS"},
                {"id": "C2", "verdict": "FAIL"}
            ]),
        ),
        (
            "lowercase pass",
            json!([
                {"id": "C1", "verdict": "pass"},
                {"id": "C2", "verdict": "PASS"}
            ]),
        ),
        (
            "numeric verdict",
            json!([
                {"id": "C1", "verdict": 1},
                {"id": "C2", "verdict": "PASS"}
            ]),
        ),
        (
            "missing verdict",
            json!([{"id": "C1"}, {"id": "C2", "verdict": "PASS"}]),
        ),
        (
            "missing id",
            json!([
                {"verdict": "PASS"},
                {"id": "C2", "verdict": "PASS"}
            ]),
        ),
        ("results not an array", json!({"id": "C1", "verdict": "PASS"})),
    ] {
        let err =
            svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
                &pool,
                crate::common::admitting_gate(),
                transition_command(
                    agent,
                    a.0,
                    ver,
                    fx.advance_work_done,
                    Some(acceptance_report(a.1, entries)),
                ),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("{name} must be rejected"));
        assert!(
            matches!(err, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)),
            "{name}: {err:?}"
        );
    }
}

#[tokio::test]
async fn acc3_structurally_valid_but_semantically_wrong_evidence_passes_gate() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    // CTR-ACB-003 example: correct UUID/IDs/PASS, but the evidence only
    // covers a canary. The structural gate MUST NOT decide business truth;
    // this documents that the example passes structurally (QA owns truth).
    let mut entries = pass_entries(&["C1", "C2"]);
    for entry in entries.as_array_mut().unwrap() {
        entry["evidence"] = json!("canary run only; regression testing NOT executed");
    }
    let result =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool,
            crate::common::admitting_gate(),
            transition_command(
                agent,
                a.0,
                ver,
                fx.advance_work_done,
                Some(acceptance_report(a.1, entries)),
            ),
        )
        .await
        .expect("structural gate is truth-agnostic; passes on correct references");
    let _ = result;
}

#[tokio::test]
async fn acc3_third_instance_with_different_catalog_rejects_foreign_id_sets() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, _a, _b) = two_instance_rig(&pool).await;
    let c = create_instance(&pool, owner, _domain, fx.version_id, &["D1"]).await;
    let ver_c = advance_to_work(&pool, owner, &c, fx.advance_start_work).await;

    // The C1/C2 set (catalog of A/B) must not satisfy C's D1 catalog.
    for entries in [
        pass_entries(&["C1", "C2"]),
        pass_entries(&["D1", "C1"]),
        pass_entries(&["D1", "D1"]),
    ] {
        let err =
            svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
                &pool,
                crate::common::admitting_gate(),
                transition_command(agent, c.0, ver_c, fx.advance_work_done, Some(acceptance_report(c.1, entries))),
            )
            .await
            .err()
            .expect("foreign/oversized id sets must be rejected for C");
        assert!(
            matches!(err, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)),
            "{err:?}"
        );
    }
    // C's own catalog advances.
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &pool,
        crate::common::admitting_gate(),
        transition_command(
            agent,
            c.0,
            ver_c,
            fx.advance_work_done,
            Some(acceptance_report(c.1, pass_entries(&["D1"]))),
        ),
    )
    .await
    .expect("own catalog advances");
}

#[tokio::test]
async fn acc3_annotated_advance_without_payload_still_submission_required() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;
    let err =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool,
            crate::common::admitting_gate(),
            transition_command(agent, a.0, ver, fx.advance_work_done, None),
        )
        .await
        .expect_err("missing payload retains existing submission_required");
    assert!(
        matches!(err, ExecuteWorkflowTransitionError::SubmissionRequired),
        "{err:?}"
    );
}

#[tokio::test]
async fn acc3_unannotated_advance_does_not_require_report() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;
    let fx = seed_model3_draft(&pool, domain, agent, Some(plain_acceptance_report_schema())).await;
    // NOTE: schema present but annotation absent — the binding gate is off;
    // only the ordinary schema applies (payload must satisfy it).
    publish_draft(&pool, owner, fx.version_id).await.unwrap();
    let (instance, ctx, _ver) = {
        let result =
            svc_workflow::application::workflow_instance::create::create_workflow_instance(
                &pool,
                crate::common::admitting_gate(),
                create_command(owner, domain, fx.version_id, json!({"title": "x"}), json!({})),
            )
            .await
            .unwrap();
        (
            result.workflow_instance_id,
            result.current_context_revision_id,
            result.workflow_state_version,
        )
    };
    let ver = advance_to_work(&pool, owner, &(instance, ctx, _ver), fx.advance_start_work).await;
    // Payload satisfies the plain schema; no report needed — a simple marker
    // object without all-PASS semantics would be schema-invalid, so use the
    // report shape but with a full-PASS set against a catalog that was never
    // required: the binding gate is off, ordinary schema decides.
    let payload = acceptance_report(ctx, pass_entries(&["WHATEVER"]));
    let result =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool,
            crate::common::admitting_gate(),
            transition_command(agent, instance, ver, fx.advance_work_done, Some(payload)),
        )
        .await
        .expect("unannotated transition keeps existing behavior (schema-only)");
    let _ = result;
}

// ============================================================================
// ACC-ACB-004 / CTR-ACB-004 — combined fail-closed + RETURN/TERMINATE
// ============================================================================

/// The Spec-sanctioned drift fixture: a Legacy DRAFT-entry definition whose
/// primary ADVANCE carries the reserved annotation. The new binary's publish
/// path cannot produce this (ACC-ACB-001 proves it); it models an old-binary
/// or drifted row. Combined must fail closed on it.
async fn seed_legacy_annotated_drift(pool: &PgPool, domain_id: Uuid, creator: Uuid) -> Uuid {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Drift')",
    )
    .bind(def_id).bind(domain_id).bind(format!("drift-{}", &Uuid::new_v4().to_string()[..8]))
    .execute(pool).await.expect("insert def");
    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, semantic_model_version) VALUES ($1, $2, 1, 'DRAFT', 1)",
    )
    .bind(ver_id).bind(def_id)
    .execute(pool).await.expect("insert version");
    let draft_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')",
    ).bind(draft_id).bind(ver_id).execute(pool).await.expect("insert draft");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', NULL)",
    ).bind(term_id).bind(ver_id).execute(pool).await.expect("insert done");
    let t1 = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema) VALUES ($1, $2, 'advance', 'advance', $3, $4, 'ADVANCE', $5)",
    ).bind(t1).bind(ver_id).bind(draft_id).bind(term_id)
    .bind(annotated_submission_schema())
    .execute(pool).await.expect("insert transition");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t1).bind(draft_id).execute(pool).await.expect("primary");
    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(pool).await.expect("publish drift fixture");
    let _ = creator;
    ver_id
}

#[tokio::test]
async fn acc4_combined_with_annotated_schema_fails_closed_zero_delta() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let ver = seed_legacy_annotated_drift(&pool, domain, owner).await;

    // Create a legacy instance on the drift version (plain create path).
    let created =
        svc_workflow::application::workflow_instance::create::create_workflow_instance(
            &pool,
            crate::common::admitting_gate(),
            create_command(owner, domain, ver, catalog_context(&["X1"]), json!({})),
        )
        .await
        .expect("create on legacy drift fixture keeps existing behavior");
    let instance = created.workflow_instance_id;

    // Resolve the primary ADVANCE transition of the drift fixture. The
    // annotation backstop is only reachable with a resolvable transition —
    // exactly the condition that matters (a schema selected under this
    // Instance's exact Definition Version).
    let (t1,): (Uuid,) = sqlx::query_as(
        "SELECT transition_id FROM workflow_transition_definitions WHERE definition_version_id = $1",
    )
    .bind(ver)
    .fetch_one(&pool)
    .await
    .unwrap();

    let before = fact_counts(&pool).await;
    let err = revise_context_and_transition(
        &pool,
        crate::common::admitting_gate(),
        ReviseContextAndTransitionCommand {
            principal_id: PrincipalId::from_uuid(owner),
            idempotency_key: format!("combined-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance),
            expected_workflow_state_version: created.workflow_state_version,
            transition_definition_id: TransitionId::from_uuid(t1),
            context_payload: json!({"title": "revised"}),
            submission_payload: json!({"acceptanceReport": {
                "contextRevisionId": Uuid::new_v4().to_string(),
                "results": []
            }}),
        },
    )
    .await
    .err()
    .expect("combined on annotated schema must fail closed");
    assert!(
        matches!(err, ReviseContextAndTransitionError::SubmissionValidationFailed(_)),
        "{err:?}"
    );
    let after = fact_counts(&pool).await;
    assert_eq!(
        (before.0, before.1, before.2, before.3, before.4),
        (after.0, after.1, after.2, after.3, after.4),
        "combined never commits (zero workflow-fact delta)"
    );
}

#[tokio::test]
async fn acc4_model3_combined_still_rejected_as_legacy_command() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, _agent, a, _b) = two_instance_rig(&pool).await;
    let err = revise_context_and_transition(
        &pool,
        crate::common::admitting_gate(),
        ReviseContextAndTransitionCommand {
            principal_id: PrincipalId::from_uuid(owner),
            idempotency_key: format!("combined-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(a.0),
            expected_workflow_state_version: a.2,
            transition_definition_id: TransitionId::from_uuid(fx.advance_start_work),
            context_payload: json!({"title": "revised"}),
            submission_payload: json!({}),
        },
    )
    .await
    .err()
    .expect("model3 combined must be rejected");
    assert!(
        matches!(err, ReviseContextAndTransitionError::LegacyCommandNotSupported),
        "{err:?}"
    );
}

#[tokio::test]
async fn acc4_return_executes_without_all_pass_after_rejected_report() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    // Rejected report first.
    let err =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool,
            crate::common::admitting_gate(),
            transition_command(
                agent,
                a.0,
                ver,
                fx.advance_work_done,
                Some(acceptance_report(Uuid::new_v4(), pass_entries(&["C1", "C2"]))),
            ),
        )
        .await
        .err()
        .expect("wrong-uuid report must fail");
    assert!(matches!(err, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)));

    // Valid RETURN (own contract fields, own schema, no all-PASS requirement).
    let result =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool,
            crate::common::admitting_gate(),
            transition_command(
                agent,
                a.0,
                ver,
                fx.return_work_start,
                Some(json!({
                    "rootCauseNodeVisitId": current_visit_of(&pool, a.0).await.to_string(),
                    "reasonCode": "NEEDS_REVISION",
                    "reason": "criteria need rework",
                })),
            ),
        )
        .await
        .expect("RETURN must remain usable without all-PASS");
    assert_eq!(result.workflow_state_version, ver + 1);
}

#[tokio::test]
async fn acc4_terminate_executes_without_all_pass() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    let result =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool,
            crate::common::admitting_gate(),
            transition_command(agent, a.0, ver, fx.terminate_work_failed, Some(json!({
                "rootCauseNodeVisitId": current_visit_of(&pool, a.0).await.to_string(),
                "reasonCode": "CANCELLED_WORK",
                "reason": "terminating without acceptance",
            }))),
        )
        .await
        .expect("TERMINATE must remain usable without all-PASS");
    assert_eq!(result.workflow_state_version, ver + 1);
}

async fn current_visit_of(pool: &PgPool, instance: Uuid) -> Uuid {
    let (visit,): (Uuid,) = sqlx::query_as(
        "SELECT current_node_visit_id FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance)
    .fetch_one(pool)
    .await
    .unwrap();
    visit
}

// ============================================================================
// ACC-ACB-005 / CTR-ACB-005 — CAS, replay, conflict discipline
// ============================================================================

#[tokio::test]
async fn acc5_stale_cas_conflicts_with_zero_delta_and_receipt() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    let before = fact_counts(&pool).await;
    let err =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool,
            crate::common::admitting_gate(),
            transition_command(
                agent,
                a.0,
                ver - 1, // stale
                fx.advance_work_done,
                Some(acceptance_report(a.1, pass_entries(&["C1", "C2"]))),
            ),
        )
        .await
        .err()
        .expect("stale CAS must conflict");
    assert!(
        matches!(err, ExecuteWorkflowTransitionError::WorkflowStateVersionConflict { .. }),
        "{err:?}"
    );
    let after = fact_counts(&pool).await;
    assert_eq!(
        (before.0, before.1, before.2, before.3, before.4),
        (after.0, after.1, after.2, after.3, after.4),
        "zero business delta"
    );
    assert_eq!(after.5, before.5 + 1, "deterministic failure receipt persists");
}

#[tokio::test]
async fn acc5_success_replay_returns_original_without_gate_or_new_facts() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    let mut cmd = transition_command(
        agent,
        a.0,
        ver,
        fx.advance_work_done,
        Some(acceptance_report(a.1, pass_entries(&["C1", "C2"]))),
    );
    cmd.idempotency_key = "acb-replay-success-key".to_string();
    let first =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool, crate::common::admitting_gate(), cmd.clone(),
        )
        .await
        .expect("first execution succeeds");
    let before = fact_counts(&pool).await;

    let replay =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool, crate::common::admitting_gate(), cmd,
        )
        .await
        .expect("same-key same-payload replay succeeds");
    assert_eq!(
        first.current_node_visit_id,
        replay.current_node_visit_id,
        "replay returns the original visit"
    );
    assert_eq!(
        first.workflow_state_version, replay.workflow_state_version,
        "replay returns the original state version"
    );
    let after = fact_counts(&pool).await;
    assert_eq!(before, after, "replay creates no new facts");
}

#[tokio::test]
async fn acc5_failure_then_replay_returns_original_failure_zero_extra_facts() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, _b) = two_instance_rig(&pool).await;
    let ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    let mut cmd = transition_command(
        agent,
        a.0,
        ver,
        fx.advance_work_done,
        Some(acceptance_report(Uuid::new_v4(), pass_entries(&["C1", "C2"]))),
    );
    cmd.idempotency_key = "acb-replay-failure-key".to_string();
    let first =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool, crate::common::admitting_gate(), cmd.clone(),
        )
        .await
        .err()
        .expect("wrong-uuid report fails");
    assert!(matches!(first, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)));
    let before = fact_counts(&pool).await;

    // Same key, same (bad) payload → original failure replayed.
    let replay =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool, crate::common::admitting_gate(), cmd,
        )
        .await
        .err()
        .expect("replayed failure");
    assert!(
        matches!(replay, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)),
        "{replay:?}"
    );
    let after = fact_counts(&pool).await;
    assert_eq!(
        (before.0, before.1, before.2, before.3, before.4),
        (after.0, after.1, after.2, after.3, after.4),
        "replay creates no facts"
    );

    // Same key, changed payload → conflict.
    let mut changed = transition_command(
        agent,
        a.0,
        ver,
        fx.advance_work_done,
        Some(acceptance_report(a.1, pass_entries(&["C1", "C2"]))),
    );
    changed.idempotency_key = "acb-replay-failure-key".to_string();
    let conflict =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool, crate::common::admitting_gate(), changed,
        )
        .await
        .err()
        .expect("changed payload same key must conflict");
    assert!(
        matches!(conflict, ExecuteWorkflowTransitionError::IdempotencyConflict { .. }),
        "{conflict:?}"
    );
}

// ============================================================================
// HTTP round-trip (real publish → create → transition entry points)
// ============================================================================

mod http_roundtrip {
    use super::*;
    use axum::body::{to_bytes, Body};
    use axum::http::Request;
    use tower::ServiceExt;

    use svc_workflow::application::provisioning::ProvisioningConfig;
    use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
    use svc_workflow::http::{self, AppState, HttpConfig};

    fn build_app(pool: sqlx::PgPool, jwks_url: &str) -> axum::Router {
        let config = HttpConfig {
            admission: crate::common::admitting_config(),
            bind_addr: "127.0.0.1:0".parse().unwrap(),
            request_body_max_bytes: 2_097_152,
            request_timeout_seconds: 30,
            jwks_config: JwksConfig {
                jwks_url: jwks_url.to_string(),
                issuer: "auth-service".to_string(),
                audience: "svc-workflow".to_string(),
                cache_ttl_secs: 300,
                http_timeout_secs: 5,
                max_stale_secs: 600,
                clock_skew_seconds: 60,
            },
            provisioning_config: ProvisioningConfig::new(vec![]),
            execution_control: svc_workflow::http::ExecutionControlConfig {
                max_returns_per_edge: 3,
            },
            auth_v1_canary_config: AuthV1CanaryConfig {
                enabled: true,
                write_enabled: true,
                allowed_client_id: "test-client".to_string(),
                allowed_sub: String::new(),
                allowed_delegating_sub: String::new(),
                jwks_url: jwks_url.to_string(),
                issuer: "auth-service".to_string(),
                audience: "svc-workflow".to_string(),
                cache_ttl_secs: 300,
                http_timeout_secs: 5,
                max_stale_secs: 600,
                clock_skew_seconds: 60,
            },
        };
        http::router(AppState::new(pool, &config), &config)
    }

    async fn req(
        app: &axum::Router,
        method: &str,
        path: &str,
        token: &str,
        body: Value,
    ) -> (u16, Value) {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(path)
                    .header("authorization", format!("Bearer {token}"))
                    .header("idempotency-key", Uuid::new_v4().to_string())
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status().as_u16();
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
    }

    #[tokio::test]
    async fn http_publish_create_transition_roundtrip_with_binding() {
        let pool = common::create_pool().await;
        let owner = common::seed_second_principal(&pool).await;
        let (_, domain) = common::seed_principal_and_domain(&pool).await;
        common::seed_domain_owner(&pool, domain, owner).await;
        let jwks = common::MockJwksServer::start().await;
        let app = build_app(pool.clone(), &jwks.url);
        let token = common::v1_token(
            owner,
            "workflow.execute workflow.read",
            "test-client",
            300,
            &jwks.key_pair,
        );

        // Author a model3 definition over real HTTP and attach the annotated
        // schema to the primary ADVANCE work->done.
        let root = format!("/internal/v1/domains/{domain}/definitions");
        let (status, def) = req(&app, "POST", &root, &token,
            json!({"definitionKey": format!("acb-http-{}", &Uuid::new_v4().to_string()[..8]),
                   "displayName": "ACB HTTP"})).await;
        assert_eq!(status, 200, "{def}");
        let def_id = def["workflowDefinitionId"].as_str().unwrap();
        let base = format!("{root}/{def_id}");
        let (status, version) = req(&app, "POST", &format!("{base}/versions"), &token,
            json!({"semanticModelVersion": 3})).await;
        assert_eq!(status, 200, "{version}");
        let ver = version["definitionVersionId"].as_str().unwrap();

        // Node IDs are minted by the draft API; fetch the graph to learn keys.
        let graph = json!({
            "definitionVersionId": ver,
            "nodes": [
                {"node_key": "start", "display_name": "Start", "order_index": 0,
                 "node_type": "TASK", "assignee_ref_type": "WORKFLOW_CREATOR",
                 "primary_advance_transition_key": "advance-1"},
                {"node_key": "work", "display_name": "Work", "order_index": 1,
                 "node_type": "TASK", "assignee_ref_type": "WORKFLOW_CREATOR",
                 "primary_advance_transition_key": "advance-2"},
                {"node_key": "done", "display_name": "Done", "order_index": 2,
                 "node_type": "TERMINAL"}
            ],
            "transitions": [
                {"transition_key": "advance-1", "display_name": "A1",
                 "source_node_key": "start", "target_node_key": "work",
                 "transition_effect": "ADVANCE"},
                {"transition_key": "advance-2", "display_name": "A2",
                 "source_node_key": "work", "target_node_key": "done",
                 "transition_effect": "ADVANCE",
                 "submission_schema": annotated_submission_schema()}
            ]
        });
        let (status, replaced) = req(&app, "PUT", &format!("{base}/draft"), &token, graph).await;
        assert_eq!(status, 200, "{replaced}");
        let (status, published) = req(&app, "POST", &format!("{base}/publish"), &token,
            json!({"versionId": ver})).await;
        assert_eq!(status, 200, "{published}");
        assert_eq!(published["versionStatus"], "PUBLISHED");

        // Create over HTTP with a valid catalog.
        let (status, created) = req(&app, "POST", "/internal/v1/workflow-instances", &token,
            json!({
                "domainId": domain,
                "definitionVersionId": ver,
                "contextPayload": catalog_context(&["C1"]),
                "metadata": {},
                "executionClass": "BUSINESS"
            }))
        .await;
        assert_eq!(status, 201, "{created}");
        let instance = created["workflowInstanceId"].as_str().unwrap();
        let ctx = created["currentContextRevisionId"].as_str().unwrap();

        // Advance start->work (creator is the assignee; same principal here).
        let (t1,): (Uuid,) = sqlx::query_as(
            "SELECT t.transition_id FROM workflow_transition_definitions t \
             WHERE t.definition_version_id = $1 AND t.transition_key = 'advance-1'",
        )
        .bind(Uuid::parse_str(ver).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
        let (status, adv) = req(&app,
            "POST",
            &format!("/internal/v1/workflow-instances/{instance}/transitions"),
            &token,
            json!({
                "expectedWorkflowStateVersion": created["workflowStateVersion"],
                "transitionDefinitionId": t1
            }))
        .await;
        assert_eq!(status, 200, "{adv}");
        let new_version = adv["workflowStateVersion"].as_i64().unwrap();

        let (t2,): (Uuid,) = sqlx::query_as(
            "SELECT t.transition_id FROM workflow_transition_definitions t \
             WHERE t.definition_version_id = $1 AND t.transition_key = 'advance-2'",
        )
        .bind(Uuid::parse_str(ver).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();

        // A foreign Context UUID over HTTP is rejected with the existing
        // envelope while the instance is still at the TASK.
        let (status, rej) = req(&app,
            "POST",
            &format!("/internal/v1/workflow-instances/{instance}/transitions"),
            &token,
            json!({
                "expectedWorkflowStateVersion": new_version,
                "transitionDefinitionId": t2,
                "submissionPayload": acceptance_report(Uuid::new_v4(), pass_entries(&["C1"]))
            }))
        .await;
        assert_eq!(status, 422, "{rej}");
        assert_eq!(rej["error"]["code"], "submission_validation_failed", "{rej}");

        // The exact report advances over HTTP.
        let report = acceptance_report(Uuid::parse_str(ctx).unwrap(), pass_entries(&["C1"]));
        let (status, done) = req(&app,
            "POST",
            &format!("/internal/v1/workflow-instances/{instance}/transitions"),
            &token,
            json!({
                "expectedWorkflowStateVersion": new_version,
                "transitionDefinitionId": t2,
                "submissionPayload": report
            }))
        .await;
        assert_eq!(status, 200, "{done}");
        assert_eq!(done["submissionId"].is_null(), false);
    }

    #[tokio::test]
    async fn http_publish_rejects_annotation_on_return_transition() {
        let pool = common::create_pool().await;
        let owner = common::seed_second_principal(&pool).await;
        let (_, domain) = common::seed_principal_and_domain(&pool).await;
        common::seed_domain_owner(&pool, domain, owner).await;
        let jwks = common::MockJwksServer::start().await;
        let app = build_app(pool.clone(), &jwks.url);
        let token = common::v1_token(
            owner,
            "workflow.execute workflow.read",
            "test-client",
            300,
            &jwks.key_pair,
        );
        let root = format!("/internal/v1/domains/{domain}/definitions");
        let (status, def) = req(&app, "POST", &root, &token,
            json!({"definitionKey": format!("acb-http-{}", &Uuid::new_v4().to_string()[..8]),
                   "displayName": "ACB HTTP Return"})).await;
        assert_eq!(status, 200, "{def}");
        let base = format!("{root}/{}", def["workflowDefinitionId"].as_str().unwrap());
        let (status, version) = req(&app, "POST", &format!("{base}/versions"), &token,
            json!({"semanticModelVersion": 3})).await;
        assert_eq!(status, 200, "{version}");
        let ver = version["definitionVersionId"].as_str().unwrap();

        let mut return_schema = plain_acceptance_report_schema();
        return_schema[ANNOTATION_KEY] = annotation_value();
        let graph = json!({
            "definitionVersionId": ver,
            "nodes": [
                {"node_key": "start", "display_name": "Start", "order_index": 0,
                 "node_type": "TASK", "assignee_ref_type": "WORKFLOW_CREATOR",
                 "primary_advance_transition_key": "advance-1"},
                {"node_key": "work", "display_name": "Work", "order_index": 1,
                 "node_type": "TASK", "assignee_ref_type": "WORKFLOW_CREATOR",
                 "primary_advance_transition_key": "advance-2"},
                {"node_key": "done", "display_name": "Done", "order_index": 2,
                 "node_type": "TERMINAL"}
            ],
            "transitions": [
                {"transition_key": "advance-1", "display_name": "A1",
                 "source_node_key": "start", "target_node_key": "work",
                 "transition_effect": "ADVANCE"},
                {"transition_key": "advance-2", "display_name": "A2",
                 "source_node_key": "work", "target_node_key": "done",
                 "transition_effect": "ADVANCE"},
                {"transition_key": "back", "display_name": "Back",
                 "source_node_key": "work", "target_node_key": "start",
                 "transition_effect": "RETURN",
                 "submission_schema": return_schema}
            ]
        });
        let (status, replaced) = req(&app, "PUT", &format!("{base}/draft"), &token, graph).await;
        assert_eq!(status, 200, "{replaced}");
        let (status, published) = req(&app, "POST", &format!("{base}/publish"), &token,
            json!({"versionId": ver})).await;
        assert_eq!(status, 422, "{published}");
        assert!(
            published.to_string().contains("ACCEPTANCE_BINDING"),
            "{published}"
        );
        // Version stays DRAFT (fail closed).
        let (st,): (String,) = sqlx::query_as(
            "SELECT version_status::text FROM workflow_definition_versions WHERE definition_version_id = $1",
        )
        .bind(Uuid::parse_str(ver).unwrap())
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(st, "DRAFT");
    }
}


// ============================================================================
// F1 (review ACB-SOURCE-SNAPSHOT-A1-INDEPENDENT-REVIEW) — an existing,
// correctly linked Context whose catalog content is missing/invalid fails as
// a deterministic 422 `submission_validation_failed` with the failure
// Receipt; the wrong-instance authoritative linkage is structurally fenced
// by the composite FK, leaving the InternalConsistency branch as
// defense-in-depth.
// ============================================================================

#[allow(clippy::type_complexity)]
async fn binding_fact_counts(pool: &PgPool) -> (i64, i64, i64, i64, i64, i64, i64) {
    let (instances, visits, submissions, events, contexts, receipts) = fact_counts(pool).await;
    let activations: (i64,) = sqlx::query_as("SELECT count(*) FROM workflow_activations")
        .fetch_one(pool)
        .await
        .unwrap();
    (instances, visits, submissions, events, contexts, receipts, activations.0)
}

/// Insert a drift Instance on the published annotated version: a Context
/// revision whose payload carries the given (missing/invalid) catalog, an
/// Instance row correctly linked to it (composite FK satisfied), and a
/// current visit on the `work` TASK owned by `agent`. The create gate cannot
/// produce these states on the public path — that is exactly the drifted
/// classification surface F1 pins. The definition itself is published through
/// the real service (publish-path purity preserved).
async fn seed_drift_instance_with_context(
    pool: &PgPool,
    domain: Uuid,
    agent: Uuid,
    owner: Uuid,
    version_id: Uuid,
    context_payload: Value,
) -> (Uuid, Uuid, Uuid) {
    let def_row: (Uuid, Uuid) = sqlx::query_as(
        "SELECT n.node_id, t.transition_id \
         FROM workflow_node_definitions n \
         JOIN workflow_transition_definitions t \
           ON t.definition_version_id = n.definition_version_id \
          AND t.source_node_id = n.node_id \
         WHERE n.definition_version_id = $1 AND n.node_key = 'work' \
           AND t.transition_key = 'advance-2'",
    )
    .bind(version_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let (work_node_id, advance_work_done) = def_row;

    let instance = Uuid::new_v4();
    let ctx = Uuid::new_v4();
    let visit = Uuid::new_v4();

    // The instance/context FKs are circular; the composite
    // fk_instance_current_ctx permits a NULL context reference, so the
    // fixture inserts the instance first and links the context afterwards.
    sqlx::query(
        "INSERT INTO workflow_instances \
         (workflow_instance_id, domain_id, definition_version_id, created_by_principal_id, \
          workflow_state_version, current_context_revision_id, current_node_visit_id, \
          external_reference, external_url, metadata, semantic_model_version, execution_class) \
         VALUES ($1, $2, $3, $4, 2, NULL, NULL, NULL, NULL, '{}', 3, 'BUSINESS')",
    )
    .bind(instance)
    .bind(domain)
    .bind(version_id)
    .bind(owner)
    .execute(pool)
    .await
    .unwrap();

    let payload_digest =
        svc_workflow::domain::definition::digest::compute_json_digest(&context_payload)
            .expect("drift payload digest");
    sqlx::query(
        "INSERT INTO workflow_context_revisions \
         (context_revision_id, workflow_instance_id, revision_number, previous_revision_id, \
          payload, payload_digest, created_by_principal_id) \
         VALUES ($1, $2, 1, NULL, $3, $4, $5)",
    )
    .bind(ctx)
    .bind(instance)
    .bind(&context_payload)
    .bind(&payload_digest)
    .bind(owner)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO workflow_node_visits \
         (node_visit_id, workflow_instance_id, node_id, visit_number, \
          assignee_principal_id, entered_by_transition_id) \
         VALUES ($1, $2, $3, 1, $4, NULL)",
    )
    .bind(visit)
    .bind(instance)
    .bind(work_node_id)
    .bind(agent)
    .execute(pool)
    .await
    .unwrap();

    sqlx::query(
        "UPDATE workflow_instances SET current_context_revision_id = $1, \
         current_node_visit_id = $2 WHERE workflow_instance_id = $3",
    )
    .bind(ctx)
    .bind(visit)
    .bind(instance)
    .execute(pool)
    .await
    .unwrap();

    (instance, ctx, advance_work_done)
}

#[tokio::test]
async fn f1_existing_linked_context_with_missing_catalog_is_deterministic_422() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, _a, _b) = two_instance_rig(&pool).await;
    let (instance, ctx, t2) = seed_drift_instance_with_context(
        &pool,
        _domain,
        agent,
        owner,
        fx.version_id,
        json!({"title": "catalog lost"}),
    )
    .await;

    let mut cmd = transition_command(
        agent,
        instance,
        2,
        t2,
        Some(acceptance_report(ctx, pass_entries(&["C1", "C2"]))),
    );
    cmd.idempotency_key = format!("f1-missing-catalog-{}", Uuid::new_v4());
    let before = binding_fact_counts(&pool).await;
    let err =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool, crate::common::admitting_gate(), cmd.clone(),
        )
        .await
        .err()
        .expect("missing catalog on a linked Context must fail");
    assert!(
        matches!(err, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)),
        "expected deterministic submission_validation_failed, got {err:?}"
    );
    let after = binding_fact_counts(&pool).await;
    assert_eq!(
        (before.0, before.1, before.2, before.3, before.4, before.6),
        (after.0, after.1, after.2, after.3, after.4, after.6),
        "zero business/activation delta"
    );
    assert_eq!(after.5, before.5 + 1, "deterministic failure receipt persists");
    let receipt: (i32, Value) = sqlx::query_as(
        "SELECT response_status, response_body FROM workflow_command_receipts \
         ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(receipt.0, 422);
    assert_eq!(receipt.1["error"], "submission_validation_failed");

    // Same-key same-payload replay returns the original deterministic failure.
    let replay_err =
        svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
            &pool, crate::common::admitting_gate(), cmd,
        )
        .await
        .err()
        .expect("replayed failure");
    assert!(matches!(replay_err, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)));
    let after_replay = binding_fact_counts(&pool).await;
    assert_eq!(after, after_replay, "replay creates no facts");
}

#[tokio::test]
async fn f1_existing_linked_context_with_malformed_catalog_rejected() {
    for (name, catalog) in [
        ("empty array", json!({"title": "t", "acceptanceCriteria": []})),
        (
            "duplicate ids",
            json!({"title": "t", "acceptanceCriteria": [
                {"id": "C1", "description": "a"},
                {"id": "C1", "description": "b"}
            ]}),
        ),
    ] {
        let pool = common::create_pool().await;
        let (_domain, fx, owner, agent, _a, _b) = two_instance_rig(&pool).await;
        let (instance, ctx, t2) = seed_drift_instance_with_context(
            &pool,
            _domain,
            agent,
            owner,
            fx.version_id,
            catalog,
        )
        .await;

        let before = binding_fact_counts(&pool).await;
        let err =
            svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
                &pool,
                crate::common::admitting_gate(),
                transition_command(
                    agent,
                    instance,
                    2,
                    t2,
                    Some(acceptance_report(ctx, pass_entries(&["C1", "C2"]))),
                ),
            )
            .await
            .err()
            .unwrap_or_else(|| panic!("{name} must be rejected"));
        assert!(
            matches!(err, ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)),
            "{name}: expected deterministic submission_validation_failed, got {err:?}"
        );
        let after = binding_fact_counts(&pool).await;
        assert_eq!(
            (before.0, before.1, before.2, before.3, before.4, before.6),
            (after.0, after.1, after.2, after.3, after.4, after.6),
            "{name}: zero business/activation delta"
        );
        assert_eq!(after.5, before.5 + 1, "{name}: failure receipt persists");
    }
}

#[tokio::test]
async fn f1_wrong_instance_context_linkage_is_structurally_fenced() {
    let pool = common::create_pool().await;
    let (_domain, fx, owner, agent, a, b) = two_instance_rig(&pool).await;
    let _ver = advance_to_work(&pool, owner, &a, fx.advance_start_work).await;

    // The composite FK fk_instance_current_ctx (current_context_revision_id,
    // workflow_instance_id) makes a wrong-instance authoritative linkage
    // unrepresentable: pointing this Instance at another Instance's Context
    // revision violates the FK. The InternalConsistency branch in the
    // transition binding gate is therefore defense-in-depth for corrupt
    // linkage that no SQL-legal state can produce; this test pins the fence.
    let result = sqlx::query(
        "UPDATE workflow_instances SET current_context_revision_id = $1 \
         WHERE workflow_instance_id = $2",
    )
    .bind(b.1)
    .bind(a.0)
    .execute(&pool)
    .await;
    assert!(
        result.is_err(),
        "composite FK must fence wrong-instance Context linkage"
    );
    let err_text = result.err().unwrap().to_string();
    assert!(err_text.contains("fk_instance_current_ctx"), "{err_text}");
}
