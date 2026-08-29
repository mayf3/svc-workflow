//! Real-TCP E2E for the RETURN 422 detail-exposure fix.
//!
//! Reproduces the incident shape over a real HTTP listener (equivalent to the
//! curl invocation in the DEV_SELF_CHECK report): a definition whose RETURN
//! submission_schema only declares `summary` — schema validation passes, then
//! the engine-level RETURN contract check rejects the payload. The response
//! must keep code `invalid_return_references` AND surface the aggregated
//! contract detail (previously swallowed).

use reqwest::Client;
use serde_json::{json, Value};

use super::database::TemporaryDatabase;
use super::server::RunningServer;
use super::*;

async fn json_response(response: reqwest::Response) -> (u16, Value) {
    let status = response.status().as_u16();
    let body = response.json().await.expect("JSON response envelope");
    (status, body)
}

#[tokio::test]
async fn return_422_exposes_aggregated_contract_detail_over_real_tcp() {
    let database = TemporaryDatabase::create().await;
    let pool = database.pool.clone();

    let (principal_id, domain_id) = seed_principal_domain_with_owner(&pool).await;
    // Incident-shaped definition: RETURN schema declares only `summary`.
    let (_, ver_id, _, _, _, draft_adv, _, ret_id, _) = seed_transition_graph_with_return_schema(
        &pool,
        domain_id,
        "WORKFLOW_CREATOR",
        "WORKFLOW_CREATOR",
        None,
        json!({
            "type": "object",
            "required": ["summary"],
            "properties": { "summary": { "type": "string" } }
        }),
    )
    .await;

    let server =
        RunningServer::start(pool.clone(), 2_097_152, &principal_id.to_string()).await;
    let client = Client::new();
    let token = common::v1_token(
        principal_id,
        "workflow.execute workflow.read",
        "e2e-client",
        300,
        &server.key_pair,
    );
    let base = server.base_url.clone();

    // Create the instance over HTTP (draft node).
    let (create_status, created) = json_response(
        client
            .post(format!("{base}/internal/v1/workflow-instances"))
            .bearer_auth(&token)
            .header("idempotency-key", format!("ret422-create-{}", Uuid::new_v4()))
            .json(&json!({
                "domainId": domain_id,
                "definitionVersionId": ver_id,
                "metadata": {"source": "ret422-e2e"},
                "contextPayload": {"title": "ret422"}
            }))
            .send()
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(create_status, 201, "create: {created}");
    let instance_id = Uuid::parse_str(created["workflowInstanceId"].as_str().unwrap()).unwrap();

    // Advance DRAFT → NORMAL (state v1 → v2).
    let (adv_status, advanced) = json_response(
        client
            .post(format!(
                "{base}/internal/v1/workflow-instances/{instance_id}/transitions"
            ))
            .bearer_auth(&token)
            .header("idempotency-key", format!("ret422-adv-{}", Uuid::new_v4()))
            .json(&json!({
                "transitionDefinitionId": draft_adv,
                "expectedWorkflowStateVersion": 1,
                "submissionPayload": null
            }))
            .send()
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(adv_status, 200, "advance: {advanced}");
    let source_visit_id =
        Uuid::parse_str(advanced["currentNodeVisitId"].as_str().unwrap()).unwrap();

    // Execute RETURN with only `summary` — schema passes, engine contract fails.
    let (ret_status, ret_body) = json_response(
        client
            .post(format!(
                "{base}/internal/v1/workflow-instances/{instance_id}/transitions"
            ))
            .bearer_auth(&token)
            .header("idempotency-key", format!("ret422-ret-{}", Uuid::new_v4()))
            .json(&json!({
                "transitionDefinitionId": ret_id,
                "expectedWorkflowStateVersion": 2,
                "submissionPayload": {"summary": "looks fine to me"}
            }))
            .send()
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(ret_status, 422, "return must be 422: {ret_body}");
    assert_eq!(
        ret_body["error"]["code"],
        "invalid_return_references",
        "code must stay stable: {ret_body}"
    );
    let detail = ret_body["error"]["details"]["detail"]
        .as_str()
        .expect("detail must be exposed");
    assert!(
        detail.contains("rootCauseNodeVisitId is required"),
        "detail must mention rootCauseNodeVisitId: {detail}"
    );
    assert!(
        detail.contains("reasonCode is required") && detail.contains("reason is required"),
        "detail must aggregate all missing fields: {detail}"
    );

    // Positive control: complete RETURN succeeds (root cause = upstream visit).
    let (ok_status, ok_body) = json_response(
        client
            .post(format!(
                "{base}/internal/v1/workflow-instances/{instance_id}/transitions"
            ))
            .bearer_auth(&token)
            .header("idempotency-key", format!("ret422-ok-{}", Uuid::new_v4()))
            .json(&json!({
                "transitionDefinitionId": ret_id,
                "expectedWorkflowStateVersion": 2,
                "submissionPayload": {
                    "summary": "revision needed",
                    "rootCauseNodeVisitId": source_visit_id,
                    "reasonCode": "NEEDS_REVISION",
                    "reason": "spec gap",
                    "relatedSubmissionIds": []
                }
            }))
            .send()
            .await
            .unwrap(),
    )
    .await;
    assert_eq!(ok_status, 200, "valid RETURN must succeed: {ok_body}");

    server.stop().await.expect("E2E server shutdown");
    database.cleanup().await;
    // NOTE: do not call TemporaryDatabase::assert_no_residue() here — E2E
    // tests run in parallel and the long-running scenario test may still hold
    // its temporary database when this test finishes, which would trip the
    // global residue assertion. Cleanup of this test's database is performed
    // by database.cleanup() above; the residue assertion remains owned by the
    // scenario test (the last E2E test to finish).
}
