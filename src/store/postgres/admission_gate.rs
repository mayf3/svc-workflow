//! In-transaction canonical identity admission enforcement for
//! assignment-producing Workflow commands.
//!
//! Governing authority:
//! `docs/specs/SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2.md`
//! CTR-CIR-003 (accepted): for each distinct Agent Principal within the
//! command, obtain the exact authoritative Auth relation/type/status plus
//! Agent Definition existence/enabled — after request identity/schema
//! authorization and **before the database commit** — by calling the pinned
//! directory reads. Total admission-through-commit is bounded to 5 seconds
//! from the first admission request start (monotonic clock) with a database
//! statement/transaction deadline no later than that bound. Any admission
//! failure rejects the ENTIRE business write: the committing transaction is
//! rolled back so the outcome is zero business delta — no partial write, no
//! automatic retry, and no deterministic failure receipt (a stored receipt
//! would be a cached cross-command result, which CTR-CIR-003 forbids).
//! Per-command identical IDs share one still-current observation; no reuse
//! across commands (fresh directory reads every command, observations are
//! consumed immediately inside the same transaction that commits them — the
//! transaction's existing lock/re-validate preconditions provide the
//! discard-on-drift discipline).
//!
//! Dormant mode: when the admission client is absent (`None`, the default
//! deployment), every method here is a no-op and runtime behavior is exactly
//! the pre-admission behavior. Activation is a separate reviewed production
//! step.
//!
//! Surfaces that CANNOT_INTRODUCE_AGENT_ASSIGNMENT (out of scope for this
//! wiring step, each with its reason):
//! - definition publish identity literals (`graph_write.rs`) — source-graph
//!   conservation is the corrected-source publish step of CTR-CIR-003 and is
//!   sequenced as a later reconciliation step;
//! - source stop-new — the reconciliation operator's source-closure command,
//!   a later step of the same Spec;
//! - lineage operator — CTR-CIR-004 migration apply, whose identities are
//!   exact plan-pinned values, not dynamic command input;
//! - provisioning `replace_owner` — Domain-role binding ownership change on
//!   the provisioning surface, never a Workflow visit/context assignment;
//! - provisioning `self_projection` — the caller's own identity
//!   registration, not an assignment of work to any principal.

use std::collections::BTreeSet;
use std::time::Instant;

use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::auth::admission::{AdmissionClient, AdmissionError};

/// Per-command admission gate.
///
/// `client == None` is the dormant deploy: every check is a no-op and the
/// database keeps its existing timeout behavior. When enabled, `start` is the
/// monotonic instant captured at app-service entry — before the transaction
/// opens — and bounds the whole admission-through-commit window.
#[derive(Clone, Copy)]
pub struct AdmissionGate<'a> {
    client: Option<&'a AdmissionClient>,
    start: Instant,
}

impl<'a> AdmissionGate<'a> {
    /// Dormant mode (admission disabled): enforcement is a no-op.
    pub fn disabled() -> Self {
        Self {
            client: None,
            start: Instant::now(),
        }
    }

    /// Build the gate at app-service entry; the monotonic command start is
    /// captured here, before the transaction opens (CTR-CIR-003).
    pub fn new(client: Option<&'a AdmissionClient>) -> Self {
        Self {
            client,
            start: Instant::now(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.client.is_some()
    }

    /// Bind the transaction-scoped database statement deadline to the
    /// remaining admission budget (CTR-CIR-003: the database statement/
    /// transaction deadline must be no later than the 5-second
    /// admission-through-commit bound).
    ///
    /// Implemented with `set_config('statement_timeout', …, is_local => true)`
    /// — the parameterized equivalent of `SET LOCAL`, scoped to exactly this
    /// transaction. Dormant mode issues no statement at all.
    pub async fn bind_statement_deadline(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<(), sqlx::Error> {
        let Some(client) = self.client else {
            return Ok(());
        };
        let remaining_ms = client.remaining_budget_ms(self.start);
        sqlx::query("SELECT set_config('statement_timeout', $1, true)")
            .bind(remaining_ms.to_string())
            .execute(&mut **tx)
            .await
            .map(|_| ())
    }

    /// Fail-closed pre-commit budget check: admission-through-commit must fit
    /// inside the 5-second window, so a command with no remaining budget must
    /// not commit. Dormant mode always passes.
    pub fn check_commit_budget(&self) -> Result<(), AdmissionError> {
        match self.client {
            None => Ok(()),
            Some(client) => {
                if client.remaining_budget_ms(self.start) == 0 {
                    Err(AdmissionError::Timeout)
                } else {
                    Ok(())
                }
            }
        }
    }

    /// Admit every distinct Agent Principal the command will persist, inside
    /// the committing transaction and before its first runtime-fact write.
    ///
    /// Duplicate IDs are collapsed into one still-current observation
    /// (CTR-CIR-003 allows per-command sharing; nothing is cached across
    /// commands). An empty set means the command persists no assignment
    /// target and acceptance is trivially satisfied — no network is touched.
    /// Dormant mode is always `Ok(())`.
    pub async fn admit(
        &self,
        principals: impl IntoIterator<Item = Uuid>,
    ) -> Result<(), AdmissionError> {
        let Some(client) = self.client else {
            return Ok(());
        };
        let distinct: BTreeSet<Uuid> = principals.into_iter().collect();
        client.admit(self.start, &distinct).await.map(|_| ())
    }
}

/// Read the definition version's `INSTANCE_INPUT_PRINCIPAL` assignee keys —
/// the context positions that carry identity-bearing values (CTR-CIR-003:
/// "all identity-bearing values reachable in the resulting
/// Context/configuration", not just the current node owner).
pub(crate) async fn required_input_principal_keys(
    tx: &mut Transaction<'_, Postgres>,
    definition_version_id: Uuid,
) -> Result<Vec<String>, sqlx::Error> {
    let mut keys: Vec<String> = sqlx::query_scalar(
        "SELECT assignee_input_key FROM workflow_node_definitions \
         WHERE definition_version_id = $1 AND assignee_ref_type = 'INSTANCE_INPUT_PRINCIPAL' \
         ORDER BY assignee_input_key",
    )
    .bind(definition_version_id)
    .fetch_all(&mut **tx)
    .await?;
    keys.dedup();
    Ok(keys)
}

/// Pure extraction of the identity values a context payload carries for the
/// definition's required `INSTANCE_INPUT_PRINCIPAL` keys.
///
/// - a missing key persists nothing → skipped (the seam's existing behavior
///   governs; admission never invents a new presence requirement);
/// - a present-but-invalid value (non-string, non-UUID) is invalid
///   identity-bearing input that CTR-CIR-003 requires rejecting before the
///   context value is persisted → `Err(detail)`.
///
/// Mirrors `validation_helpers::extract_principal_uuid_from_input`: only
/// string-form UUIDs are identity values; display names, emails and legacy
/// agent IDs are never resolved.
pub(crate) fn extract_required_input_principal_ids(
    keys: &[String],
    payload: &serde_json::Value,
) -> Result<Vec<Uuid>, String> {
    let mut ids = Vec::new();
    for key in keys {
        let Some(raw) = payload.get(key) else {
            continue;
        };
        let Some(value) = raw.as_str() else {
            return Err(format!(
                "instance input '{key}' must be a string UUID (stable principal identifier); \
                 display name / email / legacy id resolution is forbidden"
            ));
        };
        let parsed = Uuid::parse_str(value)
            .map_err(|_| format!("instance input '{key}' is not a valid UUID: '{value}'"))?;
        ids.push(parsed);
    }
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extracts_every_required_key_value_and_dedupes() {
        let keys = vec!["ownerA".to_string(), "ownerB".to_string()];
        let a = Uuid::new_v4();
        let payload = json!({ "ownerA": a.to_string(), "ownerB": a.to_string() });
        let ids = extract_required_input_principal_ids(&keys, &payload).expect("extracts");
        assert_eq!(
            ids,
            vec![a, a],
            "one entry per key; caller dedupes via admit()"
        );
    }

    #[test]
    fn missing_keys_are_skipped_not_errors() {
        let keys = vec!["present".to_string(), "absent".to_string()];
        let p = Uuid::new_v4();
        let payload = json!({ "present": p.to_string() });
        let ids = extract_required_input_principal_ids(&keys, &payload).expect("extracts");
        assert_eq!(ids, vec![p]);
    }

    #[test]
    fn non_uuid_value_is_rejected_with_key_context() {
        let keys = vec!["owner".to_string()];
        for value in ["not-a-uuid", "display name", "agent-123"] {
            let payload = json!({ "owner": value });
            let error = extract_required_input_principal_ids(&keys, &payload)
                .expect_err("invalid identity value must be rejected");
            assert!(error.contains("'owner'"), "detail names the key: {error}");
        }
    }

    #[test]
    fn non_string_value_is_rejected() {
        let keys = vec!["owner".to_string()];
        let payload = json!({ "owner": 42 });
        let error = extract_required_input_principal_ids(&keys, &payload)
            .expect_err("non-string identity value must be rejected");
        assert!(error.contains("must be a string UUID"), "{error}");
    }

    #[test]
    fn no_required_keys_means_no_extraction() {
        let payload = json!({ "anything": "unrelated" });
        let ids =
            extract_required_input_principal_ids(&[], &payload).expect("empty key set is fine");
        assert!(ids.is_empty());
    }

    #[test]
    fn dormant_gate_is_total_noop() {
        let gate = AdmissionGate::disabled();
        assert!(!gate.is_enabled());
        assert_eq!(gate.check_commit_budget(), Ok(()));
    }

    #[tokio::test]
    async fn dormant_gate_admits_without_network() {
        let gate = AdmissionGate::disabled();
        gate.admit([Uuid::new_v4(), Uuid::new_v4()])
            .await
            .expect("dormant admission is a no-op");
    }
}
