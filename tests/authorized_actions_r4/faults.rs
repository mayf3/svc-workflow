//! Faults are deliberately introduced only in newly owned disposable databases.
use super::{common,fixture,support::*};use serde_json::json;

#[tokio::test]
async fn missing_activation_is_consistency_error_even_without_execute_scope() {
    let p=fresh_pool("activationfault").await;let f=fixture::setup(&p,3).await;let id=fixture::create(&p,f).await;
    let mut tx=p.begin().await.unwrap();sqlx::query("SET LOCAL session_replication_role='replica'").execute(&mut *tx).await.unwrap();
    sqlx::query("DELETE FROM workflow_activations WHERE workflow_instance_id=$1").bind(id).execute(&mut *tx).await.unwrap();tx.commit().await.unwrap();
    let jwks=common::MockJwksServer::start().await;let app=app(&p,&jwks,true);let before=read_all_facts(&p).await;
    for scope in ["workflow.execute workflow.read","workflow.read"] {let tok=token(&jwks,f.agent,scope);let(s,b)=send(&app,"GET",&path(id),&tok,None).await;persist_probe(if scope.contains("execute"){"fault-activation-execute"}else{"fault-activation-readonly"},&json!({"status":s,"response":b}));assert_eq!(s,500,"{b}");assert_eq!(b["error"]["code"],"internal_consistency_error");assert!(b.get("detail").is_none());}
    assert_eq!(before,read_all_facts(&p).await,"failed detail must not repair facts");
}

#[tokio::test]
async fn missing_required_storage_table_is_503_not_empty_success() {
    let p=fresh_pool("storagefault").await;let f=fixture::setup(&p,1).await;let id=fixture::create(&p,f).await;
    let jwks=common::MockJwksServer::start().await;let app=app(&p,&jwks,true);let tok=token(&jwks,f.agent,"workflow.execute workflow.read");
    sqlx::query("ALTER TABLE workflow_transition_definitions RENAME TO aa_fault_transition_definitions").execute(&p).await.unwrap();
    let before=read_all_facts(&p).await;let(s,b)=send(&app,"GET",&path(id),&tok,None).await;persist_probe("fault-storage",&json!({"status":s,"response":b}));
    assert_eq!(s,503,"{b}");assert_eq!(b["error"]["code"],"service_unavailable");assert!(b.get("detail").is_none());assert_eq!(before,read_all_facts(&p).await);
    sqlx::query("ALTER TABLE aa_fault_transition_definitions RENAME TO workflow_transition_definitions").execute(&p).await.unwrap();
}
