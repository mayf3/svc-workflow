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

/// Per-instance side-effect snapshot: (visit_count, event_count, submission_count).
/// A failed transition must not add any of these beyond what existed before.
async fn instance_side_effects(pool: &PgPool, instance_id: Uuid) -> (i64, i64, i64) {
    let (visits,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_node_visits WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let (events,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_events WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .unwrap();
    let (submissions,): (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_submissions WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .unwrap();
    (visits, events, submissions)
}

/// Create an instance (DRAFT) for the INSTANCE_INPUT_PRINCIPAL definition with
/// the given payload, as creator A. Returns the instance id.
async fn create_draft_instance(
    pool: &PgPool,
    creator: Uuid,
    domain_id: Uuid,
    ver_id: Uuid,
    payload: serde_json::Value,
) -> Uuid {
    let cmd = make_command_with_payload(creator, domain_id, ver_id, payload);
    let result = create_workflow_instance(&pool, cmd).await.expect("create draft");
    result.workflow_instance_id
}

/// Attempt the DRAFT->NORMAL advance as creator A, expecting it to fail closed
/// because the INSTANCE_INPUT_PRINCIPAL node cannot resolve a valid assignee.
async fn assert_advance_fails_closed(
    pool: &PgPool,
    creator: Uuid,
    instance_id: Uuid,
    draft_advance: Uuid,
) {
    let before = instance_side_effects(pool, instance_id).await;
    let transition = ExecuteWorkflowTransitionCommand {
        principal_id: PrincipalId::from_uuid(creator),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
        expected_workflow_state_version: 1,
        transition_definition_id: TransitionId::from_uuid(draft_advance),
        submission_payload: Some(serde_json::json!({})),
    };
    let err = svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        pool, transition,
    )
    .await
    .unwrap_err();
    assert!(
        matches!(
            err,
            svc_workflow::domain::workflow_instance::errors::ExecuteWorkflowTransitionError::AssigneeResolutionFailed(_)
        ),
        "expected AssigneeResolutionFailed, got {:?}",
        err
    );
    let after = instance_side_effects(pool, instance_id).await;
    assert_eq!(
        before, after,
        "FAILED_CREATE_SIDE_EFFECT_COUNT: failed resolution must not persist new visits/events/submissions"
    );
    // Instance stays at state version 1 (DRAFT) — the transition did not commit.
    let (sv,): (i32,) = sqlx::query_as(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(sv, 1, "instance state version must remain 1 after failed advance");
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
    let result = create_workflow_instance(&pool, cmd).await.expect("create for B");
    verify_creation(&pool, &result, creator, domain_id, ver_id).await;

    // The DRAFT visit is assigned to the creator A (WORKFLOW_CREATOR).
    let draft_assignee: (Uuid,) = sqlx::query_as(
        "SELECT assignee_principal_id FROM workflow_node_visits WHERE node_visit_id = $1",
    )
    .bind(result.current_node_visit_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(draft_assignee.0, creator, "DRAFT visit is assigned to creator A");

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
    let token = common::v1_token(assignee, "workflow.read", "test-client", 300, &mock.key_pair);

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
    let token = common::v1_token(assignee, "workflow.read", "test-client", 300, &mock.key_pair);

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
// Fail-closed: resolution only accepts stable Principal UUIDs.
//
// The INSTANCE_INPUT_PRINCIPAL assignee is resolved when the workflow enters the
// NORMAL node (the DRAFT node is always WORKFLOW_CREATOR). So a fail-closed
// resolution surfaces during the DRAFT -> NORMAL advance, which is itself an
// atomic transaction: on failure no new visit / event / submission is persisted
// and the instance stays at state version 1 (DRAFT).
// ===========================================================================

#[tokio::test]
async fn missing_assignee_input_fails_closed() {
    // MISSING_INPUT_FAILS_CLOSED
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    // Payload omits the assignee key entirely.
    let instance_id =
        create_draft_instance(&pool, creator, domain_id, ver_id, serde_json::json!({})).await;
    assert_advance_fails_closed(&pool, creator, instance_id, draft_advance).await;
}

#[tokio::test]
async fn non_uuid_assignee_input_fails_closed() {
    // INVALID_UUID_FAILS_CLOSED
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    let instance_id = create_draft_instance(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": "not-a-uuid"}),
    )
    .await;
    assert_advance_fails_closed(&pool, creator, instance_id, draft_advance).await;
}

#[tokio::test]
async fn display_name_resolution_forbidden() {
    // DISPLAY_NAME_RESOLUTION_FORBIDDEN: a display name is never accepted as a
    // stable Principal identifier.
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let _assignee = seed_second_principal(&pool).await; // exists, but we pass its name
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    let instance_id = create_draft_instance(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": "Test Agent"}),
    )
    .await;
    assert_advance_fails_closed(&pool, creator, instance_id, draft_advance).await;
}

#[tokio::test]
async fn email_resolution_forbidden() {
    // EMAIL_RESOLUTION_FORBIDDEN: an email is never accepted as a stable
    // Principal identifier.
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let _assignee = seed_second_principal(&pool).await; // exists, but we pass its email
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    let instance_id = create_draft_instance(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": "agent@example.com"}),
    )
    .await;
    assert_advance_fails_closed(&pool, creator, instance_id, draft_advance).await;
}

#[tokio::test]
async fn unknown_principal_fails_closed() {
    // UNKNOWN_PRINCIPAL_FAILS_CLOSED: a well-formed UUID that is not projected.
    let pool = create_pool().await;
    let (_owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    add_member(&pool, domain_id, creator).await;

    let (_d, ver_id, _n, draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    let ghost = Uuid::new_v4();
    let instance_id = create_draft_instance(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": ghost}),
    )
    .await;
    assert_advance_fails_closed(&pool, creator, instance_id, draft_advance).await;
}

#[tokio::test]
async fn disabled_principal_fails_closed() {
    // DISABLED_PRINCIPAL_FAILS_CLOSED: projected but disabled.
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

    let (_d, ver_id, _n, draft_advance) =
        seed_published_definition_instance_input_principal(&pool, domain_id).await;

    let instance_id = create_draft_instance(
        &pool,
        creator,
        domain_id,
        ver_id,
        serde_json::json!({"assigneePrincipalId": disabled_assignee}),
    )
    .await;
    assert_advance_fails_closed(&pool, creator, instance_id, draft_advance).await;
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
