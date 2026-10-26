//! SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1 — CTR-ACB-006 downgrade gate.
//!
//! Proves the two halves of the safe-downgrade contract against a REAL old
//! binary (a server built from a pre-implementation base commit, which
//! ignores the reserved annotation entirely):
//!
//! 1. New implementation: an opt-in version is published through the real
//!    service, executed, and then durably blocked via the legitimate
//!    `DefinitionService::revoke_version` application service called by a
//!    real enabled DOMAIN_OWNER (no HTTP route for revoke exists in either
//!    binary; the application service is the legal actor path). The new
//!    binary then refuses ordinary execution of the REVOKED version.
//! 2. Old binary: spawned as a real HTTP server against the same isolated
//!    database; ordinary execution of the same REVOKED opt-in version is
//!    rejected by the old binary as well. Readback proves the downgrade
//!    gate — this is NOT a health/readback-only claim.
//!
//! Environment:
//! - `TEST_DATABASE_URL`: isolated PostgreSQL 16 test database (required).
//! - `ACB_OLD_BINARY`: path to the base-commit server binary. When unset,
//!   the old-binary leg is reported as NOT RUN on stderr and skipped —
//!   that skip is a deliverable-evidence fact, never a silent pass.

#![allow(clippy::needless_borrow)]
#![allow(unused_imports, unused_variables)]

#[path = "common/mod.rs"]
mod common;

use std::time::Duration;

use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::definition::commands::{PublishVersion, RevokeVersion};
use svc_workflow::application::definition::repository::DefinitionRepository;
use svc_workflow::application::definition::DefinitionService;
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
};
use svc_workflow::store::postgres::admission_gate::AdmissionGate;
use svc_workflow::store::postgres::definition_repository::PgDefinitionRepository;

const ANNOTATION_KEY: &str = "x-workflow-acceptance-binding-v1";

fn annotated_submission_schema() -> Value {
    json!({
        ANNOTATION_KEY: {
            "contextCatalogPointer": "/acceptanceCriteria",
            "submissionReportPointer": "/acceptanceReport",
        },
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

/// Seed the same model3 DRAFT graph as tests/41 and publish through the real
/// service. Returns (version_id, start->work, work->done transitions).
async fn seed_and_publish_annotated(
    pool: &PgPool,
    domain: Uuid,
    agent: Uuid,
    owner: Uuid,
) -> (Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Downgrade Def')",
    )
    .bind(def_id).bind(domain).bind(format!("acb-dg-{}", &Uuid::new_v4().to_string()[..8]))
    .execute(pool).await.expect("insert definition");
    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, semantic_model_version) VALUES ($1, $2, 1, 'DRAFT', 3)",
    )
    .bind(ver_id).bind(def_id)
    .execute(pool).await.expect("insert version");

    let start_id = Uuid::new_v4();
    let work_id = Uuid::new_v4();
    let done_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'start', 'Start', 0, 'TASK', 'WORKFLOW_CREATOR')",
    ).bind(start_id).bind(ver_id).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) VALUES ($1, $2, 'work', 'Work', 1, 'TASK', 'FIXED_PRINCIPAL', $3)",
    ).bind(work_id).bind(ver_id).bind(agent).execute(pool).await.unwrap();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)",
    ).bind(done_id).bind(ver_id).execute(pool).await.unwrap();

    let t1 = Uuid::new_v4();
    let t2 = Uuid::new_v4();
    for (tid, key, src, dst, schema) in [
        (t1, "advance-1", start_id, work_id, None),
        (t2, "advance-2", work_id, done_id, Some(annotated_submission_schema())),
    ] {
        sqlx::query(
            "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema) VALUES ($1, $2, $3, $3, $4, $5, 'ADVANCE', $6)",
        )
        .bind(tid).bind(ver_id).bind(key).bind(src).bind(dst).bind(schema)
        .execute(pool).await.unwrap();
    }
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t1).bind(start_id).execute(pool).await.unwrap();
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(t2).bind(work_id).execute(pool).await.unwrap();

    // Publish through the REAL application service (enabled DOMAIN_OWNER).
    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);
    service
        .publish_version(
            PublishVersion {
                actor_principal_id: owner,
                definition_version_id: ver_id,
                expected_revision: None,
            },
            AdmissionGate::disabled(),
        )
        .await
        .expect("annotated version must publish on the new implementation");

    (ver_id, t1, t2)
}

#[tokio::test]
async fn ctr6_revoked_optin_version_rejected_by_new_and_old_binary() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;
    let (ver_id, t1, t2) = seed_and_publish_annotated(&pool, domain, agent, owner).await;

    // Create an instance with a valid catalog and advance to the annotated
    // primary ADVANCE (work node).
    let created = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool,
        AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner),
            idempotency_key: format!("create-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
            domain_id: DomainId::from_uuid(domain),
            definition_version_id: DefinitionVersionId::from_uuid(ver_id),
            external_reference: None,
            external_url: None,
            metadata: json!({}),
            context_payload: json!({
                "acceptanceCriteria": [{"id": "C1", "description": "criterion one"}]
            }),
        },
    )
    .await
    .expect("instance with valid catalog must create");
    let instance = created.workflow_instance_id;
    let ctx = created.current_context_revision_id;
    let state_version = created.workflow_state_version;

    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &pool,
        AdmissionGate::disabled(),
        ExecuteWorkflowTransitionCommand {
            principal_id: PrincipalId::from_uuid(owner),
            idempotency_key: format!("trans-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance),
            expected_workflow_state_version: state_version,
            transition_definition_id: TransitionId::from_uuid(t1),
            submission_payload: None,
        },
    )
    .await
    .expect("unannotated start->work must advance");

    // Durable block through the legitimate application service, called by a
    // real enabled DOMAIN_OWNER principal (the repository's atomic_revoke is
    // the same path an HTTP route would delegate to).
    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);
    let revoked = service
        .revoke_version(RevokeVersion {
            actor_principal_id: owner,
            definition_version_id: ver_id,
        })
        .await
        .expect("enabled DOMAIN_OWNER revoke must succeed");
    assert_eq!(revoked.version_status.to_string(), "REVOKED");
    let (status,): (String,) = sqlx::query_as(
        "SELECT version_status::text FROM workflow_definition_versions WHERE definition_version_id = $1",
    )
    .bind(ver_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(status, "REVOKED", "readback: durably blocked");

    // New binary refuses ordinary execution of the REVOKED version.
    let err = svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &pool,
        AdmissionGate::disabled(),
        ExecuteWorkflowTransitionCommand {
            principal_id: PrincipalId::from_uuid(agent),
            idempotency_key: format!("trans-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance),
            expected_workflow_state_version: state_version + 1,
            transition_definition_id: TransitionId::from_uuid(t2),
            submission_payload: Some(json!({
                "acceptanceReport": {
                    "contextRevisionId": ctx.to_string(),
                    "results": [{"id": "C1", "verdict": "PASS"}]
                }
            })),
        },
    )
    .await
    .err()
    .expect("new binary must refuse execution of a REVOKED version");
    assert!(
        matches!(err, svc_workflow::domain::workflow_instance::errors::ExecuteWorkflowTransitionError::DefinitionVersionRevoked),
        "{err:?}"
    );

    // ------------------------------------------------------------------
    // Old-binary leg: the base-commit server (annotation-ignorant) must ALSO
    // reject ordinary execution of the REVOKED version over real HTTP.
    // ------------------------------------------------------------------
    let Ok(old_binary) = std::env::var("ACB_OLD_BINARY") else {
        eprintln!(
            "NOT_RUN: ACB_OLD_BINARY is not set; the old-binary downgrade leg \
             did not execute. Provide the base-commit server binary path to run it."
        );
        return;
    };
    if !std::path::Path::new(&old_binary).exists() {
        panic!("ACB_OLD_BINARY is set but the file does not exist: {old_binary}");
    }

    let jwks = common::MockJwksServer::start().await;
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let db_url = common::test_database_url();
    let mut child = std::process::Command::new(&old_binary)
        .env("DATABASE_URL", &db_url)
        .env("WORKFLOW_BIND_ADDR", "127.0.0.1")
        .env("WORKFLOW_PORT", port.to_string())
        .env("WORKFLOW_JWKS_URL", &jwks.url)
        .env("WORKFLOW_JWT_ISSUER", "auth-service")
        .env("WORKFLOW_JWT_AUDIENCE", "svc-workflow")
        .env("AUTH_V1_CANARY_ENABLED", "true")
        .env("AUTH_V1_CANARY_WRITE_ENABLED", "true")
        .env("AUTH_V1_CANARY_ALLOWED_CLIENT_ID", "test-client")
        .env(
            "WORKFLOW_PROVISIONING_PRINCIPAL_IDS",
            owner.to_string(),
        )
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn old binary");

    let token = common::v1_token(
        agent,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &jwks.key_pair,
    );
    let outcome = run_old_binary_leg(port, token, instance, t2, ctx).await;
    let _ = child.kill();
    let _ = child.wait();
    outcome.expect("old-binary leg");
}

async fn run_old_binary_leg(
    port: u16,
    token: String,
    instance: Uuid,
    t2: Uuid,
    ctx: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    // Wait for the old server to come up.
    let base = format!("http://127.0.0.1:{port}");
    let client = reqwest::Client::new();
    let mut up = false;
    for _ in 0..120 {
        if let Ok(resp) = client.get(format!("{base}/healthz")).send().await {
            if resp.status().is_success() {
                up = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    assert!(up, "old binary did not become healthy on port {port}");

    // Ordinary execution attempt over the OLD binary's HTTP API with a fully
    // correct acceptance report: rejection must come from the REVOKED status.
    let payload = json!({
        "transitionDefinitionId": t2,
        "expectedWorkflowStateVersion": 2,
        "submissionPayload": {
            "acceptanceReport": {
                "contextRevisionId": ctx.to_string(),
                "results": [{"id": "C1", "verdict": "PASS"}]
            }
        }
    });
    let resp = client
        .post(format!(
            "{base}/internal/v1/workflow-instances/{instance}/transitions"
        ))
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", Uuid::new_v4().to_string())
        .header("content-type", "application/json")
        .json(&payload)
        .send()
        .await?;
    let status = resp.status().as_u16();
    let body: Value = resp.json().await.unwrap_or(json!({}));
    assert!(
        (400..500).contains(&status),
        "old binary must reject ordinary execution of a REVOKED version; got {status} {body}"
    );
    let body_str = body.to_string();
    assert!(
        body_str.contains("definition_version_revoked"),
        "expected definition_version_revoked rejection, got: {body}"
    );
    Ok(())
}

