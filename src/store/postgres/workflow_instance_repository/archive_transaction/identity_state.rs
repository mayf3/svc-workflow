//! Archive's original identity/state predicates, shared with read evaluation.
use super::*;

type InstanceFacts = (
    Uuid,
    Uuid,
    i32,
    bool,
    Option<String>,
    bool,
    Option<chrono::DateTime<chrono::Utc>>,
);

pub(super) async fn load_instance(
    tx: &mut Transaction<'_, Postgres>,
    instance_uuid: Uuid,
    lock: bool,
) -> Result<InstanceFacts, ArchiveWorkflowInstanceError> {
    let query = format!(
        "SELECT wi.workflow_instance_id, wi.domain_id,
                    wi.workflow_state_version,
                    wi.cancelled,
                    nd.node_type::text,
                    COALESCE(wi.archived_at IS NOT NULL, FALSE),
                    wi.archived_at
             FROM workflow_instances wi
             LEFT JOIN workflow_node_visits nv ON nv.node_visit_id = wi.current_node_visit_id
             LEFT JOIN workflow_node_definitions nd ON nd.node_id = nv.node_id
             WHERE wi.workflow_instance_id = $1{}",
        if lock { " FOR UPDATE OF wi" } else { "" },
    );
    sqlx::query_as(&query)
        .bind(instance_uuid)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?
        .ok_or(ArchiveWorkflowInstanceError::InstanceNotFound)
}

pub(super) async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    domain: Uuid,
    actor: Uuid,
) -> Result<(), ArchiveWorkflowInstanceError> {
    if !super::super::cancel_transaction::governance_role(tx, domain, actor)
        .await
        .map_err(storage)?
    {
        return Err(ArchiveWorkflowInstanceError::NotDomainOwner);
    }
    Ok(())
}

pub(super) fn lifecycle(
    is_cancelled: bool,
    node_type: Option<&str>,
    is_archived: bool,
) -> Result<(), ArchiveWorkflowInstanceError> {
    let is_terminal = is_cancelled || node_type == Some("TERMINAL");
    if !is_terminal {
        return Err(ArchiveWorkflowInstanceError::InstanceNotTerminal);
    }

    // Step 6a: Archive is a one-shot lifecycle change, symmetric with cancel's
    // InstanceArchived guard. Under the FOR UPDATE row lock, an already
    // archived instance is rejected: a new idempotency key must not overwrite
    // archived_at/archive_reason, append a second archive event, or grow
    // workflow_state_version. The error path rolls back the whole transaction,
    // so no success receipt is created.
    if is_archived {
        return Err(ArchiveWorkflowInstanceError::AlreadyArchived);
    }

    Ok(())
}

pub(super) async fn activation_state(
    tx: &mut Transaction<'_, Postgres>,
    instance_uuid: Uuid,
) -> Result<(), ArchiveWorkflowInstanceError> {
    let (semantic_model_version, current_visit_id): (i16, Option<Uuid>) = sqlx::query_as(
        "SELECT wdv.semantic_model_version, wi.current_node_visit_id \
         FROM workflow_instances wi \
         JOIN workflow_definition_versions wdv \
           ON wdv.definition_version_id = wi.definition_version_id \
         WHERE wi.workflow_instance_id = $1",
    )
    .bind(instance_uuid)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;
    if semantic_model_version == 3 {
        if let Some(visit_id) = current_visit_id {
            let has_active =
                super::super::activation_facts::visit_has_active_activation_tx(tx, visit_id)
                    .await
                    .map_err(storage)?;
            if has_active {
                return Err(ArchiveWorkflowInstanceError::ActiveActivationExists);
            }
        }
    }

    Ok(())
}

pub(crate) async fn evaluate_identity_state(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    instance: Uuid,
) -> Result<(), ArchiveWorkflowInstanceError> {
    let (_, domain, _, cancelled, node_type, archived, _) =
        load_instance(tx, instance, false).await?;
    authorize(tx, domain, actor).await?;
    let enabled = sqlx::query_scalar("SELECT enabled FROM principals WHERE principal_id=$1")
        .bind(actor)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?;
    if !super::super::cancel_transaction::actor_enabled(enabled) {
        return Err(ArchiveWorkflowInstanceError::PrincipalDisabled);
    }
    lifecycle(cancelled, node_type.as_deref(), archived)?;
    activation_state(tx, instance).await
}
