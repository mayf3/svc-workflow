//! Projection tests for the engine RETURN contract merge
//! (mayf3/svc-workflow#84 D1): outgoing_transitions for RETURN-effect
//! transitions must project a submission schema that includes the
//! engine-required fields (rootCauseNodeVisitId / reasonCode / reason,
//! optional relatedSubmissionIds) — regardless of what the Definition's
//! stored schema declares — because the transition executor enforces them
//! (transition_validation::validate_return_references). Schema-valid but
//! contract-invalid payloads were the 2026-10-25 Definition-Repair blocker.
//!
//! Also pins: ADVANCE/TERMINATE projections are untouched; the merge is
//! projection-only (a full-contract RETURN still executes).
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
        jwks_config: JwksConfig {
            jwks_url: jwks_url.to_string(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
        provisioning_config: ProvisioningConfig::new(vec![]),
        execution_control: svc_workflow::http::ExecutionControlConfig { max_returns_per_edge: 3 },
        auth_v1_canary_config: AuthV1CanaryConfig {
            enabled: true,
            write_enabled: true,
            allowed_client_id: "test-client".to_string(),
            allowed_sub: String::new(),
            allowed_delegating_sub: String::new(),
            jwks_url: jwks_url.to_string(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 300,
            http_timeout_secs: 5,
            max_stale_secs: 600,
            clock_skew_seconds: 60,
        },
    };
    http::router(AppState::new(pool, &config), &config)
}

async fn do_get(app: axum::Router, path: &str, token: &str) -> (u16, Value) {
    let req = Request::builder().method("GET").uri(path)
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty()).unwrap();
    let resp = app.oneshot(req).await.unwrap();
    let status = resp.status().as_u16();
    let bytes = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!({})))
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

/// Graph mirroring cto-workflow-development-v1: implement --ADVANCE--> canary
/// --RETURN--> implement, canary --ADVANCE--> completed, TERMINATE exits.
async fn seed_v3_style_definition(pool: &PgPool, domain_id: Uuid) -> (Uuid, Uuid, Uuid) {
    let def_id = Uuid::new_v4();
    let ver_id = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_definitions (workflow_definition_id, domain_id, definition_key, display_name) VALUES ($1,$2,$3,'Return Contract Test')")
        .bind(def_id).bind(domain_id).bind(format!("ret-ctr-{}", &Uuid::new_v4().to_string()[..8]))
        .execute(pool).await.expect("def");
    sqlx::query("INSERT INTO workflow_definition_versions (definition_version_id, workflow_definition_id, version_number, version_status, context_schema, semantic_model_version) VALUES ($1,$2,1,'DRAFT',NULL,3)")
        .bind(ver_id).bind(def_id).execute(pool).await.expect("ver");

    let implement = Uuid::new_v4(); let canary = Uuid::new_v4(); let done = Uuid::new_v4(); let human = Uuid::new_v4();
    let no_ref: Option<&str> = None;
    for (id, key, idx, ntype, aref) in [(implement,"implement",0i32,"TASK",Some("WORKFLOW_CREATOR")),(canary,"canary",1,"TASK",Some("WORKFLOW_CREATOR")),(done,"completed",2,"TERMINAL",no_ref),(human,"human_required",3,"TERMINAL",no_ref)] {
        sqlx::query("INSERT INTO workflow_node_definitions (node_id, definition_version_id, node_key, display_name, order_index, node_type, assignee_ref_type) VALUES ($1,$2,$3,$3,$4,$5::node_type,$6::assignee_ref_type)")
            .bind(id).bind(ver_id).bind(key).bind(idx).bind(ntype).bind(aref)
            .execute(pool).await.expect("node");
    }
    let (to_canary, rollback, to_done) = (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
    // RETURN transition carries a STORED schema that deliberately omits the
    // engine fields — the exact #84 trap.
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema) VALUES ($1,$2,'to_canary','To Canary',$3,$4,'ADVANCE',NULL)")
        .bind(to_canary).bind(ver_id).bind(implement).bind(canary).execute(pool).await.expect("t1");
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema) VALUES ($1,$2,'rollback_to_implementation','Rollback',$3,$4,'RETURN',$5)")
        .bind(rollback).bind(ver_id).bind(canary).bind(implement)
        .bind(serde_json::json!({"type":"object","required":["failureEvidence","rollbackAction"],"properties":{"failureEvidence":{"type":"string"},"rollbackAction":{"type":"string"}}}))
        .execute(pool).await.expect("t2");
    sqlx::query("INSERT INTO workflow_transition_definitions (transition_id, definition_version_id, transition_key, display_name, source_node_id, target_node_id, transition_effect, submission_schema) VALUES ($1,$2,'development_complete','Complete',$3,$4,'ADVANCE',NULL)")
        .bind(to_done).bind(ver_id).bind(canary).bind(done).execute(pool).await.expect("t3");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2").bind(to_canary).bind(implement).execute(pool).await.expect("p1");
    sqlx::query("UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2").bind(to_done).bind(canary).execute(pool).await.expect("p2");
    sqlx::query("UPDATE workflow_definition_versions SET version_status='PUBLISHED' WHERE definition_version_id=$1").bind(ver_id).execute(pool).await.expect("publish");
    (ver_id, to_canary, rollback)
}

#[tokio::test]
async fn return_contract_projected_and_executable() {
    let pool = common::create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (owner, domain) = common::seed_principal_domain_with_owner(&pool).await;
    let (ver_id, to_canary, rollback) = seed_v3_style_definition(&pool, domain).await;
    sqlx::query("INSERT INTO domain_role_bindings (binding_id, domain_id, principal_id, role_key, enabled) VALUES ($1,$2,$3,'DOMAIN_MEMBER',TRUE)")
        .bind(Uuid::new_v4()).bind(domain).bind(owner).execute(&pool).await.expect("member");

    let inst = svc_workflow::application::workflow_instance::create::create_workflow_instance(
        &pool, svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner),
            idempotency_key: format!("create-{}", Uuid::new_v4()),
            command_schema_version: "v1".into(),
            execution_class: svc_workflow::domain::enums::WorkflowExecutionClass::NonBusinessTest,
            domain_id: DomainId::from_uuid(domain),
            definition_version_id: DefinitionVersionId::from_uuid(ver_id),
            external_reference: None, external_url: None, metadata: json!({}),
            context_payload: json!({"title":"return contract projection"}),
        }).await.expect("create").workflow_instance_id;

    let app = build_app(pool.clone(), &mock.url);
    let tok = common::v1_token(owner, "workflow.execute workflow.read", "test-client", 300, &mock.key_pair);

    // Advance implement -> canary.
    let (s, body) = do_post(app.clone(), &format!("/internal/v1/workflow-instances/{inst}/transitions"), &tok,
        json!({"transitionDefinitionId": to_canary.to_string(), "expectedWorkflowStateVersion": 1, "submissionPayload": {"summary": "to canary"}}), "rcp-adv-1").await;
    assert_eq!(s, 200, "advance: {body}");

    // Detail: RETURN projection must carry the engine contract on top of the
    // stored schema; ADVANCE projections untouched.
    let (s, detail) = do_get(app.clone(), &format!("/internal/v1/workflow-instances/{inst}"), &tok).await;
    assert_eq!(s, 200);
    let ots = detail["detail"]["outgoing_transitions"].as_array().unwrap().clone();
    let rb = ots.iter().find(|t| t["transition_key"] == json!("rollback_to_implementation")).expect("return listed");
    let schema = &rb["submission_schema"];
    for f in ["rootCauseNodeVisitId", "reasonCode", "reason"] {
        assert!(schema["required"].as_array().unwrap().iter().any(|v| v == f), "required must include {f}: {schema}");
        assert!(schema["properties"][f].is_object(), "properties must describe {f}: {schema}");
    }
    assert!(schema["required"].as_array().unwrap().iter().any(|v| v == "failureEvidence"), "stored schema fields preserved");
    let adv = ots.iter().find(|t| t["transition_key"] == json!("development_complete")).unwrap();
    assert!(adv["submission_schema"].is_null(), "ADVANCE schema untouched: {}", adv["submission_schema"]);

    // Execution behavior unchanged: full-contract RETURN succeeds with a
    // payload satisfying BOTH the stored schema and the engine contract.
    let visits: Vec<(Uuid, String)> = sqlx::query_as(
        "SELECT v.node_visit_id, n.node_key FROM workflow_node_visits v JOIN workflow_node_definitions n ON n.node_id=v.node_id WHERE v.workflow_instance_id=$1 ORDER BY v.visit_number")
        .bind(inst).fetch_all(&pool).await.expect("visits");
    let implement_visit = visits.iter().find(|(_, k)| k == "implement").map(|(v, _)| *v).expect("implement visit");
    let (s, body) = do_post(app.clone(), &format!("/internal/v1/workflow-instances/{inst}/transitions"), &tok,
        json!({"transitionDefinitionId": rollback.to_string(), "expectedWorkflowStateVersion": 2,
               "submissionPayload": {"failureEvidence":"projection test","rollbackAction":"back",
                                     "rootCauseNodeVisitId": implement_visit.to_string(),
                                     "reasonCode":"EVIDENCE_INCOMPLETE","reason":"projection contract test"}}), "rcp-ret-1").await;
    assert_eq!(s, 200, "return executes: {body}");
    assert_eq!(body["workflowStateVersion"], json!(3), "returned at v3");
}
