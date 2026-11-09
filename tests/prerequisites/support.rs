use super::*;
pub(super) async fn pool() -> PgPool {
    let _ = tracing_subscriber::fmt()
        .with_env_filter("error")
        .try_init();
    assert!(
        common::test_database_url().ends_with("/svc_prereq_20261029_impl"),
        "explicit fresh task database required"
    );
    let database = format!("svc_prereq_20261029_impl_{}", Uuid::new_v4().simple());
    let admin = sqlx::PgPool::connect(&common::admin_database_url())
        .await
        .unwrap();
    sqlx::query(&format!("CREATE DATABASE {database}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
    println!("fresh isolated database: {database}");
    let pool = sqlx::PgPool::connect(&format!("{}/{}", common::test_database_base(), database))
        .await
        .unwrap();
    sqlx::migrate::Migrator::new(std::path::Path::new("migrations"))
        .await
        .unwrap()
        .run(&pool)
        .await
        .unwrap();
    pool
}
pub(super) async fn version(p: &PgPool, f: &Fixture, opt: bool, acceptance: bool) -> (Uuid, Uuid) {
    let context = if opt {
        json!({"type":"object","x-workflow-prerequisite-v1":true,"required":["workflowPrerequisite"],"properties":{"workflowPrerequisite":svc_workflow::domain::definition::prerequisite::property_schema()}})
    } else {
        json!({"type":"object"})
    };
    draft_version(p, f, context, acceptance, true).await
}
pub(super) async fn draft_version(
    p: &PgPool,
    f: &Fixture,
    context: Value,
    acceptance: bool,
    publish: bool,
) -> (Uuid, Uuid) {
    let (d, v, n, t, end) = (
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
        Uuid::new_v4(),
    );
    sqlx::query("INSERT INTO workflow_definitions(workflow_definition_id,domain_id,definition_key,display_name) VALUES($1,$2,$3,'Prerequisite test')").bind(d).bind(f.domain).bind(d.to_string()).execute(p).await.unwrap();
    sqlx::query("INSERT INTO workflow_definition_versions(definition_version_id,workflow_definition_id,version_number,version_status,semantic_model_version,context_schema) VALUES($1,$2,1,'DRAFT',3,$3)").bind(v).bind(d).bind(context).execute(p).await.unwrap();
    sqlx::query("INSERT INTO workflow_node_definitions(node_id,definition_version_id,node_key,display_name,order_index,node_type,assignee_ref_type,fixed_principal_id) VALUES($1,$2,'task','Task',0,'TASK','FIXED_PRINCIPAL',$3)").bind(n).bind(v).bind(f.agent).execute(p).await.unwrap();
    sqlx::query("INSERT INTO workflow_node_definitions(node_id,definition_version_id,node_key,display_name,order_index,node_type) VALUES($1,$2,'done','Done',1,'TERMINAL')").bind(end).bind(v).execute(p).await.unwrap();
    let schema = if acceptance {
        acceptance_schema()
    } else {
        json!({"type":"object"})
    };
    sqlx::query("INSERT INTO workflow_transition_definitions(transition_id,definition_version_id,transition_key,display_name,source_node_id,target_node_id,transition_effect,submission_schema) VALUES($1,$2,'finish','Finish',$3,$4,'ADVANCE',$5)").bind(t).bind(v).bind(n).bind(end).bind(schema).execute(p).await.unwrap();
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2",
    )
    .bind(t)
    .bind(n)
    .execute(p)
    .await
    .unwrap();
    if publish {
        sqlx::query("UPDATE workflow_definition_versions SET version_status='PUBLISHED' WHERE definition_version_id=$1").bind(v).execute(p).await.unwrap();
    }
    (v, t)
}
pub(super) fn acceptance_schema() -> Value {
    json!({"type":"object","required":["governanceAcceptance"],"properties":{"governanceAcceptance":{"type":"object","additionalProperties":false,"required":["bindingId","acceptanceContractDigest","sourceRequestPayloadDigest","verdict"],"properties":{"bindingId":{"type":"string","format":"uuid"},"acceptanceContractDigest":{"type":"string","pattern":"^[0-9a-f]{64}$"},"sourceRequestPayloadDigest":{"type":"string","pattern":"^[0-9a-f]{64}$"},"verdict":{"type":"string","enum":["ACCEPTED","NOT_ACCEPTED"]}}}}})
}
pub(super) fn reference(i: Uuid, v: Uuid, t: Uuid) -> Value {
    json!({"workflowPrerequisite":{"kind":"TERMINAL_SUBMISSION","workflowInstanceId":i,"definitionVersionId":v,"transitionId":t}})
}
pub(super) fn create_cmd(f: &Fixture, v: Uuid, c: Value) -> CreateWorkflowInstanceCommand {
    CreateWorkflowInstanceCommand {
        principal_id: PrincipalId::from_uuid(f.owner),
        domain_id: DomainId::from_uuid(f.domain),
        definition_version_id: DefinitionVersionId::from_uuid(v),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".into(),
        execution_class: WorkflowExecutionClass::Business,
        external_reference: None,
        external_url: None,
        metadata: json!({}),
        context_payload: c,
    }
}
pub(super) async fn create(p: &PgPool, f: &Fixture, v: Uuid, c: Value) -> Uuid {
    create_workflow_instance(p, crate::common::admitting_gate(), create_cmd(f, v, c))
        .await
        .unwrap()
        .workflow_instance_id
}
pub(super) async fn http(
    router: axum::Router,
    actor: Uuid,
    m: &common::MockJwksServer,
    method: &str,
    path: &str,
    key: Option<&str>,
    body: Value,
) -> (u16, Value) {
    let token = common::v1_token(
        actor,
        "workflow.execute workflow.read",
        "gcr-http",
        300,
        &m.key_pair,
    );
    let mut r = Request::builder()
        .method(method)
        .uri(path)
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json");
    if let Some(k) = key {
        r = r.header("idempotency-key", k)
    }
    let response = router
        .oneshot(
            r.body(if method == "GET" {
                Body::empty()
            } else {
                Body::from(body.to_string())
            })
            .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status().as_u16();
    let b = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&b).unwrap_or(Value::Null))
}
