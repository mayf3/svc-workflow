use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use svc_workflow::application::workflow_instance::assistance::{
    escalate_assistance_to_human, request_assistance, resolve_assistance,
};
use svc_workflow::application::workflow_instance::create::create_workflow_instance;
use svc_workflow::application::workflow_instance::execute_transition::execute_workflow_transition;
use svc_workflow::domain::ids::*;
use svc_workflow::domain::workflow_instance::assistance::{
    AssistanceCommandResult, AssistancePayload, EscalateAssistanceCommand,
    RequestAssistanceCommand, ResolveAssistanceCommand,
};
use svc_workflow::domain::workflow_instance::commands::{
    CreateWorkflowInstanceCommand, ExecuteWorkflowTransitionCommand,
};

#[derive(Clone, Copy)]
pub(crate) struct Fixture {
    pub agent: Uuid,
    pub owner: Uuid,
    pub replacement_owner: Uuid,
    pub coordinator: Uuid,
    pub outsider: Uuid,
    pub admin: Uuid,
    pub domain: Uuid,
    pub terminal_node: Uuid,
    pub transition: Uuid,
    pub instance: Uuid,
    pub visit: Uuid,
}

pub(crate) fn request_hash(label: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(label.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) async fn seed_agent(pool: &PgPool, label: &str) -> Uuid {
    let id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO principals
         (principal_id, principal_type, display_name, email, enabled)
         VALUES ($1, 'AGENT', $2, $3, TRUE)",
    )
    .bind(id)
    .bind(format!("{label} agent"))
    .bind(format!("{label}-{}@example.test", Uuid::new_v4()))
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn bind_domain_role(pool: &PgPool, domain: Uuid, principal: Uuid, role: &str) {
    sqlx::query(
        "INSERT INTO domain_role_bindings
         (binding_id, domain_id, principal_id, role_key, enabled)
         VALUES ($1,$2,$3,$4,TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(domain)
    .bind(principal)
    .bind(role)
    .execute(pool)
    .await
    .unwrap();
}

pub(crate) async fn setup(pool: &PgPool) -> Fixture {
    let agent = seed_agent(pool, "requester").await;
    let owner = seed_agent(pool, "owner").await;
    let replacement_owner = seed_agent(pool, "replacement-owner").await;
    let coordinator = seed_agent(pool, "coordinator").await;
    let outsider = seed_agent(pool, "outsider").await;
    let admin = seed_agent(pool, "admin").await;
    let domain = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO domains (domain_id, domain_key, display_name, enabled)
         VALUES ($1,$2,'Assistance Test Domain',TRUE)",
    )
    .bind(domain)
    .bind(format!("assistance-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .unwrap();
    bind_domain_role(pool, domain, owner, "DOMAIN_OWNER").await;
    bind_domain_role(pool, domain, agent, "DOMAIN_MEMBER").await;
    bind_domain_role(pool, domain, admin, "WORKFLOW_ADMIN").await;
    sqlx::query(
        "INSERT INTO global_role_bindings
         (binding_id, principal_id, role_key, enabled)
         VALUES ($1,$2,'GLOBAL_WORKFLOW_COORDINATOR',TRUE)",
    )
    .bind(Uuid::new_v4())
    .bind(coordinator)
    .execute(pool)
    .await
    .unwrap();

    let definition = Uuid::new_v4();
    let definition_version = Uuid::new_v4();
    let draft_node = Uuid::new_v4();
    let terminal_node = Uuid::new_v4();
    let transition = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_definitions
         (workflow_definition_id, domain_id, definition_key, display_name)
         VALUES ($1,$2,$3,'Assistance Test')",
    )
    .bind(definition)
    .bind(domain)
    .bind(format!("assistance-def-{}", Uuid::new_v4()))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow_definition_versions
         (definition_version_id, workflow_definition_id, version_number,
          version_status, context_schema)
         VALUES ($1,$2,1,'DRAFT','{\"type\":\"object\"}'::jsonb)",
    )
    .bind(definition_version)
    .bind(definition)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow_node_definitions
         (node_id, definition_version_id, node_key, display_name, order_index,
          node_type, assignee_ref_type)
         VALUES ($1,$2,'draft','Draft',0,'DRAFT','WORKFLOW_CREATOR')",
    )
    .bind(draft_node)
    .bind(definition_version)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow_node_definitions
         (node_id, definition_version_id, node_key, display_name, order_index, node_type)
         VALUES ($1,$2,'done','Done',1,'TERMINAL')",
    )
    .bind(terminal_node)
    .bind(definition_version)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow_transition_definitions
         (transition_id, definition_version_id, transition_key, display_name,
          source_node_id, target_node_id, transition_effect, submission_schema)
         VALUES ($1,$2,'advance','Advance',$3,$4,'ADVANCE',
                 '{\"type\":\"object\"}'::jsonb)",
    )
    .bind(transition)
    .bind(definition_version)
    .bind(draft_node)
    .bind(terminal_node)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE workflow_node_definitions SET primary_advance_transition_id=$1
         WHERE node_id=$2",
    )
    .bind(transition)
    .bind(draft_node)
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "UPDATE workflow_definition_versions SET version_status='PUBLISHED'
         WHERE definition_version_id=$1",
    )
    .bind(definition_version)
    .execute(pool)
    .await
    .unwrap();

    let created = create_workflow_instance(
        pool,
        CreateWorkflowInstanceCommand {
            principal_id: PrincipalId::from_uuid(agent),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            domain_id: DomainId::from_uuid(domain),
            definition_version_id: DefinitionVersionId::from_uuid(definition_version),
            external_reference: None,
            external_url: None,
            metadata: serde_json::json!({}),
            context_payload: serde_json::json!({}),
        },
    )
    .await
    .unwrap();
    assert_eq!(created.workflow_state_version, 1);

    Fixture {
        agent,
        owner,
        replacement_owner,
        coordinator,
        outsider,
        admin,
        domain,
        terminal_node,
        transition,
        instance: created.workflow_instance_id,
        visit: created.current_node_visit_id,
    }
}

pub(crate) fn payload(message: &str) -> AssistancePayload {
    AssistancePayload {
        message: message.to_string(),
        supporting_payload: None,
    }
}

pub(crate) fn request_command(f: &Fixture, key: String) -> RequestAssistanceCommand {
    RequestAssistanceCommand {
        principal_id: PrincipalId::from_uuid(f.agent),
        idempotency_key: key,
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(f.instance),
        current_node_visit_id: NodeVisitId::from_uuid(f.visit),
        expected_workflow_state_version: 1,
        request: payload("Need Domain Owner assistance"),
    }
}

pub(crate) async fn request_case(pool: &PgPool, f: &Fixture) -> AssistanceCommandResult {
    request_assistance(pool, request_command(f, Uuid::new_v4().to_string()))
        .await
        .unwrap()
}

pub(crate) fn transition_command(f: &Fixture, expected: i32) -> ExecuteWorkflowTransitionCommand {
    ExecuteWorkflowTransitionCommand {
        principal_id: PrincipalId::from_uuid(f.agent),
        idempotency_key: Uuid::new_v4().to_string(),
        command_schema_version: "v1".to_string(),
        workflow_instance_id: WorkflowInstanceId::from_uuid(f.instance),
        expected_workflow_state_version: expected,
        transition_definition_id: TransitionId::from_uuid(f.transition),
        submission_payload: Some(serde_json::json!({})),
    }
}

pub(crate) async fn transition(pool: &PgPool, f: &Fixture, expected: i32) -> i32 {
    execute_workflow_transition(pool, transition_command(f, expected))
        .await
        .unwrap()
        .workflow_state_version
}

pub(crate) async fn escalate(
    pool: &PgPool,
    f: &Fixture,
    case_id: Uuid,
    expected: i32,
) -> AssistanceCommandResult {
    escalate_assistance_to_human(
        pool,
        EscalateAssistanceCommand {
            principal_id: PrincipalId::from_uuid(f.owner),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            assistance_case_id: AssistanceCaseId::from_uuid(case_id),
            expected_workflow_state_version: expected,
            escalation: payload("External Human review is required"),
        },
    )
    .await
    .unwrap()
}

pub(crate) async fn resolve(
    pool: &PgPool,
    f: &Fixture,
    case_id: Uuid,
    expected: i32,
) -> AssistanceCommandResult {
    resolve_assistance(
        pool,
        ResolveAssistanceCommand {
            principal_id: PrincipalId::from_uuid(f.owner),
            idempotency_key: Uuid::new_v4().to_string(),
            command_schema_version: "v1".to_string(),
            assistance_case_id: AssistanceCaseId::from_uuid(case_id),
            expected_workflow_state_version: expected,
            resolution: AssistancePayload {
                message: "Owner-approved resolution".to_string(),
                supporting_payload: Some(serde_json::json!({
                    "externalApprovalId": "HUMAN-42"
                })),
            },
        },
    )
    .await
    .unwrap()
}

pub(crate) async fn insert_open_case(pool: &PgPool, f: &Fixture, visit: Uuid) -> Uuid {
    let command_id = Uuid::new_v4();
    let case_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO workflow_command_receipts
         (command_id, principal_id, idempotency_key, command_type, request_hash,
          receipt_status, response_status, response_body, response_digest, completed_at)
         VALUES ($1,$2,$3,'REQUEST_WORKFLOW_ASSISTANCE',$4,
                 'COMPLETED',201,'{}'::jsonb,$5,now())",
    )
    .bind(command_id)
    .bind(f.agent)
    .bind(Uuid::new_v4().to_string())
    .bind(request_hash("synthetic assistance request"))
    .bind(request_hash("synthetic assistance response"))
    .execute(pool)
    .await
    .unwrap();
    sqlx::query(
        "INSERT INTO workflow_assistance_cases
         (assistance_case_id, workflow_instance_id, node_visit_id, status,
          requested_by_principal_id, request_payload, request_payload_digest,
          request_command_id)
         VALUES ($1,$2,$3,'OWNER_PENDING',$4,$5,$6,$7)",
    )
    .bind(case_id)
    .bind(f.instance)
    .bind(visit)
    .bind(f.agent)
    .bind(serde_json::json!({"message":"synthetic recovery case"}))
    .bind(request_hash("synthetic recovery payload"))
    .bind(command_id)
    .execute(pool)
    .await
    .unwrap();
    case_id
}

pub(crate) async fn case_status(pool: &PgPool, case_id: Uuid) -> (String, Option<String>) {
    sqlx::query_as(
        "SELECT status, void_reason_code FROM workflow_assistance_cases
         WHERE assistance_case_id=$1",
    )
    .bind(case_id)
    .fetch_one(pool)
    .await
    .unwrap()
}
