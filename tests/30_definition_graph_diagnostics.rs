//! Accepted graph diagnostics: real TCP, canonical validator, disposable DB.
#[path = "common/mod.rs"]
mod common;
use serde_json::{json, Value};
use svc_workflow::application::definition::repository::DefinitionRepository;
use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};
use svc_workflow::store::postgres::definition_repository::PgDefinitionRepository;
use uuid::Uuid;
fn build_app(pool: sqlx::PgPool, jwks_url: &str, admin_ids: Vec<Uuid>) -> axum::Router {
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
        provisioning_config: ProvisioningConfig::new(
            admin_ids
                .into_iter()
                .map(svc_workflow::domain::ids::PrincipalId::from_uuid)
                .collect(),
        ),
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

async fn request(
    client: &reqwest::Client,
    root: &str,
    method: &str,
    path: &str,
    token: &str,
    key: &str,
    body: Value,
) -> (u16, Value) {
    let response = client
        .request(method.parse().unwrap(), format!("{root}{path}"))
        .bearer_auth(token)
        .header("idempotency-key", key)
        .json(&body)
        .send()
        .await
        .unwrap();
    (response.status().as_u16(), response.json().await.unwrap())
}
fn key() -> String {
    Uuid::new_v4().to_string()
}
fn graph(id: Uuid) -> Value {
    json!({"definitionVersionId":id,"nodes":[
        {"node_key":"work","display_name":"Work","order_index":0,"node_type":"TASK","assignee_ref_type":"WORKFLOW_CREATOR","primary_advance_transition_key":"finish"},
        {"node_key":"done","display_name":"Done","order_index":1,"node_type":"TERMINAL"}],
        "transitions":[{"transition_key":"finish","display_name":"Finish","source_node_key":"work","target_node_key":"done","transition_effect":"ADVANCE"}]})
}

#[tokio::test]
async fn canonical_failures_are_actionable_atomic_and_replayable_over_tcp() {
    let pool = common::create_pool().await;
    let owner = common::seed_second_principal(&pool).await;
    let outsider = common::seed_second_principal(&pool).await;
    let (_, domain) = common::seed_principal_and_domain(&pool).await;
    common::seed_domain_owner(&pool, domain, owner).await;
    let jwks = common::MockJwksServer::start().await;
    let app = build_app(pool.clone(), &jwks.url, vec![]);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::builder().no_proxy().build().unwrap();
    let token = common::v1_token(
        owner,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &jwks.key_pair,
    );
    let other_token = common::v1_token(
        outsider,
        "workflow.execute workflow.read",
        "test-client",
        300,
        &jwks.key_pair,
    );
    let path = format!("/internal/v1/domains/{domain}/definitions");
    let (status, def) = request(
        &client,
        &root,
        "POST",
        &path,
        &token,
        &key(),
        json!({"definitionKey":key(),"displayName":"Diagnostics"}),
    )
    .await;
    assert_eq!(status, 200, "{def}");
    let base = format!("{path}/{}", def["workflowDefinitionId"].as_str().unwrap());
    let repo = PgDefinitionRepository::new(pool.clone());
    for model in [1, 3] {
        let (status, version) = request(
            &client,
            &root,
            "POST",
            &format!("{base}/versions"),
            &token,
            &key(),
            json!({"semanticModelVersion":model}),
        )
        .await;
        assert_eq!(status, 200, "{version}");
        let id = Uuid::parse_str(version["definitionVersionId"].as_str().unwrap()).unwrap();
        // Publishing an empty draft uses the same typed diagnostic and replay path.
        let publish_key = key();
        let publish_path = format!("{base}/publish");
        let empty = request(
            &client,
            &root,
            "POST",
            &publish_path,
            &token,
            &publish_key,
            json!({"versionId":id}),
        )
        .await;
        assert_eq!(empty.0, 422, "{empty:?}");
        assert_eq!(empty.1["error"]["code"], "graph_validation_failed");
        assert_eq!(
            request(
                &client,
                &root,
                "POST",
                &publish_path,
                &token,
                &publish_key,
                json!({"versionId":id})
            )
            .await,
            empty
        );
        assert!(repo.get_complete_graph(id).await.unwrap().0.is_empty());
        let mut valid = graph(id);
        if model == 1 {
            valid["nodes"][0]["node_type"] = json!("DRAFT");
        }
        let draft = format!("{base}/draft");
        assert_eq!(
            request(&client, &root, "PUT", &draft, &token, &key(), valid.clone())
                .await
                .0,
            200
        );
        let before_graph =
            serde_json::to_value(repo.get_complete_graph(id).await.unwrap()).unwrap();
        let before_version = serde_json::to_value(repo.get_version(id).await.unwrap()).unwrap();
        let mut cases = Vec::new();
        let mut bad = valid.clone();
        bad["transitions"][0]["target_node_key"] = json!("SQL secret credential missing");
        cases.push((bad, "TRANSITION_TARGET_MISSING"));
        let mut bad = valid.clone();
        bad["nodes"][0]["primary_advance_transition_key"] = json!("absent");
        cases.push((
            bad,
            if model == 1 {
                "MISSING_PRIMARY"
            } else {
                "v1_primary_advance_required"
            },
        ));
        let mut bad = valid.clone();
        bad["transitions"][0]["target_node_key"] = json!("work");
        cases.push((bad, "SELF_LOOP"));
        let mut bad = valid.clone();
        bad["nodes"][0]
            .as_object_mut()
            .unwrap()
            .remove("assignee_ref_type");
        cases.push((bad, "ASSIGNEE_REQUIRED"));
        let mut bad = valid.clone();
        bad["nodes"][1]["assignee_ref_type"] = json!("WORKFLOW_CREATOR");
        cases.push((bad, "TERMINAL_HAS_ASSIGNEE"));
        let mut bad = valid.clone();
        if model == 1 {
            bad["nodes"][0]["node_type"] = json!("NORMAL");
        } else {
            let mut extra = valid["nodes"][0].clone();
            extra["node_key"] = json!("extra");
            extra["order_index"] = json!(2);
            bad["nodes"].as_array_mut().unwrap().push(extra);
        }
        cases.push((
            bad,
            if model == 1 {
                "NO_DRAFT_NODE"
            } else {
                "v1_multiple_entry_tasks"
            },
        ));
        let mut bad = valid.clone();
        let mut terminal = valid["nodes"][1].clone();
        terminal["node_key"] = json!("unreachable");
        terminal["order_index"] = json!(3);
        bad["nodes"].as_array_mut().unwrap().push(terminal);
        cases.push((
            bad,
            if model == 1 {
                "NODE_NOT_REACHABLE"
            } else {
                "v1_unreachable_node"
            },
        ));
        for (bad, code) in cases {
            let receipt_key = key();
            let (status, result) = request(
                &client,
                &root,
                "PUT",
                &draft,
                &token,
                &receipt_key,
                bad.clone(),
            )
            .await;
            assert_eq!(status, 422, "{code}: {result}");
            assert_eq!(result["error"]["code"], "graph_validation_failed");
            let errors = result["error"]["details"]["errors"].as_array().unwrap();
            assert!(errors.iter().any(|e| e["code"] == code), "{code}: {result}");
            let first = errors[0]["code"].as_str().unwrap();
            assert!(result["error"]["message"]
                .as_str()
                .unwrap()
                .contains(&first.replace('_', " ")));
            assert!(!result.to_string().contains("SQL secret"));
            assert_eq!(
                request(&client, &root, "PUT", &draft, &token, &receipt_key, bad).await,
                (status, result)
            );
            // Seed corrupt/historical receipt fixtures by insertion in this disposable DB.
            // Completed production receipts are never edited or replayed here.
            if code == "TRANSITION_TARGET_MISSING" {
                for (mut stored, expected) in [
                    (
                        json!({"error":"graph_validation_failed","details":{"errors":[{"code":"SELF_LOOP","message":"SQL SECRET"}],"truncated":false}}),
                        500,
                    ),
                    (json!({"error":"definition_not_found"}), 404),
                ] {
                    if expected == 500 {
                        let safe_body:Value=sqlx::query_scalar("SELECT response_body FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2").bind(owner).bind(&receipt_key).fetch_one(&pool).await.unwrap();
                        stored["graphInputHash"] = safe_body["graphInputHash"].clone();
                    }
                    let fixture_key = key();
                    sqlx::query("INSERT INTO workflow_command_receipts (command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status,response_status,response_body,response_digest,completed_at) SELECT $1,principal_id,$2,command_type,request_hash,'COMPLETED',422,$3,repeat('0',64),now() FROM workflow_command_receipts WHERE principal_id=$4 AND idempotency_key=$5")
                        .bind(Uuid::new_v4()).bind(&fixture_key).bind(stored).bind(owner).bind(&receipt_key).execute(&pool).await.unwrap();
                    let mut same = valid.clone();
                    same["transitions"][0]["target_node_key"] =
                        json!("SQL secret credential missing");
                    let replay =
                        request(&client, &root, "PUT", &draft, &token, &fixture_key, same).await;
                    assert_eq!(replay.0, expected, "{replay:?}");
                    assert!(!replay.1.to_string().contains("SQL SECRET"));
                    if expected == 500 {
                        assert!(replay.1["error"].get("details").is_none());
                    }
                }
            }
            if code == "TRANSITION_TARGET_MISSING" {
                use sha2::{Digest, Sha256};
                let old_hash=hex::encode(Sha256::digest(serde_json::to_vec(&json!({"commandType":"DEFINITION_REPLACE_DRAFT","command":{"definitionVersionId":id}})).unwrap()));
                let old_key = key();
                sqlx::query("INSERT INTO workflow_command_receipts (command_id,principal_id,idempotency_key,command_type,request_hash,receipt_status,response_status,response_body,response_digest,completed_at) VALUES ($1,$2,$3,'DEFINITION_REPLACE_DRAFT',$4,'COMPLETED',200,'null'::jsonb,repeat('0',64),now())")
                    .bind(Uuid::new_v4()).bind(owner).bind(&old_key).bind(old_hash).execute(&pool).await.unwrap();
                let old = request(
                    &client,
                    &root,
                    "PUT",
                    &draft,
                    &token,
                    &old_key,
                    valid.clone(),
                )
                .await;
                assert_eq!(
                    old.0, 200,
                    "historical success format must replay unchanged: {old:?}"
                );
            }
            let conflict = request(
                &client,
                &root,
                "PUT",
                &draft,
                &token,
                &receipt_key,
                valid.clone(),
            )
            .await;
            assert_eq!(conflict.0, 409, "{conflict:?}");
            assert_eq!(conflict.1["error"]["code"], "idempotency_conflict");
            assert_eq!(
                serde_json::to_value(repo.get_complete_graph(id).await.unwrap()).unwrap(),
                before_graph
            );
            assert_eq!(
                serde_json::to_value(repo.get_version(id).await.unwrap()).unwrap(),
                before_version
            );
        }
        let denied = request(
            &client,
            &root,
            "PUT",
            &draft,
            &other_token,
            &key(),
            valid.clone(),
        )
        .await;
        assert_eq!(denied.0, 404);
        assert_eq!(denied.1["error"]["code"], "definition_not_found");
        assert!(!denied.1.to_string().contains("graph_validation"));
        let published = request(
            &client,
            &root,
            "POST",
            &format!("{base}/publish"),
            &token,
            &key(),
            json!({"versionId":id}),
        )
        .await;
        assert_eq!(published.0, 200, "{published:?}");
        assert_eq!(
            request(&client, &root, "PUT", &draft, &token, &key(), valid)
                .await
                .0,
            409
        );
    }
    // Unknown fixed identity retains the existing opaque storage rejection.
    let (_, version) = request(
        &client,
        &root,
        "POST",
        &format!("{base}/versions"),
        &token,
        &key(),
        json!({"semanticModelVersion":3}),
    )
    .await;
    let id = Uuid::parse_str(version["definitionVersionId"].as_str().unwrap()).unwrap();
    let mut bad = graph(id);
    bad["nodes"][0]["assignee_ref_type"] = json!("FIXED_PRINCIPAL");
    bad["nodes"][0]["fixed_principal_id"] = json!(Uuid::new_v4());
    let missing = request(
        &client,
        &root,
        "PUT",
        &format!("{base}/draft"),
        &token,
        &key(),
        bad.clone(),
    )
    .await;
    assert_eq!(missing.0, 503, "{missing:?}");
    assert_eq!(missing.1["error"]["code"], "service_unavailable");
    assert!(repo.get_complete_graph(id).await.unwrap().0.is_empty());
    bad["nodes"][0]["fixed_principal_id"] = json!(outsider);
    assert_eq!(
        request(
            &client,
            &root,
            "PUT",
            &format!("{base}/draft"),
            &token,
            &key(),
            bad
        )
        .await
        .0,
        200
    );
    sqlx::query("UPDATE principals SET enabled=false WHERE principal_id=$1")
        .bind(outsider)
        .execute(&pool)
        .await
        .unwrap();
    let unknown = request(
        &client,
        &root,
        "POST",
        &format!("{base}/publish"),
        &token,
        &key(),
        json!({"versionId":id}),
    )
    .await;
    assert_eq!(unknown.0, 404, "{unknown:?}");
    assert_eq!(unknown.1["error"]["code"], "definition_not_found");
    server.abort();
    pool.close().await;
}
