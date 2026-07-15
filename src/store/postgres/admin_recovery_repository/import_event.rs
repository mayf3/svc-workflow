use uuid::Uuid;

use crate::domain::workflow_instance::recovery::RecoveryError;

use super::event_fields::{exact_keys, string_field};
use super::rows::{ContextFact, EventFact, InstanceRow, VisitFact};

fn invalid(detail: impl Into<String>) -> RecoveryError {
    RecoveryError::InvalidImmutableFacts(detail.into())
}

fn canonical_uuid(value: &str) -> Option<Uuid> {
    let parsed = value.parse::<Uuid>().ok()?;
    (parsed.to_string() == value).then_some(parsed)
}

fn lowercase_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn whole_second_utc(value: &str) -> bool {
    value.len() == 20
        && chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%SZ")
            .is_ok_and(|parsed| parsed.format("%Y-%m-%dT%H:%M:%SZ").to_string() == value)
}

pub(super) fn validate(
    data: &serde_json::Value,
    event: &EventFact,
    instance: &InstanceRow,
    context: &ContextFact,
    visit: &VisitFact,
) -> Result<(), RecoveryError> {
    let keys = [
        "legacySystem",
        "legacyRecordId",
        "legacySnapshotDigest",
        "importedNodeId",
        "importedAt",
        "creatorResolution",
    ];
    let record = string_field(data, "legacyRecordId").and_then(canonical_uuid);
    let imported_node = string_field(data, "importedNodeId").and_then(canonical_uuid);
    let digest = string_field(data, "legacySnapshotDigest");
    let imported_at = string_field(data, "importedAt");
    let resolution = string_field(data, "creatorResolution");
    let expected_reference = record.map(|id| format!("migration:adc:{id}:v1"));
    if !exact_keys(data, &keys)
        || string_field(data, "legacySystem") != Some("adc")
        || imported_node != Some(visit.node_id)
        || digest.is_none_or(|value| !lowercase_digest(value))
        || imported_at.is_none_or(|value| !whole_second_utc(value))
        || !matches!(resolution, Some("LEGACY_CREATOR" | "DOMAIN_OWNER_FALLBACK"))
        || instance.external_reference.as_ref() != expected_reference.as_ref()
        || instance.created_by_principal_type == "SERVICE"
        || event.actor_principal_type != "SERVICE"
        || event.actor_principal_id == instance.created_by_principal_id
        || context.created_by_principal_id != instance.created_by_principal_id
        || (visit.node_type == "TERMINAL" && visit.assignee_principal_id.is_some())
        || (visit.node_type != "TERMINAL" && visit.assignee_principal_id.is_none())
    {
        return Err(invalid("import event data or identity facts are invalid"));
    }
    Ok(())
}
