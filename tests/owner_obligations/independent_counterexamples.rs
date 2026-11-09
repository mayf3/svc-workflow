use super::*;
#[tokio::test]
async fn independent_reject_same_principal_qa_under_different_assignee_spellings() {
    let p = pool().await;
    let (owner, domain) = seed(&p).await;
    let pm = actor(&p).await;
    let (intake, it) = managed_version(&p, domain, owner, "INTAKE").await;
    // Both TASKs resolve to the same current Owner, despite distinct ref strings.
    sqlx::query("UPDATE workflow_node_definitions SET assignee_ref_type='DOMAIN_OWNER',fixed_principal_id=NULL WHERE definition_version_id=$1 AND node_key='qa'").bind(intake).execute(&p).await.unwrap();
    publish(&p, owner, intake, None, "bad-intake")
        .await
        .unwrap();
    let (review, rt) = managed_version(&p, domain, owner, "REVIEW").await;
    publish(&p, owner, review, None, "review").await.unwrap();
    let schema = json!({"type":"object","required":["ownerIntakeContext","ownerReviewContext"],"properties":{"ownerIntakeContext":{"type":"object"},"ownerReviewContext":{"type":"object"}},"x-workflow-owner-obligation-v1":{"version":1,"intakeDefinitionVersionId":intake,"intakeAcceptanceTransitionId":it,"reviewDefinitionVersionId":review,"reviewCompletionTransitionId":rt}});
    let (source, _) = version(&p, domain, owner, schema).await;
    let r = publish(&p, owner, source, Some(pm), "policy").await;
    println!("SAME_PRINCIPAL_QA_POLICY_ACCEPTED={}", r.is_ok());
    assert!(r.is_err(),"policy publication must require independent QA, not different ref spellings for the same actor");
}
#[tokio::test]
async fn independent_reject_succeeded_create_effect_without_real_origin() {
    let p = pool().await;
    let (owner, domain, _, managed, supplier, _, _) = operation_fixture(&p).await;
    let leaf = create(&p, owner, domain, supplier, json!({}))
        .await
        .unwrap();
    let fake_command = Uuid::new_v4();
    let effect = Uuid::new_v4();
    let mut tx = p.begin().await.unwrap();
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status,response_status,response_body,response_digest) SELECT $1,principal_id,$2,command_type,request_hash,receipt_status,response_status,response_body,response_digest FROM workflow_command_receipts WHERE command_type='CREATE_WORKFLOW_INSTANCE' AND response_body->>'workflowInstanceId'=$3")
 .bind(fake_command).bind(Uuid::new_v4().to_string()).bind(leaf.workflow_instance_id.to_string()).execute(&mut *tx).await.unwrap();
    sqlx::query("INSERT INTO workflow_operation_effects(effect_id,case_id,managed_instance_id,definition_version_id,node_definition_id,slot_key,descriptor_key,descriptor_version,descriptor_hash,definition_graph_digest,first_authorizing_visit_id,actor_principal_id,business_intent_digest,concrete_request_digest,state,command_id,result,result_digest) SELECT $1,m.case_id,w.workflow_instance_id,w.definition_version_id,n.node_id,n.metadata->'workflowOperation'->>'slotKey',d->>'key',(d->>'version')::int,d->>'hash',a.intake_graph_digest,v.node_visit_id,$2,$3,$3,'SUCCEEDED',$4,r.response_body,r.response_digest FROM workflow_owner_managed_instances m JOIN workflow_instances w ON w.workflow_instance_id=m.workflow_instance_id JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n ON n.node_id=v.node_id JOIN workflow_owner_policy_approvals a ON a.source_definition_version_id=m.policy_source_version_id CROSS JOIN LATERAL jsonb_array_elements(n.metadata->'workflowOperation'->'allowedOperations') d JOIN workflow_command_receipts r ON r.command_id=$4 WHERE m.workflow_instance_id=$5")
 .bind(effect).bind(owner).bind("a".repeat(64)).bind(fake_command).bind(managed).execute(&mut *tx).await.unwrap();
    let committed = tx.commit().await;
    println!(
        "ZERO_BIRTH_SUCCEEDED_EFFECT_COMMITTED={}",
        committed.is_ok()
    );
    assert!(committed.is_err(),"SUCCEEDED create effect must require actual command birth and its causal origin, even when no origin row was supplied");
}

#[tokio::test]
async fn independent_reject_unrelated_third_actor_masking_selected_qa_self_review() {
    let p = pool().await;
    let (owner, domain) = seed(&p).await;
    let pm = actor(&p).await;
    let (intake, it) = managed_version(&p, domain, owner, "INTAKE").await;
    let qa: Uuid = sqlx::query_scalar(
        "SELECT source_node_id FROM workflow_transition_definitions WHERE transition_id=$1",
    )
    .bind(it)
    .fetch_one(&p)
    .await
    .unwrap();
    sqlx::query("UPDATE workflow_node_definitions SET assignee_ref_type='DOMAIN_OWNER',fixed_principal_id=NULL WHERE node_id=$1").bind(qa).execute(&p).await.unwrap();
    sqlx::query("UPDATE workflow_node_definitions SET order_index=3 WHERE definition_version_id=$1 AND node_type='TERMINAL'").bind(intake).execute(&p).await.unwrap();
    sqlx::query("UPDATE workflow_node_definitions SET order_index=2 WHERE node_id=$1")
        .bind(qa)
        .execute(&p)
        .await
        .unwrap();
    let extra = Uuid::new_v4();
    let extra_actor = actor(&p).await;
    let advance = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_node_definitions(node_id,definition_version_id,node_key,display_name,order_index,node_type,assignee_ref_type,fixed_principal_id) VALUES($1,$2,'other-work','Other work',1,'TASK','FIXED_PRINCIPAL',$3)").bind(extra).bind(intake).bind(extra_actor).execute(&p).await.unwrap();
    sqlx::query("UPDATE workflow_transition_definitions SET target_node_id=$2 WHERE definition_version_id=$1 AND transition_key='qa_entry'").bind(intake).bind(extra).execute(&p).await.unwrap();
    sqlx::query("INSERT INTO workflow_transition_definitions(transition_id,definition_version_id,transition_key,display_name,source_node_id,target_node_id,transition_effect) VALUES($1,$2,'to-qa','QA',$3,$4,'ADVANCE')").bind(advance).bind(intake).bind(extra).bind(qa).execute(&p).await.unwrap();
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id=$2 WHERE node_id=$1",
    )
    .bind(extra)
    .bind(advance)
    .execute(&p)
    .await
    .unwrap();
    publish(&p, owner, intake, None, "intake").await.unwrap();
    let (review, rt) = managed_version(&p, domain, owner, "REVIEW").await;
    publish(&p, owner, review, None, "review").await.unwrap();
    let(source,_)=version(&p,domain,owner,json!({"type":"object","required":["ownerIntakeContext","ownerReviewContext"],"properties":{"ownerIntakeContext":{"type":"object"},"ownerReviewContext":{"type":"object"}},"x-workflow-owner-obligation-v1":{"version":1,"intakeDefinitionVersionId":intake,"intakeAcceptanceTransitionId":it,"reviewDefinitionVersionId":review,"reviewCompletionTransitionId":rt}})).await;
    assert!(publish(&p,owner,source,Some(pm),"policy").await.is_err(),"an unrelated different actor cannot make the selected Owner QA independent of earlier Owner work");
}

#[tokio::test]
async fn independent_reject_fixed_policy_actor_aliasing_real_workflow_creator() {
    let p = pool().await;
    let (owner, domain) = seed(&p).await;
    let pm = actor(&p).await;
    let (intake, it) = managed_version(&p, domain, owner, "INTAKE").await;
    sqlx::query("UPDATE workflow_node_definitions SET fixed_principal_id=$2 WHERE definition_version_id=$1 AND node_key='task'").bind(intake).bind(pm).execute(&p).await.unwrap();
    sqlx::query("UPDATE workflow_node_definitions SET assignee_ref_type='WORKFLOW_CREATOR',fixed_principal_id=NULL WHERE definition_version_id=$1 AND node_key='qa'").bind(intake).execute(&p).await.unwrap();
    publish(&p, owner, intake, None, "intake").await.unwrap();
    let (review, rt) = managed_version(&p, domain, owner, "REVIEW").await;
    publish(&p, owner, review, None, "review").await.unwrap();
    let(source,_)=version(&p,domain,owner,json!({"type":"object","required":["ownerIntakeContext","ownerReviewContext"],"properties":{"ownerIntakeContext":{"type":"object"},"ownerReviewContext":{"type":"object"}},"x-workflow-owner-obligation-v1":{"version":1,"intakeDefinitionVersionId":intake,"intakeAcceptanceTransitionId":it,"reviewDefinitionVersionId":review,"reviewCompletionTransitionId":rt}})).await;
    assert!(
        publish(&p, owner, source, Some(pm), "policy")
            .await
            .is_err(),
        "AUTO birth creator is the configured policy actor, not the approving Owner"
    );
}

#[tokio::test]
async fn independent_reject_assistance_success_receipt_without_actual_case_event() {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    use svc_workflow::store::postgres::{
        admission_gate::AdmissionGate,
        workflow_instance_repository::owner_obligation::{effects, reads, reconcile},
    };
    let m = reads::managed(&p, owner, managed, Some(pm)).await.unwrap();
    let visit = serde_json::from_value(m["currentNode"]["nodeVisitId"].clone()).unwrap();
    effects::execute(&p,owner,Some(pm),crate::common::admitting_gate(),managed,visit,"create-dependency",effects::Execute{expected_workflow_state_version:1,expected_source_workflow_state_version:3,operation:serde_json::from_value(m["currentNode"]["operation"]["allowedOperations"][0].clone()).unwrap(),arguments:json!({"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null})}).await.unwrap();
    let advance =
        super::context_support::primary(&p, super::context_support::current(&p, managed).await.3)
            .await;
    transition(&p, owner, managed, 1, advance, None)
        .await
        .unwrap();
    let (qa,t):(Uuid,Uuid)=sqlx::query_as("SELECT v.assignee_principal_id,n.primary_advance_transition_id FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n USING(node_id) WHERE w.workflow_instance_id=$1").bind(managed).fetch_one(&p).await.unwrap();
    let binding:Value=sqlx::query_scalar("SELECT receipt FROM workflow_governance_child_bindings WHERE governance_child_instance_id=$1").bind(managed).fetch_one(&p).await.unwrap();
    transition(&p,qa,managed,2,t,Some(json!({"governanceAcceptance":{"bindingId":binding["bindingId"],"acceptanceContractDigest":binding["acceptanceContractDigest"],"sourceRequestPayloadDigest":binding["source"]["requestPayloadDigest"],"verdict":"ACCEPTED"}}))).await.unwrap();
    reconcile::reconcile_pending(&p, pm, crate::common::admitting_gate())
        .await
        .unwrap();
    let review: Uuid = sqlx::query_scalar(
        "SELECT workflow_instance_id FROM workflow_owner_managed_instances WHERE kind='REVIEW'",
    )
    .fetch_one(&p)
    .await
    .unwrap();
    let case: Uuid = sqlx::query_scalar(
        "SELECT assistance_case_id FROM workflow_assistance_cases WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    for (descriptor, command_type, status) in [
        (
            "workflow.assistance.resolve_current_review",
            "RESOLVE_WORKFLOW_ASSISTANCE",
            "RESOLVED",
        ),
        (
            "workflow.assistance.escalate_to_human",
            "ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN",
            "HUMAN_REQUIRED",
        ),
    ] {
        let cmd = Uuid::new_v4();
        let body = json!({"assistanceCaseId":case,"status":status});
        let digest = svc_workflow::domain::definition::digest::compute_json_digest(&body).unwrap();
        let mut tx = p.begin().await.unwrap();
        sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status,response_status,response_body,response_digest) VALUES($1,$2,$3,$4,$5,'COMPLETED',200,$6,$7)").bind(cmd).bind(owner).bind(Uuid::new_v4().to_string()).bind(command_type).bind("a".repeat(64)).bind(body).bind(digest).execute(&mut *tx).await.unwrap();
        sqlx::query("INSERT INTO workflow_operation_effects(effect_id,case_id,managed_instance_id,definition_version_id,node_definition_id,slot_key,descriptor_key,descriptor_version,descriptor_hash,definition_graph_digest,first_authorizing_visit_id,actor_principal_id,business_intent_digest,concrete_request_digest,state,command_id,result,result_digest) SELECT $1,m.case_id,w.workflow_instance_id,w.definition_version_id,n.node_id,n.metadata->'workflowOperation'->>'slotKey',d->>'key',(d->>'version')::int,d->>'hash',a.review_graph_digest,v.node_visit_id,$2,$3,$3,'SUCCEEDED',$4,r.response_body,r.response_digest FROM workflow_owner_managed_instances m JOIN workflow_instances w ON w.workflow_instance_id=m.workflow_instance_id JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n ON n.node_id=v.node_id JOIN workflow_owner_policy_approvals a ON a.source_definition_version_id=m.policy_source_version_id CROSS JOIN LATERAL jsonb_array_elements(n.metadata->'workflowOperation'->'allowedOperations') d JOIN workflow_command_receipts r ON r.command_id=$4 WHERE m.workflow_instance_id=$5 AND d->>'key'=$6").bind(Uuid::new_v4()).bind(owner).bind("a".repeat(64)).bind(cmd).bind(review).bind(descriptor).execute(&mut *tx).await.unwrap();
        assert!(tx.commit().await.is_err(),"{descriptor} cannot succeed from a fake completed receipt without actual case command/Event");
    }
}
