//! H-2: Assignee rules integration tests.

use super::*;
use svc_workflow::application::definition::commands::{
    CreateDraftVersion, PublishVersion, ReplaceDraftGraph,
};
use svc_workflow::domain::definition::error::DefinitionError;

#[tokio::test]
async fn test_terminal_node_with_fixed_principal_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    let result = sqlx::query(
        "UPDATE workflow_node_definitions SET assignee_ref_type = 'FIXED_PRINCIPAL', fixed_principal_id = $1 WHERE definition_version_id = $2 AND node_type = 'TERMINAL'",
    )
    .bind(assignee)
    .bind(version_id)
    .execute(&pool)
    .await
    ;
    assert!(
        result.is_err(),
        "database must reject a new Terminal assignee"
    );
}

#[tokio::test]
async fn test_non_terminal_without_assignee_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let version_id = {
        let def_id = uuid::Uuid::new_v4();
        let def_key = format!("test-def-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        sqlx::query(
            "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Test Definition')",
        )
        .bind(def_id)
        .bind(domain_id)
        .bind(&def_key)
        .execute(&pool)
        .await
        .expect("failed to insert definition");

        let create_cmd = CreateDraftVersion {
            actor_principal_id: owner,
            workflow_definition_id: def_id,
            context_schema: Some(serde_json::json!({"type": "object"})),
            json_schema_dialect: Some("https://json-schema.org/draft/2020-12/schema".to_string()),
            validator_version: Some("v1".to_string()),
            metadata: None,
        };
        let version = service
            .create_draft_version(create_cmd)
            .await
            .expect("create draft version");
        version.id.into_uuid()
    };

    let (mut raw_nodes, raw_transitions) = valid_raw_graph();
    raw_nodes[2].assignee_ref_type = Some("FIXED_PRINCIPAL".to_string());
    raw_nodes[2].fixed_principal_id = Some(assignee);
    raw_nodes[2].primary_advance_transition_key = None;

    let replace_cmd = ReplaceDraftGraph {
        actor_principal_id: owner,
        definition_version_id: version_id,
        context_schema: Some(serde_json::json!({"type": "object"})),
        nodes: raw_nodes,
        transitions: raw_transitions,
    };
    let result = service.replace_draft_graph(replace_cmd).await;
    assert!(
        result.is_err(),
        "terminal with FIXED_PRINCIPAL should be rejected"
    );
    match result.unwrap_err() {
        DefinitionError::FixedPrincipalInvalid(_) => {}
        DefinitionError::GraphValidationFailed(errors) => {
            assert!(
                errors.iter().any(|e| e.code == "TERMINAL_HAS_ASSIGNEE"),
                "expected TERMINAL_HAS_ASSIGNEE, got: {:?}",
                errors
            );
        }
        other => panic!(
            "expected FixedPrincipalInvalid or GraphValidationFailed, got: {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn test_workflow_creator_with_fixed_id_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) = {
        let def_id = uuid::Uuid::new_v4();
        let def_key = format!("test-def-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        sqlx::query(
            "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Test Definition')",
        )
        .bind(def_id)
        .bind(domain_id)
        .bind(&def_key)
        .execute(&pool)
        .await
        .expect("failed to insert definition");

        let create_cmd = CreateDraftVersion {
            actor_principal_id: owner,
            workflow_definition_id: def_id,
            context_schema: Some(serde_json::json!({"type": "object"})),
            json_schema_dialect: Some("https://json-schema.org/draft/2020-12/schema".to_string()),
            validator_version: Some("v1".to_string()),
            metadata: None,
        };
        let version = service
            .create_draft_version(create_cmd)
            .await
            .expect("create draft version");
        (def_id, version.id.into_uuid())
    };

    let (mut raw_nodes, raw_transitions) = valid_raw_graph();
    raw_nodes[0].fixed_principal_id = Some(assignee);

    let replace_cmd = ReplaceDraftGraph {
        actor_principal_id: owner,
        definition_version_id: version_id,
        context_schema: Some(serde_json::json!({"type": "object"})),
        nodes: raw_nodes,
        transitions: raw_transitions,
    };
    let result = service.replace_draft_graph(replace_cmd).await;
    assert!(
        result.is_err(),
        "WORKFLOW_CREATOR with fixed ID should be rejected"
    );
    match result.unwrap_err() {
        DefinitionError::FixedPrincipalInvalid(_) => {}
        DefinitionError::GraphValidationFailed(errors) => {
            assert!(
                errors
                    .iter()
                    .any(|e| e.code == "UNEXPECTED_FIXED_PRINCIPAL"),
                "expected UNEXPECTED_FIXED_PRINCIPAL, got: {:?}",
                errors
            );
        }
        other => panic!(
            "expected FixedPrincipalInvalid or GraphValidationFailed, got: {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn test_fixed_principal_missing_id_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;

    let (_def_id, version_id) = {
        let def_id = uuid::Uuid::new_v4();
        let def_key = format!("test-def-{}", &uuid::Uuid::new_v4().to_string()[..8]);
        sqlx::query(
            "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Test Definition')",
        )
        .bind(def_id)
        .bind(domain_id)
        .bind(&def_key)
        .execute(&pool)
        .await
        .expect("failed to insert definition");

        let create_cmd = CreateDraftVersion {
            actor_principal_id: owner,
            workflow_definition_id: def_id,
            context_schema: Some(serde_json::json!({"type": "object"})),
            json_schema_dialect: Some("https://json-schema.org/draft/2020-12/schema".to_string()),
            validator_version: Some("v1".to_string()),
            metadata: None,
        };
        let version = service
            .create_draft_version(create_cmd)
            .await
            .expect("create draft version");
        (def_id, version.id.into_uuid())
    };

    let (mut raw_nodes, raw_transitions) = valid_raw_graph();
    raw_nodes[1].assignee_ref_type = Some("FIXED_PRINCIPAL".to_string());
    raw_nodes[1].fixed_principal_id = None;

    let replace_cmd = ReplaceDraftGraph {
        actor_principal_id: owner,
        definition_version_id: version_id,
        context_schema: Some(serde_json::json!({"type": "object"})),
        nodes: raw_nodes,
        transitions: raw_transitions,
    };
    let result = service.replace_draft_graph(replace_cmd).await;
    assert!(
        result.is_err(),
        "FIXED_PRINCIPAL without ID should be rejected"
    );
    match result.unwrap_err() {
        DefinitionError::FixedPrincipalInvalid(_) => {}
        DefinitionError::GraphValidationFailed(errors) => {
            assert!(
                errors
                    .iter()
                    .any(|e| e.code == "FIXED_PRINCIPAL_MISSING_ID"),
                "expected FIXED_PRINCIPAL_MISSING_ID, got: {:?}",
                errors
            );
        }
        other => panic!(
            "expected FixedPrincipalInvalid or GraphValidationFailed, got: {:?}",
            other
        ),
    }
}

#[tokio::test]
async fn test_fixed_principal_disabled_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let disabled = seed_disabled_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, disabled).await;

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
            expected_revision: None,
        })
        .await;
    assert!(result.is_err(), "disabled principal should be rejected");
    match result.unwrap_err() {
        DefinitionError::FixedPrincipalInvalid(_) => {}
        other => panic!("expected FixedPrincipalInvalid, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_terminal_without_assignee_allowed() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
            expected_revision: None,
        })
        .await;
    assert!(
        result.is_ok(),
        "terminal without assignee should be allowed, got: {:?}",
        result.err()
    );
}

// ---------------------------------------------------------------------------
// INSTANCE_INPUT_PRINCIPAL assignee shape (v1).
// ---------------------------------------------------------------------------

/// Build a valid raw graph whose NORMAL node uses INSTANCE_INPUT_PRINCIPAL
/// resolving from the `assigneePrincipalId` instance input key.
fn valid_raw_graph_instance_input_principal() -> (
    Vec<svc_workflow::application::definition::commands::RawNodeDefinition>,
    Vec<svc_workflow::application::definition::commands::RawTransitionDefinition>,
) {
    use svc_workflow::application::definition::commands::{
        RawNodeDefinition, RawTransitionDefinition,
    };
    let nodes = vec![
        RawNodeDefinition {
            node_key: "draft".to_string(),
            display_name: "Draft".to_string(),
            order_index: 0,
            node_type: "DRAFT".to_string(),
            assignee_ref_type: Some("WORKFLOW_CREATOR".to_string()),
            fixed_principal_id: None,
            assignee_input_key: None,
            instructions: None,
            primary_advance_transition_key: Some("advance-do".to_string()),
            metadata: None,
        },
        RawNodeDefinition {
            node_key: "do".to_string(),
            display_name: "Do".to_string(),
            order_index: 1,
            node_type: "NORMAL".to_string(),
            assignee_ref_type: Some("INSTANCE_INPUT_PRINCIPAL".to_string()),
            fixed_principal_id: None,
            assignee_input_key: Some("assigneePrincipalId".to_string()),
            instructions: None,
            primary_advance_transition_key: Some("advance-done".to_string()),
            metadata: None,
        },
        RawNodeDefinition {
            node_key: "done".to_string(),
            display_name: "Done".to_string(),
            order_index: 2,
            node_type: "TERMINAL".to_string(),
            assignee_ref_type: None,
            fixed_principal_id: None,
            assignee_input_key: None,
            instructions: None,
            primary_advance_transition_key: None,
            metadata: None,
        },
    ];
    let transitions = vec![
        RawTransitionDefinition {
            transition_key: "advance-do".to_string(),
            display_name: "To Do".to_string(),
            source_node_key: "draft".to_string(),
            target_node_key: "do".to_string(),
            transition_effect: "ADVANCE".to_string(),
            submission_schema: None,
            metadata: None,
        },
        RawTransitionDefinition {
            transition_key: "advance-done".to_string(),
            display_name: "Complete".to_string(),
            source_node_key: "do".to_string(),
            target_node_key: "done".to_string(),
            transition_effect: "ADVANCE".to_string(),
            submission_schema: None,
            metadata: None,
        },
    ];
    (nodes, transitions)
}

#[tokio::test]
async fn instance_input_principal_node_publishes() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;

    let def_id = uuid::Uuid::new_v4();
    let def_key = format!("iip-def-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'IIP Def')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(&pool)
    .await
    .expect("insert def");

    let version = service
        .create_draft_version(CreateDraftVersion {
            actor_principal_id: owner,
            workflow_definition_id: def_id,
            context_schema: Some(serde_json::json!({"type":"object"})),
            json_schema_dialect: None,
            validator_version: None,
            metadata: None,
        })
        .await
        .expect("create draft");
    let version_id = version.id.into_uuid();

    let (nodes, transitions) = valid_raw_graph_instance_input_principal();
    service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: owner,
            definition_version_id: version_id,
            // The definition must be self-consistent: context_schema.required
            // covers the INSTANCE_INPUT_PRINCIPAL key.
            context_schema: Some(serde_json::json!({
                "type": "object",
                "required": ["assigneePrincipalId"],
            })),
            nodes,
            transitions,
        })
        .await
        .expect("replace graph");

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
            expected_revision: None,
        })
        .await;
    assert!(
        result.is_ok(),
        "INSTANCE_INPUT_PRINCIPAL node with a valid key should publish, got: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn instance_input_principal_missing_key_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;

    let def_id = uuid::Uuid::new_v4();
    let def_key = format!("iip-mk-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'IIP Def')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(&pool)
    .await
    .expect("insert def");
    let version = service
        .create_draft_version(CreateDraftVersion {
            actor_principal_id: owner,
            workflow_definition_id: def_id,
            context_schema: Some(serde_json::json!({"type":"object"})),
            json_schema_dialect: None,
            validator_version: None,
            metadata: None,
        })
        .await
        .expect("create draft");
    let version_id = version.id.into_uuid();

    let (mut nodes, transitions) = valid_raw_graph_instance_input_principal();
    // Remove the required key.
    nodes[1].assignee_input_key = None;

    let result = service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: owner,
            definition_version_id: version_id,
            context_schema: None,
            nodes,
            transitions,
        })
        .await;
    assert!(
        result.is_err(),
        "INSTANCE_INPUT_PRINCIPAL without an input key must be rejected"
    );
}

#[tokio::test]
async fn instance_input_principal_schema_not_covering_key_rejected() {
    // A contradictory definition (IIP node key not covered by
    // context_schema.required) must fail at draft/publish time — never after
    // instances exist.
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;

    let def_id = uuid::Uuid::new_v4();
    let def_key = format!("iip-sc-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'IIP Def')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(&pool)
    .await
    .expect("insert def");
    let version = service
        .create_draft_version(CreateDraftVersion {
            actor_principal_id: owner,
            workflow_definition_id: def_id,
            context_schema: Some(serde_json::json!({"type":"object"})),
            json_schema_dialect: None,
            validator_version: None,
            metadata: None,
        })
        .await
        .expect("create draft");
    let version_id = version.id.into_uuid();

    let (nodes, transitions) = valid_raw_graph_instance_input_principal();
    // Schema exists but does NOT require the assignee key -> the graph
    // validation must reject the self-contradictory definition already at
    // draft-replace time (and therefore also at publish time).
    let result = service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: owner,
            definition_version_id: version_id,
            context_schema: Some(serde_json::json!({"type": "object"})),
            nodes,
            transitions,
        })
        .await;
    assert!(
        result.is_err(),
        "a graph whose INSTANCE_INPUT_PRINCIPAL key is not in context_schema.required must be rejected"
    );
}

#[tokio::test]
async fn instance_input_principal_with_fixed_id_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let def_id = uuid::Uuid::new_v4();
    let def_key = format!("iip-fp-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'IIP Def')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(&pool)
    .await
    .expect("insert def");
    let version = service
        .create_draft_version(CreateDraftVersion {
            actor_principal_id: owner,
            workflow_definition_id: def_id,
            context_schema: Some(serde_json::json!({"type":"object"})),
            json_schema_dialect: None,
            validator_version: None,
            metadata: None,
        })
        .await
        .expect("create draft");
    let version_id = version.id.into_uuid();

    let (mut nodes, transitions) = valid_raw_graph_instance_input_principal();
    // INSTANCE_INPUT_PRINCIPAL must not also carry a fixed principal id.
    nodes[1].fixed_principal_id = Some(assignee);

    let result = service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: owner,
            definition_version_id: version_id,
            context_schema: None,
            nodes,
            transitions,
        })
        .await;
    assert!(
        result.is_err(),
        "INSTANCE_INPUT_PRINCIPAL with a fixed_principal_id must be rejected"
    );
}
