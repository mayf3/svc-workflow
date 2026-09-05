//! Canonical work-eligibility classification for read projections.
//!
//! SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1: ONE derivation shared by
//! every Workflow read projection that answers "is this work actionable
//! now, or waiting for its Visit Activation timer?". Derived per current
//! Node Visit from existing Workflow + Visit Activation truth only —
//! no new table, no migration, no second state machine:
//!
//! - no `workflow_activations` row (pre-0023 legacy work) = ACTIONABLE_NOW
//! - open DISPATCH_INTENT activation whose effective `nextEligibleAt`
//!   (latest eligibility event's `new_next_eligible_at`, else
//!   `initial_next_eligible_at`) is at or before now = ACTIONABLE_NOW
//! - the same with an effective instant in the future = WAITING_FOR_TIME
//! - HUMAN_WORK_ITEM activations carry no timer = ACTIONABLE_NOW
//! - closed activations (transitioned/cancelled/admin-ended) = ACTIONABLE_NOW
//!
//! Legacy/current work without an activation row stays discoverable and is
//! classified explicitly rather than hidden behind Visit Activation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Read-side eligibility classification of the current work item.
///
/// Wire shape (adjacently tagged): `{"classification":"ACTIONABLE_NOW"}` or
/// `{"classification":"WAITING_FOR_TIME","nextEligibleAt":"<RFC3339>"}` —
/// the waiting case carries the effective instant so dispatchers know when
/// the SAME work turns actionable without a second call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "classification", content = "nextEligibleAt", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkEligibility {
    /// Dispatchable now: no activation record (legacy), a closed
    /// activation, a timerless HUMAN_WORK_ITEM activation, or a
    /// DISPATCH_INTENT activation whose effective instant is due.
    ActionableNow,
    /// The DISPATCH_INTENT activation holds an effective instant in the
    /// future; the SAME work becomes actionable when it is due.
    WaitingForTime(DateTime<Utc>),
}

/// Internal wide row: what the shared SQL lateral join returns per visit.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EligibilityFactRow {
    pub activation_kind: Option<String>,
    /// Non-null iff an open (unclosed) activation row exists.
    pub open_activation_id: Option<uuid::Uuid>,
    /// Effective nextEligibleAt for an open DISPATCH_INTENT activation.
    pub effective_next_eligible_at: Option<DateTime<Utc>>,
}

impl EligibilityFactRow {
    /// Classify one current visit from its activation facts. `now` is the
    /// authoritative server instant (transaction clock), never client input.
    pub fn classify(&self, now: DateTime<Utc>) -> WorkEligibility {
        match (self.open_activation_id, self.activation_kind.as_deref()) {
            // No activation row: pre-0023 legacy work remains actionable.
            (None, _) => WorkEligibility::ActionableNow,
            (Some(_), Some("DISPATCH_INTENT")) => match self.effective_next_eligible_at {
                Some(effective) if effective > now => {
                    WorkEligibility::WaitingForTime(effective)
                }
                _ => WorkEligibility::ActionableNow,
            },
            // Open HUMAN_WORK_ITEM activation: timerless by schema CHECK.
            (Some(_), _) => WorkEligibility::ActionableNow,
        }
    }
}

/// Shared SQL fragment: activation facts for the CURRENT visit of an
/// instance, derived only from existing Visit Activation tables. Join key
/// is the caller's current node-visit column pair; safe to LEFT JOIN into
/// both summary-list and detail queries.
pub const ELIGIBILITY_FACT_SELECT: &str = r#"
    a_open.activation_kind AS activation_kind,
    a_open.activation_id   AS open_activation_id,
    eff.effective_next_eligible_at AS effective_next_eligible_at
"#;

pub const ELIGIBILITY_FACT_JOINS: &str = r#"
    LEFT JOIN workflow_activations a_open
      ON a_open.node_visit_id = v.node_visit_id
     AND NOT EXISTS (
       SELECT 1 FROM workflow_activation_closures c
       WHERE c.activation_id = a_open.activation_id)
    LEFT JOIN LATERAL (
       SELECT e.new_next_eligible_at AS effective_next_eligible_at
       FROM workflow_dispatch_eligibility_events e
       WHERE e.activation_id = a_open.activation_id
       ORDER BY e.created_at DESC, e.eligibility_event_id DESC
       LIMIT 1) eff ON eff.effective_next_eligible_at IS NOT NULL
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn row(kind: Option<&str>, open: bool, effective: Option<DateTime<Utc>>) -> EligibilityFactRow {
        EligibilityFactRow {
            activation_kind: kind.map(str::to_string),
            open_activation_id: open.then(uuid::Uuid::new_v4),
            effective_next_eligible_at: effective,
        }
    }

    fn now() -> DateTime<Utc> {
        Utc::now()
    }

    #[test]
    fn legacy_no_activation_row_is_actionable_now() {
        assert_eq!(row(None, false, None).classify(now()), WorkEligibility::ActionableNow);
    }

    #[test]
    fn closed_activation_is_actionable_now() {
        // Closed activations are excluded by the open-activation join; from
        // the classifier's view a visit with only closed rows has no open row.
        assert_eq!(row(Some("DISPATCH_INTENT"), false, None).classify(now()), WorkEligibility::ActionableNow);
    }

    #[test]
    fn human_work_item_has_no_timer_and_is_actionable() {
        assert_eq!(row(Some("HUMAN_WORK_ITEM"), true, None).classify(now()), WorkEligibility::ActionableNow);
    }

    #[test]
    fn due_dispatch_intent_is_actionable_now() {
        let past = now() - chrono::Duration::seconds(1);
        assert_eq!(row(Some("DISPATCH_INTENT"), true, Some(past)).classify(now()), WorkEligibility::ActionableNow);
    }

    #[test]
    fn waiting_for_time_carries_future_instant() {
        let future = now() + chrono::Duration::hours(2);
        assert_eq!(
            row(Some("DISPATCH_INTENT"), true, Some(future)).classify(now()),
            WorkEligibility::WaitingForTime(future)
        );
    }

    #[test]
    fn due_at_exact_instant_is_actionable() {
        let now = now();
        assert_eq!(row(Some("DISPATCH_INTENT"), true, Some(now)).classify(now), WorkEligibility::ActionableNow);
    }

    #[test]
    fn no_effective_instant_with_open_dispatch_intent_is_actionable() {
        // initial_next_eligible_at is NOT NULL for DISPATCH_INTENT by schema
        // CHECK; a NULL here can only mean the lateral found no rows, which
        // cannot happen for an open DISPATCH_INTENT. Fail open to actionable
        // rather than inventing a waiting state from nothing.
        assert_eq!(row(Some("DISPATCH_INTENT"), true, None).classify(now()), WorkEligibility::ActionableNow);
    }

    #[test]
    fn wire_shape_round_trips_with_next_eligible_at_content() {
        // B-1 regression: the waiting case MUST serialize its content and
        // accept it back — unit variants silently drop serde content.
        let future = now() + chrono::Duration::hours(3);
        let waiting = row(Some("DISPATCH_INTENT"), true, Some(future)).classify(now());
        let json = serde_json::to_value(waiting).expect("serialize");
        assert_eq!(json["classification"], "WAITING_FOR_TIME");
        assert!(json.get("nextEligibleAt").is_some(), "nextEligibleAt content missing on wire: {json}");
        let back: WorkEligibility = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, waiting);
        // The actionable case stays a bare classification tag.
        let actionable = serde_json::to_value(WorkEligibility::ActionableNow).unwrap();
        assert_eq!(actionable, serde_json::json!({"classification": "ACTIONABLE_NOW"}));
        let back_actionable: WorkEligibility = serde_json::from_value(actionable).unwrap();
        assert_eq!(back_actionable, WorkEligibility::ActionableNow);
    }
}
