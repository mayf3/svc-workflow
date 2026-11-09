//! Linked Assistance uses real accepted child terminal facts, never predicted IDs.
use super::{common,support as aa};
use serde_json::{json,Value};use sqlx::PgPool;use uuid::Uuid;
#[path="managed_fixture.rs"] mod support;
use support::*;
fn has(d:&Value,op:&str)->bool {d["authorized_actions"].as_array().unwrap().iter().any(|a|a["operation"]==op)}
#[tokio::test]
async fn linked_child_resolve_requires_actual_accepted_terminal_proof() {
    for verdict in ["ACCEPTED","NOT_ACCEPTED"] {
        let p=aa::fresh_pool(&format!("linked{}",verdict.to_lowercase())).await;let(owner,domain)=seed(&p).await;
        let(source_version,_)=version(&p,domain,owner,json!({"type":"object"})).await;publish(&p,owner,source_version,None,"source").await.unwrap();
        let(child_version,terminal)=version(&p,domain,owner,json!({"type":"object"})).await;
        sqlx::query("UPDATE workflow_transition_definitions SET submission_schema=$2 WHERE transition_id=$1").bind(terminal).bind(acceptance_schema()).execute(&p).await.unwrap();publish(&p,owner,child_version,None,"child").await.unwrap();
        let source=create(&p,owner,domain,source_version,json!({})).await.unwrap();let child=create(&p,owner,domain,child_version,json!({})).await.unwrap();
        let jwks=common::MockJwksServer::start().await;let app=aa::app(&p,&jwks,true);let tok=aa::token(&jwks,owner,"workflow.execute workflow.read");
        let case=aa::require_http(&app,"POST",&format!("{}/assistance-cases",aa::path(source.workflow_instance_id)),&tok,json!({"currentNodeVisitId":source.current_node_visit_id,"expectedWorkflowStateVersion":1,"request":{"message":"actual linked independent review"}}),201,&format!("linked-{verdict}-request")).await;
        let case_id=case["assistanceCaseId"].as_str().unwrap();let binding=aa::require_http(&app,"POST",&format!("/internal/v1/assistance-cases/{case_id}/governance-child"),&tok,json!({"governanceChildInstanceId":child.workflow_instance_id,"acceptanceTransitionId":terminal,"expectedWorkflowStateVersion":2}),200,&format!("linked-{verdict}-binding")).await;
        let pending=aa::observed_detail(&p,&app,source.workflow_instance_id,&tok,&format!("linked-{verdict}-pending-detail")).await;assert!(!has(&pending,"resolve_assistance"));assert!(has(&pending,"escalate_assistance"));
        let(s,refusal)=aa::send(&app,"POST",&format!("/internal/v1/assistance-cases/{case_id}/resolve"),&tok,Some(json!({"expectedWorkflowStateVersion":3,"resolution":{"message":"child not terminal"}}))).await;assert!(s>=400&&s<500,"{refusal}");
        let payload=json!({"governanceAcceptance":{"bindingId":binding["bindingId"],"acceptanceContractDigest":binding["acceptanceContractDigest"],"sourceRequestPayloadDigest":binding["source"]["requestPayloadDigest"],"verdict":verdict}});
        aa::require_http(&app,"POST",&format!("{}/transitions",aa::path(child.workflow_instance_id)),&tok,aa::transition_body(terminal,1,payload),200,&format!("linked-{verdict}-actual-terminal")).await;
        let terminal_detail=aa::observed_detail(&p,&app,source.workflow_instance_id,&tok,&format!("linked-{verdict}-terminal-detail")).await;
        assert_eq!(has(&terminal_detail,"resolve_assistance"),verdict=="ACCEPTED");
        let(s,case_detail)=aa::send(&app,"GET",&format!("/internal/v1/assistance-cases/{case_id}"),&tok,None).await;assert_eq!(s,200,"{case_detail}");
        let proof=case_detail["governanceTerminalProof"].clone();let review=json!({"bindingId":binding["bindingId"],"terminalEventId":proof["terminalEventId"],"submissionId":proof["submissionId"],"submissionDigest":proof["submissionDigest"],"acceptanceContractDigest":binding["acceptanceContractDigest"]});
        let(s,response)=aa::send(&app,"POST",&format!("/internal/v1/assistance-cases/{case_id}/resolve"),&tok,Some(json!({"expectedWorkflowStateVersion":3,"resolution":{"message":"literal exact child proof","supportingPayload":{"governanceChildReview":review}}}))).await;
        if verdict=="ACCEPTED" {assert_eq!(s,200,"{response}");}else{assert!(s>=400&&s<500,"{response}");}
        aa::persist_probe(&format!("linked-{verdict}-proof-result"),&json!({"binding":binding,"proof":proof,"status":s,"response":response}));
    }
}
