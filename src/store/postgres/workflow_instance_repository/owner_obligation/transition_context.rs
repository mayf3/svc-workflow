//! The only managed late-Context producer; all validation precedes facts.
use super::{storage, OwnerError};
use crate::domain::definition::{digest, owner_obligation as contract};
use serde_json::Value;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

pub(crate) async fn validate(
    tx: &mut Transaction<'_, Postgres>,
    instance: Uuid,
    version: Uuid,
    current_context: Uuid,
    target: Uuid,
    effect: &str,
    proposed: Option<&Value>,
) -> Result<Option<Value>, OwnerError> {
    let Some(proposed) = proposed else {
        return Ok(None);
    };
    if !proposed.is_object() || !matches!(effect, "ADVANCE" | "RETURN") {
        return Err(OwnerError::new(422, "invalid_input"));
    }
    let scope: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_owner_managed_instances m JOIN workflow_node_definitions n ON n.definition_version_id=$2 AND n.node_id=$3 WHERE m.workflow_instance_id=$1 AND m.kind='INTAKE' AND n.node_type='TASK')")
        .bind(instance).bind(version).bind(target).fetch_one(&mut **tx).await.map_err(storage)?;
    let metadata: Option<Value> = sqlx::query_scalar("SELECT metadata FROM workflow_node_definitions WHERE definition_version_id=$1 AND node_id=$2")
        .bind(version).bind(target).fetch_optional(&mut **tx).await.map_err(storage)?.flatten();
    if !scope
        || contract::gate(metadata.as_ref())
            .map_err(storage)?
            .is_none()
    {
        return Err(OwnerError::new(422, "invalid_input"));
    }
    let (old, stored_digest, schema): (Value, String, Option<Value>) = sqlx::query_as("SELECT c.payload,c.payload_digest,v.context_schema FROM workflow_context_revisions c JOIN workflow_definition_versions v ON v.definition_version_id=$3 WHERE c.context_revision_id=$1 AND c.workflow_instance_id=$2")
        .bind(current_context).bind(instance).bind(version).fetch_optional(&mut **tx).await.map_err(storage)?
        .ok_or_else(|| OwnerError::new(503,"authoritative_reference_unavailable"))?;
    if digest::compute_json_digest(&old).map_err(storage)? != stored_digest {
        return Err(OwnerError::new(503, "authoritative_reference_unavailable"));
    }
    let mut old_business = old.clone();
    let mut new_business = proposed.clone();
    old_business
        .as_object_mut()
        .ok_or_else(|| OwnerError::new(503, "authoritative_reference_unavailable"))?
        .remove("workflowPrerequisite");
    new_business
        .as_object_mut()
        .expect("object checked")
        .remove("workflowPrerequisite");
    if digest::compute_json_digest(&old_business).map_err(storage)?
        != digest::compute_json_digest(&new_business).map_err(storage)?
    {
        return Err(OwnerError::new(
            422,
            "owner_context_non_prerequisite_changed",
        ));
    }
    if let Some(schema) = schema {
        let validator = jsonschema::validator_for(&schema).map_err(storage)?;
        if !validator.is_valid(proposed) {
            return Err(OwnerError::new(422, "owner_context_schema_invalid"));
        }
    }
    if digest::compute_json_digest(proposed).map_err(storage)? == stored_digest {
        Ok(None)
    } else {
        Ok(Some(proposed.clone()))
    }
}

pub(crate) async fn append(
    tx: &mut Transaction<'_, Postgres>,
    instance: Uuid,
    previous: Uuid,
    actor: Uuid,
    payload: &Value,
) -> Result<Uuid, OwnerError> {
    let id = Uuid::new_v4();
    let hash = digest::compute_json_digest(payload).map_err(storage)?;
    sqlx::query("INSERT INTO workflow_context_revisions(context_revision_id,workflow_instance_id,revision_number,previous_revision_id,payload,payload_digest,created_by_principal_id) SELECT $1,$2,revision_number+1,$3,$4,$5,$6 FROM workflow_context_revisions WHERE context_revision_id=$3 AND workflow_instance_id=$2")
        .bind(id).bind(instance).bind(previous).bind(payload).bind(hash).bind(actor).execute(&mut **tx).await.map_err(storage)?;
    let updated=sqlx::query("UPDATE workflow_instances SET current_context_revision_id=$1 WHERE workflow_instance_id=$2 AND current_context_revision_id=$3")
        .bind(id).bind(instance).bind(previous).execute(&mut **tx).await.map_err(storage)?;
    if updated.rows_affected() != 1 {
        return Err(OwnerError::new(503, "authoritative_reference_unavailable"));
    }
    Ok(id)
}
