use super::*;

#[tokio::test]
async fn domain_key_conflict() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    seed_provisioning_actor(&pool).await;
    let app = build_app(
        pool.clone(),
        &mock.url,
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let conflict_key = unique_domain_key("conflict-key");
    let first = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/domains",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&unique_key("domain-key-first")),
            Some(json!({
                "domainId": Uuid::new_v4(),
                "domainKey": conflict_key,
                "enabled": true
            })),
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);

    let second = app
        .oneshot(request(
            "POST",
            "/internal/v1/admin/domains",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&unique_key("domain-key-second")),
            Some(json!({
                "domainId": Uuid::new_v4(),
                "domainKey": conflict_key,
                "enabled": true
            })),
        ))
        .await
        .unwrap();
    assert_eq!(second.status(), StatusCode::CONFLICT);
    assert_eq!(
        json_body(second).await["error"]["code"],
        "domain_identity_conflict"
    );
}

#[tokio::test]
async fn role_binding_create_and_replay() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let principal_id = Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap();
    let domain_id = Uuid::new_v4();
    seed_provisioning_actor(&pool).await;
    let app = build_app(
        pool,
        &mock.url,
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    app.clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/domains",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&unique_key("binding-domain")),
            Some(json!({
                "domainId": domain_id,
                "domainKey": unique_domain_key("binding-domain"),
                "enabled": true
            })),
        ))
        .await
        .unwrap();

    let key = unique_key("binding-replay");
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(request(
                "PUT",
                &format!("/internal/v1/admin/domains/{domain_id}/role-bindings/{principal_id}"),
                Some(&provisioning_token(&mock.key_pair)),
                Some(&key),
                Some(json!({"roleKey": "DOMAIN_OWNER", "enabled": true})),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}

#[tokio::test]
async fn role_binding_unknown_principal_failure_is_replayed() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let domain_id = Uuid::new_v4();
    seed_provisioning_actor(&pool).await;
    let app = build_app(
        pool,
        &mock.url,
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let unknown_principal_id = Uuid::new_v4();
    app.clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/domains",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&unique_key("unknown-domain")),
            Some(json!({
                "domainId": domain_id,
                "domainKey": unique_domain_key("unknown-domain"),
                "enabled": true
            })),
        ))
        .await
        .unwrap();

    let key = unique_key("unknown-principal");
    for _ in 0..2 {
        let response = app
            .clone()
            .oneshot(request(
                "PUT",
                &format!(
                    "/internal/v1/admin/domains/{domain_id}/role-bindings/{}",
                    unknown_principal_id
                ),
                Some(&provisioning_token(&mock.key_pair)),
                Some(&key),
                Some(json!({"roleKey": "DOMAIN_OWNER", "enabled": true})),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn principal_hash_covers_source_and_revision() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    seed_provisioning_actor(&pool).await;
    let app = build_app(
        pool,
        &mock.url,
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let principal_id = Uuid::new_v4();
    let key = unique_key("principal-full-hash");
    let first = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&key),
            Some(json!({
                "principalId": principal_id,
                "principalType": "agent",
                "enabled": true,
                "source": "auth-service",
                "sourceRevision": "revision-a"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let changed = app
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&key),
            Some(json!({
                "principalId": principal_id,
                "principalType": "agent",
                "enabled": true,
                "source": "auth-service-v2",
                "sourceRevision": "revision-b"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(changed.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn domain_hash_covers_display_name() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    seed_provisioning_actor(&pool).await;
    let app = build_app(
        pool.clone(),
        &mock.url,
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let domain_id = Uuid::new_v4();
    let key = unique_key("domain-full-hash");
    let first = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/domains",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&key),
            Some(json!({
                "domainId": domain_id,
                "domainKey": unique_domain_key("hash-domain"),
                "displayName": "Display A",
                "enabled": true
            })),
        ))
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    let original_key =
        sqlx::query_scalar::<_, String>("SELECT domain_key FROM domains WHERE domain_id = $1")
            .bind(domain_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let changed = app
        .oneshot(request(
            "POST",
            "/internal/v1/admin/domains",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&key),
            Some(json!({
                "domainId": domain_id,
                "domainKey": original_key,
                "displayName": "Display B",
                "enabled": true
            })),
        ))
        .await
        .unwrap();
    assert_eq!(changed.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn allowlisted_actor_bootstraps_itself_then_disabled_actor_is_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let actor_id = Uuid::new_v4();
    let actor_token_str = common::v1_token(
        actor_id,
        "workflow.admin",
        "prov-client",
        300,
        &mock.key_pair,
    );
    let app = build_app(pool.clone(), &mock.url, actor_id);
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let bootstrap = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&actor_token_str),
            Some(&unique_key("bootstrap")),
            Some(json!({
                "principalId": actor_id,
                "principalType": "agent",
                "enabled": true,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(bootstrap.status(), StatusCode::OK);

    let disable = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&actor_token_str),
            Some(&unique_key("self-disable")),
            Some(json!({
                "principalId": actor_id,
                "principalType": "agent",
                "enabled": false,
                "source": "auth-service"
            })),
        ))
        .await
        .unwrap();
    assert_eq!(disable.status(), StatusCode::OK);

    let rejected = app
        .oneshot(request(
            "POST",
            "/internal/v1/admin/domains",
            Some(&actor_token_str),
            Some(&unique_key("disabled-write")),
            Some(json!({
                "domainId": Uuid::new_v4(),
                "domainKey": unique_domain_key("disabled-write"),
                "enabled": true
            })),
        ))
        .await
        .unwrap();
    assert_eq!(rejected.status(), StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn unknown_actor_field_and_invalid_role_are_rejected() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    seed_provisioning_actor(&pool).await;
    let app = build_app(
        pool,
        &mock.url,
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let unknown = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/principals",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&unique_key("unknown-field")),
            Some(json!({
                "principalId": Uuid::new_v4(),
                "principalType": "agent",
                "enabled": true,
                "source": "auth-service",
                "actorPrincipalId": Uuid::new_v4()
            })),
        ))
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);

    let invalid_role = app
        .oneshot(request(
            "PUT",
            &format!(
                "/internal/v1/admin/domains/{}/role-bindings/{}",
                Uuid::new_v4(),
                Uuid::new_v4()
            ),
            Some(&provisioning_token(&mock.key_pair)),
            Some(&unique_key("invalid-role")),
            Some(json!({"roleKey": "FUTURE_SUPERUSER", "enabled": true})),
        ))
        .await
        .unwrap();
    assert_eq!(invalid_role.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn concurrent_owner_replacements_are_serialized() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    seed_provisioning_actor(&pool).await;
    let app = build_app(
        pool.clone(),
        &mock.url,
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let domain_id = Uuid::new_v4();
    let owner_a = Uuid::new_v4();
    let owner_b = Uuid::new_v4();
    for owner in [owner_a, owner_b] {
        let response = app
            .clone()
            .oneshot(request(
                "POST",
                "/internal/v1/admin/principals",
                Some(&provisioning_token(&mock.key_pair)),
                Some(&unique_key("owner-principal")),
                Some(json!({
                    "principalId": owner,
                    "principalType": "agent",
                    "enabled": true,
                    "source": "auth-service"
                })),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
    let domain = app
        .clone()
        .oneshot(request(
            "POST",
            "/internal/v1/admin/domains",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&unique_key("owner-domain")),
            Some(json!({
                "domainId": domain_id,
                "domainKey": unique_domain_key("owner-domain"),
                "enabled": true
            })),
        ))
        .await
        .unwrap();
    assert_eq!(domain.status(), StatusCode::OK);

    let replace = |owner, key: String| {
        app.clone().oneshot(request(
            "PUT",
            &format!("/internal/v1/admin/domains/{domain_id}/owner"),
            Some(&provisioning_token(&mock.key_pair)),
            Some(&key),
            Some(json!({"newOwnerPrincipalId": owner})),
        ))
    };
    let (a, b) = tokio::join!(
        replace(owner_a, unique_key("replace-a")),
        replace(owner_b, unique_key("replace-b"))
    );
    assert_eq!(a.unwrap().status(), StatusCode::OK);
    assert_eq!(b.unwrap().status(), StatusCode::OK);
    let active_owners: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM domain_role_bindings
         WHERE domain_id = $1 AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE",
    )
    .bind(domain_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(active_owners, 1);
}

#[tokio::test]
async fn concurrent_domain_key_collision_is_stable() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    seed_provisioning_actor(&pool).await;
    let app = build_app(
        pool.clone(),
        &mock.url,
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let domain_key = unique_domain_key("concurrent-key");
    let call = |domain_id, key: String| {
        app.clone().oneshot(request(
            "POST",
            "/internal/v1/admin/domains",
            Some(&provisioning_token(&mock.key_pair)),
            Some(&key),
            Some(json!({
                "domainId": domain_id,
                "domainKey": domain_key,
                "enabled": true
            })),
        ))
    };
    let (first, second) = tokio::join!(
        call(Uuid::new_v4(), unique_key("domain-race-a")),
        call(Uuid::new_v4(), unique_key("domain-race-b"))
    );
    let mut statuses = [first.unwrap().status(), second.unwrap().status()];
    statuses.sort();
    assert_eq!(statuses, [StatusCode::OK, StatusCode::CONFLICT]);
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM domains WHERE domain_key = $1")
        .bind(domain_key)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
async fn definition_version_includes_mapping_and_domain_gate() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    let (_, domain_id) = seed_principal_domain_with_owner(&pool).await;
    let (_, version_id, _) = seed_published_definition_normal_node(&pool, domain_id).await;
    seed_provisioning_actor(&pool).await;
    let app = build_app(
        pool.clone(),
        &mock.url,
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let response = app
        .clone()
        .oneshot(request(
            "GET",
            &format!("/internal/v1/admin/definition-versions/{version_id}"),
            Some(&provisioning_token(&mock.key_pair)),
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = json_body(response).await;
    assert_eq!(body["domainId"], domain_id.to_string());
    assert_eq!(body["canCreateInstances"], true);

    sqlx::query("UPDATE domains SET enabled = FALSE WHERE domain_id = $1")
        .bind(domain_id)
        .execute(&pool)
        .await
        .unwrap();
    let response = app
        .oneshot(request(
            "GET",
            &format!("/internal/v1/admin/definition-versions/{version_id}"),
            Some(&provisioning_token(&mock.key_pair)),
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(json_body(response).await["canCreateInstances"], false);
}

#[tokio::test]
async fn definition_version_not_found() {
    let pool = create_pool().await;
    let mock = common::MockJwksServer::start().await;
    seed_provisioning_actor(&pool).await;
    let app = build_app(
        pool,
        &mock.url,
        Uuid::parse_str(PROVISIONING_PRINCIPAL_ID).unwrap(),
    );
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    let response = app
        .oneshot(request(
            "GET",
            &format!("/internal/v1/admin/definition-versions/{}", Uuid::new_v4()),
            Some(&provisioning_token(&mock.key_pair)),
            None,
            None,
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
