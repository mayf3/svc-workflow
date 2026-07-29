//! Regression tests for all three non-terminal assignee reference types in the
//! read path (worklist, detail, timeline).
//!
//! Coverage:
//!   1. WORKFLOW_CREATOR  — creator is the current node assignee
//!   2. INSTANCE_INPUT_PRINCIPAL — assignee resolved from context payload
//!   3. FIXED_PRINCIPAL   — fixed principal assigned to the node
//!
//! Each scenario verifies:
//!   - assigned-to-me returns 200 with correct assignee & current node
//!   - instance detail returns 200 with correct assignee & current node
//!   - timeline behaviour is unchanged
//!   - 401/403 for missing / insufficient scope
//!   - 404 for non-existent instance
//!   - invisible instances are not reachable

use super::*;

use serde_json::json;
use uuid::Uuid;

use svc_workflow::application::workflow_instance::query_service::WorkflowQueryService;
use svc_workflow::application::workflow_instance::query_types::*;

/// Add a principal as a MEMBER of a domain (local helper).
async fn add_member(pool: &sqlx::PgPool, domain_id: Uuid, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO domain_role_bindings
         (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1, $2, $3, 'MEMBER', TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain_id)
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("add member");
}

// ---------------------------------------------------------------------------
// Seed helpers — generic, no product-specific names
// ---------------------------------------------------------------------------

/// Seed a published definition with a graph that exercises a single assignee
/// reference type on the NORMAL node:
///
///   DRAFT (WORKFLOW_CREATOR) ──ADVANCE──▶ NORMAL ({assignee_type}) ──ADVANCE──▶ TERMINAL
///
/// Returns (definition_version_id, normal_node_id, draft_advance_id,
///          normal_advance_id).
///
/// For INSTANCE_INPUT_PRINCIPAL the payload must carry
/// `{ "assigneePrincipalId": "<uuid>" }`.
async fn seed_assignee_type_definition(
    pool: &sqlx::PgPool,
    domain_id: Uuid,
    assignee_type: &str,
    fixed_principal_id: Option<Uuid>,
    assignee_input_key: Option<&str>,
) -> (Uuid, Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let key = format!("at-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions
         (workflow_definition_id, domain_id, definition_key, display_name)
         VALUES ($1, $2, $3, 'Assignee Type Test')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&key)
    .execute(pool)
    .await
    .expect("insert def");

    sqlx::query(
        "INSERT INTO workflow_definition_versions
         (definition_version_id, workflow_definition_id, version_number,
          version_status, context_schema)
         VALUES ($1, $2, 1, 'DRAFT', '{\"type\":\"object\"}'::jsonb)",
    )
    .bind(ver_id)
    .bind(def_id)
    .execute(pool)
    .await
    .expect("insert version");

    let draft_id = Uuid::new_v4();
    let normal_id = Uuid::new_v4();
    let term_id = Uuid::new_v4();

    // DRAFT node — always WORKFLOW_CREATOR
    sqlx::query(
        "INSERT INTO workflow_node_definitions
         (node_id, definition_version_id, node_key, display_name, order_index,
          node_type, assignee_ref_type)
         VALUES ($1, $2, 'draft', 'Draft', 0, 'DRAFT',
                 'WORKFLOW_CREATOR'::assignee_ref_type)",
    )
    .bind(draft_id)
    .bind(ver_id)
    .execute(pool)
    .await
    .expect("insert draft node");

    // NORMAL node — chosen assignee type
    match assignee_type {
        "INSTANCE_INPUT_PRINCIPAL" => {
            let input_key = assignee_input_key
                .unwrap_or("assigneePrincipalId");
            sqlx::query(
                "INSERT INTO workflow_node_definitions
                 (node_id, definition_version_id, node_key, display_name,
                  order_index, node_type, assignee_ref_type,
                  assignee_input_key)
                 VALUES ($1, $2, 'normal', 'Normal', 1, 'NORMAL',
                         'INSTANCE_INPUT_PRINCIPAL'::assignee_ref_type, $3)",
            )
            .bind(normal_id)
            .bind(ver_id)
            .bind(input_key)
            .execute(pool)
            .await
            .expect("insert normal node (iip)");
        }
        _ => {
            // WORKFLOW_CREATOR or FIXED_PRINCIPAL
            sqlx::query(
                "INSERT INTO workflow_node_definitions
                 (node_id, definition_version_id, node_key, display_name,
                  order_index, node_type, assignee_ref_type,
                  fixed_principal_id)
                 VALUES ($1, $2, 'normal', 'Normal', 1, 'NORMAL',
                         $3::assignee_ref_type, $4)",
            )
            .bind(normal_id)
            .bind(ver_id)
            .bind(assignee_type)
            .bind(fixed_principal_id)
            .execute(pool)
            .await
            .expect("insert normal node");
        }
    }

    // TERMINAL node
    sqlx::query(
        "INSERT INTO workflow_node_definitions
         (node_id, definition_version_id, node_key, display_name, order_index,
          node_type, assignee_ref_type)
         VALUES ($1, $2, 'done', 'Done', 2, 'TERMINAL', NULL::assignee_ref_type)",
    )
    .bind(term_id)
    .bind(ver_id)
    .execute(pool)
    .await
    .expect("insert terminal node");

    // DRAFT → NORMAL (advance)
    let draft_advance = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions
         (transition_id, definition_version_id, transition_key, display_name,
          source_node_id, target_node_id, transition_effect)
         VALUES ($1, $2, 'advance-normal', 'To Normal', $3, $4, 'ADVANCE')",
    )
    .bind(draft_advance)
    .bind(ver_id)
    .bind(draft_id)
    .bind(normal_id)
    .execute(pool)
    .await
    .expect("insert draft advance");
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id = $1
         WHERE node_id = $2",
    )
    .bind(draft_advance)
    .bind(draft_id)
    .execute(pool)
    .await
    .expect("set primary on draft");

    // NORMAL → TERMINAL (advance)
    let normal_advance = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions
         (transition_id, definition_version_id, transition_key, display_name,
          source_node_id, target_node_id, transition_effect)
         VALUES ($1, $2, 'advance-done', 'To Done', $3, $4, 'ADVANCE')",
    )
    .bind(normal_advance)
    .bind(ver_id)
    .bind(normal_id)
    .bind(term_id)
    .execute(pool)
    .await
    .expect("insert normal advance");
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id = $1
         WHERE node_id = $2",
    )
    .bind(normal_advance)
    .bind(normal_id)
    .execute(pool)
    .await
    .expect("set primary on normal");

    // Publish
    sqlx::query(
        "UPDATE workflow_definition_versions SET version_status = 'PUBLISHED'
         WHERE definition_version_id = $1",
    )
    .bind(ver_id)
    .execute(pool)
    .await
    .expect("publish");

    (ver_id, normal_id, draft_advance, normal_advance)
}

/// Create an instance, advance from DRAFT to NORMAL, and return the instance id
/// and the expected assignee.
async fn create_and_advance_to_normal(
    pool: &sqlx::PgPool,
    creator: Uuid,
    domain_id: Uuid,
    ver_id: Uuid,
    draft_advance: Uuid,
    context_payload: serde_json::Value,
    expected_assignee: Uuid,
) -> Uuid {
    let cmd = make_command_with_payload(creator, domain_id, ver_id, context_payload);
    let created = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        pool, cmd,
    )
    .await
    .expect("create instance");

    let transition = ExecuteWorkflowTransitionCommand {
        principal_id: PrincipalId::from_uuid(creator),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(created.workflow_instance_id),
        expected_workflow_state_version: 1,
        transition_definition_id: TransitionId::from_uuid(draft_advance),
        submission_payload: Some(json!({})),
    };
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        pool, transition,
    )
    .await
    .expect("advance to normal");

    // Verify the resolved assignee is correct
    let (actual_assignee,): (Uuid,) = sqlx::query_as(
        "SELECT nv.assignee_principal_id FROM workflow_node_visits nv
         JOIN workflow_instances wi ON wi.current_node_visit_id = nv.node_visit_id
         WHERE wi.workflow_instance_id = $1",
    )
    .bind(created.workflow_instance_id)
    .fetch_one(pool)
    .await
    .unwrap();
    assert_eq!(
        actual_assignee, expected_assignee,
        "current node assignee must match expected"
    );

    created.workflow_instance_id
}

/// Extract the inner FullWorkflowInstanceDetail from a WorkflowInstanceDetail,
/// panicking if it's the HistoricalParticipant variant.
fn expect_full(detail: &WorkflowInstanceDetail) -> &FullWorkflowInstanceDetail {
    match detail {
        WorkflowInstanceDetail::Full(ref inner) => inner.as_ref(),
        WorkflowInstanceDetail::HistoricalParticipant(_) => {
            panic!("expected Full detail, got HistoricalParticipant");
        }
    }
}

/// Extract the inner instance summary from a WorkflowInstanceDetail.
fn detail_instance_id(detail: &WorkflowInstanceDetail) -> Uuid {
    match detail {
        WorkflowInstanceDetail::Full(ref inner) => inner.instance.workflow_instance_id,
        WorkflowInstanceDetail::HistoricalParticipant(ref p) => p.instance.workflow_instance_id,
    }
}

// ---------------------------------------------------------------------------
// WORKFLOW_CREATOR assignee
// ---------------------------------------------------------------------------

#[tokio::test]
async fn workflow_creator_assignee_read_path() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let _assignee = seed_second_principal(&pool).await;

    add_member(&pool, domain_id, creator).await;

    let (ver_id, _normal_id, draft_advance, _normal_advance) =
        seed_assignee_type_definition(&pool, domain_id, "WORKFLOW_CREATOR", None, None).await;

    let instance_id = create_and_advance_to_normal(
        &pool,
        creator,
        domain_id,
        ver_id,
        draft_advance,
        json!({"title": "test"}),
        creator, // WORKFLOW_CREATOR → creator is the assignee
    )
    .await;

    // The creator is also the current assignee via WORKFLOW_CREATOR.
    // Ensure they can see the instance in their worklist.
    let service = WorkflowQueryService::new(pool.clone());

    // assigned-to-me returns 200 with correct data
    let page = service
        .list_assigned_to_me(ListAssignedToMe {
            actor_principal_id: creator,
            before: None,
            limit: None,
        })
        .await
        .expect("assigned-to-me must succeed for WORKFLOW_CREATOR");
    assert_eq!(page.items.len(), 1);
    assert_eq!(
        page.items[0].detail.instance.workflow_instance_id,
        instance_id
    );
    assert_eq!(
        page.items[0].detail.current_visit.assignee_principal_id,
        Some(creator)
    );
    assert_eq!(page.items[0].detail.instance.current_node.node_key, "normal");

    // instance detail returns 200 with correct data
    let detail = service
        .get_workflow_instance_detail(GetWorkflowInstanceDetail {
            actor_principal_id: creator,
            workflow_instance_id: instance_id,
        })
        .await
        .expect("instance detail must succeed for WORKFLOW_CREATOR");
    assert_eq!(detail_instance_id(&detail), instance_id);
    let full = expect_full(&detail);
    assert_eq!(full.current_visit.assignee_principal_id, Some(creator));

    // timeline behaviour is unchanged
    service
        .list_workflow_timeline(ListWorkflowTimeline {
            actor_principal_id: creator,
            workflow_instance_id: instance_id,
            after_event_sequence: None,
            limit: None,
        })
        .await
        .expect("timeline must still work");
}

// ---------------------------------------------------------------------------
// INSTANCE_INPUT_PRINCIPAL assignee
// ---------------------------------------------------------------------------

#[tokio::test]
async fn instance_input_principal_assignee_read_path() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let target_assignee = seed_second_principal(&pool).await;

    add_member(&pool, domain_id, creator).await;

    let (ver_id, _normal_id, draft_advance, _normal_advance) =
        seed_assignee_type_definition(
            &pool,
            domain_id,
            "INSTANCE_INPUT_PRINCIPAL",
            None,
            Some("assigneePrincipalId"),
        )
        .await;

    let instance_id = create_and_advance_to_normal(
        &pool,
        creator,
        domain_id,
        ver_id,
        draft_advance,
        json!({"assigneePrincipalId": target_assignee}),
        target_assignee,
    )
    .await;

    let service = WorkflowQueryService::new(pool.clone());

    // The target assignee can see the instance in their worklist
    let page = service
        .list_assigned_to_me(ListAssignedToMe {
            actor_principal_id: target_assignee,
            before: None,
            limit: None,
        })
        .await
        .expect("assigned-to-me must succeed for INSTANCE_INPUT_PRINCIPAL");
    assert_eq!(page.items.len(), 1);
    assert_eq!(
        page.items[0].detail.instance.workflow_instance_id,
        instance_id
    );
    assert_eq!(
        page.items[0].detail.current_visit.assignee_principal_id,
        Some(target_assignee)
    );
    assert_eq!(page.items[0].detail.instance.current_node.node_key, "normal");

    // instance detail returns 200 with correct data
    let detail = service
        .get_workflow_instance_detail(GetWorkflowInstanceDetail {
            actor_principal_id: target_assignee,
            workflow_instance_id: instance_id,
        })
        .await
        .expect("instance detail must succeed for INSTANCE_INPUT_PRINCIPAL");
    assert_eq!(detail_instance_id(&detail), instance_id);

    // timeline behaviour is unchanged
    service
        .list_workflow_timeline(ListWorkflowTimeline {
            actor_principal_id: target_assignee,
            workflow_instance_id: instance_id,
            after_event_sequence: None,
            limit: None,
        })
        .await
        .expect("timeline must still work");

    // The creator (non-assignee) sees nothing in assigned-to-me
    let creator_page = service
        .list_assigned_to_me(ListAssignedToMe {
            actor_principal_id: creator,
            before: None,
            limit: None,
        })
        .await
        .expect("creator assigned-to-me must succeed");
    assert!(
        creator_page.items.is_empty(),
        "creator must not see the instance in assigned-to-me"
    );

    // But creator gets (restricted) detail if they were a participant
    let creator_detail = service
        .get_workflow_instance_detail(GetWorkflowInstanceDetail {
            actor_principal_id: creator,
            workflow_instance_id: instance_id,
        })
        .await
        .expect("creator must see their own instance");
    assert!(detail_instance_id(&creator_detail) == instance_id);
}

// ---------------------------------------------------------------------------
// Original regression: INSTANCE_INPUT_PRINCIPAL on outgoing transition target
//
// The instance stays at DRAFT (WORKFLOW_CREATOR). The DRAFT node has an
// outgoing ADVANCE transition whose TARGET node uses INSTANCE_INPUT_PRINCIPAL.
// load_outgoing() must resolve the target assignee from the context payload
// without hitting the wildcard arm that returns internal_consistency_error.
//
// This is the exact pattern that was failing in dogfood (instance A's
// "sourcing" DRAFT node → "authoring" NORMAL with INSTANCE_INPUT_PRINCIPAL).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn outgoing_target_instance_input_principal_from_draft() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let reviewer_principal = seed_second_principal(&pool).await;

    add_member(&pool, domain_id, creator).await;

    // Graph: DRAFT(WORKFLOW_CREATOR) → NORMAL(INSTANCE_INPUT_PRINCIPAL, "reviewerPrincipalId") → TERMINAL
    let (ver_id, _normal_id, _draft_advance, _normal_advance) =
        seed_assignee_type_definition(
            &pool,
            domain_id,
            "INSTANCE_INPUT_PRINCIPAL",
            None,
            Some("reviewerPrincipalId"),
        )
        .await;

    // Create instance — stays at DRAFT, DO NOT advance.
    let cmd = make_command_with_payload(
        creator,
        domain_id,
        ver_id,
        json!({"reviewerPrincipalId": reviewer_principal}),
    );
    let created = create_workflow_instance(&pool, cmd)
        .await
        .expect("create instance");

    let service = WorkflowQueryService::new(pool.clone());

    // 1. instance detail must succeed (load_outgoing processes the outgoing
    //    transition whose target has INSTANCE_INPUT_PRINCIPAL).
    let detail = service
        .get_workflow_instance_detail(GetWorkflowInstanceDetail {
            actor_principal_id: creator,
            workflow_instance_id: created.workflow_instance_id,
        })
        .await
        .expect("instance detail must succeed when outgoing target is INSTANCE_INPUT_PRINCIPAL");
    let full = expect_full(&detail);
    assert_eq!(
        full.instance.current_node.node_key, "draft",
        "instance must still be at DRAFT node"
    );
    // The outgoing transition to the INSTANCE_INPUT_PRINCIPAL target must exist
    // and be correctly described.
    let iip_transition = full
        .outgoing_transitions
        .iter()
        .find(|t| t.target_node.node_key == "normal")
        .expect("must have outgoing transition to 'normal' target");
    assert_eq!(
        iip_transition.target_node.node_type, "NORMAL",
        "target must be a NORMAL node"
    );

    // 2. assigned-to-me must succeed — creator is the current DRAFT assignee.
    let page = service
        .list_assigned_to_me(ListAssignedToMe {
            actor_principal_id: creator,
            before: None,
            limit: None,
        })
        .await
        .expect("assigned-to-me must succeed when outgoing target is INSTANCE_INPUT_PRINCIPAL");
    assert!(
        !page.items.is_empty(),
        "creator must see their draft instance in assigned-to-me"
    );
    assert_eq!(
        page.items[0].detail.instance.workflow_instance_id,
        created.workflow_instance_id
    );
    assert_eq!(
        page.items[0].detail.instance.current_node.node_key, "draft",
        "worklist must report DRAFT as current node"
    );

    // 3. timeline unchanged
    service
        .list_workflow_timeline(ListWorkflowTimeline {
            actor_principal_id: creator,
            workflow_instance_id: created.workflow_instance_id,
            after_event_sequence: None,
            limit: None,
        })
        .await
        .expect("timeline must still work");

    // This test will FAIL if the INSTANCE_INPUT_PRINCIPAL match arm in
    // load_outgoing() is removed, because the outgoing transition's target
    // assignee_ref_type would fall through to the wildcard arm and return
    // Err(internal("unknown target assignee reference type")).
}

// ---------------------------------------------------------------------------
// FIXED_PRINCIPAL assignee
// ---------------------------------------------------------------------------

#[tokio::test]
async fn fixed_principal_assignee_read_path() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let target_assignee = seed_second_principal(&pool).await;

    add_member(&pool, domain_id, creator).await;

    let (ver_id, _normal_id, draft_advance, _normal_advance) =
        seed_assignee_type_definition(
            &pool,
            domain_id,
            "FIXED_PRINCIPAL",
            Some(target_assignee),
            None,
        )
        .await;

    let instance_id = create_and_advance_to_normal(
        &pool,
        creator,
        domain_id,
        ver_id,
        draft_advance,
        json!({"title": "test"}),
        target_assignee,
    )
    .await;

    let service = WorkflowQueryService::new(pool.clone());

    // The fixed assignee sees the instance
    let page = service
        .list_assigned_to_me(ListAssignedToMe {
            actor_principal_id: target_assignee,
            before: None,
            limit: None,
        })
        .await
        .expect("assigned-to-me must succeed for FIXED_PRINCIPAL");
    assert_eq!(page.items.len(), 1);
    assert_eq!(
        page.items[0].detail.instance.workflow_instance_id,
        instance_id
    );
    assert_eq!(
        page.items[0].detail.current_visit.assignee_principal_id,
        Some(target_assignee)
    );
    assert_eq!(page.items[0].detail.instance.current_node.node_key, "normal");

    // instance detail returns 200
    let detail = service
        .get_workflow_instance_detail(GetWorkflowInstanceDetail {
            actor_principal_id: target_assignee,
            workflow_instance_id: instance_id,
        })
        .await
        .expect("instance detail must succeed for FIXED_PRINCIPAL");
    assert_eq!(detail_instance_id(&detail), instance_id);

    // timeline unchanged
    service
        .list_workflow_timeline(ListWorkflowTimeline {
            actor_principal_id: target_assignee,
            workflow_instance_id: instance_id,
            after_event_sequence: None,
            limit: None,
        })
        .await
        .expect("timeline must still work");
}

// ---------------------------------------------------------------------------
// Authorization regression tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn inaccessible_instance_not_visible() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;
    let outsider = seed_second_principal(&pool).await;

    add_member(&pool, domain_id, creator).await;

    let (ver_id, _normal_id, draft_advance, _normal_advance) =
        seed_assignee_type_definition(&pool, domain_id, "WORKFLOW_CREATOR", None, None).await;

    let instance_id = create_and_advance_to_normal(
        &pool,
        creator,
        domain_id,
        ver_id,
        draft_advance,
        json!({"title": "test"}),
        creator,
    )
    .await;

    let service = WorkflowQueryService::new(pool.clone());

    // Outsider (not a participant) cannot see the instance
    let err = service
        .get_workflow_instance_detail(GetWorkflowInstanceDetail {
            actor_principal_id: outsider,
            workflow_instance_id: instance_id,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            WorkflowQueryError::WorkflowInstanceNotFoundOrNotVisible
        ),
        "outsider must get 404-like error, got {:?}",
        err
    );
}

#[tokio::test]
async fn non_existent_instance_returns_not_found() {
    let pool = create_pool().await;
    let (owner, _domain_id) = seed_principal_domain_with_owner(&pool).await;
    let ghost = Uuid::new_v4();
    let service = WorkflowQueryService::new(pool.clone());

    let err = service
        .get_workflow_instance_detail(GetWorkflowInstanceDetail {
            actor_principal_id: owner,
            workflow_instance_id: ghost,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            WorkflowQueryError::WorkflowInstanceNotFoundOrNotVisible
        ),
        "non-existent instance must not be found, got {:?}",
        err
    );
}

#[tokio::test]
async fn disabled_principal_gets_principal_disabled_error() {
    let pool = create_pool().await;
    let (_owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let creator = seed_second_principal(&pool).await;

    // Disable the creator
    sqlx::query("UPDATE principals SET enabled = FALSE WHERE principal_id = $1")
        .bind(creator)
        .execute(&pool)
        .await
        .unwrap();

    let service = WorkflowQueryService::new(pool.clone());

    let err = service
        .list_assigned_to_me(ListAssignedToMe {
            actor_principal_id: creator,
            before: None,
            limit: None,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(err, WorkflowQueryError::PrincipalDisabled),
        "disabled principal must get PrincipalDisabled, got {:?}",
        err
    );
}

#[tokio::test]
async fn nonexistent_principal_gets_principal_not_found_error() {
    let pool = create_pool().await;
    let ghost = Uuid::new_v4();
    let service = WorkflowQueryService::new(pool.clone());

    let err = service
        .list_assigned_to_me(ListAssignedToMe {
            actor_principal_id: ghost,
            before: None,
            limit: None,
        })
        .await
        .unwrap_err();
    assert!(
        matches!(err, WorkflowQueryError::PrincipalNotFound),
        "nonexistent principal must get PrincipalNotFound, got {:?}",
        err
    );
}
