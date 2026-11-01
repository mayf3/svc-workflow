use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::{
    application::provisioning::ProvisioningConfig,
    auth::{AuthV1CanaryConfig, JwksConfig},
    http::{self, AppState, HttpConfig},
};

pub fn app(
    pool: &PgPool,
    jwks: &crate::common::MockJwksServer,
    write_enabled: bool,
) -> axum::Router {
    let config = HttpConfig {
        admission: svc_workflow::auth::admission::AdmissionConfig::disabled(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        request_body_max_bytes: 2_097_152,
        request_timeout_seconds: 30,
        jwks_config: JwksConfig {
            jwks_url: jwks.url.clone(),
            issuer: "auth-service".into(),
            audience: "svc-workflow".into(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
        provisioning_config: ProvisioningConfig::new(vec![]),
        execution_control: http::ExecutionControlConfig {
            max_returns_per_edge: 3,
        },
        auth_v1_canary_config: AuthV1CanaryConfig {
            enabled: true,
            write_enabled,
            allowed_client_id: "authorized-actions-test".into(),
            allowed_sub: String::new(),
            allowed_delegating_sub: String::new(),
            jwks_url: jwks.url.clone(),
            issuer: "auth-service".into(),
            audience: "svc-workflow".into(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
    };
    http::router(AppState::new(pool.clone(), &config), &config)
}

pub fn token(jwks: &crate::common::MockJwksServer, actor: Uuid, scope: &str) -> String {
    crate::common::v1_token(actor, scope, "authorized-actions-test", 300, &jwks.key_pair)
}

pub fn path(instance: Uuid) -> String {
    format!("/internal/v1/workflow-instances/{instance}")
}

pub async fn send(
    app: &axum::Router,
    method: &str,
    path: &str,
    token: &str,
    body: Option<Value>,
) -> (u16, Value) {
    let mut req = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {token}"));
    let data = if let Some(body) = body {
        req = req
            .header("idempotency-key", Uuid::new_v4().to_string())
            .header("content-type", "application/json");
        Body::from(body.to_string())
    } else {
        Body::empty()
    };
    let resp = app.clone().oneshot(req.body(data).unwrap()).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| json!({"raw":String::from_utf8_lossy(&bytes)}));
    (status, value)
}

pub async fn detail(app: &axum::Router, instance: Uuid, token: &str) -> Value {
    let (status, body) = send(app, "GET", &path(instance), token, None).await;
    assert_eq!(status, 200, "detail response: {body}");
    assert_eq!(body["visibility"], "full", "{body}");
    body["detail"].clone()
}

pub fn transition_body(transition: Uuid, version: i32, payload: Value) -> Value {
    json!({"transitionDefinitionId":transition,"expectedWorkflowStateVersion":version,"submissionPayload":payload})
}

pub fn action(operation: &str) -> Value {
    json!({"operation":operation,"request_validation":"REQUIRED_AT_EXECUTION"})
}

pub fn transition_action(id: Uuid, effect: &str) -> Value {
    json!({"operation":"transition","transition_id":id,"transition_effect":effect,"request_validation":"REQUIRED_AT_EXECUTION"})
}

pub fn assert_actions(detail: &Value, status: &str, actions: Value) {
    assert_eq!(
        detail["authorized_actions_scope"], "INSTANCE_TASK_AND_GOVERNANCE_V1",
        "detail must expose the accepted endpoint-only action projection: {detail}"
    );
    assert_eq!(detail["authorized_actions_status"], status, "{detail}");
    assert_eq!(
        detail["authorized_actions"], actions,
        "literal operation set differs: {detail}"
    );
}

/// Raw isolated-fixture facts, persisted before semantic status assertions.
pub async fn raw_probe_facts(pool: &PgPool, instance: Uuid, actors: &[Uuid]) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object(
        'instance',(SELECT to_jsonb(i) FROM workflow_instances i WHERE workflow_instance_id=$1),
        'events',COALESCE((SELECT jsonb_agg(to_jsonb(e) ORDER BY event_sequence) FROM workflow_events e WHERE workflow_instance_id=$1),'[]'::jsonb),
        'visits',COALESCE((SELECT jsonb_agg(to_jsonb(v) ORDER BY created_at,node_visit_id) FROM workflow_node_visits v WHERE workflow_instance_id=$1),'[]'::jsonb),
        'submissions',COALESCE((SELECT jsonb_agg(to_jsonb(s)) FROM workflow_submissions s WHERE workflow_instance_id=$1),'[]'::jsonb),
        'receipts',COALESCE((SELECT jsonb_agg(to_jsonb(r) ORDER BY created_at,command_id) FROM workflow_command_receipts r WHERE principal_id=ANY($2)),'[]'::jsonb),
        'principals',COALESCE((SELECT jsonb_agg(to_jsonb(p)) FROM principals p WHERE principal_id=ANY($2)),'[]'::jsonb))")
        .bind(instance).bind(actors).fetch_one(pool).await.unwrap()
}

pub fn persist_probe(name: &str, value: &Value) {
    let path = format!("{}/{name}.json", std::env::var("AA_RESULT_DIR").expect("explicit isolated evidence path"));
    std::fs::write(&path, serde_json::to_vec_pretty(value).unwrap()).unwrap();
    eprintln!("RAW_PROBE_RECORD path={path}");
}

/// Every matrix case owns a new database on the one delegated loopback cluster.
pub async fn fresh_pool(label:&str)->PgPool {
    let dsn=std::env::var("TEST_DATABASE_URL").expect("explicit isolated DSN");
    assert!(dsn.starts_with("postgres://swaar4@127.0.0.1:55483/"));
    let base=dsn.rsplit_once('/').unwrap().0;
    let db=format!("aa_r4_{}_{}",label,Uuid::new_v4().simple());
    let admin=PgPool::connect(&format!("{base}/postgres")).await.unwrap();
    sqlx::query(&format!("CREATE DATABASE {db}")).execute(&admin).await.unwrap();
    admin.close().await;
    let p=PgPool::connect(&format!("{base}/{db}")).await.unwrap();
    sqlx::migrate::Migrator::new(std::path::Path::new("migrations")).await.unwrap().run(&p).await.unwrap();
    persist_probe(&format!("database-{db}"),&json!({"database":db,"label":label,"port":55483}));
    p
}
/// Independent full-table read proves detail does not reserve receipts or change side effects.
pub async fn read_all_facts(pool:&PgPool)->Value {
    let mut tx=pool.begin().await.unwrap();
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY").execute(&mut *tx).await.unwrap();
    let tables:Vec<String>=sqlx::query_scalar("SELECT table_name FROM information_schema.tables WHERE table_schema='public' AND table_type='BASE TABLE' ORDER BY table_name").fetch_all(&mut *tx).await.unwrap();
    let mut o=serde_json::Map::new();
    for t in tables {
        let rows:Value=sqlx::query_scalar(&format!("SELECT COALESCE(jsonb_agg(to_jsonb(t) ORDER BY to_jsonb(t)::text),'[]'::jsonb) FROM \"{}\" t",t.replace('"',"\"\""))).fetch_one(&mut *tx).await.unwrap();
        o.insert(t,rows);
    }
    tx.commit().await.unwrap();Value::Object(o)
}
pub async fn observed_detail(pool:&PgPool,app:&axum::Router,instance:Uuid,token:&str,label:&str)->Value {
    let before=read_all_facts(pool).await;
    let d=detail(app,instance,token).await;
    let after=read_all_facts(pool).await;
    persist_probe(label,&json!({"instance":instance,"detail":d,"before":before,"after":after}));
    assert_eq!(before,after,"SEMANTIC_STOP: successful detail changed database facts");d
}
pub fn assert_task_type(d:&Value,node_type:&str) {
    assert_eq!(d["current_task_type"],json!({"source":"CURRENT_NODE_TYPE","node_type":node_type,"business_task_type":null}));
    assert_eq!(d["current_visit"]["node"]["node_type"],node_type);
    assert_eq!(d.as_object().unwrap().len(),10,"endpoint full is a closed ten-field object");
}
pub fn case_action(operation:&str,case:&str)->Value {json!({"operation":operation,"assistance_case_id":case,"request_validation":"REQUIRED_AT_EXECUTION"})}
pub fn edge_set(edges:&[(Uuid,&str)],tail:&[&str])->Value {
    let mut e=edges.to_vec(); e.sort_by_key(|(id,_)|*id.as_bytes());
    let mut a:Vec<Value>=e.into_iter().map(|(id,effect)|transition_action(id,effect)).collect();
    a.extend(tail.iter().map(|op|action(op)));json!(a)
}
pub async fn require_http(app:&axum::Router,method:&str,path:&str,token:&str,body:Value,want:u16,label:&str)->Value {
    let(s,b)=send(app,method,path,token,Some(body.clone())).await;
    persist_probe(label,&json!({"method":method,"path":path,"request":body,"status":s,"response":b}));
    assert_eq!(s,want,"SEMANTIC_STOP: independent writer result differs in {label}: {b}");b
}
