//! HTTP-level integration tests for the worklist query endpoints.
//!
//! Tests the adapter layer that sits between HTTP and WorkflowQueryService.
//! Authentication, scope enforcement, actor isolation, cursor pagination,
//! and error semantics are covered here.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthMode, Hs256Config};
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
};
use svc_workflow::http::{self, AppState, HttpConfig};

use super::*;

const JWT_SECRET: &str = "worklist-smoke-secret-at-least-32-bytes";

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

#[derive(Serialize)]
struct Claims {
    sub: String,
    iss: &'static str,
    aud: &'static str,
    exp: usize,
    iat: usize,
    principal_type: &'static str,
    #[serde(rename = "type")]
    token_type: &'static str,
    version: &'static str,
    scope: String,
}

fn token(principal_id: Uuid, scope: &str) -> String {
    let now = chrono::Utc::now().timestamp() as usize;
    encode(
        &Header::new(Algorithm::HS256),
        &Claims {
            sub: principal_id.to_string(),
            iss: "auth-service",
            aud: "svc-workflow",
            exp: now + 300,
            iat: now,
            principal_type: "agent",
            token_type: "access",
            version: "v1",
            scope: scope.to_string(),
        },
        &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
    )
    .unwrap()
}

fn app(pool: sqlx::PgPool) -> axum::Router {
    let hs256 = Hs256Config {
        secret: JWT_SECRET.to_string(),
        issuer: "auth-service".to_string(),
        audience: "svc-workflow".to_string(),
        clock_skew_seconds: 0,
    };
    let config = HttpConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        request_body_max_bytes: 2_097_152,
        request_timeout_seconds: 30,
        auth_mode: AuthMode::TestHs256,
        hs256_config: Some(hs256),
        jwks_config: None,
        provisioning_config: ProvisioningConfig::new(Vec::new()),
    };
    http::router(AppState::new(pool, &config), &config)
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
    let app = app(pool);

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
}

#[tokio::test]
async fn missing_workflow_read_scope_returns_403() {
    let pool = create_pool().await;
    let principal_id = seed_second_principal(&pool).await;
    let app = app(pool);
    let no_read_token = token(principal_id, "workflow.execute");

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

    let app = app(pool.clone());
    let actor_token = token(assignee, "workflow.read");

    // Creator needs domain membership
    domain_membership(&pool, domain_id, creator).await;

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

    let app = app(pool.clone());
    let other_token = token(other_principal, "workflow.read");

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
async fn creator_owned_drafts_returns_only_own_drafts() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let outsider = seed_second_principal(&pool).await;
    let (version_id, _draft_advance, _, _) =
        seed_worklist_definition(&pool, domain_id, assignee).await;

    let app = app(pool.clone());
    let creator_token = token(creator, "workflow.read");

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
    let outsider_token = token(outsider, "workflow.read");
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

    let app = app(pool.clone());
    let creator_token = token(creator, "workflow.read");

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
async fn cross_domain_isolation() {
    let pool = create_pool().await;
    let (_owner1, domain1) = seed_principal_domain_with_owner(&pool).await;
    let (_owner2, domain2) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;

    let (ver1, adv1, _, _) = seed_worklist_definition(&pool, domain1, assignee).await;
    let (ver2, adv2, _, _) = seed_worklist_definition(&pool, domain2, assignee).await;

    let app = app(pool.clone());
    let assignee_token = token(assignee, "workflow.read");

    domain_membership(&pool, domain1, creator).await;
    domain_membership(&pool, domain2, creator).await;

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

    let app = app(pool.clone());
    let agent_token = token(assignee, "workflow.read workflow.execute");

    domain_membership(&pool, domain_id, creator).await;
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

    let app = app(pool.clone());

    domain_membership(&pool, domain_id, creator).await;
    create_and_advance(&pool, creator, domain_id, version_id, draft_advance).await;

    // An outsider uses a valid token but also includes query params that would
    // try to specify a different actor. The handler must ignore them entirely.
    let outsider_token = token(outsider, "workflow.read");

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
    let app = app(pool);
    let actor_token = token(principal_id, "workflow.read");

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

    let app = app(pool.clone());
    let assignee_token = token(assignee, "workflow.read");

    domain_membership(&pool, domain_id, creator).await;

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
    let app = app(pool);
    let actor_token = token(principal_id, "workflow.read");

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

    let resp = app
        .clone()
        .oneshot(request(
            "GET",
            "/internal/v1/worklists/creator-owned-drafts",
            Some(&actor_token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 0);
    assert!(body["next_cursor"].is_null());
}
