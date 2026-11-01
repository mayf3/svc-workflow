//! Endpoint-only action selection in the original authorized read snapshot.
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::application::workflow_instance::authorized_actions::*;
use crate::application::workflow_instance::query_types::*;
use crate::domain::workflow_instance::assistance::AssistanceError;
use crate::domain::workflow_instance::errors::{
    ArchiveWorkflowInstanceError as ArchiveError, CancelWorkflowInstanceError as CancelError,
    ExecuteWorkflowTransitionError as TransitionError,
};

use super::query_rows::QueryBaseRow;
use super::{
    activation_facts, archive_transaction, assistance_transaction, cancel_transaction,
    query_detail, query_visibility, transition_checks, transition_validation,
};

fn internal(detail: impl ToString) -> WorkflowQueryError {
    WorkflowQueryError::InternalConsistency(detail.to_string())
}

fn transition_allowed(result: Result<(), TransitionError>) -> Result<bool, WorkflowQueryError> {
    match result {
        Ok(()) => Ok(true),
        Err(TransitionError::StorageError(s)) => Err(WorkflowQueryError::StorageError(s)),
        Err(TransitionError::InternalConsistency(s)) => Err(internal(s)),
        Err(
            TransitionError::PrincipalNotAssignee
            | TransitionError::AssistanceOpen
            | TransitionError::SourceNodeTerminal
            | TransitionError::DefinitionVersionRevoked
            | TransitionError::DefinitionVersionDraft
            | TransitionError::TransitionNotApplicable(_)
            | TransitionError::ReturnPolicyExhausted { .. }
            | TransitionError::AssigneeResolutionFailed(_)
            | TransitionError::SubmissionValidationFailed(_)
            | TransitionError::ContextValidationFailed(_),
        ) => Ok(false),
        Err(e) => Err(internal(e)),
    }
}

fn assistance_allowed(result: Result<(), AssistanceError>) -> Result<bool, WorkflowQueryError> {
    match result {
        Ok(()) => Ok(true),
        Err(AssistanceError::StorageError(s)) => Err(WorkflowQueryError::StorageError(s)),
        Err(AssistanceError::InternalConsistency(s)) => Err(internal(s)),
        Err(
            AssistanceError::PrincipalNotAssignee
            | AssistanceError::SourceNodeTerminal
            | AssistanceError::InstanceCancelled
            | AssistanceError::InstanceArchived
            | AssistanceError::DomainOwnerMissing
            | AssistanceError::NotDomainOwner
            | AssistanceError::AssistanceAlreadyOpen
            | AssistanceError::AssistanceCaseNotFoundOrNotVisible
            | AssistanceError::AssistanceStatusConflict
            | AssistanceError::GovernedOperationRequired
            | AssistanceError::InvalidPayload(_),
        ) => Ok(false),
        Err(e) => Err(internal(e)),
    }
}

fn cancel_allowed(result: Result<(), CancelError>) -> Result<bool, WorkflowQueryError> {
    match result {
        Ok(()) => Ok(true),
        Err(CancelError::StorageError(s)) => Err(WorkflowQueryError::StorageError(s)),
        Err(CancelError::InternalConsistency(s)) => Err(internal(s)),
        Err(
            CancelError::NotDomainOwner
            | CancelError::SourceNodeTerminal
            | CancelError::AlreadyCancelled
            | CancelError::InstanceArchived,
        ) => Ok(false),
        Err(e) => Err(internal(e)),
    }
}

fn archive_allowed(result: Result<(), ArchiveError>) -> Result<bool, WorkflowQueryError> {
    match result {
        Ok(()) => Ok(true),
        Err(ArchiveError::StorageError(s)) => Err(WorkflowQueryError::StorageError(s)),
        Err(ArchiveError::InternalConsistency(s)) => Err(internal(s)),
        Err(
            ArchiveError::NotDomainOwner
            | ArchiveError::InstanceNotTerminal
            | ArchiveError::AlreadyArchived
            | ArchiveError::ActiveActivationExists,
        ) => Ok(false),
        Err(e) => Err(internal(e)),
    }
}

async fn transition_identity_state(
    tx: &mut Transaction<'_, Postgres>,
    base: &QueryBaseRow,
    actor: Uuid,
    transition_id: Uuid,
    max_returns: u32,
) -> Result<(), TransitionError> {
    use transition_validation as v;
    let instance = v::read_instance(tx, base.workflow_instance_id, false).await?;
    transition_checks::validate_not_cancelled(instance.cancelled)?;
    let visit = v::read_current_visit(
        tx,
        base.workflow_instance_id,
        instance.current_node_visit_id,
    )
    .await?;
    transition_checks::validate_nonterminal_source(visit.node_type_enum())?;
    if let Some(error) = v::validate_principal_enabled(tx, actor).await? {
        return Err(error);
    }
    let effective = query_visibility::actor_is_effective_current_assignee(
        tx,
        visit.assignee_principal_id,
        actor,
    )
    .await
    .map_err(|e| TransitionError::StorageError(e.to_string()))?;
    transition_checks::validate_assignee(effective)?;
    if assistance_transaction::has_open_assistance(tx, visit.node_visit_id)
        .await
        .map_err(|e| TransitionError::StorageError(e.to_string()))?
    {
        return Err(TransitionError::AssistanceOpen);
    }
    v::read_definition_version_status(tx, instance.definition_version_id, false).await?;
    let model = v::read_semantic_model_version(tx, instance.definition_version_id).await?;
    let source = v::read_source_node(tx, visit.node_id, instance.definition_version_id).await?;
    let transition = v::read_transition(tx, transition_id, instance.definition_version_id).await?;
    transition_checks::validate_source_and_effect(&transition, &visit, &source, model == 2)?;
    let target = v::read_target_node(
        tx,
        transition.target_node_id,
        instance.definition_version_id,
    )
    .await?;
    transition_checks::validate_target(
        &transition.transition_effect,
        &target,
        &source,
        model == 2,
    )?;
    transition_checks::return_budget(
        tx,
        base.workflow_instance_id,
        transition_id,
        &transition.transition_effect,
        max_returns,
    )
    .await?;
    transition_checks::validate_activation_node_types(&source, &target, model == 3)?;
    super::owner_obligation::guards::evaluate_transition_identity_state(
        tx,
        actor,
        base.workflow_instance_id,
        target.node_id,
        &transition.transition_effect,
    )
    .await
    .map_err(|e| {
        if e.status >= 500 {
            TransitionError::StorageError(e.code)
        } else {
            TransitionError::SubmissionValidationFailed(e.code)
        }
    })?;
    let target_assignee = v::resolve_assignee(
        tx,
        &target,
        &instance,
        base.domain_id,
        base.context_payload.as_ref(),
    )
    .await?;
    if model == 1 {
        if let Some(id) = target_assignee {
            v::verify_principal_enabled_for_transition(tx, id).await?;
        }
    }
    if model == 3 {
        if let Some(id) = target_assignee {
            activation_facts::validate_owner_is_human_or_agent(tx, id)
                .await
                .map_err(|e| TransitionError::StorageError(e.to_string()))?
                .map_err(TransitionError::AssigneeResolutionFailed)?;
        }
        if !activation_facts::validate_current_activation(
            tx,
            base.workflow_instance_id,
            visit.node_visit_id,
        )
        .await
        .map_err(|e| TransitionError::StorageError(e.to_string()))?
        {
            return Err(TransitionError::InternalConsistency(
                "VISIT_ACTIVATION_V1 source visit has no active activation to close".to_string(),
            ));
        }
    }
    Ok(())
}

async fn collect(
    tx: &mut Transaction<'_, Postgres>,
    base: &QueryBaseRow,
    actor: Uuid,
    full: &FullWorkflowInstanceDetail,
    max_returns: u32,
) -> Result<Vec<AuthorizedAction>, WorkflowQueryError> {
    use RequestValidation::RequiredAtExecution as Required;
    let mut actions = Vec::new();
    // Enumerate every actual edge. The legacy boolean is deliberately not an
    // input: it remains an unchanged diagnostic on the shared full detail.
    let mut transitions: Vec<_> = full.outgoing_transitions.iter().collect();
    transitions.sort_by_key(|t| t.transition_id);
    for t in transitions {
        if transition_allowed(
            transition_identity_state(tx, base, actor, t.transition_id, max_returns).await,
        )? {
            actions.push(AuthorizedAction::Transition {
                transition_id: t.transition_id,
                transition_effect: t.transition_effect.clone(),
                request_validation: Required,
            });
        }
    }
    if assistance_allowed(
        assistance_transaction::evaluate_request_identity_state(
            tx,
            actor,
            base.workflow_instance_id,
        )
        .await,
    )? {
        actions.push(AuthorizedAction::RequestAssistance {
            request_validation: Required,
        });
    }
    let cases: Vec<Uuid> = sqlx::query_scalar(
        "SELECT assistance_case_id FROM workflow_assistance_cases WHERE workflow_instance_id=$1 AND node_visit_id=$2 ORDER BY assistance_case_id",
    ).bind(base.workflow_instance_id).bind(full.current_node_visit_id).fetch_all(&mut **tx).await
        .map_err(query_visibility::map_storage)?;
    for (kind, resolve) in [
        (assistance_transaction::OwnerActionKind::Escalate, false),
        (assistance_transaction::OwnerActionKind::Resolve, true),
    ] {
        for &id in &cases {
            if assistance_allowed(
                assistance_transaction::evaluate_owner_identity_state(
                    tx,
                    actor,
                    base.workflow_instance_id,
                    full.current_node_visit_id,
                    id,
                    kind,
                )
                .await,
            )? {
                actions.push(if resolve {
                    AuthorizedAction::ResolveAssistance {
                        assistance_case_id: id,
                        request_validation: Required,
                    }
                } else {
                    AuthorizedAction::EscalateAssistance {
                        assistance_case_id: id,
                        request_validation: Required,
                    }
                });
            }
        }
    }
    if cancel_allowed(
        cancel_transaction::evaluate_identity_state(tx, actor, base.workflow_instance_id).await,
    )? {
        actions.push(AuthorizedAction::Cancel {
            request_validation: Required,
        });
    }
    if archive_allowed(
        archive_transaction::evaluate_identity_state(tx, actor, base.workflow_instance_id).await,
    )? {
        actions.push(AuthorizedAction::Archive {
            request_validation: Required,
        });
    }
    Ok(actions)
}

pub(crate) async fn get(
    pool: &sqlx::PgPool,
    query: GetWorkflowInstanceDetail,
    credential: ActionCredential,
) -> Result<AuthorizedInstanceDetail, WorkflowQueryError> {
    let mut snapshot = query_visibility::authorized_snapshot(
        pool,
        query.actor_principal_id,
        query.workflow_instance_id,
        "GetWorkflowInstanceDetail",
    )
    .await?;
    let detail = if snapshot.visibility.is_full() {
        let full =
            query_detail::build_full(&mut snapshot.tx, &snapshot.base, query.actor_principal_id)
                .await?;
        let node_type = full.current_visit.node.node_type.clone();
        if !matches!(node_type.as_str(), "DRAFT" | "NORMAL" | "TASK" | "TERMINAL") {
            return Err(internal("unknown current node type"));
        }
        let model = transition_validation::read_semantic_model_version(
            &mut snapshot.tx,
            snapshot.base.definition_version_id,
        )
        .await
        .map_err(|e| match e {
            TransitionError::StorageError(s) => WorkflowQueryError::StorageError(s),
            e => internal(e),
        })?;
        let supported = matches!(model, 1 | 3);
        let mut actions = if supported {
            collect(
                &mut snapshot.tx,
                &snapshot.base,
                query.actor_principal_id,
                &full,
                credential.max_returns_per_edge,
            )
            .await?
        } else {
            Vec::new()
        };
        let status = if !credential.execute_scope {
            AuthorizedActionsStatus::MissingExecuteScope
        } else if !credential.write_gate_open {
            AuthorizedActionsStatus::WriteGateClosed
        } else if !supported {
            AuthorizedActionsStatus::UnsupportedModel
        } else if actions.is_empty() {
            AuthorizedActionsStatus::NoneForActor
        } else {
            AuthorizedActionsStatus::Available
        };
        if !matches!(status, AuthorizedActionsStatus::Available) {
            actions.clear();
        }
        AuthorizedInstanceDetail::Full(Box::new(InstanceDetailWithAuthorizedActionsV1 {
            full,
            authorized_actions_scope: "INSTANCE_TASK_AND_GOVERNANCE_V1",
            authorized_actions_status: status,
            authorized_actions: actions,
            current_task_type: CurrentTaskType {
                source: "CURRENT_NODE_TYPE",
                node_type,
                business_task_type: None,
            },
        }))
    } else {
        AuthorizedInstanceDetail::HistoricalParticipant(ParticipantWorkflowInstanceDetail {
            instance: snapshot
                .base
                .participant_summary()
                .ok_or_else(|| internal("incomplete instance summary"))?,
        })
    };
    snapshot
        .tx
        .commit()
        .await
        .map_err(query_visibility::map_storage)?;
    Ok(detail)
}
