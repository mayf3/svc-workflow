//! Atomic admin repair transaction for workflow instance context.
//!
//! A maintenance capability (operator CLI), NOT a broker tool and NOT an HTTP
//! API. Appends a new context revision that only ADDS missing required
//! assignee keys, preserves the append-only revision/event history, and
//! records a security audit.
//!
//! Security model (deliberately narrow): the CLI may only run in a trusted
//! host operations environment. `operator_principal_id` is an **audit
//! attribution**, not a caller-authenticated identity — the CLI cannot prove
//! the caller is that principal. The DOMAIN_OWNER/WORKFLOW_ADMIN role check
//! only guards against misoperation; it must not be described as
//! authentication.
//!
//! Invariants enforced on the repaired payload (mirroring the create-time
//! invariant):
//!   1. strict superset — existing keys keep their exact values;
//!   2. only missing required assignee keys may be added;
//!   3. every INSTANCE_INPUT_PRINCIPAL key is a string UUID of an enabled
//!      principal;
//!   4. payload validates against the definition's context_schema.

use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::domain::definition::digest;
use crate::domain::workflow_instance::events::{
    ContextRevisedEventData, CONTEXT_REVISED_EVENT_TYPE, EVENT_SCHEMA_VERSION,
};

pub const REPAIR_SECURITY_AUDIT_ACTION: &str = "REPAIR_CONTEXT_COMMITTED";

/// Operator-facing repair command. `operator_principal_id` is audit
/// attribution (see module docs).
#[derive(Debug, Clone)]
pub struct RepairContextCommand {
    pub operator_principal_id: Uuid,
    pub workflow_instance_id: Uuid,
    pub context_payload: serde_json::Value,
    pub reason: String,
    pub repair_source: String,
}

/// Full repair plan — the dry-run report and the basis for `--apply`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RepairContextPlan {
    pub instance_id: Uuid,
    pub domain_id: Uuid,
    pub definition_version_id: Uuid,
    pub current_context_revision_id: Uuid,
    pub current_revision_number: i32,
    pub current_state_version: i32,
    pub operator_principal_id: Uuid,
    pub reason: String,
    pub value_source: String,
    pub authorization_result: String,
    pub current_payload_keys: Vec<String>,
    pub required_input_keys: Vec<String>,
    pub missing_required_keys: Vec<String>,
    pub proposed_added_keys: Vec<String>,
    pub modified_existing_keys: Vec<String>,
    pub schema_validation: String,
    pub post_repair_invariant_result: String,
    pub apply: bool,
}

/// Result of a repair invocation — a dry-run plan or an applied outcome.
#[derive(Debug, Clone, serde::Serialize)]
pub struct RepairContextOutcome {
    pub plan: RepairContextPlan,
    pub applied: bool,
    pub new_context_revision_id: Option<Uuid>,
    pub new_revision_number: Option<i32>,
    pub new_state_version: Option<i32>,
    pub event_sequence: Option<i32>,
    pub event_type: Option<String>,
    pub security_audit_action: Option<String>,
}

#[derive(Debug)]
pub enum RepairContextError {
    InstanceNotFound,
    InstanceCancelled,
    InstanceArchived,
    OperatorPrincipalNotFound,
    OperatorPrincipalDisabled,
    OperatorPrincipalTypeNotAllowed,
    OperatorNotAuthorized(String),
    DefinitionVersionRevoked,
    ContextSchemaInvalid(String),
    PayloadNotSuperset(Vec<String>),
    PayloadAddsNonRequiredKeys(Vec<String>),
    InvariantViolation(String),
    InternalConsistency(String),
    StorageError(String),
}

impl std::fmt::Display for RepairContextError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InstanceNotFound => write!(f, "instance not found"),
            Self::InstanceCancelled => write!(f, "instance is cancelled"),
            Self::InstanceArchived => write!(f, "instance is archived"),
            Self::OperatorPrincipalNotFound => write!(f, "operator principal not found"),
            Self::OperatorPrincipalDisabled => write!(f, "operator principal is disabled"),
            Self::OperatorPrincipalTypeNotAllowed => {
                write!(f, "operator principal type not allowed (SERVICE)")
            }
            Self::OperatorNotAuthorized(detail) => {
                write!(f, "operator not authorized: {detail}")
            }
            Self::DefinitionVersionRevoked => {
                write!(f, "definition version is REVOKED, repair blocked")
            }
            Self::ContextSchemaInvalid(detail) => write!(f, "context schema invalid: {detail}"),
            Self::PayloadNotSuperset(keys) => write!(
                f,
                "repair payload must not alter existing context values (changed keys: {})",
                keys.join(", ")
            ),
            Self::PayloadAddsNonRequiredKeys(keys) => write!(
                f,
                "repair may only add missing required assignee keys, got extra keys: {}",
                keys.join(", ")
            ),
            Self::InvariantViolation(detail) => {
                write!(f, "post-repair invariant violation: {detail}")
            }
            Self::InternalConsistency(detail) => write!(f, "internal consistency: {detail}"),
            Self::StorageError(detail) => write!(f, "storage error: {detail}"),
        }
    }
}

impl std::error::Error for RepairContextError {}

/// Plan (and optionally apply) a context repair in one transaction.
///
/// `apply == false` performs a full read-only dry run: every check runs, the
/// plan is returned, and the transaction rolls back — nothing is written.
pub async fn repair_context_atomically(
    pool: &PgPool,
    cmd: &RepairContextCommand,
    apply: bool,
) -> Result<RepairContextOutcome, RepairContextError> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    // ------------------------------------------------------------------
    // 1. Lock and read the instance (guards: not cancelled / not archived)
    // ------------------------------------------------------------------
    let instance: Option<(
        Uuid,
        Uuid,
        Uuid,
        Uuid,
        Uuid,
        Uuid,
        i32,
        bool,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        "SELECT workflow_instance_id, domain_id, definition_version_id, \
                created_by_principal_id, current_context_revision_id, \
                current_node_visit_id, workflow_state_version, cancelled, archived_at \
         FROM workflow_instances WHERE workflow_instance_id = $1 FOR UPDATE",
    )
    .bind(cmd.workflow_instance_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    let (
        instance_id,
        domain_id,
        definition_version_id,
        _created_by,
        current_context_revision_id,
        current_node_visit_id,
        current_state_version,
        cancelled,
        archived_at,
    ) = instance.ok_or(RepairContextError::InstanceNotFound)?;

    if cancelled {
        return Err(RepairContextError::InstanceCancelled);
    }
    if archived_at.is_some() {
        return Err(RepairContextError::InstanceArchived);
    }

    // ------------------------------------------------------------------
    // 2. Operator: audit attribution + role guard (DOMAIN_OWNER or WORKFLOW_ADMIN)
    // ------------------------------------------------------------------
    let operator: Option<(bool, String)> = sqlx::query_as(
        "SELECT enabled, principal_type::TEXT FROM principals \
         WHERE principal_id = $1",
    )
    .bind(cmd.operator_principal_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    let (operator_enabled, operator_type) =
        operator.ok_or(RepairContextError::OperatorPrincipalNotFound)?;
    if !operator_enabled {
        return Err(RepairContextError::OperatorPrincipalDisabled);
    }
    if operator_type == "SERVICE" {
        return Err(RepairContextError::OperatorPrincipalTypeNotAllowed);
    }

    let role: Option<(String,)> = sqlx::query_as(
        "SELECT role_key FROM domain_role_bindings \
         WHERE domain_id = $1 AND principal_id = $2 \
           AND role_key IN ('DOMAIN_OWNER', 'WORKFLOW_ADMIN') AND enabled = TRUE \
         ORDER BY (role_key = 'WORKFLOW_ADMIN') DESC LIMIT 1",
    )
    .bind(domain_id)
    .bind(cmd.operator_principal_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    let authorization_result = match role {
        Some((role_key,)) => format!("ok ({role_key})"),
        None => {
            return Err(RepairContextError::OperatorNotAuthorized(
                "operator must hold DOMAIN_OWNER or WORKFLOW_ADMIN for the instance domain"
                    .to_string(),
            ))
        }
    };

    // ------------------------------------------------------------------
    // 3. Definition version must not be REVOKED
    // ------------------------------------------------------------------
    let status: Option<(String,)> = sqlx::query_as(
        "SELECT version_status::TEXT FROM workflow_definition_versions \
         WHERE definition_version_id = $1",
    )
    .bind(definition_version_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    let (definition_status,) = status.ok_or(RepairContextError::InternalConsistency(
        "definition version row missing".to_string(),
    ))?;
    if definition_status == "REVOKED" {
        return Err(RepairContextError::DefinitionVersionRevoked);
    }

    // ------------------------------------------------------------------
    // 4. Read the current context revision (payload included)
    // ------------------------------------------------------------------
    let current: Option<(Uuid, i32, serde_json::Value)> = sqlx::query_as(
        "SELECT context_revision_id, revision_number, payload \
         FROM workflow_context_revisions \
         WHERE context_revision_id = $1 AND workflow_instance_id = $2",
    )
    .bind(current_context_revision_id)
    .bind(instance_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    let (current_revision_id, current_revision_number, current_payload) = current.ok_or(
        RepairContextError::InternalConsistency("current context revision not found".to_string()),
    )?;

    // ------------------------------------------------------------------
    // 5. Derive required assignee input keys from the definition
    // ------------------------------------------------------------------
    // Several nodes may reference the same input key; the invariant is about
    // the key set, so present it deduplicated (rows are sorted by key).
    let mut required_input_keys: Vec<String> = sqlx::query_scalar(
        "SELECT assignee_input_key FROM workflow_node_definitions \
         WHERE definition_version_id = $1 AND assignee_ref_type = 'INSTANCE_INPUT_PRINCIPAL' \
         ORDER BY assignee_input_key",
    )
    .bind(definition_version_id)
    .fetch_all(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;
    required_input_keys.dedup();

    // ------------------------------------------------------------------
    // 6. Superset discipline: existing keys unchanged, only missing keys added
    // ------------------------------------------------------------------
    let current_payload_obj =
        current_payload
            .as_object()
            .ok_or(RepairContextError::InternalConsistency(
                "current context payload is not a JSON object".to_string(),
            ))?;
    let new_payload_obj = cmd.context_payload.as_object().ok_or_else(|| {
        RepairContextError::ContextSchemaInvalid("repair payload must be a JSON object".to_string())
    })?;

    let mut current_payload_keys: Vec<String> = current_payload_obj.keys().cloned().collect();
    current_payload_keys.sort();
    let mut new_payload_keys: Vec<String> = new_payload_obj.keys().cloned().collect();
    new_payload_keys.sort();

    let mut modified_existing_keys: Vec<String> = Vec::new();
    for (key, value) in current_payload_obj {
        match new_payload_obj.get(key) {
            Some(new_value) if new_value == value => {}
            _ => modified_existing_keys.push(key.clone()),
        }
    }
    modified_existing_keys.sort();
    if !modified_existing_keys.is_empty() {
        return Err(RepairContextError::PayloadNotSuperset(
            modified_existing_keys,
        ));
    }

    let missing_required_keys: Vec<String> = required_input_keys
        .iter()
        .filter(|key| !current_payload_obj.contains_key(*key))
        .cloned()
        .collect();

    let mut proposed_added_keys: Vec<String> = new_payload_keys
        .iter()
        .filter(|key| !current_payload_obj.contains_key(*key))
        .cloned()
        .collect();
    proposed_added_keys.sort();

    let extra_added: Vec<String> = proposed_added_keys
        .iter()
        .filter(|key| !missing_required_keys.contains(key))
        .cloned()
        .collect();
    if !extra_added.is_empty() {
        return Err(RepairContextError::PayloadAddsNonRequiredKeys(extra_added));
    }

    // ------------------------------------------------------------------
    // 7. Schema validation of the repaired payload
    // ------------------------------------------------------------------
    let schema_row: Option<(Option<serde_json::Value>,)> = sqlx::query_as(
        "SELECT context_schema FROM workflow_definition_versions \
         WHERE definition_version_id = $1",
    )
    .bind(definition_version_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;
    let context_schema = schema_row.and_then(|(schema,)| schema);

    let schema_validation = if let Some(schema) = &context_schema {
        let validator = jsonschema::validator_for(schema).map_err(|e| {
            RepairContextError::ContextSchemaInvalid(format!(
                "context_schema compilation failed: {e}"
            ))
        })?;
        validator.validate(&cmd.context_payload).map_err(|e| {
            RepairContextError::ContextSchemaInvalid(format!(
                "repair payload failed context_schema validation: {e}"
            ))
        })?;
        "pass".to_string()
    } else {
        "pass (no context_schema)".to_string()
    };

    // ------------------------------------------------------------------
    // 8. Post-repair invariant: every required key is a string UUID of an
    //    enabled principal (mirrors create-time validation)
    // ------------------------------------------------------------------
    let mut invariant_violations: Vec<String> = Vec::new();
    for key in &required_input_keys {
        let Some(raw) = cmd.context_payload.get(key) else {
            invariant_violations.push(format!("missing required assignee key '{key}'"));
            continue;
        };
        let Some(s) = raw.as_str() else {
            invariant_violations.push(format!("assignee key '{key}' must be a string UUID"));
            continue;
        };
        let Ok(candidate) = Uuid::parse_str(s) else {
            invariant_violations.push(format!("assignee key '{key}' is not a valid UUID"));
            continue;
        };
        let principal: Option<(bool,)> =
            sqlx::query_as("SELECT enabled FROM principals WHERE principal_id = $1")
                .bind(candidate)
                .fetch_optional(&mut *tx)
                .await
                .map_err(|e| RepairContextError::StorageError(e.to_string()))?;
        match principal {
            None => invariant_violations.push(format!(
                "assignee key '{key}' references unknown principal '{candidate}'"
            )),
            Some((false,)) => invariant_violations.push(format!(
                "assignee key '{key}' references disabled principal '{candidate}'"
            )),
            _ => {}
        }
    }
    let post_repair_invariant_result = if invariant_violations.is_empty() {
        "pass".to_string()
    } else {
        format!("fail: {}", invariant_violations.join("; "))
    };

    let plan = RepairContextPlan {
        instance_id,
        domain_id,
        definition_version_id,
        current_context_revision_id: current_revision_id,
        current_revision_number,
        current_state_version,
        operator_principal_id: cmd.operator_principal_id,
        reason: cmd.reason.clone(),
        value_source: cmd.repair_source.clone(),
        authorization_result,
        current_payload_keys,
        required_input_keys,
        missing_required_keys,
        proposed_added_keys,
        modified_existing_keys,
        schema_validation,
        post_repair_invariant_result,
        apply,
    };

    if !apply {
        tx.rollback()
            .await
            .map_err(|e| RepairContextError::StorageError(e.to_string()))?;
        return Ok(RepairContextOutcome {
            plan,
            applied: false,
            new_context_revision_id: None,
            new_revision_number: None,
            new_state_version: None,
            event_sequence: None,
            event_type: None,
            security_audit_action: None,
        });
    }

    if !invariant_violations.is_empty() {
        tx.rollback()
            .await
            .map_err(|e| RepairContextError::StorageError(e.to_string()))?;
        return Err(RepairContextError::InvariantViolation(
            invariant_violations.join("; "),
        ));
    }

    // ------------------------------------------------------------------
    // 9. Apply: append revision -> update pointer -> CONTEXT_REVISED event
    //    -> security audit -> commit
    // ------------------------------------------------------------------
    let new_context_revision_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    let command_id = Uuid::new_v4();
    let new_revision_number = current_revision_number + 1;
    let new_state_version = current_state_version + 1;
    let event_sequence = new_state_version;
    let new_payload_digest = digest::compute_json_digest(&cmd.context_payload)
        .map_err(|e| RepairContextError::StorageError(e.to_string()))?;
    let current_payload_digest = digest::compute_json_digest(&current_payload)
        .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO workflow_context_revisions
            (context_revision_id, workflow_instance_id, revision_number,
             previous_revision_id, payload, payload_digest,
             created_by_principal_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        "#,
    )
    .bind(new_context_revision_id)
    .bind(instance_id)
    .bind(new_revision_number)
    .bind(current_revision_id)
    .bind(&cmd.context_payload)
    .bind(&new_payload_digest)
    .bind(cmd.operator_principal_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    sqlx::query(
        "UPDATE workflow_instances \
         SET current_context_revision_id = $1, workflow_state_version = $2 \
         WHERE workflow_instance_id = $3",
    )
    .bind(new_context_revision_id)
    .bind(new_state_version)
    .bind(instance_id)
    .execute(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    // Command receipt: the CONTEXT_REVISED event's command_id references
    // workflow_command_receipts (deferred FK), so the repair leaves the same
    // command/attempt trail as any other state-changing command.
    let request_hash = digest::compute_sha256(
        format!("repair-context:{}:{}", instance_id, new_revision_number).as_bytes(),
    );
    sqlx::query(
        "INSERT INTO workflow_command_receipts \
         (command_id, principal_id, idempotency_key, command_type, request_hash, receipt_status) \
         VALUES ($1, $2, $3, 'REPAIR_WORKFLOW_CONTEXT', $4, 'COMPLETED')",
    )
    .bind(command_id)
    .bind(cmd.operator_principal_id)
    .bind(format!("repair-{}-{}", instance_id, Uuid::new_v4()))
    .bind(request_hash)
    .execute(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    // Current node id for the event payload (mirrors the revise path).
    let node_id: Option<(Uuid,)> = sqlx::query_as(
        "SELECT nd.node_id FROM workflow_node_visits nv \
         JOIN workflow_node_definitions nd ON nd.node_id = nv.node_id \
         WHERE nv.node_visit_id = $1 AND nv.workflow_instance_id = $2",
    )
    .bind(current_node_visit_id)
    .bind(instance_id)
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;
    let current_node_id =
        node_id
            .map(|(id,)| id)
            .ok_or(RepairContextError::InternalConsistency(
                "current node visit row missing".to_string(),
            ))?;

    let event_data = ContextRevisedEventData {
        previous_context_revision_id: current_revision_id.to_string(),
        new_context_revision_id: new_context_revision_id.to_string(),
        previous_payload_digest: current_payload_digest,
        new_payload_digest: new_payload_digest.clone(),
        current_node_id: current_node_id.to_string(),
    };
    let event_data_json = serde_json::to_value(&event_data)
        .map_err(|e| RepairContextError::StorageError(e.to_string()))?;
    let event_data_digest = digest::compute_json_digest(&event_data_json)
        .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    sqlx::query(
        r#"
        INSERT INTO workflow_events
            (event_id, workflow_instance_id, event_sequence, event_schema_version,
             command_id, event_type, source_node_visit_id, target_node_visit_id,
             context_revision_id, event_data, event_data_digest,
             actor_principal_id, old_workflow_state_version, new_workflow_state_version)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
        "#,
    )
    .bind(event_id)
    .bind(instance_id)
    .bind(event_sequence)
    .bind(EVENT_SCHEMA_VERSION)
    .bind(command_id)
    .bind(CONTEXT_REVISED_EVENT_TYPE)
    .bind(current_node_visit_id)
    .bind(current_node_visit_id)
    .bind(new_context_revision_id)
    .bind(&event_data_json)
    .bind(&event_data_digest)
    .bind(cmd.operator_principal_id)
    .bind(current_state_version)
    .bind(new_state_version)
    .execute(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    // Security audit: instanceId, old/new revision, operator, reason,
    // repair source, added keys, payload digest — append-only governance trail.
    let audit_details = json!({
        "reason": cmd.reason,
        "repairSource": cmd.repair_source,
        "operatorPrincipalId": cmd.operator_principal_id,
        "oldContextRevisionId": current_revision_id,
        "oldRevisionNumber": current_revision_number,
        "newContextRevisionId": new_context_revision_id,
        "newRevisionNumber": new_revision_number,
        "oldWorkflowStateVersion": current_state_version,
        "newWorkflowStateVersion": new_state_version,
        "addedKeys": plan.proposed_added_keys,
        "payloadDigest": new_payload_digest,
    });
    sqlx::query(
        "INSERT INTO workflow_security_audits \
         (audit_id, principal_id, action, resource_type, resource_id, details) \
         VALUES ($1, $2, $3, 'WORKFLOW_INSTANCE', $4, $5)",
    )
    .bind(Uuid::new_v4())
    .bind(cmd.operator_principal_id)
    .bind(REPAIR_SECURITY_AUDIT_ACTION)
    .bind(instance_id.to_string())
    .bind(&audit_details)
    .execute(&mut *tx)
    .await
    .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    tx.commit()
        .await
        .map_err(|e| RepairContextError::StorageError(e.to_string()))?;

    Ok(RepairContextOutcome {
        plan,
        applied: true,
        new_context_revision_id: Some(new_context_revision_id),
        new_revision_number: Some(new_revision_number),
        new_state_version: Some(new_state_version),
        event_sequence: Some(event_sequence),
        event_type: Some(CONTEXT_REVISED_EVENT_TYPE.to_string()),
        security_audit_action: Some(REPAIR_SECURITY_AUDIT_ACTION.to_string()),
    })
}
