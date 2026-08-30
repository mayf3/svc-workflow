//! Validation helpers for the atomic workflow transition transaction.
//!
//! Provides lock_instance, validate_principal, validate_definition_version,
//! read_transition, read_source_node, read_target_node, resolve_assignee,
//! and submission validation for ADVANCE / RETURN / TERMINATE transitions.

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::enums::{AssigneeRefType, DefinitionVersionStatus, NodeType};
use crate::domain::workflow_instance::errors::ExecuteWorkflowTransitionError;

use super::row_types::*;
use super::transition_receipt::complete_transition_receipt;
use super::transition_rows::*;

/// Lock and read the workflow instance for transition.
pub(super) async fn lock_instance(
    tx: &mut Transaction<'_, Postgres>,
    instance_uuid: Uuid,
) -> Result<InstanceLockRow, ExecuteWorkflowTransitionError> {
    let instance: Option<InstanceLockRow> = sqlx::query_as(
        "SELECT workflow_instance_id, created_by_principal_id, \
         definition_version_id, current_context_revision_id, \
         current_node_visit_id, workflow_state_version, cancelled \
         FROM workflow_instances WHERE workflow_instance_id = $1 FOR UPDATE",
    )
    .bind(instance_uuid)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    instance.ok_or(ExecuteWorkflowTransitionError::InstanceNotFound)
}

/// Validate principal exists and is enabled inside the transaction.
pub(super) async fn validate_principal_enabled(
    tx: &mut Transaction<'_, Postgres>,
    principal_uuid: Uuid,
) -> Result<Option<ExecuteWorkflowTransitionError>, ExecuteWorkflowTransitionError> {
    let principal: Option<(bool,)> =
        sqlx::query_as("SELECT enabled FROM principals WHERE principal_id = $1")
            .bind(principal_uuid)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    match principal {
        None => Ok(Some(ExecuteWorkflowTransitionError::PrincipalNotFound)),
        Some((enabled,)) if !enabled => Ok(Some(ExecuteWorkflowTransitionError::PrincipalDisabled)),
        _ => Ok(None),
    }
}

/// Validate definition version status for transition.
/// PUBLISHED and DEPRECATED are allowed; REVOKED and DRAFT are blocked.
pub(super) async fn validate_definition_version_status(
    tx: &mut Transaction<'_, Postgres>,
    definition_version_id: Uuid,
) -> Result<DefinitionVersionStatus, ExecuteWorkflowTransitionError> {
    let status: Option<(String,)> = sqlx::query_as(
        "SELECT version_status::TEXT FROM workflow_definition_versions \
         WHERE definition_version_id = $1 FOR UPDATE",
    )
    .bind(definition_version_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    match status {
        None => Err(ExecuteWorkflowTransitionError::InternalConsistency(
            "definition version not found for instance".to_string(),
        )),
        Some((s,)) if s == "REVOKED" => {
            Err(ExecuteWorkflowTransitionError::DefinitionVersionRevoked)
        }
        Some((s,)) if s == "DRAFT" => Err(ExecuteWorkflowTransitionError::DefinitionVersionDraft),
        Some((s,)) => {
            // Parse the status for the caller
            let parsed = s
                .parse::<DefinitionVersionStatus>()
                .unwrap_or(DefinitionVersionStatus::PUBLISHED);
            Ok(parsed)
        }
    }
}

/// Read the current node visit with node definition details.
/// Read the semantic model version of an instance's definition version.
///
/// 1 = Legacy, 2 = Minimal. Source of truth for runtime dispatch; never
/// inferred from node shapes.
pub(super) async fn read_semantic_model_version(
    tx: &mut Transaction<'_, Postgres>,
    definition_version_id: Uuid,
) -> Result<i16, ExecuteWorkflowTransitionError> {
    let value: Option<(i16,)> = sqlx::query_as(
        "SELECT semantic_model_version FROM workflow_definition_versions \
         WHERE definition_version_id = $1",
    )
    .bind(definition_version_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    match value {
        Some((v,)) => Ok(v),
        None => Err(ExecuteWorkflowTransitionError::InternalConsistency(
            "definition version not found for instance".to_string(),
        )),
    }
}

pub(super) async fn read_current_visit(
    tx: &mut Transaction<'_, Postgres>,
    instance_uuid: Uuid,
    current_node_visit_id: Uuid,
) -> Result<CurrentVisitFullRow, ExecuteWorkflowTransitionError> {
    let visit: Option<CurrentVisitFullRow> = sqlx::query_as(
        "SELECT nv.node_visit_id, nv.node_id, nv.assignee_principal_id, \
                nd.node_type::TEXT AS node_type, \
                nd.primary_advance_transition_id, nd.order_index \
         FROM workflow_node_visits nv \
         JOIN workflow_node_definitions nd ON nd.node_id = nv.node_id \
         WHERE nv.node_visit_id = $1 AND nv.workflow_instance_id = $2",
    )
    .bind(current_node_visit_id)
    .bind(instance_uuid)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    visit.ok_or(ExecuteWorkflowTransitionError::CurrentVisitNotFound)
}

/// Read the source node definition (from current visit's node_id).
pub(super) async fn read_source_node(
    tx: &mut Transaction<'_, Postgres>,
    node_id: Uuid,
    definition_version_id: Uuid,
) -> Result<SourceNodeRow, ExecuteWorkflowTransitionError> {
    let node: Option<SourceNodeRow> = sqlx::query_as(
        "SELECT node_id, node_type::TEXT AS node_type, \
                primary_advance_transition_id, order_index \
         FROM workflow_node_definitions \
         WHERE node_id = $1 AND definition_version_id = $2",
    )
    .bind(node_id)
    .bind(definition_version_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    node.ok_or(ExecuteWorkflowTransitionError::InternalConsistency(
        "source node not found in instance definition version".to_string(),
    ))
}

/// Read a transition definition and validate it exists.
pub(super) async fn read_transition(
    tx: &mut Transaction<'_, Postgres>,
    transition_id: Uuid,
    definition_version_id: Uuid,
) -> Result<TransitionDefinitionRow, ExecuteWorkflowTransitionError> {
    let trans: Option<TransitionDefinitionRow> = sqlx::query_as(
        "SELECT transition_id, transition_key, definition_version_id, \
                source_node_id, target_node_id, transition_effect::TEXT AS transition_effect, \
                submission_schema \
         FROM workflow_transition_definitions \
         WHERE transition_id = $1 AND definition_version_id = $2",
    )
    .bind(transition_id)
    .bind(definition_version_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    trans.ok_or(ExecuteWorkflowTransitionError::TransitionNotApplicable(
        "transition definition not found for this version".to_string(),
    ))
}

/// Read the target node definition.
pub(super) async fn read_target_node(
    tx: &mut Transaction<'_, Postgres>,
    node_id: Uuid,
    definition_version_id: Uuid,
) -> Result<TargetNodeRow, ExecuteWorkflowTransitionError> {
    let node: Option<TargetNodeRow> = sqlx::query_as(
        "SELECT node_id, node_type::TEXT AS node_type, \
                assignee_ref_type::TEXT AS assignee_ref_type, \
                fixed_principal_id, assignee_input_key, order_index \
         FROM workflow_node_definitions \
         WHERE node_id = $1 AND definition_version_id = $2",
    )
    .bind(node_id)
    .bind(definition_version_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    node.ok_or(ExecuteWorkflowTransitionError::InternalConsistency(
        "target node not found in definition version".to_string(),
    ))
}

/// Resolve target assignee for the target node.
pub(super) async fn resolve_assignee(
    tx: &mut Transaction<'_, Postgres>,
    target_node: &TargetNodeRow,
    instance: &InstanceLockRow,
    domain_uuid: Uuid,
    context_payload: Option<&serde_json::Value>,
) -> Result<Option<Uuid>, ExecuteWorkflowTransitionError> {
    if target_node.node_type_enum() == NodeType::TERMINAL {
        // Published legacy Terminal definitions can still carry an obsolete
        // reference. It never grants authority and every new Terminal visit is unassigned.
        return Ok(None);
    }
    match target_node.assignee_ref_type_enum() {
        Some(AssigneeRefType::WorkflowCreator) => Ok(Some(instance.created_by_principal_id)),
        Some(AssigneeRefType::DomainOwner) => {
            let owner: Option<(Uuid, bool)> = sqlx::query_as(
                "SELECT principal_id, enabled FROM domain_role_bindings \
                 WHERE domain_id = $1 AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE \
                 LIMIT 1",
            )
            .bind(domain_uuid)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

            let (owner_id, _) = owner.ok_or_else(|| {
                ExecuteWorkflowTransitionError::AssigneeResolutionFailed(
                    "no enabled DOMAIN_OWNER found for domain".to_string(),
                )
            })?;

            verify_principal_enabled_for_transition(tx, owner_id)
                .await
                .map(Some)
        }
        Some(AssigneeRefType::FixedPrincipal) => {
            let fixed_id = target_node.fixed_principal_id.ok_or_else(|| {
                ExecuteWorkflowTransitionError::AssigneeResolutionFailed(
                    "FIXED_PRINCIPAL node has no principal_id configured".to_string(),
                )
            })?;

            verify_principal_enabled_for_transition(tx, fixed_id)
                .await
                .map(Some)
        }
        Some(AssigneeRefType::InstanceInputPrincipal) => {
            let input_key = target_node.assignee_input_key.as_deref().ok_or_else(|| {
                ExecuteWorkflowTransitionError::AssigneeResolutionFailed(
                    "INSTANCE_INPUT_PRINCIPAL node has no assignee_input_key configured"
                        .to_string(),
                )
            })?;
            let payload = context_payload.ok_or_else(|| {
                ExecuteWorkflowTransitionError::AssigneeResolutionFailed(
                    "INSTANCE_INPUT_PRINCIPAL resolution requires the instance context payload"
                        .to_string(),
                )
            })?;
            let raw = payload.get(input_key).ok_or_else(|| {
                ExecuteWorkflowTransitionError::AssigneeResolutionFailed(format!(
                    "instance input is missing required assignee key '{}'",
                    input_key
                ))
            })?;
            let s = raw.as_str().ok_or_else(|| {
                ExecuteWorkflowTransitionError::AssigneeResolutionFailed(format!(
                    "instance input '{}' must be a string UUID (stable principal identifier); \
                     display name / email / legacy id resolution is forbidden",
                    input_key
                ))
            })?;
            let candidate = Uuid::parse_str(s).map_err(|_| {
                ExecuteWorkflowTransitionError::AssigneeResolutionFailed(format!(
                    "instance input '{}' is not a valid UUID: '{}'",
                    input_key, s
                ))
            })?;
            verify_principal_enabled_for_transition(tx, candidate)
                .await
                .map(Some)
        }
        None => Err(ExecuteWorkflowTransitionError::AssigneeResolutionFailed(
            "non-terminal node has no valid assignee reference".to_string(),
        )),
    }
}

/// Verify a principal exists and is enabled, returning its UUID. Fail-closed.
async fn verify_principal_enabled_for_transition(
    tx: &mut Transaction<'_, Postgres>,
    candidate: Uuid,
) -> Result<Uuid, ExecuteWorkflowTransitionError> {
    let row: Option<(bool,)> =
        sqlx::query_as("SELECT enabled FROM principals WHERE principal_id = $1")
            .bind(candidate)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    match row {
        None => Err(ExecuteWorkflowTransitionError::AssigneeResolutionFailed(
            "resolved principal not found".to_string(),
        )),
        Some((enabled,)) if !enabled => {
            Err(ExecuteWorkflowTransitionError::AssigneeResolutionFailed(
                "resolved principal is disabled".to_string(),
            ))
        }
        _ => Ok(candidate),
    }
}

/// Validate submission payload size (≤ 1 MiB).
pub(super) fn validate_submission_size(
    payload: &serde_json::Value,
) -> Result<(), ExecuteWorkflowTransitionError> {
    let serialized = serde_json::to_vec(payload).map_err(|e| {
        ExecuteWorkflowTransitionError::SizeLimitExceeded(format!(
            "submission serialization failed: {}",
            e
        ))
    })?;

    if serialized.len() > 1048576 {
        return Err(ExecuteWorkflowTransitionError::SizeLimitExceeded(
            "submission payload exceeds 1 MiB".to_string(),
        ));
    }

    Ok(())
}

/// Validate submission payload against a JSON schema.
pub(super) fn validate_submission_schema(
    schema: &Option<serde_json::Value>,
    payload: &serde_json::Value,
) -> Result<(), ExecuteWorkflowTransitionError> {
    if let Some(schema_value) = schema {
        let validator = jsonschema::validator_for(schema_value).map_err(|e| {
            ExecuteWorkflowTransitionError::SubmissionValidationFailed(format!(
                "submission schema compilation failed: {}",
                e
            ))
        })?;

        validator.validate(payload).map_err(|e| {
            ExecuteWorkflowTransitionError::SubmissionValidationFailed(format!(
                "submission payload failed schema validation: {}",
                e
            ))
        })?;
    }

    Ok(())
}

/// Pure contract-field validation for RETURN submissions.
///
/// Returns a list of human-readable errors for missing/malformed RETURN
/// contract fields (`rootCauseNodeVisitId`, `reasonCode`, `reason`,
/// optional `relatedSubmissionIds`). Empty list means the payload carries
/// all fields in the correct shape; instance-ownership checks still run
/// against the DB in [`validate_return_references`].
///
/// Extracted as a pure function so property-based tests can cover the
/// contract without a database.
pub(crate) fn collect_return_contract_errors(
    payload: &serde_json::Value,
) -> Vec<String> {
    let mut contract_errors: Vec<String> = Vec::new();

    // rootCauseNodeVisitId: required, must be a valid UUID
    match payload
        .get("rootCauseNodeVisitId")
        .and_then(|v| v.as_str())
    {
        Some(s) => {
            if Uuid::parse_str(s).is_err() {
                contract_errors.push(format!(
                    "rootCauseNodeVisitId is not a valid UUID: '{}'",
                    s
                ));
            }
        }
        None => {
            contract_errors.push(
                "rootCauseNodeVisitId is required and must be a valid UUID".to_string(),
            );
        }
    }

    // reasonCode / reason: required
    if payload.get("reasonCode").is_none() {
        contract_errors.push("reasonCode is required for RETURN submissions".to_string());
    }
    if payload.get("reason").is_none() {
        contract_errors.push("reason is required for RETURN submissions".to_string());
    }

    // relatedSubmissionIds: optional, but if present must be an array of UUID strings
    match payload.get("relatedSubmissionIds") {
        None => {}
        Some(v) if v.is_array() => {
            for entry in v.as_array().expect("checked is_array") {
                match entry.as_str() {
                    Some(s) => {
                        if Uuid::parse_str(s).is_err() {
                            contract_errors.push(format!(
                                "relatedSubmissionIds entry is not a valid UUID: '{}'",
                                s
                            ));
                        }
                    }
                    None => contract_errors
                        .push("relatedSubmissionIds entries must be strings".to_string()),
                }
            }
        }
        Some(_) => {
            contract_errors.push(
                "relatedSubmissionIds must be an array of UUID strings when present".to_string(),
            );
        }
    }

    contract_errors
}

/// Validate RETURN submission references.
///
/// RETURN submissions require the engine-level contract fields:
/// `rootCauseNodeVisitId` (valid UUID belonging to this instance),
/// `reasonCode`, and `reason`. Missing fields are aggregated into a single
/// error so callers see the full contract in one response.
pub(super) async fn validate_return_references(
    tx: &mut Transaction<'_, Postgres>,
    payload: &serde_json::Value,
    instance_uuid: Uuid,
) -> Result<(), ExecuteWorkflowTransitionError> {
    // Aggregate every missing/invalid contract field first so the caller sees
    // the full RETURN contract in one error instead of fixing fields one at a
    // time (root cause of repeated 422 with no progress).
    let contract_errors = collect_return_contract_errors(payload);

    if !contract_errors.is_empty() {
        let detail = format!(
            "RETURN submissions require: rootCauseNodeVisitId (valid UUID), reasonCode, reason, \
             relatedSubmissionIds (optional array of UUIDs); {}",
            contract_errors.join("; ")
        );
        return Err(ExecuteWorkflowTransitionError::InvalidReturnReferences(
            detail,
        ));
    }

    // rootCauseNodeVisitId parsed successfully by collect_return_contract_errors
    let root_cause = payload
        .get("rootCauseNodeVisitId")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok())
        .expect("validated by collect_return_contract_errors");

    // Verify rootCauseNodeVisitId exists and belongs to this instance
    let root_visit: Option<(Uuid,)> = sqlx::query_as(
        "SELECT node_visit_id FROM workflow_node_visits \
         WHERE node_visit_id = $1 AND workflow_instance_id = $2",
    )
    .bind(root_cause)
    .bind(instance_uuid)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

    if root_visit.is_none() {
        return Err(ExecuteWorkflowTransitionError::InvalidReturnReferences(
            "rootCauseNodeVisitId does not exist or belongs to a different instance".to_string(),
        ));
    }

    // Verify each relatedSubmissionId exists and belongs to this instance
    if let Some(related) = payload
        .get("relatedSubmissionIds")
        .and_then(|v| v.as_array())
    {
        for entry in related {
            let sub_id = Uuid::parse_str(entry.as_str().expect("validated above"))
                .expect("validated above");

            let sub: Option<(Uuid,)> = sqlx::query_as(
                "SELECT submission_id FROM workflow_submissions \
                 WHERE submission_id = $1 AND workflow_instance_id = $2",
            )
            .bind(sub_id)
            .bind(instance_uuid)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;

            if sub.is_none() {
                return Err(ExecuteWorkflowTransitionError::InvalidReturnReferences(
                    format!(
                        "relatedSubmissionId {} does not exist or belongs to a different instance",
                        sub_id
                    ),
                ));
            }
        }
    }

    Ok(())
}

/// Map an ExecuteWorkflowTransitionError to a response body for deterministic failure receipts.
pub(crate) fn error_response_body(err: &ExecuteWorkflowTransitionError) -> serde_json::Value {
    match err {
        ExecuteWorkflowTransitionError::WorkflowStateVersionConflict { expected, actual } => {
            serde_json::json!({
                "error": "workflow_state_version_conflict",
                "expected": expected,
                "actual": actual,
            })
        }
        ExecuteWorkflowTransitionError::SubmissionValidationFailed(detail) => {
            serde_json::json!({
                "error": "submission_validation_failed",
                "detail": detail,
            })
        }
        ExecuteWorkflowTransitionError::TransitionNotApplicable(detail) => {
            serde_json::json!({
                "error": "transition_not_applicable",
                "detail": detail,
            })
        }
        ExecuteWorkflowTransitionError::SizeLimitExceeded(detail)
        | ExecuteWorkflowTransitionError::InvalidReturnReferences(detail)
        | ExecuteWorkflowTransitionError::AssigneeResolutionFailed(detail) => {
            let label = crate::domain::workflow_instance::errors::transition_error_label(err);
            serde_json::json!({"error": label, "detail": detail})
        }
        _ => {
            let label = crate::domain::workflow_instance::errors::transition_error_label(err);
            serde_json::json!({"error": label})
        }
    }
}

pub(super) fn is_deterministic_error(err: &ExecuteWorkflowTransitionError) -> bool {
    matches!(
        err,
        ExecuteWorkflowTransitionError::PrincipalNotFound
            | ExecuteWorkflowTransitionError::PrincipalDisabled
            | ExecuteWorkflowTransitionError::InstanceNotFound
            | ExecuteWorkflowTransitionError::CurrentVisitNotFound
            | ExecuteWorkflowTransitionError::PrincipalNotAssignee
            | ExecuteWorkflowTransitionError::AssistanceOpen
            | ExecuteWorkflowTransitionError::SourceNodeTerminal
            | ExecuteWorkflowTransitionError::DefinitionVersionRevoked
            | ExecuteWorkflowTransitionError::DefinitionVersionDraft
            | ExecuteWorkflowTransitionError::WorkflowStateVersionConflict { .. }
            | ExecuteWorkflowTransitionError::TransitionNotApplicable(_)
            | ExecuteWorkflowTransitionError::SubmissionRequired
            | ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)
            | ExecuteWorkflowTransitionError::SizeLimitExceeded(_)
            | ExecuteWorkflowTransitionError::InvalidReturnReferences(_)
            | ExecuteWorkflowTransitionError::AssigneeResolutionFailed(_)
    )
}

/// Persist a deterministic failure receipt. Call only before runtime facts are written.
pub(super) async fn persist_deterministic_failure(
    mut tx: Transaction<'_, Postgres>,
    command_id: Uuid,
    err: &ExecuteWorkflowTransitionError,
) -> Result<(), ExecuteWorkflowTransitionError> {
    let status = crate::domain::workflow_instance::errors::transition_error_code(err);
    let body = error_response_body(err);
    let response_digest =
        digest::compute_json_digest(&body).map_err(ExecuteWorkflowTransitionError::StorageError)?;
    complete_transition_receipt(&mut tx, command_id, status, &body, &response_digest).await?;
    tx.commit()
        .await
        .map_err(|error| ExecuteWorkflowTransitionError::StorageError(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::collect_return_contract_errors;
    use serde_json::json;

    /// A fully valid RETURN payload produces no contract errors.
    #[test]
    fn valid_return_payload_has_no_contract_errors() {
        let payload = json!({
            "rootCauseNodeVisitId": "550e8400-e29b-41d4-a716-446655440000",
            "reasonCode": "NEEDS_REVISION",
            "reason": "spec gap",
            "relatedSubmissionIds": ["550e8400-e29b-41d4-a716-446655440001"],
        });
        assert!(collect_return_contract_errors(&payload).is_empty());
    }

    /// Missing every required field reports each one (aggregated, not just the first).
    #[test]
    fn missing_all_required_fields_reports_each() {
        let payload = json!({ "summary": "no contract fields" });
        let errors = collect_return_contract_errors(&payload);
        assert_eq!(errors.len(), 3);
        assert!(
            errors
                .iter()
                .any(|e| e.contains("rootCauseNodeVisitId is required"))
        );
        assert!(errors.iter().any(|e| e.contains("reasonCode is required")));
        assert!(errors.iter().any(|e| e.contains("reason is required")));
    }

    /// Missing just one field reports exactly that one.
    #[test]
    fn missing_single_field_reports_only_it() {
        let base = json!({
            "rootCauseNodeVisitId": "550e8400-e29b-41d4-a716-446655440000",
            "reasonCode": "NEEDS_REVISION",
            "reason": "spec gap",
        });
        // Remove reasonCode
        let mut payload = base.clone();
        payload.as_object_mut().unwrap().remove("reasonCode");
        let errors = collect_return_contract_errors(&payload);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("reasonCode is required"));

        // Remove rootCauseNodeVisitId
        let mut payload = base.clone();
        payload.as_object_mut().unwrap().remove("rootCauseNodeVisitId");
        let errors = collect_return_contract_errors(&payload);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("rootCauseNodeVisitId is required"));

        // Remove reason
        let mut payload = base.clone();
        payload.as_object_mut().unwrap().remove("reason");
        let errors = collect_return_contract_errors(&payload);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("reason is required"));
    }

    /// Malformed rootCauseNodeVisitId is reported as invalid UUID.
    #[test]
    fn malformed_root_cause_reports_invalid_uuid() {
        let payload = json!({
            "rootCauseNodeVisitId": "not-a-uuid",
            "reasonCode": "NEEDS_REVISION",
            "reason": "spec gap",
        });
        let errors = collect_return_contract_errors(&payload);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not a valid UUID"));
    }

    /// relatedSubmissionIds shape violations are reported.
    #[test]
    fn malformed_related_submissions_reported() {
        // Not an array
        let payload = json!({
            "rootCauseNodeVisitId": "550e8400-e29b-41d4-a716-446655440000",
            "reasonCode": "NEEDS_REVISION",
            "reason": "spec gap",
            "relatedSubmissionIds": "oops",
        });
        let errors = collect_return_contract_errors(&payload);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("must be an array"));

        // Entry is not a string
        let payload = json!({
            "rootCauseNodeVisitId": "550e8400-e29b-41d4-a716-446655440000",
            "reasonCode": "NEEDS_REVISION",
            "reason": "spec gap",
            "relatedSubmissionIds": [123],
        });
        let errors = collect_return_contract_errors(&payload);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("must be strings"));

        // Entry is not a valid UUID
        let payload = json!({
            "rootCauseNodeVisitId": "550e8400-e29b-41d4-a716-446655440000",
            "reasonCode": "NEEDS_REVISION",
            "reason": "spec gap",
            "relatedSubmissionIds": ["not-a-uuid"],
        });
        let errors = collect_return_contract_errors(&payload);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("not a valid UUID"));
    }

    // PBT: any string-valued rootCauseNodeVisitId is either accepted (valid
    // UUID) or reported as invalid — never silently ignored.
    proptest::proptest! {
        #[test]
        fn root_cause_never_silently_ignored(
            s in proptest::string::string_regex(".*").unwrap()
        ) {
            let payload = json!({
                "rootCauseNodeVisitId": s,
                "reasonCode": "NEEDS_REVISION",
                "reason": "spec gap",
            });
            let errors = collect_return_contract_errors(&payload);
            let uuid_parse_ok = s.parse::<uuid::Uuid>().is_ok();
            if uuid_parse_ok {
                assert!(errors.is_empty(), "valid UUID should pass: {}", s);
            } else {
                assert_eq!(errors.len(), 1, "invalid UUID must be reported: {}", s);
                assert!(errors[0].contains("not a valid UUID"), "{}", errors[0]);
            }
        }
    }

    // PBT: an arbitrary JSON value never panics the contract checker and the
    // number of errors is bounded by the four contract fields it validates.
    proptest::proptest! {
        #[test]
        fn arbitrary_payload_is_bounded_and_total(
            value in proptest::collection::vec(proptest::arbitrary::any::<u8>(), 0..64)
        ) {
            let payload: serde_json::Value = serde_json::from_slice(&value)
                .unwrap_or(serde_json::Value::Null);
            let errors = collect_return_contract_errors(&payload);
            // rootCauseNodeVisitId / reasonCode / reason / relatedSubmissionIds
            assert!(errors.len() <= 6, "unexpected error count: {}", errors.len());
        }
    }
}
