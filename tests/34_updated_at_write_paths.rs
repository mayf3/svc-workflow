//! Regression: transition/revise write paths must maintain
//! workflow_instances.updated_at (2026-10-26; task 7cd02846 root cause —
//! ADVANCE/RETURN projection updates, combined revise+transition and
//! revise-only all skipped updated_at while cancel/archive maintained it;
//! production census showed 312/654 advanced instances with
//! updated_at == created_at, making global summaries misleading).
#![allow(clippy::needless_borrow)]
#![allow(unused_imports, unused_variables)]

#[path = "common/mod.rs"]
mod common;

use axum::body::{to_bytes, Body};
use axum::http::Request;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::commands::CreateWorkflowInstanceCommand;
use svc_workflow::http::{self, AppState, HttpConfig};

fn build_app(pool: sqlx::PgPool, jwks_url: &str) -> axum::Router {
    let config = HttpConfig {
        admission: svc_workflow::auth::admission::AdmissionConfig::disabled(),
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        request_body_max_bytes: 2_097_152,
        request_timeout_seconds: 30,
        jwks_config: JwksConfig { jwks_url: jwks_url.to_string(), issuer: "auth-service".into(), audience: "svc-workflow".into(), cache_ttl_secs: 300, http_timeout_secs: 5, max_stale_secs: 600, clock_skew_seconds: 60 },
        provisioning_config: ProvisioningConfig::new(vec![]),
        execution_control: svc_workflow::http::ExecutionControlConfig { max_returns_per_edge: 3 },
        auth_v1_canary_config: AuthV1CanaryConfig { enabled: true, write_enabled: true, allowed_client_id: "test-client".into(), allowed_sub: String::new(), allowed_delegating_sub: String::new(), jwks_url: jwks_url.to_string(), issuer: "auth-service".into(), audience: "svc-workflow".into(), cache_ttl_secs: 300, http_timeout_secs: 5, max_stale_secs: 600, clock_skew_seconds: 60 },
    };
    http::router(AppState::new(pool, &config), &config)
}

async fn do_post(app: axum::Router, path: &str, token: &str, body: Value, idem: &str) -> (u16, Value) {
    let req = Request::builder().method("POST").uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("idempotency-key", idem)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string())).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
}

/// Same graph shape as test 33: implement --ADVANCE--> canary --RETURN-->
/// implement, canary --ADVANCE--> completed.
async fn seed(pool: &PgPool, domain: Uuid) -> (Uuid, Uuid, Uuid) {
    let (def, ver) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1,$2,$3,'updated_at test')")
        .bind(def).bind(domain).bind(format!("upd-{}", &Uuid::new_v4().to_string()[..8])).execute(pool).await.expect("def");
    sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema, semantic_model_version) VALUES ($1,$2,1,'DRAFT',NULL,3)")
        .bind(ver).bind(def).execute(pool).await.expect("ver");
    let (a, c, done) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    for (id, key, idx, ty, aref) in [(a,"implement",0,"TASK",Some("WORKFLOW_CREATOR")),(c,"canary",1,"TASK",Some("WORKFLOW_CREATOR")),(done,"completed",2,"TERMINAL",None::<&str>)] {
        sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1,$2,$3,$3,$4,$5::node_type,$6::assignee_ref_type)")
            .bind(id).bind(ver).bind(key).bind(idx).bind(ty).bind(aref).execute(pool).await.expect("node");
    }
    let (adv, ret, fin) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    for (id, key, src, tgt, eff) in [(adv,"to_canary",a,c,"ADVANCE"),(ret,"rollback",c,a,"RETURN"),(fin,"to_done",c,done,"ADVANCE")] {
        sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1,$2,$3,$3,$4,$5,$6::transition_effect)")
            .bind(id).bind(ver).bind(key).bind(src).bind(tgt).bind(eff).execute(pool).await.expect("t");
    }
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2").bind(adv).bind(a).execute(pool).await.expect("p1");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2").bind(fin).bind(c).execute(pool).await.expect("p2");
    sqlx::query("UPDATE workflow_definition_versions SET version_status='PUBLISHED' WHERE definition_version_id=$1").bind(ver).execute(pool).await.expect("pub");
    (ver, adv, ret)
}

#[tokio::test]
async fn advance_and_return_maintain_updated_at() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let (ver, adv, ret) = seed(&pool, domain).await;
    sqlx::query("INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1,$2,$3,'DOMAIN_MEMBER',TRUE)")
        .bind(Uuid::new_v4()).bind(domain).bind(owner).execute(&pool).await.expect("member");

    let inst = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool, svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner), idempotency_key: format!("c-{}", Uuid::new_v4()),
            command_schema_version: "v1".into(),
            execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::NonBusinessTest,
            domain_id: DomainId::from_uuid(domain), definition_version_id: DefinitionVersionId::from_uuid(ver),
            external_reference: None, external_url: None, metadata: json!({}), context_payload: json!({"title":"updated_at"}),
        }).await.expect("create").workflow_instance_id;

    let stamp: (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as("SELECT created_at, updated_at FROM workflow_instances WHERE workflow_instance_id=$1")
            .bind(inst).fetch_one(&pool).await.expect("t0");
    assert_eq!(stamp.0, stamp.1, "fresh instance baseline");

    let app = build_app(pool.clone(), &mock.url);
    let tok = common::v1_token(owner, "workflow.execute workflow.read", "test-client", 300, &mock.key_pair);
    let (s, body) = do_post(app.clone(), &format!("/internal/v1/workflow-instances/{inst}/transitions"), &tok,
        json!({"transitionDefinitionId": adv.to_string(), "expectedWorkflowStateVersion": 1, "submissionPayload": {"summary":"advance"}}), "upd-adv-1").await;
    assert_eq!(s, 200, "{body}");
    let after_adv: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM workflow_instances WHERE workflow_instance_id=$1")
            .bind(inst).fetch_one(&pool).await.expect("t1");
    assert!(after_adv > stamp.0, "ADVANCE must bump updated_at (root cause of 7cd02846)");

    let visits: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT v.node_visit_id, n.node_key FROM workflow_node_visits v JOIN workflow_node_definitions n ON n.node_id=v.node_id WHERE v.workflow_instance_id=$1 ORDER BY v.visit_number")
        .bind(inst).fetch_all(&pool).await.expect("visits");
    let implement_visit = visits.iter().find(|(_, k)| k == "implement").map(|(v, _)| *v).unwrap();
    let (s, body) = do_post(app.clone(), &format!("/internal/v1/workflow-instances/{inst}/transitions"), &tok,
        json!({"transitionDefinitionId": ret.to_string(), "expectedWorkflowStateVersion": 2,
               "submissionPayload": {"failureEvidence":"t","rollbackAction":"t","rootCauseNodeVisitId": implement_visit.to_string(),"reasonCode":"T","reason":"t"}}), "upd-ret-1").await;
    assert_eq!(s, 200, "{body}");
    let after_ret: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM workflow_instances WHERE workflow_instance_id=$1")
            .bind(inst).fetch_one(&pool).await.expect("t2");
    assert!(after_ret > after_adv, "RETURN must bump updated_at again");
}

/// Minimal DRAFT+TERMINAL graph for the revise-only write path (mirrors the
/// tests/17 seed used by test_revise_context_by_creator_succeeds; revise
/// requires the current node to be a DRAFT node). Returns version_id.
async fn seed_draft_terminal(pool: &PgPool, domain: Uuid) -> Uuid {
    let (def, ver) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1,$2,$3,'updated_at revise test')")
        .bind(def).bind(domain).bind(format!("upd-rev-{}", &Uuid::new_v4().to_string()[..8])).execute(pool).await.expect("def");
    sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema) VALUES ($1,$2,1,'DRAFT',NULL)")
        .bind(ver).bind(def).execute(pool).await.expect("ver");
    let (d, t) = (Uuid::new_v4(), Uuid::new_v4());
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1,$2,'draft','draft',0,'DRAFT','WORKFLOW_CREATOR')")
        .bind(d).bind(ver).execute(pool).await.expect("draft node");
    sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1,$2,'done','done',1,'TERMINAL',NULL)")
        .bind(t).bind(ver).execute(pool).await.expect("terminal node");
    let adv = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect) VALUES ($1,$2,'advance','advance',$3,$4,'ADVANCE')")
        .bind(adv).bind(ver).bind(d).bind(t).execute(pool).await.expect("t");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2").bind(adv).bind(d).execute(pool).await.expect("p");
    sqlx::query("UPDATE workflow_definition_versions SET version_status='PUBLISHED' WHERE definition_version_id=$1").bind(ver).execute(pool).await.expect("pub");
    ver
}

/// Revise-only write path: creator revise must succeed and maintain
/// workflow_instances.updated_at (regression 2026-10-26: the revise UPDATE in
/// revise_transaction.rs dropped the comma before `updated_at = now()`,
/// making every revise fail with a SQL syntax error, so neither the revise
/// itself nor its updated_at maintenance could execute; tests/17 exercises
/// revise but never asserts updated_at, and this file previously covered
/// ADVANCE/RETURN only).
#[tokio::test]
async fn revise_only_maintains_updated_at() {
    let pool = common::create_pool().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let ver = seed_draft_terminal(&pool, domain).await;
    sqlx::query("INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1,$2,$3,'DOMAIN_MEMBER',TRUE)")
        .bind(Uuid::new_v4()).bind(domain).bind(owner).execute(&pool).await.expect("member");

    let inst = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool, svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner), idempotency_key: format!("c-{}", Uuid::new_v4()),
            command_schema_version: "v1".into(),
            execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::Business,
            domain_id: DomainId::from_uuid(domain), definition_version_id: DefinitionVersionId::from_uuid(ver),
            external_reference: None, external_url: None, metadata: json!({}), context_payload: json!({"title":"v1"}),
        }).await.expect("create").workflow_instance_id;

    let baseline: (chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>) =
        sqlx::query_as("SELECT created_at, updated_at FROM workflow_instances WHERE workflow_instance_id=$1")
            .bind(inst).fetch_one(&pool).await.expect("t0");
    assert_eq!(baseline.0, baseline.1, "fresh instance baseline");

    let result = svc_workflow::application::workflow_instance::revise::revise_workflow_context(
        &pool,
        svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        svc_workflow::domain::workflow_instance::commands::ReviseWorkflowContextCommand {
            principal_id: PrincipalId::from_uuid(owner),
            idempotency_key: format!("c-{}", Uuid::new_v4()),
            command_schema_version: "v1".into(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(inst),
            expected_workflow_state_version: 1,
            context_payload: json!({"title":"v2"}),
        },
    ).await.expect("creator revise must succeed");
    assert_eq!(result.workflow_state_version, 2);
    assert_eq!(result.event_sequence, 2);

    let after_rev: chrono::DateTime<chrono::Utc> =
        sqlx::query_scalar("SELECT updated_at FROM workflow_instances WHERE workflow_instance_id=$1")
            .bind(inst).fetch_one(&pool).await.expect("t1");
    assert!(after_rev > baseline.0, "revise-only must bump updated_at (task 7cd02846 write-path obligation)");

    // Instance projection carries the new revision and state version.
    let inst_row: (i32, Uuid) = sqlx::query_as(
        "SELECT workflow_state_version, current_context_revision_id FROM workflow_instances WHERE workflow_instance_id=$1")
        .bind(inst).fetch_one(&pool).await.expect("inst row");
    assert_eq!(inst_row.0, 2);
    assert_eq!(inst_row.1, result.current_context_revision_id);
}
