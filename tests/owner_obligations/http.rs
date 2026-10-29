use super::*;

#[tokio::test]
async fn http_actual_managed_effect_dto_jwt_and_visibility() {
    let p = pool().await;
    let (owner, domain, pm, managed, supplier, _, source) = operation_fixture(&p).await;
    let jwk = common::MockJwksServer::start().await;
    let app = http_support::app(&p, &jwk, pm).await;
    let (status, m) = http_support::http(
        app.clone(),
        owner,
        &jwk,
        "GET",
        &format!("/internal/v1/workflow-instances/{managed}/owner-obligation"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 200, "{m}");
    let source_version: i32 = sqlx::query_scalar(
        "SELECT workflow_state_version FROM workflow_instances WHERE workflow_instance_id=$1",
    )
    .bind(source)
    .fetch_one(&p)
    .await
    .unwrap();
    let visit = m["currentNode"]["nodeVisitId"].as_str().unwrap();
    let url=format!("/internal/v1/workflow-instances/{managed}/node-visits/{visit}/operation-effects/create-dependency:execute");
    let body = json!({"expectedWorkflowStateVersion":1,"expectedSourceWorkflowStateVersion":source_version,"operation":m["currentNode"]["operation"]["allowedOperations"][0],"arguments":{"domainId":domain,"definitionVersionId":supplier,"contextPayload":{},"metadata":null}});
    let (status, result) = http_support::http(
        app.clone(),
        owner,
        &jwk,
        "POST",
        &url,
        Some("real-http-effect"),
        body.clone(),
    )
    .await;
    assert_eq!(status, 200, "{result}");
    assert_eq!(result.as_object().unwrap().len(), 6);
    assert_eq!(result["state"], "SUCCEEDED");
    println!("actual Axum managed GET: {m}\nactual Axum effect POST: {result}");
    let effect = result["effectId"].as_str().unwrap();
    let (status, read) = http_support::http(
        app.clone(),
        owner,
        &jwk,
        "GET",
        &format!("/internal/v1/workflow-instances/{managed}/operation-effects/{effect}"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 200, "{read}");
    assert_eq!(read["result"], result["result"]);
    let outsider = actor(&p).await;
    let (status, _) = http_support::http(
        app.clone(),
        outsider,
        &jwk,
        "GET",
        &format!("/internal/v1/workflow-instances/{managed}/owner-obligation"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 404);
    let mut forged = body;
    forged["actorPrincipalId"] = json!(owner);
    let (status, _) = http_support::http(
        app.clone(),
        owner,
        &jwk,
        "POST",
        &url,
        Some("forged"),
        forged,
    )
    .await;
    assert_eq!(status, 400);
    sqlx::query("UPDATE principals SET enabled=FALSE WHERE principal_id=$1")
        .bind(owner)
        .execute(&p)
        .await
        .unwrap();
    let (status, _) = http_support::http(
        app,
        owner,
        &jwk,
        "GET",
        &format!("/internal/v1/workflow-instances/{managed}/owner-obligation"),
        None,
        json!({}),
    )
    .await;
    assert_eq!(status, 404);
}
#[tokio::test]
async fn http_publication_owner_approval_replay_and_no_pm_ordinary_write_delegation() {
    let p = pool().await;
    let (owner, domain, pm, source, _, _) = policy_fixture(&p).await;
    let definition:Uuid=sqlx::query_scalar("SELECT workflow_definition_id FROM workflow_definition_versions WHERE definition_version_id=$1").bind(source).fetch_one(&p).await.unwrap();
    let jwk = common::MockJwksServer::start().await;
    let app = http_support::app(&p, &jwk, pm).await;
    let path = format!("/internal/v1/domains/{domain}/definitions/{definition}/publish");
    let outsider = actor(&p).await;
    let (status, denied) = http_support::http(
        app.clone(),
        outsider,
        &jwk,
        "POST",
        &path,
        Some("not-owner"),
        json!({"versionId":source}),
    )
    .await;
    assert_eq!(status, 404, "{denied}");
    assert_eq!(denied["error"]["code"], "definition_not_found");
    let (status, result) = http_support::http(
        app.clone(),
        owner,
        &jwk,
        "POST",
        &path,
        Some("http-owner-publish"),
        json!({"versionId":source}),
    )
    .await;
    assert_eq!(status, 200, "{result}");
    let (status, replay) = http_support::http(
        app.clone(),
        owner,
        &jwk,
        "POST",
        &path,
        Some("http-owner-publish"),
        json!({"versionId":source}),
    )
    .await;
    assert_eq!(status, 200, "{replay}");
    assert_eq!(result, replay);
    let (v, _) = version(&p, domain, owner, json!({"type":"object"})).await;
    publish(&p, owner, v, None, "ordinary-version")
        .await
        .unwrap();
    let (status, denied) = http_support::http(
        app,
        pm,
        &jwk,
        "POST",
        "/internal/v1/workflow-instances",
        Some("pm-ordinary-bypass"),
        json!({"domainId":domain,"definitionVersionId":v,"contextPayload":{},"metadata":{}}),
    )
    .await;
    assert_eq!(status, 403, "{denied}");
    let row:(Uuid,Uuid,i64)=sqlx::query_as("SELECT approved_by_principal_id,configured_policy_actor_principal_id,(SELECT count(*) FROM workflow_events) FROM workflow_owner_policy_approvals WHERE source_definition_version_id=$1").bind(source).fetch_one(&p).await.unwrap();
    assert_eq!(row, (owner, pm, 0));
    println!("actual Axum policy publish: {result}; opaque denied actor: {denied}");
}
#[tokio::test]
async fn http_loaded_policy_actor_mismatch_refuses_admission_then_actual_producer_stops() {
    let p = pool().await;
    let (owner, _, _, managed, _, _, _) = operation_fixture(&p).await;
    let wrong = actor(&p).await;
    let jwk = common::MockJwksServer::start().await;
    let app = http_support::app(&p, &jwk, wrong).await;
    let path = format!("/internal/v1/workflow-instances/{managed}/owner-obligation");
    let (status, error) =
        http_support::http(app.clone(), owner, &jwk, "GET", &path, None, json!({})).await;
    assert_eq!(status, 409, "{error}");
    assert_eq!(error["error"]["code"], "policy_actor_mismatch");
    svc_workflow::store::postgres::workflow_instance_repository::owner_obligation::reconcile::reconcile_configured(&p,Some(wrong),svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled()).await.unwrap();
    let (status, stopped) =
        http_support::http(app, owner, &jwk, "GET", &path, None, json!({})).await;
    assert_eq!(status, 200, "{stopped}");
    assert_eq!(stopped["state"], "STOPPED");
    assert_eq!(stopped["stop"]["code"], "POLICY_ACTOR_MISMATCH");
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM workflow_operation_effects")
        .fetch_one(&p)
        .await
        .unwrap();
    assert_eq!(count, 0);
}
