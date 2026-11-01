//! Block the original integrity read after its RR Instance/Owner snapshot, then commit Owner change.
use super::{common,fixture,support::*};use serde_json::json;use uuid::Uuid;
#[tokio::test]
async fn repeatable_read_authority_snapshot_is_pre_owner_replacement_then_rechecks() {
    let p=fresh_pool("snapshot").await;let f=fixture::setup_variant(&p,1,true).await;let id=fixture::create(&p,f).await;
    let jwks=common::MockJwksServer::start().await;let app=app(&p,&jwks,true);let tok=token(&jwks,f.owner,"workflow.execute workflow.read");
    let mut blocker=p.begin().await.unwrap();
    let blocker_pid:i32=sqlx::query_scalar("SELECT pg_backend_pid()").fetch_one(&mut *blocker).await.unwrap();
    sqlx::query("LOCK TABLE workflow_transition_definitions IN ACCESS EXCLUSIVE MODE").execute(&mut *blocker).await.unwrap();
    let read_app=app.clone();let read_tok=tok.clone();let read=tokio::spawn(async move{send(&read_app,"GET",&path(id),&read_tok,None).await});
    // authorized_snapshot already read the Instance and Owner in RR before its
    // integrity LEFT JOIN is blocked. A relation barrier also blocks that read,
    // so observing only the later load_outgoing FROM can never synchronize it.
    let deadline=tokio::time::Instant::now()+std::time::Duration::from_secs(10);
    let mut observation:Option<serde_json::Value>=None;
    while tokio::time::Instant::now()<deadline {
        observation=sqlx::query_scalar("SELECT to_jsonb(observed) FROM (
            SELECT a.pid AS reader_pid,a.datname AS database_name,a.state,
                   a.wait_event_type,a.wait_event,a.query,a.backend_xmin::text AS backend_xmin,
                   pg_blocking_pids(a.pid) AS blocker_pids,$1::int AS expected_blocker_pid,
                   l.relation::regclass::text AS relation_name,l.mode,l.granted
            FROM pg_stat_activity a JOIN pg_locks l ON l.pid=a.pid
            WHERE a.datname=current_database() AND a.state='active' AND a.wait_event_type='Lock'
              AND $1=ANY(pg_blocking_pids(a.pid)) AND l.locktype='relation'
              AND l.relation='workflow_transition_definitions'::regclass
              AND l.mode='AccessShareLock' AND NOT l.granted
              AND a.query LIKE '%FROM workflow_submissions s%'
              AND a.query LIKE '%LEFT JOIN workflow_transition_definitions t%'
            ORDER BY a.pid LIMIT 1) observed")
            .bind(blocker_pid).fetch_optional(&p).await.unwrap();
        if observation.is_some() {break;}
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
    let Some(observed)=observation else {
        let diagnostics:serde_json::Value=sqlx::query_scalar("SELECT COALESCE(jsonb_agg(jsonb_build_object(
            'pid',pid,'state',state,'wait_event_type',wait_event_type,'wait_event',wait_event,
            'query',query,'backend_xmin',backend_xmin::text,'blocking_pids',pg_blocking_pids(pid)) ORDER BY pid),'[]'::jsonb)
            FROM pg_stat_activity WHERE datname=current_database()")
            .fetch_one(&p).await.unwrap();
        persist_probe("snapshot-owner-sync-timeout",&json!({"blocker_pid":blocker_pid,"diagnostics":diagnostics}));
        blocker.rollback().await.unwrap();read.abort();let _=read.await;
        panic!("RR reader was not observed at the exact blocker/integrity relation barrier within 10s");
    };
    assert_ne!(observed["reader_pid"],json!(blocker_pid));
    assert!(observed["blocker_pids"].as_array().unwrap().contains(&json!(blocker_pid)));
    assert_eq!(observed["relation_name"],"workflow_transition_definitions");
    assert_eq!(observed["mode"],"AccessShareLock");assert_eq!(observed["granted"],false);
    assert!(observed["backend_xmin"].as_str().is_some(),"reader retains a real snapshot");
    persist_probe("snapshot-owner-observed-reader",&observed);
    // Fixture-only atomic role replacement needs no transition-table lock.
    let mut change=p.begin().await.unwrap();sqlx::query("UPDATE domain_role_bindings SET enabled=FALSE WHERE domain_id=$1 AND role_key='DOMAIN_OWNER' AND enabled").bind(f.domain).execute(&mut *change).await.unwrap();sqlx::query("INSERT INTO domain_role_bindings(binding_id,domain_id,principal_id,role_key,enabled) VALUES($1,$2,$3,'DOMAIN_OWNER',TRUE)").bind(Uuid::new_v4()).bind(f.domain).bind(f.outsider).execute(&mut *change).await.unwrap();change.commit().await.unwrap();blocker.rollback().await.unwrap();
    let(s,b)=read.await.unwrap();assert_eq!(s,200,"{b}");assert_actions(&b["detail"],"AVAILABLE",json!([action("cancel")]));
    let(s,after)=send(&app,"GET",&path(id),&tok,None).await;assert_eq!(s,404,"fresh read must see replaced Owner");
    let before=raw_probe_facts(&p,id,&[f.owner]).await;let(s,refused)=send(&app,"POST",&format!("{}/cancel",path(id)),&tok,Some(json!({"reason":"read permitted only at old snapshot"}))).await;assert_eq!(s,403,"{refused}");let facts=raw_probe_facts(&p,id,&[f.owner]).await;for k in ["instance","events","visits","submissions"] {assert_eq!(before[k],facts[k]);}
    persist_probe("snapshot-owner",&json!({"observed_reader":observed,"old_snapshot":b,"fresh_status":404,"fresh_response":after,"execute_refusal":refused,"before":before,"after":facts}));
}
