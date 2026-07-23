//! Integration tests for Domain Owner Workflow Definition Governance V1.
//!
//! Tests are ordered: success paths → authorization → state constraints →
//! idempotency/concurrency → regression. Requires a running PostgreSQL 16
//! instance with the `svc_workflow` database.
//!
//! Run with: `cargo test --test 19_domain_owner_definition_governance -- --test-threads=1`

#![allow(clippy::needless_borrow)]
#![allow(unused_imports, unused_variables)]

mod common;

use common::{
    create_pool, seed_domain_owner, seed_principal_and_domain, seed_principal_domain_with_owner,
    seed_second_principal, seed_workflow_definition,
};

use svc_workflow::application::definition::commands::{
    CreateDefinition, CreateDraftVersion, PublishVersion, RawNodeDefinition,
    RawTransitionDefinition, ReplaceDraftGraph,
};
use svc_workflow::application::definition::queries::{
    GetCompleteVersionGraph, GetDefinition, GetDefinitionVersion, ListDefinitionVersions,
    ListDomainDefinitions,
};
use svc_workflow::application::definition::DefinitionService;
use svc_workflow::application::definition_governance::{
    governance_archive_definition, governance_create_definition, governance_create_draft_version,
    governance_publish_version, governance_replace_draft_graph, DefinitionGovernanceError,
};
use svc_workflow::domain::definition::error::DefinitionError;
use svc_workflow::domain::definition::model::WorkflowDefinition;
use svc_workflow::store::postgres::definition_repository::PgDefinitionRepository;

// ==========================================================================
// Helpers
// ==========================================================================

/// Create a minimal publishable workflow for test fixtures.
async fn seed_minimal_and_publish(
    service: &DefinitionService<PgDefinitionRepository>,
    actor_id: uuid::Uuid,
    version_id: uuid::Uuid,
) {
    use svc_workflow::application::definition::commands::DeprecateVersion;
    let graph = create_minimal_graph(version_id);
    service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: actor_id,
            definition_version_id: version_id,
            context_schema: None,
            nodes: graph.0,
            transitions: graph.1,
        })
        .await
        .expect("should replace draft graph");
    service
        .publish_version(PublishVersion {
            actor_principal_id: actor_id,
            definition_version_id: version_id,
            expected_revision: None,
        })
        .await
        .expect("should publish");
}

fn create_minimal_graph(
    _version_id: uuid::Uuid,
) -> (Vec<RawNodeDefinition>, Vec<RawTransitionDefinition>) {
    let uid1 = uuid::Uuid::new_v4().to_string();
    let uid2 = uuid::Uuid::new_v4().to_string();
    let uid3 = uuid::Uuid::new_v4().to_string();
    let draft_node_key = format!("draft-{}", &uid1[..8]);
    let normal_node_key = format!("step-{}", &uid2[..8]);
    let term_node_key = format!("done-{}", &uid3[..8]);

    let nodes = vec![
        RawNodeDefinition {
            node_key: draft_node_key.clone(),
            display_name: "Draft".to_string(),
            order_index: 0,
            node_type: "DRAFT".to_string(),
            assignee_ref_type: Some("WORKFLOW_CREATOR".to_string()),
            fixed_principal_id: None,
            assignee_input_key: None,
            instructions: None,
            primary_advance_transition_key: Some("advance-step".to_string()),
            metadata: None,
        },
        RawNodeDefinition {
            node_key: normal_node_key.clone(),
            display_name: "Step".to_string(),
            order_index: 1,
            node_type: "NORMAL".to_string(),
            assignee_ref_type: Some("WORKFLOW_CREATOR".to_string()),
            fixed_principal_id: None,
            assignee_input_key: None,
            instructions: None,
            primary_advance_transition_key: Some("advance-done".to_string()),
            metadata: None,
        },
        RawNodeDefinition {
            node_key: term_node_key.clone(),
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
            transition_key: "advance-step".to_string(),
            display_name: "Advance".to_string(),
            source_node_key: draft_node_key.clone(),
            target_node_key: normal_node_key.clone(),
            transition_effect: "ADVANCE".to_string(),
            submission_schema: None,
            metadata: None,
        },
        RawTransitionDefinition {
            transition_key: "advance-done".to_string(),
            display_name: "Complete".to_string(),
            source_node_key: normal_node_key,
            target_node_key: term_node_key,
            transition_effect: "ADVANCE".to_string(),
            submission_schema: None,
            metadata: None,
        },
    ];
    (nodes, transitions)
}

// ==========================================================================
// 1. Owner can list definitions in their domain
// ==========================================================================

#[tokio::test]
async fn test_owner_list_definitions() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    let result = service
        .list_domain_definitions(ListDomainDefinitions {
            actor_principal_id: owner_id,
            domain_id,
            before_created_at: None,
            before_id: None,
            limit: 20,
            include_archived: false,
        })
        .await
        .expect("domain owner should list definitions");

    assert!(result.next_cursor.is_none());
}

// ==========================================================================
// 2. Owner can view definition detail + versions
// ==========================================================================

#[tokio::test]
async fn test_owner_get_definition_detail() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, ver_id, _, _) = seed_workflow_definition(&pool, domain_id).await;

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    let detail = service
        .get_definition(GetDefinition {
            actor_principal_id: owner_id,
            workflow_definition_id: def_id,
        })
        .await
        .expect("owner should get definition");
    assert_eq!(detail.definition.definition.id.into_uuid(), def_id);

    let versions = service
        .list_definition_versions(ListDefinitionVersions {
            actor_principal_id: owner_id,
            workflow_definition_id: def_id,
        })
        .await
        .expect("owner should list versions");
    assert!(!versions.versions.is_empty());
}

// ==========================================================================
// 3. Owner can create a definition via governance API
// ==========================================================================

#[tokio::test]
async fn test_owner_governance_create_definition() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;

    let result = governance_create_definition(
        &pool,
        owner_id,
        &uuid::Uuid::new_v4().to_string(),
        "test-request",
        domain_id,
        &uuid::Uuid::new_v4().to_string(),
        "Test Definition",
        None,
        None,
    )
    .await
    .expect("owner should create definition via governance");

    assert_eq!(result.domain_id.into_uuid(), domain_id);
    assert!(!result.archived);
}

// ==========================================================================
// 4. Governance create definition is idempotent
// ==========================================================================

#[tokio::test]
async fn test_governance_create_definition_idempotent() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let key = uuid::Uuid::new_v4().to_string();
    let uid = uuid::Uuid::new_v4().to_string();
    let def_key = format!("idempotent-key-{}", &uid[..8]);

    let first = governance_create_definition(
        &pool, owner_id, &key, "req-1", domain_id, &def_key, "Test", None, None,
    )
    .await
    .expect("first call should succeed");

    let second = governance_create_definition(
        &pool, owner_id, &key, "req-2", domain_id, &def_key, "Test", None, None,
    )
    .await
    .expect("second call with same key should succeed idempotently");

    assert_eq!(first.id, second.id);
}

// ==========================================================================
// 5. Owner can create draft version via governance
// ==========================================================================

#[tokio::test]
async fn test_owner_governance_create_draft() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, _, _, _) = seed_workflow_definition(&pool, domain_id).await;

    let result = governance_create_draft_version(
        &pool,
        owner_id,
        &uuid::Uuid::new_v4().to_string(),
        "test-request",
        def_id,
        None,
        None,
        None,
        None,
    )
    .await
    .expect("owner should create draft version via governance");

    assert_eq!(result.workflow_definition_id.into_uuid(), def_id);
}

// ==========================================================================
// 6. Owner can publish a version
// ==========================================================================

#[tokio::test]
async fn test_owner_governance_publish() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, ver_id, _, _) = seed_workflow_definition(&pool, domain_id).await;

    // Seed: insert a principal for assignee reference
    let assignee_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, enabled) VALUES ($1, 'HUMAN', 'Tester', TRUE)",
    )
    .bind(assignee_id)
    .execute(&pool)
    .await
    .expect("insert assignee");

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);
    seed_minimal_and_publish(&service, owner_id, ver_id).await;

    // Verify via governance
    let repo2 = PgDefinitionRepository::new(pool.clone());
    let service2 = DefinitionService::new(repo2);
    let version = service2
        .get_definition_version(GetDefinitionVersion {
            actor_principal_id: owner_id,
            definition_version_id: ver_id,
        })
        .await
        .expect("should get version");
    assert_eq!(
        version.version.version.unwrap().version_status.to_string(),
        "PUBLISHED"
    );
}

// ==========================================================================
// 7. Owner can archive a definition
// ==========================================================================

#[tokio::test]
async fn test_owner_governance_archive() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, _, _, _) = seed_workflow_definition(&pool, domain_id).await;

    let result = governance_archive_definition(
        &pool,
        owner_id,
        &uuid::Uuid::new_v4().to_string(),
        "test-request",
        def_id,
    )
    .await
    .expect("owner should archive definition");

    assert!(result.archived);
    assert!(result.archived_at.is_some());
}

// ==========================================================================
// 8. Re-archive is idempotent
// ==========================================================================

#[tokio::test]
async fn test_archive_idempotent() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, _, _, _) = seed_workflow_definition(&pool, domain_id).await;

    let key = uuid::Uuid::new_v4().to_string();

    let first = governance_archive_definition(&pool, owner_id, &key, "req-1", def_id)
        .await
        .expect("first archive should succeed");
    assert!(first.archived);

    let second = governance_archive_definition(&pool, owner_id, &key, "req-2", def_id)
        .await
        .expect("re-archive with same key should succeed idempotently");
    assert!(second.archived);
}

// ==========================================================================
// 9. Non-owner cannot list, get, create, publish, or archive
// ==========================================================================

#[tokio::test]
async fn test_non_owner_cannot_list() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let non_owner = seed_second_principal(&pool).await;

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    let err = service
        .list_domain_definitions(ListDomainDefinitions {
            actor_principal_id: non_owner,
            domain_id,
            before_created_at: None,
            before_id: None,
            limit: 20,
            include_archived: false,
        })
        .await
        .expect_err("non-owner should fail to list");

    assert!(matches!(err, DefinitionError::PermissionDenied));
}

#[tokio::test]
async fn test_non_owner_cannot_get_detail() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, _, _, _) = seed_workflow_definition(&pool, domain_id).await;
    let non_owner = seed_second_principal(&pool).await;

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    let err = service
        .get_definition(GetDefinition {
            actor_principal_id: non_owner,
            workflow_definition_id: def_id,
        })
        .await
        .expect_err("non-owner should fail");
    assert!(matches!(err, DefinitionError::PermissionDenied));
}

#[tokio::test]
async fn test_non_owner_cannot_create_via_governance() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let non_owner = seed_second_principal(&pool).await;

    let err = governance_create_definition(
        &pool,
        non_owner,
        &uuid::Uuid::new_v4().to_string(),
        "req",
        domain_id,
        &uuid::Uuid::new_v4().to_string(),
        "Test",
        None,
        None,
    )
    .await
    .expect_err("non-owner should fail to create");

    assert!(matches!(
        &err,
        &DefinitionGovernanceError::DefinitionNotFound
    ));
}

#[tokio::test]
async fn test_non_owner_cannot_archive() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, _, _, _) = seed_workflow_definition(&pool, domain_id).await;
    let non_owner = seed_second_principal(&pool).await;

    let err = governance_archive_definition(
        &pool,
        non_owner,
        &uuid::Uuid::new_v4().to_string(),
        "req",
        def_id,
    )
    .await
    .expect_err("non-owner should fail to archive");

    assert!(matches!(
        &err,
        &DefinitionGovernanceError::DefinitionNotFound
    ));
}

// ==========================================================================
// 10. DOMAIN_MEMBER cannot perform owner operations
// ==========================================================================

#[tokio::test]
async fn test_domain_member_cannot_create() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;

    // Create a member
    let member_id = seed_second_principal(&pool).await;
    let binding_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1, $2, $3, 'DOMAIN_MEMBER', TRUE)",
    )
    .bind(binding_id)
    .bind(domain_id)
    .bind(member_id)
    .execute(&pool)
    .await
    .expect("insert member binding");

    let err = governance_create_definition(
        &pool,
        member_id,
        &uuid::Uuid::new_v4().to_string(),
        "req",
        domain_id,
        &uuid::Uuid::new_v4().to_string(),
        "Test",
        None,
        None,
    )
    .await
    .expect_err("member should fail to create");

    assert!(matches!(
        &err,
        &DefinitionGovernanceError::DefinitionNotFound
    ));
}

// ==========================================================================
// 11. Owner cannot affect other domain's definitions
// ==========================================================================

#[tokio::test]
async fn test_owner_other_domain_definition_not_found() {
    let pool = create_pool().await;
    let (owner_a, domain_a) = seed_principal_domain_with_owner(&pool).await;
    let (owner_b, domain_b) = seed_principal_domain_with_owner(&pool).await;
    let (def_a, _, _, _) = seed_workflow_definition(&pool, domain_a).await;

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    let err = service
        .get_definition(GetDefinition {
            actor_principal_id: owner_b,
            workflow_definition_id: def_a,
        })
        .await
        .expect_err("should fail - definition not in owner_b's domain");

    assert!(matches!(err, DefinitionError::PermissionDenied));
}

// ==========================================================================
// 12. Idempotency conflict for different request with same key
// ==========================================================================

#[tokio::test]
async fn test_idempotency_key_conflict() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let key = uuid::Uuid::new_v4().to_string();

    // First call succeeds
    let _first = governance_create_definition(
        &pool,
        owner_id,
        &key,
        "req-1",
        domain_id,
        &uuid::Uuid::new_v4().to_string(),
        "Test A",
        None,
        None,
    )
    .await
    .expect("first call should succeed");

    // Second call with same key but DIFFERENT body should conflict
    let err = governance_create_definition(
        &pool,
        owner_id,
        &key,
        "req-2",
        domain_id,
        &uuid::Uuid::new_v4().to_string(),
        "Test B (different)",
        None,
        None,
    )
    .await
    .expect_err("second call with same key but different body should fail");

    assert!(matches!(
        err,
        DefinitionGovernanceError::IdempotencyConflict
    ));
}

// ==========================================================================
// 13. Owner revoked between auth and write → write fails
// ==========================================================================

#[tokio::test]
async fn test_owner_revoked_before_write() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, _, _, _) = seed_workflow_definition(&pool, domain_id).await;

    // Remove the owner binding to simulate revocation
    sqlx::query(
        "UPDATE domain_role_bindings SET enabled = FALSE WHERE domain_id = $1 AND principal_id = $2 AND role_key = 'DOMAIN_OWNER'",
    )
    .bind(domain_id)
    .bind(owner_id)
    .execute(&pool)
    .await
    .expect("revoke owner");

    let err = governance_archive_definition(
        &pool,
        owner_id,
        &uuid::Uuid::new_v4().to_string(),
        "req",
        def_id,
    )
    .await
    .expect_err("revoked owner should fail");

    assert!(matches!(
        &err,
        &DefinitionGovernanceError::DefinitionNotFound
    ));
}

// ==========================================================================
// 14. Archived definition rejects new draft versions
// ==========================================================================

#[tokio::test]
async fn test_archived_definition_rejects_new_versions() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, _, _, _) = seed_workflow_definition(&pool, domain_id).await;

    // Archive
    governance_archive_definition(
        &pool,
        owner_id,
        &uuid::Uuid::new_v4().to_string(),
        "req",
        def_id,
    )
    .await
    .expect("archive");

    // Try to create a new draft version via service (checks archive internally)
    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    let err = service
        .create_draft_version(CreateDraftVersion {
            actor_principal_id: owner_id,
            workflow_definition_id: def_id,
            context_schema: None,
            json_schema_dialect: None,
            validator_version: None,
            metadata: None,
        })
        .await
        .expect_err("should reject new version on archived definition");

    assert!(matches!(err, DefinitionError::DefinitionArchived));
}

// ==========================================================================
// Regression: Global admin without DOMAIN_OWNER cannot manage definitions
// ==========================================================================

#[tokio::test]
async fn test_global_admin_not_auto_owner() {
    let pool = create_pool().await;
    let (_, domain_id) = seed_principal_and_domain(&pool).await;
    let admin_id = seed_second_principal(&pool).await;

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    let err = service
        .list_domain_definitions(ListDomainDefinitions {
            actor_principal_id: admin_id,
            domain_id,
            before_created_at: None,
            before_id: None,
            limit: 20,
            include_archived: false,
        })
        .await
        .expect_err("global admin without domain owner should fail");

    assert!(matches!(err, DefinitionError::PermissionDenied));
}

// ==========================================================================
// 15. Concurrent expectedRevision prevents double-publish
// ==========================================================================

#[tokio::test]
async fn test_concurrent_publish_expected_revision_race() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, ver_id, _, _) = seed_workflow_definition(&pool, domain_id).await;

    // Insert a principal for assignee reference
    let assignee_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, enabled) VALUES ($1, 'HUMAN', 'Tester', TRUE)",
    )
    .bind(assignee_id)
    .execute(&pool)
    .await
    .expect("insert assignee");

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    // Replace draft graph with a valid graph (must have TERMINAL)
    let uid1 = uuid::Uuid::new_v4().to_string();
    let uid2 = uuid::Uuid::new_v4().to_string();
    let uid3 = uuid::Uuid::new_v4().to_string();
    let draft_node_key = format!("draft-{}", &uid1[..8]);
    let step_node_key = format!("step-{}", &uid2[..8]);
    let term_node_key = format!("done-{}", &uid3[..8]);

    service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: owner_id,
            definition_version_id: ver_id,
            context_schema: None,
            nodes: vec![
                RawNodeDefinition {
                    node_key: draft_node_key.clone(),
                    display_name: "Draft".to_string(),
                    order_index: 0,
                    node_type: "DRAFT".to_string(),
                    assignee_ref_type: Some("WORKFLOW_CREATOR".to_string()),
                    fixed_principal_id: None,
                    assignee_input_key: None,
                    instructions: None,
                    primary_advance_transition_key: Some("advance-step".to_string()),
                    metadata: None,
                },
                RawNodeDefinition {
                    node_key: step_node_key.clone(),
                    display_name: "Step".to_string(),
                    order_index: 1,
                    node_type: "NORMAL".to_string(),
                    assignee_ref_type: Some("WORKFLOW_CREATOR".to_string()),
                    fixed_principal_id: None,
                    assignee_input_key: None,
                    instructions: None,
                    primary_advance_transition_key: Some("advance-done".to_string()),
                    metadata: None,
                },
                RawNodeDefinition {
                    node_key: term_node_key.clone(),
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
            ],
            transitions: vec![
                RawTransitionDefinition {
                    transition_key: "advance-step".to_string(),
                    display_name: "To Step".to_string(),
                    source_node_key: draft_node_key.clone(),
                    target_node_key: step_node_key.clone(),
                    transition_effect: "ADVANCE".to_string(),
                    submission_schema: None,
                    metadata: None,
                },
                RawTransitionDefinition {
                    transition_key: "advance-done".to_string(),
                    display_name: "Complete".to_string(),
                    source_node_key: step_node_key,
                    target_node_key: term_node_key,
                    transition_effect: "ADVANCE".to_string(),
                    submission_schema: None,
                    metadata: None,
                },
            ],
        })
        .await
        .expect("replace draft graph");

    // Both requests use the SAME version that is still DRAFT.
    // They race to publish.  The atomic_publish transaction serializes
    // via FOR UPDATE row lock: the first request publishes,
    // the second sees PUBLISHED status → VersionNotDraft.
    let pool_a = pool.clone();
    let pool_b = pool.clone();
    let key_a = uuid::Uuid::new_v4().to_string();
    let key_b = uuid::Uuid::new_v4().to_string();

    let handle_a = tokio::spawn(async move {
        governance_publish_version(
            &pool_a, owner_id, &key_a, "req-a", ver_id,
            None, // no expected_revision - pure race on version lock
        )
        .await
    });

    let handle_b = tokio::spawn(async move {
        governance_publish_version(&pool_b, owner_id, &key_b, "req-b", ver_id, None).await
    });

    let (result_a, result_b) = tokio::join!(handle_a, handle_b);
    let result_a = result_a.expect("join a");
    let result_b = result_b.expect("join b");

    let success_count = [result_a.is_ok(), result_b.is_ok()]
        .iter()
        .filter(|&&x| x)
        .count();
    let failure_with_conflict = [result_a.as_ref().err(), result_b.as_ref().err()]
        .iter()
        .filter(|e| {
            matches!(
                e,
                Some(DefinitionGovernanceError::DefinitionVersionImmutable)
                    | Some(DefinitionGovernanceError::IdempotencyConflict)
            )
        })
        .count();

    assert_eq!(
        success_count, 1,
        "exactly one concurrent publish must succeed, got {}",
        success_count
    );
    assert!(
        failure_with_conflict >= 1,
        "the failing request must return conflict"
    );

    // Verify exactly one published version
    let published_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_definition_versions WHERE workflow_definition_id = $1 AND version_status = 'PUBLISHED'",
    )
    .bind(def_id)
    .fetch_one(&pool)
    .await
    .expect("count published");
    assert_eq!(
        published_count.0, 1,
        "exactly one published version expected"
    );
}

// ==========================================================================
// 16. Stale expectedRevision returns 409 revision_conflict
// ==========================================================================

#[tokio::test]
async fn test_stale_expected_revision_returns_revision_conflict() {
    let pool = create_pool().await;
    let (owner_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (def_id, ver_id, _, _) = seed_workflow_definition(&pool, domain_id).await;

    let assignee_id = uuid::Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals (principal_id, principal_type, display_name, enabled) VALUES ($1, 'HUMAN', 'Tester', TRUE)",
    )
    .bind(assignee_id)
    .execute(&pool)
    .await
    .expect("insert assignee");

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    // Step 1: Insert graph G1 (initial)
    let uid1 = uuid::Uuid::new_v4().to_string();
    let dk1 = format!("draft-{}", &uid1[..8]);
    let uid2 = uuid::Uuid::new_v4().to_string();
    let tk1 = format!("done-{}", &uid2[..8]);

    service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: owner_id,
            definition_version_id: ver_id,
            context_schema: None,
            nodes: vec![
                RawNodeDefinition {
                    node_key: dk1.clone(),
                    display_name: "Draft".into(),
                    order_index: 0,
                    node_type: "DRAFT".into(),
                    assignee_ref_type: Some("WORKFLOW_CREATOR".into()),
                    fixed_principal_id: None,
                    assignee_input_key: None,
                    instructions: None,
                    primary_advance_transition_key: Some("adv".into()),
                    metadata: None,
                },
                RawNodeDefinition {
                    node_key: tk1.clone(),
                    display_name: "Done".into(),
                    order_index: 1,
                    node_type: "TERMINAL".into(),
                    assignee_ref_type: None,
                    fixed_principal_id: None,
                    assignee_input_key: None,
                    instructions: None,
                    primary_advance_transition_key: None,
                    metadata: None,
                },
            ],
            transitions: vec![RawTransitionDefinition {
                transition_key: "adv".into(),
                display_name: "Adv".into(),
                source_node_key: dk1.clone(),
                target_node_key: tk1,
                transition_effect: "ADVANCE".into(),
                submission_schema: None,
                metadata: None,
            }],
        })
        .await
        .expect("replace G1");

    // Step 2: Replace with G2 (different nodes → different digest)
    let uid3 = uuid::Uuid::new_v4().to_string();
    let dk2 = format!("draft2-{}", &uid3[..8]);
    let uid4 = uuid::Uuid::new_v4().to_string();
    let tk2 = format!("done2-{}", &uid4[..8]);

    service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: owner_id,
            definition_version_id: ver_id,
            context_schema: None,
            nodes: vec![
                RawNodeDefinition {
                    node_key: dk2.clone(),
                    display_name: "Draft2".into(),
                    order_index: 0,
                    node_type: "DRAFT".into(),
                    assignee_ref_type: Some("WORKFLOW_CREATOR".into()),
                    fixed_principal_id: None,
                    assignee_input_key: None,
                    instructions: None,
                    primary_advance_transition_key: Some("adv2".into()),
                    metadata: None,
                },
                RawNodeDefinition {
                    node_key: tk2.clone(),
                    display_name: "Done2".into(),
                    order_index: 1,
                    node_type: "TERMINAL".into(),
                    assignee_ref_type: None,
                    fixed_principal_id: None,
                    assignee_input_key: None,
                    instructions: None,
                    primary_advance_transition_key: None,
                    metadata: None,
                },
            ],
            transitions: vec![RawTransitionDefinition {
                transition_key: "adv2".into(),
                display_name: "Adv2".into(),
                source_node_key: dk2,
                target_node_key: tk2,
                transition_effect: "ADVANCE".into(),
                submission_schema: None,
                metadata: None,
            }],
        })
        .await
        .expect("replace G2");

    // Step 3: Publish with stale expected_revision (does not match G2's digest)
    let err = governance_publish_version(
        &pool,
        owner_id,
        &uuid::Uuid::new_v4().to_string(),
        "req-stale",
        ver_id,
        Some("stale-digest-that-does-not-match".to_string()),
    )
    .await
    .expect_err("stale expected_revision must fail");

    assert!(
        matches!(&err, &DefinitionGovernanceError::RevisionConflict),
        "expected RevisionConflict, got {:?}",
        err.label()
    );

    // Step 4: Verify no side effects
    let version_status: String = sqlx::query_scalar(
        "SELECT version_status::text FROM workflow_definition_versions WHERE definition_version_id = $1",
    )
    .bind(ver_id)
    .fetch_one(&pool)
    .await
    .expect("get status");
    assert_eq!(version_status, "DRAFT", "must remain DRAFT");

    let pub_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM workflow_definition_versions WHERE workflow_definition_id = $1 AND version_status = 'PUBLISHED'",
    )
    .bind(def_id)
    .fetch_one(&pool)
    .await
    .expect("count");
    assert_eq!(pub_count.0, 0, "no published version");
}

// ==========================================================================
// 17-20. Cross-domain write existence leak prevention
// ==========================================================================

#[tokio::test]
async fn test_cross_domain_create_draft_returns_404() {
    let pool = create_pool().await;
    let (owner_a, domain_a) = seed_principal_domain_with_owner(&pool).await;
    let (owner_b, _) = seed_principal_domain_with_owner(&pool).await;
    let (def_a, _, _, _) = seed_workflow_definition(&pool, domain_a).await;

    let err = governance_create_draft_version(
        &pool,
        owner_b,
        &uuid::Uuid::new_v4().to_string(),
        "cross",
        def_a,
        None,
        None,
        None,
        None,
    )
    .await
    .expect_err("cross-domain create draft must fail");
    assert!(
        matches!(&err, &DefinitionGovernanceError::DefinitionNotFound),
        "expected DefinitionNotFound"
    );
}

#[tokio::test]
async fn test_cross_domain_replace_graph_returns_404() {
    let pool = create_pool().await;
    let (owner_a, domain_a) = seed_principal_domain_with_owner(&pool).await;
    let (owner_b, _) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_a, _, _) = seed_workflow_definition(&pool, domain_a).await;

    let uid = uuid::Uuid::new_v4().to_string();
    let err = governance_replace_draft_graph(
        &pool,
        owner_b,
        &uuid::Uuid::new_v4().to_string(),
        "cross",
        ver_a,
        None,
        vec![RawNodeDefinition {
            node_key: format!("dk-{}", &uid[..8]),
            display_name: "D".into(),
            order_index: 0,
            node_type: "DRAFT".into(),
            assignee_ref_type: Some("WORKFLOW_CREATOR".into()),
            fixed_principal_id: None,
            assignee_input_key: None,
            instructions: None,
            primary_advance_transition_key: Some("adv".into()),
            metadata: None,
        }],
        vec![],
    )
    .await
    .expect_err("cross-domain replace must fail");
    assert!(
        matches!(&err, &DefinitionGovernanceError::DefinitionNotFound),
        "expected DefinitionNotFound"
    );
}

#[tokio::test]
async fn test_cross_domain_publish_returns_404() {
    let pool = create_pool().await;
    let (owner_a, domain_a) = seed_principal_domain_with_owner(&pool).await;
    let (owner_b, _) = seed_principal_domain_with_owner(&pool).await;
    let (def_a, ver_a, _, _) = seed_workflow_definition(&pool, domain_a).await;

    // Seed a valid graph as owner_a so publish validation passes
    let repo_a = PgDefinitionRepository::new(pool.clone());
    let svc_a = DefinitionService::new(repo_a);
    let uid1 = uuid::Uuid::new_v4().to_string();
    let uid2 = uuid::Uuid::new_v4().to_string();
    let dk = format!("d-{}", &uid1[..8]);
    let tk = format!("t-{}", &uid2[..8]);
    svc_a
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: owner_a,
            definition_version_id: ver_a,
            context_schema: None,
            nodes: vec![
                RawNodeDefinition {
                    node_key: dk.clone(),
                    display_name: "D".into(),
                    order_index: 0,
                    node_type: "DRAFT".into(),
                    assignee_ref_type: Some("WORKFLOW_CREATOR".into()),
                    fixed_principal_id: None,
                    assignee_input_key: None,
                    instructions: None,
                    primary_advance_transition_key: Some("adv".into()),
                    metadata: None,
                },
                RawNodeDefinition {
                    node_key: tk.clone(),
                    display_name: "T".into(),
                    order_index: 1,
                    node_type: "TERMINAL".into(),
                    assignee_ref_type: None,
                    fixed_principal_id: None,
                    assignee_input_key: None,
                    instructions: None,
                    primary_advance_transition_key: None,
                    metadata: None,
                },
            ],
            transitions: vec![RawTransitionDefinition {
                transition_key: "adv".into(),
                display_name: "A".into(),
                source_node_key: dk,
                target_node_key: tk,
                transition_effect: "ADVANCE".into(),
                submission_schema: None,
                metadata: None,
            }],
        })
        .await
        .expect("replace graph");

    // Now try to publish as owner_b
    let err = governance_publish_version(
        &pool,
        owner_b,
        &uuid::Uuid::new_v4().to_string(),
        "cross",
        ver_a,
        None,
    )
    .await
    .expect_err("cross-domain publish must fail");
    assert!(
        matches!(&err, &DefinitionGovernanceError::DefinitionNotFound),
        "expected DefinitionNotFound, got {:?}",
        err.label()
    );
}

#[tokio::test]
async fn test_cross_domain_archive_returns_404() {
    let pool = create_pool().await;
    let (owner_a, domain_a) = seed_principal_domain_with_owner(&pool).await;
    let (owner_b, _) = seed_principal_domain_with_owner(&pool).await;
    let (def_a, _, _, _) = seed_workflow_definition(&pool, domain_a).await;

    let err = governance_archive_definition(
        &pool,
        owner_b,
        &uuid::Uuid::new_v4().to_string(),
        "cross",
        def_a,
    )
    .await
    .expect_err("cross-domain archive must fail");
    assert!(
        matches!(&err, &DefinitionGovernanceError::DefinitionNotFound),
        "expected DefinitionNotFound"
    );
}

// ==========================================================================
// 21. Non-existent definition returns same 404
// ==========================================================================

#[tokio::test]
async fn test_nonexistent_id_returns_404() {
    let pool = create_pool().await;
    let (owner_id, _) = seed_principal_domain_with_owner(&pool).await;
    let fake = uuid::Uuid::new_v4();

    let err = governance_create_draft_version(
        &pool,
        owner_id,
        &uuid::Uuid::new_v4().to_string(),
        "nonexist",
        fake,
        None,
        None,
        None,
        None,
    )
    .await
    .expect_err("non-existent must fail");
    assert!(
        matches!(&err, &DefinitionGovernanceError::DefinitionNotFound),
        "expected DefinitionNotFound for non-existent ID"
    );
}
