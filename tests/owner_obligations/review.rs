use super::*;

#[tokio::test]
async fn review_real_terminal_singleton_without_wake_resolve_and_own_cleanup() {
    review_roundtrip("ACCEPTED").await;
}
#[tokio::test]
async fn review_not_accepted_has_no_resolve_admission_and_escalates_then_finishes() {
    review_roundtrip("NOT_ACCEPTED").await;
}
async fn review_roundtrip(verdict: &str) {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    use svc_workflow::store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::{effects, reads, reconcile},
    };
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit: Uuid = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
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
    effects::execute(
        &p,
        owner,
        Some(pm),
        AdmissionGate::disabled(),
        managed,
        visit,
        "create-dependency",
        b,
    )
    .await
    .unwrap();
    let advance:Uuid=sqlx::query_scalar("SELECT n.primary_advance_transition_id FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n USING(node_id) WHERE w.workflow_instance_id=$1").bind(managed).fetch_one(&p).await.unwrap();
    transition(&p, owner, managed, 1, advance, None)
        .await
        .unwrap();
    let (qa,t):(Uuid,Uuid)=sqlx::query_as("SELECT v.assignee_principal_id,n.primary_advance_transition_id FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n USING(node_id) WHERE w.workflow_instance_id=$1").bind(managed).fetch_one(&p).await.unwrap();
    let binding:Value=sqlx::query_scalar("SELECT receipt FROM workflow_governance_child_bindings WHERE governance_child_instance_id=$1").bind(managed).fetch_one(&p).await.unwrap();
    let acceptance = json!({"governanceAcceptance":{"bindingId":binding["bindingId"],"acceptanceContractDigest":binding["acceptanceContractDigest"],"sourceRequestPayloadDigest":binding["source"]["requestPayloadDigest"],"verdict":verdict}});
    transition(&p, qa, managed, 2, t, Some(acceptance))
        .await
        .unwrap();
    let births = reconcile::reconcile_pending(&p, pm, AdmissionGate::disabled())
        .await
        .unwrap();
    assert_eq!(births.len(), 1);
    let review: Uuid = serde_json::from_value(births[0]["workflowInstanceId"].clone()).unwrap();
    let m = reads::managed(&p, owner, review, Some(pm)).await.unwrap();
    assert_eq!(m["managedKind"], "REVIEW");
    let resolving = verdict == "ACCEPTED";
    let outcome = if resolving {
        "RESOLVED"
    } else {
        "HUMAN_REQUIRED"
    };
    let proof = if resolving {
        assert_eq!(m["currentNode"]["reviewAdmission"]["status"], "ADMISSIBLE");
        let p = m["currentNode"]["reviewAdmission"]["proof"].clone();
        assert_eq!(p.as_object().unwrap().len(), 5);
        Some(p)
    } else {
        assert!(m["currentNode"].get("reviewAdmission").is_none());
        None
    };
    let wakes: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_governance_child_reviews")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(wakes, 0);
    let sv: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let visit: Uuid = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    let b = effects::Execute {
        expected_workflow_state_version: 1,
        expected_source_workflow_state_version: sv,
        operation: serde_json::from_value(
            m["currentNode"]["operation"]["allowedOperations"][if resolving { 0 } else { 1 }]
                .clone(),
        )
        .unwrap(),
        arguments: if resolving {
            json!({"assistanceCaseId":m["caseId"],"expectedWorkflowStateVersion":sv,"resolution":{"message":"Verified exact independent QA","supportingPayload":{"governanceChildReview":proof.unwrap()}}})
        } else {
            json!({"assistanceCaseId":m["caseId"],"expectedWorkflowStateVersion":sv,"escalation":{"message":"Independent QA did not accept; human disposition required"}})
        },
    };
    let result = effects::execute(
        &p,
        owner,
        Some(pm),
        AdmissionGate::disabled(),
        review,
        visit,
        "owner-decision",
        b.clone(),
    )
    .await
    .unwrap();
    assert_eq!(result["result"]["status"], outcome);
    let replay = effects::execute(
        &p,
        owner,
        Some(pm),
        AdmissionGate::disabled(),
        review,
        visit,
        "owner-decision",
        b.clone(),
    )
    .await
    .unwrap();
    assert_eq!(replay["result"], result["result"]);
    assert_eq!(replay["replayed"], true);
    if resolving {
        let jwk = common::MockJwksServer::start().await;
        let app = http_support::app(&p, &jwk, pm).await;
        let (status, detail) = http_support::http(
            app.clone(),
            owner,
            &jwk,
            "GET",
            &format!(
                "/internal/v1/assistance-cases/{}",
                m["caseId"].as_str().unwrap()
            ),
            None,
            json!({}),
        )
        .await;
        assert_eq!(status, 200, "{detail}");
        assert_eq!(detail["workflowInstanceId"], json!(source));
        assert_eq!(detail["assistanceCaseId"], m["caseId"]);
        assert_eq!(detail["resolution"], b.arguments["resolution"]);
        let responsibilities = detail["ownerObligation"]["responsibilities"]
            .as_array()
            .unwrap();
        assert_eq!(responsibilities.len(), 2);
        assert_eq!(responsibilities[0]["kind"], "INTAKE");
        assert_eq!(responsibilities[0]["state"], "COMPLETED");
        assert_eq!(responsibilities[1]["kind"], "REVIEW");
        assert_eq!(responsibilities[1]["state"], "MATERIALIZED");
        assert_eq!(
            responsibilities[1]["managedWorkflowInstanceId"],
            json!(review)
        );

        assert_ne!(detail["workflowStateVersion"], json!(sv));
        let stored_proof = detail
            .pointer("/resolution/supportingPayload/governanceChildReview")
            .unwrap();
        assert_eq!(stored_proof.as_object().unwrap().len(), 5);
        assert_eq!(
            stored_proof["bindingId"],
            detail["governanceBinding"]["bindingId"]
        );
        let mut restored = b.clone();
        restored.arguments["resolution"]["supportingPayload"]["governanceChildReview"] =
            stored_proof.clone();
        let url=format!("/internal/v1/workflow-instances/{review}/node-visits/{visit}/operation-effects/owner-decision:execute");
        let (status, same) = http_support::http(
            app.clone(),
            owner,
            &jwk,
            "POST",
            &url,
            None,
            serde_json::to_value(restored.clone()).unwrap(),
        )
        .await;
        assert_eq!(status, 200, "{same}");
        assert_eq!(same["replayed"], true);
        assert_eq!(same["result"], result["result"]);
        restored.arguments["resolution"]["message"] =
            json!("Different materialized message after success");
        let (status, conflict) = http_support::http(
            app.clone(),
            owner,
            &jwk,
            "POST",
            &url,
            None,
            serde_json::to_value(restored).unwrap(),
        )
        .await;
        assert_eq!(status, 409, "{conflict}");
        assert_eq!(conflict["error"]["code"], "idempotency_conflict");
        println!("actual HTTP resolved case detail for immutable proof reconstruction: {detail}");
        let (status, projection) = http_support::http(
            app.clone(),
            owner,
            &jwk,
            "GET",
            &format!("/internal/v1/workflow-instances/{review}/owner-obligation"),
            None,
            json!({}),
        )
        .await;
        assert_eq!(status, 200, "{projection}");
        assert!(projection["currentNode"].get("reviewAdmission").is_none());
        assert_eq!(
            projection["currentNode"]["operation"]["effect"]["state"],
            "SUCCEEDED"
        );
        let mut changed = b;
        changed.operation =
            serde_json::from_value(m["currentNode"]["operation"]["allowedOperations"][1].clone())
                .unwrap();
        changed.arguments = json!({"assistanceCaseId":m["caseId"],"expectedWorkflowStateVersion":sv,"escalation":{"message":"Changed allowed descriptor after actual resolve"}});
        let (status,error)=http_support::http(app,owner,&jwk,"POST",&format!("/internal/v1/workflow-instances/{review}/node-visits/{visit}/operation-effects/owner-decision:execute"),None,serde_json::to_value(changed).unwrap()).await;
        assert_eq!(status, 409, "{error}");
        assert_eq!(error["error"]["code"], "idempotency_conflict");
        println!("actual HTTP own-resolved changed-descriptor refusal: {error}");
    }
    let after = reads::managed(&p, owner, review, Some(pm)).await.unwrap();
    assert_eq!(after["state"], "ACTIVE");
    assert!(after["currentNode"].get("reviewAdmission").is_none());
    let first:Uuid=sqlx::query_scalar("SELECT n.primary_advance_transition_id FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n USING(node_id) WHERE w.workflow_instance_id=$1").bind(review).fetch_one(&p).await.unwrap();
    transition(&p, owner, review, 1, first, None).await.unwrap();
    let (qa,last):(Uuid,Uuid)=sqlx::query_as("SELECT v.assignee_principal_id,n.primary_advance_transition_id FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n USING(node_id) WHERE w.workflow_instance_id=$1").bind(review).fetch_one(&p).await.unwrap();
    transition(
        &p,
        qa,
        review,
        2,
        last,
        Some(json!({"reviewOutcome":outcome})),
    )
    .await
    .unwrap();
    assert_eq!(
        reads::managed(&p, owner, review, Some(pm)).await.unwrap()["state"],
        "COMPLETED"
    );
    assert!(
        reconcile::reconcile_pending(&p, pm, AdmissionGate::disabled())
            .await
            .unwrap()
            .is_empty()
    );
    let review_events:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_events e JOIN workflow_command_receipts r USING(command_id) WHERE r.command_type='AUTO_CREATE_OWNER_REVIEW'").fetch_one(&p).await.unwrap();
    assert_eq!(review_events, 1);
    println!("real managed REVIEW admission: {m}; original resolve result: {result}");
}
