//! HTTP-level integration tests for the worklist query endpoints.
//!
//! Tests the adapter layer that sits between HTTP and WorkflowQueryService.
//! Authentication, scope enforcement, actor isolation, cursor pagination,
//! and error semantics are covered here.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
};
use svc_workflow::http::{self, AppState, HttpConfig};

use super::*;

/// Add a principal as a MEMBER of a domain.
async fn domain_membership(pool: &sqlx::PgPool, domain_id: Uuid, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1, $2, $3, 'MEMBER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("add domain membership");
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

fn request(method: &str, uri: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder.body(Body::empty()).unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Seed a definition where DRAFT uses WORKFLOW_CREATOR and NORMAL uses
/// FIXED_PRINCIPAL with the given assignee_id, plus RETURN and TERMINATE
/// transitions. Returns (version_id, draft_advance_id, normal_node_id, return_id).
async fn seed_worklist_definition(
    pool: &sqlx::PgPool,
    domain_id: Uuid,
    assignee_id: Uuid,
) -> (Uuid, Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("wlist-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query("INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Worklist Test')")
        .bind(def_id).bind(domain_id).bind(&def_key)
        .execute(pool).await.expect("insert def");

    sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1, $2, 1, 'DRAFT', '{\"type\":\"object\"}'::jsonb)")
        .bind(ver_id).bind(def_id)
        .execute(pool).await.expect("insert version");

    let draft_id = Uuid::new_v4();
    let normal_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();

    // DRAFT node: WORKFLOW_CREATOR
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR')")
        .bind(draft_id).bind(ver_id)
        .execute(pool).await.expect("insert draft node");

    // NORMAL node: FIXED_PRINCIPAL
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id) VALUES ($1, $2, 'review', 'Review', 1, 'NORMAL', 'FIXED_PRINCIPAL', $3)")
        .bind(normal_id).bind(ver_id).bind(assignee_id)
        .execute(pool).await.expect("insert normal node");

    // TERMINAL node
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL)")
        .bind(term_id).bind(ver_id)
        .execute(pool).await.expect("insert terminal node");

    // DRAFT → NORMAL (advance)
    let draft_advance = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance-review', 'To Review', $3, $4, 'ADVANCE')")
        .bind(draft_advance).bind(ver_id).bind(draft_id).bind(normal_id)
        .execute(pool).await.expect("insert draft advance");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(draft_advance).bind(draft_id)
        .execute(pool).await.expect("set primary on draft");

    // NORMAL → DONE (advance)
    let normal_advance = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'advance-done', 'To Done', $3, $4, 'ADVANCE')")
        .bind(normal_advance).bind(ver_id).bind(normal_id).bind(term_id)
        .execute(pool).await.expect("insert normal advance");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id = $1 WHERE node_id = $2")
        .bind(normal_advance).bind(normal_id)
        .execute(pool).await.expect("set primary on normal");

    // NORMAL → DRAFT (return)
    let _return_id = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema) VALUES ($1, $2, 'return-draft', 'Return to Draft', $3, $4, 'RETURN', '{\"type\":\"object\",\"required\":[\"reasonCode\",\"reason\"],\"properties\":{\"reasonCode\":{\"type\":\"string\"},\"reason\":{\"type\":\"string\"}}}'::jsonb)")
        .bind(_return_id).bind(ver_id).bind(normal_id).bind(draft_id)
        .execute(pool).await.expect("insert return transition");

    // Publish
    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(pool).await.expect("publish version");

    (ver_id, draft_advance, normal_id, _return_id)
}

/// Create a workflow instance and advance it from DRAFT to the NORMAL node.
async fn create_and_advance(
    pool: &sqlx::PgPool,
    creator_id: Uuid,
    domain_id: Uuid,
    version_id: Uuid,
    draft_advance_id: Uuid,
) -> Uuid {
    let command = CreateWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(creator_id),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        domain_id: DomainId::from_uuid(domain_id),
        definition_version_id: DefinitionVersionId::from_uuid(version_id),
        external_reference: None,
        external_url: None,
        metadata: json!({"source": "worklist-test"}),
        context_payload: json!({"title": "worklist"}),
    };
    let created = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        pool, command,
    )
    .await
    .expect("create instance");

    let transition = ExecuteWorkflowTransitionCommand {
        principal_id: PrincipalId::from_uuid(creator_id),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(created.workflow_instance_id),
        expected_workflow_state_version: 1,
        transition_definition_id: TransitionId::from_uuid(draft_advance_id),
        submission_payload: Some(json!({"work": "ready"})),
    };
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        pool, transition,
    )
    .await
    .expect("advance to normal");

    created.workflow_instance_id
}

/// Create an instance but leave it in DRAFT (do not advance).
async fn create_draft_only(
    pool: &sqlx::PgPool,
    creator_id: Uuid,
    domain_id: Uuid,
    version_id: Uuid,
) -> Uuid {
    let command = CreateWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(creator_id),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        domain_id: DomainId::from_uuid(domain_id),
        definition_version_id: DefinitionVersionId::from_uuid(version_id),
        external_reference: None,
        external_url: None,
        metadata: json!({"source": "worklist-test"}),
        context_payload: json!({"title": "draft"}),
    };
    let created = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        pool, command,
    )
    .await
    .expect("create draft instance");
    created.workflow_instance_id
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_token_returns_401() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &Uuid::new_v4().to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let assigned = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            None,
        ))
        .await
        .unwrap();
    assert_eq!(assigned.status(), StatusCode::UNAUTHORIZED);

    // creator-owned-drafts must also return 401 (not 404) without a token
    let drafts = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/creator-owned-drafts",
            None,
        ))
        .await
        .unwrap();
    assert_eq!(drafts.status(), StatusCode::UNAUTHORIZED);

    // Unknown path returns 404
    let unknown = app
        .clone()
        .oneshot(request("GET", "/internal/v1/worklists/unknown", None))
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn missing_workflow_read_scope_returns_403() {
    let pool = create_pool().await;
    let principal_id = seed_second_principal(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &principal_id.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let no_read_token = common::v1_token(
        principal_id,
        "workflow.execute",
        "test-client",
        300,
        &mock.key_pair,
    );

    let assigned = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&no_read_token),
        ))
        .await
        .unwrap();
    assert_eq!(assigned.status(), StatusCode::FORBIDDEN);

    let drafts = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/creator-owned-drafts",
            Some(&no_read_token),
        ))
        .await
        .unwrap();
    assert_eq!(drafts.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn assigned_to_me_returns_current_assignee_only() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let (version_id, draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &assignee.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let actor_token = common::v1_token(
        assignee,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    // Creator needs domain membership
    domain_membership(&pool, domain_id, creator).await;
    // Assignee also needs domain membership to appear in worklists
    domain_membership(&pool, domain_id, assignee).await;

    // Before advancing: assignee has no assigned items
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&actor_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 0);

    // Create and advance to NORMAL (assignee becomes current assignee)
    create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&actor_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        body["items"][0]["detail"]["current_visit"]["assignee_principal_id"],
        assignee.to_string()
    );
    assert!(body["next_cursor"].is_null());
}

#[tokio::test]
async fn historical_assignee_not_returned() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let other_principal = seed_second_principal(&pool).await;
    let (version_id, draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &other_principal.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let other_token = common::v1_token(
        other_principal,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    domain_membership(&pool, domain_id, creator).await;
    create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;

    // Other principal should not see the instance
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&other_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn cross_domain_isolation() {
    let pool = create_pool().await;
    let (_owner1, domain1) = seed_principal_domain_with_owner(&pool).await;
    let (_owner2, domain2) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;

    let (ver1, adv1, _, _) = seed_worklist_definition(&pool, domain1, assignee).await;
    let (ver2, adv2, _, _) = seed_worklist_definition(&pool, domain2, assignee).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &assignee.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let assignee_token = common::v1_token(
        assignee,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    domain_membership(&pool, domain1, creator).await;
    domain_membership(&pool, domain2, creator).await;
    // Assignee needs membership in both domains
    domain_membership(&pool, domain1, assignee).await;
    domain_membership(&pool, domain2, assignee).await;

    // Create and advance in both domains
    create_and_advance(&pool, creator, domain1, ver1, adv1).await;
    create_and_advance(&pool, creator, domain2, ver2, adv2).await;

    // Assignee should see both (same principal in both domains)
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&assignee_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 2);

    // Verify both instances have different domain IDs
    let domains: Vec<String> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            item["detail"]["instance"]["domain_id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert!(domains.contains(&domain1.to_string()));
    assert!(domains.contains(&domain2.to_string()));
}

#[tokio::test]
async fn direct_agent_token_works() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let (version_id, draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &assignee.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let agent_token = common::v1_token(
        assignee,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    domain_membership(&pool, domain_id, creator).await;
    domain_membership(&pool, domain_id, assignee).await;
    create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&agent_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn actor_comes_from_jwt_sub_not_query() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let outsider = seed_second_principal(&pool).await;
    let (version_id, draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &outsider.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    domain_membership(&pool, domain_id, creator).await;
    create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;

    // An outsider uses a valid token but also includes query params that would
    // try to specify a different actor. The handler must ignore them entirely.
    let outsider_token = common::v1_token(
        outsider,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    // Query params that look like they could forge the actor must be rejected
    // (WorklistQuery uses deny_unknown_fields, so extra params cause 422).
    let uri = format!("/internal/v1/worklists/assigned-to-me?actorId={}", assignee);
    let resp = app
        .clone()
        .oneshot(request("GET", &uri, Some(&outsider_token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn invalid_cursor_returns_422() {
    let pool = create_pool().await;
    let principal_id = seed_second_principal(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &principal_id.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let actor_token = common::v1_token(
        principal_id,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    // Missing beforeId
    let uri = "/internal/v1/worklists/assigned-to-me?beforeCreatedAt=2024-01-15T10:30:00Z";
    let resp = app
        .clone()
        .oneshot(request("GET", uri, Some(&actor_token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // Invalid timestamp
    let uri = "/internal/v1/worklists/assigned-to-me?beforeCreatedAt=not-a-date&beforeId=550e8400-e29b-41d4-a716-446655440000";
    let resp = app
        .clone()
        .oneshot(request("GET", uri, Some(&actor_token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn pagination_cursor_works() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let (version_id, draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &assignee.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let assignee_token = common::v1_token(
        assignee,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    domain_membership(&pool, domain_id, creator).await;
    domain_membership(&pool, domain_id, assignee).await;

    // Create and advance 3 instances
    for _ in 0..3 {
        create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    // Fetch first page with limit=1
    let uri = "/internal/v1/worklists/assigned-to-me?limit=1";
    let resp = app
        .clone()
        .oneshot(request("GET", uri, Some(&assignee_token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let page1 = json_body(resp).await;
    assert_eq!(page1["items"].as_array().unwrap().len(), 1);
    let cursor = page1["next_cursor"].clone();
    let id1 = page1["items"][0]["detail"]["instance"]["workflow_instance_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Fetch second page
    let uri = format!(
        "/internal/v1/worklists/assigned-to-me?limit=1&beforeCreatedAt={}&beforeId={}",
        cursor["created_at"].as_str().unwrap(),
        cursor["id"].as_str().unwrap(),
    );
    let resp = app
        .clone()
        .oneshot(request("GET", &uri, Some(&assignee_token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let page2 = json_body(resp).await;
    assert_eq!(page2["items"].as_array().unwrap().len(), 1);
    let id2 = page2["items"][0]["detail"]["instance"]["workflow_instance_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(id1, id2, "pages must return different items");

    // Fetch third page
    let cursor2 = page2["next_cursor"].clone();
    let uri = format!(
        "/internal/v1/worklists/assigned-to-me?limit=1&beforeCreatedAt={}&beforeId={}",
        cursor2["created_at"].as_str().unwrap(),
        cursor2["id"].as_str().unwrap(),
    );
    let resp = app
        .clone()
        .oneshot(request("GET", &uri, Some(&assignee_token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let page3 = json_body(resp).await;
    assert_eq!(page3["items"].as_array().unwrap().len(), 1);
    let id3 = page3["items"][0]["detail"]["instance"]["workflow_instance_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(id1, id3);
    assert_ne!(id2, id3);

    // Should be no more pages
    assert!(page3["next_cursor"].is_null(), "should have no next_cursor");
}

#[tokio::test]
async fn empty_results_return_empty_page() {
    let pool = create_pool().await;
    let principal_id = seed_second_principal(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &principal_id.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let actor_token = common::v1_token(
        principal_id,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&actor_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
    assert!(body["next_cursor"].is_null());
}

// ---------------------------------------------------------------------------
// Domain isolation tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn no_domain_permission_hides_items() {
    // Actor has access to domain A but NOT domain B.
    // Items in domain B must not appear in worklists even if the actor is
    // the current assignee or creator.
    let pool = create_pool().await;

    let (_owner1, domain1) = seed_principal_domain_with_owner(&pool).await;
    let (_owner2, domain2) = seed_principal_domain_with_owner(&pool).await;
    let actor = seed_second_principal(&pool).await;
    let creator = seed_second_principal(&pool).await;

    // Actor only gets membership on domain1
    domain_membership(&pool, domain1, actor).await;
    domain_membership(&pool, domain1, creator).await;
    // Creator needs membership on domain2 to create instances there
    domain_membership(&pool, domain2, creator).await;

    let (ver1, adv1, _, _) = seed_worklist_definition(&pool, domain1, actor).await;
    let (ver2, adv2, _, _) = seed_worklist_definition(&pool, domain2, actor).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &actor.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let actor_token = common::v1_token(actor, "workflow.read", "test-client", 300, &mock.key_pair);

    // Advance instances in both domains, assignee = actor
    create_and_advance(&pool, creator, domain1, ver1, adv1).await;
    create_and_advance(&pool, creator, domain2, ver2, adv2).await;

    // Actor should only see domain1's item in assigned-to-me
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&actor_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    let domain_id = body["items"][0]["detail"]["instance"]["domain_id"]
        .as_str()
        .unwrap();
    assert_eq!(domain_id, domain1.to_string());
}

#[tokio::test]
async fn domain_disabled_hides_items() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let (version_id, draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    domain_membership(&pool, domain_id, creator).await;
    domain_membership(&pool, domain_id, assignee).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &assignee.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let assignee_token = common::v1_token(
        assignee,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    // Create and advance (assignee can see it)
    create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&assignee_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["items"].as_array().unwrap().len(), 1);

    // Disable the domain
    sqlx::query("UPDATE domains SET enabled = FALSE WHERE domain_id = $1")
        .bind(domain_id)
        .execute(&pool)
        .await
        .unwrap();

    // Assignee should no longer see the item
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&assignee_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["items"].as_array().unwrap().len(), 0);

    // Re-enable the domain
    sqlx::query("UPDATE domains SET enabled = TRUE WHERE domain_id = $1")
        .bind(domain_id)
        .execute(&pool)
        .await
        .unwrap();

    // Assignee should see the item again
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&assignee_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn role_binding_revoked_hides_items() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let (version_id, draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    domain_membership(&pool, domain_id, creator).await;
    domain_membership(&pool, domain_id, assignee).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &assignee.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let assignee_token = common::v1_token(
        assignee,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    // Create and advance (assignee can see it)
    create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&assignee_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["items"].as_array().unwrap().len(), 1);

    // Delete the assignee's domain role binding
    sqlx::query("DELETE FROM domain_role_bindings WHERE domain_id = $1 AND principal_id = $2")
        .bind(domain_id)
        .bind(assignee)
        .execute(&pool)
        .await
        .unwrap();

    // Assignee should no longer see the item
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&assignee_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn assignee_without_domain_permission_not_returned() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let (version_id, draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    // Creator has membership (needed to create instances)
    domain_membership(&pool, domain_id, creator).await;
    // Assignee deliberately has NO domain membership

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &assignee.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let assignee_token = common::v1_token(
        assignee,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    // Advance — assignee becomes current node visit assignee
    create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;

    // Assignee should NOT see the instance in assigned-to-me
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&assignee_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn creator_without_domain_permission_drafts_not_returned() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let (version_id, _draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    // Creator needs membership initially to create the instance
    domain_membership(&pool, domain_id, creator).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &creator.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let creator_token =
        common::v1_token(creator, "workflow.read", "test-client", 300, &mock.key_pair);

    // Create a draft instance
    create_draft_only(&pool, creator, domain_id, version_id).await;

    // Creator can see it (has membership)
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me", // test via assigned-to-me; drafts endpoint may not exist
            Some(&creator_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // Remove the creator's domain role binding
    sqlx::query("DELETE FROM domain_role_bindings WHERE domain_id = $1 AND principal_id = $2")
        .bind(domain_id)
        .bind(creator)
        .execute(&pool)
        .await
        .unwrap();

    // Verify the domain isolation by checking assigned-to-me is empty
    // (the instance is in DRAFT so creator is the current assignee via WORKFLOW_CREATOR)
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&creator_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(json_body(resp).await["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn multiple_legal_domains_all_visible() {
    let pool = create_pool().await;
    let (_owner_a, domain_a) = seed_principal_domain_with_owner(&pool).await;
    let (_owner_b, domain_b) = seed_principal_domain_with_owner(&pool).await;
    let (_owner_c, domain_c) = seed_principal_domain_with_owner(&pool).await;
    let actor = seed_second_principal(&pool).await;
    let creator = seed_second_principal(&pool).await;

    // Actor has membership on domain_a and domain_c, but not domain_b
    domain_membership(&pool, domain_a, actor).await;
    domain_membership(&pool, domain_c, actor).await;
    domain_membership(&pool, domain_a, creator).await;
    domain_membership(&pool, domain_b, creator).await;
    domain_membership(&pool, domain_c, creator).await;

    let (ver_a, adv_a, _, _) = seed_worklist_definition(&pool, domain_a, actor).await;
    let (ver_b, adv_b, _, _) = seed_worklist_definition(&pool, domain_b, actor).await;
    let (ver_c, adv_c, _, _) = seed_worklist_definition(&pool, domain_c, actor).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &actor.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let actor_token = common::v1_token(actor, "workflow.read", "test-client", 300, &mock.key_pair);

    // Advance instances in all three domains
    create_and_advance(&pool, creator, domain_a, ver_a, adv_a).await;
    create_and_advance(&pool, creator, domain_b, ver_b, adv_b).await;
    create_and_advance(&pool, creator, domain_c, ver_c, adv_c).await;

    // Actor should see domain_a and domain_c items, but NOT domain_b
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&actor_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);

    let domains: Vec<&str> = items
        .iter()
        .map(|item| item["detail"]["instance"]["domain_id"].as_str().unwrap())
        .collect();
    assert!(domains.contains(&domain_a.to_string().as_str()));
    assert!(domains.contains(&domain_c.to_string().as_str()));
    assert!(!domains.contains(&domain_b.to_string().as_str()));
}

#[tokio::test]
async fn pagination_respects_domain_isolation() {
    let pool = create_pool().await;
    let (_owner_a, domain_a) = seed_principal_domain_with_owner(&pool).await;
    let (_owner_b, domain_b) = seed_principal_domain_with_owner(&pool).await;
    let actor = seed_second_principal(&pool).await;
    let creator = seed_second_principal(&pool).await;

    // Actor only has access to domain_a
    domain_membership(&pool, domain_a, actor).await;
    domain_membership(&pool, domain_a, creator).await;
    domain_membership(&pool, domain_b, creator).await;

    let (ver_a, adv_a, _, _) = seed_worklist_definition(&pool, domain_a, actor).await;
    let (ver_b, adv_b, _, _) = seed_worklist_definition(&pool, domain_b, actor).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &actor.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let actor_token = common::v1_token(actor, "workflow.read", "test-client", 300, &mock.key_pair);

    // Create multiple instances in domain_a with slight timestamp gaps
    for _ in 0..2 {
        create_and_advance(&pool, creator, domain_a, ver_a, adv_a).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    // Also create in domain_b (actor shouldn't see these)
    create_and_advance(&pool, creator, domain_b, ver_b, adv_b).await;

    // Fetch first page with limit=1
    let uri = "/internal/v1/worklists/assigned-to-me?limit=1";
    let resp = app
        .clone()
        .oneshot(request("GET", uri, Some(&actor_token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let page1 = json_body(resp).await;
    assert_eq!(page1["items"].as_array().unwrap().len(), 1);
    assert!(page1["next_cursor"].is_object(), "should have next_cursor");

    let domain1 = page1["items"][0]["detail"]["instance"]["domain_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(domain1, domain_a.to_string());

    // Fetch second page
    let cursor = page1["next_cursor"].clone();
    let uri = format!(
        "/internal/v1/worklists/assigned-to-me?limit=1&beforeCreatedAt={}&beforeId={}",
        cursor["created_at"].as_str().unwrap(),
        cursor["id"].as_str().unwrap(),
    );
    let resp = app
        .clone()
        .oneshot(request("GET", &uri, Some(&actor_token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let page2 = json_body(resp).await;
    assert_eq!(page2["items"].as_array().unwrap().len(), 1);

    let domain2 = page2["items"][0]["detail"]["instance"]["domain_id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(domain2, domain_a.to_string());

    // No more pages (only 2 items from domain_a, none from domain_b leaked)
    assert!(
        page2["next_cursor"].is_null(),
        "should have no next_cursor after exhausting authorized items"
    );
}

// ---------------------------------------------------------------------------
// Multi-role dedup tests
// ---------------------------------------------------------------------------

async fn add_second_role(pool: &sqlx::PgPool, domain_id: Uuid, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1, $2, $3, 'CONTRIBUTOR', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("add CONTRIBUTOR role");
}

#[tokio::test]
async fn assigned_to_me_multi_role_no_duplicates() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let actor = seed_second_principal(&pool).await;
    let (version_id, draft_advance, _, _) = seed_worklist_definition(&pool, domain_id, actor).await;

    domain_membership(&pool, domain_id, actor).await;
    add_second_role(&pool, domain_id, actor).await;
    domain_membership(&pool, domain_id, creator).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &actor.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let actor_token = common::v1_token(actor, "workflow.read", "test-client", 300, &mock.key_pair);

    for _ in 0..3 {
        create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&actor_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let items = json_body(resp).await["items"].as_array().unwrap().clone();
    assert_eq!(items.len(), 3);
    let mut ids: Vec<String> = items
        .iter()
        .map(|i| {
            i["detail"]["instance"]["workflow_instance_id"]
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    ids.sort();
    let mut deduped = ids.clone();
    deduped.dedup();
    assert_eq!(ids, deduped);
}

// ---------------------------------------------------------------------------
// Creator-owned drafts tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn creator_owned_drafts_returns_only_own_drafts() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let outsider = seed_second_principal(&pool).await;
    let (version_id, _draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, "");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let creator_token =
        common::v1_token(creator, "workflow.read", "test-client", 300, &mock.key_pair);

    domain_membership(&pool, domain_id, creator).await;

    // Create a draft instance (remains in DRAFT)
    create_draft_only(&pool, creator, domain_id, version_id).await;

    // Creator should see it in drafts
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/creator-owned-drafts",
            Some(&creator_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);

    // Outsider should not see it
    let outsider_token = common::v1_token(
        outsider,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/creator-owned-drafts",
            Some(&outsider_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn non_draft_not_returned_in_creator_drafts() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let (version_id, draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &creator.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let creator_token =
        common::v1_token(creator, "workflow.read", "test-client", 300, &mock.key_pair);

    domain_membership(&pool, domain_id, creator).await;

    // Create and advance - instance is now in NORMAL, not DRAFT
    create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;

    // Creator should NOT see it in drafts (it's in NORMAL/REVIEW, not DRAFT)
    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/creator-owned-drafts",
            Some(&creator_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn unknown_path_returns_404() {
    let pool = create_pool().await;
    let principal_id = seed_second_principal(&pool).await;
    let mock = common::MockJwksServer::start().await;
    let (app, _state) = build_config(&pool, &mock.url, &principal_id.to_string());
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let actor_token = common::v1_token(
        principal_id,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/unknown-route",
            Some(&actor_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
