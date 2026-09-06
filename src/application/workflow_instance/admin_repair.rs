//! Application wrapper for the maintenance repair capability (operator CLI).
//!
//! This is an operations/governance capability, NOT a broker tool and NOT an
//! HTTP API. `operator_principal_id` is audit attribution — the trusted host
//! environment runs the CLI; the role check (DOMAIN_OWNER or WORKFLOW_ADMIN)
//! guards against misoperation, it does not authenticate the caller.

use sqlx::PgPool;
use uuid::Uuid;

use crate::store::postgres::admission_gate::AdmissionGate;
use crate::store::postgres::workflow_instance_repository::repair_transaction::{
    repair_context_atomically, RepairContextCommand, RepairContextError, RepairContextOutcome,
};

/// Operator-facing repair request.
#[derive(Debug, Clone)]
pub struct RepairContextRequest {
    pub operator_principal_id: Uuid,
    pub workflow_instance_id: Uuid,
    pub context_payload: serde_json::Value,
    pub reason: String,
    pub repair_source: String,
}

fn to_command(request: RepairContextRequest) -> RepairContextCommand {
    RepairContextCommand {
        operator_principal_id: request.operator_principal_id,
        workflow_instance_id: request.workflow_instance_id,
        context_payload: request.context_payload,
        reason: request.reason,
        repair_source: request.repair_source,
    }
}

/// Full read-only dry run: every local check runs, nothing is written. The
/// gate is always dormant here — planning performs no admission (and no
/// directory traffic), per CTR-CIR-003's write-scoped rule.
pub async fn plan_repair_context(
    pool: &PgPool,
    request: RepairContextRequest,
) -> Result<RepairContextOutcome, RepairContextError> {
    repair_context_atomically(pool, AdmissionGate::disabled(), &to_command(request), false).await
}

/// Apply the repair: append context revision + pointer update + event +
/// security audit. Only reachable with an explicit operator decision.
/// `admission` admits the repaired payload's identity-bearing values before
/// the context revision is written.
pub async fn apply_repair_context(
    pool: &PgPool,
    admission: AdmissionGate<'_>,
    request: RepairContextRequest,
) -> Result<RepairContextOutcome, RepairContextError> {
    repair_context_atomically(pool, admission, &to_command(request), true).await
}
