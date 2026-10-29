use super::*;

#[tokio::test]
async fn publication_malformed_policy_is_rejected() {
    let p = pool().await;
    let (owner, domain) = seed(&p).await;
    let (v, _) = version(
        &p,
        domain,
        owner,
        json!({"type":"object","x-workflow-owner-obligation-v1":{"version":1}}),
    )
    .await;
    let result = svc_workflow::application::definition_governance::governance_publish_version(
        &p,
        owner,
        "malformed-policy",
        "req",
        v,
        None,
        svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
    )
    .await;
    assert!(matches!(result,Err(svc_workflow::application::definition_governance::DefinitionGovernanceError::GraphValidationFailed(_))),"malformed closed policy must fail publication, got {result:?}");
}
#[tokio::test]
async fn publication_receipt_approval_version_share_commit_and_replay() {
    let p = pool().await;
    let (owner, _, pm, source, _, _) = policy_fixture(&p).await;
    let v = publish(&p, owner, source, Some(pm), "source-publish")
        .await
        .unwrap();
    let again = publish(&p, owner, source, Some(pm), "source-publish")
        .await
        .unwrap();
    assert_eq!(v, again);
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_owner_policy_approvals a JOIN workflow_command_receipts r ON r.command_id=a.publish_command_id WHERE a.source_definition_version_id=$1 AND r.command_type='DEFINITION_PUBLISH' AND r.receipt_status='COMPLETED' AND r.response_status=200").bind(source).fetch_one(&p).await.unwrap();
    assert_eq!(n, 1);
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_events")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(events, 0);
    let mutate=sqlx::query("UPDATE workflow_owner_policy_approvals SET configured_policy_actor_principal_id=$1 WHERE source_definition_version_id=$2").bind(owner).bind(source).execute(&p).await;
    assert!(mutate.is_err());
}
#[tokio::test]
async fn publication_approval_fault_rolls_back_version_and_receipt() {
    let p = pool().await;
    let (owner, _, pm, source, _, _) = policy_fixture(&p).await;
    sqlx::raw_sql("CREATE FUNCTION reject_test_policy() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'isolated injected approval failure'; END $$; CREATE TRIGGER reject_test_policy BEFORE INSERT ON workflow_owner_policy_approvals FOR EACH ROW EXECUTE FUNCTION reject_test_policy();").execute(&p).await.unwrap();
    assert!(publish(&p, owner, source, Some(pm), "fault").await.is_err());
    let status:String=sqlx::query_scalar("SELECT version_status::text FROM workflow_definition_versions WHERE definition_version_id=$1").bind(source).fetch_one(&p).await.unwrap();
    assert_eq!(status, "DRAFT");
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_command_receipts WHERE idempotency_key='fault'",
    )
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(count, 0);
}
#[test]
fn production_registry_and_closed_optional_fields_have_exact_canonical_meaning() {
    use svc_workflow::domain::definition::{digest, owner_obligation as c};
    let rows = c::registry();
    assert_eq!(rows.len(), 3);
    for r in &rows {
        assert_eq!(r["descriptor"]["version"], 1);
        assert_eq!(
            digest::compute_json_digest(&r["canonicalInput"]).unwrap(),
            r["descriptor"]["hash"]
        );
    }
    let mut op = json!({"workflowOperation":{"version":1,"slotKey":"opaque/slot ? with unicode 等","allowedOperations":[rows[0]["descriptor"]],"requiredForAdvance":true,"targetDefinitionVersionId":Uuid::nil(),"targetDefinitionDigest":"a".repeat(64)}});
    assert!(c::operation(Some(&op)).unwrap().is_some());
    for key in [
        "targetDefinitionVersionId",
        "targetDefinitionDigest",
        "prerequisiteProducer",
    ] {
        let mut v = op.clone();
        v["workflowOperation"][key] = Value::Null;
        assert!(c::operation(Some(&v)).is_err());
    }
    op["workflowOperation"]["unexpectedPlanId"] = json!("new-model-plan");
    assert!(c::operation(Some(&op)).is_err());
}
