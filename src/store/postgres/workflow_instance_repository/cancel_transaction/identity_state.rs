//! Cancel's original identity/state predicates, shared with read evaluation.
use super::*;

type InstanceFacts = (
    Uuid,
    Uuid,
    Option<Uuid>,
    i32,
    bool,
    Option<String>,
    Option<String>,
    Option<chrono::DateTime<chrono::Utc>>,
);

pub(super) async fn load_instance(
    tx: &mut Transaction<'_, Postgres>,
    instance_uuid: Uuid,
    lock: bool,
) -> Result<InstanceFacts, CancelWorkflowInstanceError> {
    let query = format!(
        "SELECT wi.workflow_instance_id, wi.domain_id,
                    wi.current_node_visit_id, wi.workflow_state_version,
                    wi.cancelled,
                    nd.node_type::text,
                    nd.node_key,
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
        .ok_or(CancelWorkflowInstanceError::InstanceNotFound)
}

pub(crate) async fn governance_role(
    tx: &mut Transaction<'_, Postgres>,
    domain_id: Uuid,
    principal_uuid: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
           SELECT 1 FROM domain_role_bindings
           WHERE domain_id = $1 AND principal_id = $2
             AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE)
         OR EXISTS(
           SELECT 1 FROM global_role_bindings g
           JOIN principals p ON p.principal_id = g.principal_id
           WHERE g.principal_id = $2
             AND g.role_key = 'GLOBAL_WORKFLOW_COORDINATOR' AND g.enabled = TRUE
             AND p.enabled = TRUE)",
    )
    .bind(domain_id)
    .bind(principal_uuid)
    .fetch_one(&mut **tx)
    .await
}

pub(crate) fn actor_enabled(enabled: Option<bool>) -> bool {
    enabled == Some(true)
}

pub(super) fn lifecycle(
    is_cancelled: bool,
    current_node_type: Option<&str>,
    archived: bool,
) -> Result<(), CancelWorkflowInstanceError> {
    if is_cancelled {
        return Err(CancelWorkflowInstanceError::AlreadyCancelled);
    }
    if current_node_type == Some("TERMINAL") {
        return Err(CancelWorkflowInstanceError::SourceNodeTerminal);
    }
    if archived {
        return Err(CancelWorkflowInstanceError::InstanceArchived);
    }

    Ok(())
}

pub(super) async fn dangling_has_no_runtime_fact(
    tx: &mut Transaction<'_, Postgres>,
    instance_uuid: Uuid,
    current_node_visit_id: Option<Uuid>,
) -> Result<(), CancelWorkflowInstanceError> {
    if current_node_visit_id.is_none() {
        let open_runtime_fact: bool = sqlx::query_scalar(
            "SELECT EXISTS(
               SELECT 1 FROM workflow_activations a
               LEFT JOIN workflow_activation_closures c
                 ON c.activation_id = a.activation_id
               WHERE a.workflow_instance_id = $1
                 AND c.activation_id IS NULL)",
        )
        .bind(instance_uuid)
        .fetch_one(&mut **tx)
        .await
        .map_err(storage)?;
        if open_runtime_fact {
            return Err(CancelWorkflowInstanceError::InternalConsistency(
                "dangling instance has an open runtime fact — fail-closed, cancel refused"
                    .to_string(),
            ));
        }
    }

    Ok(())
}

pub(super) async fn semantic_model_version(
    tx: &mut Transaction<'_, Postgres>,
    instance_uuid: Uuid,
) -> Result<i16, CancelWorkflowInstanceError> {
    let (semantic_model_version,): (i16,) = sqlx::query_as(
        "SELECT wdv.semantic_model_version \
         FROM workflow_instances wi \
         JOIN workflow_definition_versions wdv \
           ON wdv.definition_version_id = wi.definition_version_id \
         WHERE wi.workflow_instance_id = $1",
    )
    .bind(instance_uuid)
    .fetch_one(&mut **tx)
    .await
    .map_err(storage)?;
    Ok(semantic_model_version)
}

pub(super) fn activation_closable(closed: bool) -> Result<(), CancelWorkflowInstanceError> {
    if !closed {
        return Err(CancelWorkflowInstanceError::InternalConsistency(
            "VISIT_ACTIVATION_V1 current visit has no active activation to close".to_string(),
        ));
    }
    Ok(())
}

pub(crate) async fn evaluate_identity_state(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    instance: Uuid,
) -> Result<(), CancelWorkflowInstanceError> {
    let (_, domain, visit, _, cancelled, node_type, _, archived) =
        load_instance(tx, instance, false).await?;
    authorize(tx, domain, actor, instance).await?;
    let enabled = sqlx::query_scalar("SELECT enabled FROM principals WHERE principal_id=$1")
        .bind(actor)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?;
    if !actor_enabled(enabled) {
        return Err(CancelWorkflowInstanceError::PrincipalDisabled);
    }
    lifecycle(cancelled, node_type.as_deref(), archived.is_some())?;
    dangling_has_no_runtime_fact(tx, instance, visit).await?;
    let model = semantic_model_version(tx, instance).await?;
    if let Some(visit) = activation_visit(model, visit) {
        activation_closable(
            super::super::activation_facts::validate_current_activation(tx, instance, visit)
                .await
                .map_err(storage)?,
        )?;
    }
    Ok(())
}

pub(super) async fn authorize(
    tx: &mut Transaction<'_, Postgres>,
    domain: Uuid,
    actor: Uuid,
    instance: Uuid,
) -> Result<(), CancelWorkflowInstanceError> {
    if !governance_role(tx, domain, actor).await.map_err(storage)?
        && !super::super::owner_obligation::intents::cleanup_authorized(tx, instance, actor)
            .await
            .map_err(storage)?
    {
        return Err(CancelWorkflowInstanceError::NotDomainOwner);
    }
    Ok(())
}

pub(super) fn activation_visit(model: i16, visit: Option<Uuid>) -> Option<Uuid> {
    if model == 3 {
        visit
    } else {
        None
    }
}
