//! Lifecycle status-change operations.
//!
//! Handles DeprecateVersion and RevokeVersion, delegating to the
//! repository's atomic_deprecate / atomic_revoke (B-1).

use crate::domain::definition::error::DefinitionError;

use super::super::commands::{DeprecateVersion, RevokeVersion};
use super::super::repository::DefinitionRepository;
use super::super::service::DefinitionService;

/// Maximum recorded length for a deprecation reason (mirrors the
/// `deprecation_reason` CHECK constraint added by migration 0024).
const MAX_DEPRECATION_REASON_LEN: usize = 256;

/// Reason token for a source-only stop-new deprecation of a held version
/// (CTR-CIR-003, SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2, final
/// paragraph): for exactly the two versions of parent CTR-WACI-010 the
/// reviewed plan may deprecate without publishing a replacement, "atomically
/// recording SOURCE_IDENTITY_UNRESOLVED and preserving every task/history
/// fact". Written in the same transaction that flips the version to
/// DEPRECATED; new-instance creation on a DEPRECATED version is already
/// refused by the runtime's `VersionNotPublished` rule. See
/// docs/runbooks/SOURCE_ONLY_STOP_NEW_V1.md.
pub const SOURCE_IDENTITY_UNRESOLVED: &str = "SOURCE_IDENTITY_UNRESOLVED";

impl<R: DefinitionRepository> DefinitionService<R> {
    /// Deprecate a PUBLISHED version -> DEPRECATED.
    ///
    /// B-1: Uses atomic_deprecate which locks the version row across
    /// all checks and writes in a single transaction. The optional
    /// `deprecation_reason` is recorded in that same transaction
    /// (migration 0024); `None` writes NULL, preserving the prior behavior.
    pub async fn deprecate_version(
        &self,
        cmd: DeprecateVersion,
    ) -> Result<crate::domain::definition::model::WorkflowDefinitionVersion, DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        if let Some(reason) = cmd.deprecation_reason.as_deref() {
            if reason.is_empty() || reason.len() > MAX_DEPRECATION_REASON_LEN {
                return Err(DefinitionError::SchemaValidationFailed(format!(
                    "deprecation_reason must be 1-{MAX_DEPRECATION_REASON_LEN} characters"
                )));
            }
        }

        let updated = self
            .repo
            .atomic_deprecate(
                cmd.definition_version_id,
                cmd.actor_principal_id,
                cmd.deprecation_reason.as_deref(),
            )
            .await?;

        Ok(updated)
    }

    /// Revoke a PUBLISHED or DEPRECATED version -> REVOKED.
    ///
    /// B-1: Uses atomic_revoke which locks the version row across
    /// all checks and writes in a single transaction.
    pub async fn revoke_version(
        &self,
        cmd: RevokeVersion,
    ) -> Result<crate::domain::definition::model::WorkflowDefinitionVersion, DefinitionError> {
        self.ensure_principal_enabled(cmd.actor_principal_id)
            .await?;

        let updated = self
            .repo
            .atomic_revoke(cmd.definition_version_id, cmd.actor_principal_id)
            .await?;

        Ok(updated)
    }
}
