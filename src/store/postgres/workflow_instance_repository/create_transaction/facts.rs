//! Shared create predicates and facts inside the caller's transaction.
use super::*;
use sqlx::{Postgres, Transaction};
pub(crate) async fn create_on_tx(
    tx: &mut Transaction<'_, Postgres>,
    admission: AdmissionGate<'_>,
    cmd: &CreateWorkflowInstanceCommand,
    actual_command_id: Uuid,
) -> Result<CreateResult, CreateWorkflowInstanceError> {
    let principal_uuid = cmd.principal_id.into_uuid();
    let domain_uuid = cmd.domain_id.into_uuid();
    let definition_version_uuid = cmd.definition_version_id.into_uuid();
    let workflow_instance_id = Uuid::new_v4();
    let context_revision_id = Uuid::new_v4();
    let node_visit_id = Uuid::new_v4();
    let event_id = Uuid::new_v4();
    let activation_id = Uuid::new_v4();
    macro_rules! deterministic_failure {
        ($err:expr) => {
            return Err($err)
        };
    }
    macro_rules! validation_result {
        ($result:expr) => {
            $result?
        };
    }
    validation_result!(validation_helpers::validate_request_sizes(&cmd));
    let version_info = validation_result!(
        lock_and_validate_version(tx, definition_version_uuid, domain_uuid).await
    );
    if let Some(err) =
        validation_result!(validation_helpers::validate_domain_enabled(tx, domain_uuid).await)
    {
        deterministic_failure!(err);
    }
    if let Some(err) =
        validation_result!(validation_helpers::validate_principal_enabled(tx, principal_uuid).await)
    {
        deterministic_failure!(err);
    }
    let managed_role =
        crate::domain::definition::owner_obligation::role(version_info.context_schema.as_ref())
            .map_err(CreateWorkflowInstanceError::ContextValidationFailed)?;
    let policy_authorized = if managed_role.is_some() {
        super::super::owner_obligation::intents::policy_actor_authorized(
            tx,
            actual_command_id,
            domain_uuid,
            definition_version_uuid,
            principal_uuid,
        )
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?
    } else {
        false
    };
    if managed_role.is_some() && !policy_authorized {
        return Err(CreateWorkflowInstanceError::ContextValidationFailed(
            "MANAGED_WORKFLOW_POLICY_REQUIRED".into(),
        ));
    }
    if !policy_authorized {
        if let Some(err) =
            validation_helpers::validate_domain_membership(tx, domain_uuid, principal_uuid).await?
        {
            return Err(err);
        }
    }
    // Work execution class marking (SVC_WORKFLOW_WORK_EXECUTION_CLASS_V1,
    // CTR-WEC-002): NON_BUSINESS_TEST is a governance decision reserved to
    // the enabled DOMAIN_OWNER of the target domain — the exact in-tx
    // predicate family as cancel/archive. An ordinary member must not be
    // able to suppress work (assigned to any principal) from the automated
    // business dispatcher.
    if cmd.execution_class == WorkflowExecutionClass::NonBusinessTest {
        let is_owner: bool = sqlx::query_scalar(
            "SELECT EXISTS(
               SELECT 1 FROM domain_role_bindings
               WHERE domain_id = $1 AND principal_id = $2
                 AND role_key = 'DOMAIN_OWNER' AND enabled = TRUE)",
        )
        .bind(domain_uuid)
        .bind(principal_uuid)
        .fetch_one(&mut **tx)
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        if !is_owner {
            deterministic_failure!(CreateWorkflowInstanceError::NotDomainOwner);
        }
    }
    // Semantic model dispatch: entry node comes from the definition's
    // semantic model, never guessed from node shape.
    let entry_node = match version_info.semantic_model_version {
        1 => validation_result!(read_draft_node(tx, definition_version_uuid).await),
        2 => {
            // Minimal definitions are trusted to satisfy the Minimal
            // contract — graph legality is the caller's responsibility
            // (they may run validate_minimal_graph before submission).
            // The Runtime keeps only cheap execution fail-closeds: the
            // entry TASK must be determinable, and forbidden DOMAIN_OWNER
            // assignment on the entry is rejected outright.
            let entry =
                validation_result!(read_minimal_entry_node(tx, definition_version_uuid).await);
            if entry.assignee_ref_type == crate::domain::enums::AssigneeRefType::DomainOwner {
                deterministic_failure!(CreateWorkflowInstanceError::AssigneeResolutionFailed(
                    "Minimal (V2) definition entry uses forbidden DOMAIN_OWNER assignee"
                        .to_string(),
                ));
            }
            entry
        }
        3 => {
            // VISIT_ACTIVATION_V1: entry must be a TASK of the new model.
            // Owner references outside the closed set are rejected here as
            // well (the validator already rejects them at publish time).
            let entry = validation_result!(
                read_visit_activation_entry_node(tx, definition_version_uuid).await
            );
            if entry.assignee_ref_type
                == crate::domain::enums::AssigneeRefType::InstanceInputPrincipal
            {
                deterministic_failure!(CreateWorkflowInstanceError::AssigneeResolutionFailed(
                    "VISIT_ACTIVATION_V1 entry uses INSTANCE_INPUT_PRINCIPAL, which is not                      in the new-model owner reference set"
                        .to_string(),
                ));
            }
            entry
        }
        other => {
            return Err(CreateWorkflowInstanceError::InternalConsistency(format!(
                "unknown semantic_model_version {other} for definition version"
            )))
        }
    };
    let resolved_assignee_id = validation_result!(
        validation_helpers::resolve_assignee(
            tx,
            &entry_node,
            principal_uuid,
            domain_uuid,
            &cmd.context_payload,
        )
        .await
    );
    // VISIT_ACTIVATION_V1 owners must resolve to an enabled canonical HUMAN
    // or AGENT Principal; SERVICE can never own new-model work.
    if version_info.semantic_model_version == 3 {
        let owner_type_check =
            activation_facts::validate_owner_is_human_or_agent(tx, resolved_assignee_id)
                .await
                .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        if let Err(reason) = owner_type_check {
            deterministic_failure!(CreateWorkflowInstanceError::AssigneeResolutionFailed(
                reason
            ));
        }
    }
    validation_result!(validation_helpers::validate_context_schema(
        &version_info.context_schema,
        &cmd,
    ));
    // The definition's real node requirements (INSTANCE_INPUT_PRINCIPAL
    // assignee keys) are a hard create-time invariant: every future node
    // that reads its assignee from the context payload must already be
    // resolvable, or the instance would be created half-legal and the
    // read path would fail with an internal consistency error later.
    validation_result!(
        validation_helpers::validate_instance_input_principal_keys(
            tx,
            definition_version_uuid,
            &cmd.context_payload,
        )
        .await
    );

    // ---------------------------------------------------------------
    // SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1 (CTR-ACB-002): for a
    // definition with an opt-in transition, the formal Context must carry a
    // valid acceptanceCriteria catalog. Checked after the ordinary Context
    // schema validation above, before any workflow-fact write below.
    // ---------------------------------------------------------------
    validation_result!(
        validation_helpers::validate_acceptance_catalog_gate(
            tx,
            definition_version_uuid,
            &cmd.context_payload
        )
        .await
    );

    if version_info.semantic_model_version != 3
        && crate::domain::definition::prerequisite::parse(
            version_info.context_schema.as_ref(),
            &cmd.context_payload,
        )
        .map_err(CreateWorkflowInstanceError::ContextValidationFailed)?
        .is_some()
    {
        return Err(CreateWorkflowInstanceError::ContextValidationFailed(
            "prerequisites require VISIT_ACTIVATION_V1".into(),
        ));
    }
    let prerequisite_readiness = super::super::prerequisite::admit(
        tx,
        principal_uuid,
        domain_uuid,
        version_info.context_schema.as_ref(),
        &cmd.context_payload,
    )
    .await?;

    // ---------------------------------------------------------------
    // CTR-CIR-003: canonical identity admission, inside the committing
    // transaction and before the first runtime-fact write. The command's
    // Agent Principal set is the resolved entry-visit assignee (including a
    // WORKFLOW_CREATOR resolution — it is an assignment fact) plus every
    // INSTANCE_INPUT_PRINCIPAL identity value persisted in the context
    // payload. Per-command identical IDs share one still-current observation;
    // any admission failure rolls the whole transaction back (zero business
    // delta — no partial write, no deterministic failure receipt, no retry).
    // ---------------------------------------------------------------
    let input_keys =
        admission_gate_module::required_input_principal_keys(tx, definition_version_uuid)
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
    let input_principal_ids = admission_gate_module::extract_required_input_principal_ids(
        &input_keys,
        &cmd.context_payload,
    )
    .map_err(CreateWorkflowInstanceError::AssigneeResolutionFailed)?;
    let mut admission_principals = BTreeSet::from([resolved_assignee_id]);
    admission_principals.extend(input_principal_ids);
    // T72 (WF-GS-07): resolve lineage through the COMMITTING transaction —
    // never borrow a second pool connection while this transaction holds one
    // (self-starves a max_connections=1 deployment until acquire timeout).
    let validated_targets = admission
        .admit_on_tx(tx, admission_principals)
        .await
        .map_err(CreateWorkflowInstanceError::AdmissionFailed)?;
    let resolved_assignee_id = validated_targets
        .responsibility(resolved_assignee_id)
        .map_err(CreateWorkflowInstanceError::AdmissionFailed)?;

    // ---------------------------------------------------------------
    // Step 9: Insert WorkflowInstance
    // ---------------------------------------------------------------
    let workflow_state_version = 1i32;

    sqlx::query(
        r#"
        INSERT INTO workflow_instances
            (workflow_instance_id, domain_id, definition_version_id,
             created_by_principal_id, workflow_state_version,
             current_context_revision_id, current_node_visit_id,
             external_reference, external_url, metadata,
             semantic_model_version, execution_class)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12::workflow_execution_class)
        "#,
    )
    .bind(workflow_instance_id)
    .bind(domain_uuid)
    .bind(definition_version_uuid)
    .bind(principal_uuid)
    .bind(workflow_state_version)
    .bind(context_revision_id)
    .bind(node_visit_id)
    .bind(&cmd.external_reference)
    .bind(&cmd.external_url)
    .bind(&cmd.metadata)
    .bind(version_info.semantic_model_version)
    .bind(cmd.execution_class.as_str())
    .execute(&mut **tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // ---------------------------------------------------------------
    // Step 10: Insert WorkflowContextRevision #1
    // ---------------------------------------------------------------
    let revision_number = 1i32;
    let payload_digest = digest::compute_json_digest(&cmd.context_payload)
        .map_err(CreateWorkflowInstanceError::StorageError)?;

    sqlx::query(
        r#"
        INSERT INTO workflow_context_revisions
            (context_revision_id, workflow_instance_id, revision_number,
             previous_revision_id, payload, payload_digest,
             created_by_principal_id)
        VALUES ($1, $2, $3, NULL, $4, $5, $6)
        "#,
    )
    .bind(context_revision_id)
    .bind(workflow_instance_id)
    .bind(revision_number)
    .bind(&cmd.context_payload)
    .bind(&payload_digest)
    .bind(principal_uuid)
    .execute(&mut **tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // ---------------------------------------------------------------
    // Step 11: Insert NodeVisit #1
    // ---------------------------------------------------------------
    let visit_number = 1i32;

    sqlx::query(
        r#"
        INSERT INTO workflow_node_visits
            (node_visit_id, workflow_instance_id, node_id, visit_number,
             assignee_principal_id, entered_by_transition_id)
        VALUES ($1, $2, $3, $4, $5, NULL)
        "#,
    )
    .bind(node_visit_id)
    .bind(workflow_instance_id)
    .bind(entry_node.node_id)
    .bind(visit_number)
    .bind(resolved_assignee_id)
    .execute(&mut **tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // ---------------------------------------------------------------
    // Step 11b: Canonical activation for VISIT_ACTIVATION_V1 (same tx)
    // ---------------------------------------------------------------
    if version_info.semantic_model_version == 3 {
        // Kind derives only from the resolved canonical Principal type,
        // never from a caller field or node name.
        let owner_type: (String,) =
            sqlx::query_as("SELECT principal_type::text FROM principals WHERE principal_id = $1")
                .bind(resolved_assignee_id)
                .fetch_one(&mut **tx)
                .await
                .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        let activation_kind = if owner_type.0 == "AGENT" {
            crate::domain::enums::ActivationKind::DispatchIntent
        } else {
            crate::domain::enums::ActivationKind::HumanWorkItem
        };
        activation_facts::insert_activation(
            tx,
            activation_id,
            workflow_instance_id,
            node_visit_id,
            activation_kind,
            resolved_assignee_id,
            actual_command_id,
        )
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        // CTR-SWEC-007: push-first kick for a first DISPATCH_INTENT.
        if activation_kind == crate::domain::enums::ActivationKind::DispatchIntent
            && cmd.execution_class != WorkflowExecutionClass::NonBusinessTest
        {
            crate::store::postgres::outbox::queue_execution_kick(
                tx,
                workflow_instance_id,
                node_visit_id,
                activation_id,
            )
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        }
    }

    // ---------------------------------------------------------------
    // Step 12: Insert INSTANCE_CREATED WorkflowEvent #1
    // ---------------------------------------------------------------
    let event_sequence = 1i32;
    let definition_digest_str = version_info.definition_digest.as_deref().unwrap_or("");

    let event_data = InstanceCreatedEventData {
        definition_version_id: definition_version_uuid.to_string(),
        definition_digest: definition_digest_str.to_string(),
        initial_node_id: entry_node.node_id.to_string(),
        assignee_resolution_type: entry_node.assignee_ref_type.to_string(),
    };

    let event_data_json = serde_json::to_value(&event_data)
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
    let event_data_digest = digest::compute_json_digest(&event_data_json)
        .map_err(CreateWorkflowInstanceError::StorageError)?;

    sqlx::query(
        r#"
        INSERT INTO workflow_events
            (event_id, workflow_instance_id, event_sequence, event_schema_version,
             command_id, event_type, source_node_visit_id, target_node_visit_id,
             context_revision_id, event_data, event_data_digest,
             actor_principal_id, old_workflow_state_version, new_workflow_state_version)
        VALUES ($1, $2, $3, $4, $5, $6, NULL, $7, $8, $9, $10, $11, 0, 1)
        "#,
    )
    .bind(event_id)
    .bind(workflow_instance_id)
    .bind(event_sequence)
    .bind(EVENT_SCHEMA_VERSION)
    .bind(actual_command_id)
    .bind(INSTANCE_CREATED_EVENT_TYPE)
    .bind(node_visit_id)
    .bind(context_revision_id)
    .bind(&event_data_json)
    .bind(&event_data_digest)
    .bind(principal_uuid)
    .execute(&mut **tx)
    .await
    .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;

    // ---------------------------------------------------------------
    // Step 12b: WORKFLOW_EXECUTION_CONTROL_V1 outbox facts
    // (CTR-SWEC-001/002). BUSINESS instances get the PENDING canonical
    // forum binding + the workflow_created forum event. Non-BUSINESS
    // (canary) instances get nothing — no forum surface for canaries.
    // ---------------------------------------------------------------
    if cmd.execution_class != WorkflowExecutionClass::NonBusinessTest {
        crate::store::postgres::outbox::queue_forum_binding(tx, workflow_instance_id)
            .await
            .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
        crate::store::postgres::outbox::queue_forum_event(
            tx,
            workflow_instance_id,
            &format!("wf_created:{workflow_instance_id}"),
            serde_json::json!({
                "eventType": "workflow_created",
                "workflowInstanceId": workflow_instance_id,
                "definitionVersionId": definition_version_uuid,
                "initialNodeId": entry_node.node_id,
            }),
        )
        .await
        .map_err(|e| CreateWorkflowInstanceError::StorageError(e.to_string()))?;
    }

    // CTR-SWPO-009: retain the single locked admission result, bound to the
    // actual initial facts inserted above. No post-create readiness read.
    let admission_observation = prerequisite_readiness.as_ref().map(|readiness| {
        let mut observation = serde_json::json!({
            "version": 1, "kind": "TERMINAL_SUBMISSION", "status": readiness["status"],
            "dependentInstanceId": workflow_instance_id,
            "dependentDefinitionVersionId": definition_version_uuid,
            "initialContextRevisionId": context_revision_id,
            "initialContextSha256": payload_digest,
            "initialVisitId": node_visit_id, "birthEventId": event_id,
            "sourceInstanceId": readiness["sourceInstanceId"],
            "sourceDefinitionVersionId": readiness["sourceDefinitionVersionId"],
            "transitionId": readiness["transitionId"],
        });
        if readiness["status"] == "READY" {
            for key in ["eventId", "submissionId", "submissionSha256"] {
                observation[key] = readiness[key].clone();
            }
        }
        observation
    });
    Ok(CreateResult {
        admission_observation,
        prerequisite_readiness,
        workflow_instance_id,
        workflow_state_version,
        current_context_revision_id: context_revision_id,
        current_node_visit_id: node_visit_id,
        event_sequence,
    })
}
