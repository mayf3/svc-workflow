//! V2 DOGFOOD: a real, low-risk, disposable Minimal workflow run with a
//! real Agent principal (hr-agent UUID) end to end.
//!
//! Definition (semantic_model_version = 2):
//!
//!   A (entry, Creator)
//!     └─ ADVANCE → B (FixedPrincipal = hr-agent)
//!                    ├─ approve → TERMINAL (done)
//!                    └─ reject  → RETURN → A
//!
//! Instance 1 (normal completion):  A → B → approve → TERMINAL
//! Instance 2 (return then finish): A → B → reject → A → B → approve → TERMINAL
//!
//! The definition graph is validated with validate_minimal_graph before
//! seeding (the Runtime itself also validates at create). Evidence is
//! printed per step so it can be cross-checked against the DB afterwards.

use svc_workflow::application::workflow_instance::create::create_workflow_instance;
use svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition;
use svc_workflow::domain::definition::graph::validate_minimal_graph;
use svc_workflow::domain::definition::model::{
    AssigneeRef, NodeDefinition, TransitionDefinition, WorkflowGraph,
};
use svc_workflow::domain::enums::{AssigneeRefType, NodeType, TransitionEffect};
use svc_workflow::domain::ids::{DefinitionVersionId, NodeId, TransitionId, WorkflowDefinitionId};
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
};

use super::*;
use super::minimal_runtime::{seed_v2_definition, AssigneeSpec, V2Def};

/// The real hr-agent machine principal.
const HR_AGENT_PRINCIPAL: &str = "bc970ced-710f-4479-9ff0-e295a1c59424";

/// Print a dogfood evidence line.
macro_rules! evidence {
    ($($arg:tt)*) => {
        println!("[v2-dogfood] {}", format!($($arg)*));
    };
}

async fn insert_real_principal(pool: &PgPool, principal_id: Uuid) {
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) \
         VALUES ($1, 'AGENT', 'hr-agent (dogfood)', 'hr-agent@dogfood.local', TRUE) \
         ON CONFLICT (principal_id) DO NOTHING",
    )
    .bind(principal_id)
    .execute(pool)
    .await
    .expect("insert hr-agent principal");
}

/// A disposable dogfood domain owned by the hr-agent, who is also a member.
async fn seed_dogfood_domain(pool: &PgPool, owner: Uuid) -> Uuid {
    let domain_id = Uuid::new_v4();
    let domain_key = format!("v2-dogfood-{}", &Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO domains (domain_id, domain_key, display_name, enabled) \
         VALUES ($1, $2, 'V2 Dogfood', TRUE)",
    )
    .bind(domain_id)
    .bind(&domain_key)
    .execute(pool)
    .await
    .expect("insert dogfood domain");

    let owner_binding = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) \
         VALUES ($1, $2, $3, 'DOMAIN_OWNER', TRUE)",
    )
    .bind(owner_binding)
    .bind(domain_id)
    .bind(owner)
    .execute(pool)
    .await
    .expect("insert dogfood owner");

    let member_binding = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) \
         VALUES ($1, $2, $3, 'MEMBER', TRUE)",
    )
    .bind(member_binding)
    .bind(domain_id)
    .bind(owner)
    .execute(pool)
    .await
    .expect("insert dogfood member");

    evidence!("domain {domain_key} ({domain_id}) created; hr-agent is owner + member");
    domain_id
}

/// The dogfood V2 definition: A(Creator) → B(FixedPrincipal=hr-agent),
/// B → approve → TERMINAL, B → reject → RETURN A.
async fn seed_dogfood_definition(pool: &PgPool, domain_id: Uuid, hr_agent: Uuid) -> V2Def {
    // Validate the exact graph shape with the frozen Minimal validator
    // before seeding (defensive; the Runtime also validates at create).
    let version_id = DefinitionVersionId::from_uuid(Uuid::new_v4());
    let graph = WorkflowGraph {
        nodes: vec![
            NodeDefinition {
                node_id: NodeId::from_uuid(Uuid::from_u128(8001)),
                definition_version_id: version_id,
                node_key: "a".to_string(),
                display_name: "A".to_string(),
                order_index: 0,
                node_type: NodeType::NORMAL,
                assignee_ref: Some(AssigneeRef {
                    ref_type: AssigneeRefType::WorkflowCreator,
                    fixed_principal_id: None,
                    assignee_input_key: None,
                }),
                instructions: None,
                primary_advance_transition_id: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
            NodeDefinition {
                node_id: NodeId::from_uuid(Uuid::from_u128(8002)),
                definition_version_id: version_id,
                node_key: "b".to_string(),
                display_name: "B".to_string(),
                order_index: 0,
                node_type: NodeType::NORMAL,
                assignee_ref: Some(AssigneeRef {
                    ref_type: AssigneeRefType::FixedPrincipal,
                    fixed_principal_id: Some(svc_workflow::domain::ids::PrincipalId::from_uuid(hr_agent)),
                    assignee_input_key: None,
                }),
                instructions: None,
                primary_advance_transition_id: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
            NodeDefinition {
                node_id: NodeId::from_uuid(Uuid::from_u128(8003)),
                definition_version_id: version_id,
                node_key: "done".to_string(),
                display_name: "done".to_string(),
                order_index: 0,
                node_type: NodeType::TERMINAL,
                assignee_ref: None,
                instructions: None,
                primary_advance_transition_id: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
        ],
        transitions: vec![
            TransitionDefinition {
                transition_id: TransitionId::from_uuid(Uuid::from_u128(9001)),
                definition_version_id: version_id,
                transition_key: "a_to_b".to_string(),
                display_name: "a_to_b".to_string(),
                source_node_id: NodeId::from_uuid(Uuid::from_u128(8001)),
                target_node_id: NodeId::from_uuid(Uuid::from_u128(8002)),
                transition_effect: TransitionEffect::Advance,
                submission_schema: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
            TransitionDefinition {
                transition_id: TransitionId::from_uuid(Uuid::from_u128(9002)),
                definition_version_id: version_id,
                transition_key: "approve".to_string(),
                display_name: "approve".to_string(),
                source_node_id: NodeId::from_uuid(Uuid::from_u128(8002)),
                target_node_id: NodeId::from_uuid(Uuid::from_u128(8003)),
                transition_effect: TransitionEffect::Advance,
                submission_schema: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
            TransitionDefinition {
                transition_id: TransitionId::from_uuid(Uuid::from_u128(9003)),
                definition_version_id: version_id,
                transition_key: "reject".to_string(),
                display_name: "reject".to_string(),
                source_node_id: NodeId::from_uuid(Uuid::from_u128(8002)),
                target_node_id: NodeId::from_uuid(Uuid::from_u128(8001)),
                transition_effect: TransitionEffect::Return,
                submission_schema: None,
                metadata: None,
                created_at: chrono::Utc::now(),
            },
        ],
        context_schema: None,
    };
    let validation = validate_minimal_graph(&graph);
    assert!(
        validation.valid,
        "dogfood V2 graph must pass validate_minimal_graph: {:?}",
        validation.errors
    );
    evidence!("validate_minimal_graph PASS on dogfood graph");

    seed_v2_definition(
        pool,
        domain_id,
        &[
            ("a", AssigneeSpec::Creator),
            ("b", AssigneeSpec::Fixed(hr_agent)),
        ],
        &["done"],
        &[
            ("a_to_b", "a", "b", "ADVANCE"),
            ("approve", "b", "done", "ADVANCE"),
            ("reject", "b", "a", "RETURN"),
        ],
        true,
    )
    .await
}

async fn create_instance(
    pool: &PgPool,
    creator: Uuid,
    domain_id: Uuid,
    ver_id: Uuid,
) -> Uuid {
    let result = create_workflow_instance(
        pool,
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(creator),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(ver_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({"dogfood": "v2"}),
            context_payload: serde_json::json!({}),
        },
    )
    .await
    .expect("create v2 dogfood instance");
    evidence!(
        "instance {} created; state_version={} current_visit={}",
        result.workflow_instance_id,
        result.workflow_state_version,
        result.current_node_visit_id
    );
    result.workflow_instance_id
}

async fn run_transition(
    pool: &PgPool,
    actor: Uuid,
    instance_id: Uuid,
    expected_version: i32,
    transition_id: Uuid,
    label: &str,
) {
    let result = execute_workflow_transition(
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
    .unwrap_or_else(|e| panic!("transition '{label}' failed: {e:?}"));
    evidence!(
        "transition '{label}' OK -> state_version={} current_visit={}",
        result.workflow_state_version,
        result.current_node_visit_id
    );
}

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

async fn visit_counts_by_node(pool: &PgPool, instance_id: Uuid) -> std::collections::HashMap<Uuid, i64> {
    let rows: Vec<(Uuid, i64)> = sqlx::query_as(
        "SELECT node_id, COUNT(*) FROM workflow_node_visits \
         WHERE workflow_instance_id = $1 GROUP BY node_id",
    )
    .bind(instance_id)
    .fetch_all(pool)
    .await
    .expect("visit counts");
    rows.into_iter().collect()
}

/// The graph looks up node ids by key for assertions.
fn node_id_of(def: &V2Def, key: &str) -> Uuid {
    def.nodes[key]
}

#[tokio::test]
async fn v2_dogfood_normal_completion() {
    let pool = create_pool().await;
    let hr_agent = Uuid::parse_str(HR_AGENT_PRINCIPAL).unwrap();
    insert_real_principal(&pool, hr_agent).await;
    let domain_id = seed_dogfood_domain(&pool, hr_agent).await;
    let def = seed_dogfood_definition(&pool, domain_id, hr_agent).await;

    // --- Instance 1: A -> B -> approve -> TERMINAL ---
    let instance = create_instance(&pool, hr_agent, domain_id, def.ver_id).await;

    let (node_id, assignee) = current_visit(&pool, instance).await;
    assert_eq!(node_id, node_id_of(&def, "a"), "entry TASK A");
    assert_eq!(assignee, Some(hr_agent), "entry assignee = Creator (hr-agent)");
    evidence!("entry: node=A assignee=hr-agent ✓");

    run_transition(&pool, hr_agent, instance, 1, def.transitions["a_to_b"], "a_to_b").await;
    let (node_id, assignee) = current_visit(&pool, instance).await;
    assert_eq!(node_id, node_id_of(&def, "b"));
    assert_eq!(assignee, Some(hr_agent), "B assignee = FixedPrincipal (hr-agent)");
    evidence!("after a_to_b: node=B assignee=hr-agent ✓");

    run_transition(&pool, hr_agent, instance, 2, def.transitions["approve"], "approve").await;
    let (node_id, assignee) = current_visit(&pool, instance).await;
    assert_eq!(node_id, node_id_of(&def, "done"), "TERMINAL reached");
    assert_eq!(assignee, None, "TERMINAL visit carries no assignee (assigned-to-me gone)");
    evidence!("after approve: node=TERMINAL(done) assignee=NULL ✓ (no longer in assigned-to-me)");

    // No further transition may execute on a completed instance.
    let extra = execute_workflow_transition(
        &pool,
        ExecuteWorkflowTransitionCommand {
            principal_id: PrincipalId::from_uuid(hr_agent),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance),
            expected_workflow_state_version: 3,
            transition_definition_id: TransitionId::from_uuid(def.transitions["a_to_b"]),
            submission_payload: None,
        },
    )
    .await;
    assert!(extra.is_err(), "completed instance must reject further transitions");
    evidence!("further transition rejected ✓");

    let counts = visit_counts_by_node(&pool, instance).await;
    assert_eq!(counts[&node_id_of(&def, "a")], 1);
    assert_eq!(counts[&node_id_of(&def, "b")], 1);
    assert_eq!(counts[&node_id_of(&def, "done")], 1);
    evidence!("visit history: A=1 B=1 TERMINAL=1 ✓");
}

#[tokio::test]
async fn v2_dogfood_return_then_complete() {
    let pool = create_pool().await;
    let hr_agent = Uuid::parse_str(HR_AGENT_PRINCIPAL).unwrap();
    insert_real_principal(&pool, hr_agent).await;
    let domain_id = seed_dogfood_domain(&pool, hr_agent).await;
    let def = seed_dogfood_definition(&pool, domain_id, hr_agent).await;

    // --- Instance 2: A -> B -> reject (RETURN A) -> A -> B -> approve ---
    let instance = create_instance(&pool, hr_agent, domain_id, def.ver_id).await;
    run_transition(&pool, hr_agent, instance, 1, def.transitions["a_to_b"], "a_to_b").await;

    // Reject: RETURN to ancestor A creates a NEW A visit.
    run_transition(&pool, hr_agent, instance, 2, def.transitions["reject"], "reject").await;
    let (node_id, assignee) = current_visit(&pool, instance).await;
    assert_eq!(node_id, node_id_of(&def, "a"), "RETURN lands back on A");
    assert_eq!(assignee, Some(hr_agent), "A assignee re-resolved to Creator (hr-agent)");
    evidence!("after reject: node=A (new visit) assignee=hr-agent re-resolved ✓");

    let counts = visit_counts_by_node(&pool, instance).await;
    assert_eq!(counts[&node_id_of(&def, "a")], 2, "A visited twice (history preserved)");
    assert_eq!(counts[&node_id_of(&def, "b")], 1);
    evidence!("visit history after RETURN: A=2 B=1 (historical visits preserved) ✓");

    // Re-run the path to completion.
    run_transition(&pool, hr_agent, instance, 3, def.transitions["a_to_b"], "a_to_b (again)").await;
    run_transition(&pool, hr_agent, instance, 4, def.transitions["approve"], "approve").await;
    let (node_id, assignee) = current_visit(&pool, instance).await;
    assert_eq!(node_id, node_id_of(&def, "done"), "completed after return");
    assert_eq!(assignee, None);
    evidence!("final: TERMINAL reached after reject -> rework ✓");

    let counts = visit_counts_by_node(&pool, instance).await;
    assert_eq!(counts[&node_id_of(&def, "a")], 2);
    assert_eq!(counts[&node_id_of(&def, "b")], 2);
    assert_eq!(counts[&node_id_of(&def, "done")], 1);
    evidence!("final visit history: A=2 B=2 TERMINAL=1 ✓");
}
