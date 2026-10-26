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
//! F3 hardening (review ACB-SOURCE-SNAPSHOT-A1-INDEPENDENT-REVIEW):
//! - explicit isolated `TEST_DATABASE_URL` is REQUIRED; the shared default
//!   database and any default-port PostgreSQL are refused outright;
//! - the old binary is launched with `env_clear` plus a minimal, closed
//!   variable set — Forum sync and canonical-identity admission are
//!   explicitly disabled and can never be enabled by an inherited or
//!   worktree-`.env` value;
//! - the child runs in an owned temp cwd containing an empty `.env` and a
//!   copy of `migrations/` (the server loads both relative to cwd);
//! - the child is wrapped in an RAII guard so every failure/panic path kills
//!   and reaps it;
//! - the old binary's SHA-256 is pinned, binding the executable to its
//!   declared source commit.
//!
//! The instance used for the old-binary leg is `NON_BUSINESS_TEST` (the
//! actor is its enabled DOMAIN_OWNER), so no Forum binding/outbox facts are
//! seeded in the first place.
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

use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
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

/// SHA-256 of the archived pre-implementation server binary used for the
/// downgrade leg. The hash binds the executable to its declared source
/// identity; a binary that does not match is refused, never silently used.
const OLD_BINARY_SHA256: &str =
    "d97aa01913b9324675af45b2384075bd542c4c0d91d8fedc0e463c6857ec20d5";
/// Source commit the archived old binary was built from (pre-implementation
/// deployment line; it does not contain this feature).
const OLD_BINARY_SOURCE_HEAD: &str = "b2e4736f4f7e6350bcf1db44e016d583b20e2b21";

// ============================================================================
// Isolation primitives (pure; covered by the tests below)
// ============================================================================

/// Require an explicitly configured, non-default, non-default-port test
/// database. The shared default (`localhost:5432/svc_workflow`) and any
/// default-port PostgreSQL are refused; this suite may only run against the
/// dedicated isolated instance.
fn ensure_explicit_isolated_db(url: Option<String>) -> String {
    let url = url.unwrap_or_default();
    assert!(
        !url.trim().is_empty(),
        "TEST_DATABASE_URL must be set explicitly; refusing to fall back to the \
         shared default database"
    );
    let lower = url.to_lowercase();
    assert!(
        !lower.contains("@localhost:5432/") && !lower.contains("@127.0.0.1:5432/"),
        "TEST_DATABASE_URL must not point at a default-port PostgreSQL \
         (localhost:5432 / 127.0.0.1:5432); use the dedicated isolated instance"
    );
    assert!(
        !lower.ends_with("/svc_workflow"),
        "TEST_DATABASE_URL must not use the shared default database name \
         'svc_workflow'"
    );
    url
}

fn sha256_file(path: &str) -> String {
    let bytes = std::fs::read(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    hex::encode(hasher.finalize())
}

/// Create an owned temp cwd for the old binary: empty `.env` (the server
/// runs dotenvy::dotenv() at startup) and a copy of `migrations/` (the
/// server loads migrations relative to cwd).
fn prepare_owned_cwd() -> PathBuf {
    let dir = std::env::temp_dir().join(format!("acb-old-binary-cwd-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).expect("create owned cwd");
    std::fs::write(dir.join(".env"), "").expect("write empty .env");
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("migrations");
    let dst = dir.join("migrations");
    std::fs::create_dir_all(&dst).expect("create migrations dir");
    for entry in std::fs::read_dir(&src).expect("read migrations source") {
        let entry = entry.expect("migrations entry");
        if entry.file_type().unwrap().is_file() {
            std::fs::copy(entry.path(), dst.join(entry.file_name()))
                .expect("copy migration file");
        }
    }
    dir
}

/// Build the old-binary command with a CLOSED environment: `env_clear` plus
/// the minimal variables the server needs, with Forum sync and canonical
/// identity admission explicitly disabled. No parent or `.env` value can
/// enable an integration.
fn build_old_binary_command(
    binary: &str,
    db_url: &str,
    port: u16,
    jwks_url: &str,
    admin_principal: Uuid,
    cwd: &Path,
) -> Command {
    let mut cmd = Command::new(binary);
    cmd.env_clear()
        .env("DATABASE_URL", db_url)
        .env("WORKFLOW_BIND_ADDR", "127.0.0.1")
        .env("WORKFLOW_PORT", port.to_string())
        .env("WORKFLOW_JWKS_URL", jwks_url)
        .env("WORKFLOW_JWT_ISSUER", "auth-service")
        .env("WORKFLOW_JWT_AUDIENCE", "svc-workflow")
        .env("AUTH_V1_CANARY_ENABLED", "true")
        .env("AUTH_V1_CANARY_WRITE_ENABLED", "true")
        .env("AUTH_V1_CANARY_ALLOWED_CLIENT_ID", "test-client")
        .env("WORKFLOW_PROVISIONING_PRINCIPAL_IDS", admin_principal.to_string())
        // Explicit disablement: both integrations are opt-in via env; pin
        // the off values so inheritance bugs cannot flip them on.
        .env("WORKFLOW_ADMISSION_ENABLED", "0")
        .env("WORKFLOW_FORUM_SYNC_ENABLED", "false")
        .current_dir(cwd);
    cmd
}

/// RAII guard: the old server is killed and reaped on every path, including
/// assertion failures and panics inside the leg.
struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn minimal_env_keys() -> &'static [&'static str] {
    &[
        "DATABASE_URL",
        "WORKFLOW_BIND_ADDR",
        "WORKFLOW_PORT",
        "WORKFLOW_JWKS_URL",
        "WORKFLOW_JWT_ISSUER",
        "WORKFLOW_JWT_AUDIENCE",
        "AUTH_V1_CANARY_ENABLED",
        "AUTH_V1_CANARY_WRITE_ENABLED",
        "AUTH_V1_CANARY_ALLOWED_CLIENT_ID",
        "WORKFLOW_PROVISIONING_PRINCIPAL_IDS",
        "WORKFLOW_ADMISSION_ENABLED",
        "WORKFLOW_FORUM_SYNC_ENABLED",
    ]
}

// ============================================================================
// Isolation coverage (no server, no database)
// ============================================================================

#[test]
fn isolated_db_guard_rejects_shared_defaults() {
    ensure_explicit_isolated_db(Some(
        "postgres://postgres:postgres@127.0.0.1:55432/svc_workflow_acb_impl_27oct".to_string(),
    ));
    for hostile in [
        None,
        Some("".to_string()),
        Some("postgres://postgres:postgres@localhost:5432/svc_workflow".to_string()),
        Some("postgres://postgres:postgres@127.0.0.1:5432/svc_workflow".to_string()),
        Some("postgres://postgres:postgres@127.0.0.1:55432/svc_workflow".to_string()),
    ] {
        let result = std::panic::catch_unwind(|| ensure_explicit_isolated_db(hostile.clone()));
        assert!(result.is_err(), "hostile URL must be refused: {hostile:?}");
    }
}

#[test]
fn old_binary_command_isolation_is_closed() {
    let cwd = prepare_owned_cwd();
    let admin = Uuid::new_v4();
    let cmd = build_old_binary_command(
        "/definitely/not/started",
        "postgres://postgres:postgres@127.0.0.1:55432/svc_workflow_acb_impl_27oct",
        19999,
        "http://127.0.0.1:1/jwks",
        admin,
        &cwd,
    );

    // The environment is exactly the minimal closed set — nothing inherited.
    let keys: std::collections::BTreeSet<String> = cmd
        .get_envs()
        .filter_map(|(k, _)| k.to_str().map(str::to_string))
        .collect();
    let expected: std::collections::BTreeSet<String> = minimal_env_keys()
        .iter()
        .map(|k| k.to_string())
        .collect();
    assert_eq!(
        keys, expected,
        "child env must be exactly the minimal closed set"
    );

    // Integrations are pinned off; no integration URL key exists at all.
    let value_of = |name: &str| {
        cmd.get_envs()
            .find(|(k, _)| k.to_str() == Some(name))
            .and_then(|(_, v)| v.map(|v| v.to_string_lossy().into_owned()))
    };
    assert_eq!(value_of("WORKFLOW_ADMISSION_ENABLED").as_deref(), Some("0"));
    assert_eq!(
        value_of("WORKFLOW_FORUM_SYNC_ENABLED").as_deref(),
        Some("false")
    );
    for integration_key in [
        "WORKFLOW_FORUM_AUTH_BASE_URL",
        "WORKFLOW_FORUM_CLIENT_ID",
        "WORKFLOW_FORUM_CLIENT_SECRET",
        "WORKFLOW_ADMISSION_AUTH_BASE_URL",
        "WORKFLOW_ADMISSION_CORE_BASE_URL",
        "WORKFLOW_EXECUTION_KICK_URL",
        "AUTH_V1_CANARY_ALLOWED_SUB",
        "AUTH_V1_CANARY_ALLOWED_DELEGATING_SUB",
    ] {
        assert!(
            !keys.contains(integration_key),
            "{integration_key} must be absent from the child env"
        );
    }

    // The cwd is owned: empty `.env`, migrations copied, not the worktree.
    let actual_cwd = cmd.get_current_dir().expect("cwd set").to_path_buf();
    assert_eq!(actual_cwd, cwd);
    assert_ne!(
        actual_cwd,
        PathBuf::from(env!("CARGO_MANIFEST_DIR")),
        "child must not run in the worktree"
    );
    let env_meta = std::fs::metadata(cwd.join(".env")).expect(".env exists");
    assert_eq!(env_meta.len(), 0, ".env must be empty");
    assert!(
        std::fs::metadata(cwd.join("migrations")).unwrap().is_dir(),
        "migrations copied into owned cwd"
    );
    let migrated = std::fs::read_dir(cwd.join("migrations")).unwrap().count();
    assert!(
        migrated >= 28,
        "all migration files copied (found {migrated})"
    );

    let _ = std::fs::remove_dir_all(&cwd);
}

// ============================================================================
// Fixtures (definition via the real publish service; NON_BUSINESS_TEST work)
// ============================================================================

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

/// Seed the model3 DRAFT graph and publish through the real service with an
/// enabled DOMAIN_OWNER actor. Returns (version_id, start->work, work->done).
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

// ============================================================================
// The downgrade-gate test
// ============================================================================

#[tokio::test]
async fn ctr6_revoked_optin_version_rejected_by_new_and_old_binary() {
    // F3: explicit isolated database — never the shared default.
    let db_url = ensure_explicit_isolated_db(std::env::var("TEST_DATABASE_URL").ok());
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let agent = common::seed_second_principal(&pool).await;
    let (ver_id, t1, t2) = seed_and_publish_annotated(&pool, domain, agent, owner).await;

    // Create NON_BUSINESS_TEST work (the owner is this domain's enabled
    // DOMAIN_OWNER): no Forum binding, no Forum event, no execution kick —
    // the old binary therefore has nothing integration-shaped to touch.
    let created = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool,
        AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner),
            idempotency_key: format!("create-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::NonBusinessTest,
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
    // Old-binary leg: the pre-implementation server (annotation-ignorant)
    // must ALSO reject ordinary execution of the REVOKED version over real
    // HTTP. Identity is pinned by SHA-256 before spawn.
    // ------------------------------------------------------------------
    let Ok(old_binary) = std::env::var("ACB_OLD_BINARY") else {
        eprintln!(
            "NOT_RUN: ACB_OLD_BINARY is not set; the old-binary downgrade leg \
             did not execute. Provide the base-commit server binary path to run it."
        );
        return;
    };
    assert!(
        Path::new(&old_binary).exists(),
        "ACB_OLD_BINARY is set but the file does not exist: {old_binary}"
    );
    let actual_hash = sha256_file(&old_binary);
    assert_eq!(
        actual_hash, OLD_BINARY_SHA256,
        "old binary identity mismatch: {old_binary} has sha256 {actual_hash}, \
         expected {OLD_BINARY_SHA256} (source {OLD_BINARY_SOURCE_HEAD})"
    );

    let jwks = common::MockJwksServer::start().await;
    let port = {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.local_addr().unwrap().port()
    };
    let cwd = prepare_owned_cwd();
    let mut child = ChildGuard(
        build_old_binary_command(&old_binary, &db_url, port, &jwks.url, owner, &cwd)
            .spawn()
            .expect("spawn old binary"),
    );

    let token = common::v1_token(
        agent,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &jwks.key_pair,
    );
    run_old_binary_leg(port, token, instance, t2, ctx).await;
    // ChildGuard drops here (and on every panic above): kill + wait.
    drop(child);
    let _ = std::fs::remove_dir_all(&cwd);
}

async fn run_old_binary_leg(port: u16, token: String, instance: Uuid, t2: Uuid, ctx: Uuid) {
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
        .await
        .expect("old binary HTTP request");
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
}
