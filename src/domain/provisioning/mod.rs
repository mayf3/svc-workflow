//! Provisioning commands, error types, and command type constants.
//!
//! These types define the identity provisioning API's command structure
//! and error semantics. They follow the same pattern as admin recovery
//! commands in `src/domain/workflow_instance/recovery.rs`.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::ids::{DomainId, PrincipalId};

// ---------------------------------------------------------------------------
// Command type constants
// ---------------------------------------------------------------------------

/// Command type for upserting a principal.
pub const COMMAND_TYPE_PROVISION_PRINCIPAL: &str = "PROVISION_PRINCIPAL";
/// Command type for upserting a domain.
pub const COMMAND_TYPE_PROVISION_DOMAIN: &str = "PROVISION_DOMAIN";
/// Command type for upserting a role binding.
pub const COMMAND_TYPE_PROVISION_ROLE_BINDING: &str = "PROVISION_ROLE_BINDING";
/// Command type for revoking a role binding.
pub const COMMAND_TYPE_REVOKE_ROLE_BINDING: &str = "REVOKE_ROLE_BINDING";
/// Command type for atomic owner replacement.
pub const COMMAND_TYPE_REPLACE_OWNER: &str = "PROVISION_REPLACE_OWNER";
/// Command type for upserting a global (domain-independent) role binding.
pub const COMMAND_TYPE_PROVISION_GLOBAL_ROLE_BINDING: &str = "PROVISION_GLOBAL_ROLE_BINDING";
/// Command type for revoking a global (domain-independent) role binding.
pub const COMMAND_TYPE_REVOKE_GLOBAL_ROLE_BINDING: &str = "REVOKE_GLOBAL_ROLE_BINDING";
/// Command schema version.
pub const PROVISIONING_SCHEMA_VERSION: &str = "v1";

/// The formal cross-domain read-only workflow role.
///
/// Holders may read workflow instance summaries across all domains via
/// `GET /internal/v1/workflow-instances/global`. It grants no write
/// powers: transitions stay assignee-gated, cancel/archive stay
/// DOMAIN_OWNER-gated, provisioning stays admin-gated.
pub const GLOBAL_WORKFLOW_COORDINATOR_ROLE: &str = "GLOBAL_WORKFLOW_COORDINATOR";

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

/// Upsert a principal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionPrincipalCommand {
    pub principal_id: PrincipalId,
    pub principal_type: String,
    pub enabled: bool,
    pub source: String,
    pub source_revision: Option<String>,
}

/// Upsert a domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionDomainCommand {
    pub domain_id: DomainId,
    pub domain_key: String,
    pub display_name: Option<String>,
    pub enabled: bool,
}

/// Create or enable a role binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionRoleBindingCommand {
    pub domain_id: DomainId,
    pub principal_id: PrincipalId,
    pub role_key: String,
    pub enabled: bool,
}

/// Revoke a role binding (set enabled=false).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeRoleBindingCommand {
    pub domain_id: DomainId,
    pub principal_id: PrincipalId,
    pub role_key: String,
}

/// Upsert a global (domain-independent) role binding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisionGlobalRoleBindingCommand {
    pub principal_id: PrincipalId,
    pub role_key: String,
    pub enabled: bool,
}

/// Revoke a global role binding (set enabled=false).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevokeGlobalRoleBindingCommand {
    pub principal_id: PrincipalId,
    pub role_key: String,
}

/// Atomically replace the domain owner.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplaceOwnerCommand {
    pub domain_id: DomainId,
    pub new_owner_id: PrincipalId,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors that can occur during provisioning operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvisioningError {
    // Identity failures
    PrincipalNotFound,
    PrincipalDisabled,
    PrincipalTypeConflict,
    PrincipalTypeInvalid,
    DomainNotFound,
    DomainDisabled,
    DomainIdentityConflict,
    DomainOwnerConflict,

    // Role binding failures
    BindingAlreadyExists,
    BindingNotFound,
    RoleKeyInvalid,

    // Definition version failures
    DefinitionVersionNotFound,

    // Authorization
    PermissionDenied,
    PrincipalTypeNotAllowed,
    InvalidInput(String),

    // Idempotency
    IdempotencyConflict,
    CommandStillProcessing,

    // Infrastructure
    InternalConsistency(String),
    StorageError(String),
}

impl ProvisioningError {
    /// Stable HTTP-compatible error label.
    pub fn label(&self) -> &'static str {
        use ProvisioningError::*;
        match self {
            PrincipalNotFound => "principal_not_found",
            PrincipalDisabled => "principal_disabled",
            PrincipalTypeConflict => "principal_type_conflict",
            PrincipalTypeInvalid => "principal_type_invalid",
            DomainNotFound => "domain_not_found",
            DomainDisabled => "domain_disabled",
            DomainIdentityConflict => "domain_identity_conflict",
            DomainOwnerConflict => "domain_owner_conflict",
            BindingAlreadyExists => "binding_already_exists",
            BindingNotFound => "binding_not_found",
            RoleKeyInvalid => "role_key_invalid",
            DefinitionVersionNotFound => "definition_version_not_found",
            PermissionDenied => "permission_denied",
            PrincipalTypeNotAllowed => "principal_type_not_allowed",
            InvalidInput(_) => "invalid_input",
            IdempotencyConflict => "idempotency_conflict",
            CommandStillProcessing => "command_still_processing",
            InternalConsistency(_) => "internal_consistency_error",
            StorageError(_) => "storage_error",
        }
    }

    /// Human-readable detail for error responses (no secrets).
    pub fn detail(&self) -> Option<&str> {
        use ProvisioningError::*;
        match self {
            InvalidInput(d) => Some(d),
            InternalConsistency(_) => Some("internal consistency error"),
            StorageError(_) => Some("storage error"),
            _ => None,
        }
    }

    /// HTTP status code.
    pub fn status_code(&self) -> u16 {
        use ProvisioningError::*;
        match self {
            PrincipalNotFound | DomainNotFound | DefinitionVersionNotFound => 404,
            PrincipalDisabled | DomainDisabled | PermissionDenied | PrincipalTypeNotAllowed => 403,
            PrincipalTypeConflict
            | DomainIdentityConflict
            | DomainOwnerConflict
            | BindingAlreadyExists
            | IdempotencyConflict => 409,
            PrincipalTypeInvalid | RoleKeyInvalid | InvalidInput(_) => 422,
            CommandStillProcessing => 425,
            InternalConsistency(_) => 500,
            StorageError(_) => 503,
            BindingNotFound => 404,
        }
    }
}

impl std::fmt::Display for ProvisioningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl std::error::Error for ProvisioningError {}

/// Owner replacement result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerReplacementResult {
    pub previous_owner_id: Option<PrincipalId>,
    pub new_owner_id: PrincipalId,
}
