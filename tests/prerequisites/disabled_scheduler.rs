use super::*;
#[tokio::test]
async fn disabled_scheduler_with_valid_jwt_and_enabled_binding_is_rejected() {
    let p = pool().await;
    let f = setup(&p).await;
    let (v, _) = version(&p, &f, false, false).await;
    create(&p, &f, v, json!({})).await;
    sqlx::query("INSERT INTO global_role_bindings(binding_id,principal_id,role_key,enabled) VALUES($1,$2,'GLOBAL_SCHEDULER_READ',true)").bind(Uuid::new_v4()).bind(f.coordinator).execute(&p).await.unwrap();
    let m = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &m).await;
    assert_eq!(
        http(
            router.clone(),
            f.coordinator,
            &m,
            "GET",
            "/internal/v1/dispatch-intents/ready",
            None,
            json!({})
        )
        .await
        .0,
        200
    );
    assert_eq!(
        http(
            router.clone(),
            f.owner,
            &m,
            "GET",
            "/internal/v1/dispatch-intents/ready",
            None,
            json!({})
        )
        .await
        .0,
        403
    );
    sqlx::query("UPDATE principals SET enabled=false WHERE principal_id=$1")
        .bind(f.coordinator)
        .execute(&p)
        .await
        .unwrap();
    let (s, b) = http(
        router,
        f.coordinator,
        &m,
        "GET",
        "/internal/v1/dispatch-intents/ready",
        None,
        json!({}),
    )
    .await;
    assert_eq!(s, 403, "{b}");
    assert!(b.get("items").is_none());
    assert!(matches!(svc_workflow::store::postgres::workflow_instance_repository::query_ready_intents::list(&p,f.coordinator,50,None).await,Err(svc_workflow::application::workflow_instance::query_types::WorkflowQueryError::PrincipalDisabled)));
}
