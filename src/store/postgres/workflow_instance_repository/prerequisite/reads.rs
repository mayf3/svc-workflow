use super::*;
pub(crate) async fn reader_allowed(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    instance: Uuid,
) -> Result<bool, WorkflowQueryError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_instances wi JOIN domains d USING(domain_id) JOIN principals p ON p.principal_id=$1 AND p.enabled JOIN workflow_definition_versions dv ON dv.definition_version_id=wi.definition_version_id JOIN workflow_definitions wd ON wd.workflow_definition_id=dv.workflow_definition_id AND wd.domain_id=wi.domain_id JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id AND v.workflow_instance_id=wi.workflow_instance_id JOIN workflow_node_definitions n ON n.node_id=v.node_id AND n.definition_version_id=wi.definition_version_id WHERE wi.workflow_instance_id=$2 AND d.enabled AND (v.assignee_principal_id=$1 OR EXISTS(SELECT 1 FROM domain_role_bindings b WHERE b.domain_id=wi.domain_id AND b.principal_id=$1 AND b.role_key='DOMAIN_OWNER' AND b.enabled)))").bind(actor).bind(instance).fetch_one(&mut **tx).await.map_err(storage)
}
/// All consumers, including the READY poll, share this exact evaluator.
pub(crate) async fn project(
    tx: &mut Transaction<'_, Postgres>,
    instance: Uuid,
) -> Result<Option<Value>, WorkflowQueryError> {
    let row:Option<(Uuid,Option<Value>,Option<Value>,Option<Uuid>,Option<String>)>=sqlx::query_as("SELECT wi.domain_id,dv.context_schema,cr.payload,cr.workflow_instance_id,cr.payload_digest FROM workflow_instances wi JOIN workflow_definition_versions dv USING(definition_version_id) LEFT JOIN workflow_context_revisions cr ON cr.workflow_instance_id=wi.workflow_instance_id AND cr.revision_number=1 WHERE wi.workflow_instance_id=$1").bind(instance).fetch_optional(&mut **tx).await.map_err(storage)?;
    let Some((domain, schema, context, context_instance, context_digest)) = row else {
        return Err(WorkflowQueryError::WorkflowInstanceNotFoundOrNotVisible);
    };
    let context = context.unwrap_or(Value::Null);
    match contract::parse(schema.as_ref(), &context) {
        Ok(None) => Ok(None),
        Ok(Some(r))
            if context_instance == Some(instance)
                && digest::compute_json_digest(&context).ok() == context_digest =>
        {
            source::evaluate(tx, domain, &r).await.map(Some)
        }
        // Never fabricate coordinates. Invalid shape with preserved valid IDs can
        // yield identified UNKNOWN; an unrecoverable triple is omitted as one unit.
        _ => {
            let raw = &context["workflowPrerequisite"];
            let id = |key: &str| raw[key].as_str().and_then(|s| s.parse::<Uuid>().ok());
            match (
                id("workflowInstanceId"),
                id("definitionVersionId"),
                id("transitionId"),
            ) {
                (Some(source), Some(version), Some(transition)) => Ok(Some(shape(
                    &Reference {
                        kind: contract::Kind::TerminalSubmission,
                        workflow_instance_id: source,
                        definition_version_id: version,
                        transition_id: transition,
                    },
                    "UNKNOWN",
                    Some("LINEAGE_INCONSISTENT"),
                ))),
                _ => Ok(Some(
                    json!({"status":"UNKNOWN","reason":"LINEAGE_INCONSISTENT"}),
                )),
            }
        }
    }
}
pub async fn read(
    pool: &sqlx::PgPool,
    actor: Uuid,
    instance: Uuid,
) -> Result<Value, WorkflowQueryError> {
    // Readiness alone can represent corrupt/missing Context. Authorize using
    // actual enabled actor, Domain, immutable Definition and current Visit facts
    // before touching the closed projection; ordinary readers retain all checks.
    let mut tx =
        query_visibility::actor_snapshot(pool, actor, Some(instance), "PrerequisiteReadiness")
            .await?;
    if !reader_allowed(&mut tx, actor, instance).await? {
        return Err(WorkflowQueryError::WorkflowInstanceNotFoundOrNotVisible);
    }
    let value = project(&mut tx, instance).await?;
    tx.commit().await.map_err(storage)?;
    Ok(value.map_or_else(|| json!({}), |v| json!({"prerequisiteReadiness":v})))
}
