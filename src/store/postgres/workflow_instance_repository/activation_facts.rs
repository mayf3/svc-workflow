//! Canonical activation fact helpers for VISIT_ACTIVATION_V1 (model 3).
//!
//! Implements the storage mechanics of SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
//! (accepted v0.4.0 §5.7-5.9):
//!
//! - exactly one immutable `workflow_activations` row per Node Visit,
//!   created atomically with the Visit entry transaction;
//! - immutable `workflow_activation_closures` rows written in the same
//!   transaction as the command that ends current work;
//! - immutable `workflow_dispatch_eligibility_events` rows as the only
//!   writers of later `nextEligibleAt` values;
//! - owner type validation: VISIT_ACTIVATION_V1 TASK owners must resolve to
//!   an enabled canonical HUMAN or AGENT Principal.

use uuid::Uuid;

use crate::domain::enums::{ActivationKind, PrincipalType};

pub(crate) const CLOSURE_REASON_TRANSITIONED: &str = "TRANSITIONED";
pub(crate) const CLOSURE_REASON_CANCELLED: &str = "CANCELLED";
pub(crate) const CLOSURE_REASON_ADMIN_MOVE: &str = "ADMIN_MOVE";
pub(crate) const CLOSURE_REASON_ADMIN_TERMINATE: &str = "ADMIN_TERMINATE";

pub(crate) const CAUSE_CLASS_WAKE: &str = "WAKE";

/// Insert the canonical activation for a newly created Visit.
///
/// `activation_at` and the initial `nextEligibleAt` are the same
/// transaction timestamp: `now()` is constant inside a transaction, so the
/// initial wait instant is authored server-side in the activation
/// transaction itself (CTR-ARCH-012) with no client input and no
/// post-commit fill.
pub(crate) async fn insert_activation(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    activation_id: Uuid,
    workflow_instance_id: Uuid,
    node_visit_id: Uuid,
    kind: ActivationKind,
    owner_principal_id: Uuid,
    command_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO workflow_activations
            (activation_id, workflow_instance_id, node_visit_id,
             activation_kind, owner_principal_id,
             activation_at, initial_next_eligible_at, command_id)
        VALUES ($1, $2, $3, $4::activation_kind, $5, NOW(),
                CASE WHEN $4::activation_kind = 'DISPATCH_INTENT'
                     THEN NOW() ELSE NULL END,
                $6)
        "#,
    )
    .bind(activation_id)
    .bind(workflow_instance_id)
    .bind(node_visit_id)
    .bind(kind.to_string())
    .bind(owner_principal_id)
    .bind(command_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Close the current activation of `node_visit_id`, if one exists and is
/// still active. Returns `true` when a closure row was written.
///
/// Legacy Instances never have activations, so legacy call sites simply do
/// not call this; VISIT_ACTIVATION_V1 call sites use
/// [`close_activation_by_visit_required`], which fails closed when the
/// expected activation is missing or already closed.
pub(crate) async fn close_activation_by_visit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workflow_instance_id: Uuid,
    node_visit_id: Uuid,
    closure_reason: &str,
    command_id: Uuid,
    event_id: Option<Uuid>,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        r#"
        INSERT INTO workflow_activation_closures
            (activation_id, closed_at, closure_reason, command_id, event_id)
        SELECT a.activation_id, NOW(), $3, $4, $5
          FROM workflow_activations a
         WHERE a.workflow_instance_id = $1
           AND a.node_visit_id = $2
           AND NOT EXISTS (
               SELECT 1 FROM workflow_activation_closures c
                WHERE c.activation_id = a.activation_id
           )
        "#,
    )
    .bind(workflow_instance_id)
    .bind(node_visit_id)
    .bind(closure_reason)
    .bind(command_id)
    .bind(event_id)
    .execute(&mut **tx)
    .await?;
    Ok(result.rows_affected() == 1)
}

/// VISIT_ACTIVATION_V1 variant: the source activation MUST exist and be
/// active. A missing or already-closed activation is invariant drift and
/// fails closed (the caller decides whether that is a deterministic
/// command failure or an internal consistency error).
pub(crate) async fn close_activation_by_visit_required(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workflow_instance_id: Uuid,
    node_visit_id: Uuid,
    closure_reason: &str,
    command_id: Uuid,
    event_id: Option<Uuid>,
) -> Result<bool, sqlx::Error> {
    close_activation_by_visit(
        tx,
        workflow_instance_id,
        node_visit_id,
        closure_reason,
        command_id,
        event_id,
    )
    .await
}

/// Archive guard: true when the Visit still has an ACTIVE activation
/// (transaction variant — runs under the caller's Instance lock).
pub(crate) async fn visit_has_active_activation_tx(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node_visit_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let row: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT 1::bigint
          FROM workflow_activations a
         WHERE a.node_visit_id = $1
           AND NOT EXISTS (
               SELECT 1 FROM workflow_activation_closures c
                WHERE c.activation_id = a.activation_id
           )
         LIMIT 1
        "#,
    )
    .bind(node_visit_id)
    .fetch_optional(&mut **tx)
    .await?;
    Ok(row.is_some())
}

/// Append one immutable eligibility fact. The only lawful writers of later
/// `nextEligibleAt` values are wake (server now) and the bounded
/// Scheduler-defer command of a later authority.
pub(crate) async fn insert_eligibility_event(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    eligibility_event_id: Uuid,
    activation_id: Uuid,
    previous_next_eligible_at: chrono::DateTime<chrono::Utc>,
    new_next_eligible_at: chrono::DateTime<chrono::Utc>,
    cause_class: &str,
    command_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO workflow_dispatch_eligibility_events
            (eligibility_event_id, activation_id,
             previous_next_eligible_at, new_next_eligible_at,
             cause_class, command_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        "#,
    )
    .bind(eligibility_event_id)
    .bind(activation_id)
    .bind(previous_next_eligible_at)
    .bind(new_next_eligible_at)
    .bind(cause_class)
    .bind(command_id)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Resolve the activation of a Visit for the wake command: the activation
/// must exist, belong to the Visit, and be a DISPATCH_INTENT. Returns the
/// activation id, its closure time (None = active), and its current
/// `nextEligibleAt` (latest eligibility fact, else the initial value).
pub(crate) async fn find_dispatch_activation_for_wake(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workflow_instance_id: Uuid,
    node_visit_id: Uuid,
) -> Result<Option<WakeActivationRow>, sqlx::Error> {
    let row: Option<(
        Uuid,
        Option<chrono::DateTime<chrono::Utc>>,
        Option<chrono::DateTime<chrono::Utc>>,
    )> = sqlx::query_as(
        r#"
        SELECT a.activation_id,
               c.closed_at,
               COALESCE(
                   (SELECT e.new_next_eligible_at
                      FROM workflow_dispatch_eligibility_events e
                     WHERE e.activation_id = a.activation_id
                     ORDER BY e.created_at DESC, e.eligibility_event_id DESC
                     LIMIT 1),
                   a.initial_next_eligible_at)
          FROM workflow_activations a
          LEFT JOIN workflow_activation_closures c
            ON c.activation_id = a.activation_id
         WHERE a.workflow_instance_id = $1
           AND a.node_visit_id = $2
           AND a.activation_kind = 'DISPATCH_INTENT'
        "#,
    )
    .bind(workflow_instance_id)
    .bind(node_visit_id)
    .fetch_optional(&mut **tx)
    .await?;

    Ok(row.map(|(activation_id, closed_at, next)| WakeActivationRow {
        activation_id,
        closed_at,
        current_next_eligible_at: next,
    }))
}

/// Wake-facing projection of one activation.
pub(crate) struct WakeActivationRow {
    pub activation_id: Uuid,
    pub closed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub current_next_eligible_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// VISIT_ACTIVATION_V1 TASK owner validation: the resolved owner must be an
/// enabled canonical HUMAN or AGENT Principal. SERVICE (and any unknown
/// type) can never own new-model work.
///
/// Outer error = storage failure; inner Err = deterministic business
/// rejection reason (caller maps to its own error class).
pub(crate) async fn validate_owner_is_human_or_agent(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    owner_principal_id: Uuid,
) -> Result<Result<(), String>, sqlx::Error> {
    let row: Option<(String, bool)> = sqlx::query_as(
        "SELECT principal_type::text, enabled FROM principals WHERE principal_id = $1",
    )
    .bind(owner_principal_id)
    .fetch_optional(&mut **tx)
    .await?;

    let Some((principal_type, enabled)) = row else {
        // Missing principal row for a resolved owner is an internal
        // consistency failure, not a caller-classified error.
        return Ok(Err(
            "owner principal row missing for resolved VISIT_ACTIVATION_V1 owner".to_string()
        ));
    };

    let type_ok = principal_type == PrincipalType::HUMAN.to_string()
        || principal_type == PrincipalType::AGENT.to_string();
    if !type_ok || !enabled {
        return Ok(Err(format!(
            "VISIT_ACTIVATION_V1 owner must be an enabled HUMAN or AGENT principal \
             (type={principal_type}, enabled={enabled})"
        )));
    }
    Ok(Ok(()))
}

/// Activation consistency validation for REBUILD_PROJECTION on
/// VISIT_ACTIVATION_V1 Instances (CTR-VAI-012): exactly-one activation per
/// Visit, every DISPATCH_INTENT activation has an initial wait timestamp,
/// and no Visit has two activations (DB-unique already, re-checked here for
/// rebuild evidence).
pub(crate) async fn validate_instance_activation_consistency(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    workflow_instance_id: Uuid,
) -> Result<(), String> {
    // Visits of this instance without exactly one activation.
    let drift: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT 1::bigint
          FROM workflow_node_visits v
         WHERE v.workflow_instance_id = $1
           AND (SELECT COUNT(*) FROM workflow_activations a
                 WHERE a.node_visit_id = v.node_visit_id) <> 1
         LIMIT 1
        "#,
    )
    .bind(workflow_instance_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;
    if drift.is_some() {
        return Err(
            "activation cardinality drift: every VISIT_ACTIVATION_V1 Visit must have \
             exactly one activation"
                .to_string(),
        );
    }

    // DISPATCH_INTENT activations must carry an initial wait timestamp and
    // their eligibility facts must be ordered (new > previous per row).
    let bad_intent: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT 1::bigint
          FROM workflow_activations a
         WHERE a.workflow_instance_id = $1
           AND a.activation_kind = 'DISPATCH_INTENT'
           AND (a.initial_next_eligible_at IS NULL
                OR EXISTS (
                    SELECT 1 FROM workflow_dispatch_eligibility_events e
                     WHERE e.activation_id = a.activation_id
                       AND e.new_next_eligible_at <= e.previous_next_eligible_at
                ))
         LIMIT 1
        "#,
    )
    .bind(workflow_instance_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;
    if bad_intent.is_some() {
        return Err(
            "dispatch intent eligibility drift: missing initial nextEligibleAt or \
             non-progressing eligibility fact"
                .to_string(),
        );
    }

    // Closure rows must reference activations of this instance (FK already
    // guarantees the activation exists; verify same-instance linkage).
    let foreign_closure: Option<(i64,)> = sqlx::query_as(
        r#"
        SELECT 1::bigint
          FROM workflow_activation_closures c
          JOIN workflow_activations a ON a.activation_id = c.activation_id
         WHERE a.workflow_instance_id = $1
           AND EXISTS (
               SELECT 1 FROM workflow_node_visits v
                WHERE v.node_visit_id = a.node_visit_id
                  AND v.workflow_instance_id <> $1
           )
         LIMIT 1
        "#,
    )
    .bind(workflow_instance_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| e.to_string())?;
    if foreign_closure.is_some() {
        return Err("activation closure drift: cross-instance activation linkage".to_string());
    }

    Ok(())
}
