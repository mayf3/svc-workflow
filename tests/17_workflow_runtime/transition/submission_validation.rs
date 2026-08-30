use super::*;

/// Schema non-null but payload is None → SubmissionRequired.
#[tokio::test]
async fn test_transition_submission_required() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, _, ret_id, _) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    // RETURN has a submission_schema, but we provide None
    let cmd = make_transition_command(principal_id, instance_id, 2, ret_id, None);
    let err = execute_workflow_transition(&pool, cmd).await.unwrap_err();
    assert!(matches!(
        err,
        ExecuteWorkflowTransitionError::SubmissionRequired
    ));
}

/// Schema NULL and payload None → no submission, succeeds.
#[tokio::test]
async fn test_transition_schema_null_no_payload_succeeds() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, normal_adv, _, _) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    // NORMAL→TERMINAL has no submission_schema
    let cmd = make_transition_command(principal_id, instance_id, 2, normal_adv, None);
    let result = execute_workflow_transition(&pool, cmd).await.unwrap();
    assert_eq!(result.submission_id, None);
}

/// Schema NULL and payload Some → creates submission (no schema validation).
#[tokio::test]
async fn test_transition_schema_null_with_payload_creates_submission() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, normal_adv, _, _) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    let payload = serde_json::json!({"any": "data"});
    let cmd = make_transition_command(principal_id, instance_id, 2, normal_adv, Some(payload));
    let result = execute_workflow_transition(&pool, cmd).await.unwrap();
    assert!(result.submission_id.is_some());
}

/// Schema validation: required field missing.
#[tokio::test]
async fn test_transition_submission_required_field_missing() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, _, _, term_trans_id) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    // Missing both required fields
    let payload = serde_json::json!({"extra": "field"});
    let cmd = make_transition_command(principal_id, instance_id, 2, term_trans_id, Some(payload));
    let err = execute_workflow_transition(&pool, cmd).await.unwrap_err();
    assert!(matches!(
        err,
        ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)
    ));
}

/// Schema validation: type error.
#[tokio::test]
async fn test_transition_submission_type_error() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, _, _, term_trans_id) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    let payload = serde_json::json!({"reasonCode": 123, "reason": "test"});
    let cmd = make_transition_command(principal_id, instance_id, 2, term_trans_id, Some(payload));
    let err = execute_workflow_transition(&pool, cmd).await.unwrap_err();
    assert!(matches!(
        err,
        ExecuteWorkflowTransitionError::SubmissionValidationFailed(_)
    ));
}

/// Schema validation: valid payload succeeds.
#[tokio::test]
async fn test_transition_submission_valid_schema() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, _, _, term_trans_id) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    let payload = serde_json::json!({"reasonCode": "DUPLICATE", "reason": "This is a duplicate"});
    let cmd = make_transition_command(principal_id, instance_id, 2, term_trans_id, Some(payload));
    let result = execute_workflow_transition(&pool, cmd).await.unwrap();
    assert!(result.submission_id.is_some());
}

/// Payload size > 1 MiB is rejected.
#[tokio::test]
async fn test_transition_submission_size_exceeded() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, normal_adv, _, _) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    let large_data = "x".repeat(1024 * 1024 + 1);
    let payload = serde_json::json!({"data": large_data});

    let cmd = make_transition_command(principal_id, instance_id, 2, normal_adv, Some(payload));
    let err = execute_workflow_transition(&pool, cmd).await.unwrap_err();
    assert!(matches!(
        err,
        ExecuteWorkflowTransitionError::SizeLimitExceeded(_)
    ));
}

/// RETURN validation: rootCauseNodeVisitId must belong to same instance.
#[tokio::test]
async fn test_transition_return_root_cause_wrong_instance() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, _, ret_id, _) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _source_visit_id) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    let fake_visit = Uuid::new_v4();
    let payload = serde_json::json!({
        "rootCauseNodeVisitId": fake_visit.to_string(),
        "relatedSubmissionIds": [],
        "reasonCode": "NEEDS_REVISION",
        "reason": "Need changes",
    });

    let cmd = make_transition_command(principal_id, instance_id, 2, ret_id, Some(payload));
    let err = execute_workflow_transition(&pool, cmd).await.unwrap_err();
    assert!(matches!(
        err,
        ExecuteWorkflowTransitionError::InvalidReturnReferences(_)
    ));
}

/// RETURN validation: relatedSubmissionIds must belong to same instance.
#[tokio::test]
async fn test_transition_return_related_submission_wrong_instance() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, _, ret_id, _) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _source_visit_id) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    let fake_sub = Uuid::new_v4();
    let payload = serde_json::json!({
        "rootCauseNodeVisitId": _source_visit_id.to_string(),
        "relatedSubmissionIds": [fake_sub.to_string()],
        "reasonCode": "NEEDS_REVISION",
        "reason": "Need changes",
    });

    let cmd = make_transition_command(principal_id, instance_id, 2, ret_id, Some(payload));
    let err = execute_workflow_transition(&pool, cmd).await.unwrap_err();
    assert!(matches!(
        err,
        ExecuteWorkflowTransitionError::InvalidReturnReferences(_)
    ));
}

/// RETURN normal path: valid root cause (upstream visit) + reason + related
/// submissions all owned by the instance → transition succeeds.
#[tokio::test]
async fn test_transition_return_valid_references_succeeds() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, _, ret_id, _) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, source_visit_id) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    // rootCauseNodeVisitId = the visit that created this submission path
    // (upstream visit of the current node), relatedSubmissionIds empty.
    let payload = serde_json::json!({
        "rootCauseNodeVisitId": source_visit_id.to_string(),
        "relatedSubmissionIds": [],
        "reasonCode": "NEEDS_REVISION",
        "reason": "Needs revision before approval",
    });

    let cmd = make_transition_command(principal_id, instance_id, 2, ret_id, Some(payload));
    let result = execute_workflow_transition(&pool, cmd).await.unwrap();
    assert!(result.submission_id.is_some());
}

/// RETURN missing required contract fields → aggregated InvalidReturnReferences
/// with the full contract listed (root cause of the reported 422 loop).
///
/// Mirrors the real incident: the definition's RETURN submission_schema only
/// declares `summary` (so schema validation passes), while the engine-level
/// RETURN contract (rootCauseNodeVisitId/reasonCode/reason) is enforced by
/// validate_return_references — the exact path that produced the opaque 422
/// `invalid_return_references` on instance 121e76b4.
#[tokio::test]
async fn test_transition_return_missing_contract_fields_reports_all() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    // Incident-shaped definition: RETURN submission_schema only declares
    // `summary` (no RETURN contract fields), so schema validation passes and
    // the engine-level RETURN contract check is what rejects the payload.
    let (_, ver_id, _, _, _, draft_adv, _, ret_id, _) = seed_transition_graph_with_return_schema(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
        serde_json::json!({
            "type": "object",
            "required": ["summary"],
            "properties": { "summary": { "type": "string" } }
        }),
    )
    .await;

    let (_, instance_id, _source_visit_id) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    // Only a summary-like field: all RETURN contract fields missing.
    let payload = serde_json::json!({ "summary": "looks fine to me" });

    let cmd = make_transition_command(principal_id, instance_id, 2, ret_id, Some(payload));
    let err = execute_workflow_transition(&pool, cmd).await.unwrap_err();
    match err {
        ExecuteWorkflowTransitionError::InvalidReturnReferences(detail) => {
            assert!(
                detail.contains("rootCauseNodeVisitId is required"),
                "detail must mention rootCauseNodeVisitId: {}",
                detail
            );
            assert!(
                detail.contains("reasonCode is required"),
                "detail must mention reasonCode: {}",
                detail
            );
            assert!(
                detail.contains("reason is required"),
                "detail must mention reason: {}",
                detail
            );
            // Aggregated: all three reported in one error, not just the first.
            assert_eq!(detail.matches("is required").count(), 3);
        }
        other => panic!("expected InvalidReturnReferences, got {:?}", other),
    }
}

/// RETURN schema declares reasonCode/reason (as the test seed does) but the
/// caller omits rootCauseNodeVisitId → schema passes, engine contract fails
/// with a single missing-field error.
#[tokio::test]
async fn test_transition_return_missing_root_cause_only() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, _, ret_id, _) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _source_visit_id) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    let payload = serde_json::json!({
        "reasonCode": "NEEDS_REVISION",
        "reason": "Need changes",
    });

    let cmd = make_transition_command(principal_id, instance_id, 2, ret_id, Some(payload));
    let err = execute_workflow_transition(&pool, cmd).await.unwrap_err();
    match err {
        ExecuteWorkflowTransitionError::InvalidReturnReferences(detail) => {
            assert!(
                detail.contains("rootCauseNodeVisitId is required"),
                "detail must mention rootCauseNodeVisitId: {}",
                detail
            );
        }
        other => panic!("expected InvalidReturnReferences, got {:?}", other),
    }
}

/// RETURN malformed rootCauseNodeVisitId (not a UUID) → rejected with reason.
#[tokio::test]
async fn test_transition_return_root_cause_not_uuid() {
    let pool = create_pool().await;
    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, ver_id, _, _, _, draft_adv, _, ret_id, _) = seed_transition_graph(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
    )
    .await;

    let (_, instance_id, _source_visit_id) =
        create_and_advance_to_normal(&pool, principal_id, domain_id, draft_adv, ver_id).await;

    let payload = serde_json::json!({
        "rootCauseNodeVisitId": "not-a-uuid",
        "relatedSubmissionIds": [],
        "reasonCode": "NEEDS_REVISION",
        "reason": "Need changes",
    });

    let cmd = make_transition_command(principal_id, instance_id, 2, ret_id, Some(payload));
    let err = execute_workflow_transition(&pool, cmd).await.unwrap_err();
    match err {
        ExecuteWorkflowTransitionError::InvalidReturnReferences(detail) => {
            assert!(
                detail.contains("not a valid UUID"),
                "detail must explain the malformed UUID: {}",
                detail
            );
        }
        other => panic!("expected InvalidReturnReferences, got {:?}", other),
    }
}
