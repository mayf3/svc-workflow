//! Real producer/effect/reconcile setup, with HTTP detail and original transition execution.
use super::{common,support as aa};use axum::{body::{to_bytes,Body},http::Request};use tower::ServiceExt;use serde_json::{json,Value};use sqlx::PgPool;use uuid::Uuid;
#[path="managed_fixture.rs"] mod support;use support::*;
#[path="managed_http.rs"] mod http_support;
use svc_workflow::store::postgres::{admission_gate::AdmissionGate,workflow_instance_repository::owner_obligation::{effects,reads,reconcile}};
fn includes(d:&Value,id:Uuid)->bool {d["authorized_actions"].as_array().unwrap().iter().any(|a|a["operation"]=="transition"&&a["transition_id"]==json!(id))}
async fn current(p:&PgPool,id:Uuid)->(i32,Uuid,Uuid,Uuid) {sqlx::query_as("SELECT w.workflow_state_version,v.node_visit_id,v.assignee_principal_id,n.primary_advance_transition_id FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n USING(node_id) WHERE w.workflow_instance_id=$1").bind(id).fetch_one(p).await.unwrap()}
#[tokio::test]
async fn managed_review_nonterminal_and_terminal_preserve_effect_and_payload_boundaries() {
    for verdict in ["ACCEPTED","NOT_ACCEPTED"] {
        let p=aa::fresh_pool(&format!("review{}",verdict.to_lowercase())).await;let(owner,domain,pm,intake,supplier,_,source)=operation_fixture(&p).await;
        let m=reads::managed(&p,owner,intake,Some(pm)).await.unwrap();let (_,visit,_,_)=current(&p,intake).await;
        let sv:i32=sqlx::query_scalar("SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1").bind(source).fetch_one(&p).await.unwrap();
        effects::execute(&p,owner,Some(pm),AdmissionGate::disabled(),intake,visit,"create-dependency",effects::Execute{expected_workflow_state_version:1,expected_source_workflow_state_version:sv,operation:serde_json::from_value(m["currentNode"]["operation"]["allowedOperations"][0].clone()).unwrap(),arguments:json!({"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null})}).await.unwrap();
        let jwks=common::MockJwksServer::start().await;let app=http_support::app(&p,&jwks,pm).await;
        let owner_token=common::v1_token(owner,"workflow.execute workflow.read","gcr-http",300,&jwks.key_pair);
        let (_,_,_,first)=current(&p,intake).await;aa::require_http(&app,"POST",&format!("{}/transitions",aa::path(intake)),&owner_token,json!({"transitionDefinitionId":first,"expectedWorkflowStateVersion":1}),200,&format!("review-{verdict}-intake-forward")).await;
        let(_,_,qa,last)=current(&p,intake).await;let qa_token=common::v1_token(qa,"workflow.execute workflow.read","gcr-http",300,&jwks.key_pair);
        let binding:Value=sqlx::query_scalar("SELECT receipt FROM workflow_governance_child_bindings WHERE governance_child_instance_id=$1").bind(intake).fetch_one(&p).await.unwrap();
        aa::require_http(&app,"POST",&format!("{}/transitions",aa::path(intake)),&qa_token,aa::transition_body(last,2,json!({"governanceAcceptance":{"bindingId":binding["bindingId"],"acceptanceContractDigest":binding["acceptanceContractDigest"],"sourceRequestPayloadDigest":binding["source"]["requestPayloadDigest"],"verdict":verdict}})),200,&format!("review-{verdict}-intake-terminal")).await;
        let births=reconcile::reconcile_pending(&p,pm,AdmissionGate::disabled()).await.unwrap();assert_eq!(births.len(),1);let review:Uuid=serde_json::from_value(births[0]["workflowInstanceId"].clone()).unwrap();let(_,visit,_,forward)=current(&p,review).await;
        let pending=aa::observed_detail(&p,&app,review,&owner_token,&format!("review-{verdict}-effect-pending")).await;assert!(!includes(&pending,forward));
        let m=reads::managed(&p,owner,review,Some(pm)).await.unwrap();let sv:i32=sqlx::query_scalar("SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1").bind(source).fetch_one(&p).await.unwrap();let accepted=verdict=="ACCEPTED";let outcome=if accepted {"RESOLVED"}else{"HUMAN_REQUIRED"};
        let args=if accepted {json!({"assistanceCaseId":m["caseId"],"expectedWorkflowStateVersion":sv,"resolution":{"message":"real reviewed outcome","supportingPayload":{"governanceChildReview":m["currentNode"]["reviewAdmission"]["proof"]}}})}else{json!({"assistanceCaseId":m["caseId"],"expectedWorkflowStateVersion":sv,"escalation":{"message":"actual non-accepted outcome"}})};
        effects::execute(&p,owner,Some(pm),AdmissionGate::disabled(),review,visit,"owner-decision",effects::Execute{expected_workflow_state_version:1,expected_source_workflow_state_version:sv,operation:serde_json::from_value(m["currentNode"]["operation"]["allowedOperations"][if accepted {0}else{1}].clone()).unwrap(),arguments:args}).await.unwrap();
        let nonterminal=aa::observed_detail(&p,&app,review,&owner_token,&format!("review-{verdict}-nonterminal-effect-done")).await;assert!(includes(&nonterminal,forward),"nonterminal Review must not require terminal payload");
        aa::require_http(&app,"POST",&format!("{}/transitions",aa::path(review)),&owner_token,json!({"transitionDefinitionId":forward,"expectedWorkflowStateVersion":1}),200,&format!("review-{verdict}-forward")).await;
        let(_,_,qa,last)=current(&p,review).await;let qa_token=common::v1_token(qa,"workflow.execute workflow.read","gcr-http",300,&jwks.key_pair);let d=aa::observed_detail(&p,&app,review,&qa_token,&format!("review-{verdict}-terminal-selectable")).await;assert!(includes(&d,last),"real own effect and source outcome permit terminal selection");
        let before=aa::raw_probe_facts(&p,review,&[qa]).await;let(s,b)=aa::send(&app,"POST",&format!("{}/transitions",aa::path(review)),&qa_token,Some(aa::transition_body(last,2,json!({"reviewOutcome":if accepted {"HUMAN_REQUIRED"}else{"RESOLVED"}})))).await;assert_eq!(s,422,"{b}");let after=aa::raw_probe_facts(&p,review,&[qa]).await;for k in ["instance","events","visits","submissions"] {assert_eq!(before[k],after[k]);}
        aa::persist_probe(&format!("review-{verdict}-payload-refusal"),&json!({"status":s,"response":b,"before":before,"after":after}));
        aa::require_http(&app,"POST",&format!("{}/transitions",aa::path(review)),&qa_token,aa::transition_body(last,2,json!({"reviewOutcome":outcome})),200,&format!("review-{verdict}-terminal-commit")).await;
    }
}
