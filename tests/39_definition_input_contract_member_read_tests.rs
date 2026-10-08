//! Integration tests for SVC_WORKFLOW_DEFINITION_INPUT_CONTRACT_MEMBER_READ_V1.
//!
//! Requires a running PostgreSQL with the `svc_workflow` database (same as the
//! other suites).
//!
//! Matrix (spec §5):
//! - P1 member + active binding + PUBLISHED version reads the input contract
//! - P2 characterization: the four H-5 reads stay DOMAIN_OWNER-only for members
//! - P3 owner: new surface allowed, H-5 reads unchanged
//! - P4 non-member: existing vs unknown version both land in the opaque-404
//!   denial set (handler mapping equality is asserted in the handler unit test)
//! - P5 revoked binding (enabled = FALSE) denied
//! - P6 cross-domain member denied
//! - P7 member + DRAFT version -> VersionNotPublished, no schema bytes
//! - P8 member + DEPRECATED version -> VersionNotPublished
//! - P9 disabled principal denied
//! - P10 disabled domain -> DomainDisabled (mirrors create)
//! - P11 wire shape: exactly the five contract fields, nothing else

mod common;

use common::{
    create_pool, seed_domain_member, seed_domain_owner, seed_principal_and_domain,
    seed_second_principal, set_domain_enabled, set_principal_enabled,
};

use svc_workflow::application::definition::commands::{
    CreateDefinition, CreateDraftVersion, DeprecateVersion, PublishVersion, RawNodeDefinition,
    RawTransitionDefinition, ReplaceDraftGraph,
};
use svc_workflow::application::definition::queries::{
    GetDefinition, GetDefinitionVersion, GetPublishedVersionInputContract,
};
use svc_workflow::application::definition::DefinitionService;
use svc_workflow::domain::definition::error::DefinitionError;
use svc_workflow::domain::enums::DefinitionVersionStatus;
use svc_workflow::store::postgres::admission_gate::AdmissionGate;
use svc_workflow::store::postgres::definition_repository::PgDefinitionRepository;

fn input_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object",
        "required": ["title"],
        "properties": { "title": { "type": "string" } }
    })
}

struct PublishedVersion {
    domain_id: uuid::Uuid,
    definition_id: uuid::Uuid,
    version_id: uuid::Uuid,
    owner_id: uuid::Uuid,
}

/// Owner + domain + definition + one PUBLISHED version carrying INPUT_SCHEMA.
async fn seed_published_version(pool: &sqlx::PgPool) -> PublishedVersion {
    let (owner_id, domain_id) = seed_principal_and_domain(pool).await;
    seed_domain_owner(pool, domain_id, owner_id).await;

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    let def = service
        .create_definition(CreateDefinition {
            actor_principal_id: owner_id,
            owner_domain_id: domain_id,
            definition_key: format!("input-contract-{}", &uuid::Uuid::new_v4().to_string()[..8]),
            display_name: "Input Contract Fixture".to_string(),
            description: None,
            metadata: None,
        })
        .await
        .expect("seed: create definition");

    let version = service
        .create_draft_version(CreateDraftVersion {
            actor_principal_id: owner_id,
            workflow_definition_id: def.id.into_uuid(),
            context_schema: Some(input_schema()),
            json_schema_dialect: None,
            validator_version: None,
            metadata: None,
            semantic_model_version: 1,
        })
        .await
        .expect("seed: create draft version");

    let version_id = version.id.into_uuid();

    let (nodes, transitions) = minimal_graph();
    service
        .replace_draft_graph(ReplaceDraftGraph {
            actor_principal_id: owner_id,
            definition_version_id: version_id,
            context_schema: None,
            nodes,
            transitions,
        })
        .await
        .expect("seed: replace draft graph");

    service
        .publish_version(
            PublishVersion {
                actor_principal_id: owner_id,
                definition_version_id: version_id,
                expected_revision: None,
            },
            // Dormant admission mode preserves pre-admission publish behavior;
            // the seed fixture carries no identity literals to admit.
            AdmissionGate::disabled(),
        )
        .await
        .expect("seed: publish");

    PublishedVersion {
        domain_id,
        definition_id: def.id.into_uuid(),
        version_id,
        owner_id,
    }
}

/// Minimal legacy graph: DRAFT -> NORMAL -> TERMINAL with WORKFLOW_CREATOR
/// assignees (same shape as the governance suite's fixture).
fn minimal_graph() -> (Vec<RawNodeDefinition>, Vec<RawTransitionDefinition>) {
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
            source_node_key: normal_node_key.clone(),
            target_node_key: term_node_key,
            transition_effect: "ADVANCE".to_string(),
            submission_schema: None,
            metadata: None,
        },
    ];
    (nodes, transitions)
}

async fn member_read(
    pool: &sqlx::PgPool,
    member_id: uuid::Uuid,
    version_id: uuid::Uuid,
) -> Result<
    svc_workflow::application::definition::queries::PublishedVersionInputContract,
    DefinitionError,
> {
    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);
    service
        .get_published_version_input_contract(GetPublishedVersionInputContract {
            actor_principal_id: member_id,
            definition_version_id: version_id,
        })
        .await
}

// ---------------------------------------------------------------------------
// P1 — the fix itself
// ---------------------------------------------------------------------------

#[tokio::test]
async fn p1_member_with_active_binding_reads_published_input_contract() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member_id = seed_domain_member(&pool, seeded.domain_id, true).await;

    let contract = member_read(&pool, member_id, seeded.version_id)
        .await
        .expect("member must read the published input contract");

    assert_eq!(contract.definition_version_id, seeded.version_id);
    assert_eq!(contract.definition_id, seeded.definition_id);
    assert_eq!(contract.version_status, DefinitionVersionStatus::PUBLISHED);
    assert_eq!(contract.context_schema, Some(input_schema()));
}

// ---------------------------------------------------------------------------
// P2/P3 — H-5 unchanged
// ---------------------------------------------------------------------------

#[tokio::test]
async fn p2_member_still_denied_on_h5_owner_reads() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member_id = seed_domain_member(&pool, seeded.domain_id, true).await;

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    let err = service
        .get_definition(GetDefinition {
            actor_principal_id: member_id,
            workflow_definition_id: seeded.definition_id,
        })
        .await
        .expect_err("H-5 must still deny member get_definition");
    assert!(matches!(err, DefinitionError::PermissionDenied));

    let err = service
        .get_definition_version(GetDefinitionVersion {
            actor_principal_id: member_id,
            definition_version_id: seeded.version_id,
        })
        .await
        .expect_err("H-5 must still deny member get_definition_version");
    assert!(matches!(err, DefinitionError::PermissionDenied));
}

#[tokio::test]
async fn p3_owner_reads_new_surface_and_h5_reads_still_work() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;

    let contract = member_read(&pool, seeded.owner_id, seeded.version_id)
        .await
        .expect("owner may read the input contract via the new surface");
    assert_eq!(contract.version_status, DefinitionVersionStatus::PUBLISHED);

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);
    service
        .get_definition_version(GetDefinitionVersion {
            actor_principal_id: seeded.owner_id,
            definition_version_id: seeded.version_id,
        })
        .await
        .expect("owner H-5 read unchanged");
}

// ---------------------------------------------------------------------------
// P4-P6 — denials that must stay opaque
// ---------------------------------------------------------------------------

#[tokio::test]
async fn p4_non_member_denied_on_existing_and_unknown_version() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let stranger_id = seed_second_principal(&pool).await;

    let err = member_read(&pool, stranger_id, seeded.version_id)
        .await
        .expect_err("non-member must be denied on an existing version");
    assert!(matches!(err, DefinitionError::PermissionDenied));

    let err = member_read(&pool, stranger_id, uuid::Uuid::new_v4())
        .await
        .expect_err("non-member must be denied on an unknown version");
    assert!(matches!(err, DefinitionError::DefinitionVersionNotFound));
}

#[tokio::test]
async fn p5_revoked_membership_binding_denies_read() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member_id = seed_domain_member(&pool, seeded.domain_id, false).await;

    let err = member_read(&pool, member_id, seeded.version_id)
        .await
        .expect_err("disabled binding must be denied");
    assert!(matches!(err, DefinitionError::PermissionDenied));
}

#[tokio::test]
async fn p6_cross_domain_member_denied() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;

    let (other_owner, other_domain) = seed_principal_and_domain(&pool).await;
    seed_domain_owner(&pool, other_domain, other_owner).await;
    let other_member = seed_domain_member(&pool, other_domain, true).await;

    let err = member_read(&pool, other_member, seeded.version_id)
        .await
        .expect_err("cross-domain member must be denied");
    assert!(matches!(err, DefinitionError::PermissionDenied));
}

// ---------------------------------------------------------------------------
// P7/P8 — non-PUBLISHED versions never expose schema bytes
// ---------------------------------------------------------------------------

#[tokio::test]
async fn p7_draft_version_denied_for_member() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member_id = seed_domain_member(&pool, seeded.domain_id, true).await;

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);
    let draft = service
        .create_draft_version(CreateDraftVersion {
            actor_principal_id: seeded.owner_id,
            workflow_definition_id: seeded.definition_id,
            context_schema: Some(serde_json::json!({ "type": "object", "x-secret": "draft-only" })),
            json_schema_dialect: None,
            validator_version: None,
            metadata: None,
            semantic_model_version: 1,
        })
        .await
        .expect("seed: second draft version");
    let draft_id = draft.id.into_uuid();

    let err = member_read(&pool, member_id, draft_id)
        .await
        .expect_err("DRAFT version must not be readable");
    assert!(matches!(err, DefinitionError::VersionNotPublished));
}

#[tokio::test]
async fn p8_deprecated_version_denied_for_member() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member_id = seed_domain_member(&pool, seeded.domain_id, true).await;

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);
    service
        .deprecate_version(DeprecateVersion {
            actor_principal_id: seeded.owner_id,
            definition_version_id: seeded.version_id,
            deprecation_reason: None,
        })
        .await
        .expect("seed: deprecate");

    let err = member_read(&pool, member_id, seeded.version_id)
        .await
        .expect_err("DEPRECATED version must not be readable");
    assert!(matches!(err, DefinitionError::VersionNotPublished));
}

// ---------------------------------------------------------------------------
// P9/P10 — disabled principal / disabled domain
// ---------------------------------------------------------------------------

#[tokio::test]
async fn p9_disabled_principal_denied() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member_id = seed_domain_member(&pool, seeded.domain_id, true).await;
    set_principal_enabled(&pool, member_id, false).await;

    let err = member_read(&pool, member_id, seeded.version_id)
        .await
        .expect_err("disabled principal must be denied");
    assert!(matches!(err, DefinitionError::PrincipalDisabled));
}

#[tokio::test]
async fn p10_disabled_domain_denied_like_create() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member_id = seed_domain_member(&pool, seeded.domain_id, true).await;
    set_domain_enabled(&pool, seeded.domain_id, false).await;

    let err = member_read(&pool, member_id, seeded.version_id)
        .await
        .expect_err("disabled domain must be denied");
    assert!(matches!(err, DefinitionError::DomainDisabled));
}

// ---------------------------------------------------------------------------
// P11 — wire shape: exactly the contract fields
// ---------------------------------------------------------------------------

#[tokio::test]
async fn p11_serialized_contract_carries_no_management_data() {
    let pool = create_pool().await;
    let seeded = seed_published_version(&pool).await;
    let member_id = seed_domain_member(&pool, seeded.domain_id, true).await;

    let contract = member_read(&pool, member_id, seeded.version_id)
        .await
        .expect("member read");

    let wire = serde_json::to_value(&contract).expect("serialize");
    let obj = wire.as_object().expect("json object");
    let mut keys: Vec<&str> = obj.keys().map(|k| k.as_str()).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        vec![
            "contextSchema",
            "definitionId",
            "definitionVersionId",
            "versionNumber",
            "versionStatus",
        ]
    );
    assert_eq!(obj["contextSchema"], input_schema());
    assert!(obj.get("nodes").is_none());
    assert!(obj.get("transitions").is_none());
    assert!(obj.get("instructions").is_none());
    assert!(obj.get("assignee").is_none());
    assert!(obj.get("submissionSchema").is_none());
}
