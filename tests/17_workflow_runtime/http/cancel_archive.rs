//! HTTP-level integration tests for Instance Cancel and Archive V1 adapters.
//!
//! Exercises the shipped HTTP contract end-to-end:
//! POST /internal/v1/workflow-instances/{workflowInstanceId}/cancel
//! POST /internal/v1/workflow-instances/{workflowInstanceId}/archive
//!
//! Covers the adapter's idempotency-key hashing (64-hex request hash) and the
//! server-authoritative state-version contract (expected_workflow_state_version
//! sentinel 0), which the legacy candidate's HTTP layer did not satisfy.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};

use super::*;

fn token(principal_id: Uuid, scope: &str, key_pair: &common::RsaTestKeyPair) -> String {
    common::v1_token(principal_id, scope, "test-client", 300, key_pair)
}

fn build_config(
    pool: &sqlx::PgPool,
    jwks_url: &str,
    allowed_sub: &str,
) -> (axum::Router, AppState) {
    let config = HttpConfig {
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
        provisioning_config: ProvisioningConfig::new(Vec::new()),
        auth_v1_canary_config: AuthV1CanaryConfig {
            enabled: true,
            write_enabled: true,
            allowed_client_id: "test-client".to_string(),
            allowed_sub: allowed_sub.to_string(),
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
    let state = AppState::new(pool.clone(), &config);
    (http::router(state.clone(), &config), state)
}

fn request(
    method: &str,
    uri: &str,
    token: Option<&str>,
    key: Option<&str>,
    body: Option<Value>,
) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if let Some(key) = key {
        builder = builder.header("idempotency-key", key);
    }
    if body.is_some() {
        builder = builder.header("content-type", "application/json");
    }
    builder
        .body(body.map_or_else(Body::empty, |value| Body::from(value.to_string())))
        .unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Seed an instance via the HTTP create endpoint. Returns
/// (workflow_instance_id, workflow_state_version).
async fn create_instance_via_http(
    app: &axum::Router,
    token: &str,
    domain_id: Uuid,
    definition_version_id: Uuid,
    label: &str,
) -> (String, i64) {
    let created = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/workflow-instances",
            Some(token),
            Some(label),
            Some(json!({
                "domainId": domain_id,
                "definitionVersionId": definition_version_id,
                "externalReference": format!("{label}-{}", Uuid::new_v4()),
                "metadata": {},
                "contextPayload": {}
            })),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let body = json_body(created).await;
    (
        body["workflowInstanceId"].as_str().unwrap().to_string(),
        body["workflowStateVersion"].as_i64().unwrap(),
    )
}

/// Advance the instance to TERMINAL via the HTTP transition endpoint
/// (draft -> review -> done for the normal-node fixture).
async fn advance_to_terminal_via_http(
    app: &axum::Router,
    pool: &sqlx::PgPool,
    token: &str,
    workflow_instance_id: &str,
    definition_version_id: Uuid,
    first_state_version: i64,
) {
    let mut state_version = first_state_version;
    for node_key in ["draft", "review"] {
        let transition_id: Uuid = sqlx::query_scalar(
            "SELECT t.transition_id
             FROM workflow_transition_definitions t
             JOIN workflow_node_definitions n ON n.node_id = t.source_node_id
             WHERE t.definition_version_id = $1 AND n.node_key = $2",
        )
        .bind(definition_version_id)
        .bind(node_key)
        .fetch_one(pool)
        .await
        .unwrap();
        let transitioned = app
            .clone()
            .oneshot(request(
                "POST",
                &format!("/internal/v1/workflow-instances/{workflow_instance_id}/transitions"),
                Some(token),
                Some(&format!("advance-{node_key}")),
                Some(json!({
                    "transitionDefinitionId": transition_id,
                    "expectedWorkflowStateVersion": state_version
                })),
            ))
            .await
            .unwrap();
        assert_eq!(transitioned.status(), StatusCode::OK);
        state_version = json_body(transitioned).await["workflowStateVersion"]
            .as_i64()
            .unwrap();
    }
}

#[tokio::test]
async fn domain_owner_cancel_active_via_http() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, definition_version_id, _) =
        seed_published_definition_normal_node(&pool, domain_id).await;

    let (app, _state) = build_config(&pool, &mock.url, &principal_id.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let full_token = token(
        principal_id,
        "workflow.execute workflow.read",
        &mock.key_pair,
    );

    let (instance_id, _) = create_instance_via_http(
        &app,
        &full_token,
        domain_id,
        definition_version_id,
        "cancel-http",
    )
    .await;

    // Cancel the active instance via HTTP.
    let cancelled = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/cancel"),
            Some(&full_token),
            Some("cancel-http-1"),
            Some(json!({ "reason": "duplicate_instance" })),
        ))
        .await
        .unwrap();
    assert_eq!(cancelled.status(), StatusCode::OK);
    let cancel_body = json_body(cancelled).await;
    assert_eq!(
        cancel_body["workflowInstanceId"].as_str().unwrap(),
        instance_id
    );
    assert_eq!(cancel_body["replayed"], false);

    // DB flag is set.
    let cancelled_flag: bool = sqlx::query_scalar(
        "SELECT cancelled FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(Uuid::parse_str(&instance_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(cancelled_flag, "instance should be marked cancelled");

    // Submission history endpoint still readable after cancel (cancel does
    // not delete submissions or their history).
    let submissions = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/internal/v1/workflow-instances/{instance_id}/submissions"),
            Some(&full_token),
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(submissions.status(), StatusCode::OK);

    // Hidden from the assigned-to-me worklist.
    let worklist = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me?limit=20",
            Some(&full_token),
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(worklist.status(), StatusCode::OK);
    let worklist_body = json_body(worklist).await;
    let ids: Vec<&str> = worklist_body["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["detail"]["instance"]["workflowInstanceId"].as_str())
        .collect();
    assert!(
        !ids.contains(&instance_id.as_str()),
        "cancelled instance must not appear in assigned-to-me worklist"
    );

    // Timeline still readable and contains the cancel event.
    let timeline = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/internal/v1/workflow-instances/{instance_id}/timeline?limit=50"),
            Some(&full_token),
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(timeline.status(), StatusCode::OK);
    let timeline_body = json_body(timeline).await;
    let cancel_events: Vec<_> = timeline_body["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["event_type"] == "WORKFLOW_INSTANCE_CANCELLED")
        .collect();
    assert_eq!(cancel_events.len(), 1, "exactly one cancel event");
}

#[tokio::test]
async fn cancel_is_idempotent_via_http() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, definition_version_id, _) =
        seed_published_definition_normal_node(&pool, domain_id).await;

    let (app, _state) = build_config(&pool, &mock.url, &principal_id.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let full_token = token(
        principal_id,
        "workflow.execute workflow.read",
        &mock.key_pair,
    );

    let (instance_id, _) = create_instance_via_http(
        &app,
        &full_token,
        domain_id,
        definition_version_id,
        "cancel-idem",
    )
    .await;

    let cancel_once = || {
        request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/cancel"),
            Some(&full_token),
            Some("cancel-idem-key"),
            Some(json!({ "reason": "duplicate_instance" })),
        )
    };

    let first = app.clone().oneshot(cancel_once()).await.unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(json_body(first).await["replayed"], false);

    // Same key + same body -> idempotent replay.
    let replay = app.clone().oneshot(cancel_once()).await.unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(json_body(replay).await["replayed"], true);

    // New key on the already-cancelled instance -> explicit 409 rejection.
    let again = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/cancel"),
            Some(&full_token),
            Some("cancel-idem-key-2"),
            Some(json!({ "reason": "duplicate_instance" })),
        ))
        .await
        .unwrap();
    assert_eq!(again.status(), StatusCode::CONFLICT);
    assert_eq!(json_body(again).await["error"]["code"], "already_cancelled");
}

#[tokio::test]
async fn non_owner_cancel_denied_via_http() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let non_owner_id = seed_second_principal(&pool).await;
    let (_, definition_version_id, _) =
        seed_published_definition_normal_node(&pool, domain_id).await;

    // Empty allowed_sub disables the sub allow-list so the non-owner's token
    // reaches the handler and is rejected by the domain-owner authorization
    // (403 not_domain_owner) rather than by the auth layer (401).
    let (app, _state) = build_config(&pool, &mock.url, "");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    // The owner creates the instance.
    let owner_token = token(owner_id, "workflow.execute", &mock.key_pair);
    let (instance_id, _) = create_instance_via_http(
        &app,
        &owner_token,
        domain_id,
        definition_version_id,
        "cancel-denied",
    )
    .await;

    // Non-owner (with execute scope) cannot cancel.
    let non_owner_token = token(non_owner_id, "workflow.execute", &mock.key_pair);
    let denied = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/cancel"),
            Some(&non_owner_token),
            Some("cancel-denied-key"),
            Some(json!({ "reason": "cleanup" })),
        ))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(json_body(denied).await["error"]["code"], "not_domain_owner");
}

#[tokio::test]
async fn cancel_terminal_instance_denied_via_http() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, definition_version_id, _) =
        seed_published_definition_normal_node(&pool, domain_id).await;

    let (app, _state) = build_config(&pool, &mock.url, &principal_id.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let full_token = token(
        principal_id,
        "workflow.execute workflow.read",
        &mock.key_pair,
    );

    let (instance_id, state_version) = create_instance_via_http(
        &app,
        &full_token,
        domain_id,
        definition_version_id,
        "cancel-term",
    )
    .await;
    advance_to_terminal_via_http(
        &app,
        &pool,
        &full_token,
        &instance_id,
        definition_version_id,
        state_version,
    )
    .await;

    let denied = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/cancel"),
            Some(&full_token),
            Some("cancel-term-key"),
            Some(json!({ "reason": "cleanup" })),
        ))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::CONFLICT);
    assert_eq!(
        json_body(denied).await["error"]["code"],
        "source_node_terminal"
    );
}

#[tokio::test]
async fn domain_owner_archive_terminal_via_http() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, definition_version_id, _) =
        seed_published_definition_normal_node(&pool, domain_id).await;

    let (app, _state) = build_config(&pool, &mock.url, &principal_id.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let full_token = token(
        principal_id,
        "workflow.execute workflow.read",
        &mock.key_pair,
    );

    let (instance_id, state_version) = create_instance_via_http(
        &app,
        &full_token,
        domain_id,
        definition_version_id,
        "archive-http",
    )
    .await;
    advance_to_terminal_via_http(
        &app,
        &pool,
        &full_token,
        &instance_id,
        definition_version_id,
        state_version,
    )
    .await;

    // Archive the terminal instance via HTTP.
    let archived = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
            Some(&full_token),
            Some("archive-http-1"),
            Some(json!({ "reason": "test_instance_cleanup" })),
        ))
        .await
        .unwrap();
    assert_eq!(archived.status(), StatusCode::OK);
    let archive_body = json_body(archived).await;
    assert_eq!(
        archive_body["workflowInstanceId"].as_str().unwrap(),
        instance_id
    );
    assert_eq!(archive_body["replayed"], false);

    // Detail still readable.
    let detail = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/internal/v1/workflow-instances/{instance_id}"),
            Some(&full_token),
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    assert_eq!(json_body(detail).await["visibility"], "full");

    // Submission history still readable.
    let submissions = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/internal/v1/workflow-instances/{instance_id}/submissions"),
            Some(&full_token),
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(submissions.status(), StatusCode::OK);

    // Timeline still readable and contains exactly one archive event.
    let timeline = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/internal/v1/workflow-instances/{instance_id}/timeline?limit=50"),
            Some(&full_token),
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(timeline.status(), StatusCode::OK);
    let timeline_body = json_body(timeline).await;
    let archive_events: Vec<_> = timeline_body["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|e| e["event_type"] == "WORKFLOW_INSTANCE_ARCHIVED")
        .collect();
    assert_eq!(archive_events.len(), 1, "exactly one archive event");

    // Same key replays idempotently.
    let replay = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
            Some(&full_token),
            Some("archive-http-1"),
            Some(json!({ "reason": "test_instance_cleanup" })),
        ))
        .await
        .unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(json_body(replay).await["replayed"], true);

    // Archived instance is hidden from the default assigned-to-me worklist.
    let worklist = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me?limit=20",
            Some(&full_token),
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(worklist.status(), StatusCode::OK);
    let worklist_body = json_body(worklist).await;
    let ids: Vec<&str> = worklist_body["items"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|item| item["detail"]["instance"]["workflowInstanceId"].as_str())
        .collect();
    assert!(
        !ids.contains(&instance_id.as_str()),
        "archived instance must not appear in assigned-to-me worklist"
    );
}

#[tokio::test]
async fn archive_active_instance_denied_via_http() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, definition_version_id, _) =
        seed_published_definition_normal_node(&pool, domain_id).await;

    let (app, _state) = build_config(&pool, &mock.url, &principal_id.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let full_token = token(principal_id, "workflow.execute", &mock.key_pair);

    let (instance_id, _) = create_instance_via_http(
        &app,
        &full_token,
        domain_id,
        definition_version_id,
        "archive-active",
    )
    .await;

    let denied = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
            Some(&full_token),
            Some("archive-active-key"),
            Some(json!({ "reason": "cleanup" })),
        ))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::CONFLICT);
    assert_eq!(
        json_body(denied).await["error"]["code"],
        "instance_not_terminal"
    );
}

#[tokio::test]
async fn non_owner_archive_denied_via_http() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let non_owner_id = seed_second_principal(&pool).await;
    let (_, definition_version_id, _) =
        seed_published_definition_normal_node(&pool, domain_id).await;

    // Empty allowed_sub disables the sub allow-list so the non-owner's token
    // reaches the handler and is rejected by the domain-owner authorization
    // (403 not_domain_owner) rather than by the auth layer (401).
    let (app, _state) = build_config(&pool, &mock.url, "");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let owner_token = token(owner_id, "workflow.execute", &mock.key_pair);
    let (instance_id, state_version) = create_instance_via_http(
        &app,
        &owner_token,
        domain_id,
        definition_version_id,
        "archive-denied",
    )
    .await;
    advance_to_terminal_via_http(
        &app,
        &pool,
        &owner_token,
        &instance_id,
        definition_version_id,
        state_version,
    )
    .await;

    let non_owner_token = token(non_owner_id, "workflow.execute", &mock.key_pair);
    let denied = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
            Some(&non_owner_token),
            Some("archive-denied-key"),
            Some(json!({ "reason": "cleanup" })),
        ))
        .await
        .unwrap();
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(json_body(denied).await["error"]["code"], "not_domain_owner");
}

#[tokio::test]
async fn archive_already_archived_and_conflict_via_http() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, definition_version_id, _) =
        seed_published_definition_normal_node(&pool, domain_id).await;

    let (app, _state) = build_config(&pool, &mock.url, &principal_id.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let full_token = token(
        principal_id,
        "workflow.execute workflow.read",
        &mock.key_pair,
    );

    let (instance_id, state_version) = create_instance_via_http(
        &app,
        &full_token,
        domain_id,
        definition_version_id,
        "archive-once-http",
    )
    .await;
    advance_to_terminal_via_http(
        &app,
        &pool,
        &full_token,
        &instance_id,
        definition_version_id,
        state_version,
    )
    .await;

    // First archive with key1 -> 200.
    let first = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
            Some(&full_token),
            Some("archive-once-http-1"),
            Some(json!({ "reason": "cleanup" })),
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(json_body(first).await["replayed"], false);

    // New key on the already-archived instance -> 409 already_archived.
    let again = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
            Some(&full_token),
            Some("archive-once-http-2"),
            Some(json!({ "reason": "another_cleanup" })),
        ))
        .await
        .unwrap();
    assert_eq!(again.status(), StatusCode::CONFLICT);
    assert_eq!(json_body(again).await["error"]["code"], "already_archived");

    // Same key + different request body -> 409 idempotency_conflict.
    let conflicting = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
            Some(&full_token),
            Some("archive-once-http-1"),
            Some(json!({ "reason": "different_reason" })),
        ))
        .await
        .unwrap();
    assert_eq!(conflicting.status(), StatusCode::CONFLICT);
    assert_eq!(
        json_body(conflicting).await["error"]["code"],
        "idempotency_conflict"
    );

    // Same key + same request body still replays with the original result.
    let replay = app
        .clone()
        .oneshot(request(
            "POST",
            &format!("/internal/v1/workflow-instances/{instance_id}/archive"),
            Some(&full_token),
            Some("archive-once-http-1"),
            Some(json!({ "reason": "cleanup" })),
        ))
        .await
        .unwrap();
    assert_eq!(replay.status(), StatusCode::OK);
    assert_eq!(json_body(replay).await["replayed"], true);

    // Invariants: exactly one archive event, no success receipt for key2.
    let event_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_events
         WHERE workflow_instance_id = $1 AND event_type = 'WORKFLOW_INSTANCE_ARCHIVED'",
    )
    .bind(Uuid::parse_str(&instance_id).unwrap())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        event_count, 1,
        "exactly one archive event after rejected retries"
    );

    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM workflow_command_receipts
         WHERE principal_id = $1 AND idempotency_key = 'archive-once-http-2'",
    )
    .bind(principal_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        receipt_count, 0,
        "no receipt may exist for the rejected key"
    );
}
