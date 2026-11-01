use super::*;
pub async fn seed(p: &PgPool) -> (Uuid, Uuid) {
    let owner = Uuid::new_v4();
    let domain = Uuid::new_v4();
    sqlx::query("INSERT INTO principals(principal_id,principal_type,display_name) VALUES($1,'AGENT','owner')").bind(owner).execute(p).await.unwrap();
    sqlx::query("INSERT INTO domains(domain_id,domain_key,display_name) VALUES($1,$2,'isolated')")
        .bind(domain)
        .bind(domain.to_string())
        .execute(p)
        .await
        .unwrap();
    sqlx::query("INSERT INTO domain_role_bindings(binding_id,domain_id,principal_id,role_key) VALUES($1,$2,$3,'DOMAIN_OWNER')").bind(Uuid::new_v4()).bind(domain).bind(owner).execute(p).await.unwrap();
    (owner, domain)
}
pub async fn version(p: &PgPool, domain: Uuid, owner: Uuid, schema: Value) -> (Uuid, Uuid) {
    let d = Uuid::new_v4();
    let v = Uuid::new_v4();
    let n = Uuid::new_v4();
    let end = Uuid::new_v4();
    let t = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_definitions(workflow_definition_id,domain_id,definition_key,display_name) VALUES($1,$2,$3,'Owner test')").bind(d).bind(domain).bind(d.to_string()).execute(p).await.unwrap();
    sqlx::query("INSERT INTO workflow_definition_versions(definition_version_id,workflow_definition_id,version_number,version_status,semantic_model_version,context_schema) VALUES($1,$2,1,'DRAFT',3,$3)").bind(v).bind(d).bind(schema).execute(p).await.unwrap();
    sqlx::query("INSERT INTO workflow_node_definitions(node_id,definition_version_id,node_key,display_name,order_index,node_type,assignee_ref_type,fixed_principal_id) VALUES($1,$2,'task','Task',0,'TASK','FIXED_PRINCIPAL',$3)").bind(n).bind(v).bind(owner).execute(p).await.unwrap();
    sqlx::query("INSERT INTO workflow_node_definitions(node_id,definition_version_id,node_key,display_name,order_index,node_type) VALUES($1,$2,'done','Done',1,'TERMINAL')").bind(end).bind(v).execute(p).await.unwrap();
    sqlx::query("INSERT INTO workflow_transition_definitions(transition_id,definition_version_id,transition_key,display_name,source_node_id,target_node_id,transition_effect) VALUES($1,$2,'finish','Finish',$3,$4,'ADVANCE')").bind(t).bind(v).bind(n).bind(end).execute(p).await.unwrap();
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2",
    )
    .bind(t)
    .bind(n)
    .execute(p)
    .await
    .unwrap();
    (v, t)
}
pub async fn actor(p: &PgPool) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO principals(principal_id,principal_type,display_name) VALUES($1,'AGENT','test actor')").bind(id).execute(p).await.unwrap();
    id
}
pub fn acceptance_schema() -> Value {
    json!({"type":"object","required":["governanceAcceptance"],"properties":{"governanceAcceptance":{"type":"object","additionalProperties":false,"required":["bindingId","acceptanceContractDigest","sourceRequestPayloadDigest","verdict"],"properties":{"bindingId":{"type":"string","format":"uuid"},"acceptanceContractDigest":{"type":"string","pattern":"^[0-9a-f]{64}$"},"sourceRequestPayloadDigest":{"type":"string","pattern":"^[0-9a-f]{64}$"},"verdict":{"type":"string","enum":["ACCEPTED","NOT_ACCEPTED"]}}}}})
}
pub async fn managed_version(p: &PgPool, domain: Uuid, owner: Uuid, role: &str) -> (Uuid, Uuid) {
    let (v,t)=version(p,domain,owner,json!({"type":"object","x-workflow-owner-obligation-role-v1":role,"properties":{"workflowPrerequisite":svc_workflow::domain::definition::prerequisite::property_schema()}})).await;
    let first: Uuid = sqlx::query_scalar(
        "SELECT source_node_id FROM workflow_transition_definitions WHERE transition_id=$1",
    )
    .bind(t)
    .fetch_one(p)
    .await
    .unwrap();
    sqlx::query("UPDATE workflow_node_definitions SET order_index=2 WHERE definition_version_id=$1 AND node_type='TERMINAL'").bind(v).execute(p).await.unwrap();
    let qa = actor(p).await;
    let node = Uuid::new_v4();
    let advance = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_node_definitions(node_id,definition_version_id,node_key,display_name,order_index,node_type,assignee_ref_type,fixed_principal_id,primary_advance_transition_id) VALUES($1,$2,'qa','QA',1,'TASK','FIXED_PRINCIPAL',$3,NULL)").bind(node).bind(v).bind(qa).execute(p).await.unwrap();
    sqlx::query("UPDATE workflow_transition_definitions SET source_node_id=$1,submission_schema=$3 WHERE transition_id=$2").bind(node).bind(t).bind(if role=="INTAKE"{acceptance_schema()}else{json!({"type":"object","required":["reviewOutcome"],"properties":{"reviewOutcome":{"type":"string","enum":["RESOLVED","HUMAN_REQUIRED"]}}})}).execute(p).await.unwrap();
    sqlx::query("INSERT INTO workflow_transition_definitions(transition_id,definition_version_id,transition_key,display_name,source_node_id,target_node_id,transition_effect) VALUES($1,$2,'qa_entry','QA entry',$3,$4,'ADVANCE')").bind(advance).bind(v).bind(first).bind(node).execute(p).await.unwrap();
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2",
    )
    .bind(advance)
    .bind(first)
    .execute(p)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id=$1 WHERE node_id=$2",
    )
    .bind(t)
    .bind(node)
    .execute(p)
    .await
    .unwrap();
    (v, t)
}
pub async fn publish(
    p: &PgPool,
    owner: Uuid,
    v: Uuid,
    pm: Option<Uuid>,
    key: &str,
) -> Result<
    svc_workflow::domain::definition::model::WorkflowDefinitionVersion,
    svc_workflow::application::definition_governance::DefinitionGovernanceError,
> {
    svc_workflow::application::definition_governance::governance_publish_version_with_actor(
        p,
        owner,
        key,
        "isolated-test",
        v,
        None,
        svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        pm,
    )
    .await
}
pub async fn policy_fixture(p: &PgPool) -> (Uuid, Uuid, Uuid, Uuid, Uuid, Uuid) {
    policy_fixture_required(p, false).await
}
pub async fn policy_fixture_required(
    p: &PgPool,
    required: bool,
) -> (Uuid, Uuid, Uuid, Uuid, Uuid, Uuid) {
    let (owner, domain) = seed(p).await;
    let pm = actor(p).await;
    let (intake, it) = managed_version(p, domain, owner, "INTAKE").await;
    if required {
        sqlx::query("UPDATE workflow_definition_versions SET context_schema=context_schema || $2 WHERE definition_version_id=$1").bind(intake).bind(json!({"required":["actualBusinessInput"]})).execute(p).await.unwrap();
    }
    publish(p, owner, intake, None, "intake-publish")
        .await
        .unwrap();
    let (review, rt) = managed_version(p, domain, owner, "REVIEW").await;
    publish(p, owner, review, None, "review-publish")
        .await
        .unwrap();
    let schema = json!({"type":"object","required":["ownerIntakeContext","ownerReviewContext"],"properties":{"ownerIntakeContext":{"type":"object"},"ownerReviewContext":{"type":"object"}},"x-workflow-owner-obligation-v1":{"version":1,"intakeDefinitionVersionId":intake,"intakeAcceptanceTransitionId":it,"reviewDefinitionVersionId":review,"reviewCompletionTransitionId":rt}});
    let (source, _) = version(p, domain, owner, schema).await;
    (owner, domain, pm, source, intake, review)
}
pub async fn create(
    p: &PgPool,
    owner: Uuid,
    domain: Uuid,
    v: Uuid,
    context: Value,
) -> Result<
    svc_workflow::application::workflow_instance::create::CreateWorkflowInstanceResult,
    svc_workflow::domain::workflow_instance::errors::CreateWorkflowInstanceError,
> {
    use svc_workflow::{
        domain::{
            enums::WorkflowExecutionClass, ids::*,
            workflow_instance::commands::CreateWorkflowInstanceCommand,
        },
        store::postgres::admission_gate::AdmissionGate,
    };
    svc_workflow::application::workflow_instance::create::create_workflow_instance(
        p,
        AdmissionGate::disabled(),
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(owner),
            domain_id: DomainId::from_uuid(domain),
            definition_version_id: DefinitionVersionId::from_uuid(v),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".into(),
            execution_class: WorkflowExecutionClass::Business,
            external_reference: None,
            external_url: None,
            metadata: json!({}),
            context_payload: context,
        },
    )
    .await
}
pub async fn request(
    p: &PgPool,
    owner: Uuid,
    instance: Uuid,
    visit: Uuid,
) -> svc_workflow::domain::workflow_instance::assistance::AssistanceCommandResult {
    use svc_workflow::domain::{ids::*, workflow_instance::assistance::*};
    svc_workflow::application::workflow_instance::assistance::request_assistance(
        p,
        RequestAssistanceCommand {
            principal_id: PrincipalId::from_uuid(owner),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".into(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance),
            current_node_visit_id: NodeVisitId::from_uuid(visit),
            expected_workflow_state_version: 1,
            request: AssistancePayload {
                message: "Require governed help".into(),
                supporting_payload: None,
            },
        },
    )
    .await
    .unwrap()
}
pub async fn operation_fixture(p: &PgPool) -> (Uuid, Uuid, Uuid, Uuid, Uuid, Uuid, Uuid) {
    operation_fixture_return(p, false).await
}
pub async fn operation_fixture_return(
    p: &PgPool,
    with_return: bool,
) -> (Uuid, Uuid, Uuid, Uuid, Uuid, Uuid, Uuid) {
    let (owner, domain) = seed(p).await;
    let pm = actor(p).await;
    let (supplier, terminal) = version(p, domain, owner, json!({"type":"object"})).await;
    publish(p, owner, supplier, None, "supplier").await.unwrap();
    let digest: String = sqlx::query_scalar(
        "SELECT definition_digest FROM workflow_definition_versions WHERE definition_version_id=$1",
    )
    .bind(supplier)
    .fetch_one(p)
    .await
    .unwrap();
    let (intake, it) = managed_version(p, domain, owner, "INTAKE").await;
    let descriptor =
        svc_workflow::domain::definition::owner_obligation::registry()[0]["descriptor"].clone();
    sqlx::query("UPDATE workflow_node_definitions SET metadata=$2 WHERE definition_version_id=$1 AND node_key='task'").bind(intake).bind(json!({"workflowOperation":{"version":1,"slotKey":"create-dependency","allowedOperations":[descriptor],"requiredForAdvance":true,"targetDefinitionVersionId":supplier,"targetDefinitionDigest":digest}})).execute(p).await.unwrap();
    if with_return {
        let ids:Vec<(Uuid,String)>=sqlx::query_as("SELECT node_id,node_key FROM workflow_node_definitions WHERE definition_version_id=$1 AND node_type='TASK'").bind(intake).fetch_all(p).await.unwrap();
        let task = ids.iter().find(|(_, k)| k == "task").unwrap().0;
        let qa = ids.iter().find(|(_, k)| k == "qa").unwrap().0;
        sqlx::query("INSERT INTO workflow_transition_definitions(transition_id,definition_version_id,transition_key,display_name,source_node_id,target_node_id,transition_effect,submission_schema) VALUES($1,$2,'return','Return',$3,$4,'RETURN',$5)").bind(Uuid::new_v4()).bind(intake).bind(qa).bind(task).bind(json!({"type":"object","required":["reason"],"properties":{"reason":{"type":"string"}}})).execute(p).await.unwrap();
    }
    publish(p, owner, intake, None, "intake").await.unwrap();
    let (review, rt) = managed_version(p, domain, owner, "REVIEW").await;
    let registry = svc_workflow::domain::definition::owner_obligation::registry();
    sqlx::query("UPDATE workflow_node_definitions SET metadata=$2 WHERE definition_version_id=$1 AND node_key='task'").bind(review).bind(json!({"workflowOperation":{"version":1,"slotKey":"owner-decision","allowedOperations":[registry[1]["descriptor"],registry[2]["descriptor"]],"requiredForAdvance":true}})).execute(p).await.unwrap();
    publish(p, owner, review, None, "review").await.unwrap();
    let schema = json!({"type":"object","required":["ownerIntakeContext","ownerReviewContext"],"properties":{"ownerIntakeContext":{"type":"object"},"ownerReviewContext":{"type":"object"}},"x-workflow-owner-obligation-v1":{"version":1,"intakeDefinitionVersionId":intake,"intakeAcceptanceTransitionId":it,"reviewDefinitionVersionId":review,"reviewCompletionTransitionId":rt}});
    let (source, _) = version(p, domain, owner, schema).await;
    publish(p, owner, source, Some(pm), "source").await.unwrap();
    let s = create(
        p,
        owner,
        domain,
        source,
        json!({"ownerIntakeContext":{},"ownerReviewContext":{}}),
    )
    .await
    .unwrap();
    request(p, owner, s.workflow_instance_id, s.current_node_visit_id).await;
    svc_workflow::store::postgres::workflow_instance_repository::owner_obligation::reconcile::reconcile_pending(p,pm,svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled()).await.unwrap();
    let managed: Uuid = sqlx::query_scalar(
        "SELECT workflow_instance_id FROM workflow_owner_managed_instances WHERE kind='INTAKE'",
    )
    .fetch_one(p)
    .await
    .unwrap();
    (
        owner,
        domain,
        pm,
        managed,
        supplier,
        terminal,
        s.workflow_instance_id,
    )
}
pub async fn transition(p:&PgPool,actor:Uuid,instance:Uuid,version:i32,t:Uuid,payload:Option<Value>)->Result<svc_workflow::application::workflow_instance::execute_transition::ExecuteWorkflowTransitionResult,svc_workflow::domain::workflow_instance::errors::ExecuteWorkflowTransitionError>{
    use svc_workflow::domain::{
        ids::*, workflow_instance::commands::ExecuteWorkflowTransitionCommand,
    };
    svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition(
        p,
        svc_workflow::store::postgres::admission_gate::AdmissionGate::disabled(),
        ExecuteWorkflowTransitionCommand {
            principal_id: PrincipalId::from_uuid(actor),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".into(),
            workflow_instance_id: WorkflowInstanceId::from_uuid(instance),
            expected_workflow_state_version: version,
            transition_definition_id: TransitionId::from_uuid(t),
            submission_payload: payload,
        },
    )
    .await
}

pub async fn predecessor_pool() -> PgPool {
    let name = format!("svc_obl_pred_{}", Uuid::new_v4().simple());
    let admin = PgPool::connect(&common::admin_database_url())
        .await
        .unwrap();
    sqlx::query(&format!("CREATE DATABASE {name}"))
        .execute(&admin)
        .await
        .unwrap();
    admin.close().await;
    println!("fresh retained predecessor DB: {name}");
    let p = PgPool::connect(&format!("{}/{name}", common::test_database_base()))
        .await
        .unwrap();
    let dir = std::env::temp_dir().join(format!("owner-obligation-predecessor-{}", Uuid::new_v4()));
    std::fs::create_dir(&dir).unwrap();
    for e in std::fs::read_dir("migrations").unwrap() {
        let e = e.unwrap();
        let name = e.file_name();
        if name
            .to_string_lossy()
            .split('_')
            .next()
            .unwrap()
            .parse::<u32>()
            .unwrap()
            <= 30
        {
            std::fs::copy(e.path(), dir.join(name)).unwrap();
        }
    }
    sqlx::migrate::Migrator::new(dir.as_path())
        .await
        .unwrap()
        .run(&p)
        .await
        .unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    p
}
pub async fn history(p: &PgPool) -> Value {
    sqlx::query_scalar("SELECT jsonb_build_object('events',(SELECT jsonb_agg(to_jsonb(e) ORDER BY event_id) FROM workflow_events e),'receipts',(SELECT jsonb_agg(to_jsonb(r) ORDER BY command_id) FROM workflow_command_receipts r),'instances',(SELECT jsonb_agg(to_jsonb(w) ORDER BY workflow_instance_id) FROM workflow_instances w),'contexts',(SELECT jsonb_agg(to_jsonb(c) ORDER BY context_revision_id) FROM workflow_context_revisions c),'activations',(SELECT jsonb_agg(to_jsonb(a) ORDER BY activation_id) FROM workflow_activations a))").fetch_one(p).await.unwrap()
}
