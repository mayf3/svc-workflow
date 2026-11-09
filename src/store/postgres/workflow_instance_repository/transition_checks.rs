//! Shared, side-effect-free transition selection checks (CTR-AA-003/004).
use super::transition_rows::*;
use crate::domain::enums::NodeType;
use crate::domain::workflow_instance::errors::ExecuteWorkflowTransitionError;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

// The writer persists deterministic failures at its original callsite; this
// module only returns its existing error and never claims a receipt or writes.
macro_rules! deterministic_failure {
    ($error:expr) => {
        return Err($error)
    };
}

pub(super) fn validate_source_and_effect(
    transition: &TransitionDefinitionRow,
    current_visit: &CurrentVisitFullRow,
    source_node: &SourceNodeRow,
    is_minimal: bool,
) -> Result<(), ExecuteWorkflowTransitionError> {
    // Validate transition source matches current node
    if transition.source_node_id != current_visit.node_id {
        deterministic_failure!(ExecuteWorkflowTransitionError::TransitionNotApplicable(
            "transition source node does not match current visit node".to_string(),
        ));
    }

    // Validate transition effect, dispatching on the semantic model:
    //  - Legacy (1): primary ADVANCE + orderIndex RETURN rules (unchanged)
    //  - Minimal (2): any legal outgoing ADVANCE (no primary), RETURN by graph
    //    ancestor (no orderIndex), TERMINATE forbidden (fail closed)
    let effect = transition.transition_effect.clone();
    match effect.as_str() {
        "ADVANCE" => {
            if !is_minimal {
                // Verify this is the primary ADVANCE transition
                let primary_id = source_node.primary_advance_transition_id.ok_or_else(|| {
                    ExecuteWorkflowTransitionError::InternalConsistency(
                        "source node has no primary advance transition".to_string(),
                    )
                })?;
                if transition.transition_id != primary_id {
                    deterministic_failure!(
                        ExecuteWorkflowTransitionError::TransitionNotApplicable(
                            "ADVANCE must use the primary advance transition".to_string(),
                        )
                    );
                }
            }
        }
        "RETURN" => {
            if !is_minimal {
                // Verify it's NOT the primary ADVANCE transition
                if let Some(primary_id) = source_node.primary_advance_transition_id {
                    if transition.transition_id == primary_id {
                        deterministic_failure!(
                            ExecuteWorkflowTransitionError::TransitionNotApplicable(
                                "RETURN transition must not be the primary advance".to_string(),
                            )
                        );
                    }
                }
            }
        }
        "TERMINATE" => {
            if is_minimal {
                deterministic_failure!(ExecuteWorkflowTransitionError::TransitionNotApplicable(
                    "TERMINATE is forbidden in Minimal (V2) semantics".to_string(),
                ));
            }
            // Verify it's NOT the primary ADVANCE transition
            if let Some(primary_id) = source_node.primary_advance_transition_id {
                if transition.transition_id == primary_id {
                    deterministic_failure!(
                        ExecuteWorkflowTransitionError::TransitionNotApplicable(
                            "TERMINATE transition must not be the primary advance".to_string(),
                        )
                    );
                }
            }
        }
        _ => {
            deterministic_failure!(ExecuteWorkflowTransitionError::TransitionNotApplicable(
                format!("unknown transition effect: {}", effect)
            ));
        }
    }

    Ok(())
}

pub(super) fn validate_target(
    effect: &str,
    target_node: &TargetNodeRow,
    source_node: &SourceNodeRow,
    is_minimal: bool,
) -> Result<(), ExecuteWorkflowTransitionError> {
    match effect {
        "ADVANCE" => {
            // ADVANCE allows any target: TASK continues, TERMINAL completes
            // (normal completion contract, shared by Legacy and Minimal).
        }
        "RETURN" => {
            // Target must be non-TERMINAL
            if target_node.node_type_enum() == NodeType::TERMINAL {
                deterministic_failure!(ExecuteWorkflowTransitionError::TransitionNotApplicable(
                    "RETURN target must not be a TERMINAL node".to_string(),
                ));
            }
            if !is_minimal {
                // Legacy: orderIndex ordering rule. Minimal (V2) relies on the
                // validator-guaranteed strict ADVANCE ancestor relation instead.
                if target_node.order_index >= source_node.order_index {
                    deterministic_failure!(
                        ExecuteWorkflowTransitionError::TransitionNotApplicable(
                            "RETURN target must have lower order_index than source".to_string(),
                        )
                    );
                }
            }
        }
        "TERMINATE" => {
            // Minimal is rejected earlier (v2 fail-closed); Legacy requires a
            // TERMINAL target.
            if target_node.node_type_enum() != NodeType::TERMINAL {
                deterministic_failure!(ExecuteWorkflowTransitionError::TransitionNotApplicable(
                    "TERMINATE target must be a TERMINAL node".to_string(),
                ));
            }
        }
        _ => {
            deterministic_failure!(ExecuteWorkflowTransitionError::TransitionNotApplicable(
                format!("unknown transition effect: {}", effect)
            ));
        }
    }

    Ok(())
}

pub(super) async fn return_budget(
    tx: &mut Transaction<'_, Postgres>,
    instance_uuid: Uuid,
    transition_id: Uuid,
    effect: &str,
    max_returns_per_edge: u32,
) -> Result<(i64, u32), ExecuteWorkflowTransitionError> {
    let managed_scope:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_managed_instances WHERE workflow_instance_id=$1)").bind(instance_uuid).fetch_one(&mut **tx).await.map_err(|e|ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;
    let max_returns_per_edge = if managed_scope {
        3
    } else {
        max_returns_per_edge
    };
    let prior_return_count: i64 = if effect == "RETURN" {
        let row: (i64,) = sqlx::query_as(
            // TransitionCommittedEventData serializes snake_case (no camelCase
            // rename on that struct) — match the canonical key.
            "SELECT COUNT(*) FROM workflow_events
              WHERE workflow_instance_id = $1
                AND transition_effect = 'RETURN'
                AND event_data->>'transition_definition_id' = $2",
        )
        .bind(instance_uuid)
        .bind(transition_id.to_string())
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| ExecuteWorkflowTransitionError::StorageError(e.to_string()))?;
        row.0
    } else {
        0
    };
    if effect == "RETURN" && prior_return_count >= max_returns_per_edge as i64 {
        deterministic_failure!(ExecuteWorkflowTransitionError::ReturnPolicyExhausted {
            limit: max_returns_per_edge as i32,
        });
    }

    Ok((prior_return_count, max_returns_per_edge))
}

pub(super) fn validate_activation_node_types(
    source_node: &SourceNodeRow,
    target_node: &TargetNodeRow,
    is_visit_activation: bool,
) -> Result<(), ExecuteWorkflowTransitionError> {
    if is_visit_activation {
        // New-model graphs contain exactly TASK | TERMINAL; anything else on
        // a v1 instance is invariant drift and must not transition.
        if source_node.node_type_enum() != NodeType::TASK {
            deterministic_failure!(ExecuteWorkflowTransitionError::InternalConsistency(
                "VISIT_ACTIVATION_V1 source node is not a TASK".to_string(),
            ));
        }
        let target_is_task_or_terminal = matches!(
            target_node.node_type_enum(),
            NodeType::TASK | NodeType::TERMINAL
        );
        if !target_is_task_or_terminal {
            deterministic_failure!(ExecuteWorkflowTransitionError::InternalConsistency(
                "VISIT_ACTIVATION_V1 target node is neither TASK nor TERMINAL".to_string(),
            ));
        }
        if target_node.assignee_ref_type_enum()
            == Some(crate::domain::enums::AssigneeRefType::InstanceInputPrincipal)
        {
            deterministic_failure!(ExecuteWorkflowTransitionError::AssigneeResolutionFailed(
                "VISIT_ACTIVATION_V1 target uses INSTANCE_INPUT_PRINCIPAL, which is not                  in the new-model owner reference set"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

pub(super) fn validate_not_cancelled(
    cancelled: bool,
) -> Result<(), ExecuteWorkflowTransitionError> {
    if cancelled {
        return Err(ExecuteWorkflowTransitionError::SourceNodeTerminal);
    }
    Ok(())
}

pub(super) fn validate_nonterminal_source(
    node_type: NodeType,
) -> Result<(), ExecuteWorkflowTransitionError> {
    if node_type == NodeType::TERMINAL {
        return Err(ExecuteWorkflowTransitionError::SourceNodeTerminal);
    }
    Ok(())
}

pub(super) fn validate_assignee(
    effective_assignee: bool,
) -> Result<(), ExecuteWorkflowTransitionError> {
    if !effective_assignee {
        return Err(ExecuteWorkflowTransitionError::PrincipalNotAssignee);
    }
    Ok(())
}
