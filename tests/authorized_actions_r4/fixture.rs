use serde_json::json;
use sqlx::PgPool;
use svc_workflow::{
    application::definition::{commands::{CreateDefinition,CreateDraftVersion,PublishVersion,RawNodeDefinition,RawTransitionDefinition,ReplaceDraftGraph},repository::DefinitionRepository,DefinitionService},
    application::workflow_instance::create::create_workflow_instance,
    domain::{definition::{graph::validate_graph,model::WorkflowGraph},enums::WorkflowExecutionClass,ids::*,workflow_instance::commands::CreateWorkflowInstanceCommand},
    store::postgres::{admission_gate::AdmissionGate,definition_repository::PgDefinitionRepository},
};
use uuid::Uuid;

#[derive(Clone,Copy)]
pub struct Fixture {pub agent:Uuid,pub owner:Uuid,pub outsider:Uuid,pub coordinator:Uuid,pub domain:Uuid,pub version:Uuid,pub start:Uuid,pub work:Uuid,pub done:Uuid,pub advance:Uuid,pub finish:Uuid,pub ret:Uuid,pub terminate:Uuid}

fn node(key:&str,index:i32,kind:&str,primary:Option<&str>)->RawNodeDefinition {
    RawNodeDefinition {node_key:key.into(),display_name:key.into(),order_index:index,node_type:kind.into(),assignee_ref_type:if kind=="TERMINAL" {None}else{Some("WORKFLOW_CREATOR".into())},fixed_principal_id:None,assignee_input_key:None,instructions:None,primary_advance_transition_key:primary.map(str::to_owned),metadata:None}
}
fn edge(key:&str,source:&str,target:&str,effect:&str)->RawTransitionDefinition {
    RawTransitionDefinition {transition_key:key.into(),display_name:key.into(),source_node_key:source.into(),target_node_key:target.into(),transition_effect:effect.into(),submission_schema:None,metadata:None}
}
pub async fn setup(pool:&PgPool,model:i16)->Fixture { setup_variant(pool,model,false).await }
pub async fn setup_variant(pool:&PgPool,model:i16,work_owner:bool)->Fixture {
    let agent=Uuid::new_v4();let owner=Uuid::new_v4();let outsider=Uuid::new_v4();let coordinator=Uuid::new_v4();let domain=Uuid::new_v4();
    for (id,name) in [(agent,"agent"),(owner,"owner"),(outsider,"outsider"),(coordinator,"coordinator")] {
        sqlx::query("INSERT INTO principals(principal_id,principal_type,display_name,email,enabled) VALUES($1,'AGENT',$2,$3,TRUE)").bind(id).bind(name).bind(format!("{id}@example.test")).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO domains(domain_id,domain_key,display_name,enabled) VALUES($1,$2,'AA R4 isolated',TRUE)").bind(domain).bind(format!("aa-r4-{domain}")).execute(pool).await.unwrap();
    for (actor,role) in [(owner,"DOMAIN_OWNER"),(agent,"DOMAIN_MEMBER")] {
        sqlx::query("INSERT INTO domain_role_bindings(binding_id,domain_id,principal_id,role_key,enabled) VALUES($1,$2,$3,$4,TRUE)").bind(Uuid::new_v4()).bind(domain).bind(actor).bind(role).execute(pool).await.unwrap();
    }
    sqlx::query("INSERT INTO global_role_bindings(binding_id,principal_id,role_key,enabled) VALUES($1,$2,'GLOBAL_WORKFLOW_COORDINATOR',TRUE)").bind(Uuid::new_v4()).bind(coordinator).execute(pool).await.unwrap();
    let service=DefinitionService::new(PgDefinitionRepository::new(pool.clone()));
    let def=service.create_definition(CreateDefinition{actor_principal_id:owner,owner_domain_id:domain,definition_key:format!("aa-r4-{}",Uuid::new_v4()),display_name:"AA legitimate graph".into(),description:None,metadata:None}).await.unwrap();
    let version=service.create_draft_version(CreateDraftVersion{actor_principal_id:owner,workflow_definition_id:def.id.into_uuid(),context_schema:Some(json!({"type":"object"})),json_schema_dialect:None,validator_version:None,metadata:None,semantic_model_version:model}).await.unwrap().id.into_uuid();
    let mut work=node("work",1,if model==3 {"TASK"}else{"NORMAL"},if model==2 {None}else{Some("work_done")});
    if work_owner {work.assignee_ref_type=Some("FIXED_PRINCIPAL".into());work.fixed_principal_id=Some(owner);}
    service.replace_draft_graph(ReplaceDraftGraph{actor_principal_id:owner,definition_version_id:version,context_schema:Some(json!({"type":"object"})),nodes:vec![node("start",0,if model==3 {"TASK"}else if model==2 {"NORMAL"}else{"DRAFT"},if model==2 {None}else{Some("start_work")}),work,node("done",2,"TERMINAL",None)],transitions:{let mut e=vec![edge("start_work","start","work","ADVANCE"),edge("work_done","work","done","ADVANCE"),edge("work_return","work","start","RETURN")];if model!=2 {e.push(edge("work_terminate","work","done","TERMINATE"));}e}}).await.expect("real Definition graph replacement accepts legal fixture");
    let (nodes,transitions)=service.repo.get_complete_graph(version).await.unwrap();
    if model==1 {let v=validate_graph(&WorkflowGraph{nodes:nodes.clone(),transitions:transitions.clone(),context_schema:Some(json!({"type":"object"}))});assert!(v.valid,"Legacy graph validation: {:?}",v.errors);}
    let published=service.publish_version(PublishVersion{actor_principal_id:owner,definition_version_id:version,expected_revision:None},AdmissionGate::disabled()).await.expect("real publication of legitimate graph");
    super::support::persist_probe(&format!("graph-{version}"),&json!({"model":model,"publication":published,"nodes":nodes,"transitions":transitions}));
    let n=|k:&str|nodes.iter().find(|n|n.node_key==k).unwrap().node_id.into_uuid();let t=|k:&str|transitions.iter().find(|t|t.transition_key==k).unwrap().transition_id.into_uuid();
    Fixture{agent,owner,outsider,coordinator,domain,version,start:n("start"),work:n("work"),done:n("done"),advance:t("start_work"),finish:t("work_done"),ret:t("work_return"),terminate:if model==2 {t("work_done")}else{t("work_terminate")}}
}
pub async fn create(pool:&PgPool,f:Fixture)->Uuid {
    create_workflow_instance(pool,AdmissionGate::disabled(),CreateWorkflowInstanceCommand{principal_id:PrincipalId::from_uuid(f.agent),idempotency_key:Uuid::new_v4().to_string(),command_schema_version:"v1".into(),execution_class:WorkflowExecutionClass::Business,domain_id:DomainId::from_uuid(f.domain),definition_version_id:DefinitionVersionId::from_uuid(f.version),external_reference:None,external_url:None,metadata:json!({}),context_payload:json!({})}).await.unwrap().workflow_instance_id
}
