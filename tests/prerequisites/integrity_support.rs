use super::*;
pub(super) async fn receipt(p: &PgPool, actor: Uuid, kind: &str) -> Uuid {
    let cmd = Uuid::new_v4();
    sqlx::query("INSERT INTO workflow_command_receipts(command_id,principal_id,idempotency_key,command_type,request_hash) VALUES($1,$2,$3,$4,$5)").bind(cmd).bind(actor).bind(cmd.to_string()).bind(kind).bind("a".repeat(64)).execute(p).await.unwrap();
    cmd
}
pub(super) async fn append(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cmd: Uuid,
    instance: Uuid,
    kind: &str,
    slot: i16,
) -> Result<(), sqlx::Error> {
    sqlx::query("INSERT INTO workflow_events(event_id,workflow_instance_id,event_sequence,event_schema_version,command_id,event_type,context_revision_id,actor_principal_id,old_workflow_state_version,new_workflow_state_version,command_event_slot) SELECT $1,wi.workflow_instance_id,wi.workflow_state_version+1,'v1',$2,$3,wi.current_context_revision_id,r.principal_id,wi.workflow_state_version,wi.workflow_state_version+1,$4 FROM workflow_instances wi CROSS JOIN workflow_command_receipts r WHERE wi.workflow_instance_id=$5 AND r.command_id=$2").bind(Uuid::new_v4()).bind(cmd).bind(kind).bind(slot).bind(instance).execute(&mut **tx).await.map(|_|())
}
pub(super) async fn complete(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    cmd: Uuid,
    status: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE workflow_command_receipts SET receipt_status='COMPLETED',response_status=$2,response_body='{}'::jsonb WHERE command_id=$1").bind(cmd).bind(status).execute(&mut **tx).await.map(|_|())
}
pub(super) async fn pair(
    p: &PgPool,
) -> (
    Fixture,
    Uuid,
    svc_workflow::domain::workflow_instance::governance_child::CreateAndBindGovernanceChildCommand,
    Value,
) {
    let f = setup(p).await;
    let case = request_case(p, &f).await;
    let (v, t) = version(p, &f, false, true).await;
    let cmd=svc_workflow::domain::workflow_instance::governance_child::CreateAndBindGovernanceChildCommand{principal_id:PrincipalId::from_uuid(f.owner),idempotency_key:Uuid::new_v4().to_string(),command_schema_version:"v1".into(),assistance_case_id:AssistanceCaseId::from_uuid(case.assistance_case_id),definition_version_id:DefinitionVersionId::from_uuid(v),context_payload:json!({}),acceptance_transition_id:TransitionId::from_uuid(t),expected_workflow_state_version:2};
    let b =
        svc_workflow::application::workflow_instance::assistance::create_and_bind_governance_child(
            p,
            crate::common::admitting_gate(),
            cmd.clone(),
        )
        .await
        .unwrap();
    let id=sqlx::query_scalar("SELECT command_id FROM workflow_command_receipts WHERE principal_id=$1 AND idempotency_key=$2").bind(f.owner).bind(&cmd.idempotency_key).fetch_one(p).await.unwrap();
    (f, id, cmd, b)
}
