//! Domain Owner definition governance — idempotent, audited write operations.
//!
//! Each public function is a single atomic unit:
//! - Begins a transaction
//! - Acquires a generic idempotent receipt
//! - Delegates to [`DefinitionService`] for authorization + business logic
//! - Writes a durable security audit record
//! - Completes the receipt
//! - Commits the transaction
//!
//! Handler layer never calls the receipt or audit primitives directly.
//! All authorization originates here or in [`DefinitionService`].

mod receipt;

use sqlx::PgPool;
use uuid::Uuid;

use self::receipt::{compute_receipt_hash, handle_receipt_result};
use crate::application::definition::commands::{
    ArchiveDefinition, CreateDefinition, CreateDraftVersion, PublishVersion, RawNodeDefinition,
    RawTransitionDefinition, ReplaceDraftGraph,
};
use crate::application::definition::DefinitionService;
use crate::domain::definition::error::DefinitionError;
use crate::domain::definition::model::{WorkflowDefinition, WorkflowDefinitionVersion};
use crate::store::postgres::definition_repository::PgDefinitionRepository;
use crate::store::postgres::domain_role_repository::write_security_audit;
use crate::store::postgres::provisioning_repository::{
    acquire_receipt, complete_receipt, AcquireReceipt,
};

const COMMAND_TYPE_CREATE_DEFINITION: &str = "DEFINITION_CREATE";
const COMMAND_TYPE_CREATE_DRAFT: &str = "DEFINITION_CREATE_DRAFT";
const COMMAND_TYPE_REPLACE_DRAFT: &str = "DEFINITION_REPLACE_DRAFT";
const COMMAND_TYPE_PUBLISH: &str = "DEFINITION_PUBLISH";
const COMMAND_TYPE_ARCHIVE: &str = "DEFINITION_ARCHIVE";

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur during definition governance operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DefinitionGovernanceError {
    NotDomainOwner,
    DomainDisabled,
    DefinitionNotFound,
    DefinitionArchived,
    DefinitionNotEditable,
    DefinitionKeyConflict,
    RevisionConflict,
    DirectTokenRequired,
    IdempotencyConflict,
    CommandStillProcessing,
    InternalConsistency(String),
    StorageError(String),
}

impl DefinitionGovernanceError {
    pub fn label(&self) -> &'static str {
        match self {
            Self::NotDomainOwner => "not_domain_owner",
            Self::DomainDisabled => "domain_disabled",
            Self::DefinitionNotFound => "definition_not_found",
            Self::DefinitionArchived => "definition_not_editable",
            Self::DefinitionNotEditable => "definition_not_editable",
            Self::DefinitionKeyConflict => "definition_key_conflict",
            Self::RevisionConflict => "revision_conflict",
            Self::DirectTokenRequired => "direct_token_required",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::CommandStillProcessing => "command_still_processing",
            Self::InternalConsistency(_) => "internal_consistency_error",
            Self::StorageError(_) => "service_unavailable",
        }
    }

    pub fn status_code(&self) -> u16 {
        match self {
            Self::NotDomainOwner | Self::DomainDisabled | Self::DirectTokenRequired => 403,
            Self::DefinitionNotFound => 404,
            Self::DefinitionArchived
            | Self::DefinitionNotEditable
            | Self::DefinitionKeyConflict
            | Self::RevisionConflict
            | Self::IdempotencyConflict => 409,
            Self::CommandStillProcessing => 425,
            Self::InternalConsistency(_) => 500,
            Self::StorageError(_) => 503,
        }
    }
}

impl From<DefinitionError> for DefinitionGovernanceError {
    fn from(e: DefinitionError) -> Self {
        match e {
            DefinitionError::PermissionDenied => Self::NotDomainOwner,
            DefinitionError::DomainDisabled => Self::DomainDisabled,
            DefinitionError::DefinitionNotFound => Self::DefinitionNotFound,
            DefinitionError::DefinitionVersionNotFound => Self::DefinitionNotFound,
            DefinitionError::DefinitionArchived => Self::DefinitionArchived,
            DefinitionError::VersionNotDraft => Self::DefinitionNotEditable,
            DefinitionError::DefinitionKeyConflict => Self::DefinitionKeyConflict,
            DefinitionError::ConcurrentModification(_) => Self::RevisionConflict,
            DefinitionError::InvalidLifecycleTransition => Self::DefinitionNotEditable,
            DefinitionError::PrincipalNotFound
            | DefinitionError::PrincipalDisabled
            | DefinitionError::DomainNotFound
            | DefinitionError::FixedPrincipalInvalid(_) => Self::NotDomainOwner,
            DefinitionError::GraphValidationFailed(_)
            | DefinitionError::SchemaValidationFailed(_)
            | DefinitionError::DigestFailure(_) => {
                Self::InternalConsistency(format!("validation error: {}", e))
            }
            DefinitionError::StorageError(d) => Self::StorageError(d),
        }
    }
}

// ---------------------------------------------------------------------------
// Governance write operations
// ---------------------------------------------------------------------------

/// Create a workflow definition (idempotent).
pub async fn governance_create_definition(
    pool: &PgPool,
    actor_id: Uuid,
    idempotency_key: &str,
    request_id: &str,
    domain_id: Uuid,
    definition_key: &str,
    display_name: &str,
    description: Option<&str>,
    metadata: Option<&serde_json::Value>,
) -> Result<WorkflowDefinition, DefinitionGovernanceError> {
    let definition_key_owned = definition_key.to_string();
    let display_name_owned = display_name.to_string();
    let description_owned = description.map(|s| s.to_string());
    let metadata_owned = metadata.cloned();

    governance_with_receipt(
        pool,
        actor_id,
        idempotency_key,
        COMMAND_TYPE_CREATE_DEFINITION,
        request_id,
        |service, _cmd: serde_json::Value| async move {
            let cmd = CreateDefinition {
                actor_principal_id: actor_id,
                owner_domain_id: domain_id,
                definition_key: definition_key_owned,
                display_name: display_name_owned,
                description: description_owned,
                metadata: metadata_owned,
            };
            let result = service.create_definition(cmd).await?;
            Ok(result)
        },
        serde_json::json!({"domainId": domain_id, "definitionKey": definition_key}),
    )
    .await
}

/// Create a draft version (idempotent).
pub async fn governance_create_draft_version(
    pool: &PgPool,
    actor_id: Uuid,
    idempotency_key: &str,
    request_id: &str,
    workflow_definition_id: Uuid,
    context_schema: Option<serde_json::Value>,
    json_schema_dialect: Option<String>,
    validator_version: Option<String>,
    metadata: Option<serde_json::Value>,
) -> Result<WorkflowDefinitionVersion, DefinitionGovernanceError> {
    let def_id = workflow_definition_id;
    let cs = context_schema;
    let jsd = json_schema_dialect;
    let vv = validator_version;
    let md = metadata;

    governance_with_receipt(
        pool,
        actor_id,
        idempotency_key,
        COMMAND_TYPE_CREATE_DRAFT,
        request_id,
        |service, _cmd: serde_json::Value| async move {
            let cmd = CreateDraftVersion {
                actor_principal_id: actor_id,
                workflow_definition_id: def_id,
                context_schema: cs,
                json_schema_dialect: jsd,
                validator_version: vv,
                metadata: md,
            };
            let result = service.create_draft_version(cmd).await?;
            Ok(result)
        },
        serde_json::json!({"workflowDefinitionId": def_id}),
    )
    .await
}

/// Replace a draft graph (idempotent).
pub async fn governance_replace_draft_graph(
    pool: &PgPool,
    actor_id: Uuid,
    idempotency_key: &str,
    request_id: &str,
    definition_version_id: Uuid,
    context_schema: Option<serde_json::Value>,
    nodes: Vec<RawNodeDefinition>,
    transitions: Vec<RawTransitionDefinition>,
) -> Result<(), DefinitionGovernanceError> {
    let version_id = definition_version_id;
    let cs = context_schema;
    let nd = nodes;
    let tr = transitions;

    governance_with_receipt(
        pool,
        actor_id,
        idempotency_key,
        COMMAND_TYPE_REPLACE_DRAFT,
        request_id,
        |service, _cmd: serde_json::Value| async move {
            let cmd = ReplaceDraftGraph {
                actor_principal_id: actor_id,
                definition_version_id: version_id,
                context_schema: cs,
                nodes: nd,
                transitions: tr,
            };
            service.replace_draft_graph(cmd).await?;
            Ok(())
        },
        serde_json::json!({"definitionVersionId": version_id}),
    )
    .await
}

/// Publish a version (idempotent).
pub async fn governance_publish_version(
    pool: &PgPool,
    actor_id: Uuid,
    idempotency_key: &str,
    request_id: &str,
    version_id: Uuid,
    expected_revision: Option<String>,
) -> Result<WorkflowDefinitionVersion, DefinitionGovernanceError> {
    let vid = version_id;
    let exp_for_receipt = expected_revision.clone();

    governance_with_receipt(
        pool,
        actor_id,
        idempotency_key,
        COMMAND_TYPE_PUBLISH,
        request_id,
        |service, _cmd: serde_json::Value| async move {
            let cmd = PublishVersion {
                actor_principal_id: actor_id,
                definition_version_id: vid,
                expected_revision,
            };
            let result = service.publish_version(cmd).await?;
            Ok(result)
        },
        serde_json::json!({"definitionVersionId": vid, "expectedRevision": exp_for_receipt}),
    )
    .await
}

/// Archive a workflow definition (idempotent).
pub async fn governance_archive_definition(
    pool: &PgPool,
    actor_id: Uuid,
    idempotency_key: &str,
    request_id: &str,
    workflow_definition_id: Uuid,
) -> Result<WorkflowDefinition, DefinitionGovernanceError> {
    let def_id = workflow_definition_id;

    governance_with_receipt(
        pool,
        actor_id,
        idempotency_key,
        COMMAND_TYPE_ARCHIVE,
        request_id,
        |service, _cmd: serde_json::Value| async move {
            let cmd = ArchiveDefinition {
                actor_principal_id: actor_id,
                workflow_definition_id: def_id,
            };
            let result = service.archive_definition(cmd).await?;
            Ok(result)
        },
        serde_json::json!({"workflowDefinitionId": def_id}),
    )
    .await
}

// ---------------------------------------------------------------------------
// Generic receipt wrapper
// ---------------------------------------------------------------------------

/// Execute a definition write operation inside a managed receipt transaction.
///
/// 1. Begin transaction
/// 2. Acquire idempotent receipt
/// 3. If not owned → replay / conflict / still-processing
/// 4. Call the business logic (which does auth + domain checks + writes)
/// 5. Write security audit
/// 6. Complete receipt
/// 7. Commit
async fn governance_with_receipt<F, Fut, T>(
    pool: &PgPool,
    actor_id: Uuid,
    idempotency_key: &str,
    command_type: &str,
    _request_id: &str,
    business_logic: F,
    command: serde_json::Value,
) -> Result<T, DefinitionGovernanceError>
where
    F: FnOnce(DefinitionService<PgDefinitionRepository>, serde_json::Value) -> Fut,
    Fut: std::future::Future<Output = Result<T, DefinitionError>>,
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| DefinitionGovernanceError::StorageError(e.to_string()))?;

    let request_hash = compute_receipt_hash(
        &serde_json::json!({ "commandType": command_type, "command": serde_json::to_value(&command).unwrap_or_default() }),
    );

    let receipt: AcquireReceipt = acquire_receipt(
        &mut tx,
        actor_id,
        idempotency_key,
        command_type,
        &request_hash,
    )
    .await
    .map_err(|e| DefinitionGovernanceError::StorageError(e.to_string()))?;

    if !receipt.is_owned() {
        tx.commit()
            .await
            .map_err(|e| DefinitionGovernanceError::StorageError(e.to_string()))?;
        return handle_receipt_result(receipt);
    }

    let repo = PgDefinitionRepository::new(pool.clone());
    let service = DefinitionService::new(repo);

    let result = business_logic(service, command).await;

    match result {
        Ok(response) => {
            let audit_details = serde_json::json!({
                "operation": command_type,
                "actorPrincipalId": actor_id,
                "result": "success",
            });
            write_security_audit(
                &mut tx,
                actor_id,
                command_type,
                actor_id,
                Uuid::default(),
                &audit_details,
            )
            .await
            .map_err(|e| DefinitionGovernanceError::StorageError(e.to_string()))?;

            let response_json =
                serde_json::to_value(&response).unwrap_or(serde_json::json!({"status": "success"}));
            complete_receipt(&mut tx, receipt.command_id(), 200, &response_json)
                .await
                .map_err(|e| DefinitionGovernanceError::StorageError(e.to_string()))?;

            tx.commit()
                .await
                .map_err(|e| DefinitionGovernanceError::StorageError(e.to_string()))?;

            Ok(response)
        }
        Err(def_err) => {
            let gov_err: DefinitionGovernanceError = def_err.into();
            let error_response = serde_json::json!({"error": gov_err.label()});
            let status = gov_err.status_code() as i32;

            if !matches!(
                gov_err,
                DefinitionGovernanceError::StorageError(_)
                    | DefinitionGovernanceError::InternalConsistency(_)
            ) {
                complete_receipt(&mut tx, receipt.command_id(), status, &error_response)
                    .await
                    .map_err(|e| DefinitionGovernanceError::StorageError(e.to_string()))?;
            }

            tx.commit()
                .await
                .map_err(|e| DefinitionGovernanceError::StorageError(e.to_string()))?;

            Err(gov_err)
        }
    }
}
