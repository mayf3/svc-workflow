//! HTTP integration tests for the GLOBAL_WORKFLOW_AUDITOR governance-read
//! role (2026-10-22 owner decision; mayf3/svc-workflow#80).
//!
//! Covers:
//! - auditor reads instance detail across domains (full visibility via
//!   `classify_visibility`, previously DOMAIN_OWNER/participant-only)
//! - auditor reads submission history cross-domain
//! - auditor reads domain definitions (list / detail) cross-domain
//! - auditor sees the global instance summary list
//! - auditor gains NO write powers (transition / cancel / publish denied)
//! - principal without the role is unchanged (cross-domain reads denied)
//! - provisioning closed set accepts GLOBAL_WORKFLOW_AUDITOR and rejects
//!   unknown role keys
#![allow(clippy::needless_borrow)]
#![allow(unused_imports, unused_variables)]

#[path = "common/mod.rs"]
mod common;

use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use svc_workflow::http::{self, AppState, HttpConfig};

fn build_app(pool: sqlx::PgPool, jwks_url: &str, admin_ids: Vec<Uuid>) -> axum::Router {
    let config = HttpConfig {
        admission: svc_workflow::auth::admission::AdmissionConfig::disabled(),
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
        provisioning_config: ProvisioningConfig::new(
            admin_ids
                .into_iter()
                .map(svc_workflow::domain::ids::PrincipalId::from_uuid)
                .collect(),
        ),
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

fn direct_token(subject: Uuid, scope: &str, key_pair: &common::RsaTestKeyPair) -> String {
    common::v1_token(subject, scope, "test-client", 300, key_pair)
}

async fn do_get(app: axum::Router, path: &str, token: &str) -> (u16, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

async fn do_post(
    app: axum::Router,
    path: &str,
    token: &str,
    body: Value,
    idem_key: &str,
) -> (u16, Value) {
    let req = Request::builder()
        .method("POST")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", idem_key)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

async fn do_put(
    app: axum::Router,
    path: &str,
    token: &str,
    body: Value,
    idem_key: &str,
) -> (u16, Value) {
    let req = Request::builder()
        .method("PUT")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", idem_key)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

// ---------------------------------------------------------------------------
// Seed helpers (direct SQL, mirroring 23_global_coordinator.rs)
// ---------------------------------------------------------------------------

async fn seed_published_definition(pool: &PgPool, domain_id: Uuid) -> (Uuid, Uuid, String) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("auditor-test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Auditor Test Def')",
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

    (def_id, ver_id, def_key)
}

async fn create_instance(
    pool: &PgPool,
    creator_id: Uuid,
    domain_id: Uuid,
    definition_version_id: Uuid,
    title: &str,
) -> Uuid {
    let result = svc_workflow::application::workflow_instance::create::create_workflow_instance(
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
            context_payload: serde_json::json!({ "title": title }),
        },
    )
    .await
    .expect("create instance should succeed");
    result.workflow_instance_id
}

async fn seed_agent(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'Test Agent', NULL, TRUE)",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("insert agent");
    id
}

async fn grant_domain_membership(pool: &PgPool, domain_id: Uuid, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1, $2, $3, 'DOMAIN_MEMBER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("grant domain membership");
}

async fn grant_global_auditor(pool: &PgPool, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO global_role_bindings (binding_id, principal_id, role_key, enabled) VALUES ($1, $2, 'GLOBAL_WORKFLOW_AUDITOR', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("grant global auditor");
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Auditor reads cross-domain instance detail, submission history and
/// definitions; cannot write anything; a role-less principal stays denied.
#[tokio::test]
async fn auditor_full_cross_domain_read_and_no_write() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    // Two unrelated domains, each with a published definition and one
    // instance. The auditor is a member of NEITHER domain.
    let (owner_a, domain_a) = common::seed_principal_domain_with_owner(&pool).await;
    let (_owner_b, domain_b) = common::seed_principal_domain_with_owner(&pool).await;
    let (def_a, ver_a, _key_a) = seed_published_definition(&pool, domain_a).await;
    let (_def_b, ver_b, _key_b) = seed_published_definition(&pool, domain_b).await;
    grant_domain_membership(&pool, domain_a, owner_a).await;
    grant_domain_membership(&pool, domain_b, owner_a).await;
    let inst_a = create_instance(&pool, owner_a, domain_a, ver_a, "auditor read a").await;
    let _inst_b = create_instance(&pool, owner_a, domain_b, ver_b, "auditor read b").await;

    let auditor = seed_agent(&pool).await;
    let outsider = seed_agent(&pool).await;
    grant_global_auditor(&pool, auditor).await;

    let app = build_app(pool.clone(), &mock.url, vec![]);
    let t_auditor = direct_token(auditor, "workflow.read", &mock.key_pair);
    let t_outsider = direct_token(outsider, "workflow.read", &mock.key_pair);
    let t_creator = direct_token(owner_a, "workflow.read", &mock.key_pair);

    // 1. Cross-domain instance detail: 200 for auditor, hidden for outsider.
    let (s, body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_a}"),
        &t_auditor,
    )
    .await;
    assert_eq!(s, 200, "auditor detail cross-domain: {body}");
    let (s, _body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_a}"),
        &t_outsider,
    )
    .await;
    assert!(
        s == 403 || s == 404,
        "outsider must stay denied without the role, got {s}"
    );

    // 2. Cross-domain submission history (empty but authorized).
    let (s, body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_a}/submissions"),
        &t_auditor,
    )
    .await;
    assert_eq!(s, 200, "auditor submission history cross-domain: {body}");
    assert_eq!(body["items"].as_array().map(Vec::len), Some(0));

    // 3. Cross-domain definition reads.
    let (s, body) = do_get(
        app.clone(),
        &format!("/internal/v1/domains/{domain_a}/definitions"),
        &t_auditor,
    )
    .await;
    assert_eq!(s, 200, "auditor definition list cross-domain: {body}");
    assert_eq!(
        body["items"].as_array().map(Vec::len),
        Some(1),
        "definition list contents: {body}"
    );
    let (s, _body) = do_get(
        app.clone(),
        &format!("/internal/v1/domains/{domain_a}/definitions/{def_a}"),
        &t_auditor,
    )
    .await;
    assert_eq!(s, 200, "auditor definition detail cross-domain");

    // 4. Auditor sees the global summary list too.
    let (s, body) = do_get(
        app.clone(),
        "/internal/v1/workflow-instances/global",
        &t_auditor,
    )
    .await;
    assert_eq!(s, 200, "auditor global list: {body}");

    // 5. NO write powers: transition on someone else's instance stays denied.
    let (s, _body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_a}/transitions"),
        &t_auditor,
        json!({"transitionDefinitionId": null, "submissionPayload": {"summary": "auditor write attempt"}}),
        &format!("auditor-transition-{}", Uuid::new_v4()),
    )
    .await;
    assert_ne!(s, 200, "auditor must not transition foreign instances");

    // 6. NO write powers: cancel + definition publish stay denied.
    let (s, _body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_a}/cancel"),
        &t_auditor,
        json!({"reason": "auditor write attempt"}),
        &format!("auditor-cancel-{}", Uuid::new_v4()),
    )
    .await;
    assert_ne!(s, 200, "auditor must not cancel foreign instances");

    let (s, _body) = do_post(
        app.clone(),
        &format!("/internal/v1/domains/{domain_a}/definitions/{def_a}/publish"),
        &t_auditor,
        json!({"definitionVersionId": ver_a}),
        &format!("auditor-publish-{}", Uuid::new_v4()),
    )
    .await;
    assert_ne!(s, 200, "auditor must not publish definitions");

    // 7. Domain boundary for non-auditor reads is unchanged.
    let (s, _body) = do_get(
        app.clone(),
        &format!("/internal/v1/domains/{domain_a}/definitions"),
        &t_outsider,
    )
    .await;
    assert_ne!(s, 200, "outsider definition read must stay denied");

    // The creating owner (plain participant, no global role) is unaffected.
    let (s, _body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst_a}"),
        &t_creator,
    )
    .await;
    assert_eq!(s, 200, "creator detail must keep working");
}

/// Provisioning closed set: GLOBAL_WORKFLOW_AUDITOR accepted, junk denied.
#[tokio::test]
async fn auditor_role_key_provisioning_lifecycle() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let admin = seed_agent(&pool).await;
    let target = seed_agent(&pool).await;
    let app = build_app(pool.clone(), &mock.url, vec![admin]);
    let t_admin = direct_token(admin, "workflow.admin", &mock.key_pair);

    let (s, body) = do_put(
        app.clone(),
        &format!("/internal/v1/admin/global-role-bindings/{target}"),
        &t_admin,
        json!({"roleKey": "GLOBAL_WORKFLOW_AUDITOR", "enabled": true}),
        &format!("auditor-provision-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(s, 200, "auditor provisioning accepted: {body}");

    let (s, _body) = do_put(
        app.clone(),
        &format!("/internal/v1/admin/global-role-bindings/{target}"),
        &t_admin,
        json!({"roleKey": "GLOBAL_WORKFLOW_TOTALLY_FAKE", "enabled": true}),
        &format!("auditor-provision-bad-{}", Uuid::new_v4()),
    )
    .await;
    assert_eq!(s, 422, "unknown role key must be rejected");
}
