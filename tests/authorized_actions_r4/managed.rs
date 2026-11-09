use super::{common,support as aa};
use axum::{body::{to_bytes,Body},http::Request};
use serde_json::{json,Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
#[path="managed_fixture.rs"] mod support;
use support::*;
#[path="managed_http.rs"] mod http_support;
#[path="managed_context.rs"] mod context_support;

fn includes(d:&Value,id:Uuid)->bool {d["authorized_actions"].as_array().expect("accepted field required").iter().any(|a|a["operation"]=="transition" && a["transition_id"]==json!(id))}
fn has_projection(d:&Value) {assert_eq!(d["authorized_actions_scope"],"INSTANCE_TASK_AND_GOVERNANCE_V1","detail must expose the accepted endpoint-only action projection: {d}");}

#[tokio::test]
async fn managed_required_effect_and_real_waiting_producer_preserve_non_gate_return() {
    let p=aa::fresh_pool("managedgate").await;
    let f=context_support::fixture(&p).await;
    let jwks=common::MockJwksServer::start().await;let app=http_support::app(&p,&jwks,f.pm).await;
    let token=common::v1_token(f.owner,"workflow.execute workflow.read","gcr-http",300,&jwks.key_pair);
    let (v,c,_,node)=context_support::current(&p,f.managed).await;let advance=context_support::primary(&p,node).await;
    let pending=aa::observed_detail(&p,&app,f.managed,&token,"managed-pending-source-effect").await;
    let before=aa::raw_probe_facts(&p,f.managed,&[f.owner,f.pm]).await;
    let(s,b)=aa::send(&app,"POST",&format!("{}/transitions",aa::path(f.managed)),&token,Some(json!({"transitionDefinitionId":advance,"expectedWorkflowStateVersion":v,"submissionPayload":context_support::report(c)}))).await;
    let after=aa::raw_probe_facts(&p,f.managed,&[f.owner,f.pm]).await;
    aa::persist_probe("managed-pending-refusal",&json!({"status":s,"response":b,"before":before,"after":after}));assert!(s>=400&&s<500,"SEMANTIC_STOP: required-effect ADVANCE accepted/errored: {b}");for k in ["instance","events","visits","submissions"] {assert_eq!(before[k],after[k]);}
    let effect=context_support::effect(&p,&f).await;
    let supplier:Uuid=serde_json::from_value(effect["result"]["workflowInstanceId"].clone()).unwrap();
    let supplier_terminal:bool=sqlx::query_scalar("SELECT n.node_type='TERMINAL' FROM workflow_instances w JOIN workflow_node_visits v ON v.node_visit_id=w.current_node_visit_id JOIN workflow_node_definitions n USING(node_id) WHERE w.workflow_instance_id=$1").bind(supplier).fetch_one(&p).await.unwrap();assert!(!supplier_terminal,"fixture supplier is truly WAITING");
    let produced=aa::observed_detail(&p,&app,f.managed,&token,"managed-produced-waiting-entry").await;
    let mut context=context_support::initial();context["workflowPrerequisite"]=context_support::reference(&f,supplier);
    aa::require_http(&app,"POST",&format!("{}/transitions",aa::path(f.managed)),&token,json!({"transitionDefinitionId":advance,"expectedWorkflowStateVersion":v,"submissionPayload":context_support::report(c),"contextPayload":context}),200,"managed-real-waiting-entry-command").await;
    let (v,c,_,node)=context_support::current(&p,f.managed).await;let next=context_support::primary(&p,node).await;
    let non_gate=aa::observed_detail(&p,&app,f.managed,&token,"managed-non-gate-forward").await;
    aa::require_http(&app,"POST",&format!("{}/transitions",aa::path(f.managed)),&token,json!({"transitionDefinitionId":next,"expectedWorkflowStateVersion":v,"submissionPayload":context_support::report(c)}),200,"managed-non-gate-forward-command").await;
    let (v,_,visit,node)=context_support::current(&p,f.managed).await;let blocked_forward=context_support::primary(&p,node).await;
    let ret:Uuid=sqlx::query_scalar("SELECT transition_id FROM workflow_transition_definitions WHERE definition_version_id=(SELECT definition_version_id FROM workflow_instances WHERE workflow_instance_id=$1) AND transition_key='return-before-second-effect'").bind(f.managed).fetch_one(&p).await.unwrap();
    let return_without_effect=aa::observed_detail(&p,&app,f.managed,&token,"managed-source-pending-return-valid").await;
    aa::require_http(&app,"POST",&format!("{}/transitions",aa::path(f.managed)),&token,json!({"transitionDefinitionId":ret,"expectedWorkflowStateVersion":v,"submissionPayload":{"rootCauseNodeVisitId":visit,"reasonCode":"OTHER","reason":"source effect is requiredForAdvance only"}}),200,"managed-return-with-pending-source-effect-command").await;
    for d in [&pending,&produced,&non_gate,&return_without_effect] {has_projection(d);}
    assert!(!includes(&pending,advance));assert!(includes(&produced,advance),"real producer, payload still pending, must remain eligible");assert!(includes(&non_gate,next),"non-gate target must not add producer requirement");assert!(!includes(&return_without_effect,blocked_forward));assert!(includes(&return_without_effect,ret),"requiredForAdvance must not become requiredForReturn");
}

#[tokio::test]
async fn opted_in_source_ordinary_owner_actions_are_excluded_and_refused() {
    let p=aa::fresh_pool("managedsource").await;let(owner,_,pm,_,_,_,source)=operation_fixture(&p).await;
    let jwks=common::MockJwksServer::start().await;let app=http_support::app(&p,&jwks,pm).await;let token=common::v1_token(owner,"workflow.execute workflow.read","gcr-http",300,&jwks.key_pair);
    let d=aa::observed_detail(&p,&app,source,&token,"opted-in-source-owner-detail").await;
    let(case,version):(Uuid,i32)=sqlx::query_as("SELECT c.assistance_case_id,w.workflow_state_version FROM workflow_assistance_cases c JOIN workflow_instances w USING(workflow_instance_id) WHERE c.workflow_instance_id=$1 AND c.status='OWNER_PENDING'").bind(source).fetch_one(&p).await.unwrap();
    for (suffix,body) in [("escalate-to-human",json!({"expectedWorkflowStateVersion":version,"escalation":{"message":"ordinary endpoint has no managed effect provenance"}})),("resolve",json!({"expectedWorkflowStateVersion":version,"resolution":{"message":"ordinary endpoint has no managed effect provenance"}}))] {
        let before=aa::read_all_facts(&p).await;
        let(s,b)=aa::send(&app,"POST",&format!("/internal/v1/assistance-cases/{case}/{suffix}"),&token,Some(body)).await;
        let after=aa::read_all_facts(&p).await;
        aa::persist_probe(&format!("managed-ordinary-{suffix}-refusal"),&json!({"status":s,"response":b,"before":before,"after":after}));assert!(s>=400&&s<500,"SEMANTIC_STOP: ordinary managed action accepted or 5xx: {b}");
        for table in ["workflow_instances","workflow_events","workflow_assistance_cases","workflow_operation_effects"] {assert_eq!(before[table],after[table]);}
    }
    has_projection(&d);for a in d["authorized_actions"].as_array().unwrap() {assert!(!matches!(a["operation"].as_str(),Some("escalate_assistance"|"resolve_assistance")));}
}
