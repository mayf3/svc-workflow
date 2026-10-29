use super::*;
#[tokio::test]
async fn unidentifiable_context_is_closed_unknown_after_real_authorization() {
    let p = pool().await;
    let f = setup(&p).await;
    let (v, t) = version(&p, &f, false, false).await;
    let source = create(&p, &f, v, json!({})).await;
    let (dv, _) = version(&p, &f, true, false).await;
    let dep = create(&p, &f, dv, reference(source, v, t)).await;
    sqlx::query("INSERT INTO domain_role_bindings(binding_id,domain_id,principal_id,role_key,enabled) VALUES($1,$2,$3,'DOMAIN_MEMBER',true)").bind(Uuid::new_v4()).bind(f.domain).bind(f.outsider).execute(&p).await.unwrap();
    let mock = common::MockJwksServer::start().await;
    let router = http_support::app(&p, &mock).await;
    let path = format!("/internal/v1/workflow-instances/{dep}/readiness");
    for broken in [
        json!({}),
        json!({"workflowPrerequisite":{"workflowInstanceId":source}}),
        json!({"workflowPrerequisite":{"kind":"TERMINAL_SUBMISSION","workflowInstanceId":source,"definitionVersionId":v,"transitionId":"invalid"}}),
    ] {
        let mut tx = p.begin().await.unwrap();
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await
            .unwrap();
        sqlx::query(
            "UPDATE workflow_context_revisions SET payload=$2 WHERE workflow_instance_id=$1",
        )
        .bind(dep)
        .bind(broken)
        .execute(&mut *tx)
        .await
        .unwrap();
        tx.commit().await.unwrap();
        for actor in [f.agent, f.owner] {
            let (s, b) = http(router.clone(), actor, &mock, "GET", &path, None, json!({})).await;
            assert_eq!(s, 200, "{b}");
            assert_eq!(
                b,
                json!({"prerequisiteReadiness":{"status":"UNKNOWN","reason":"LINEAGE_INCONSISTENT"}})
            );
        }
        assert_ne!(
            http(
                router.clone(),
                f.outsider,
                &mock,
                "GET",
                &path,
                None,
                json!({})
            )
            .await
            .0,
            200
        );
    }
    let mut identified = reference(source, v, t);
    identified["workflowPrerequisite"]
        .as_object_mut()
        .unwrap()
        .remove("kind");
    let mut tx = p.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("UPDATE workflow_context_revisions SET payload=$2 WHERE workflow_instance_id=$1")
        .bind(dep)
        .bind(identified)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let (s, b) = http(
        router.clone(),
        f.agent,
        &mock,
        "GET",
        &path,
        None,
        json!({}),
    )
    .await;
    assert_eq!(s, 200);
    assert_eq!(
        b,
        json!({"prerequisiteReadiness":{"status":"UNKNOWN","reason":"LINEAGE_INCONSISTENT","sourceInstanceId":source,"sourceDefinitionVersionId":v,"transitionId":t}})
    );
    let other = setup(&p).await;
    assert_ne!(
        http(
            router.clone(),
            other.owner,
            &mock,
            "GET",
            &path,
            None,
            json!({})
        )
        .await
        .0,
        200
    );
    let mut tx = p.begin().await.unwrap();
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("DELETE FROM workflow_context_revisions WHERE workflow_instance_id=$1")
        .bind(dep)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let (s, b) = http(router, f.agent, &mock, "GET", &path, None, json!({})).await;
    assert_eq!(s, 200, "{b}");
    assert_eq!(
        b,
        json!({"prerequisiteReadiness":{"status":"UNKNOWN","reason":"LINEAGE_INCONSISTENT"}})
    );
    // Ordinary detail still rejects corrupt Context; only readiness has this seam.
    assert!(
        svc_workflow::application::workflow_instance::query_service::WorkflowQueryService::new(
            p.clone()
        )
        .get_workflow_instance_detail(
            svc_workflow::application::workflow_instance::query_types::GetWorkflowInstanceDetail {
                actor_principal_id: f.agent,
                workflow_instance_id: dep
            }
        )
        .await
        .is_err()
    );
    p.close().await;
    let error = svc_workflow::store::postgres::workflow_instance_repository::prerequisite::read(
        &p, f.agent, dep,
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error,
        svc_workflow::application::workflow_instance::query_types::WorkflowQueryError::StorageError(
            _
        )
    ));
}
