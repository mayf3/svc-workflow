use super::*;

#[tokio::test]
async fn readiness_required_operation_cannot_be_skipped_by_advance() {
    let p = pool().await;
    let (owner, _, _, managed, _, _, _) = operation_fixture(&p).await;
    let t:Uuid=sqlx::query_scalar("SELECT n.primary_advance_transition_id FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n USING(node_id) WHERE w.workflow_instance_id=$1").bind(managed).fetch_one(&p).await.unwrap();
    assert!(
        transition(&p, owner, managed, 1, t, None).await.is_err(),
        "real required effect cannot be skipped"
    );
}
#[tokio::test]
async fn effects_concurrent_create_response_loss_replay_and_changed_body() {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    use svc_workflow::store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::{effects, reads},
    };
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit: Uuid = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    let version: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let b = effects::Execute {
        expected_workflow_state_version: 1,
        expected_source_workflow_state_version: version,
        operation: serde_json::from_value(
            m["currentNode"]["operation"]["allowedOperations"][0].clone(),
        )
        .unwrap(),
        arguments: json!({"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null}),
    };
    let (a, c) = tokio::join!(
        effects::execute(
            &p,
            owner,
            Some(pm),
            crate::common::admitting_gate(),
            managed,
            visit,
            "create-dependency",
            b.clone()
        ),
        effects::execute(
            &p,
            owner,
            Some(pm),
            crate::common::admitting_gate(),
            managed,
            visit,
            "create-dependency",
            b.clone()
        )
    );
    assert!(a.is_ok(), "{a:?}");
    assert!(c.is_ok(), "{c:?}");
    let a = a.unwrap();
    let c = c.unwrap();
    assert_eq!(a["effectId"], c["effectId"]);
    assert_eq!(a["result"], c["result"]);
    let after = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    assert_eq!(
        after["currentNode"]["operation"]["effect"]["result"],
        a["result"]
    );
    let receipt: Value = sqlx::query_scalar(
        "SELECT response_body FROM workflow_command_receipts WHERE command_id=$1",
    )
    .bind(a["commandId"].as_str().unwrap().parse::<Uuid>().unwrap())
    .fetch_one(&p)
    .await
    .unwrap();
    assert_eq!(receipt, a["result"]);
    assert!(receipt.get("effectId").is_none());
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_owner_dependency_origins")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(n, 1);
    let mut changed = b;
    changed.arguments["contextPayload"] = json!({"changed":true});
    assert_eq!(
        effects::execute(
            &p,
            owner,
            Some(pm),
            crate::common::admitting_gate(),
            managed,
            visit,
            "create-dependency",
            changed
        )
        .await
        .unwrap_err()
        .code,
        "idempotency_conflict"
    );
}
#[tokio::test]
async fn effects_known_failure_receipt_and_absorbing_stop_are_zero_business_facts() {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    use svc_workflow::store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::{effects, reads},
    };
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit: Uuid = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    let version: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let b = effects::Execute {
        expected_workflow_state_version: 99,
        expected_source_workflow_state_version: version,
        operation: serde_json::from_value(
            m["currentNode"]["operation"]["allowedOperations"][0].clone(),
        )
        .unwrap(),
        arguments: json!({"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null}),
    };
    assert_eq!(
        effects::execute(
            &p,
            owner,
            Some(pm),
            crate::common::admitting_gate(),
            managed,
            visit,
            "create-dependency",
            b
        )
        .await
        .unwrap_err()
        .code,
        "workflow_state_version_conflict"
    );
    let pair:(String,i32,i64)=sqlx::query_as("SELECT e.state,r.response_status,(SELECT count(*) FROM workflow_events x WHERE x.command_id=e.command_id) FROM workflow_operation_effects e JOIN workflow_command_receipts r ON r.command_id=e.command_id").fetch_one(&p).await.unwrap();
    assert_eq!(pair, ("REJECTED_ZERO_FACTS".into(), 409, 0));
    let state = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    assert_eq!(state["state"], "STOPPED");
    assert_eq!(
        state["currentNode"]["operation"]["effect"]["state"],
        "REJECTED_ZERO_FACTS"
    );
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_owner_dependency_origins")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(n, 0);
}
#[tokio::test]
async fn effects_return_visits_reuse_same_effect_and_third_return_stops_with_actual_event() {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) =
        operation_fixture_return(&p, true).await;
    use svc_workflow::store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::{effects, reads},
    };
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    let sv: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let b = effects::Execute {
        expected_workflow_state_version: 1,
        expected_source_workflow_state_version: sv,
        operation: serde_json::from_value(
            m["currentNode"]["operation"]["allowedOperations"][0].clone(),
        )
        .unwrap(),
        arguments: json!({"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null}),
    };
    let result = effects::execute(
        &p,
        owner,
        Some(pm),
        crate::common::admitting_gate(),
        managed,
        visit,
        "create-dependency",
        b,
    )
    .await
    .unwrap();
    let mut version = 1;
    for i in 0..3 {
        let advance:Uuid=sqlx::query_scalar("SELECT n.primary_advance_transition_id FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n USING(node_id) WHERE w.workflow_instance_id=$1").bind(managed).fetch_one(&p).await.unwrap();
        transition(&p, owner, managed, version, advance, None)
            .await
            .unwrap();
        version += 1;
        let (qa,back):(Uuid,Uuid)=sqlx::query_as("SELECT v.assignee_principal_id,t.transition_id FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_transition_definitions t ON t.source_node_id=v.node_id AND t.transition_effect='RETURN' WHERE w.workflow_instance_id=$1").bind(managed).fetch_one(&p).await.unwrap();
        transition(&p,qa,managed,version,back,Some(json!({"rootCauseNodeVisitId":visit,"reasonCode":"QA_FAILED","reason":"Real QA return","relatedSubmissionIds":[]}))).await.unwrap();
        version += 1;
        let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
        assert_eq!(
            m["currentNode"]["operation"]["effect"]["effectId"],
            result["effectId"]
        );
        if i < 2 {
            assert_eq!(m["state"], "ACTIVE")
        } else {
            assert_eq!(m["state"], "STOPPED")
        }
    }
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_operation_effects")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(n, 1);
    let row:(String,Uuid,Uuid)=sqlx::query_as("SELECT stop_code,origin_event_id,node_visit_id FROM workflow_owner_obligation_intents WHERE kind='STOP'").fetch_one(&p).await.unwrap();
    assert_eq!(row.0, "RETURN_LIMIT_REACHED");
    let real:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_events WHERE event_id=$1 AND event_type='WORKFLOW_TRANSITION_COMMITTED' AND target_node_visit_id=$2)").bind(row.1).bind(row.2).fetch_one(&p).await.unwrap();
    assert!(real);
}
#[tokio::test]
async fn effects_storage_fault_after_real_create_rolls_back_claim_receipt_child_and_origin() {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    use svc_workflow::store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::{effects, reads},
    };
    let before = history(&p).await;
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    let sv: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    sqlx::raw_sql("CREATE FUNCTION isolated_reject_origin() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'isolated fault after actual ordinary birth'; END $$; CREATE TRIGGER isolated_reject_origin BEFORE INSERT ON workflow_owner_dependency_origins FOR EACH ROW EXECUTE FUNCTION isolated_reject_origin();").execute(&p).await.unwrap();
    let b = effects::Execute {
        expected_workflow_state_version: 1,
        expected_source_workflow_state_version: sv,
        operation: serde_json::from_value(
            m["currentNode"]["operation"]["allowedOperations"][0].clone(),
        )
        .unwrap(),
        arguments: json!({"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null}),
    };
    assert!(effects::execute(
        &p,
        owner,
        Some(pm),
        crate::common::admitting_gate(),
        managed,
        visit,
        "create-dependency",
        b
    )
    .await
    .is_err());
    assert_eq!(history(&p).await, before);
    let count:(i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM workflow_operation_effects),(SELECT count(*) FROM workflow_owner_dependency_origins)").fetch_one(&p).await.unwrap();
    assert_eq!(count, (0, 0));
    assert_eq!(
        reads::managed(&p, owner, managed, Some(pm)).await.unwrap()["currentNode"]["operation"]
            ["effect"]["state"],
        "NOT_STARTED"
    );
}
