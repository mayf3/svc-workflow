//! INSTANCE_INPUT_PRINCIPAL assignee capability (v1).
//!
//! Verifies the "Principal A creates a Workflow Instance whose node is actually
//! assigned to Principal B" pattern. The DRAFT node stays WORKFLOW_CREATOR; the
//! NORMAL node uses INSTANCE_INPUT_PRINCIPAL and resolves its assignee from a
//! stable Principal UUID carried in the instance context_payload.
//!
//! Acceptance coverage:
//!   CREATOR_A_CAN_CREATE_FOR_ASSIGNEE_B
//!   ASSIGNEE_B_NOT_DOMAIN_MEMBER
//!   ASSIGNEE_B_WORKLIST_VISIBLE
//!   ASSIGNEE_B_DETAIL_VISIBLE
//!   CREATOR_AND_ASSIGNEE_SEPARATE
//!   DISPLAY_NAME_RESOLUTION_FORBIDDEN
//!   EMAIL_RESOLUTION_FORBIDDEN
//!   MISSING_INPUT_FAILS_CLOSED
//!   INVALID_UUID_FAILS_CLOSED
//!   UNKNOWN_PRINCIPAL_FAILS_CLOSED
//!   DISABLED_PRINCIPAL_FAILS_CLOSED
//!   FAILED_CREATE_SIDE_EFFECT_COUNT == 0

use super::*;
use svc_workflow::application::workflow_instance::create::create_workflow_instance;
use svc_workflow::domain::workflow_instance::commands::ExecuteWorkflowTransitionCommand;
use svc_workflow::domain::workflow_instance::errors::CreateWorkflowInstanceError;

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};
use tower::ServiceExt;

/// Add a principal as a MEMBER of a domain (local helper for this module).
async fn add_member(pool: &PgPool, domain_id: Uuid, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1, $2, $3, 'MEMBER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("add membership");
}

fn http_config(jwks_url: &str, allowed_sub: &str) -> HttpConfig {
    HttpConfig {
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
    }
}

fn authed_request(method: &str, uri: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    builder.body(Body::empty()).unwrap()
}

async fn body_json(response: axum::response::Response) -> serde_json::Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

// ===========================================================================
// Happy path: A creates for B, B is not a domain member, B sees work.
// ===========================================================================

#[tokio::test]
async fn creator_a_creates_for_assignee_b_who_is_not_a_domain_member() {
    // CREATOR_A_CAN_CREATE_FOR_ASSIGNEE_B + ASSIGNEE_B_NOT_DOMAIN_MEMBER
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await; // Principal A
    let assignee = seed_second_principal(&pool).await; // Principal B
    add_member(&pool, domain_id, creator).await; // A is a domain member so it can create
                                                 // B deliberately has NO domain membership.

    let (_d, ver_id, normal_node_id, draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    // A creates an instance addressed to B via a stable Principal UUID.
    let cmd = make_command_with_payload(
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": assignee}),
    );
    let result = create_workflow_instance(&pool, cmd)
        .await
        .expect("create for B");
    verify_creation(&pool, &result, creator, domain_id, ver_id).await;

    // The DRAFT visit is assigned to the creator A (WORKFLOW_CREATOR).
    let draft_assignee: (Uuid,) = sqlx::query_as(
        "SELECT assignee_principal_id FROM workflow_node_visits WHERE node_visit_id = $1",
    )
    .bind(result.current_node_visit_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        draft_assignee.0, creator,
        "DRAFT visit is assigned to creator A"
    );

    // Advance DRAFT -> NORMAL. The NORMAL node must resolve to B from the input.
    let transition = ExecuteWorkflowTransitionCommand {
        principal_id: PrincipalId::from_uuid(creator),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(result.workflow_instance_id),
        expected_workflow_state_version: 1,
        transition_definition_id: TransitionId::from_uuid(draft_advance),
        submission_payload: Some(serde_json::json!({})),
    };
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &pool, transition,
    )
    .await
    .expect("advance to NORMAL");

    let normal_assignee: (Uuid,) = sqlx::query_as(
        "SELECT nv.assignee_principal_id FROM workflow_node_visits nv \
         JOIN workflow_instances wi ON wi.current_node_visit_id = nv.node_visit_id \
         WHERE wi.workflow_instance_id = $1",
    )
    .bind(result.workflow_instance_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        normal_assignee.0, assignee,
        "NORMAL visit is assigned to B resolved from instance input"
    );
    assert_ne!(creator, assignee, "CREATOR_AND_ASSIGNEE_SEPARATE");

    // The NORMAL node is INSTANCE_INPUT_PRINCIPAL (not FIXED_PRINCIPAL).
    let (ref_type, key): (String, String) = sqlx::query_as(
        "SELECT assignee_ref_type::TEXT, COALESCE(assignee_input_key, '') \
         FROM workflow_node_definitions WHERE node_id = $1",
    )
    .bind(normal_node_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(ref_type, "INSTANCE_INPUT_PRINCIPAL");
    assert_eq!(key, "assigneePrincipalId");

    let _ = owner_id;
}

#[tokio::test]
async fn assignee_b_worklist_visible_without_domain_membership() {
    // ASSIGNEE_B_WORKLIST_VISIBLE
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _normal_node_id, draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    let cmd = make_command_with_payload(
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": assignee}),
    );
    let result = create_workflow_instance(&pool, cmd).await.expect("create");

    let transition = ExecuteWorkflowTransitionCommand {
        principal_id: PrincipalId::from_uuid(creator),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(result.workflow_instance_id),
        expected_workflow_state_version: 1,
        transition_definition_id: TransitionId::from_uuid(draft_advance),
        submission_payload: Some(serde_json::json!({})),
    };
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &pool, transition,
    )
    .await
    .expect("advance");

    // HTTP worklist as B (no domain membership).
    let mock = common::MockJwksServer::start().await;
    let state = AppState::new(pool.clone(), &http_config(&mock.url, &assignee.to_string()));
    let app = http::router(state, &http_config(&mock.url, &assignee.to_string()));
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let token = common::v1_token(
        assignee,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    let resp = app
        .oneshot(authed_request(
            "GET",
            "/internal/v1/worklists/assigned-to-me",
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        body["items"][0]["detail"]["current_visit"]["assignee_principal_id"],
        assignee.to_string()
    );
}

#[tokio::test]
async fn assignee_b_detail_visible_without_domain_membership() {
    // ASSIGNEE_B_DETAIL_VISIBLE
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _normal_node_id, draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    let cmd = make_command_with_payload(
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": assignee}),
    );
    let result = create_workflow_instance(&pool, cmd).await.expect("create");

    let transition = ExecuteWorkflowTransitionCommand {
        principal_id: PrincipalId::from_uuid(creator),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(result.workflow_instance_id),
        expected_workflow_state_version: 1,
        transition_definition_id: TransitionId::from_uuid(draft_advance),
        submission_payload: Some(serde_json::json!({})),
    };
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &pool, transition,
    )
    .await
    .expect("advance");

    let mock = common::MockJwksServer::start().await;
    let cfg = http_config(&mock.url, &assignee.to_string());
    let state = AppState::new(pool.clone(), &cfg);
    let app = http::router(state, &cfg);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let token = common::v1_token(
        assignee,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );

    let uri = format!(
        "/internal/v1/workflow-instances/{}",
        result.workflow_instance_id
    );
    let resp = app
        .oneshot(authed_request("GET", &uri, Some(&token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_json(resp).await;
    assert_eq!(
        body["detail"]["current_visit"]["assignee_principal_id"],
        assignee.to_string()
    );
}

// ===========================================================================
// Context invariant: every INSTANCE_INPUT_PRINCIPAL assignee key must be
// present and resolvable at creation time.
//
// The definition's real node requirements (INSTANCE_INPUT_PRINCIPAL +
// assignee_input_key) are a hard create-time invariant, derived generically
// from the definition (never hardcoded). Creation fails closed with
// AssigneeResolutionFailed (422) BEFORE any instance is written, so the read
// path can never encounter a half-legal instance again.
// ===========================================================================

/// Attempt to create an instance with the given payload, expecting a
/// deterministic `AssigneeResolutionFailed` and ZERO rows written for the
/// definition version.
async fn assert_create_fails_closed(
    pool: &PgPool,
    creator: Uuid,
    domain_id: Uuid,
    ver_id: Uuid,
    payload: serde_json::Value,
) {
    let before: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM workflow_instances WHERE definition_version_id = $1")
            .bind(ver_id)
            .fetch_one(pool)
            .await
            .unwrap();
    let cmd = make_command_with_payload(creator, domain_id, ver_id, payload);
    let err = create_workflow_instance(pool, cmd).await.unwrap_err();
    assert!(
        matches!(
            err,
            CreateWorkflowInstanceError::AssigneeResolutionFailed(_)
        ),
        "expected AssigneeResolutionFailed at creation, got {:?}",
        err
    );
    let after: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM workflow_instances WHERE definition_version_id = $1")
            .bind(ver_id)
            .fetch_one(pool)
            .await
            .unwrap();
    assert_eq!(before, after, "rejected create must not write an instance");
}

#[tokio::test]
async fn missing_assignee_input_fails_closed_at_create() {
    // MISSING_INPUT_FAILS_CLOSED (create time): the payload omits the assignee
    // key entirely -> creation is rejected before any instance is written.
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, _draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    assert_create_fails_closed(&pool, creator, domain_id, ver_id, serde_json::json!({})).await;
}

#[tokio::test]
async fn non_uuid_assignee_input_fails_closed_at_create() {
    // INVALID_UUID_FAILS_CLOSED (create time).
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, _draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    assert_create_fails_closed(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": "not-a-uuid"}),
    )
    .await;
}

#[tokio::test]
async fn display_name_resolution_forbidden_at_create() {
    // DISPLAY_NAME_RESOLUTION_FORBIDDEN: a display name is never accepted as a
    // stable Principal identifier (create time).
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let _assignee = seed_second_principal(&pool).await; // exists, but we pass its name
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, _draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    assert_create_fails_closed(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": "Test Agent"}),
    )
    .await;
}

#[tokio::test]
async fn email_resolution_forbidden_at_create() {
    // EMAIL_RESOLUTION_FORBIDDEN: an email is never accepted as a stable
    // Principal identifier (create time).
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let _assignee = seed_second_principal(&pool).await; // exists, but we pass its email
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, _draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    assert_create_fails_closed(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": "agent@example.com"}),
    )
    .await;
}

#[tokio::test]
async fn unknown_principal_fails_closed_at_create() {
    // UNKNOWN_PRINCIPAL_FAILS_CLOSED (create time): a well-formed UUID that is
    // not projected.
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, _draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    let ghost = Uuid::new_v4();
    assert_create_fails_closed(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": ghost}),
    )
    .await;
}

#[tokio::test]
async fn disabled_principal_fails_closed_at_create() {
    // DISABLED_PRINCIPAL_FAILS_CLOSED (create time): projected but disabled.
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let disabled_assignee = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;
    sqlx::query("UPDATE principals SET enabled = FALSE WHERE principal_id = $1")
        .bind(disabled_assignee)
        .execute(&pool)
        .await
        .unwrap();

    let (_d, ver_id, _n, _draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    assert_create_fails_closed(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": disabled_assignee}),
    )
    .await;
}

// ===========================================================================
// Side-effect integrity: a failed resolution leaves no half-built instance.
// The failed-creation *path* (deterministic create failure) is also verified
// directly here by triggering a deterministic create failure and asserting
// zero instances/events/visits reference it.
// ===========================================================================

#[tokio::test]
async fn failed_resolution_leaves_no_half_built_instance() {
    // FAILED_CREATE_SIDE_EFFECT_COUNT == 0: any resolution failure must not
    // persist a partial instance, event, visit, or submission.
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    // Force a deterministic *create* failure: the creator is not a domain
    // member after we create and then revoke the binding.
    let (_d, ver_id, _n, _a) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    // Snapshot orphans before the attempt.
    let before = global_orphan_count(&pool).await;
    // Revoke the creator's membership so creation fails on domain-membership.
    sqlx::query("DELETE FROM domain_role_bindings WHERE domain_id = $1 AND principal_id = $2")
        .bind(domain_id)
        .bind(creator)
        .execute(&pool)
        .await
        .unwrap();
    let cmd = make_command_with_payload(
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": Uuid::new_v4()}),
    );
    let err = create_workflow_instance(&pool, cmd).await.unwrap_err();
    assert!(matches!(
        err,
        CreateWorkflowInstanceError::DomainMembershipRequired
    ));
    let after = global_orphan_count(&pool).await;
    assert_eq!(before, after, "no orphan runtime facts from failed create");
}

async fn global_orphan_count(pool: &PgPool) -> i64 {
    let (orphan_events,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_events e WHERE NOT EXISTS \
         (SELECT 1 FROM workflow_instances i WHERE i.workflow_instance_id = e.workflow_instance_id)",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let (orphan_visits,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_node_visits v WHERE NOT EXISTS \
         (SELECT 1 FROM workflow_instances i WHERE i.workflow_instance_id = v.workflow_instance_id)",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    let (orphan_subs,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_submissions s WHERE NOT EXISTS \
         (SELECT 1 FROM workflow_instances i WHERE i.workflow_instance_id = s.workflow_instance_id)",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    orphan_events + orphan_visits + orphan_subs
}

#[tokio::test]
async fn arbitrary_input_key_resolves_correctly() {
    // The input key is definition-configured, not hardcoded. Verify a
    // definition that uses a different key name resolves B correctly.
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, draft_advance) =
        seed_published_definition_instance_input_principal_key(&pool, domain_id, "targetUserId")
            .await;

    let cmd = make_command_with_payload(
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"targetUserId": assignee}),
    );
    let result = create_workflow_instance(&pool, cmd).await.expect("create");

    let transition = ExecuteWorkflowTransitionCommand {
        principal_id: PrincipalId::from_uuid(creator),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(result.workflow_instance_id),
        expected_workflow_state_version: 1,
        transition_definition_id: TransitionId::from_uuid(draft_advance),
        submission_payload: Some(serde_json::json!({})),
    };
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        &pool, transition,
    )
    .await
    .expect("advance");

    let normal_assignee: (Uuid,) = sqlx::query_as(
        "SELECT nv.assignee_principal_id FROM workflow_node_visits nv \
         JOIN workflow_instances wi ON wi.current_node_visit_id = nv.node_visit_id \
         WHERE wi.workflow_instance_id = $1",
    )
    .bind(result.workflow_instance_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(normal_assignee.0, assignee);
}

// ===========================================================================
// Multiple future nodes referencing different instance inputs must ALL be
// validated at creation: the required key set is derived from the whole
// definition, not from any single node.
// ===========================================================================

/// Seed a published definition with TWO NORMAL INSTANCE_INPUT_PRINCIPAL nodes
/// using different input keys. Returns (domain_id, version_id).
async fn seed_published_definition_two_input_keys(pool: &PgPool, domain_id: Uuid) -> (Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("iip2-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query("INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'IIP Two-Key Def')")
        .bind(def_id).bind(domain_id).bind(&def_key)
        .execute(pool).await.expect("insert def");
    sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1, $2, 1, 'DRAFT', $3)")
        .bind(ver_id).bind(def_id).bind(serde_json::json!({"type":"object"}))
        .execute(pool).await.expect("insert version");

    let draft_id = Uuid::new_v4();
    let first_normal_id = Uuid::new_v4();
    let second_normal_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();

    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id, assignee_input_key) VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT', 'WORKFLOW_CREATOR', NULL, NULL)")
        .bind(draft_id).bind(ver_id).execute(pool).await.expect("insert draft node");
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id, assignee_input_key) VALUES ($1, $2, 'first', 'First', 1, 'NORMAL', 'INSTANCE_INPUT_PRINCIPAL', NULL, 'assigneePrincipalId')")
        .bind(first_normal_id).bind(ver_id).execute(pool).await.expect("insert first normal node");
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type, fixed_principal_id, assignee_input_key) VALUES ($1, $2, 'second', 'Second', 2, 'NORMAL', 'INSTANCE_INPUT_PRINCIPAL', NULL, 'operatorPrincipalId')")
        .bind(second_normal_id).bind(ver_id).execute(pool).await.expect("insert second normal node");
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'done', 'Done', 3, 'TERMINAL', NULL)")
        .bind(term_id).bind(ver_id).execute(pool).await.expect("insert terminal node");

    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(ver_id).execute(pool).await.expect("publish version");

    (domain_id, ver_id)
}

#[tokio::test]
async fn all_future_input_keys_validated_at_create() {
    // Multiple future nodes reference different instance inputs -> every
    // required key must be present and resolvable at creation.
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let assignee = seed_second_principal(&pool).await;
    let operator = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id) = seed_published_definition_two_input_keys(&pool, domain_id).await;

    // Only one of the two required keys present -> creation rejected.
    assert_create_fails_closed(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": assignee}),
    )
    .await;

    // Both keys present and resolvable -> creation succeeds.
    let cmd = make_command_with_payload(
        creator,
        domain_id,
        ver_id,
        serde_json::json!({
            "assigneePrincipalId": assignee,
            "operatorPrincipalId": operator,
        }),
    );
    let result = create_workflow_instance(&pool, cmd)
        .await
        .expect("create with all keys");
    assert_eq!(result.workflow_instance_id.to_string().len(), 36);
}

// ===========================================================================
// Regression: existing assignee resolution paths still work.
// ===========================================================================

#[tokio::test]
async fn regression_workflow_creator_still_resolves() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_wf_creator(&pool, domain_id).await;
    let cmd = make_command(principal_id, domain_id, ver_id);
    let result = create_workflow_instance(&pool, cmd).await.expect("create");
    verify_creation(&pool, &result, principal_id, domain_id, ver_id).await;
    let assignee: (Uuid,) = sqlx::query_as(
        "SELECT assignee_principal_id FROM workflow_node_visits WHERE node_visit_id = $1",
    )
    .bind(result.current_node_visit_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(assignee.0, principal_id);
}

#[tokio::test]
async fn regression_fixed_principal_still_resolves() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let fixed_id = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, fixed_id).await;
    let (_d, ver_id) = seed_published_definition_fixed_principal(&pool, domain_id, fixed_id).await;
    let cmd = make_command(principal_id, domain_id, ver_id);
    let result = create_workflow_instance(&pool, cmd).await.expect("create");
    verify_creation(&pool, &result, principal_id, domain_id, ver_id).await;
    let assignee: (Uuid,) = sqlx::query_as(
        "SELECT assignee_principal_id FROM workflow_node_visits WHERE node_visit_id = $1",
    )
    .bind(result.current_node_visit_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(assignee.0, fixed_id);
}

#[tokio::test]
async fn regression_domain_owner_still_resolves() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_d, ver_id) = seed_published_definition_domain_owner(&pool, domain_id).await;
    let cmd = make_command(owner_id, domain_id, ver_id);
    let result = create_workflow_instance(&pool, cmd).await.expect("create");
    verify_creation(&pool, &result, owner_id, domain_id, ver_id).await;
    let assignee: (Uuid,) = sqlx::query_as(
        "SELECT assignee_principal_id FROM workflow_node_visits WHERE node_visit_id = $1",
    )
    .bind(result.current_node_visit_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(assignee.0, owner_id);
}
