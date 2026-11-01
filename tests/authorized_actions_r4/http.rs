use super::{common,fixture,support::*};
use serde_json::json;

// Catches omitted action fields and the rejected filter(legacy true) implementation.
// This test proves legitimate HTTP allow/refuse behavior before its expected RED.
#[tokio::test]
async fn cancelled_true_edge_has_no_action_after_independent_allow_and_refuse() {
    let pool=common::create_pool().await;
    // Candidate may bind the exact RED database/IDs/receipts without replaying any command.
    if let Ok(source)=std::env::var("AA_REUSE_CANCELLED_RED") {
        let r:serde_json::Value=serde_json::from_slice(&std::fs::read(source).unwrap()).unwrap();
        let instance=serde_json::from_value(r["instance"].clone()).unwrap();
        let actor_id=serde_json::from_value(r["actor"].clone()).unwrap();
        let owner_id=serde_json::from_value(r["owner"].clone()).unwrap();
        let jwks=common::MockJwksServer::start().await;let app=app(&pool,&jwks,true);
        let token=token(&jwks,actor_id,"workflow.execute workflow.read");
        let before=raw_probe_facts(&pool,instance,&[actor_id,owner_id]).await;
        assert_eq!(before,r["after_refusal"],"exact RED fixture must remain unchanged");
        let d=observed_detail(&pool,&app,instance,&token,"same-red-fixture-candidate-detail").await;
        let mut old=d.clone();
        for key in ["authorized_actions_scope","authorized_actions_status","authorized_actions","current_task_type"] {old.as_object_mut().unwrap().remove(key);}
        assert_eq!(old,r["detail"],"candidate changed legacy outgoing diagnostic/shared detail");
        assert_actions(&d,"NONE_FOR_ACTOR",json!([]));assert_task_type(&d,"NORMAL");return;
    }
    let f=fixture::setup(&pool,1).await;
    let instance=fixture::create(&pool,f).await;
    let jwks=common::MockJwksServer::start().await;
    let app=app(&pool,&jwks,true);
    let actor=token(&jwks,f.agent,"workflow.execute workflow.read");
    let owner=token(&jwks,f.owner,"workflow.execute workflow.read");
    let initial=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;
    let (allowed_status,allowed_body)=send(&app,"POST",&format!("{}/transitions",path(instance)),&actor,Some(transition_body(f.advance,1,json!({})))).await;
    let advanced=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;
    persist_probe("red-allow",&json!({"instance":instance,"before":initial,"status":allowed_status,"response":allowed_body,"after":advanced}));
    assert_eq!(allowed_status,200,"SEMANTIC_STOP: legitimate ADVANCE failed {allowed_body}");
    assert_eq!(advanced["instance"]["workflow_state_version"],2);
    assert_eq!(advanced["events"].as_array().unwrap().len(),2);
    assert_eq!(advanced["submissions"].as_array().unwrap().len(),1);
    let (cancel_status,cancel_body)=send(&app,"POST",&format!("{}/cancel",path(instance)),&owner,Some(json!({"reason":"isolated #1344 cancelled+true control"}))).await;
    let cancelled=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;
    persist_probe("red-cancel",&json!({"instance":instance,"status":cancel_status,"response":cancel_body,"before":advanced,"after":cancelled}));
    assert_eq!(cancel_status,200,"SEMANTIC_STOP: legitimate cancel failed {cancel_body}");
    let d=detail(&app,instance,&actor).await;
    let after_read=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;
    assert_eq!(cancelled,after_read,"successful detail changed business or receipt facts");
    let old=d["outgoing_transitions"].as_array().unwrap().iter().find(|e|e["transition_id"]==json!(f.finish)).unwrap();
    assert_eq!(old["executable_for_actor"],true,"old diagnostic meaning must be retained");
    let (refused_status,refused_body)=send(&app,"POST",&format!("{}/transitions",path(instance)),&actor,Some(transition_body(f.finish,3,json!({})))).await;
    let refused=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;
    persist_probe("red-result",&json!({"instance":instance,"actor":f.agent,"owner":f.owner,"transition":f.finish,"detail":d,"before_refusal":cancelled,"status":refused_status,"response":refused_body,"after_refusal":refused}));
    assert_eq!(refused_status,409,"SEMANTIC_STOP: cancelled transition was accepted {refused_body}");
    assert_eq!(refused_body["error"]["code"],"source_node_terminal");
    for key in ["instance","events","submissions","visits"] {assert_eq!(cancelled[key],refused[key],"SEMANTIC_STOP: refused command changed {key}");}
    eprintln!("LEGITIMATE_HTTP_CONTROL instance={instance} advance={allowed_status} cancel={cancel_status} cancelled_transition={refused_status} old_true={} before_version={} after_version={} before_events={} after_events={}",old["executable_for_actor"],cancelled["instance"]["workflow_state_version"],refused["instance"]["workflow_state_version"],cancelled["events"].as_array().unwrap().len(),refused["events"].as_array().unwrap().len());
    assert_actions(&d,"NONE_FOR_ACTOR",json!([]));
}
