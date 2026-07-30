//! HTTP-level integration tests for the submission history endpoint.
//!
//! Exercises only the thin adapter over `list_submission_history`. Authorization,
//! visibility, and pagination are asserted to match the reused application query.
//!
//! The fixture (`complete_query_instance`) is the generic Reviewer→RETURN
//! scenario (no real UUIDs): the creator submits work, the assignee returns it
//! via a RETURN transition carrying `relatedSubmissionIds`, and the instance is
//! later driven terminal — so every visibility tier is exercised on one dataset.

use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;
use uuid::Uuid;

use svc_workflow::application::provisioning::ProvisioningConfig;
use svc_workflow::auth::{AuthV1CanaryConfig, JwksConfig};
use svc_workflow::http::{self, AppState, HttpConfig};

use super::*;

/// Build the router with Auth V1 pointed at the mock JWKS server. Empty
/// allow-lists accept any authenticated principal so multiple actors can be
/// exercised against the same fixture in one test.
fn build_config(pool: &sqlx::PgPool, jwks_url: &str) -> axum::Router {
    let config = HttpConfig {
        bind_addr: "127.0.0.1:0".parse().unwrap(),
        request_body_max_bytes: 2_097_152,
        request_timeout_seconds: 30,
        jwks_config: JwksConfig {
            jwks_url: jwks_url.to_string(),
            issuer: "auth-service".to_string(),
            audience: "svc-workflow".to_string(),
            cache_ttl_secs: 30,
            http_timeout_secs: 5,
            max_stale_secs: 60,
            clock_skew_seconds: 0,
        },
        provisioning_config: ProvisioningConfig::new(Vec::new()),
        auth_v1_canary_config: AuthV1CanaryConfig {
            enabled: true,
            ..Default::default()
        },
    };
    http::router(AppState::new(pool.clone(), &config), &config)
}

fn request(method: &str, uri: &str, token: Option<&str>) -> Request<Body> {
    let mut builder = Request::builder().method(method).uri(uri);
    if let Some(tok) = token {
        builder = builder.header("authorization", format!("Bearer {tok}"));
    }
    builder.body(Body::empty()).unwrap()
}

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn submissions_uri(instance: Uuid) -> String {
    format!("/internal/v1/workflow-instances/{instance}/submissions")
}

/// Assert that two `Value` numbers are equal as i64 (avoids f64 vs int compare).
fn as_i64(v: &Value) -> i64 {
    v.as_i64().unwrap_or_else(|| panic!("expected integer, got {v}"))
}

// FULL_VISIBILITY_READS_ALL_SUBMISSIONS
#[tokio::test]
async fn full_visibility_reads_all_submissions() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    let token = v1_token(
        completed.seed.owner,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .oneshot(request("GET", &submissions_uri(completed.instance), Some(&token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["items"].as_array().unwrap();
    // The fixture produces 4 transitions (2 creator submits + RETURN + TERMINATE).
    assert_eq!(items.len(), 4);
    // A full viewer sees every transition effect including RETURN and TERMINATE.
    let effects: Vec<&str> = items.iter().map(|i| i["transition_effect"].as_str().unwrap()).collect();
    assert!(effects.contains(&"ADVANCE"));
    assert!(effects.contains(&"RETURN"));
    assert!(effects.contains(&"TERMINATE"));
    // next_cursor is null when all results fit in one page.
    assert!(body["next_cursor"].is_null());
}

// ACTOR_READS_OWN_SUBMISSION
#[tokio::test]
async fn actor_reads_own_submission() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    // The assignee/approver authored the RETURN and the TERMINATE submissions.
    let token = v1_token(
        completed.seed.assignee,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .oneshot(request("GET", &submissions_uri(completed.instance), Some(&token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["items"].as_array().unwrap();
    assert!(!items.is_empty());
    // The assignee is a historical participant (was an assignee), so it sees its
    // own authored submissions plus the RETURN it executed.
    assert!(items.iter().all(|i| {
        i["author_principal_id"] == completed.seed.assignee.to_string()
            || i["transition_effect"] == "RETURN"
    }));
}

// HISTORICAL_PARTICIPANT_READS_RELATED_RETURN
//
// Canonical brief scenario: the Reviewer is no longer the current assignee
// (instance terminated), yet must still read the RETURN submission payload
// referencing its earlier submission via relatedSubmissionIds — purely from
// being a historical participant, not from the assigned-to-me worklist.
#[tokio::test]
async fn historical_participant_reads_related_return_payload() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    // Sanity: the instance is terminal (current node is a TERMINAL node), so the
    // creator is NOT a current assignee — its access comes purely from being a
    // historical participant who authored a related submission.
    let current_node_type: String = sqlx::query_scalar(
        "SELECT n.node_type::text FROM workflow_instances i
         JOIN workflow_node_visits v ON v.node_visit_id = i.current_node_visit_id
         JOIN workflow_node_definitions n ON n.node_id = v.node_id
         WHERE i.workflow_instance_id = $1",
    )
    .bind(completed.instance)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(current_node_type, "TERMINAL");

    let token = v1_token(
        completed.seed.creator,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .oneshot(request("GET", &submissions_uri(completed.instance), Some(&token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["items"].as_array().unwrap();

    // The creator must see the RETURN feedback submission that references its
    // own earlier submission, even though it is no longer the assignee.
    let feedback = items
        .iter()
        .find(|i| i["submission_id"] == completed.feedback_submission.to_string())
        .unwrap_or_else(|| panic!("creator must see its related RETURN feedback"));
    assert_eq!(feedback["transition_effect"], "RETURN");
    // The relatedSubmissionIds in the RETURN payload must name the creator's own submission.
    let related: Vec<&str> = feedback["payload"]["relatedSubmissionIds"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(related.contains(&completed.creator_submission.to_string().as_str()));
    // And the creator sees its own authored ADVANCE submissions too.
    assert!(items
        .iter()
        .any(|i| i["submission_id"] == completed.creator_submission.to_string()));
}

// HISTORICAL_PARTICIPANT_CANNOT_READ_UNRELATED_SUBMISSION
//
// A different historical participant (the assignee) must not see submissions
// authored by someone else unless they are RETURN feedback referencing its own
// submissions. The creator's second ADVANCE submission must be invisible.
#[tokio::test]
async fn historical_participant_cannot_read_unrelated_submission() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    // The second creator ADVANCE submission is unrelated to the assignee.
    let second_creator_submission: Uuid = sqlx::query_scalar(
        "SELECT submission_id FROM workflow_submissions
         WHERE workflow_instance_id = $1 AND author_principal_id = $2
         ORDER BY created_at ASC LIMIT 1 OFFSET 1",
    )
    .bind(completed.instance)
    .bind(completed.seed.creator)
    .fetch_one(&pool)
    .await
    .unwrap();

    let token = v1_token(
        completed.seed.assignee,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .oneshot(request("GET", &submissions_uri(completed.instance), Some(&token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["items"].as_array().unwrap();
    let ids: Vec<String> = items
        .iter()
        .map(|i| i["submission_id"].as_str().unwrap().to_string())
        .collect();
    // The assignee did not author this submission and it is a plain ADVANCE
    // (not a RETURN referencing the assignee's own submission), so it is hidden.
    assert!(
        !ids.contains(&second_creator_submission.to_string()),
        "assignee must not see an unrelated creator ADVANCE submission; saw {ids:?}"
    );
}

// CROSS_DOMAIN_ACCESS_DENIED
//
// An outsider with no domain membership and no historical participation must
// not see the instance at all — 404 indistinguishable from a missing instance,
// failing closed without leaking existence.
#[tokio::test]
async fn cross_domain_access_denied_is_not_found() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    let token = v1_token(
        completed.seed.outsider,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .oneshot(request("GET", &submissions_uri(completed.instance), Some(&token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "workflow_instance_not_found_or_not_visible");
    // No submission data leaks.
    assert!(body.get("items").is_none());
}

// UNKNOWN_INSTANCE_NOT_VISIBLE
//
// A valid actor querying a random (non-existent) instance id must get the same
// 404 as a hidden instance — the response must not reveal existence.
#[tokio::test]
async fn unknown_instance_is_not_found_and_indistinguishable() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    let owner_token = v1_token(
        completed.seed.owner,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let unknown = Uuid::new_v4();
    let resp_unknown = app
        .clone()
        .oneshot(request("GET", &submissions_uri(unknown), Some(&owner_token)))
        .await
        .unwrap();

    let outsider_token = v1_token(
        completed.seed.outsider,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp_hidden = app
        .oneshot(request("GET", &submissions_uri(completed.instance), Some(&outsider_token)))
        .await
        .unwrap();

    // Both must be 404 with the identical opaque code and body shape.
    assert_eq!(resp_unknown.status(), StatusCode::NOT_FOUND);
    assert_eq!(resp_hidden.status(), StatusCode::NOT_FOUND);
    let body_unknown = json_body(resp_unknown).await;
    let body_hidden = json_body(resp_hidden).await;
    assert_eq!(body_unknown, body_hidden);
    assert_eq!(body_unknown["error"]["code"], "workflow_instance_not_found_or_not_visible");
}

// PAYLOAD_AND_DIGEST_RETURNED
//
// The HTTP response must serialize the existing SubmissionHistoryItem fields
// verbatim, including the original payload object and its digest string.
#[tokio::test]
async fn payload_and_digest_returned_verbatim() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    // Read the expected payload + digest directly from the store for comparison.
    let (stored_payload, stored_digest, stored_schema): (Value, String, String) =
        sqlx::query_as(
            "SELECT payload, payload_digest, schema_version FROM workflow_submissions
             WHERE submission_id = $1",
        )
        .bind(completed.creator_submission)
        .fetch_one(&pool)
        .await
        .unwrap();

    let token = v1_token(
        completed.seed.owner,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .oneshot(request("GET", &submissions_uri(completed.instance), Some(&token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = json_body(resp).await;
    let items = body["items"].as_array().unwrap();
    let item = items
        .iter()
        .find(|i| i["submission_id"] == completed.creator_submission.to_string())
        .unwrap();

    // Core fields preserved exactly.
    assert_eq!(item["submission_id"], completed.creator_submission.to_string());
    assert_eq!(item["workflow_instance_id"], completed.instance.to_string());
    assert_eq!(item["author_principal_id"], completed.seed.creator.to_string());
    assert_eq!(item["transition_effect"], "ADVANCE");
    // Payload object preserved verbatim.
    assert_eq!(item["payload"], stored_payload);
    assert!(item["payload"].get("work").is_some());
    // Digest preserved verbatim.
    assert_eq!(item["payload_digest"].as_str().unwrap(), stored_digest);
    assert!(!stored_digest.is_empty());
    // Schema version + other structural fields present.
    assert_eq!(item["schema_version"].as_str().unwrap(), stored_schema);
    assert!(item.get("created_at").is_some());
    assert!(item["source_node"].get("node_id").is_some());
    assert!(item.get("context_revision_id").is_some());
    assert!(item.get("transition_id").is_some());
    assert!(item.get("source_node_visit_id").is_some());
}

// PAGINATION_REUSES_EXISTING_QUERY_SEMANTICS
//
// The `after` cursor is the exact (created_at, id) keyset the application query
// already uses. Paginating with the returned next_cursor must produce no
// duplicates/gaps, and an invalid/half cursor must be rejected.
#[tokio::test]
async fn pagination_reuses_existing_query_semantics() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    let token = v1_token(
        completed.seed.owner,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let uri = format!("{}?limit=2", submissions_uri(completed.instance));
    let first = app
        .clone()
        .oneshot(request("GET", &uri, Some(&token)))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let first_body = json_body(first).await;
    let first_ids: Vec<String> = first_body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["submission_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(first_ids.len(), 2);
    // Ascending keyset order by (created_at, submission_id).
    assert!(first_body["next_cursor"].is_object());
    let next = first_body["next_cursor"].clone();

    // Build the next-page URI from the returned cursor fields exactly. The
    // cursor is the (created_at, id) keyset the application query already uses;
    // RFC 3339 `Z` timestamps and UUIDs are URL-safe so they are inserted raw.
    let after_created_at = next["created_at"].as_str().unwrap();
    let after_id = next["id"].as_str().unwrap();
    let uri2 = format!(
        "{}?afterCreatedAt={}&afterId={}&limit=2",
        submissions_uri(completed.instance),
        after_created_at,
        after_id
    );
    let second = app
        .clone()
        .oneshot(request("GET", &uri2, Some(&token)))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::OK);
    let second_body = json_body(second).await;
    let second_ids: Vec<String> = second_body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["submission_id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(second_ids.len(), 2);
    // No overlap between pages.
    assert!(first_ids.iter().all(|id| !second_ids.contains(id)));

    // Union of both pages equals the full set.
    let mut union: Vec<String> = first_ids.into_iter().chain(second_ids.into_iter()).collect();
    union.sort();
    let expected_count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM workflow_submissions WHERE workflow_instance_id = $1",
    )
    .bind(completed.instance)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(union.len(), as_i64(&json!(expected_count)) as usize);

    // Invalid limit (0) is rejected by the reused query rules -> 422.
    let bad = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("{}?limit=0", submissions_uri(completed.instance)),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(bad.status(), StatusCode::UNPROCESSABLE_ENTITY);

    // A half-present cursor (afterCreatedAt without afterId) is rejected -> 422.
    let half = app
        .oneshot(request(
            "GET",
            &format!(
                "{}?afterCreatedAt={}",
                submissions_uri(completed.instance),
                after_created_at
            ),
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(half.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let half_body = json_body(half).await;
    assert_eq!(half_body["error"]["code"], "invalid_cursor");
}

// HTTP adapter-level guards (auth/scope/routing): show the adapter enforces only
// scope + path and otherwise stays out of the way.
#[tokio::test]
async fn missing_token_is_unauthorized() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    let resp = app
        .oneshot(request("GET", &submissions_uri(completed.instance), None))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
}

#[tokio::test]
async fn missing_scope_is_forbidden() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    let token = v1_token(
        completed.seed.owner,
        "some.other.scope",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .oneshot(request("GET", &submissions_uri(completed.instance), Some(&token)))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn invalid_instance_path_is_bad_request() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    let token = v1_token(
        completed.seed.owner,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .oneshot(request(
            "GET",
            "/internal/v1/workflow-instances/not-a-uuid/submissions",
            Some(&token),
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "invalid_path_parameter");
}

#[tokio::test]
async fn unknown_query_field_is_rejected() {
    let pool = create_pool().await;
    let mock = MockJwksServer::start().await;
    let app = build_config(&pool, &mock.url);
    let completed = complete_query_instance(&pool).await;

    let token = v1_token(
        completed.seed.owner,
        "workflow.read",
        "test-client",
        300,
        &mock.key_pair,
    );
    let resp = app
        .oneshot(request(
            "GET",
            &format!("{}?actorPrincipalId={}", submissions_uri(completed.instance), completed.seed.owner),
            Some(&token),
        ))
        .await
        .unwrap();
    // deny_unknown_fields rejects an actor override at the query layer.
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = json_body(resp).await;
    assert_eq!(body["error"]["code"], "invalid_pagination");
}
