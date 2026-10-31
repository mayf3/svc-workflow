//! HTTP integration tests for canonical-assignee continuity (P1 2026-10-22:
//! my_tasks projection, instance read visibility and transition actor
//! authorization must agree on one identity answer under the identity
//! repair lineage, migration 0025 / CTR-CIR V2).
//!
//! Fixture mirrors the production incident: an instance's stored current
//! visit assignee is the STALE (pre-cutover) principal; the actor
//! authenticates as its canonical successor recorded in
//! `workflow_identity_successor_lines`. Stored assignee UUID is never
//! rewritten.
//!
//! Covers:
//! - canonical successor reads instance detail (previously 404)
//! - outgoing transition executable_for_actor=true, blocked_reason absent
//!   (previously ACTOR_NOT_CURRENT_ASSIGNEE)
//! - canonical successor reads submission history (previously 404)
//! - canonical successor executes the transition (previously
//!   principal_not_assignee)
//! - a principal with no lineage edge stays denied on every surface
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

async fn seed_identity_line(pool: &PgPool, source: Uuid, successor: Uuid) {
    sqlx::query(
        "INSERT INTO workflow_identity_successor_lines
           (id, source_principal_id, successor_principal_id, legacy_agent_id,
            canonical_agent_id, classification, evidence, repair_reason)
         VALUES ($1, $2, $3, 'legacy-agent', 'agt-canonical-test', 'STALE_PRINCIPAL_WITH_UNIQUE_REPAIR', '\"test fixture\"', '\"test fixture\"')",
    )
    .bind(Uuid::new_v4())
    .bind(source)
    .bind(successor)
    .execute(pool)
    .await
    .expect("insert identity successor line");
}

async fn seed_published_definition(pool: &PgPool, domain_id: Uuid) -> (Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("canon-test-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Canonical Test Def')",
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

    (def_id, ver_id, trans_id)
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

#[tokio::test]
async fn canonical_assignee_reads_and_advances_where_stale_uuid_stored() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;

    // The creator IS the stale (pre-cutover) principal, so the DRAFT visit
    // is born with the stale UUID stored as assignee — exactly the
    // production stored state — without mutating any visit row (the table
    // is append-only/immutable).
    let stale = seed_agent(&pool).await;
    let canonical = seed_agent(&pool).await; // actor (canonical successor)
    let outsider = seed_agent(&pool).await;
    seed_identity_line(&pool, stale, canonical).await;

    let domain = common::seed_principal_domain_with_owner(&pool).await.1;
    let (_def_id, ver_id, trans_id) = seed_published_definition(&pool, domain).await;
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1, $2, $3, 'DOMAIN_MEMBER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain)
    .bind(stale)
    .execute(&pool)
    .await
    .expect("grant stale principal domain membership");
    let inst = create_instance(&pool, stale, domain, ver_id, "canon continuity").await;

    let app = build_app(pool.clone(), &mock.url, vec![]);
    let t_canon_read = direct_token(canonical, "workflow.read", &mock.key_pair);
    let t_canon_exec = direct_token(canonical, "workflow.execute", &mock.key_pair);
    let t_outsider_read = direct_token(outsider, "workflow.read", &mock.key_pair);
    let t_outsider_exec = direct_token(outsider, "workflow.execute", &mock.key_pair);

    // 1. Canonical successor reads the instance detail (previously 404).
    let (s, body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst}"),
        &t_canon_read,
    )
    .await;
    assert_eq!(s, 200, "canonical successor detail: {body}");

    // 2. Outgoing transition executable and unblocked (previously
    //    ACTOR_NOT_CURRENT_ASSIGNEE).
    let transitions = body["detail"]["outgoing_transitions"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let primary = transitions
        .iter()
        .find(|t| t["transition_id"] == json!(trans_id.to_string()))
        .expect("primary advance listed");
    assert_eq!(
        primary["executable_for_actor"], json!(true),
        "primary must be executable for canonical successor: {primary}"
    );
    assert_eq!(primary["blocked_reason"], Value::Null);

    // 3. Submission history readable (previously 404).
    let (s, body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst}/submissions"),
        &t_canon_read,
    )
    .await;
    assert_eq!(s, 200, "canonical successor submission history: {body}");

    // 4. Principal without a lineage edge stays denied on every surface.
    let (s, _body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst}"),
        &t_outsider_read,
    )
    .await;
    assert!(s == 403 || s == 404, "outsider detail must stay denied, got {s}");
    let (s, _body) = do_get(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst}/submissions"),
        &t_outsider_read,
    )
    .await;
    assert!(s == 403 || s == 404, "outsider submissions must stay denied, got {s}");
    let (s, body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst}/transitions"),
        &t_outsider_exec,
        json!({
            "transitionDefinitionId": trans_id.to_string(),
            "expectedWorkflowStateVersion": 1,
            "submissionPayload": null
        }),
        "canon-outsider-transition-1",
    )
    .await;
    assert_eq!(s, 403, "outsider transition must be denied: {body}");
    assert_eq!(body["error"]["code"], "principal_not_assignee");

    // 5. Canonical successor executes the transition (previously
    //    principal_not_assignee).
    let (s, body) = do_post(
        app.clone(),
        &format!("/internal/v1/workflow-instances/{inst}/transitions"),
        &t_canon_exec,
        json!({
            "transitionDefinitionId": trans_id.to_string(),
            "expectedWorkflowStateVersion": 1,
            "submissionPayload": null
        }),
        "canon-transition-1",
    )
    .await;
    assert_eq!(s, 200, "canonical successor transition: {body}");

    // 6. Instance actually advanced to TERMINAL (verified in the store:
    //    post-terminal detail re-reads follow the pre-existing
    //    historical-participant restricted semantics, out of L0 scope).
    let node_type: String = sqlx::query_scalar(
        "SELECT n.node_type::text
         FROM workflow_instances wi
         JOIN workflow_node_visits v ON v.node_visit_id = wi.current_node_visit_id
         JOIN workflow_node_definitions n ON n.node_id = v.node_id
         WHERE wi.workflow_instance_id = $1",
    )
    .bind(inst)
    .fetch_one(&pool)
    .await
    .expect("state after advance");
    assert_eq!(node_type, "TERMINAL", "instance must have advanced");
}
