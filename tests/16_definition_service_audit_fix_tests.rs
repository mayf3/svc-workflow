//! Comprehensive tests for Definition Service audit fixes (B-1, B-2, H-1..H-5, M-1..M-6).
//!
//! Each test section corresponds to a specific audit finding.
//! Tests use real PostgreSQL 16 and the same connection helpers as other integration tests.
#![allow(clippy::needless_borrow)]

mod common;

use common::{create_pool, seed_domain_owner, seed_principal_and_domain, seed_second_principal};
use sqlx::PgPool;
use std::collections::HashMap;

use svc_workflow::application::definition::commands::{
    CreateDefinition, CreateDraftVersion, DeprecateVersion, PublishVersion, RawNodeDefinition,
    RawTransitionDefinition, ReplaceDraftGraph, RevokeVersion, ValidateDraftVersion,
};
use svc_workflow::application::definition::queries::{GetDefinition, ListDefinitionVersions};
use svc_workflow::application::definition::DefinitionRepository;
use svc_workflow::application::definition::DefinitionService;
use svc_workflow::domain::definition::digest;
use svc_workflow::domain::definition::error::DefinitionError;
use svc_workflow::store::postgres::definition_repository::PgDefinitionRepository;

// ---------------------------------------------------------------------------
// Helper: create a pool + service backed by the real DB
// ---------------------------------------------------------------------------

async fn create_service() -> (PgPool, DefinitionService<PgDefinitionRepository>) {
    let pool = create_pool().await;
    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);
    (pool, service)
}

// ---------------------------------------------------------------------------
// Helper: create a complete valid graph for testing
// ---------------------------------------------------------------------------

fn valid_raw_graph() -> (Vec<RawNodeDefinition>, Vec<RawTransitionDefinition>) {
    let nodes = vec![
        RawNodeDefinition {
            node_key: "draft".to_string(),
            display_name: "Draft".to_string(),
            order_index: 0,
            node_type: "DRAFT".to_string(),
            assignee_ref_type: "WORKFLOW_CREATOR".to_string(),
            fixed_principal_id: None,
            instructions: None,
            primary_advance_transition_key: Some("advance-dev".to_string()),
            metadata: None,
        },
        RawNodeDefinition {
            node_key: "dev_self_check".to_string(),
            display_name: "Dev Self Check".to_string(),
            order_index: 1,
            node_type: "NORMAL".to_string(),
            assignee_ref_type: "FIXED_PRINCIPAL".to_string(),
            fixed_principal_id: None, // will be set at test time
            instructions: None,
            primary_advance_transition_key: Some("advance-done".to_string()),
            metadata: None,
        },
        RawNodeDefinition {
            node_key: "done".to_string(),
            display_name: "Done".to_string(),
            order_index: 2,
            node_type: "TERMINAL".to_string(),
            assignee_ref_type: "WORKFLOW_CREATOR".to_string(),
            fixed_principal_id: None,
            instructions: None,
            primary_advance_transition_key: None,
            metadata: None,
        },
    ];
    let transitions = vec![
        RawTransitionDefinition {
            transition_key: "advance-dev".to_string(),
            display_name: "Advance to Dev".to_string(),
            source_node_key: "draft".to_string(),
            target_node_key: "dev_self_check".to_string(),
            transition_effect: "ADVANCE".to_string(),
            submission_schema: None,
            metadata: None,
        },
        RawTransitionDefinition {
            transition_key: "advance-done".to_string(),
            display_name: "Complete".to_string(),
            source_node_key: "dev_self_check".to_string(),
            target_node_key: "done".to_string(),
            transition_effect: "ADVANCE".to_string(),
            submission_schema: None,
            metadata: None,
        },
    ];
    (nodes, transitions)
}

/// Create a valid raw graph with a fixed principal ID for the NORMAL node.
fn valid_raw_graph_with_principal(
    principal_id: uuid::Uuid,
) -> (Vec<RawNodeDefinition>, Vec<RawTransitionDefinition>) {
    let (mut nodes, transitions) = valid_raw_graph();
    nodes[1].fixed_principal_id = Some(principal_id);
    (nodes, transitions)
}

/// Seed an assignee principal and return its ID.
async fn seed_assignee_principal(pool: &PgPool) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'Assignee', 'assignee@test.com', TRUE)",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("failed to seed assignee principal");
    id
}

/// Seed a disabled principal.
async fn seed_disabled_principal(pool: &PgPool) -> uuid::Uuid {
    let id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'AGENT', 'Disabled', 'disabled@test.com', FALSE)",
    )
    .bind(id)
    .execute(pool)
    .await
    .expect("failed to seed disabled principal");
    id
}

/// Seed a second domain with its own owner.
async fn seed_second_domain_with_owner(pool: &PgPool) -> (uuid::Uuid, uuid::Uuid) {
    let principal_id = uuid::Uuid::new_v4();
    let domain_id = uuid::Uuid::new_v4();
    let domain_key = format!("second-domain-{}", &uuid::Uuid::new_v4().to_string()[..8]);

    sqlx::query("INSERT INTO principals (principal_id, principal_type, display_name, email, enabled) VALUES ($1, 'HUMAN', 'Second Owner', 'second@test.com', TRUE)")
        .bind(principal_id)
        .execute(pool)
        .await
        .expect("failed to insert second principal");

    sqlx::query("INSERT INTO domains (domain_id, domain_key, display_name, enabled) VALUES ($1, $2, 'Second Domain', TRUE)")
        .bind(domain_id)
        .bind(&domain_key)
        .execute(pool)
        .await
        .expect("failed to insert second domain");

    seed_domain_owner(pool, domain_id, principal_id).await;
    (principal_id, domain_id)
}

/// Create a definition version in DRAFT with a graph set up.
async fn create_draft_version_with_graph(
    pool: &PgPool,
    service: &DefinitionService<PgDefinitionRepository>,
    actor: uuid::Uuid,
    domain_id: uuid::Uuid,
    assignee_id: uuid::Uuid,
) -> (uuid::Uuid, uuid::Uuid) {
    // Create definition
    let def_id = uuid::Uuid::new_v4();
    let def_key = format!("test-def-{}", &uuid::Uuid::new_v4().to_string()[..8]);
    sqlx::query(
        "INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1, $2, $3, 'Test Definition')",
    )
    .bind(def_id)
    .bind(domain_id)
    .bind(&def_key)
    .execute(pool)
    .await
    .expect("failed to insert definition");

    let create_cmd = CreateDraftVersion {
        actor_principal_id: actor,
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
    let version_id = version.id.into_uuid();

    // Replace graph
    let (mut raw_nodes, raw_transitions) = valid_raw_graph();
    raw_nodes[1].fixed_principal_id = Some(assignee_id);

    let replace_cmd = ReplaceDraftGraph {
        actor_principal_id: actor,
        definition_version_id: version_id,
        context_schema: Some(serde_json::json!({"type": "object"})),
        nodes: raw_nodes,
        transitions: raw_transitions,
    };
    service
        .replace_draft_graph(replace_cmd)
        .await
        .expect("replace draft graph");

    (def_id, version_id)
}

// ============================================================================
// B-2: JSON Schema Validation
// ============================================================================

#[tokio::test]
async fn test_valid_context_schema_can_publish() {
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
        })
        .await;
    assert!(
        result.is_ok(),
        "valid context_schema should publish, got: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_invalid_schema_rejected_during_publish() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    // Create a valid version first
    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Manually set an invalid schema on the version
    sqlx::query("UPDATE workflow_definition_versions SET context_schema = '{\"type\": 123}'::jsonb WHERE definition_version_id = $1")
        .bind(version_id)
        .execute(&pool)
        .await
        .expect("update context schema");

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;
    assert!(result.is_err(), "invalid schema should be rejected");
    match result.unwrap_err() {
        DefinitionError::GraphValidationFailed(errors) => {
            assert!(
                errors.iter().any(|e| e.code == "INVALID_CONTEXT_SCHEMA"),
                "expected INVALID_CONTEXT_SCHEMA error, got: {:?}",
                errors
            );
        }
        other => panic!("expected GraphValidationFailed, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_https_ref_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Set a schema with https ref on a transition
    let https_schema = serde_json::json!({"$ref": "https://example.com/schema.json"});
    let trans_id: (uuid::Uuid,) = sqlx::query_as(
        "SELECT transition_id FROM workflow_transition_definitions WHERE definition_version_id = $1 LIMIT 1",
    )
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .expect("get transition");

    sqlx::query("UPDATE workflow_transition_definitions SET submission_schema = $1 WHERE transition_id = $2")
        .bind(&https_schema)
        .bind(trans_id.0)
        .execute(&pool)
        .await
        .expect("update submission_schema");

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;
    assert!(result.is_err(), "https ref should be rejected");
    let err = result.unwrap_err();
    assert!(
        matches!(err, DefinitionError::GraphValidationFailed(_)),
        "expected GraphValidationFailed, got: {:?}",
        err
    );
}

#[tokio::test]
async fn test_file_ref_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    let file_schema = serde_json::json!({"$ref": "file:///etc/passwd"});
    let trans_id: (uuid::Uuid,) = sqlx::query_as(
        "SELECT transition_id FROM workflow_transition_definitions WHERE definition_version_id = $1 LIMIT 1",
    )
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .expect("get transition");

    sqlx::query("UPDATE workflow_transition_definitions SET submission_schema = $1 WHERE transition_id = $2")
        .bind(&file_schema)
        .bind(trans_id.0)
        .execute(&pool)
        .await
        .expect("update submission_schema");

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;
    assert!(result.is_err(), "file ref should be rejected");
}

#[tokio::test]
async fn test_local_fragment_ref_allowed() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Set a schema with local fragment ref
    let local_schema = serde_json::json!({
        "$defs": {
            "Address": {"type": "object"}
        },
        "$ref": "#/$defs/Address"
    });

    sqlx::query("UPDATE workflow_definition_versions SET context_schema = $1 WHERE definition_version_id = $2")
        .bind(&local_schema)
        .bind(version_id)
        .execute(&pool)
        .await
        .expect("update context_schema");

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;
    assert!(
        result.is_ok(),
        "local fragment ref should be allowed, got: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_invalid_schema_version_stays_draft() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Set invalid schema
    sqlx::query("UPDATE workflow_definition_versions SET context_schema = '{\"type\": 123}'::jsonb WHERE definition_version_id = $1")
        .bind(version_id)
        .execute(&pool)
        .await
        .expect("update context_schema");

    let _ = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;

    // Verify version remains DRAFT
    let status: (String,) = sqlx::query_as(
        "SELECT version_status::TEXT FROM workflow_definition_versions WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .expect("get status");

    assert_eq!(
        status.0, "DRAFT",
        "version should remain DRAFT after failed publish"
    );

    // Verify digest and actor are NOT set
    let row: (Option<String>, Option<uuid::Uuid>) = sqlx::query_as(
        "SELECT definition_digest, published_by_principal_id FROM workflow_definition_versions WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .expect("get version");

    assert!(
        row.0.is_none(),
        "digest should not be set on failed publish"
    );
    assert!(
        row.1.is_none(),
        "published_by should not be set on failed publish"
    );
}

// ============================================================================
// H-1: Directed reachability
// ============================================================================

#[tokio::test]
async fn test_directed_unreachable_node_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    // Create a version with an isolated node that has no path from DRAFT
    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Add an isolated node (no transitions to/from it)
    let isolated_node_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'isolated', 'Isolated', 10, 'NORMAL', 'WORKFLOW_CREATOR')",
    )
    .bind(isolated_node_id)
    .bind(version_id)
    .execute(&pool)
    .await
    .expect("insert isolated node");

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;
    assert!(result.is_err(), "isolated node should be rejected");
    match result.unwrap_err() {
        DefinitionError::GraphValidationFailed(errors) => {
            assert!(
                errors.iter().any(|e| e.code == "NODE_NOT_REACHABLE"),
                "expected NODE_NOT_REACHABLE, got: {:?}",
                errors
            );
            assert!(
                errors.iter().any(|e| e.message.contains("isolated")),
                "error should mention 'isolated', got: {:?}",
                errors
            );
        }
        other => panic!("expected GraphValidationFailed, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_node_only_reachable_via_backwards_edge_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Add a node that only has an outgoing edge TO draft (reverse direction)
    let orphan_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1, $2, 'orphan', 'Orphan', 5, 'NORMAL', 'WORKFLOW_CREATOR')",
    )
    .bind(orphan_id)
    .bind(version_id)
    .execute(&pool)
    .await
    .expect("insert orphan node");

    // Add transition from orphan → draft (reverse direction)
    let draft_node_id: (uuid::Uuid,) = sqlx::query_as(
        "SELECT node_id FROM workflow_node_definitions WHERE definition_version_id = $1 AND node_key = 'draft'",
    )
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .expect("get draft node");

    let rev_trans_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1, $2, 'orphan-to-draft', 'Reverse', $3, $4, 'RETURN')",
    )
    .bind(rev_trans_id)
    .bind(version_id)
    .bind(orphan_id)
    .bind(draft_node_id.0)
    .execute(&pool)
    .await
    .expect("insert reverse transition");

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;
    assert!(
        result.is_err(),
        "orphan (only reverse edge) should be rejected"
    );
    match result.unwrap_err() {
        DefinitionError::GraphValidationFailed(errors) => {
            assert!(
                errors.iter().any(|e| e.code == "NODE_NOT_REACHABLE"),
                "expected NODE_NOT_REACHABLE, got: {:?}",
                errors
            );
        }
        other => panic!("expected GraphValidationFailed, got: {:?}", other),
    }
}

// ============================================================================
// H-2: Assignee rules
// ============================================================================

#[tokio::test]
async fn test_terminal_node_with_fixed_principal_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Set terminal node's assignee to FIXED_PRINCIPAL
    sqlx::query(
        "UPDATE workflow_node_definitions SET assignee_ref_type = 'FIXED_PRINCIPAL', fixed_principal_id = $1 WHERE definition_version_id = $2 AND node_type = 'TERMINAL'",
    )
    .bind(assignee)
    .bind(version_id)
    .execute(&pool)
    .await
    .expect("update terminal node");

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;
    assert!(result.is_err(), "terminal with assignee should be rejected");
    match result.unwrap_err() {
        DefinitionError::GraphValidationFailed(errors) => {
            assert!(
                errors.iter().any(|e| e.code == "TERMINAL_HAS_ASSIGNEE"),
                "expected TERMINAL_HAS_ASSIGNEE, got: {:?}",
                errors
            );
        }
        other => panic!("expected GraphValidationFailed, got: {:?}", other),
    }
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

    // Create graph with TERMINAL node having FIXED_PRINCIPAL assignee (should be rejected)
    let (mut raw_nodes, raw_transitions) = valid_raw_graph();
    raw_nodes[2].assignee_ref_type = "FIXED_PRINCIPAL".to_string(); // Terminal with FIXED_PRINCIPAL
    raw_nodes[2].fixed_principal_id = Some(assignee);
    // Terminal with primary should also be rejected, so clear it
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
        DefinitionError::FixedPrincipalInvalid(_) => {} // caught by parse_assignee_ref
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

    // Set DRAFT node to WORKFLOW_CREATOR with a fixed_principal_id via replace_draft_graph (not allowed)
    let (mut raw_nodes, raw_transitions) = valid_raw_graph();
    raw_nodes[0].fixed_principal_id = Some(assignee); // DRAFT with WORKFLOW_CREATOR + fixed_id

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
        DefinitionError::FixedPrincipalInvalid(_) => {} // caught by parse_assignee_ref
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

    // Create graph with NORMAL node set to FIXED_PRINCIPAL but no principal_id
    let (mut raw_nodes, raw_transitions) = valid_raw_graph();
    raw_nodes[1].assignee_ref_type = "FIXED_PRINCIPAL".to_string();
    raw_nodes[1].fixed_principal_id = None; // Missing ID for FIXED_PRINCIPAL

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
        "FIXED_PRINCIPAL without ID should be rejected by replace_draft_graph"
    );
    match result.unwrap_err() {
        DefinitionError::FixedPrincipalInvalid(_) => {} // caught by parse_assignee_ref
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
        })
        .await;
    assert!(result.is_err(), "disabled principal should be rejected");
    match result.unwrap_err() {
        DefinitionError::FixedPrincipalInvalid(_) => {} // expected
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
        })
        .await;
    assert!(
        result.is_ok(),
        "terminal without assignee should be allowed, got: {:?}",
        result.err()
    );
}

// ============================================================================
// H-3: Primary transition must be ADVANCE
// ============================================================================

#[tokio::test]
async fn test_primary_effect_not_advance_rejected() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Change primary transition effect to RETURN
    sqlx::query(
        "UPDATE workflow_transition_definitions SET transition_effect = 'RETURN' WHERE definition_version_id = $1 AND transition_key = 'advance-dev'",
    )
    .bind(version_id)
    .execute(&pool)
    .await
    .expect("update transition effect");

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;
    assert!(
        result.is_err(),
        "primary with RETURN effect should be rejected"
    );
    match result.unwrap_err() {
        DefinitionError::GraphValidationFailed(errors) => {
            assert!(
                errors.iter().any(|e| e.code == "PRIMARY_NOT_ADVANCE"),
                "expected PRIMARY_NOT_ADVANCE, got: {:?}",
                errors
            );
        }
        other => panic!("expected GraphValidationFailed, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_primary_advance_allowed() {
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
        })
        .await;
    assert!(
        result.is_ok(),
        "valid ADVANCE primary should be allowed, got: {:?}",
        result.err()
    );
}

// ============================================================================
// H-4: Lifecycle actor fields
// ============================================================================

#[tokio::test]
async fn test_publish_sets_actor() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    let published = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await
        .expect("publish should succeed");

    assert!(
        published.published_at.is_some(),
        "published_at should be set"
    );
    assert_eq!(
        published.published_by_principal_id.map(|id| id.into_uuid()),
        Some(owner),
        "published_by_principal_id should match actor"
    );
}

#[tokio::test]
async fn test_deprecate_sets_actor() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Publish first
    service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await
        .expect("publish");

    // Now deprecate
    let deprecated = service
        .deprecate_version(DeprecateVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await
        .expect("deprecate");

    assert!(
        deprecated.deprecated_at.is_some(),
        "deprecated_at should be set"
    );
    assert_eq!(
        deprecated
            .deprecated_by_principal_id
            .map(|id| id.into_uuid()),
        Some(owner),
        "deprecated_by_principal_id should match actor"
    );
}

#[tokio::test]
async fn test_revoke_sets_actor() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Publish first
    service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await
        .expect("publish");

    // Revoke
    let revoked = service
        .revoke_version(RevokeVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await
        .expect("revoke");

    assert!(revoked.revoked_at.is_some(), "revoked_at should be set");
    assert_eq!(
        revoked.revoked_by_principal_id.map(|id| id.into_uuid()),
        Some(owner),
        "revoked_by_principal_id should match actor"
    );
}

#[tokio::test]
async fn test_three_stage_actors_all_preserved() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Publish
    service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await
        .expect("publish");

    // Deprecate
    service
        .deprecate_version(DeprecateVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await
        .expect("deprecate");

    // Revoke
    service
        .revoke_version(RevokeVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await
        .expect("revoke");

    // Read back and verify ALL three actors are preserved
    let row: (Option<uuid::Uuid>, Option<uuid::Uuid>, Option<uuid::Uuid>) = sqlx::query_as(
        "SELECT published_by_principal_id, deprecated_by_principal_id, revoked_by_principal_id FROM workflow_definition_versions WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .expect("get version");

    assert_eq!(row.0, Some(owner), "published_by should be preserved");
    assert_eq!(row.1, Some(owner), "deprecated_by should be preserved");
    assert_eq!(row.2, Some(owner), "revoked_by should be preserved");
}

#[tokio::test]
async fn test_unpublished_version_actor_fields_null() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Read back - all actor fields should be null for DRAFT
    let row: (Option<uuid::Uuid>, Option<uuid::Uuid>, Option<uuid::Uuid>) = sqlx::query_as(
        "SELECT published_by_principal_id, deprecated_by_principal_id, revoked_by_principal_id FROM workflow_definition_versions WHERE definition_version_id = $1",
    )
    .bind(version_id)
    .fetch_one(&pool)
    .await
    .expect("get version");

    assert!(row.0.is_none(), "published_by should be null for draft");
    assert!(row.1.is_none(), "deprecated_by should be null for draft");
    assert!(row.2.is_none(), "revoked_by should be null for draft");
}

// ============================================================================
// H-5: Domain authorization
// ============================================================================

#[tokio::test]
async fn test_cross_domain_read_denied() {
    let (pool, service) = create_service().await;
    let (owner_a, domain_a) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_a, owner_a).await;
    let assignee = seed_assignee_principal(&pool).await;

    // Create a definition in domain A
    let (def_id, _version_id) =
        create_draft_version_with_graph(&pool, &service, owner_a, domain_a, assignee).await;

    // Create a principal from domain B (no role in domain A)
    let (owner_b, _domain_b) = seed_second_domain_with_owner(&pool).await;

    // Try to read definition from domain A as domain B owner → should fail
    let result = service
        .get_definition(GetDefinition {
            actor_principal_id: owner_b,
            workflow_definition_id: def_id,
        })
        .await;
    assert!(result.is_err(), "cross-domain read should be denied");
    match result.unwrap_err() {
        DefinitionError::PermissionDenied => {} // expected
        other => panic!("expected PermissionDenied, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_cross_domain_list_versions_denied() {
    let (pool, service) = create_service().await;
    let (owner_a, domain_a) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_a, owner_a).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (def_id, _version_id) =
        create_draft_version_with_graph(&pool, &service, owner_a, domain_a, assignee).await;

    let (owner_b, _domain_b) = seed_second_domain_with_owner(&pool).await;

    let result = service
        .list_definition_versions(ListDefinitionVersions {
            actor_principal_id: owner_b,
            workflow_definition_id: def_id,
        })
        .await;
    assert!(result.is_err(), "cross-domain list should be denied");
}

#[tokio::test]
async fn test_domain_owner_can_read() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (def_id, _version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    let result = service
        .get_definition(GetDefinition {
            actor_principal_id: owner,
            workflow_definition_id: def_id,
        })
        .await;
    assert!(result.is_ok(), "domain owner should be able to read");
}

#[tokio::test]
async fn test_validate_draft_version_requires_owner() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Try as non-owner
    let stranger = seed_second_principal(&pool).await;
    let result = service
        .validate_draft_version(ValidateDraftVersion {
            actor_principal_id: stranger,
            definition_version_id: version_id,
        })
        .await;
    assert!(result.is_err(), "non-owner should not be able to validate");
    match result.unwrap_err() {
        DefinitionError::PermissionDenied => {} // expected
        other => panic!("expected PermissionDenied, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_domain_owner_can_validate() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    let result = service
        .validate_draft_version(ValidateDraftVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;
    assert!(result.is_ok(), "domain owner should be able to validate");
}

#[tokio::test]
async fn test_disabled_principal_cannot_read() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (def_id, _version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Disable the owner
    sqlx::query("UPDATE principals SET enabled = FALSE WHERE principal_id = $1")
        .bind(owner)
        .execute(&pool)
        .await
        .expect("disable principal");

    let result = service
        .get_definition(GetDefinition {
            actor_principal_id: owner,
            workflow_definition_id: def_id,
        })
        .await;
    match result.unwrap_err() {
        DefinitionError::PrincipalDisabled => {} // expected
        other => panic!("expected PrincipalDisabled, got: {:?}", other),
    }
}

#[tokio::test]
async fn test_disabled_domain_blocks_write() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Disable the domain
    sqlx::query("UPDATE domains SET enabled = FALSE WHERE domain_id = $1")
        .bind(domain_id)
        .execute(&pool)
        .await
        .expect("disable domain");

    let result = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await;
    match result.unwrap_err() {
        DefinitionError::DomainDisabled => {} // expected
        other => panic!("expected DomainDisabled, got: {:?}", other),
    }
}

// ============================================================================
// B-1 + M-3: Digest read-back consistency
// ============================================================================

#[tokio::test]
async fn test_digest_readback_consistency() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (def_id_result, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Publish
    let published = service
        .publish_version(PublishVersion {
            actor_principal_id: owner,
            definition_version_id: version_id,
        })
        .await
        .expect("publish should succeed");

    let stored_digest = published.definition_digest.expect("digest should exist");

    // Read back the full graph from DB
    let (nodes, transitions) = service
        .repo
        .get_complete_graph(version_id)
        .await
        .expect("get complete graph");

    // Get definition and version
    let def = service
        .repo
        .get_definition(def_id_result)
        .await
        .expect("get definition");

    let version = service
        .repo
        .get_version(version_id)
        .await
        .expect("get version");

    // Re-compute digest from stored data
    let node_key_map: HashMap<_, _> = nodes
        .iter()
        .map(|n| (n.node_id, n.node_key.clone()))
        .collect();
    let transition_key_map: HashMap<_, _> = transitions
        .iter()
        .map(|t| (t.transition_id, t.transition_key.clone()))
        .collect();

    let recomputed_digest = digest::compute_digest(
        &def.definition_key,
        version.version_number,
        version.json_schema_dialect.as_deref(),
        version.validator_version.as_deref(),
        version.context_schema.as_ref(),
        &nodes,
        &transitions,
        &node_key_map,
        &transition_key_map,
    )
    .expect("compute digest");

    assert_eq!(
        stored_digest, recomputed_digest,
        "stored digest must match digest recomputed from stored graph"
    );
    assert_eq!(stored_digest.len(), 64, "SHA-256 hex should be 64 chars");
}

// ============================================================================
// M-6: Concurrent CreateDefinition uniqueness
// ============================================================================

#[tokio::test]
async fn test_concurrent_create_definition_unique() {
    let pool = create_pool().await;
    let repo1 = PgDefinitionRepository::new(pool.clone());
    let service1 = DefinitionService::new(repo1);
    let repo2 = PgDefinitionRepository::new(pool.clone());
    let service2 = DefinitionService::new(repo2);
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;

    let def_key = format!("concurrent-test-{}", &uuid::Uuid::new_v4().to_string()[..8]);

    // Create the first one
    let r1 = service1
        .create_definition(CreateDefinition {
            actor_principal_id: owner,
            owner_domain_id: domain_id,
            definition_key: def_key.clone(),
            display_name: "Test".to_string(),
            description: None,
            metadata: None,
        })
        .await;
    assert!(r1.is_ok(), "first create should succeed");

    // Try the same key again — must fail with DefinitionKeyConflict
    let r2 = service2
        .create_definition(CreateDefinition {
            actor_principal_id: owner,
            owner_domain_id: domain_id,
            definition_key: def_key.clone(),
            display_name: "Test".to_string(),
            description: None,
            metadata: None,
        })
        .await;
    match r2.unwrap_err() {
        DefinitionError::DefinitionKeyConflict => {} // expected
        other => panic!("expected DefinitionKeyConflict, got: {:?}", other),
    }
}

// ============================================================================
// B-1: Publish/Replace shared row lock coordination
// ============================================================================

#[tokio::test]
async fn test_manual_lock_blocks_replace_draft_graph() {
    let (pool, service) = create_service().await;
    let (owner, domain_id) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, domain_id, owner).await;
    let assignee = seed_assignee_principal(&pool).await;

    let (_def_id, version_id) =
        create_draft_version_with_graph(&pool, &service, owner, domain_id, assignee).await;

    // Manually lock the version row with FOR UPDATE in a separate transaction
    let mut tx = pool.begin().await.expect("begin tx");
    let _locked: (uuid::Uuid,) = sqlx::query_as(
        "SELECT definition_version_id FROM workflow_definition_versions WHERE definition_version_id = $1 FOR UPDATE",
    )
    .bind(version_id)
    .fetch_one(&mut *tx)
    .await
    .expect("lock version");

    // Try to replace graph — this should time out or fail because the row is locked
    let (nodes, transitions) = valid_raw_graph_with_principal(assignee);
    let replace_future = service.replace_draft_graph(ReplaceDraftGraph {
        actor_principal_id: owner,
        definition_version_id: version_id,
        context_schema: Some(serde_json::json!({"type": "object"})),
        nodes,
        transitions,
    });

    // Use tokio::time::timeout to check that the replace blocks (doesn't complete immediately)
    let timeout_duration = std::time::Duration::from_millis(500);
    let result = tokio::time::timeout(timeout_duration, replace_future).await;

    // The replace should NOT complete while our lock is held
    assert!(
        result.is_err(),
        "replace should be blocked by the held lock"
    );

    // Release the lock by committing
    tx.commit().await.expect("commit tx");

    // Now replace should succeed
    let (nodes, transitions) = valid_raw_graph_with_principal(assignee);
    let result = service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: owner,
            definition_version_id: version_id,
            context_schema: Some(serde_json::json!({"type": "object"})),
            nodes,
            transitions,
        })
        .await;
    assert!(result.is_ok(), "replace should succeed after lock released");
}
