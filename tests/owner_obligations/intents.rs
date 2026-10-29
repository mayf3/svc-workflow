use super::*;

#[tokio::test]
async fn intents_ordinary_create_cannot_birth_managed_role() {
    let p = pool().await;
    let (owner, domain) = seed(&p).await;
    let (v, _) = managed_version(&p, domain, owner, "INTAKE").await;
    publish(&p, owner, v, None, "publish").await.unwrap();
    assert!(
        create(&p, owner, domain, v, json!({})).await.is_err(),
        "managed target must be internally policy created"
    );
}
#[tokio::test]
async fn intents_request_has_atomic_unique_intake_without_naked_chat() {
    let p = pool().await;
    let (owner, domain, pm, source, _, _) = policy_fixture(&p).await;
    publish(&p, owner, source, Some(pm), "source")
        .await
        .unwrap();
    let s = create(
        &p,
        owner,
        domain,
        source,
        json!({"ownerIntakeContext":{},"ownerReviewContext":{}}),
    )
    .await
    .unwrap();
    let c = request(&p, owner, s.workflow_instance_id, s.current_node_visit_id).await;
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_owner_obligation_intents WHERE case_id=$1 AND kind='INTAKE' AND state='PENDING'").bind(c.assistance_case_id).fetch_one(&p).await.unwrap();
    assert_eq!(n, 1);
    let wakes: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_outbox WHERE outbox_kind='OWNER_ASSISTANCE_WAKE'",
    )
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(wakes, 0);
}
#[tokio::test]
async fn intents_auto_intake_is_one_exact_pair_under_concurrency_and_restart() {
    let p = pool().await;
    let (owner, domain, pm, source, _, _) = policy_fixture(&p).await;
    publish(&p, owner, source, Some(pm), "source")
        .await
        .unwrap();
    let s = create(
        &p,
        owner,
        domain,
        source,
        json!({"ownerIntakeContext":{},"ownerReviewContext":{}}),
    )
    .await
    .unwrap();
    request(&p, owner, s.workflow_instance_id, s.current_node_visit_id).await;
    use svc_workflow::store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::reconcile::reconcile_pending,
    };
    let (a, b) = tokio::join!(
        reconcile_pending(&p, pm, AdmissionGate::disabled()),
        reconcile_pending(&p, pm, AdmissionGate::disabled())
    );
    assert!(a.is_ok(), "{a:?}");
    assert!(b.is_ok(), "{b:?}");
    assert!(reconcile_pending(&p, pm, AdmissionGate::disabled())
        .await
        .unwrap()
        .is_empty());
    let n: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_owner_managed_instances WHERE kind='INTAKE'",
    )
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(n, 1);
    let pair:Vec<(String,i64)>=sqlx::query_as("SELECT r.command_type,count(e.event_id) FROM workflow_command_receipts r LEFT JOIN workflow_events e USING(command_id) WHERE r.command_type='AUTO_CREATE_AND_BIND_OWNER_INTAKE' GROUP BY r.command_id").fetch_all(&p).await.unwrap();
    assert_eq!(pair, vec![("AUTO_CREATE_AND_BIND_OWNER_INTAKE".into(), 2)]);
}
#[tokio::test]
async fn intents_config_actor_mismatch_stops_before_any_birth() {
    let p = pool().await;
    let (owner, domain, pm, source, _, _) = policy_fixture(&p).await;
    publish(&p, owner, source, Some(pm), "source")
        .await
        .unwrap();
    let s = create(
        &p,
        owner,
        domain,
        source,
        json!({"ownerIntakeContext":{},"ownerReviewContext":{}}),
    )
    .await
    .unwrap();
    request(&p, owner, s.workflow_instance_id, s.current_node_visit_id).await;
    svc_workflow::store::postgres::workflow_instance_repository::owner_obligation::reconcile::reconcile_pending(&p,owner,svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled()).await.unwrap();
    let row: (String, String, Option<Uuid>) = sqlx::query_as(
        "SELECT state,stop_code,managed_instance_id FROM workflow_owner_obligation_intents",
    )
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(
        row,
        ("STOPPED".into(), "POLICY_ACTOR_MISMATCH".into(), None)
    );
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_instances")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(n, 1);
}
#[tokio::test]
async fn intents_invalid_initial_context_is_known_failure_no_auto_business_facts() {
    let p = pool().await;
    let (owner, domain, pm, source, _, _) = policy_fixture_required(&p, true).await;
    publish(&p, owner, source, Some(pm), "source")
        .await
        .unwrap();
    let s = create(
        &p,
        owner,
        domain,
        source,
        json!({"ownerIntakeContext":{},"ownerReviewContext":{}}),
    )
    .await
    .unwrap();
    request(&p, owner, s.workflow_instance_id, s.current_node_visit_id).await;
    use svc_workflow::store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::reconcile::reconcile_pending,
    };
    assert!(reconcile_pending(&p, pm, AdmissionGate::disabled())
        .await
        .unwrap()
        .is_empty());
    assert!(reconcile_pending(&p, pm, AdmissionGate::disabled())
        .await
        .unwrap()
        .is_empty());
    let row:(String,Option<Uuid>,String)=sqlx::query_as("SELECT state,managed_instance_id,outcome FROM workflow_owner_obligation_intents WHERE kind='INTAKE'").fetch_one(&p).await.unwrap();
    assert_eq!(
        row,
        (
            "BLOCKED_KNOWN".into(),
            None,
            "POLICY_INITIAL_CONTEXT_INVALID".into()
        )
    );
    let receipts:Vec<(i32,i64)>=sqlx::query_as("SELECT r.response_status,(SELECT count(*) FROM workflow_events e WHERE e.command_id=r.command_id) FROM workflow_command_receipts r WHERE r.command_type='AUTO_CREATE_AND_BIND_OWNER_INTAKE'").fetch_all(&p).await.unwrap();
    assert_eq!(receipts, vec![(422, 0)]);
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_instances")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(n, 1);
}
#[tokio::test]
async fn intents_storage_fault_after_pair_rolls_back_all_birth_binding_and_receipt_then_reconciles()
{
    let p = pool().await;
    let (owner, domain, pm, source, _, _) = policy_fixture(&p).await;
    publish(&p, owner, source, Some(pm), "source")
        .await
        .unwrap();
    let s = create(
        &p,
        owner,
        domain,
        source,
        json!({"ownerIntakeContext":{},"ownerReviewContext":{}}),
    )
    .await
    .unwrap();
    request(&p, owner, s.workflow_instance_id, s.current_node_visit_id).await;
    sqlx::raw_sql("CREATE FUNCTION isolated_reject_managed() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'isolated fault after real pair'; END $$; CREATE TRIGGER isolated_reject_managed BEFORE INSERT ON workflow_owner_managed_instances FOR EACH ROW EXECUTE FUNCTION isolated_reject_managed();").execute(&p).await.unwrap();
    use svc_workflow::store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::reconcile::reconcile_pending,
    };
    assert!(reconcile_pending(&p, pm, AdmissionGate::disabled())
        .await
        .is_err());
    let facts:(i64,i64,i64,i32)=sqlx::query_as("SELECT (SELECT count(*) FROM workflow_instances),(SELECT count(*) FROM workflow_governance_child_bindings),(SELECT count(*) FROM workflow_command_receipts WHERE command_type='AUTO_CREATE_AND_BIND_OWNER_INTAKE'),workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1").bind(s.workflow_instance_id).fetch_one(&p).await.unwrap();
    assert_eq!(facts, (1, 0, 0, 2));
    let state: (String, Option<Uuid>) = sqlx::query_as(
        "SELECT state,last_command_id FROM workflow_owner_obligation_intents WHERE kind='INTAKE'",
    )
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(state, ("PENDING".into(), None));
    sqlx::query("DROP TRIGGER isolated_reject_managed ON workflow_owner_managed_instances")
        .execute(&p)
        .await
        .unwrap();
    assert_eq!(
        reconcile_pending(&p, pm, AdmissionGate::disabled())
            .await
            .unwrap()
            .len(),
        1
    );
}
#[tokio::test]
async fn intents_actual_request_receipt_and_wrong_policy_references_are_database_guarded() {
    let p = pool().await;
    let (owner, domain, pm, source, _, _) = policy_fixture(&p).await;
    publish(&p, owner, source, Some(pm), "source")
        .await
        .unwrap();
    let s = create(
        &p,
        owner,
        domain,
        source,
        json!({"ownerIntakeContext":{},"ownerReviewContext":{}}),
    )
    .await
    .unwrap();
    let case = request(&p, owner, s.workflow_instance_id, s.current_node_visit_id).await;
    let (intent,digest):(Uuid,String)=sqlx::query_as("SELECT intent_id,policy_digest FROM workflow_owner_obligation_intents WHERE case_id=$1 AND kind='INTAKE'").bind(case.assistance_case_id).fetch_one(&p).await.unwrap();
    assert!(sqlx::query(
        "UPDATE workflow_owner_obligation_intents SET policy_digest=$2 WHERE intent_id=$1"
    )
    .bind(intent)
    .bind("a".repeat(64))
    .execute(&p)
    .await
    .is_err());
    let actual: String = sqlx::query_scalar(
        "SELECT policy_digest FROM workflow_owner_obligation_intents WHERE intent_id=$1",
    )
    .bind(intent)
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(actual, digest);
}
