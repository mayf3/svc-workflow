use super::*;
pub(crate) async fn reader_allowed(
    tx: &mut Transaction<'_, Postgres>,
    actor: Uuid,
    instance: Uuid,
) -> Result<bool, WorkflowQueryError> {
    sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_instances wi JOIN domains d USING(domain_id) JOIN principals p ON p.principal_id=$1 AND p.enabled JOIN workflow_node_visits v ON v.node_visit_id=wi.current_node_visit_id AND v.workflow_instance_id=wi.workflow_instance_id WHERE wi.workflow_instance_id=$2 AND d.enabled AND (v.assignee_principal_id=$1 OR EXISTS(SELECT 1 FROM domain_role_bindings b WHERE b.domain_id=wi.domain_id AND b.principal_id=$1 AND b.role_key='DOMAIN_OWNER' AND b.enabled)))").bind(actor).bind(instance).fetch_one(&mut **tx).await.map_err(storage)
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
        // still yield UNKNOWN; missing/invalid coordinates fail the transport closed.
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
                _ => Err(WorkflowQueryError::InternalConsistency(
                    "malformed prerequisite has no exact reference coordinates".into(),
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
    let mut snapshot =
        query_visibility::authorized_snapshot(pool, actor, instance, "PrerequisiteReadiness")
            .await?;
    if !reader_allowed(&mut snapshot.tx, actor, instance).await? {
        return Err(WorkflowQueryError::WorkflowInstanceNotFoundOrNotVisible);
    }
    let value = project(&mut snapshot.tx, instance).await?;
    snapshot.tx.commit().await.map_err(storage)?;
    Ok(value.map_or_else(|| json!({}), |v| json!({"prerequisiteReadiness":v})))
}
