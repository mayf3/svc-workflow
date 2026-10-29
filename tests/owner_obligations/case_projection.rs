use super::*;

#[tokio::test]
async fn case_owner_detail_projects_the_original_responsibility() {
    let p = pool().await;
    let (owner, _, pm, managed, _, _, source) = operation_fixture(&p).await;
    let case: Uuid = sqlx::query_scalar(
        "SELECT assistance_case_id FROM workflow_assistance_cases WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let jwk = common::MockJwksServer::start().await;
    let app = http_support::app(&p, &jwk, pm).await;
    let (status, body) = http_support::http(
        app,
        owner,
        &jwk,
        "GET",
        &format!("/internal/v1/assistance-cases/{case}"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 200, "{body}");
    assert_eq!(body["ownerObligation"]["version"], 1, "{body}");
    assert_eq!(
        body["ownerObligation"]["responsibilities"][0]["managedWorkflowInstanceId"],
        json!(managed)
    );
    println!("actual Axum Owner detail projection: {body}");
}

#[tokio::test]
async fn case_prebirth_stop_current_owner_privacy_and_persisted_unknown_vs_outage() {
    let p = pool().await;
    let (owner, domain, pm, source, _, _) = policy_fixture(&p).await;
    publish(&p, owner, source, Some(pm), "source")
        .await
        .unwrap();
    let s = create(
        &p,
        owner,
        domain,
        source,
        json!({"ownerIntakeContext":{},"ownerReviewContext":{}}),
    )
    .await
    .unwrap();
    let requested = request(&p, owner, s.workflow_instance_id, s.current_node_visit_id).await;
    let case: Uuid = sqlx::query_scalar(
        "SELECT assistance_case_id FROM workflow_assistance_cases WHERE workflow_instance_id=$1",
    )
    .bind(s.workflow_instance_id)
    .fetch_one(&p)
    .await
    .unwrap();
    let intent: Uuid = sqlx::query_scalar(
        "SELECT intent_id FROM workflow_owner_obligation_intents WHERE case_id=$1",
    )
    .bind(case)
    .fetch_one(&p)
    .await
    .unwrap();
    let coordinator = actor(&p).await;
    let other = actor(&p).await;
    let outsider = actor(&p).await;
    sqlx::query("INSERT INTO global_role_bindings(binding_id,principal_id,role_key) VALUES($1,$2,'GLOBAL_WORKFLOW_COORDINATOR')").bind(Uuid::new_v4()).bind(coordinator).execute(&p).await.unwrap();
    use svc_workflow::domain::{ids::*, provisioning::ReplaceOwnerCommand};
    svc_workflow::application::provisioning::replace_owner(
        &p,
        &ReplaceOwnerCommand {
            domain_id: DomainId::from_uuid(domain),
            new_owner_id: PrincipalId::from_uuid(other),
        },
        "replacement",
        "test",
        &PrincipalId::from_uuid(coordinator),
    )
    .await
    .unwrap();
    let jwk = common::MockJwksServer::start().await;
    let app = http_support::app(&p, &jwk, pm).await;
    let path = format!("/internal/v1/assistance-cases/{case}");
    let (status, detail) =
        http_support::http(app.clone(), other, &jwk, "GET", &path, None, json!({})).await;
    assert_eq!(status, 200, "{detail}");
    let r = &detail["ownerObligation"]["responsibilities"][0];
    assert_eq!(r["responsibilityId"], json!(intent));
    assert_eq!(r["state"], "STOPPED");
    assert_eq!(r["stop"]["responsibilityId"], json!(intent));
    assert_eq!(r["stop"]["code"], "OWNER_CHANGED_UNSUPPORTED");
    assert!(r.get("managedWorkflowInstanceId").is_none());
    for field in ["nodeVisitId", "attemptId", "effectId"] {
        assert!(r["stop"].get(field).is_none());
    }
    assert!(r["stop"]["ownerReplacementCommandId"].is_string());
    let (status, inbox) = http_support::http(
        app.clone(),
        other,
        &jwk,
        "GET",
        "/internal/v1/assistance-cases/owner-inbox",
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 200, "{inbox}");
    assert_eq!(
        inbox["items"][0]["ownerObligation"],
        detail["ownerObligation"]
    );
    let (status, old_detail) =
        http_support::http(app.clone(), owner, &jwk, "GET", &path, None, json!({})).await;
    assert_eq!(status, 200, "{old_detail}");
    assert!(old_detail.get("ownerObligation").is_none());
    let (status, requested_by) = http_support::http(
        app.clone(),
        owner,
        &jwk,
        "GET",
        "/internal/v1/assistance-cases/requested-by-me",
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 200, "{requested_by}");
    assert!(requested_by["items"][0].get("ownerObligation").is_none());
    let (status, _) =
        http_support::http(app.clone(), outsider, &jwk, "GET", &path, None, json!({})).await;
    assert_eq!(status, 403);
    let (status, coordinator_detail) = http_support::http(
        app.clone(),
        coordinator,
        &jwk,
        "GET",
        &path,
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 404, "{coordinator_detail}");
    sqlx::query("UPDATE principals SET enabled=FALSE WHERE principal_id=$1")
        .bind(other)
        .execute(&p)
        .await
        .unwrap();
    let (status, _) =
        http_support::http(app.clone(), other, &jwk, "GET", &path, None, json!({})).await;
    assert_eq!(status, 403);
    sqlx::query("UPDATE principals SET enabled=TRUE WHERE principal_id=$1")
        .bind(other)
        .execute(&p)
        .await
        .unwrap();
    // Corrupt only this fresh synthetic DB to prove absence cannot become UNKNOWN.
    sqlx::query(
        "ALTER TABLE workflow_owner_obligation_intents DISABLE TRIGGER owner_intent_immutable",
    )
    .execute(&p)
    .await
    .unwrap();
    sqlx::query("DELETE FROM workflow_owner_obligation_intents WHERE intent_id=$1")
        .bind(intent)
        .execute(&p)
        .await
        .unwrap();
    let (status, error) = http_support::http(app, other, &jwk, "GET", &path, None, json!({})).await;
    assert_eq!(status, 503, "{error}");
    assert_eq!(error["error"]["code"], "service_unavailable");
    println!(
        "actual Axum prebirth STOP: {}; missing expected lineage: {}",
        detail, error
    );
    let _ = requested;
}

#[tokio::test]
async fn case_durable_unknown_is_not_storage_failure_and_ordinary_case_omits_field() {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    let case: Uuid = sqlx::query_scalar(
        "SELECT assistance_case_id FROM workflow_assistance_cases WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    sqlx::query("UPDATE workflow_owner_obligation_intents SET state='UNKNOWN',outcome='OUTCOME_UNKNOWN' WHERE managed_instance_id=$1 AND kind='INTAKE'").bind(managed).execute(&p).await.unwrap();
    let jwk = common::MockJwksServer::start().await;
    let app = http_support::app(&p, &jwk, pm).await;
    let (status, unknown) = http_support::http(
        app.clone(),
        owner,
        &jwk,
        "GET",
        &format!("/internal/v1/assistance-cases/{case}"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 200, "{unknown}");
    assert_eq!(
        unknown["ownerObligation"]["responsibilities"][0]["state"],
        "UNKNOWN"
    );
    let ordinary = create(&p, owner, domain, supplier, json!({}))
        .await
        .unwrap();
    request(
        &p,
        owner,
        ordinary.workflow_instance_id,
        ordinary.current_node_visit_id,
    )
    .await;
    let ordinary_case: Uuid = sqlx::query_scalar(
        "SELECT assistance_case_id FROM workflow_assistance_cases WHERE workflow_instance_id=$1",
    )
    .bind(ordinary.workflow_instance_id)
    .fetch_one(&p)
    .await
    .unwrap();
    let (status, plain) = http_support::http(
        app.clone(),
        owner,
        &jwk,
        "GET",
        &format!("/internal/v1/assistance-cases/{ordinary_case}"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 200, "{plain}");
    assert!(plain.get("ownerObligation").is_none());
    sqlx::query(
        "ALTER TABLE workflow_owner_obligation_intents RENAME TO isolated_unavailable_intents",
    )
    .execute(&p)
    .await
    .unwrap();
    let (status, error) = http_support::http(
        app,
        owner,
        &jwk,
        "GET",
        &format!("/internal/v1/assistance-cases/{case}"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 503, "{error}");
    assert_eq!(error["error"]["code"], "service_unavailable");
}
