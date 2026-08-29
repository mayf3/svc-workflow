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
use sqlx::{Connection, Executor, PgConnection};

use super::server::RunningServer;
use super::*;

/// Run-scoped isolated E2E database owned by this module.
///
/// The shared `database::TemporaryDatabase` names its databases with the
/// `svc_workflow_e2e_` prefix and its residue assertion counts EVERY
/// database under that prefix. The default test harness runs tests in
/// parallel, so when two E2E tests overlap, whichever finishes first counts
/// the sibling test's still-live database as residue and fails spuriously.
///
/// This helper is run-scoped instead:
///
/// - its databases use a prefix owned by this module (`svc_workflow_ret422_`)
///   which the scenario test's prefix-scoped residue assertion cannot see —
///   and whose residue assertion here cannot see the scenario test's
///   databases either;
/// - `cleanup` asserts that exactly the database THIS run created is gone,
///   so a genuine failure to drop this run's own database still fails loud,
///   while a concurrently running foreign E2E database is preserved.
struct RunScopedDatabase {
    pool: PgPool,
    name: String,
}

impl RunScopedDatabase {
    async fn create() -> Self {
        let name = format!("svc_workflow_ret422_{}", Uuid::new_v4().simple());
        let mut admin = PgConnection::connect(&crate::common::admin_database_url())
            .await
            .expect("connect to PostgreSQL administration database");
        admin
            .execute(format!("CREATE DATABASE {name}").as_str())
            .await
            .expect("create run-scoped E2E database");
        match Self::initialize(&name).await {
            Ok(pool) => Self { pool, name },
            Err(error) => {
                Self::drop_named(&name)
                    .await
                    .expect("drop run-scoped E2E database after setup failure");
                panic!("initialize run-scoped E2E database: {error}");
            }
        }
    }

    /// Drop this run's own database, then fail loud if that exact database
    /// is still present (run-scoped residue assertion: only the name created
    /// by this test instance is judged — never a foreign database).
    async fn cleanup(self) {
        self.pool.close().await;
        Self::drop_named(&self.name)
            .await
            .expect("drop run-scoped E2E database");
        let mut admin = PgConnection::connect(&crate::common::admin_database_url())
            .await
            .expect("connect for run-scoped E2E residue check");
        let remaining: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pg_database WHERE datname = $1")
                .bind(&self.name)
                .fetch_one(&mut admin)
                .await
                .expect("query run-scoped E2E database residue");
        assert_eq!(
            remaining, 0,
            "run-scoped E2E database {} created by this test must not remain",
            self.name
        );
    }

    async fn initialize(name: &str) -> Result<PgPool, String> {
        let url = format!("{}/{}", crate::common::test_database_base(), name);
        let pool = PgPool::connect(&url)
            .await
            .map_err(|error| format!("connect: {error}"))?;
        let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
            .await
            .map_err(|error| format!("load migrations: {error}"))?;
        migrator
            .run(&pool)
            .await
            .map_err(|error| format!("run migrations: {error}"))?;
        Ok(pool)
    }

    async fn drop_named(name: &str) -> Result<(), String> {
        let mut admin = PgConnection::connect(&crate::common::admin_database_url())
            .await
            .map_err(|error| format!("connect for drop: {error}"))?;
        admin
            .execute(format!("DROP DATABASE {name} WITH (FORCE)").as_str())
            .await
            .map_err(|error| format!("drop database: {error}"))?;
        Ok(())
    }
}

async fn json_response(response: reqwest::Response) -> (u16, Value) {
    let status = response.status().as_u16();
    let body = response.json().await.expect("JSON response envelope");
    (status, body)
}

#[tokio::test]
async fn return_422_exposes_aggregated_contract_detail_over_real_tcp() {
    let database = RunScopedDatabase::create().await;
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
    // Run-scoped teardown: cleanup() drops this test's own database and then
    // asserts that exact name is gone (fail-loud on this run's own residue).
    // A concurrently running E2E test (scenario) holding its own isolated
    // database is neither visible to that assertion nor misjudged by it,
    // and this test's database is likewise invisible to the scenario
    // test's prefix-scoped residue assertion.
    database.cleanup().await;
}
