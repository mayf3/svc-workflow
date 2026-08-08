//! EXPLICIT V2 CREATION: the production creation path accepts an explicit
//! semantic_model_version=2 choice, and a V2 version created through that
//! path really runs the Minimal Runtime end to end.
//!
//! Full production-path chain:
//!   governance_create_definition
//!   -> governance_create_draft_version(semantic_model_version=2)
//!   -> governance_replace_draft_graph (A -> B, approve / reject RETURN)
//!   -> governance_publish_version
//!   -> create instance (Minimal Runtime dispatch)
//!   -> ADVANCE -> RETURN -> ADVANCE -> TERMINAL

use svc_workflow::application::definition_governance::{
    governance_create_draft_version, governance_create_definition,
    governance_publish_version, governance_replace_draft_graph,
};
use svc_workflow::application::workflow_instance::create::create_workflow_instance;
use svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition;
use svc_workflow::domain::definition::model::SemanticModelVersion;
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
};

use super::*;

#[tokio::test]
async fn explicit_v2_creation_runs_minimal_runtime() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let fixed = seed_second_principal(&pool).await;

    // 1. Create definition through the production path.
    let def = governance_create_definition(
        &pool,
        owner,
        &Uuid::new_v4().to_string(),
        "req",
        domain_id,
        &format!("v2-prod-{}", &Uuid::new_v4().to_string()[..8]),
        "Explicit V2",
        None,
        None,
    )
    .await
    .expect("create definition");
    let def_id = def.id.into_uuid();

    // 2. Create a draft version with explicit semantic_model_version = 2.
    let version = governance_create_draft_version(
        &pool,
        owner,
        &Uuid::new_v4().to_string(),
        "req",
        def_id,
        Some(serde_json::json!({"type": "object"})),
        None,
        None,
        Some(serde_json::json!({"explicit": "v2"})),
        2,
    )
    .await
    .expect("create draft version v2");
    assert_eq!(
        version.semantic_model_version,
        SemanticModelVersion::Minimal,
        "production path must persist semantic_model_version=2"
    );
    let ver_id = version.id.into_uuid();

    // 3. Replace the graph through the production path (V2 graph: no DRAFT,
    //    no primary advance, no TERMINATE).
    let nodes = vec![
        svc_workflow::application::definition::commands::RawNodeDefinition {
            node_key: "a".into(),
            display_name: "A".into(),
            order_index: 0,
            node_type: "NORMAL".into(),
            assignee_ref_type: Some("WORKFLOW_CREATOR".into()),
            fixed_principal_id: None,
            assignee_input_key: None,
            instructions: None,
            primary_advance_transition_key: None,
            metadata: None,
        },
        svc_workflow::application::definition::commands::RawNodeDefinition {
            node_key: "b".into(),
            display_name: "B".into(),
            order_index: 1,
            node_type: "NORMAL".into(),
            assignee_ref_type: Some("FIXED_PRINCIPAL".into()),
            fixed_principal_id: Some(fixed),
            assignee_input_key: None,
            instructions: None,
            primary_advance_transition_key: None,
            metadata: None,
        },
        svc_workflow::application::definition::commands::RawNodeDefinition {
            node_key: "done".into(),
            display_name: "done".into(),
            order_index: 2,
            node_type: "TERMINAL".into(),
            assignee_ref_type: None,
            fixed_principal_id: None,
            assignee_input_key: None,
            instructions: None,
            primary_advance_transition_key: None,
            metadata: None,
        },
    ];
    let transitions = vec![
        svc_workflow::application::definition::commands::RawTransitionDefinition {
            transition_key: "a_to_b".into(),
            display_name: "a_to_b".into(),
            source_node_key: "a".into(),
            target_node_key: "b".into(),
            transition_effect: "ADVANCE".into(),
            submission_schema: None,
            metadata: None,
        },
        svc_workflow::application::definition::commands::RawTransitionDefinition {
            transition_key: "approve".into(),
            display_name: "approve".into(),
            source_node_key: "b".into(),
            target_node_key: "done".into(),
            transition_effect: "ADVANCE".into(),
            submission_schema: None,
            metadata: None,
        },
        svc_workflow::application::definition::commands::RawTransitionDefinition {
            transition_key: "reject".into(),
            display_name: "reject".into(),
            source_node_key: "b".into(),
            target_node_key: "a".into(),
            transition_effect: "RETURN".into(),
            submission_schema: None,
            metadata: None,
        },
    ];
    governance_replace_draft_graph(
        &pool,
        owner,
        &Uuid::new_v4().to_string(),
        "req",
        ver_id,
        Some(serde_json::json!({"type": "object"})),
        nodes,
        transitions,
    )
    .await
    .expect("replace graph");

    // 4. Publish through the production path (Minimal validator dispatch).
    governance_publish_version(
        &pool,
        owner,
        &Uuid::new_v4().to_string(),
        "req",
        ver_id,
        None,
    )
    .await
    .expect("publish v2 version");

    // 5. Create an instance: Minimal Runtime must dispatch and pick entry A.
    let instance = create_workflow_instance(
        &pool,
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            domain_id: DomainId::from_uuid(domain_id),
            definition_version_id: DefinitionVersionId::from_uuid(ver_id),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({"v2": "production-path"}),
            context_payload: serde_json::json!({}),
        },
    )
    .await
    .expect("create v2 instance through production-path definition")
    .workflow_instance_id;

    // Entry A = the ADVANCE-graph root (Creator).
    let (node_id, assignee): (Uuid, Option<Uuid>) = sqlx::query_as(
        "SELECT v.node_id, v.assignee_principal_id FROM workflow_node_visits v \
         JOIN workflow_instances i ON i.current_node_visit_id = v.node_visit_id \
         WHERE i.workflow_instance_id = $1",
    )
    .bind(instance)
    .fetch_one(&pool)
    .await
    .expect("current visit");
    let node_key: String =
        sqlx::query_scalar("SELECT node_key FROM workflow_node_definitions WHERE node_id = $1")
            .bind(node_id)
            .fetch_one(&pool)
            .await
            .expect("node key");
    assert_eq!(node_key, "a", "Minimal entry from ADVANCE graph");
    assert_eq!(assignee, Some(owner), "Creator assignee");

    // 6. Execute: A -> B (ADVANCE), B -> A (RETURN), A -> B, B -> TERMINAL.
    let trans_ids: std::collections::HashMap<String, Uuid> = sqlx::query_as(
        "SELECT transition_key, transition_id FROM workflow_transition_definitions \
         WHERE definition_version_id = $1",
    )
    .bind(ver_id)
    .fetch_all(&pool)
    .await
    .expect("transition ids")
    .into_iter()
    .collect();

    let mut step = |actor: Uuid, expected: i32, key: &str| {
        let t = trans_ids[key];
        execute_workflow_transition(
            &pool,
            ExecuteWorkflowTransitionCommand {
                principal_id: PrincipalId::from_uuid(actor),
                idempotency_key: Uuid::new_v4().to_string(),
                command_schema_version: "v1".to_string(),
                workflow_instance_id: WorkflowInstanceId::from_uuid(instance),
                expected_workflow_state_version: expected,
                transition_definition_id: TransitionId::from_uuid(t),
                submission_payload: None,
            },
        )
    };

    // A is assigned to the Creator (owner); B is assigned to the fixed principal.
    step(owner, 1, "a_to_b").await.expect("A -> B");
    step(fixed, 2, "reject").await.expect("B RETURN A");
    step(owner, 3, "a_to_b").await.expect("A -> B (again)");
    step(fixed, 4, "approve").await.expect("B -> TERMINAL");

    let (node_id, assignee): (Uuid, Option<Uuid>) = sqlx::query_as(
        "SELECT v.node_id, v.assignee_principal_id FROM workflow_node_visits v \
         JOIN workflow_instances i ON i.current_node_visit_id = v.node_visit_id \
         WHERE i.workflow_instance_id = $1",
    )
    .bind(instance)
    .fetch_one(&pool)
    .await
    .expect("final visit");
    let node_key: String =
        sqlx::query_scalar("SELECT node_key FROM workflow_node_definitions WHERE node_id = $1")
            .bind(node_id)
            .fetch_one(&pool)
            .await
            .expect("final node key");
    assert_eq!(node_key, "done", "TERMINAL reached via production-path V2");
    assert_eq!(assignee, None);

    let visits: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM workflow_node_visits WHERE workflow_instance_id = $1")
            .bind(instance)
            .fetch_one(&pool)
            .await
            .expect("visit count");
    assert_eq!(visits, 5, "A#1 B#1 A#2 B#2 done");
}

#[tokio::test]
async fn default_and_explicit_v1_remain_legacy() {
    let pool = create_pool().await;
    let (owner, domain_id) = seed_principal_domain_with_owner(&pool).await;

    let def = governance_create_definition(
        &pool,
        owner,
        &Uuid::new_v4().to_string(),
        "req",
        domain_id,
        &format!("v1-default-{}", &Uuid::new_v4().to_string()[..8]),
        "V1 default",
        None,
        None,
    )
    .await
    .expect("create definition");
    let def_id = def.id.into_uuid();

    // No field provided (caller does not opt in) -> Legacy(1).
    let version = governance_create_draft_version(
        &pool,
        owner,
        &Uuid::new_v4().to_string(),
        "req",
        def_id,
        Some(serde_json::json!({"type": "object"})),
        None,
        None,
        None,
        1,
    )
    .await
    .expect("create draft version default");
    assert_eq!(
        version.semantic_model_version,
        SemanticModelVersion::Legacy,
        "omitted semanticModelVersion must stay Legacy(1)"
    );

    // Explicit 1 -> Legacy.
    let version2 = governance_create_draft_version(
        &pool,
        owner,
        &Uuid::new_v4().to_string(),
        "req",
        def_id,
        Some(serde_json::json!({"type": "object"})),
        None,
        None,
        None,
        1,
    )
    .await
    .expect("create draft version explicit v1");
    assert_eq!(version2.semantic_model_version, SemanticModelVersion::Legacy);
}
