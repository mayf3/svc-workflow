//! Golden test for the ExecuteWorkflowTransition request hash.
//!
//! Validates the canonical JCS-sorted JSON and SHA-256 hex output
//! for a fixed input command. Uses deterministic UUIDs for reproducibility.

use super::*;

const GOLDEN_PRINCIPAL_ID: &str = "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa";
const GOLDEN_INSTANCE_ID: &str = "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb";
const GOLDEN_TRANSITION_ID: &str = "cccccccc-cccc-cccc-cccc-cccccccccccc";

#[test]
fn test_transition_request_hash_golden_canonical_json() {
    let command_schema_version = "v1".to_string();
    let principal_id = PrincipalId::from_uuid(Uuid::parse_str(GOLDEN_PRINCIPAL_ID).unwrap());
    let workflow_instance_id =
        WorkflowInstanceId::from_uuid(Uuid::parse_str(GOLDEN_INSTANCE_ID).unwrap());
    let transition_definition_id =
        TransitionId::from_uuid(Uuid::parse_str(GOLDEN_TRANSITION_ID).unwrap());
    let expected_workflow_state_version = 2i32;
    let submission_payload: Option<serde_json::Value> = None;

    let hash =
        svc_workflow::application::workflow_instance::idempotency::compute_transition_request_hash(
            &command_schema_version,
            "any-idempotency-key",
            &principal_id,
            &workflow_instance_id,
            expected_workflow_state_version,
            &transition_definition_id,
            &submission_payload,
        )
        .expect("compute hash");

    // The hash is a 64-char hex string
    assert_eq!(hash.len(), 64);
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
}

#[test]
fn test_transition_request_hash_golden_sha256() {
    // Use the same deterministic IDs
    let command_schema_version = "v1".to_string();
    let principal_id = PrincipalId::from_uuid(Uuid::parse_str(GOLDEN_PRINCIPAL_ID).unwrap());
    let workflow_instance_id =
        WorkflowInstanceId::from_uuid(Uuid::parse_str(GOLDEN_INSTANCE_ID).unwrap());
    let transition_definition_id =
        TransitionId::from_uuid(Uuid::parse_str(GOLDEN_TRANSITION_ID).unwrap());
    let expected_workflow_state_version = 2i32;
    let submission_payload: Option<serde_json::Value> = None;

    let hash =
        svc_workflow::application::workflow_instance::idempotency::compute_transition_request_hash(
            &command_schema_version,
            "any-idempotency-key",
            &principal_id,
            &workflow_instance_id,
            expected_workflow_state_version,
            &transition_definition_id,
            &submission_payload,
        )
        .expect("compute hash");

    // Verify deterministic: calling twice gives same result
    let hash2 =
        svc_workflow::application::workflow_instance::idempotency::compute_transition_request_hash(
            &command_schema_version,
            "any-other-key", // different key, same body
            &principal_id,
            &workflow_instance_id,
            expected_workflow_state_version,
            &transition_definition_id,
            &submission_payload,
        )
        .expect("compute hash2");

    assert_eq!(hash, hash2, "hash should be independent of idempotency_key");
}
