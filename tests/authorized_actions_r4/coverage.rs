//! Missing explicit ACC-AA-002/006 cases; test fixtures own their databases.
use super::{common, fixture, support::*};
use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

async fn unchanged_runtime(p: &PgPool, before: &Value, instance: Uuid, actors: &[Uuid]) {
    let after = raw_probe_facts(p, instance, actors).await;
    for key in ["instance", "events", "visits", "submissions"] {
        assert_eq!(before[key], after[key], "refusal changed {key}");
    }
}

fn has_transition(d: &Value, id: Uuid) -> bool {
    d["authorized_actions"].as_array().unwrap().iter()
        .any(|a| a["operation"] == "transition" && a["transition_id"] == json!(id))
}

#[tokio::test]
async fn draft_definition_defensive_detail_omits_transition_and_writer_still_refuses() {
    let p = fresh_pool("draftdef").await;
    let f = fixture::setup(&p, 1).await;
    let instance = fixture::create(&p, f).await;
    let jwks = common::MockJwksServer::start().await;
    let app = app(&p, &jwks, true);
    let agent = token(&jwks, f.agent, "workflow.execute workflow.read");
    let owner = token(&jwks, f.owner, "workflow.execute workflow.read");
    let published = observed_detail(&p, &app, instance, &agent, "draft-published-control").await;
    assert_actions(&published, "AVAILABLE", edge_set(&[(f.advance, "ADVANCE")], &["request_assistance"]));
    // Deliberate defensive corruption in this disposable DB only. Normal
    // accepted lifecycle correctly forbids PUBLISHED -> DRAFT.
    let mut tx = p.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role='replica'").execute(&mut *tx).await.unwrap();
    sqlx::query("UPDATE workflow_definition_versions SET version_status='DRAFT' WHERE definition_version_id=$1")
        .bind(f.version).execute(&mut *tx).await.unwrap();
    tx.commit().await.unwrap();
    let d = observed_detail(&p, &app, instance, &agent, "draft-agent-detail").await;
    assert_eq!(d["instance"]["definition_version_status"], "DRAFT");
    assert_actions(&d, "AVAILABLE", json!([action("request_assistance")]));
    let d_owner = observed_detail(&p, &app, instance, &owner, "draft-owner-detail").await;
    assert_actions(&d_owner, "AVAILABLE", json!([action("cancel")]));
    let before = raw_probe_facts(&p, instance, &[f.agent]).await;
    let (status, response) = send(&app, "POST", &format!("{}/transitions", path(instance)), &agent,
        Some(transition_body(f.advance, 1, json!({})))).await;
    persist_probe("draft-original-writer-refusal", &json!({"status":status,"response":response,"before":before}));
    assert_eq!(status, 500, "DRAFT retains the original internal-state refusal: {response}");
    assert_eq!(response["error"]["code"], "internal_consistency_error");
    unchanged_runtime(&p, &before, instance, &[f.agent]).await;
    let receipt: (i64, Option<i32>, Option<String>) = sqlx::query_as(
        "SELECT count(*),max(response_status),max(response_body->>'error') FROM workflow_command_receipts WHERE principal_id=$1 AND command_type='EXECUTE_WORKFLOW_TRANSITION'")
        .bind(f.agent).fetch_one(&p).await.unwrap();
    assert_eq!(receipt, (1, Some(500), Some("definition_version_draft".into())));
    persist_probe("draft-failure-receipt", &json!({"receipt":receipt}));
}

/// A fixture-only view pauses ONLY RR readers, so the unmodified READ
/// COMMITTED transition writer can actually commit while detail is paused.
/// Existing FKs remain bound to the renamed physical table. No production
/// schema/source is changed; normal before/after fact probes read base tables.
async fn install_rr_barrier(p: &PgPool, key: i64) {
    sqlx::query("ALTER TABLE workflow_transition_definitions RENAME TO aa_snapshot_transition_facts")
        .execute(p).await.unwrap();
    let sql = format!("CREATE VIEW workflow_transition_definitions AS
        WITH barrier AS MATERIALIZED (
          SELECT CASE WHEN current_setting('transaction_isolation')='repeatable read'
                 THEN pg_advisory_xact_lock_shared({key}::bigint) ELSE NULL END AS waited)
        SELECT t.* FROM aa_snapshot_transition_facts t CROSS JOIN barrier");
    sqlx::query(&sql).execute(p).await.unwrap();
}

#[tokio::test]
async fn repeatable_read_case_and_return_changes_use_one_snapshot_then_execution_rechecks() {
    for mode in ["case", "return"] {
        let p = fresh_pool(&format!("snapshot{mode}")).await;
        let f = fixture::setup(&p, 1).await;
        let instance = fixture::create(&p, f).await;
        let jwks = common::MockJwksServer::start().await;
        let app = app(&p, &jwks, true);
        let agent = token(&jwks, f.agent, "workflow.execute workflow.read");
        let owner = token(&jwks, f.owner, "workflow.execute workflow.read");
        require_http(&app, "POST", &format!("{}/transitions", path(instance)), &agent,
            transition_body(f.advance, 1, json!({})), 200, &format!("snapshot-{mode}-enter")).await;
        // At max-1 the real RETURN is still selectable. The concurrent third
        // RETURN writes the actual Event/count and opens target Assistance.
        if mode == "return" {
            for count in 0..2 {
                let d = observed_detail(&p, &app, instance, &agent, &format!("snapshot-return-before-{count}")).await;
                let version = d["instance"]["workflow_state_version"].as_i64().unwrap() as i32;
                require_http(&app, "POST", &format!("{}/transitions", path(instance)), &agent,
                    transition_body(f.ret, version, json!({"rootCauseNodeVisitId":d["current_node_visit_id"],"reasonCode":"OTHER","reason":"real prior RETURN"})),
                    200, &format!("snapshot-return-prior-{count}")).await;
                require_http(&app, "POST", &format!("{}/transitions", path(instance)), &agent,
                    transition_body(f.advance, version + 1, json!({})), 200,
                    &format!("snapshot-return-reenter-{count}")).await;
            }
        }
        let before = observed_detail(&p, &app, instance, &agent, &format!("snapshot-{mode}-before")).await;
        assert_actions(&before, "AVAILABLE", edge_set(&[(f.finish,"ADVANCE"),(f.ret,"RETURN"),(f.terminate,"TERMINATE")], &["request_assistance"]));
        let version = before["instance"]["workflow_state_version"].as_i64().unwrap() as i32;
        let key = (i64::from_be_bytes(Uuid::new_v4().as_bytes()[..8].try_into().unwrap()) & i64::MAX).max(1);
        install_rr_barrier(&p, key).await;
        let mut blocker = p.begin().await.unwrap();
        sqlx::query("SELECT pg_advisory_xact_lock($1)").bind(key).execute(&mut *blocker).await.unwrap();
        let read_app = app.clone();
        let read_token = agent.clone();
        let read = tokio::spawn(async move { send(&read_app,"GET",&path(instance),&read_token,None).await });
        let mut paused = false;
        for _ in 0..500 {
            let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE datname=current_database() AND wait_event_type='Lock' AND lower(wait_event)='advisory' AND query LIKE '%workflow_transition_definitions%')")
                .fetch_one(&p).await.unwrap();
            if waiting { paused = true; break; }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        if !paused { blocker.rollback().await.unwrap(); read.abort(); panic!("RR detail never reached observed advisory barrier"); }
        let mutation = if mode == "case" {
            json!({"currentNodeVisitId":before["current_node_visit_id"],"expectedWorkflowStateVersion":version,"request":{"message":"actual concurrent Assistance"}})
        } else {
            transition_body(f.ret, version, json!({"rootCauseNodeVisitId":before["current_node_visit_id"],"reasonCode":"OTHER","reason":"actual concurrent max-th RETURN"}))
        };
        let suffix = if mode == "case" { "assistance-cases" } else { "transitions" };
        let mutation_result = tokio::time::timeout(std::time::Duration::from_secs(10),
            send(&app,"POST",&format!("{}/{suffix}",path(instance)),&agent,Some(mutation.clone()))).await;
        blocker.rollback().await.unwrap();
        let (mutation_status, mutation_response) = mutation_result.expect("real writer must bypass only-RR barrier");
        assert_eq!(mutation_status, if mode=="case" {201}else{200}, "{mutation_response}");
        let (status, old) = read.await.unwrap();
        assert_eq!(status, 200, "{old}");
        assert_eq!(old["detail"], before, "one RR snapshot must retain old state, case and RETURN count");
        // The max-th RETURN appends both the transition and target
        // Assistance Event; an ordinary Assistance request appends one.
        let current_version = version + if mode == "return" { 2 } else { 1 };
        let fresh = observed_detail(&p,&app,instance,&agent,&format!("snapshot-{mode}-fresh")).await;
        assert_eq!(fresh["instance"]["workflow_state_version"],current_version);
        assert_actions(&fresh,"NONE_FOR_ACTOR",json!([]));
        let facts = raw_probe_facts(&p,instance,&[f.agent,f.owner]).await;
        let (stale_status, stale) = send(&app,"POST",&format!("{}/transitions",path(instance)),&agent,
            Some(transition_body(f.finish,version,json!({})))).await;
        assert_eq!(stale_status,409,"{stale}");
        assert_eq!(stale["error"]["code"],"workflow_state_version_conflict");
        unchanged_runtime(&p,&facts,instance,&[f.agent,f.owner]).await;
        let (current_status,current) = send(&app,"POST",&format!("{}/transitions",path(instance)),&agent,
            Some(transition_body(if mode=="case" {f.finish}else{f.advance},current_version,json!({})))).await;
        assert_eq!(current_status,409,"{current}");
        assert_eq!(current["error"]["code"],"assistance_open");
        unchanged_runtime(&p,&facts,instance,&[f.agent,f.owner]).await;
        if mode == "return" {
            let case = mutation_response["returnPolicyEscalation"]["assistanceCaseId"].as_str().expect("max-th RETURN must open a real case");
            require_http(&app,"POST",&format!("/internal/v1/assistance-cases/{case}/resolve"),&owner,
                json!({"expectedWorkflowStateVersion":current_version,"resolution":{"message":"real Owner resolution of RETURN cap case"}}),200,"snapshot-return-resolve").await;
            require_http(&app,"POST",&format!("{}/transitions",path(instance)),&agent,
                transition_body(f.advance,current_version+1,json!({})),200,"snapshot-return-final-reentry").await;
            let capped=observed_detail(&p,&app,instance,&agent,"snapshot-return-cap-after-resolution").await;
            assert!(!has_transition(&capped,f.ret));
            assert!(has_transition(&capped,f.finish));
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_events WHERE workflow_instance_id=$1 AND transition_effect='RETURN' AND event_data->>'transition_definition_id'=$2")
                .bind(instance).bind(f.ret.to_string()).fetch_one(&p).await.unwrap();
            assert_eq!(count,3);
            let facts=raw_probe_facts(&p,instance,&[f.agent]).await;
            let (s,b)=send(&app,"POST",&format!("{}/transitions",path(instance)),&agent,
                Some(transition_body(f.ret,current_version+2,json!({"rootCauseNodeVisitId":capped["current_node_visit_id"],"reasonCode":"OTHER","reason":"RETURN at stored cap"})))).await;
            assert_eq!(s,409,"{b}");assert_eq!(b["error"]["code"],"return_policy_exhausted");
            unchanged_runtime(&p,&facts,instance,&[f.agent]).await;
        }
        persist_probe(&format!("snapshot-{mode}-result"),&json!({"barrier_key":key,"observed_rr_wait":paused,"old_snapshot":old,"mutation_request":mutation,"mutation_status":mutation_status,"mutation_response":mutation_response,"fresh":fresh,"stale_refusal":stale,"current_refusal":current}));
        sqlx::query("DROP VIEW workflow_transition_definitions").execute(&p).await.unwrap();
        sqlx::query("ALTER TABLE aa_snapshot_transition_facts RENAME TO workflow_transition_definitions").execute(&p).await.unwrap();
    }
}

#[tokio::test]
async fn persisted_future_eligibility_does_not_add_a_transition_time_gate() {
    let p=fresh_pool("waitingtime").await;
    let f=fixture::setup(&p,3).await;
    let instance=fixture::create(&p,f).await;
    let jwks=common::MockJwksServer::start().await;
    let app=app(&p,&jwks,true);
    let agent=token(&jwks,f.agent,"workflow.execute workflow.read");
    let before=observed_detail(&p,&app,instance,&agent,"time-before-future-fact").await;
    assert_actions(&before,"AVAILABLE",edge_set(&[(f.advance,"ADVANCE")],&["request_assistance"]));
    // Same persisted fixture mechanism as existing 28::wake_applies_and_noops;
    // no fake classification string and no SCHEDULER_DEFER API claim.
    let (activation,initial):(Uuid,chrono::DateTime<chrono::Utc>)=sqlx::query_as("SELECT activation_id,initial_next_eligible_at FROM workflow_activations WHERE workflow_instance_id=$1 AND node_visit_id=$2 AND activation_kind='DISPATCH_INTENT'")
        .bind(instance).bind(serde_json::from_value::<Uuid>(before["current_node_visit_id"].clone()).unwrap()).fetch_one(&p).await.unwrap();
    let future=initial+chrono::Duration::hours(1);
    let event=Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_dispatch_eligibility_events(eligibility_event_id,activation_id,previous_next_eligible_at,new_next_eligible_at,cause_class,command_id) VALUES($1,$2,$3,$4,'SCHEDULER_DEFER',$5)")
        .bind(event).bind(activation).bind(initial).bind(future).bind(Uuid::new_v4()).execute(&p).await.unwrap();
    let d=observed_detail(&p,&app,instance,&agent,"time-real-waiting-detail").await;
    assert_eq!(d["current_visit"]["eligibility"]["classification"],"WAITING_FOR_TIME");
    let projected=chrono::DateTime::parse_from_rfc3339(d["current_visit"]["eligibility"]["nextEligibleAt"].as_str().unwrap()).unwrap().with_timezone(&chrono::Utc);
    assert_eq!(projected,future);assert!(future>chrono::Utc::now());
    assert_actions(&d,"AVAILABLE",edge_set(&[(f.advance,"ADVANCE")],&["request_assistance"]));
    require_http(&app,"POST",&format!("{}/transitions",path(instance)),&agent,
        transition_body(f.advance,1,json!({})),200,"time-independent-transition-allowed").await;
    let closure:i64=sqlx::query_scalar("SELECT count(*) FROM workflow_activation_closures WHERE activation_id=$1 AND closure_reason='TRANSITIONED'")
        .bind(activation).fetch_one(&p).await.unwrap();assert_eq!(closure,1);
    let retained:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM workflow_dispatch_eligibility_events WHERE eligibility_event_id=$1 AND activation_id=$2 AND new_next_eligible_at=$3)")
        .bind(event).bind(activation).bind(future).fetch_one(&p).await.unwrap();assert!(retained);
    persist_probe("time-persisted-fact-and-transition",&json!({"activation_id":activation,"eligibility_event_id":event,"future":future,"projected":d["current_visit"]["eligibility"],"closure_count":closure,"eligibility_fact_retained":retained}));
}
