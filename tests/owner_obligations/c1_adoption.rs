use super::*;
use svc_workflow::application::workflow_instance::assistance::adopt_stock_owner_intake;
use svc_workflow::domain::{ids::{AssistanceCaseId, NodeVisitId, PrincipalId}, workflow_instance::assistance::AdoptStockOwnerIntakeCommand};

async fn adopt(p: &PgPool, owner: Uuid, case: Uuid, visit: Uuid, version: i32, key: &str) -> Result<svc_workflow::domain::workflow_instance::assistance::AssistanceCommandResult, svc_workflow::domain::workflow_instance::assistance::AssistanceError> {
    adopt_stock_owner_intake(p, AdoptStockOwnerIntakeCommand { principal_id: PrincipalId::from_uuid(owner), idempotency_key:key.into(), command_schema_version:"v1".into(), assistance_case_id:AssistanceCaseId::from_uuid(case), current_node_visit_id:NodeVisitId::from_uuid(visit), expected_workflow_state_version:version }).await
}

#[tokio::test]
async fn c1_approval_missing_is_rejected_for_human_required_case() {
    let p=pool().await; let (owner,domain)=seed(&p).await; let (source,_)=version(&p,domain,owner,json!({})).await; publish(&p,owner,source,None,"plain").await.unwrap();
    let s=create(&p,owner,domain,source,json!({})).await.unwrap(); let c=request(&p,owner,s.workflow_instance_id,s.current_node_visit_id).await;
    let jwks=common::MockJwksServer::start().await; let app=http_support::app(&p,&jwks,owner).await;
    let (status,_)=http_support::http(app,owner,&jwks,"POST",&format!("/internal/v1/assistance-cases/{}/escalate-to-human",c.assistance_case_id),Some("esc"),json!({"expectedWorkflowStateVersion":2,"escalation":{"message":"human"}})).await;
    assert_eq!(status,200); assert_eq!(adopt(&p,owner,c.assistance_case_id,c.node_visit_id,3,"adopt-missing").await.unwrap_err().code(),"owner_policy_approval_missing");
}

#[tokio::test]
async fn c1_non_owner_is_rejected() {
    let p=pool().await; let (owner,domain,pm,source,_,_)=policy_fixture(&p).await; publish(&p,owner,source,Some(pm),"source").await.unwrap(); let s=create(&p,owner,domain,source,json!({"ownerIntakeContext":{},"ownerReviewContext":{}})).await.unwrap(); let c=request(&p,owner,s.workflow_instance_id,s.current_node_visit_id).await;
    let intent:Uuid=sqlx::query_scalar("SELECT intent_id FROM workflow_owner_obligation_intents WHERE case_id=$1 AND kind='INTAKE'").bind(c.assistance_case_id).fetch_one(&p).await.unwrap();
    let jwks=common::MockJwksServer::start().await; let app=http_support::app(&p,&jwks,owner).await; let path=format!("/internal/v1/assistance-cases/{}/escalate-to-human",c.assistance_case_id);
    let (st,_)=http_support::http(app.clone(),owner,&jwks,"POST",&path,Some("esc"),json!({"expectedWorkflowStateVersion":2,"escalation":{"message":"human"}})).await; assert_eq!(st,409);
    let (st,_)=http_support::http(app,owner,&jwks,"POST",&path,Some("esc-cancel"),json!({"expectedWorkflowStateVersion":2,"intentCancelRef":intent,"escalation":{"message":"human"}})).await; assert_eq!(st,200);
    let outsider=actor(&p).await; assert_eq!(adopt(&p,outsider,c.assistance_case_id,c.node_visit_id,3,"adopt-outsider").await.unwrap_err().code(),"not_domain_owner");
}

#[tokio::test]
async fn c1_status_drift_is_rejected() {
    let p=pool().await; let (owner,domain,pm,source,_,_)=policy_fixture(&p).await; publish(&p,owner,source,Some(pm),"source").await.unwrap(); let s=create(&p,owner,domain,source,json!({"ownerIntakeContext":{},"ownerReviewContext":{}})).await.unwrap(); let c=request(&p,owner,s.workflow_instance_id,s.current_node_visit_id).await;
    assert_eq!(adopt(&p,owner,c.assistance_case_id,c.node_visit_id,2,"adopt-drift").await.unwrap_err().code(),"assistance_status_conflict");
}

#[tokio::test]
async fn c1_duplicate_adoption_replays_without_duplicate_intent() {
    let p=pool().await; let (owner,domain,pm,source,_,_)=policy_fixture(&p).await; publish(&p,owner,source,Some(pm),"source").await.unwrap(); let s=create(&p,owner,domain,source,json!({"ownerIntakeContext":{},"ownerReviewContext":{}})).await.unwrap(); let c=request(&p,owner,s.workflow_instance_id,s.current_node_visit_id).await;
    let intent:Uuid=sqlx::query_scalar("SELECT intent_id FROM workflow_owner_obligation_intents WHERE case_id=$1 AND kind='INTAKE'").bind(c.assistance_case_id).fetch_one(&p).await.unwrap(); let jwks=common::MockJwksServer::start().await; let app=http_support::app(&p,&jwks,owner).await; let path=format!("/internal/v1/assistance-cases/{}/escalate-to-human",c.assistance_case_id); let (st,_)=http_support::http(app.clone(),owner,&jwks,"POST",&path,Some("adopt-esc-no"),json!({"expectedWorkflowStateVersion":2,"escalation":{"message":"human"}})).await; assert_eq!(st,409); let (st,_)=http_support::http(app,owner,&jwks,"POST",&path,Some("adopt-esc"),json!({"expectedWorkflowStateVersion":2,"intentCancelRef":intent,"escalation":{"message":"human"}})).await; assert_eq!(st,200);
    let a=adopt(&p,owner,c.assistance_case_id,c.node_visit_id,3,"adopt-dup").await.unwrap(); let b=adopt(&p,owner,c.assistance_case_id,c.node_visit_id,3,"adopt-dup").await.unwrap(); assert!(!a.replayed); assert!(b.replayed);
    let n:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_owner_obligation_intents WHERE case_id=$1 AND kind='INTAKE'").bind(c.assistance_case_id).fetch_one(&p).await.unwrap(); assert_eq!(n,1);
}

#[tokio::test]
async fn guard_a_http_requires_cancel_ref_and_supersedes_intent() {
    let p=pool().await; let (owner,domain,pm,source,_,_)=policy_fixture(&p).await; publish(&p,owner,source,Some(pm),"source").await.unwrap(); let s=create(&p,owner,domain,source,json!({"ownerIntakeContext":{},"ownerReviewContext":{}})).await.unwrap(); let c=request(&p,owner,s.workflow_instance_id,s.current_node_visit_id).await;
    let intent:Uuid=sqlx::query_scalar("SELECT intent_id FROM workflow_owner_obligation_intents WHERE case_id=$1 AND kind='INTAKE'").bind(c.assistance_case_id).fetch_one(&p).await.unwrap(); let jwks=common::MockJwksServer::start().await; let app=http_support::app(&p,&jwks,owner).await;
    let path=format!("/internal/v1/assistance-cases/{}/escalate-to-human",c.assistance_case_id); let (st,body)=http_support::http(app.clone(),owner,&jwks,"POST",&path,Some("guard-no-ref"),json!({"expectedWorkflowStateVersion":2,"escalation":{"message":"human"}})).await; assert_eq!(st,409); assert!(body.to_string().contains("intent_active"), "unexpected Guard-A response: {body}");
    let (st,_)=http_support::http(app,owner,&jwks,"POST",&path,Some("guard-cancel"),json!({"expectedWorkflowStateVersion":2,"intentCancelRef":intent,"escalation":{"message":"human"}})).await; assert_eq!(st,200);
    let row:(String,String)=sqlx::query_as("SELECT state,stop_code FROM workflow_owner_obligation_intents WHERE intent_id=$1").bind(intent).fetch_one(&p).await.unwrap(); assert_eq!(row,("STOPPED".into(),"SUPERSEDED_BY_HUMAN_ESCALATION".into()));
}
