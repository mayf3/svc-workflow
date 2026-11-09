use super::{common,fixture,support::*};
use serde_json::{json,Value};
use uuid::Uuid;

#[tokio::test]
async fn legal_edges_sort_complete_and_each_effect_executes_independently() {
    let pool=fresh_pool("edges").await;let f=fixture::setup(&pool,1).await;
    let jwks=common::MockJwksServer::start().await;let app=app(&pool,&jwks,true);
    let actor=token(&jwks,f.agent,"workflow.execute workflow.read");let owner=token(&jwks,f.owner,"workflow.execute workflow.read");
    let mut detail_work=Value::Null;let mut detail_start=Value::Null;let mut terminal=Value::Null;
    for (label,edge) in [("advance",f.finish),("return",f.ret),("terminate",f.terminate)] {
        let instance=fixture::create(&pool,f).await;
        detail_start=observed_detail(&pool,&app,instance,&actor,&format!("{label}-draft-read")).await;
        require_http(&app,"POST",&format!("{}/transitions",path(instance)),&actor,transition_body(f.advance,1,json!({})),200,&format!("{label}-setup")).await;
        detail_work=observed_detail(&pool,&app,instance,&actor,&format!("{label}-normal-read")).await;
        let v=&detail_work["current_node_visit_id"];
        let payload=if edge==f.ret {json!({"rootCauseNodeVisitId":v,"reasonCode":"OTHER","reason":"independent literal RETURN payload"})}else{json!({})};
        let before=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;
        require_http(&app,"POST",&format!("{}/transitions",path(instance)),&actor,transition_body(edge,2,payload),200,&format!("{label}-command")).await;
        let after=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;
        persist_probe(&format!("{label}-durable-facts"),&json!({"before":before,"after":after}));
        assert_eq!(after["instance"]["workflow_state_version"],3);assert_eq!(after["events"].as_array().unwrap().len(),3);assert_eq!(after["submissions"].as_array().unwrap().len(),2);
        if edge!=f.ret {terminal=observed_detail(&pool,&app,instance,&owner,&format!("{label}-terminal-read")).await;}
    }
    assert_actions(&detail_start,"AVAILABLE",edge_set(&[(f.advance,"ADVANCE")],&["request_assistance"]));assert_task_type(&detail_start,"DRAFT");
    assert_actions(&detail_work,"AVAILABLE",edge_set(&[(f.finish,"ADVANCE"),(f.ret,"RETURN"),(f.terminate,"TERMINATE")],&["request_assistance"]));assert_task_type(&detail_work,"NORMAL");
    assert_actions(&terminal,"AVAILABLE",json!([action("archive")]));assert_task_type(&terminal,"TERMINAL");
}

#[tokio::test]
async fn assistance_open_escalate_resolve_matches_each_original_writer() {
    let pool=fresh_pool("assistance").await;let f=fixture::setup(&pool,1).await;let instance=fixture::create(&pool,f).await;
    let jwks=common::MockJwksServer::start().await;let app=app(&pool,&jwks,true);
    let actor=token(&jwks,f.agent,"workflow.execute workflow.read");let owner=token(&jwks,f.owner,"workflow.execute workflow.read");
    let unopened=observed_detail(&pool,&app,instance,&actor,"assistance-unopened").await;
    let request=json!({"currentNodeVisitId":unopened["current_node_visit_id"],"expectedWorkflowStateVersion":1,"request":{"message":"isolated ordinary Assistance"}});
    let r=require_http(&app,"POST",&format!("{}/assistance-cases",path(instance)),&actor,request,201,"request-assistance-command").await;let case=r["assistanceCaseId"].as_str().unwrap();
    let agent_open=observed_detail(&pool,&app,instance,&actor,"assistance-agent-open").await;
    let owner_open=observed_detail(&pool,&app,instance,&owner,"assistance-owner-open").await;
    let before=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;
    let denial=require_http(&app,"POST",&format!("{}/transitions",path(instance)),&actor,transition_body(f.advance,2,json!({})),409,"open-case-transition-refusal").await;
    let after=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;
    for k in ["instance","events","visits","submissions"] {assert_eq!(before[k],after[k],"SEMANTIC_STOP: open case refusal mutated {k}");}
    persist_probe("open-case-refusal-facts",&json!({"before":before,"after":after,"denial":denial}));
    require_http(&app,"POST",&format!("/internal/v1/assistance-cases/{case}/escalate-to-human"),&owner,json!({"expectedWorkflowStateVersion":2,"escalation":{"message":"ordinary escalation"}}),200,"escalate-command").await;
    let human=observed_detail(&pool,&app,instance,&owner,"assistance-human-required").await;
    require_http(&app,"POST",&format!("/internal/v1/assistance-cases/{case}/resolve"),&owner,json!({"expectedWorkflowStateVersion":3,"resolution":{"message":"ordinary owner resolution"}}),200,"resolve-command").await;
    let resolved=observed_detail(&pool,&app,instance,&actor,"assistance-resolved").await;
    assert_actions(&unopened,"AVAILABLE",edge_set(&[(f.advance,"ADVANCE")],&["request_assistance"]));
    assert_actions(&agent_open,"NONE_FOR_ACTOR",json!([]));
    assert_actions(&owner_open,"AVAILABLE",json!([case_action("escalate_assistance",case),case_action("resolve_assistance",case),action("cancel")]));
    assert_actions(&human,"AVAILABLE",json!([case_action("resolve_assistance",case),action("cancel")]));
    assert_actions(&resolved,"AVAILABLE",edge_set(&[(f.advance,"ADVANCE")],&["request_assistance"]));
}

#[tokio::test]
async fn lifecycle_owner_coordinator_archive_and_repeated_archive() {
    let pool=fresh_pool("lifecycle").await;let f=fixture::setup(&pool,3).await;let instance=fixture::create(&pool,f).await;
    let jwks=common::MockJwksServer::start().await;let app=app(&pool,&jwks,true);
    let actor=token(&jwks,f.agent,"workflow.execute workflow.read");let owner=token(&jwks,f.owner,"workflow.execute workflow.read");let coordinator=token(&jwks,f.coordinator,"workflow.execute workflow.read");
    let before=observed_detail(&pool,&app,instance,&owner,"lifecycle-before").await;
    let (hidden,body)=send(&app,"GET",&path(instance),&coordinator,None).await;persist_probe("coordinator-detail-hidden",&json!({"status":hidden,"body":body}));assert_eq!(hidden,404,"Coordinator role does not imply full detail");
    require_http(&app,"POST",&format!("{}/cancel",path(instance)),&coordinator,json!({"reason":"independent Coordinator cancellation"}),200,"coordinator-cancel").await;
    let cancelled=observed_detail(&pool,&app,instance,&owner,"lifecycle-cancelled").await;
    let actor_cancelled=observed_detail(&pool,&app,instance,&actor,"lifecycle-actor-cancelled").await;
    require_http(&app,"POST",&format!("{}/archive",path(instance)),&coordinator,json!({"reason":"independent Coordinator archive"}),200,"coordinator-archive").await;
    let archived=observed_detail(&pool,&app,instance,&owner,"lifecycle-archived").await;
    let snap=raw_probe_facts(&pool,instance,&[f.agent,f.owner,f.coordinator]).await;
    let(s,b)=send(&app,"POST",&format!("{}/archive",path(instance)),&coordinator,Some(json!({"reason":"repeat archive new key"}))).await;
    let after=raw_probe_facts(&pool,instance,&[f.agent,f.owner,f.coordinator]).await;persist_probe("repeat-archive",&json!({"status":s,"body":b,"before":snap,"after":after}));assert!(s>=400,"SEMANTIC_STOP: repeated archive was accepted");for k in ["instance","events","visits","submissions"] {assert_eq!(snap[k],after[k]);}
    assert_actions(&before,"AVAILABLE",json!([action("cancel")]));assert_task_type(&before,"TASK");
    assert_actions(&cancelled,"AVAILABLE",json!([action("archive")]));assert_actions(&actor_cancelled,"NONE_FOR_ACTOR",json!([]));assert_actions(&archived,"NONE_FOR_ACTOR",json!([]));
}

#[tokio::test]
async fn scope_gate_priority_full_restricted_hidden_and_minimal2() {
    let pool=fresh_pool("status").await;let f=fixture::setup_variant(&pool,1,true).await;let instance=fixture::create(&pool,f).await;
    let jwks=common::MockJwksServer::start().await;let app_on=app(&pool,&jwks,true);let app_off=app(&pool,&jwks,false);
    let actor=token(&jwks,f.agent,"workflow.execute workflow.read");let read=token(&jwks,f.agent,"workflow.read");let owner=token(&jwks,f.owner,"workflow.execute workflow.read");let outsider=token(&jwks,f.outsider,"workflow.execute workflow.read");
    let missing=observed_detail(&pool,&app_on,instance,&read,"scope-missing").await;let gate=observed_detail(&pool,&app_off,instance,&actor,"gate-closed").await;let both=observed_detail(&pool,&app_off,instance,&read,"scope-gate-priority").await;
    let(s,b)=send(&app_on,"GET",&path(instance),&outsider,None).await;persist_probe("hidden-detail",&json!({"status":s,"body":b}));assert_eq!(s,404);
    require_http(&app_on,"POST",&format!("{}/transitions",path(instance)),&actor,transition_body(f.advance,1,json!({})),200,"restricted-setup").await;
    let(s,restricted)=send(&app_on,"GET",&path(instance),&actor,None).await;persist_probe("restricted-detail",&json!({"status":s,"body":restricted}));assert_eq!(s,200);assert_eq!(restricted["visibility"],"historical_participant");for field in ["authorized_actions_scope","authorized_actions_status","authorized_actions","current_task_type"] {assert!(restricted["detail"].get(field).is_none());}
    let owner_full=observed_detail(&pool,&app_on,instance,&owner,"owner-full-normal").await;
    let minimal=fixture::setup(&pool,2).await;let mid=fixture::create(&pool,minimal).await;let mt=token(&jwks,minimal.agent,"workflow.execute workflow.read");let md=observed_detail(&pool,&app_on,mid,&mt,"minimal2-unsupported").await;
    assert_actions(&missing,"MISSING_EXECUTE_SCOPE",json!([]));assert_actions(&gate,"WRITE_GATE_CLOSED",json!([]));assert_actions(&both,"MISSING_EXECUTE_SCOPE",json!([]));assert_actions(&md,"UNSUPPORTED_MODEL",json!([]));
    assert_actions(&owner_full,"AVAILABLE",edge_set(&[(f.finish,"ADVANCE"),(f.ret,"RETURN"),(f.terminate,"TERMINATE")],&["request_assistance","cancel"]));
}

#[tokio::test]
async fn return_zero_penultimate_and_cap_use_events_and_read_never_opens_case() {
    let pool=fresh_pool("returncap").await;let f=fixture::setup(&pool,1).await;let instance=fixture::create(&pool,f).await;
    let jwks=common::MockJwksServer::start().await;let app=app(&pool,&jwks,true);let actor=token(&jwks,f.agent,"workflow.execute workflow.read");let owner=token(&jwks,f.owner,"workflow.execute workflow.read");
    let mut version=1;let mut samples=vec![];
    for count in 0..3 {
        require_http(&app,"POST",&format!("{}/transitions",path(instance)),&actor,transition_body(f.advance,version,json!({})),200,&format!("cap-{count}-advance")).await;version+=1;
        let d=observed_detail(&pool,&app,instance,&actor,&format!("cap-{count}-read")).await;
        let events:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1 AND transition_effect='RETURN' AND event_data->>'transition_definition_id'=$2").bind(instance).bind(f.ret.to_string()).fetch_one(&pool).await.unwrap();assert_eq!(events,count);
        let cases:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_assistance_cases WHERE workflow_instance_id=$1").bind(instance).fetch_one(&pool).await.unwrap();assert_eq!(cases,0,"read at count=max-1 must not open target case");
        let payload=json!({"rootCauseNodeVisitId":d["current_node_visit_id"],"reasonCode":"OTHER","reason":"legitimate repeat RETURN"});samples.push(d);
        require_http(&app,"POST",&format!("{}/transitions",path(instance)),&actor,transition_body(f.ret,version,payload),200,&format!("cap-{count}-return")).await;version+=if count==2 {2}else{1};
    }
    let (case,):(Uuid,)=sqlx::query_as("SELECT assistance_case_id FROM workflow_assistance_cases WHERE workflow_instance_id=$1 AND status='OWNER_PENDING'").bind(instance).fetch_one(&pool).await.unwrap();
    require_http(&app,"POST",&format!("/internal/v1/assistance-cases/{case}/resolve"),&owner,json!({"expectedWorkflowStateVersion":version,"resolution":{"message":"Owner resolved exact max-return escalation"}}),200,"cap-target-resolve").await;version+=1;
    require_http(&app,"POST",&format!("{}/transitions",path(instance)),&actor,transition_body(f.advance,version,json!({})),200,"cap-final-advance").await;version+=1;
    let cap=observed_detail(&pool,&app,instance,&actor,"cap-exhausted-read").await;
    let before=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;
    let(s,b)=send(&app,"POST",&format!("{}/transitions",path(instance)),&actor,Some(transition_body(f.ret,version,json!({"rootCauseNodeVisitId":cap["current_node_visit_id"],"reasonCode":"OTHER","reason":"cap refusal"})))).await;
    let after=raw_probe_facts(&pool,instance,&[f.agent,f.owner]).await;persist_probe("cap-exhausted-refusal",&json!({"status":s,"body":b,"before":before,"after":after}));assert_eq!(s,409,"SEMANTIC_STOP: RETURN at cap not refused: {b}");for k in ["instance","events","submissions","visits"] {assert_eq!(before[k],after[k]);}
    for d in samples {assert_actions(&d,"AVAILABLE",edge_set(&[(f.finish,"ADVANCE"),(f.ret,"RETURN"),(f.terminate,"TERMINATE")],&["request_assistance"]));}
    assert_actions(&cap,"AVAILABLE",edge_set(&[(f.finish,"ADVANCE"),(f.terminate,"TERMINATE")],&["request_assistance"]));
}
