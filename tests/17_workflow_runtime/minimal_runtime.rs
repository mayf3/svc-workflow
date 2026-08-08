//! V2 Minimal semantic model runtime tests.
//!
//! These tests construct `semantic_model_version = 2` definition versions
//! directly (test fixtures only — production APIs cannot create V2) and
//! exercise the Minimal runtime: entry selection from the ADVANCE graph,
//! the three V2 assignee selectors, multi-outgoing ADVANCE branches,
//! ADVANCE to TASK / TERMINAL, RETURN to ancestor, orderIndex irrelevance,
//! and fail-closed behavior on Legacy garbage.

use std::collections::HashMap;

use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
};
use svc_workflow::domain::workflow_instance::errors::ExecuteWorkflowTransitionError;
use svc_workflow::application::workflow_instance::create::create_workflow_instance;
use svc_workflow::application::workflow_instance::execute_transition::{
    execute_workflow_transition, ExecuteWorkflowTransitionResult,
};

use super::*;

/// Assignee selector spec for a V2 node.
pub(crate) enum AssigneeSpec {
    Creator,
    Fixed(Uuid),
    Context(&'static str),
}

/// A seeded V2 definition graph.
pub(crate) struct V2Def {
    pub(crate) ver_id: Uuid,
    pub(crate) nodes: HashMap<String, Uuid>,
    pub(crate) transitions: HashMap<String, Uuid>,
}

/// Seed a V2 (semantic_model_version = 2) PUBLISHED definition.
///
/// `nodes`: (key, NORMAL+assignee spec). `terminals`: TERMINAL keys.
/// `edges`: (edge key, source key, target key, effect).
pub(crate) async fn seed_v2_definition(
    pool: &PgPool,
    domain_id: Uuid,
    nodes: &[(&str, AssigneeSpec)],
    terminals: &[&str],
    edges: &[(&str, &str, &str, &str)],
    publish: bool,
) -> V2Def {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    let def_key = format!("v2-{}", &Uuid::new_v4().to_string()[..8]);

    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) \
         VALUES ($1, $2, $3, 'V2 Minimal')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(pool)
    .await
    .expect("insert v2 def");

    // DRAFT first (graph immutability trigger forbids editing PUBLISHED
    // graphs); semantic_model_version = 2 set from the start; published at
    // the end. Production APIs cannot create V2 — this is a test fixture.
    sqlx::query(
        "INSERT INTO workflow_definition_versions \
           (definition_version_id, workflow_definition_id, version_number, version_status, \
            semantic_model_version, context_schema) \
         VALUES ($1, $2, 1, 'DRAFT', 2, '{\"type\":\"object\"}'::jsonb)",
    )
    .bind(ver_id)
    .bind(def_id)
    .execute(pool)
    .await
    .expect("insert v2 version");

    let mut node_ids = HashMap::new();
    for (idx, (key, spec)) in nodes.iter().enumerate() {
        let node_id = Uuid::new_v4();
        let (assignee_type, fixed_id, input_key): (&str, Option<Uuid>, Option<&str>) = match spec {
            AssigneeSpec::Creator => ("WORKFLOW_CREATOR", None, None),
            AssigneeSpec::Fixed(pid) => ("FIXED_PRINCIPAL", Some(*pid), None),
            AssigneeSpec::Context(key) => ("INSTANCE_INPUT_PRINCIPAL", None, Some(key)),
        };
        sqlx::query(
            "INSERT INTO workflow_node_definitions \
               (node_id, definition_version_id, node_key, display_name, order_index, node_type, \
                assignee_ref_type, fixed_principal_id, assignee_input_key) \
             VALUES ($1, $2, $3, $3, $4, 'NORMAL', $5::assignee_ref_type, $6, $7)",
        )
        .bind(node_id)
        .bind(ver_id)
        .bind(key)
        .bind(idx as i32)
        .bind(assignee_type)
        .bind(fixed_id)
        .bind(input_key)
        .execute(pool)
        .await
        .expect("insert v2 node");
        node_ids.insert(key.to_string(), node_id);
    }

    for (tidx, key) in terminals.iter().enumerate() {
        let node_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_node_definitions \
               (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) \
             VALUES ($1, $2, $3, $3, $4, 'TERMINAL', NULL)",
        )
        .bind(node_id)
        .bind(ver_id)
        .bind(key)
        .bind(1000 + tidx as i32)
        .execute(pool)
        .await
        .expect("insert v2 terminal");
        node_ids.insert(key.to_string(), node_id);
    }

    let mut transition_ids = HashMap::new();
    for (idx, (key, source, target, effect)) in edges.iter().enumerate() {
        let trans_id = Uuid::new_v4();
        sqlx::query(
            "INSERT INTO workflow_transition_definitions \
               (transition_id, definition_version_id, transition_key, display_name, source_node_id, \
                target_node_id, transition_effect) \
             VALUES ($1, $2, $3, $3, $4, $5, $6::transition_effect)",
        )
        .bind(trans_id)
        .bind(ver_id)
        .bind(key)
        .bind(node_ids[*source])
        .bind(node_ids[*target])
        .bind(effect)
        .execute(pool)
        .await
        .expect("insert v2 transition");
        transition_ids.insert(key.to_string(), trans_id);
    }

    // Publish now that the graph is fully written (unless the caller needs
    // pre-publish mutations, e.g. to simulate validator bypass).
    if publish {
        sqlx::query(
            "UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1",
        )
        .bind(ver_id)
        .execute(pool)
        .await
        .expect("publish v2 version");
    }

    V2Def {
        ver_id,
        nodes: node_ids,
        transitions: transition_ids,
    }
}

async fn exec_transition(
    pool: &PgPool,
    actor: Uuid,
    instance_id: Uuid,
    expected_version: i32,
    transition_id: Uuid,
) -> Result<ExecuteWorkflowTransitionResult, ExecuteWorkflowTransitionError> {
    execute_workflow_transition(
        pool,
        ExecuteWorkflowTransitionCommand {
            principal_id: PrincipalId::from_uuid(actor),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance_id),
            expected_workflow_state_version: expected_version,
            transition_definition_id: TransitionId::from_uuid(transition_id),
            submission_payload: None,
        },
    )
    .await
}

/// Current visit (node_id, assignee) for an instance.
async fn current_visit(pool: &PgPool, instance_id: Uuid) -> (Uuid, Option<Uuid>) {
    let (node_id, assignee): (Uuid, Option<Uuid>) = sqlx::query_as(
        "SELECT v.node_id, v.assignee_principal_id \
         FROM workflow_node_visits v JOIN workflow_instances i ON i.current_node_visit_id = v.node_visit_id \
         WHERE i.workflow_instance_id = $1",
    )
    .bind(instance_id)
    .fetch_one(pool)
    .await
    .expect("current visit");
    (node_id, assignee)
}

async fn visit_count(pool: &PgPool, instance_id: Uuid) -> i64 {
    sqlx::query_scalar("SELECT COUNT(*) FROM workflow_node_visits WHERE workflow_instance_id = $1")
        .bind(instance_id)
        .fetch_one(pool)
        .await
        .expect("visit count")
}

async fn create_v2_instance(
    pool: &PgPool,
    creator: Uuid,
    domain_id: Uuid,
    ver_id: Uuid,
    context_payload: serde_json::Value,
) -> Uuid {
    let cmd = CreateWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(creator),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        domain_id: DomainId::from_uuid(domain_id),
        definition_version_id: DefinitionVersionId::from_uuid(ver_id),
        external_reference: None,
        external_url: None,
        metadata: serde_json::json!({"source": "v2-test"}),
        context_payload,
    };
    create_workflow_instance(pool, cmd)
        .await
        .expect("create v2 instance")
        .workflow_instance_id
}

// ---------------------------------------------------------------------------
// V2 instance creation: entry from ADVANCE graph
// ---------------------------------------------------------------------------

#[tokio::test]
async fn v2_create_entry_from_advance_graph() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let fixed = seed_second_principal(&pool).await;

    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("entry", AssigneeSpec::Fixed(fixed)), ("work", AssigneeSpec::Creator)],
        &["done"],
        &[
            ("entry_to_work", "entry", "work", "ADVANCE"),
            ("work_to_done", "work", "done", "ADVANCE"),
        ],
        true,
    )
    .await;

    let instance = create_v2_instance(&pool, creator, domain_id, def.ver_id, serde_json::json!({}))
        .await;

    let (node_id, assignee) = current_visit(&pool, instance).await;
    assert_eq!(node_id, def.nodes["entry"], "entry TASK must become the current visit");
    assert_eq!(assignee, Some(fixed), "entry assignee from FixedPrincipal selector");
}

#[tokio::test]
async fn v2_create_entry_assignee_creator() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;

    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("entry", AssigneeSpec::Creator)],
        &["done"],
        &[("entry_to_done", "entry", "done", "ADVANCE")],
        true,
    )
    .await;

    let instance = create_v2_instance(&pool, creator, domain_id, def.ver_id, serde_json::json!({}))
        .await;
    let (node_id, assignee) = current_visit(&pool, instance).await;
    assert_eq!(node_id, def.nodes["entry"]);
    assert_eq!(assignee, Some(creator), "Creator selector resolves to instance creator");
}

#[tokio::test]
async fn v2_create_entry_assignee_context_principal() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let reviewer = seed_second_principal(&pool).await;

    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("review", AssigneeSpec::Context("reviewer"))],
        &["done"],
        &[("review_to_done", "review", "done", "ADVANCE")],
        true,
    )
    .await;

    let context = serde_json::json!({"reviewer": reviewer.to_string()});
    let instance = create_v2_instance(&pool, creator, domain_id, def.ver_id, context).await;

    let (_, assignee) = current_visit(&pool, instance).await;
    assert_eq!(assignee, Some(reviewer), "ContextPrincipal resolves from context payload");
}

#[tokio::test]
async fn v2_context_key_missing_create_fails() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;

    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("review", AssigneeSpec::Context("reviewer"))],
        &["done"],
        &[("review_to_done", "review", "done", "ADVANCE")],
        true,
    )
    .await;

    let result = create_workflow_instance(
        &pool,
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(creator),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(def.ver_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({}),
            context_payload: serde_json::json!({}),
        },
    )
    .await;
    assert!(
        result.is_err(),
        "missing context key must fail closed at create time"
    );
}

#[tokio::test]
async fn v2_context_invalid_value_create_fails() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;

    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("review", AssigneeSpec::Context("reviewer"))],
        &["done"],
        &[("review_to_done", "review", "done", "ADVANCE")],
        true,
    )
    .await;

    let result = create_workflow_instance(
        &pool,
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(creator),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(def.ver_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({}),
            context_payload: serde_json::json!({"reviewer": "not-a-uuid"}),
        },
    )
    .await;
    assert!(
        result.is_err(),
        "invalid context value must fail closed at create time"
    );
}

// ---------------------------------------------------------------------------
// V2 ADVANCE
// ---------------------------------------------------------------------------

#[tokio::test]
async fn v2_advance_to_task_creates_new_visit() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;

    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("entry", AssigneeSpec::Creator), ("work", AssigneeSpec::Creator)],
        &["done"],
        &[
            ("entry_to_work", "entry", "work", "ADVANCE"),
            ("work_to_done", "work", "done", "ADVANCE"),
        ],
        true,
    )
    .await;

    let instance = create_v2_instance(&pool, creator, domain_id, def.ver_id, serde_json::json!({}))
        .await;

    exec_transition(&pool, creator, instance, 1, def.transitions["entry_to_work"])
        .await
        .expect("ADVANCE entry->work");

    let (node_id, assignee) = current_visit(&pool, instance).await;
    assert_eq!(node_id, def.nodes["work"], "ADVANCE to TASK moves current visit");
    assert_eq!(assignee, Some(creator));
    assert_eq!(visit_count(&pool, instance).await, 2, "new visit created");
}

#[tokio::test]
async fn v2_advance_to_terminal_completes() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;

    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("entry", AssigneeSpec::Creator)],
        &["done"],
        &[("entry_to_done", "entry", "done", "ADVANCE")],
        true,
    )
    .await;

    let instance = create_v2_instance(&pool, creator, domain_id, def.ver_id, serde_json::json!({}))
        .await;

    exec_transition(&pool, creator, instance, 1, def.transitions["entry_to_done"])
        .await
        .expect("ADVANCE entry->TERMINAL");

    let (node_id, assignee) = current_visit(&pool, instance).await;
    assert_eq!(node_id, def.nodes["done"], "ADVANCE to TERMINAL completes the workflow");
    assert_eq!(assignee, None, "TERMINAL visit carries no assignee");
    assert_eq!(visit_count(&pool, instance).await, 2);
}

#[tokio::test]
async fn v2_branch_both_outgoing_executable() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;

    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("review", AssigneeSpec::Creator)],
        &["published", "archived"],
        &[
            ("approve", "review", "published", "ADVANCE"),
            ("reject", "review", "archived", "ADVANCE"),
        ],
        true,
    )
    .await;

    let instance = create_v2_instance(&pool, creator, domain_id, def.ver_id, serde_json::json!({}))
        .await;

    // Branch 1: approve -> published (TERMINAL)
    exec_transition(&pool, creator, instance, 1, def.transitions["approve"])
        .await
        .expect("approve branch");
    let (node_id, _) = current_visit(&pool, instance).await;
    assert_eq!(node_id, def.nodes["published"]);

    // Branch 2: reject -> archived (TERMINAL) — fresh instance, same def
    let instance2 =
        create_v2_instance(&pool, creator, domain_id, def.ver_id, serde_json::json!({})).await;
    exec_transition(&pool, creator, instance2, 1, def.transitions["reject"])
        .await
        .expect("reject branch");
    let (node_id2, _) = current_visit(&pool, instance2).await;
    assert_eq!(node_id2, def.nodes["archived"]);
}

// ---------------------------------------------------------------------------
// V2 RETURN
// ---------------------------------------------------------------------------

#[tokio::test]
async fn v2_return_to_ancestor_revisits() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;

    // a -> b -> c -> done; c RETURN a
    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[
            ("a", AssigneeSpec::Creator),
            ("b", AssigneeSpec::Creator),
            ("c", AssigneeSpec::Creator),
        ],
        &["done"],
        &[
            ("a_to_b", "a", "b", "ADVANCE"),
            ("b_to_c", "b", "c", "ADVANCE"),
            ("c_to_done", "c", "done", "ADVANCE"),
            ("c_return_a", "c", "a", "RETURN"),
        ],
        true,
    )
    .await;

    let instance = create_v2_instance(&pool, creator, domain_id, def.ver_id, serde_json::json!({}))
        .await;
    exec_transition(&pool, creator, instance, 1, def.transitions["a_to_b"]).await.unwrap();
    exec_transition(&pool, creator, instance, 2, def.transitions["b_to_c"]).await.unwrap();

    exec_transition(&pool, creator, instance, 3, def.transitions["c_return_a"])
        .await
        .expect("C RETURN A");

    let (node_id, assignee) = current_visit(&pool, instance).await;
    assert_eq!(node_id, def.nodes["a"], "RETURN creates a new visit of the ancestor TASK");
    assert_eq!(assignee, Some(creator), "ancestor assignee re-resolved");
    assert_eq!(
        visit_count(&pool, instance).await,
        4,
        "historical A/B/C visits are preserved alongside the new A visit"
    );
}

#[tokio::test]
async fn v2_order_index_irrelevant() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;

    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("entry", AssigneeSpec::Creator), ("work", AssigneeSpec::Creator)],
        &["done"],
        &[
            ("entry_to_work", "entry", "work", "ADVANCE"),
            ("work_to_done", "work", "done", "ADVANCE"),
            ("work_return_entry", "work", "entry", "RETURN"),
        ],
        false,
    )
    .await;

    // Shuffle order_index so any Legacy orderIndex semantics would diverge
    // (before publishing — graphs are immutable once PUBLISHED).
    sqlx::query(
        "UPDATE workflow_node_definitions SET order_index = CASE node_key \
           WHEN 'entry' THEN 99 WHEN 'work' THEN 50 ELSE order_index END \
         WHERE definition_version_id = $1",
    )
    .bind(def.ver_id)
    .execute(&pool)
    .await
    .expect("shuffle order_index");
    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(def.ver_id)
        .execute(&pool)
        .await
        .expect("publish after shuffle");

    let instance = create_v2_instance(&pool, creator, domain_id, def.ver_id, serde_json::json!({}))
        .await;
    exec_transition(&pool, creator, instance, 1, def.transitions["entry_to_work"])
        .await
        .expect("entry->work with shuffled order_index");
    exec_transition(&pool, creator, instance, 2, def.transitions["work_return_entry"])
        .await
        .expect("RETURN with shuffled order_index (no orderIndex rule in V2)");

    let (node_id, _) = current_visit(&pool, instance).await;
    assert_eq!(node_id, def.nodes["entry"]);
}

// ---------------------------------------------------------------------------
// V2 fail closed on Legacy garbage (validator bypass)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn v2_domain_owner_target_fail_closed() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;

    // Definition bypasses the V2 validator: 'work' uses DOMAIN_OWNER.
    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("entry", AssigneeSpec::Creator), ("work", AssigneeSpec::Creator)],
        &["done"],
        &[
            ("entry_to_work", "entry", "work", "ADVANCE"),
            ("work_to_done", "work", "done", "ADVANCE"),
        ],
        false,
    )
    .await;

    // Rewire 'work' to DOMAIN_OWNER behind the validator's back (before
    // publishing — graphs are immutable once PUBLISHED).
    sqlx::query(
        "UPDATE workflow_node_definitions SET assignee_ref_type = 'DOMAIN_OWNER', \
           fixed_principal_id = NULL, assignee_input_key = NULL \
         WHERE node_id = $1",
    )
    .bind(def.nodes["work"])
    .execute(&pool)
    .await
    .expect("rewire to DOMAIN_OWNER");
    sqlx::query("UPDATE workflow_definition_versions SET version_status = 'PUBLISHED' WHERE definition_version_id = $1")
        .bind(def.ver_id)
        .execute(&pool)
        .await
        .expect("publish after rewire");

    // Graph legality is the caller's responsibility; the Runtime keeps the
    // cheap execution fail-closed: resolving a DOMAIN_OWNER target in V2
    // must fail when the transition executes.
    let instance = create_v2_instance(&pool, creator, domain_id, def.ver_id, serde_json::json!({}))
        .await;
    let result = exec_transition(&pool, creator, instance, 1, def.transitions["entry_to_work"])
        .await;
    assert!(
        matches!(
            result,
            Err(ExecuteWorkflowTransitionError::AssigneeResolutionFailed(_))
        ),
        "V2 runtime must fail closed on DOMAIN_OWNER assignee, got: {result:?}"
    );
}

#[tokio::test]
async fn v2_terminate_fail_closed() {
    let pool = create_pool().await;
    let (creator, domain_id) = seed_principal_domain_with_owner(&pool).await;

    // Definition bypasses the V2 validator with a TERMINATE transition.
    let def = seed_v2_definition(
        &pool,
        domain_id,
        &[("entry", AssigneeSpec::Creator)],
        &["done"],
        &[
            ("entry_to_done", "entry", "done", "ADVANCE"),
            ("terminate", "entry", "done", "TERMINATE"),
        ],
        true,
    )
    .await;

    // Create succeeds (graph legality is the caller's responsibility); the
    // Runtime rejects the TERMINATE effect at execution time.
    let instance = create_v2_instance(&pool, creator, domain_id, def.ver_id, serde_json::json!({}))
        .await;
    let result = exec_transition(&pool, creator, instance, 1, def.transitions["terminate"]).await;
    assert!(
        matches!(
            result,
            Err(ExecuteWorkflowTransitionError::TransitionNotApplicable(_))
        ),
        "V2 runtime must fail closed on TERMINATE, got: {result:?}"
    );
}


