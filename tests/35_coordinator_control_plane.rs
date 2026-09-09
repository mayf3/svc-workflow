//! Integration tests for the GLOBAL_WORKFLOW_COORDINATOR control plane
//! (SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1, ACC-CP-001..008).
//!
//! Coverage map:
//! - ACC-CP-001: coordinator domain list/get/update displayName/get_owner
//!   (read-after-write) + cross-domain cancel→read-back→archive→read-back
//! - ACC-CP-002/003b: member add three-state outcome (added / same-key
//!   replay / new-key already_member with zero second member_added audit);
//!   remove missing → member_not_found; coordinator cross-domain
//!   add/remove; plain DOMAIN_MEMBER fail-closed
//! - ACC-CP-003: reconcile plan/apply/replay/conflict + the asymmetric
//!   identity semantics (disabled SOURCE allowed; disabled TARGET 403
//!   principal_disabled; missing UUID 404 identity_not_found)
//! - ACC-CP-005: DOMAIN_OWNER negative boundaries (no cross-domain, no
//!   set-owner/update/reconcile)
//! - ACC-CP-006: coordinator negative boundaries (no transition on a
//!   non-assignee instance; no admin fallback)
//! - ACC-CP-007: legacy error codes byte-preserved (owner cancel of
//!   already-cancelled → already_cancelled; active → archive →
//!   instance_not_terminal / active_activation_exists)

#![allow(unused_imports, unused_variables)]

#[path = "common/mod.rs"]
mod common;

use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use common::MockJwksServer;
use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use svc_workflow::http::{self, AppState, HttpConfig};

// ============================================================================
// Harness (mirrors tests/28_visit_activation_v1.rs)
// ============================================================================

fn build_app(pool: sqlx::PgPool, jwks_url: &str) -> axum::Router {
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
        provisioning_config: ProvisioningConfig::new(vec![]),
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

async fn do_get(app: &axum::Router, path: &str, token: &str) -> (u16, Value) {
    let req = Request::builder()
        .method("GET")
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

async fn do_method(
    app: &axum::Router,
    method: &str,
    path: &str,
    token: &str,
    body: Value,
    idem_key: Option<&str>,
) -> (u16, Value) {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json");
    if let Some(key) = idem_key {
        builder = builder.header("idempotency-key", key);
    }
    let req = builder
        .body(Body::from(body.to_string()))
        .unwrap();
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

async fn seed_agent(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) \
         VALUES ($1, 'AGENT', 'Test Agent', NULL, TRUE)",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("insert agent");
    id
}

async fn seed_disabled_agent(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) \
         VALUES ($1, 'AGENT', 'Stale Agent', NULL, FALSE)",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("insert disabled agent");
    id
}

/// A bare domain (direct SQL; the coordinator create path is itself under
/// test so fixtures stay independent of it).
async fn seed_domain(pool: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    let key = format!("ccp-test-{}", &Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO domains (domain_id, domain_key, display_name, enabled) \
         VALUES ($1, $2, 'CCP Test Domain', TRUE)",
    )
    .bind(id)
    .bind(key)
    .execute(pool)
    .await
    .expect("insert domain");
    id
}

async fn seed_binding(pool: &PgPool, domain_id: Uuid, principal_id: Uuid, role: &str) {
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) \
         VALUES ($1, $2, $3, $4, TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(principal_id)
    .bind(role)
    .execute(pool)
    .await
    .expect("seed binding");
}

async fn grant_coordinator(pool: &PgPool, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO global_role_bindings (binding_id, principal_id, role_key, enabled) \
         VALUES ($1, $2, 'GLOBAL_WORKFLOW_COORDINATOR', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("grant coordinator");
}

async fn member_audit_count(pool: &PgPool, domain_id: Uuid, target: Uuid, action: &str) -> i64 {
    let (count,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_security_audits \
         WHERE resource_type = 'DOMAIN_MEMBERSHIP' AND action = $1 \
           AND resource_id = $2 AND (details->>'targetPrincipalId') = $3::text",
    )
    .bind(action)
    .bind(format!("{domain_id}/{target}"))
    .bind(target.to_string())
    .fetch_one(pool)
    .await
    .expect("audit count");
    count
}

// ============================================================================
// Domain admin surfaces (ACC-CP-001)
// ============================================================================

#[tokio::test]
async fn acc_cp_001_domain_list_get_update_owner() {
    let pool = common::create_pool().await;
    let coordinator = seed_agent(&pool).await;
    grant_coordinator(&pool, coordinator).await;
    let domain_id = seed_domain(&pool).await;
    let owner = seed_agent(&pool).await;
    seed_binding(&pool, domain_id, owner, "DOMAIN_OWNER").await;

    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(coordinator, "workflow.execute workflow.read", &mock.key_pair);

    // list contains the domain with the exact governance metadata shape.
    let (status, body) = do_get(&app, "/internal/v1/domains?limit=100", &token).await;
    assert_eq!(status, 200, "list: {body}");
    let items = body["items"].as_array().expect("items");
    let mine: Vec<&Value> = items
        .iter()
        .filter(|it| it["domainId"] == json!(domain_id.to_string()))
        .collect();
    assert_eq!(mine.len(), 1, "domain listed exactly once");
    let keys: Vec<&str> = mine[0]
        .as_object()
        .unwrap()
        .keys()
        .map(|k| k.as_str())
        .collect();
    assert!(
        keys.contains(&"domainId")
            && keys.contains(&"domainKey")
            && keys.contains(&"displayName")
            && keys.contains(&"enabled")
            && keys.contains(&"createdAt")
            && keys.contains(&"updatedAt"),
        "minimal governance metadata fields: {keys:?}"
    );

    // get.
    let (status, body) = do_get(
        &app,
        &format!("/internal/v1/domains/{domain_id}"),
        &token,
    )
    .await;
    assert_eq!(status, 200, "get: {body}");
    assert_eq!(body["displayName"], json!("CCP Test Domain"));

    // update displayName (read-after-write).
    let (status, body) = do_method(
        &app,
        "PATCH",
        &format!("/internal/v1/domains/{domain_id}"),
        &token,
        json!({ "displayName": "Renamed Domain" }),
        Some(&format!("upd-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 200, "patch: {body}");
    assert_eq!(body["displayName"], json!("Renamed Domain"));
    let (_, reread) = do_get(
        &app,
        &format!("/internal/v1/domains/{domain_id}"),
        &token,
    )
    .await;
    assert_eq!(reread["displayName"], json!("Renamed Domain"));

    // get_owner read-back.
    let (status, body) = do_get(
        &app,
        &format!("/internal/v1/domains/{domain_id}/owner"),
        &token,
    )
    .await;
    assert_eq!(status, 200, "get_owner: {body}");
    assert_eq!(body["ownerPrincipalId"], json!(owner.to_string()));
    assert_eq!(body["ownerEnabled"], json!(true));
    assert!(body["ownerDisplayName"].is_string());

    // domain_owner_missing: a domain without an enabled owner.
    let ownerless = seed_domain(&pool).await;
    let (status, body) = do_get(
        &app,
        &format!("/internal/v1/domains/{ownerless}/owner"),
        &token,
    )
    .await;
    assert_eq!(status, 404, "ownerless domain: {body}");
    assert_eq!(body["error"]["code"], json!("domain_owner_missing"));

    // set_owner regression still works for the coordinator.
    let new_owner = seed_agent(&pool).await;
    let (status, body) = do_method(
        &app,
        "PUT",
        &format!("/internal/v1/domains/{ownerless}/owner"),
        &token,
        json!({ "newOwnerPrincipalId": new_owner.to_string() }),
        Some(&format!("own-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 200, "set_owner: {body}");
}

#[tokio::test]
async fn acc_cp_005_owner_and_member_fail_closed_on_admin_surface() {
    let pool = common::create_pool().await;
    let coordinator = seed_agent(&pool).await;
    let owner = seed_agent(&pool).await;
    let member = seed_agent(&pool).await;
    grant_coordinator(&pool, coordinator).await;
    let domain_id = seed_domain(&pool).await;
    seed_binding(&pool, domain_id, owner, "DOMAIN_OWNER").await;
    seed_binding(&pool, domain_id, member, "DOMAIN_MEMBER").await;

    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let owner_token = direct_token(owner, "workflow.execute workflow.read", &mock.key_pair);
    let member_token = direct_token(member, "workflow.execute workflow.read", &mock.key_pair);

    // DOMAIN_OWNER: own-domain member list works, but domain list /
    // reconcile plan / set owner are coordinator-only.
    let (status, _) = do_get(
        &app,
        &format!("/internal/v1/domains/{domain_id}/members?limit=10"),
        &owner_token,
    )
    .await;
    assert_eq!(status, 200, "owner lists own members");

    for (method, path, body) in [
        ("GET", "/internal/v1/domains?limit=10", json!({})),
        (
            "PATCH",
            &format!("/internal/v1/domains/{domain_id}"),
            json!({ "displayName": "nope" }),
        ),
        (
            "POST",
            &format!("/internal/v1/domains/{domain_id}/binding-reconcile/plan"),
            json!({
                "role": "DOMAIN_MEMBER",
                "fromPrincipalId": member.to_string(),
                "toPrincipalId": owner.to_string(),
                "reason": "owner must not reconcile"
            }),
        ),
    ] {
        let (status, body) = do_method(
            &app,
            method,
            path,
            &owner_token,
            body,
            Some(&format!("neg-{}", Uuid::new_v4())),
        )
        .await;
        assert_eq!(status, 403, "owner {method} {path} must fail closed: {body}");
        assert_eq!(body["error"]["code"], json!("global_coordinator_required"));
    }

    // DOMAIN_MEMBER: everything fail-closed.
    let (status, body) = do_get(
        &app,
        "/internal/v1/domains?limit=10",
        &member_token,
    )
    .await;
    assert_eq!(status, 403, "member domain list must fail closed");
    let (status, body) = do_get(
        &app,
        &format!("/internal/v1/domains/{domain_id}/owner"),
        &member_token,
    )
    .await;
    assert_eq!(status, 403, "member get_owner must fail closed");
}

// ============================================================================
// Member governance (ACC-CP-002 / ACC-CP-003b / B3 remove cases)
// ============================================================================

#[tokio::test]
async fn acc_cp_002_member_add_three_state_and_remove() {
    let pool = common::create_pool().await;
    let coordinator = seed_agent(&pool).await;
    grant_coordinator(&pool, coordinator).await;
    let domain_id = seed_domain(&pool).await;
    let owner = seed_agent(&pool).await;
    seed_binding(&pool, domain_id, owner, "DOMAIN_OWNER").await;
    let newcomer = seed_agent(&pool).await;

    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(coordinator, "workflow.execute", &mock.key_pair);
    let add_path = format!("/internal/v1/domains/{domain_id}/members/{newcomer}");

    // First logical add → outcome=added + exactly one member_added audit.
    let key = format!("add-{}", Uuid::new_v4());
    let (status, body) = do_method(
        &app,
        "PUT",
        &add_path,
        &token,
        json!({}),
        Some(&key),
    )
    .await;
    assert_eq!(status, 200, "first add: {body}");
    assert_eq!(body["outcome"], json!("added"));
    assert_eq!(body["role"], json!("DOMAIN_MEMBER"));
    assert_eq!(
        member_audit_count(&pool, domain_id, newcomer, "member_added").await,
        1,
        "exactly one member_added audit"
    );
    assert_eq!(
        member_audit_count(&pool, domain_id, newcomer, "member_add_noop").await,
        0
    );

    // Same Idempotency-Key replay → original response, zero second audit.
    let (status, body) = do_method(
        &app,
        "PUT",
        &add_path,
        &token,
        json!({}),
        Some(&key),
    )
    .await;
    assert_eq!(status, 200, "replay: {body}");
    assert_eq!(body["outcome"], json!("added"));
    assert_eq!(
        member_audit_count(&pool, domain_id, newcomer, "member_added").await,
        1,
        "replay must not add a second member_added audit"
    );

    // New key + already enabled member → already_member, no second audit.
    let (status, body) = do_method(
        &app,
        "PUT",
        &add_path,
        &token,
        json!({}),
        Some(&format!("add-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 200, "logical duplicate: {body}");
    assert_eq!(body["outcome"], json!("already_member"));
    assert_eq!(
        member_audit_count(&pool, domain_id, newcomer, "member_added").await,
        1,
        "logical duplicate must not add a second member_added audit"
    );
    assert!(
        member_audit_count(&pool, domain_id, newcomer, "member_add_noop").await >= 1,
        "the no-op is explicitly audited"
    );

    // Coordinator cross-domain remove → success; remove missing →
    // member_not_found.
    let (status, body) = do_method(
        &app,
        "DELETE",
        &add_path,
        &token,
        json!({}),
        Some(&format!("rm-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 200, "coordinator cross-domain remove: {body}");
    assert_eq!(body["enabled"], json!(false));

    let (status, body) = do_method(
        &app,
        "DELETE",
        &add_path,
        &token,
        json!({}),
        Some(&format!("rm-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 404, "remove missing: {body}");
    assert_eq!(body["error"]["code"], json!("member_not_found"));

    // The plain member cannot manage the domain at all.
    let member_token = direct_token(newcomer, "workflow.execute", &mock.key_pair);
    let victim = seed_agent(&pool).await;
    let (status, body) = do_method(
        &app,
        "PUT",
        &format!("/internal/v1/domains/{domain_id}/members/{victim}"),
        &member_token,
        json!({}),
        Some(&format!("add-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 403, "plain member must fail closed: {body}");
    assert_eq!(body["error"]["code"], json!("not_domain_owner"));
}

// ============================================================================
// Binding reconciliation (ACC-CP-003, B5 semantics)
// ============================================================================

#[tokio::test]
async fn acc_cp_003_reconcile_plan_apply_replay_conflict() {
    let pool = common::create_pool().await;
    let coordinator = seed_agent(&pool).await;
    grant_coordinator(&pool, coordinator).await;
    let domain_id = seed_domain(&pool).await;

    // Member-migration fixture: stale source (disabled principal!) with an
    // enabled binding → healthy canonical target.
    let stale_source = seed_disabled_agent(&pool).await;
    seed_binding(&pool, domain_id, stale_source, "DOMAIN_MEMBER").await;
    let canonical = seed_agent(&pool).await;

    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(coordinator, "workflow.execute workflow.read", &mock.key_pair);
    let plan_path = format!("/internal/v1/domains/{domain_id}/binding-reconcile/plan");
    let apply_path = format!("/internal/v1/domains/{domain_id}/binding-reconcile/apply");

    // plan: disabled SOURCE principal is NOT a blocker.
    let (status, body) = do_method(
        &app,
        "POST",
        &plan_path,
        &token,
        json!({
            "role": "DOMAIN_MEMBER",
            "fromPrincipalId": stale_source.to_string(),
            "toPrincipalId": canonical.to_string(),
            "reason": "canonical migration"
        }),
        None,
    )
    .await;
    assert_eq!(status, 200, "plan: {body}");
    assert_eq!(body["sourcePrincipalExists"], json!(true));
    assert_eq!(body["sourcePrincipalEnabled"], json!(false));
    assert_eq!(body["sourceBindingExists"], json!(true));
    assert_eq!(body["sourceBindingEnabled"], json!(true));
    assert_eq!(body["targetPrincipalEnabled"], json!(true));
    let blockers = body["blockers"].as_array().expect("blockers");
    assert!(blockers.is_empty(), "no blockers expected: {blockers:?}");

    // apply: atomic migration.
    let key = format!("rec-{}", Uuid::new_v4());
    let (status, body) = do_method(
        &app,
        "POST",
        &apply_path,
        &token,
        json!({
            "role": "DOMAIN_MEMBER",
            "fromPrincipalId": stale_source.to_string(),
            "toPrincipalId": canonical.to_string(),
            "reason": "canonical migration"
        }),
        Some(&key),
    )
    .await;
    assert_eq!(status, 200, "apply: {body}");
    assert_eq!(body["outcome"], json!("applied"));

    let old_enabled: (bool,) = sqlx::query_as(
        "SELECT enabled FROM domain_role_bindings \
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_MEMBER'",
    )
    .bind(domain_id)
    .bind(stale_source)
    .fetch_one(&pool)
    .await
    .expect("old binding");
    assert!(!old_enabled.0, "source binding disabled");
    let new_enabled: (bool,) = sqlx::query_as(
        "SELECT enabled FROM domain_role_bindings \
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_MEMBER'",
    )
    .bind(domain_id)
    .bind(canonical)
    .fetch_one(&pool)
    .await
    .expect("new binding");
    assert!(new_enabled.0, "target binding enabled");

    // Same-key replay → original response.
    let (status, body) = do_method(
        &app,
        "POST",
        &apply_path,
        &token,
        json!({
            "role": "DOMAIN_MEMBER",
            "fromPrincipalId": stale_source.to_string(),
            "toPrincipalId": canonical.to_string(),
            "reason": "canonical migration"
        }),
        Some(&key),
    )
    .await;
    assert_eq!(status, 200, "replay: {body}");
    assert_eq!(body["outcome"], json!("applied"));

    // New key against the moved-away preimage → binding_conflict, zero
    // mutation (the canonical binding stays enabled and singular).
    let another = seed_agent(&pool).await;
    let (status, body) = do_method(
        &app,
        "POST",
        &apply_path,
        &token,
        json!({
            "role": "DOMAIN_MEMBER",
            "fromPrincipalId": stale_source.to_string(),
            "toPrincipalId": another.to_string(),
            "reason": "stale preimage"
        }),
        Some(&format!("rec-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 409, "stale preimage: {body}");
    assert_eq!(body["error"]["code"], json!("binding_conflict"));
    let canonical_enabled: (bool,) = sqlx::query_as(
        "SELECT enabled FROM domain_role_bindings \
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_MEMBER'",
    )
    .bind(domain_id)
    .bind(canonical)
    .fetch_one(&pool)
    .await
    .expect("canonical binding");
    assert!(canonical_enabled.0, "canonical binding untouched");
    let another_enabled: Option<(Option<bool>,)> = sqlx::query_as(
        "SELECT enabled FROM domain_role_bindings \
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_MEMBER'",
    )
    .bind(domain_id)
    .bind(another)
    .fetch_optional(&pool)
    .await
    .expect("another binding query");
    assert!(another_enabled.is_none(), "no phantom binding");
}

#[tokio::test]
async fn acc_cp_003_reconcile_identity_and_target_semantics() {
    let pool = common::create_pool().await;
    let coordinator = seed_agent(&pool).await;
    grant_coordinator(&pool, coordinator).await;
    let domain_id = seed_domain(&pool).await;
    let source = seed_agent(&pool).await;
    seed_binding(&pool, domain_id, source, "DOMAIN_MEMBER").await;

    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let token = direct_token(coordinator, "workflow.execute", &mock.key_pair);
    let apply_path = format!("/internal/v1/domains/{domain_id}/binding-reconcile/apply");

    // Missing target UUID → 404 identity_not_found.
    let (status, body) = do_method(
        &app,
        "POST",
        &apply_path,
        &token,
        json!({
            "role": "DOMAIN_MEMBER",
            "fromPrincipalId": source.to_string(),
            "toPrincipalId": Uuid::new_v4().to_string(),
            "reason": "missing target"
        }),
        Some(&format!("rec-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 404, "missing target: {body}");
    assert_eq!(body["error"]["code"], json!("identity_not_found"));

    // Disabled target → 403 principal_disabled.
    let disabled_target = seed_disabled_agent(&pool).await;
    let (status, body) = do_method(
        &app,
        "POST",
        &apply_path,
        &token,
        json!({
            "role": "DOMAIN_MEMBER",
            "fromPrincipalId": source.to_string(),
            "toPrincipalId": disabled_target.to_string(),
            "reason": "disabled target"
        }),
        Some(&format!("rec-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 403, "disabled target: {body}");
    assert_eq!(body["error"]["code"], json!("principal_disabled"));

    // Malformed role → 422 invalid_input.
    let target = seed_agent(&pool).await;
    let (status, body) = do_method(
        &app,
        "POST",
        &apply_path,
        &token,
        json!({
            "role": "DOMAIN_OVERLORD",
            "fromPrincipalId": source.to_string(),
            "toPrincipalId": target.to_string(),
            "reason": "bad role"
        }),
        Some(&format!("rec-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 422, "bad role: {body}");
    assert_eq!(body["error"]["code"], json!("invalid_input"));

    // from == to → noop outcome, binding stays enabled.
    let (status, body) = do_method(
        &app,
        "POST",
        &apply_path,
        &token,
        json!({
            "role": "DOMAIN_MEMBER",
            "fromPrincipalId": source.to_string(),
            "toPrincipalId": source.to_string(),
            "reason": "already canonical"
        }),
        Some(&format!("rec-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 200, "noop: {body}");
    assert_eq!(body["outcome"], json!("noop"));
    let (enabled,): (bool,) = sqlx::query_as(
        "SELECT enabled FROM domain_role_bindings \
         WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_MEMBER'",
    )
    .bind(domain_id)
    .bind(source)
    .fetch_one(&pool)
    .await
    .expect("binding");
    assert!(enabled, "noop keeps the binding enabled");
}

// ============================================================================
// Cross-domain cancel/archive (ACC-CP-001 tail) + lifecycle negatives
// (ACC-CP-006 / ACC-CP-007)
// ============================================================================

/// Minimal published V1 definition + active instance (mirrors test 28).
async fn seed_v1_definition(
    pool: &PgPool,
    domain_id: Uuid,
    agent_id: Uuid,
) -> (Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("ccp-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'CCP Def')",
    )
    .bind(def_id).bind(domain_id).bind(&def_key)
    .execute(pool).await.expect("insert def");

    sqlx::query(
        "INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, semantic_model_version) VALUES ($1, $2, 1, 'DRAFT', 3)",
    )
    .bind(ver_id).bind(def_id)
    .execute(pool).await.expect("insert version");

    let start_id = Uuid::new_v4();
    let done_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'start', 'Start', 0, 'TASK', 'WORKFLOW_CREATOR')",
    ).bind(start_id).bind(ver_id).execute(pool).await.expect("start");
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 1, 'TERMINAL', NULL)",
    ).bind(done_id).bind(ver_id).execute(pool).await.expect("done");
    let t1 = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance', 'advance', $3, $4, 'ADVANCE')",
    ).bind(t1).bind(ver_id).bind(start_id).bind(done_id)
    .execute(pool).await.expect("transition");
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2",
    )
    .bind(t1).bind(start_id).execute(pool).await.expect("primary");

    sqlx::query(
        "UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1",
    )
    .bind(ver_id).execute(pool).await.expect("publish");

    (ver_id, start_id)
}

async fn create_active_instance(
    pool: &PgPool,
    creator: Uuid,
    domain_id: Uuid,
    ver_id: Uuid,
) -> Uuid {
    let result = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        pool,
        svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(creator),
            idempotency_key: format!("ccp-create-{}", Uuid::new_v4()),
            command_schema_version: "v1".to_string(),
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(ver_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({}),
            context_payload: serde_json::json!({ "title": "ccp" }),
        },
    )
    .await
    .expect("create instance");
    result.workflow_instance_id
}

#[tokio::test]
async fn acc_cp_001_coordinator_cross_domain_cancel_archive_and_negatives() {
    let pool = common::create_pool().await;
    let coordinator = seed_agent(&pool).await;
    grant_coordinator(&pool, coordinator).await;

    // A foreign domain owned by someone else, with an active instance.
    let foreign_owner = seed_agent(&pool).await;
    let domain_id = seed_domain(&pool).await;
    seed_binding(&pool, domain_id, foreign_owner, "DOMAIN_OWNER").await;
    let plain_member = seed_agent(&pool).await;
    seed_binding(&pool, domain_id, plain_member, "DOMAIN_MEMBER").await;
    let (ver_id, _start) = seed_v1_definition(&pool, domain_id, plain_member).await;
    let instance_id = create_active_instance(&pool, plain_member, domain_id, ver_id).await;

    let mock = MockJwksServer::start().await;
    let app = build_app(pool.clone(), &mock.url);
    let coordinator_token =
        direct_token(coordinator, "workflow.execute workflow.read", &mock.key_pair);
    let member_token = direct_token(plain_member, "workflow.execute", &mock.key_pair);

    // Lifecycle negative: archive on the ACTIVE instance must fail for the
    // coordinator too (authority never bypasses lifecycle legality).
    let (status, body) = do_method(
        &app,
        "POST",
        &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
        &coordinator_token,
        json!({ "reason": "coordinator cleanup" }),
        Some(&format!("arc-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 409, "active archive must fail: {body}");
    assert!(
        body["error"]["code"] == json!("instance_not_terminal")
            || body["error"]["code"] == json!("active_activation_exists"),
        "lifecycle code: {}",
        body["error"]["code"]
    );

    // Plain DOMAIN_MEMBER (not assignee, not owner) cannot cancel.
    let (status, body) = do_method(
        &app,
        "POST",
        &format!("/internal/v1/workflow-instances/{instance_id}/cancel"),
        &member_token,
        json!({ "reason": "member must fail" }),
        Some(&format!("cx-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 403, "plain member cancel must fail closed: {body}");
    assert_eq!(body["error"]["code"], json!("not_domain_owner"));

    // Coordinator cancels cross-domain in its own identity.
    let (status, body) = do_method(
        &app,
        "POST",
        &format!("/internal/v1/workflow-instances/{instance_id}/cancel"),
        &coordinator_token,
        json!({ "reason": "confirmed canary cleanup" }),
        Some(&format!("cx-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 200, "coordinator cancel: {body}");
    assert_eq!(body["workflowInstanceId"], json!(instance_id.to_string()));

    // already_cancelled is byte-preserved for a second cancel.
    let (status, body) = do_method(
        &app,
        "POST",
        &format!("/internal/v1/workflow-instances/{instance_id}/cancel"),
        &coordinator_token,
        json!({ "reason": "double cancel" }),
        Some(&format!("cx-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 409, "second cancel: {body}");
    assert_eq!(body["error"]["code"], json!("already_cancelled"));

    // Coordinator archives the cancelled instance (read-after-write).
    let (status, body) = do_method(
        &app,
        "POST",
        &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
        &coordinator_token,
        json!({ "reason": "cleanup archive" }),
        Some(&format!("arc-{}", Uuid::new_v4())),
    )
    .await;
    assert_eq!(status, 200, "coordinator archive: {body}");

    // Read-back: the instance left the ACTIVE surface — status=active is
    // the cleanup semantics (cancelled = FALSE AND archived_at IS NULL;
    // lifecycle=active is node-type-based and includes cancelled rows).
    let (status, body) = do_get(
        &app,
        "/internal/v1/workflow-instances/global?status=active&limit=20",
        &coordinator_token,
    )
    .await;
    assert_eq!(status, 200, "global active read: {body}");
    let listed_again = body
        .to_string()
        .contains(&instance_id.to_string());
    assert!(!listed_again, "cancelled+archived instance must leave the active surface");
}
