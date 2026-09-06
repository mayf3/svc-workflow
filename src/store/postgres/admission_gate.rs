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
//! Surfaces wired for canonical identity admission:
//! - workflow instance create / transition / revise / revise-and-transition /
//!   admin recovery (assignment-producing runtime commands);
//! - definition publish identity literals (CTR-CIR-003: "Apply this admission
//!   rule to corrected-source publish/defaults/enums ... Validate all
//!   supplied role identities and all identity-bearing values reachable in
//!   the resulting Context/configuration"). Draft/reopen persist literals
//!   too, but a DRAFT is not executable truth — publish is the authoritative
//!   gate where the version becomes runnable, so admission fires there only.
//!
//! Surfaces that CANNOT_INTRODUCE_AGENT_ASSIGNMENT (out of scope, each with
//! its reason):
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
use crate::domain::definition::error::GraphValidationError;
use crate::domain::definition::model::NodeDefinition;
use crate::domain::enums::AssigneeRefType;

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

/// Graph-validation error codes mirroring the INSTANCE_INPUT grammar family
/// of `domain/definition/graph/assignee_validation.rs`.
const INSTANCE_INPUT_LITERAL_NOT_STRING: &str = "INSTANCE_INPUT_LITERAL_NOT_STRING";
const INSTANCE_INPUT_LITERAL_NOT_UUID: &str = "INSTANCE_INPUT_LITERAL_NOT_UUID";

/// Pure extraction of every identity literal a definition publish would make
/// executable (CTR-CIR-003: "Validate all supplied role identities and all
/// identity-bearing values reachable in the resulting
/// Context/configuration").
///
/// Two sources, deduplicated into one distinct set:
/// 1. every FIXED_PRINCIPAL `fixed_principal_id` in the graph — collected
///    without type pre-filtering: the admission Auth directory read itself
///    rejects a missing/noncanonical/non-active principal (404/422 maps to
///    admission rejection), which IS the canonical Agent check;
/// 2. every identity literal the context schema carries for an
///    INSTANCE_INPUT_PRINCIPAL node's `assignee_input_key`: the `default`,
///    `enum` and `examples` values at that flat property path. Only
///    string-form UUIDs are identity literals (mirroring
///    `extract_required_input_principal_ids`); a present-but-invalid value in
///    an identity-bearing position is rejected BEFORE any persistence as a
///    graph-validation error.
///
/// Draft replacement persists the same literals, but a DRAFT is not
/// executable truth; publish is the authoritative gate where admission fires
/// (CTR-CIR-003: "corrected-source publish").
pub fn collect_definition_publish_identity_literals(
    nodes: &[NodeDefinition],
    context_schema: Option<&serde_json::Value>,
) -> Result<BTreeSet<Uuid>, Vec<GraphValidationError>> {
    let mut literals: BTreeSet<Uuid> = BTreeSet::new();
    let mut errors: Vec<GraphValidationError> = Vec::new();

    for node in nodes {
        let Some(assignee_ref) = &node.assignee_ref else {
            continue;
        };
        match assignee_ref.ref_type {
            AssigneeRefType::FixedPrincipal => {
                if let Some(principal) = assignee_ref.fixed_principal_id {
                    literals.insert(principal.into_uuid());
                }
            }
            AssigneeRefType::InstanceInputPrincipal => {
                let Some(key) = &assignee_ref.assignee_input_key else {
                    continue;
                };
                collect_schema_identity_literals(
                    context_schema,
                    key,
                    &mut literals,
                    &mut errors,
                );
            }
            _ => {}
        }
    }

    if errors.is_empty() {
        Ok(literals)
    } else {
        Err(errors)
    }
}

/// Extract the `default`, `enum` and `examples` identity literals at the
/// flat property path `properties.<key>` of the context schema, appending
/// graph-validation errors for any present-but-invalid value.
fn collect_schema_identity_literals(
    context_schema: Option<&serde_json::Value>,
    key: &str,
    literals: &mut BTreeSet<Uuid>,
    errors: &mut Vec<GraphValidationError>,
) {
    let Some(property) = context_schema
        .and_then(|schema| schema.get("properties"))
        .and_then(|properties| properties.get(key))
    else {
        // No declared property: nothing authoritative is supplied by the
        // schema; per-creation inputs remain governed by the runtime seams.
        return;
    };

    // `default`: a single value.
    inspect_identity_literal(
        property.get("default"),
        key,
        "default",
        literals,
        errors,
    );

    // `enum` / `examples`: arrays of values.
    for keyword in ["enum", "examples"] {
        if let Some(values) = property.get(keyword).and_then(|v| v.as_array()) {
            for (index, value) in values.iter().enumerate() {
                inspect_identity_literal(
                    Some(value),
                    key,
                    &format!("{keyword}[{index}]"),
                    literals,
                    errors,
                );
            }
        }
    }
}

/// Validate one schema-positioned identity value: only string-form UUIDs are
/// identity literals; anything else in an identity-bearing position is a
/// validation error (CTR-CIR-003 requires invalid/stale input to be rejected
/// before persisting an identity-bearing Context value).
fn inspect_identity_literal(
    value: Option<&serde_json::Value>,
    key: &str,
    position: &str,
    literals: &mut BTreeSet<Uuid>,
    errors: &mut Vec<GraphValidationError>,
) {
    let Some(value) = value else {
        return;
    };
    let Some(raw) = value.as_str() else {
        errors.push(GraphValidationError::new(
            INSTANCE_INPUT_LITERAL_NOT_STRING,
            format!(
                "context_schema property '{key}' {position} carries a non-string value; \
                 identity literals must be exact Principal UUID strings"
            ),
        ));
        return;
    };
    match Uuid::parse_str(raw) {
        Ok(principal) => {
            literals.insert(principal);
        }
        Err(_) => {
            errors.push(GraphValidationError::new(
                INSTANCE_INPUT_LITERAL_NOT_UUID,
                format!(
                    "context_schema property '{key}' {position} carries '{raw}' which is not a \
                     UUID; identity literals must be exact Principal UUIDs (display name / \
                     email / legacy id resolution is forbidden)"
                ),
            ));
        }
    }
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

    // ---- definition publish identity-literal extraction (CTR-CIR-003) ----

    use crate::domain::definition::model::AssigneeRef;
    use crate::domain::enums::NodeType;
    use crate::domain::ids::{DefinitionVersionId, NodeId, PrincipalId};

    fn node(
        node_key: &str,
        ref_type: AssigneeRefType,
        fixed: Option<Uuid>,
        input_key: Option<&str>,
    ) -> NodeDefinition {
        NodeDefinition {
            node_id: NodeId::from_uuid(Uuid::new_v4()),
            definition_version_id: DefinitionVersionId::from_uuid(Uuid::new_v4()),
            node_key: node_key.to_string(),
            display_name: "Node".to_string(),
            order_index: 0,
            node_type: NodeType::NORMAL,
            assignee_ref: Some(AssigneeRef {
                ref_type,
                fixed_principal_id: fixed.map(PrincipalId::from_uuid),
                assignee_input_key: input_key.map(|s| s.to_string()),
            }),
            instructions: None,
            primary_advance_transition_id: None,
            metadata: None,
            created_at: chrono::Utc::now(),
        }
    }

    fn schema_property(key: &str, property: serde_json::Value) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": [key],
            "properties": { key: property }
        })
    }

    #[test]
    fn collects_fixed_principals_and_dedupes() {
        let agent = Uuid::new_v4();
        let other = Uuid::new_v4();
        let nodes = vec![
            node("a", AssigneeRefType::FixedPrincipal, Some(agent), None),
            node("b", AssigneeRefType::FixedPrincipal, Some(agent), None),
            node("c", AssigneeRefType::FixedPrincipal, Some(other), None),
            node("d", AssigneeRefType::WorkflowCreator, None, None),
            node(
                "e",
                AssigneeRefType::InstanceInputPrincipal,
                None,
                Some("owner"),
            ),
        ];
        let literals =
            collect_definition_publish_identity_literals(&nodes, None).expect("no schema errors");
        assert_eq!(literals, BTreeSet::from([agent, other]));
    }

    #[test]
    fn collects_default_enum_and_examples_literals() {
        let default = Uuid::new_v4();
        let enum_a = Uuid::new_v4();
        let enum_b = Uuid::new_v4();
        let example = Uuid::new_v4();
        let nodes = vec![node(
            "work",
            AssigneeRefType::InstanceInputPrincipal,
            None,
            Some("owner"),
        )];
        let schema = schema_property(
            "owner",
            serde_json::json!({
                "type": "string",
                "default": default.to_string(),
                "enum": [enum_a.to_string(), enum_b.to_string(), default.to_string()],
                "examples": [example.to_string()],
            }),
        );
        let literals = collect_definition_publish_identity_literals(&nodes, Some(&schema))
            .expect("schema literals valid");
        assert_eq!(
            literals,
            BTreeSet::from([default, enum_a, enum_b, example]),
            "each distinct schema literal collected exactly once"
        );
    }

    #[test]
    fn schema_literal_dedupes_with_fixed_principal() {
        let agent = Uuid::new_v4();
        let nodes = vec![
            node("work", AssigneeRefType::FixedPrincipal, Some(agent), None),
            node(
                "pick",
                AssigneeRefType::InstanceInputPrincipal,
                None,
                Some("owner"),
            ),
        ];
        let schema = schema_property(
            "owner",
            serde_json::json!({ "type": "string", "default": agent.to_string() }),
        );
        let literals = collect_definition_publish_identity_literals(&nodes, Some(&schema))
            .expect("schema literals valid");
        assert_eq!(literals, BTreeSet::from([agent]));
    }

    #[test]
    fn non_uuid_string_literal_is_rejected_naming_key_and_position() {
        let nodes = vec![node(
            "work",
            AssigneeRefType::InstanceInputPrincipal,
            None,
            Some("owner"),
        )];
        let schema = schema_property(
            "owner",
            serde_json::json!({ "type": "string", "default": "display name" }),
        );
        let errors = collect_definition_publish_identity_literals(&nodes, Some(&schema))
            .expect_err("non-UUID identity literal must be rejected");
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].code, "INSTANCE_INPUT_LITERAL_NOT_UUID");
        assert!(
            errors[0].message.contains("'owner'") && errors[0].message.contains("default"),
            "error names the property path: {}",
            errors[0].message
        );
    }

    #[test]
    fn non_string_literal_is_rejected() {
        let nodes = vec![node(
            "work",
            AssigneeRefType::InstanceInputPrincipal,
            None,
            Some("owner"),
        )];
        let schema = schema_property("owner", serde_json::json!({ "type": "string", "default": 42 }));
        let errors = collect_definition_publish_identity_literals(&nodes, Some(&schema))
            .expect_err("non-string identity literal must be rejected");
        assert_eq!(errors[0].code, "INSTANCE_INPUT_LITERAL_NOT_STRING");
    }

    #[test]
    fn enum_positions_name_the_index() {
        let nodes = vec![node(
            "work",
            AssigneeRefType::InstanceInputPrincipal,
            None,
            Some("owner"),
        )];
        let good = Uuid::new_v4();
        let schema = schema_property(
            "owner",
            serde_json::json!({
                "type": "string",
                "enum": [good.to_string(), "agent-123"],
            }),
        );
        let errors = collect_definition_publish_identity_literals(&nodes, Some(&schema))
            .expect_err("invalid enum entry must be rejected");
        assert_eq!(errors.len(), 1);
        assert!(
            errors[0].message.contains("enum[1]") && errors[0].message.contains("agent-123"),
            "error names the keyword position: {}",
            errors[0].message
        );
    }

    #[test]
    fn missing_property_or_schema_is_skipped_not_an_error() {
        let nodes = vec![node(
            "work",
            AssigneeRefType::InstanceInputPrincipal,
            None,
            Some("absent_key"),
        )];
        // Property absent for the key.
        let schema = schema_property(
            "other",
            serde_json::json!({ "type": "string", "default": "junk" }),
        );
        let literals = collect_definition_publish_identity_literals(&nodes, Some(&schema))
            .expect("absent property contributes nothing");
        assert!(literals.is_empty());

        // No schema at all (IIP schema coverage is enforced elsewhere).
        let nodes = vec![node(
            "work",
            AssigneeRefType::InstanceInputPrincipal,
            None,
            Some("owner"),
        )];
        let literals =
            collect_definition_publish_identity_literals(&nodes, None).expect("no schema");
        assert!(literals.is_empty());

        // Fixed principals still collected alongside.
        let fixed = Uuid::new_v4();
        let nodes = vec![
            node("a", AssigneeRefType::FixedPrincipal, Some(fixed), None),
            node(
                "w",
                AssigneeRefType::InstanceInputPrincipal,
                None,
                Some("absent_key"),
            ),
        ];
        let literals =
            collect_definition_publish_identity_literals(&nodes, None).expect("collected");
        assert_eq!(literals, BTreeSet::from([fixed]));
    }
}
