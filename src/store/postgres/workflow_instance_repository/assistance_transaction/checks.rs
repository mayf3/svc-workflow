//! Shared Assistance identity/state checks. Read evaluation never claims a command.
use super::*;

#[derive(Clone, Copy)]
pub(crate) enum OwnerActionKind {
    Escalate,
    Resolve,
}

impl From<&write::OwnerAction> for OwnerActionKind {
    fn from(action: &write::OwnerAction) -> Self {
        match action {
            write::OwnerAction::Escalate(_) => Self::Escalate,
            write::OwnerAction::Resolve(_) => Self::Resolve,
        }
    }
}

pub(super) fn request_lifecycle(instance: &LockedInstance) -> Result<(), AssistanceError> {
    if instance.cancelled {
        return Err(AssistanceError::InstanceCancelled);
    }
    if instance.archived_at.is_some() {
        return Err(AssistanceError::InstanceArchived);
    }
    if instance.node_type.as_deref() == Some("TERMINAL") {
        return Err(AssistanceError::SourceNodeTerminal);
    }
    Ok(())
}

pub(super) fn request_assignee(
    instance: &LockedInstance,
    actor: Uuid,
) -> Result<(), AssistanceError> {
    if instance.assignee_principal_id != Some(actor) {
        return Err(AssistanceError::PrincipalNotAssignee);
    }
    Ok(())
}

pub(super) fn request_open_case(open_exists: bool) -> Result<(), AssistanceError> {
    if open_exists {
        return Err(AssistanceError::AssistanceAlreadyOpen);
    }
    Ok(())
}

pub(super) fn owner_actor(owner: Uuid, actor: Uuid) -> Result<(), AssistanceError> {
    if owner != actor {
        return Err(AssistanceError::NotDomainOwner);
    }
    Ok(())
}

#[derive(Debug, FromRow)]
pub(super) struct CaseFacts {
    pub(super) assistance_case_id: Uuid,
    pub(super) workflow_instance_id: Uuid,
    pub(super) node_visit_id: Uuid,
    pub(super) status: String,
    pub(super) created_at: DateTime<Utc>,
}

pub(super) async fn load_case(
    tx: &mut Transaction<'_, Postgres>,
    case: Uuid,
    lock: bool,
) -> Result<CaseFacts, AssistanceError> {
    let query = format!(
        "SELECT assistance_case_id, workflow_instance_id, node_visit_id, status, created_at
         FROM workflow_assistance_cases WHERE assistance_case_id = $1{}",
        if lock { " FOR UPDATE" } else { "" },
    );
    sqlx::query_as(&query)
        .bind(case)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(AssistanceError::AssistanceCaseNotFoundOrNotVisible)
}

pub(super) fn owner_case(
    action: OwnerActionKind,
    instance: Uuid,
    current_visit: Option<Uuid>,
    case: &CaseFacts,
) -> Result<(), AssistanceError> {
    if case.workflow_instance_id != instance || current_visit != Some(case.node_visit_id) {
        return Err(AssistanceError::AssistanceStatusConflict);
    }
    let status_ok = match action {
        OwnerActionKind::Escalate => case.status == "OWNER_PENDING",
        OwnerActionKind::Resolve => {
            case.status == "OWNER_PENDING" || case.status == "HUMAN_REQUIRED"
        }
    };
    if !status_ok {
        return Err(AssistanceError::AssistanceStatusConflict);
    }
    Ok(())
}

pub(crate) async fn evaluate_request_identity_state(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    instance: Uuid,
) -> Result<(), AssistanceError> {
    let instance = load_instance(tx, instance, false).await?;
    let visit = instance
        .current_node_visit_id
        .ok_or(AssistanceError::CurrentVisitNotFound)?;
    request_lifecycle(&instance)?;
    validate_actor_enabled(tx, actor).await?;
    request_assignee(&instance, actor)?;
    effective_owner_with_lock(tx, instance.domain_id, false).await?;
    request_open_case(has_open_assistance(tx, visit).await?)
}

pub(crate) async fn evaluate_owner_identity_state(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    instance: Uuid,
    current_visit: Uuid,
    case: Uuid,
    action: OwnerActionKind,
) -> Result<(), AssistanceError> {
    super::super::owner_obligation::intents::require_effect_for_command(tx, case, None).await?;
    let instance = load_instance(tx, instance, false).await?;
    validate_actor_enabled(tx, actor).await?;
    owner_actor(
        effective_owner_with_lock(tx, instance.domain_id, false).await?,
        actor,
    )?;
    let case_facts = load_case(tx, case, false).await?;
    if instance.current_node_visit_id != Some(current_visit) {
        return Err(AssistanceError::AssistanceStatusConflict);
    }
    owner_case(
        action,
        instance.workflow_instance_id,
        instance.current_node_visit_id,
        &case_facts,
    )?;
    if matches!(action, OwnerActionKind::Resolve) {
        governance_child::validate_resolve_identity_state(tx, case).await?;
    }
    Ok(())
}
