//! ACC-AA-004: existing retired managed cleanup actor, after original visibility.
use super::{common, support as aa};
use axum::{body::{to_bytes,Body},http::Request};
use serde_json::{json,Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;
#[path="managed_fixture.rs"] mod support;
use support::*;
#[path="managed_http.rs"] mod http_support;

#[tokio::test]
async fn retired_managed_cleanup_actor_is_projected_only_with_existing_full_visibility() {
    let p=aa::fresh_pool("cleanupactor").await;
    let (owner,_,pm,managed,_,_,source)=operation_fixture(&p).await;
    let jwks=common::MockJwksServer::start().await;
    let app=http_support::app(&p,&jwks,pm).await;
    let owner_token=common::v1_token(owner,"workflow.execute workflow.read","gcr-http",300,&jwks.key_pair);
    let pm_token=common::v1_token(pm,"workflow.execute workflow.read","gcr-http",300,&jwks.key_pair);
    // Cleanup eligibility grants no full read. The actual creator retains
    // its existing historical-participant restricted read (Query contract §3).
    let creator:Uuid=sqlx::query_scalar("SELECT created_by_principal_id FROM workflow_instances WHERE workflow_instance_id=$1")
        .bind(managed).fetch_one(&p).await.unwrap();assert_eq!(creator,pm);
    let (restricted_status,restricted)=aa::send(&app,"GET",&aa::path(managed),&pm_token,None).await;
    assert_eq!(restricted_status,200,"{restricted}");assert_eq!(restricted["visibility"],"historical_participant");
    let keys=|value:&Value|{let mut names:Vec<String>=value.as_object().unwrap().keys().cloned().collect();names.sort();names};
    assert_eq!(keys(&restricted),vec!["detail".to_owned(),"visibility".to_owned()]);
    assert_eq!(keys(&restricted["detail"]),vec!["instance".to_owned()]);
    assert_eq!(keys(&restricted["detail"]["instance"]),
        ["created_at","current_node","definition_version_id","definition_version_status","domain_enabled","domain_id","is_terminal","workflow_instance_id","workflow_state_version"].map(str::to_owned).to_vec());
    assert_eq!(keys(&restricted["detail"]["instance"]["current_node"]),
        ["display_name","node_id","node_key","node_type"].map(str::to_owned).to_vec());
    for field in ["authorized_actions_scope","authorized_actions_status","authorized_actions","current_task_type"] {
        assert!(restricted["detail"].get(field).is_none(),"restricted creator read leaked {field}");
    }
    aa::persist_probe("cleanup-creator-restricted-read",&json!({"managed":managed,"creator":creator,"policy_actor":pm,"status":restricted_status,"response":restricted}));
    // Existing GLOBAL_WORKFLOW_AUDITOR full visibility is independent of the
    // existing cleanup authorization and adds no governance write capability.
    sqlx::query("INSERT INTO global_role_bindings(binding_id,principal_id,role_key,enabled) VALUES($1,$2,'GLOBAL_WORKFLOW_AUDITOR',TRUE)")
        .bind(Uuid::new_v4()).bind(pm).execute(&p).await.unwrap();
    let active=aa::observed_detail(&p,&app,managed,&pm_token,"cleanup-active-policy-actor-read").await;
    aa::assert_actions(&active,"NONE_FOR_ACTOR",json!([]));
    let denied_before=aa::raw_probe_facts(&p,managed,&[pm]).await;
    let (denied_status,denied)=aa::send(&app,"POST",&format!("{}/cancel",aa::path(managed)),&pm_token,
        Some(json!({"reason":"active policy actor has no cleanup write grant"}))).await;
    assert_eq!(denied_status,403,"{denied}");assert_eq!(denied["error"]["code"],"not_domain_owner");
    let denied_after=aa::raw_probe_facts(&p,managed,&[pm]).await;
    for key in ["instance","events","visits","submissions"] {assert_eq!(denied_before[key],denied_after[key]);}
    aa::persist_probe("cleanup-active-policy-actor-refusal",&json!({"status":denied_status,"response":denied,"before":denied_before,"after":denied_after}));
    aa::require_http(&app,"POST",&format!("{}/cancel",aa::path(source)),&owner_token,
        json!({"reason":"retire actual managed obligation before separate cleanup"}),200,"cleanup-parent-cancel").await;
    let state:String=sqlx::query_scalar("SELECT state FROM workflow_owner_obligation_intents WHERE managed_instance_id=$1 AND kind='INTAKE'")
        .bind(managed).fetch_one(&p).await.unwrap();assert_eq!(state,"RETIRED");
    let d=aa::observed_detail(&p,&app,managed,&pm_token,"cleanup-retired-policy-actor-detail").await;
    aa::assert_actions(&d,"AVAILABLE",json!([aa::action("cancel")]));
    let before=aa::raw_probe_facts(&p,managed,&[pm]).await;
    let response=aa::require_http(&app,"POST",&format!("{}/cancel",aa::path(managed)),&pm_token,
        json!({"reason":"actual authorized retired-obligation cleanup"}),200,"cleanup-actual-policy-actor-cancel").await;
    let after=aa::raw_probe_facts(&p,managed,&[pm]).await;
    assert_eq!(after["instance"]["cancelled"],true);
    assert_eq!(after["instance"]["workflow_state_version"].as_i64().unwrap(),before["instance"]["workflow_state_version"].as_i64().unwrap()+1);
    assert_eq!(after["events"].as_array().unwrap().len(),before["events"].as_array().unwrap().len()+1);
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_command_receipts r JOIN workflow_events e USING(command_id) WHERE r.principal_id=$1 AND r.command_type='CANCEL_WORKFLOW_INSTANCE' AND r.receipt_status='COMPLETED' AND r.response_status=200 AND e.workflow_instance_id=$2")
        .bind(pm).bind(managed).fetch_one(&p).await.unwrap();assert_eq!(count,1);
    let closed:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_activation_closures c JOIN workflow_activations a USING(activation_id) WHERE a.workflow_instance_id=$1 AND c.closure_reason='CANCELLED'")
        .bind(managed).fetch_one(&p).await.unwrap();assert_eq!(closed,1);
    aa::persist_probe("cleanupactor-receipt-result",&json!({"state":state,"restricted_status":restricted_status,"before":before,"after":after,"response":response,"successful_cleanup_commands":count,"closure_count":closed}));
}
