//! Same-transaction publication validation and exact receipted approval.
use crate::domain::definition::{
    error::{DefinitionError, GraphValidationError},
    model::{NodeDefinition, TransitionDefinition},
    owner_obligation as contract,
};
use serde_json::{json, Value};
use sqlx::{Postgres, Transaction};
use uuid::Uuid;
type PolicyTargetRow = (Uuid, String, i16, Option<String>, Option<Value>);
fn invalid(code: &str) -> DefinitionError {
    DefinitionError::GraphValidationFailed(vec![GraphValidationError::new(code, code)])
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn validate_and_approve(
    tx: &mut Transaction<'_, Postgres>,
    version: Uuid,
    domain: Uuid,
    owner: Uuid,
    digest: &str,
    schema: Option<&Value>,
    nodes: &[NodeDefinition],
    _transitions: &[TransitionDefinition],
    command: Option<Uuid>,
    actor: Option<Uuid>,
) -> Result<(), DefinitionError> {
    let map = |e: sqlx::Error| DefinitionError::StorageError(e.to_string());
    for n in nodes {
        let op = contract::operation(n.metadata.as_ref()).map_err(|e| invalid(&e))?;
        if let Some(op) = op {
            if let Some(target) = op.target_definition_version_id {
                let row:Option<(Uuid,String,Option<String>,Option<Value>)>=sqlx::query_as("SELECT d.domain_id,v.version_status::text,v.definition_digest,v.context_schema FROM workflow_definition_versions v JOIN workflow_definitions d USING(workflow_definition_id) WHERE v.definition_version_id=$1 AND NOT d.archived FOR SHARE OF v").bind(target).fetch_optional(&mut **tx).await.map_err(map)?;
                if !row.is_some_and(|(d, s, h, cs)| {
                    d == domain
                        && s == "PUBLISHED"
                        && h == op.target_definition_digest
                        && contract::policy(cs.as_ref()).ok().flatten().is_none()
                        && contract::role(cs.as_ref()).ok().flatten().is_none()
                }) {
                    return Err(invalid("owner_operation_target_invalid"));
                }
            }
        }
    }
    let Some(p) = contract::policy(schema).map_err(|e| invalid(&e))? else {
        return Ok(());
    };
    let mut target_digests = Vec::new();
    for (target, transition, role) in [
        (
            p.intake_definition_version_id,
            p.intake_acceptance_transition_id,
            "INTAKE",
        ),
        (
            p.review_definition_version_id,
            p.review_completion_transition_id,
            "REVIEW",
        ),
    ] {
        let row:Option<PolicyTargetRow>=sqlx::query_as("SELECT d.domain_id,v.version_status::text,v.semantic_model_version,v.definition_digest,v.context_schema FROM workflow_definition_versions v JOIN workflow_definitions d USING(workflow_definition_id) WHERE v.definition_version_id=$1 AND NOT d.archived FOR SHARE OF v").bind(target).fetch_optional(&mut **tx).await.map_err(map)?;
        let Some((d, status, model, h, cs)) = row else {
            return Err(invalid("owner_policy_target_not_published"));
        };
        if status != "PUBLISHED" || h.is_none() {
            return Err(invalid("owner_policy_target_not_published"));
        }
        if d != domain {
            return Err(invalid("owner_policy_target_domain_mismatch"));
        }
        if model != 3 || contract::role(cs.as_ref()).ok().flatten() != Some(role) {
            return Err(invalid("owner_policy_target_role_mismatch"));
        }
        if contract::policy(cs.as_ref())
            .map_err(|e| invalid(&e))?
            .is_some()
        {
            return Err(invalid("owner_policy_recursive_target"));
        }
        let sub:Option<Value>=sqlx::query_scalar("SELECT t.submission_schema FROM workflow_transition_definitions t JOIN workflow_node_definitions n ON n.node_id=t.target_node_id WHERE t.transition_id=$1 AND t.definition_version_id=$2 AND n.definition_version_id=$2 AND n.node_type='TERMINAL' AND t.transition_effect='ADVANCE' AND t.submission_schema IS NOT NULL").bind(transition).bind(target).fetch_optional(&mut **tx).await.map_err(map)?;
        let Some(sub) = sub else {
            return Err(invalid("owner_policy_target_role_mismatch"));
        };
        if role == "INTAKE"
            && !crate::domain::workflow_instance::governance_child::declares_acceptance(&sub)
        {
            return Err(invalid("owner_policy_target_role_mismatch"));
        }
        if role == "REVIEW"
            && !(sub["required"]
                .as_array()
                .is_some_and(|a| a.contains(&json!("reviewOutcome")))
                && sub["properties"]["reviewOutcome"]["enum"]
                    == json!(["RESOLVED", "HUMAN_REQUIRED"]))
        {
            return Err(invalid("owner_policy_target_role_mismatch"));
        }
        let qa:bool=sqlx::query_scalar("SELECT count(*)>=2 AND count(DISTINCT (coalesce(fixed_principal_id::text,assignee_ref_type::text)))>=2 FROM workflow_node_definitions WHERE definition_version_id=$1 AND node_type='TASK'").bind(target).fetch_one(&mut **tx).await.map_err(map)?;
        if !qa {
            return Err(invalid("owner_policy_target_role_mismatch"));
        }
        let graph:Value=sqlx::query_scalar("SELECT jsonb_build_object('nodes',(SELECT jsonb_agg(to_jsonb(n)-'created_at' ORDER BY node_id) FROM workflow_node_definitions n WHERE definition_version_id=$1),'transitions',(SELECT jsonb_agg(to_jsonb(t)-'created_at' ORDER BY transition_id) FROM workflow_transition_definitions t WHERE definition_version_id=$1))").bind(target).fetch_one(&mut **tx).await.map_err(map)?;
        target_digests.push((
            h.unwrap(),
            crate::domain::definition::digest::compute_json_digest(&graph)
                .map_err(DefinitionError::DigestFailure)?,
            crate::domain::definition::digest::compute_json_digest(&json!(cs))
                .map_err(DefinitionError::DigestFailure)?,
        ));
    }
    let command = command.ok_or(DefinitionError::PermissionDenied)?;
    let actor = actor.ok_or(DefinitionError::PermissionDenied)?;
    let enabled:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM principals WHERE principal_id=$1 AND enabled AND principal_type='AGENT')").bind(actor).fetch_one(&mut **tx).await.map_err(map)?;
    if !enabled {
        return Err(DefinitionError::PrincipalDisabled);
    }
    let pd = crate::domain::definition::digest::compute_json_digest(
        &json!({"policy":p,"sourceDefinitionDigest":digest,"targetDigests":target_digests}),
    )
    .map_err(DefinitionError::DigestFailure)?;
    let replacement:Option<Uuid>=sqlx::query_scalar("SELECT command_id FROM workflow_command_receipts WHERE command_type='PROVISION_REPLACE_OWNER' AND receipt_status='COMPLETED' AND response_status BETWEEN 200 AND 299 AND response_body->>'domainId'=$1 AND response_body->>'newOwnerId'=$2 ORDER BY completed_at DESC,command_id DESC LIMIT 1").bind(domain.to_string()).bind(owner.to_string()).fetch_optional(&mut **tx).await.map_err(map)?;
    sqlx::query("INSERT INTO workflow_owner_policy_approvals(source_definition_version_id,policy_digest,publish_command_id,approved_by_principal_id,configured_policy_actor_principal_id,domain_id,source_definition_digest,intake_definition_version_id,review_definition_version_id,intake_acceptance_transition_id,review_completion_transition_id,intake_target_version_digest,intake_graph_digest,intake_schema_digest,review_target_version_digest,review_graph_digest,review_schema_digest,owner_replacement_command_id) VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18)").bind(version).bind(pd).bind(command).bind(owner).bind(actor).bind(domain).bind(digest).bind(p.intake_definition_version_id).bind(p.review_definition_version_id).bind(p.intake_acceptance_transition_id).bind(p.review_completion_transition_id).bind(&target_digests[0].0).bind(&target_digests[0].1).bind(&target_digests[0].2).bind(&target_digests[1].0).bind(&target_digests[1].1).bind(&target_digests[1].2).bind(replacement).execute(&mut **tx).await.map_err(map)?;
    Ok(())
}
