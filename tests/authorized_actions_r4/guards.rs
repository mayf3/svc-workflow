//! Additional #1344 acceptance probes against independent existing HTTP writers.
use super::{common,fixture,support::*};
use serde_json::{json,Value};
use sqlx::PgPool;
use uuid::Uuid;
use svc_workflow::{application::{definition::{commands::RevokeVersion,DefinitionService},provisioning::replace_owner},domain::{ids::*,provisioning::ReplaceOwnerCommand},store::postgres::definition_repository::PgDefinitionRepository};

fn has(d:&Value,op:&str)->bool {d["authorized_actions"].as_array().expect("required action array").iter().any(|a|a["operation"]==op)}
async fn unchanged_business(p:&PgPool,before:&Value,instance:Uuid,actors:&[Uuid]) {
    let after=raw_probe_facts(p,instance,actors).await;
    for k in ["instance","events","visits","submissions"] {assert_eq!(before[k],after[k],"refusal changed {k}");}
}
async fn keyed(app:&axum::Router,path:&str,token:&str,key:&str,body:&Value)->(u16,Value) {
    use axum::{body::{Body,to_bytes},http::Request};use tower::ServiceExt;
    let req=Request::builder().method("POST").uri(path).header("authorization",format!("Bearer {token}")).header("idempotency-key",key).header("content-type","application/json").body(Body::from(body.to_string())).unwrap();
    let r=app.clone().oneshot(req).await.unwrap();let s=r.status().as_u16();let b=to_bytes(r.into_body(),usize::MAX).await.unwrap();(s,serde_json::from_slice(&b).unwrap())
}

#[tokio::test]
async fn model3_legitimate_edges_payload_cas_and_replay_keep_original_rules() {
    let p=fresh_pool("model3payload").await;let f=fixture::setup(&p,3).await;let id=fixture::create(&p,f).await;
    let jwks=common::MockJwksServer::start().await;let app=app(&p,&jwks,true);let tok=token(&jwks,f.agent,"workflow.execute workflow.read");
    let d=observed_detail(&p,&app,id,&tok,"model3-initial").await;
    assert_actions(&d,"AVAILABLE",edge_set(&[(f.advance,"ADVANCE")],&["request_assistance"]));assert_task_type(&d,"TASK");
    require_http(&app,"POST",&format!("{}/transitions",path(id)),&tok,transition_body(f.advance,1,json!({})),200,"model3-legitimate-advance").await;
    let d=observed_detail(&p,&app,id,&tok,"model3-work").await;
    assert_actions(&d,"AVAILABLE",edge_set(&[(f.finish,"ADVANCE"),(f.ret,"RETURN"),(f.terminate,"TERMINATE")],&["request_assistance"]));
    for (label,body,code) in [
        ("model3-return-invalid-payload",transition_body(f.ret,2,json!({})),"invalid_return_references"),
        ("model3-cas-stale",transition_body(f.finish,1,json!({})),"workflow_state_version_conflict")
    ] {
        let before=raw_probe_facts(&p,id,&[f.agent]).await;
        let(s,b)=send(&app,"POST",&format!("{}/transitions",path(id)),&tok,Some(body)).await;
        persist_probe(label,&json!({"status":s,"response":b,"before":before}));assert_eq!(b["error"]["code"],code,"independent payload/CAS check");assert!(s>=400&&s<500);
        unchanged_business(&p,&before,id,&[f.agent]).await;
    }
    let payload=json!({"rootCauseNodeVisitId":d["current_node_visit_id"],"reasonCode":"OTHER","reason":"valid RETURN with explicit references"});
    require_http(&app,"POST",&format!("{}/transitions",path(id)),&tok,transition_body(f.ret,2,payload),200,"model3-legitimate-return").await;
    require_http(&app,"POST",&format!("{}/transitions",path(id)),&tok,transition_body(f.advance,3,json!({})),200,"model3-reenter").await;
    let d=observed_detail(&p,&app,id,&tok,"model3-before-terminate").await;assert!(d["authorized_actions"].as_array().unwrap().contains(&transition_action(f.terminate,"TERMINATE")));
    let key=Uuid::new_v4().to_string();let body=transition_body(f.terminate,4,json!({}));let url=format!("{}/transitions",path(id));
    let(s,first)=keyed(&app,&url,&tok,&key,&body).await;assert_eq!(s,200,"{first}");let before=read_all_facts(&p).await;
    let(s,replay)=keyed(&app,&url,&tok,&key,&body).await;assert_eq!(s,200,"{replay}");let after=read_all_facts(&p).await;assert_eq!(before,after,"replay wrote facts");
    let mut changed=body;changed["submissionPayload"]=json!({"different":true});let(s,conflict)=keyed(&app,&url,&tok,&key,&changed).await;
    assert_eq!(s,409,"{conflict}");assert_eq!(conflict["error"]["code"],"idempotency_conflict");
    let after_conflict=read_all_facts(&p).await;
    // Existing Transition contract §16: different hash requires one attempt audit.
    // Same-body replay above still compares every table without an exception.
    let original=before["workflow_command_receipts"].as_array().unwrap().iter()
        .find(|r|r["principal_id"]==json!(f.agent)&&r["idempotency_key"]==key).unwrap();
    assert_eq!(original["command_type"],"EXECUTE_WORKFLOW_TRANSITION");
    let hash=svc_workflow::application::workflow_instance::idempotency::compute_transition_request_hash;
    let original_hash=hash("v1",&key,&PrincipalId::from_uuid(f.agent),&WorkflowInstanceId::from_uuid(id),4,&TransitionId::from_uuid(f.terminate),&Some(json!({}))).unwrap();
    let changed_hash=hash("v1",&key,&PrincipalId::from_uuid(f.agent),&WorkflowInstanceId::from_uuid(id),4,&TransitionId::from_uuid(f.terminate),&Some(changed["submissionPayload"].clone())).unwrap();
    assert_eq!(original["request_hash"],original_hash);assert_ne!(original_hash,changed_hash);
    let old_audits=before["workflow_command_attempt_audits"].as_array().unwrap();
    let new_audits=after_conflict["workflow_command_attempt_audits"].as_array().unwrap();
    assert_eq!(new_audits.len(),old_audits.len()+1,"exactly one accepted conflict audit");
    for audit in old_audits {assert!(new_audits.contains(audit),"prior audit changed");}
    let added:Vec<&Value>=new_audits.iter().filter(|a|!old_audits.contains(a)).collect();assert_eq!(added.len(),1);
    let audit=added[0];assert_eq!(audit["principal_id"],json!(f.agent));assert_eq!(audit["idempotency_key"],key);
    assert_eq!(audit["command_id"],original["command_id"]);assert_eq!(audit["attempt_type"],"IDEMPOTENCY_CONFLICT");
    assert_eq!(audit["failure_reason"],"request hash mismatch");assert_eq!(audit["request_hash"],changed_hash);
    assert_eq!(audit["details"],json!({"conflictType":"IDEMPOTENCY_KEY_MISMATCH","originalRequestHash":original_hash,"newRequestHash":changed_hash}));
    for (table,facts) in before.as_object().unwrap() {
        if table!="workflow_command_attempt_audits" {assert_eq!(facts,&after_conflict[table],"conflict changed non-audit table {table}");}
    }
    persist_probe("model3-replay-conflict",&json!({"first":first,"replay":replay,"conflict":conflict,"before":before,"after":after_conflict,"conflict_audit":audit,"original_hash":original_hash,"changed_hash":changed_hash}));
}

#[tokio::test]
async fn revoked_definition_disables_transitions_but_keeps_independent_assistance_cancel() {
    let p=fresh_pool("revoked").await;let f=fixture::setup(&p,1).await;let id=fixture::create(&p,f).await;
    DefinitionService::new(PgDefinitionRepository::new(p.clone())).revoke_version(RevokeVersion{actor_principal_id:f.owner,definition_version_id:f.version}).await.unwrap();
    let jwks=common::MockJwksServer::start().await;let app=app(&p,&jwks,true);let agent=token(&jwks,f.agent,"workflow.execute workflow.read");let owner=token(&jwks,f.owner,"workflow.execute workflow.read");
    let d=observed_detail(&p,&app,id,&agent,"revoked-agent").await;assert_actions(&d,"AVAILABLE",json!([action("request_assistance")]));
    let d=observed_detail(&p,&app,id,&owner,"revoked-owner").await;assert_actions(&d,"AVAILABLE",json!([action("cancel")]));
    let before=raw_probe_facts(&p,id,&[f.agent]).await;let(s,b)=send(&app,"POST",&format!("{}/transitions",path(id)),&agent,Some(transition_body(f.advance,1,json!({})))).await;
    assert_eq!(s,409,"{b}");assert_eq!(b["error"]["code"],"definition_version_revoked");unchanged_business(&p,&before,id,&[f.agent]).await;
    require_http(&app,"POST",&format!("{}/assistance-cases",path(id)),&agent,json!({"currentNodeVisitId":d["current_node_visit_id"],"expectedWorkflowStateVersion":1,"request":{"message":"revocation must not turn Assistance into transition permission"}}),201,"revoked-independent-assistance").await;
    require_http(&app,"POST",&format!("{}/cancel",path(id)),&owner,json!({"reason":"independent governance lifecycle"}),200,"revoked-independent-cancel").await;
}

#[tokio::test]
async fn disabled_target_and_disabled_caller_fail_closed() {
    let p=fresh_pool("disabled").await;let f=fixture::setup_variant(&p,1,true).await;let id=fixture::create(&p,f).await;
    sqlx::query("UPDATE principals SET enabled=FALSE WHERE principal_id=$1").bind(f.owner).execute(&p).await.unwrap();
    let jwks=common::MockJwksServer::start().await;let app=app(&p,&jwks,true);let agent=token(&jwks,f.agent,"workflow.execute workflow.read");
    let d=observed_detail(&p,&app,id,&agent,"disabled-target-detail").await;assert!(!has(&d,"transition"),"disabled target omitted");assert!(!has(&d,"request_assistance"),"no enabled Owner for Assistance");
    let before=raw_probe_facts(&p,id,&[f.agent]).await;let(s,b)=send(&app,"POST",&format!("{}/transitions",path(id)),&agent,Some(transition_body(f.advance,1,json!({})))).await;
    assert!(s>=400&&s<500,"{b}");unchanged_business(&p,&before,id,&[f.agent]).await;
    sqlx::query("UPDATE principals SET enabled=FALSE WHERE principal_id=$1").bind(f.agent).execute(&p).await.unwrap();
    let(s,b)=send(&app,"GET",&path(id),&agent,None).await;persist_probe("disabled-caller-refusal",&json!({"status":s,"response":b}));assert_eq!(s,403,"disabled Principal keeps original principal_disabled refusal");assert_eq!(b["error"]["code"],"principal_disabled");
}

#[tokio::test]
async fn replaced_owner_loses_cancel_while_new_owner_gets_current_authority() {
    let p=fresh_pool("replacement").await;let f=fixture::setup_variant(&p,1,true).await;let id=fixture::create(&p,f).await;
    let jwks=common::MockJwksServer::start().await;let app=app(&p,&jwks,true);let agent=token(&jwks,f.agent,"workflow.execute workflow.read");let old=token(&jwks,f.owner,"workflow.execute workflow.read");let new=token(&jwks,f.outsider,"workflow.execute workflow.read");
    require_http(&app,"POST",&format!("{}/transitions",path(id)),&agent,transition_body(f.advance,1,json!({})),200,"replacement-setup").await;
    replace_owner(&p,&ReplaceOwnerCommand{domain_id:DomainId::from_uuid(f.domain),new_owner_id:PrincipalId::from_uuid(f.outsider)},&Uuid::new_v4().to_string(),"isolated-AA-replacement",&PrincipalId::from_uuid(f.coordinator)).await.unwrap();
    let old_d=observed_detail(&p,&app,id,&old,"old-owner-still-raw-assignee").await;assert!(!has(&old_d,"cancel"));assert!(has(&old_d,"transition"));
    let new_d=observed_detail(&p,&app,id,&new,"new-current-owner").await;assert_actions(&new_d,"AVAILABLE",json!([action("cancel")]));
    let before=raw_probe_facts(&p,id,&[f.owner,f.outsider]).await;let(s,b)=send(&app,"POST",&format!("{}/cancel",path(id)),&old,Some(json!({"reason":"stale Owner is only current assignee"}))).await;assert_eq!(s,403,"{b}");unchanged_business(&p,&before,id,&[f.owner,f.outsider]).await;
    require_http(&app,"POST",&format!("{}/cancel",path(id)),&new,json!({"reason":"current Owner authority"}),200,"new-owner-cancel").await;
}
