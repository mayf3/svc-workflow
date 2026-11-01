use super::{common,fixture,support::*};
use serde_json::{json,Value};
use sqlx::PgPool;
use uuid::Uuid;

/// Capture on baseline; set AA_SHARED_BASELINE to that exact record on candidate.
/// The second process reads the same durable facts, with no fixture replay.
#[tokio::test]
async fn exact_fixture_shared_reads_and_cti_bytes_remain_unchanged() {
    let baseline=std::env::var("AA_SHARED_BASELINE").ok().map(|p|serde_json::from_slice::<Value>(&std::fs::read(p).unwrap()).unwrap());
    let (pool,instance,agent,owner)=if let Some(b)=&baseline {
        (PgPool::connect(b["dsn"].as_str().unwrap()).await.unwrap(),serde_json::from_value(b["instance"].clone()).unwrap(),serde_json::from_value(b["agent"].clone()).unwrap(),serde_json::from_value(b["owner"].clone()).unwrap())
    }else{
        let p=fresh_pool("shared").await;let f=fixture::setup(&p,3).await;let id=fixture::create(&p,f).await;(p,id,f.agent,f.owner)
    };
    let jwks=common::MockJwksServer::start().await;let app=app(&pool,&jwks,true);let token=token(&jwks,agent,"workflow.execute workflow.read");let owner_token=super::support::token(&jwks,owner,"workflow.execute workflow.read");
    let before=read_all_facts(&pool).await;
    let (s,detail)=send(&app,"GET",&path(instance),&token,None).await;assert_eq!(s,200);
    let (s,input)=send(&app,"GET",&format!("{}/current-task-input",path(instance)),&token,None).await;assert_eq!(s,200,"{input}");
    let binding=&input["binding"];let descriptor=&input["detail_body"];
    let url=format!("{}/current-task-input?view=body&expectedNodeVisitId={}&expectedWorkflowStateVersion={}&expectedContextRevisionId={}&bodyKind=CURRENT_DETAIL&bodyId={}&expectedBodySha256={}",path(instance),binding["node_visit_id"].as_str().unwrap(),binding["workflow_state_version"],binding["context_revision_id"].as_str().unwrap(),descriptor["id"].as_str().unwrap(),descriptor["sha256"].as_str().unwrap());
    let (s,body)=send(&app,"GET",&url,&token,None).await;assert_eq!(s,200,"{body}");
    let text=descriptor["text"].as_str().unwrap();assert!(descriptor["next_offset"].is_null(),"small literal fixture must fit exact CURRENT_DETAIL bytes");
    use sha2::{Digest,Sha256};assert_eq!(hex::encode(Sha256::digest(text.as_bytes())),descriptor["sha256"].as_str().unwrap());
    let parsed:Value=serde_json::from_str(text).unwrap();assert_eq!(parsed.as_object().unwrap().len(),6,"shared Full remains six fields");
    for field in ["authorized_actions_scope","authorized_actions_status","authorized_actions","current_task_type"] {assert!(parsed.get(field).is_none());}
    let mut shared=json!({"cti_input":input,"cti_body":body,"detail_without_new_fields":detail});
    for field in ["authorized_actions_scope","authorized_actions_status","authorized_actions","current_task_type"] {shared["detail_without_new_fields"]["detail"].as_object_mut().unwrap().remove(field);}
    for (label,url,t) in [("timeline",format!("{}/timeline",path(instance)),&owner_token),("submissions",format!("{}/submissions",path(instance)),&owner_token),("assigned",String::from("/internal/v1/worklists/assigned-to-me"),&token),("drafts",String::from("/internal/v1/worklists/creator-owned-drafts"),&token)] {
        let(s,b)=send(&app,"GET",&url,t,None).await;assert_eq!(s,200,"shared {label}: {b}");shared[label]=b;
    }
    let after=read_all_facts(&pool).await;assert_eq!(before,after,"all shared reads write zero facts");
    let db:String=sqlx::query_scalar("SELECT current_database()").fetch_one(&pool).await.unwrap();
    let record=json!({"dsn":format!("postgres://swaar4@127.0.0.1:55483/{db}"),"instance":instance,"agent":agent,"owner":owner,"shared":shared,"facts":before});
    persist_probe("shared-read-record",&record);
    if let Some(b)=baseline {assert_eq!(record["shared"],b["shared"],"candidate altered exact baseline shared responses/body bytes/digest");assert_eq!(record["facts"],b["facts"],"candidate changed durable fixture");}
}

#[tokio::test]
async fn successor_transition_permission_does_not_grant_raw_assignee_assistance() {
    let pool=fresh_pool("successor").await;let f=fixture::setup(&pool,1).await;let instance=fixture::create(&pool,f).await;
    sqlx::query("INSERT INTO workflow_identity_successor_lines(id,source_principal_id,successor_principal_id,legacy_agent_id,canonical_agent_id,classification,evidence,repair_reason) VALUES($1,$2,$3,'legacy-aa-r4','canonical-aa-r4','STALE_PRINCIPAL_WITH_UNIQUE_REPAIR','\"isolated evidence\"','\"isolated evidence\"')").bind(Uuid::new_v4()).bind(f.agent).bind(f.outsider).execute(&pool).await.unwrap();
    let jwks=common::MockJwksServer::start().await;let app=app(&pool,&jwks,true);let successor=token(&jwks,f.outsider,"workflow.execute workflow.read");
    let d=observed_detail(&pool,&app,instance,&successor,"successor-full").await;
    let before=raw_probe_facts(&pool,instance,&[f.agent,f.outsider]).await;
    let refused=require_http(&app,"POST",&format!("{}/assistance-cases",path(instance)),&successor,json!({"currentNodeVisitId":d["current_node_visit_id"],"expectedWorkflowStateVersion":1,"request":{"message":"canonical successor is not raw assignee"}}),403,"successor-assistance-refusal").await;
    let after=raw_probe_facts(&pool,instance,&[f.agent,f.outsider]).await;for k in ["instance","events","visits","submissions"] {assert_eq!(before[k],after[k]);}
    persist_probe("successor-assistance-facts",&json!({"before":before,"after":after,"refusal":refused}));
    require_http(&app,"POST",&format!("{}/transitions",path(instance)),&successor,transition_body(f.advance,1,json!({})),200,"successor-legitimate-transition").await;
    assert_actions(&d,"AVAILABLE",edge_set(&[(f.advance,"ADVANCE")],&[]));
}
