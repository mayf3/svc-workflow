-- Owner-obligation V1: five relations, two named command extensions only.
LOCK TABLE workflow_command_receipts,workflow_events IN ACCESS EXCLUSIVE MODE;
CREATE TABLE workflow_owner_policy_approvals (
 source_definition_version_id UUID PRIMARY KEY REFERENCES workflow_definition_versions,
 policy_digest TEXT NOT NULL CHECK(policy_digest ~ '^[0-9a-f]{64}$'),
 publish_command_id UUID NOT NULL UNIQUE REFERENCES workflow_command_receipts,
 approved_by_principal_id UUID NOT NULL REFERENCES principals,
 configured_policy_actor_principal_id UUID NOT NULL REFERENCES principals,
 domain_id UUID NOT NULL REFERENCES domains,
 source_definition_digest TEXT NOT NULL,
 intake_definition_version_id UUID NOT NULL REFERENCES workflow_definition_versions,
 review_definition_version_id UUID NOT NULL REFERENCES workflow_definition_versions,
 intake_acceptance_transition_id UUID NOT NULL REFERENCES workflow_transition_definitions,
 review_completion_transition_id UUID NOT NULL REFERENCES workflow_transition_definitions,
 intake_target_version_digest TEXT NOT NULL,intake_graph_digest TEXT NOT NULL,intake_schema_digest TEXT NOT NULL,
 review_target_version_digest TEXT NOT NULL,review_graph_digest TEXT NOT NULL,review_schema_digest TEXT NOT NULL,
 owner_replacement_command_id UUID REFERENCES workflow_command_receipts,
 published_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE TABLE workflow_owner_obligation_intents (
 intent_id UUID PRIMARY KEY,kind TEXT NOT NULL CHECK(kind IN ('INTAKE','REVIEW','STOP')),
 case_id UUID NOT NULL REFERENCES workflow_assistance_cases,
 source_instance_id UUID NOT NULL REFERENCES workflow_instances,
 source_node_visit_id UUID NOT NULL REFERENCES workflow_node_visits,
 request_event_id UUID NOT NULL REFERENCES workflow_events,
 request_context_revision_id UUID NOT NULL REFERENCES workflow_context_revisions,
 source_definition_version_id UUID NOT NULL REFERENCES workflow_definition_versions,
 request_digest TEXT NOT NULL,policy_digest TEXT NOT NULL,
 policy_source_version_id UUID NOT NULL REFERENCES workflow_owner_policy_approvals,
 selected_definition_version_id UUID REFERENCES workflow_definition_versions,
 selected_transition_id UUID REFERENCES workflow_transition_definitions,
 binding_id UUID REFERENCES workflow_governance_child_bindings,
 child_terminal_event_id UUID REFERENCES workflow_events,
 managed_instance_id UUID REFERENCES workflow_instances,
 state TEXT NOT NULL CHECK(state IN ('PENDING','MATERIALIZED','BLOCKED_KNOWN','STOPPED','RETIRED','COMPLETED','UNKNOWN')),
 outcome TEXT,last_command_id UUID REFERENCES workflow_command_receipts,
 owner_replacement_command_id UUID REFERENCES workflow_command_receipts,
 node_visit_id UUID REFERENCES workflow_node_visits,last_attempt_id TEXT,
 origin_event_id UUID REFERENCES workflow_events,stop_code TEXT,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 CHECK(kind<>'REVIEW' OR (binding_id IS NOT NULL AND child_terminal_event_id IS NOT NULL)),
 CHECK(state<>'STOPPED' OR stop_code IS NOT NULL)
);
CREATE UNIQUE INDEX owner_intake_case ON workflow_owner_obligation_intents(case_id) WHERE kind='INTAKE';
CREATE UNIQUE INDEX owner_review_terminal ON workflow_owner_obligation_intents(binding_id,child_terminal_event_id) WHERE kind='REVIEW';
CREATE UNIQUE INDEX owner_stop_replacement ON workflow_owner_obligation_intents(managed_instance_id,owner_replacement_command_id,stop_code) WHERE kind='STOP' AND owner_replacement_command_id IS NOT NULL;
CREATE UNIQUE INDEX owner_stop_attempt ON workflow_owner_obligation_intents(managed_instance_id,node_visit_id,last_attempt_id,stop_code) WHERE kind='STOP' AND last_attempt_id IS NOT NULL;
CREATE UNIQUE INDEX owner_stop_event ON workflow_owner_obligation_intents(managed_instance_id,origin_event_id,stop_code) WHERE kind='STOP' AND origin_event_id IS NOT NULL;
CREATE TABLE workflow_owner_managed_instances (
 workflow_instance_id UUID PRIMARY KEY REFERENCES workflow_instances,
 kind TEXT NOT NULL CHECK(kind IN ('INTAKE','REVIEW')),
 case_id UUID NOT NULL REFERENCES workflow_assistance_cases,
 policy_source_version_id UUID NOT NULL REFERENCES workflow_owner_policy_approvals,
 policy_digest TEXT NOT NULL,binding_id UUID NOT NULL REFERENCES workflow_governance_child_bindings,
 child_terminal_event_id UUID REFERENCES workflow_events,
 creation_command_id UUID NOT NULL UNIQUE REFERENCES workflow_command_receipts,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
 CHECK((kind='REVIEW')=(child_terminal_event_id IS NOT NULL))
);
CREATE UNIQUE INDEX managed_intake_case ON workflow_owner_managed_instances(case_id) WHERE kind='INTAKE';
CREATE UNIQUE INDEX managed_review_terminal ON workflow_owner_managed_instances(binding_id,child_terminal_event_id) WHERE kind='REVIEW';
CREATE TABLE workflow_operation_effects (
 effect_id UUID PRIMARY KEY,case_id UUID NOT NULL REFERENCES workflow_assistance_cases,
 managed_instance_id UUID NOT NULL REFERENCES workflow_owner_managed_instances,
 definition_version_id UUID NOT NULL REFERENCES workflow_definition_versions,
 node_definition_id UUID NOT NULL REFERENCES workflow_node_definitions,
 slot_key TEXT NOT NULL CHECK(length(slot_key)>0),logical_occurrence INTEGER NOT NULL DEFAULT 1 CHECK(logical_occurrence=1),
 descriptor_key TEXT NOT NULL,descriptor_version INTEGER NOT NULL,descriptor_hash TEXT NOT NULL,
 definition_graph_digest TEXT NOT NULL,first_authorizing_visit_id UUID NOT NULL REFERENCES workflow_node_visits,
 actor_principal_id UUID NOT NULL REFERENCES principals,business_intent_digest TEXT NOT NULL,concrete_request_digest TEXT NOT NULL,
 state TEXT NOT NULL CHECK(state IN ('PREPARED','STARTED','SUCCEEDED','REJECTED_ZERO_FACTS','UNKNOWN')),
 command_id UUID UNIQUE REFERENCES workflow_command_receipts,result JSONB,result_digest TEXT,failure JSONB,last_outcome_code TEXT,
 UNIQUE(case_id,managed_instance_id,definition_version_id,node_definition_id,slot_key,logical_occurrence),
 CHECK(state<>'SUCCEEDED' OR (command_id IS NOT NULL AND result IS NOT NULL AND result_digest IS NOT NULL)),
 CHECK(state<>'REJECTED_ZERO_FACTS' OR (command_id IS NOT NULL AND failure IS NOT NULL))
);
CREATE TABLE workflow_owner_dependency_origins (
 created_instance_id UUID PRIMARY KEY REFERENCES workflow_instances,
 parent_effect_id UUID NOT NULL UNIQUE REFERENCES workflow_operation_effects,
 parent_managed_instance_id UUID NOT NULL REFERENCES workflow_owner_managed_instances,
 case_id UUID NOT NULL REFERENCES workflow_assistance_cases,
 create_command_id UUID NOT NULL UNIQUE REFERENCES workflow_command_receipts,
 target_definition_version_id UUID NOT NULL REFERENCES workflow_definition_versions,
 created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE FUNCTION owner_fact_immutable() RETURNS TRIGGER LANGUAGE plpgsql AS $$ BEGIN
 IF TG_OP='DELETE' OR TG_TABLE_NAME IN ('workflow_owner_policy_approvals','workflow_owner_managed_instances','workflow_owner_dependency_origins')
 OR (TG_TABLE_NAME='workflow_operation_effects' AND (to_jsonb(OLD)->>'state') IN ('SUCCEEDED','REJECTED_ZERO_FACTS')) THEN
 RAISE EXCEPTION 'owner obligation fact is immutable' USING ERRCODE='23514'; END IF;
 IF TG_TABLE_NAME='workflow_operation_effects' AND
 (to_jsonb(NEW)-ARRAY['state','command_id','result','result_digest','failure','last_outcome_code']) IS DISTINCT FROM
 (to_jsonb(OLD)-ARRAY['state','command_id','result','result_digest','failure','last_outcome_code']) THEN
 RAISE EXCEPTION 'semantic effect identity and input immutable' USING ERRCODE='23514'; END IF;
 IF TG_TABLE_NAME='workflow_owner_obligation_intents' AND (to_jsonb(OLD)->>'state') IN ('STOPPED','RETIRED','COMPLETED') AND NEW IS DISTINCT FROM OLD THEN
 RAISE EXCEPTION 'stopped obligation cannot restart' USING ERRCODE='23514'; END IF;
 IF TG_TABLE_NAME='workflow_owner_obligation_intents' THEN
 IF (to_jsonb(NEW)-ARRAY['state','outcome','last_command_id','owner_replacement_command_id','node_visit_id','last_attempt_id','origin_event_id','stop_code','updated_at','binding_id','managed_instance_id']) IS DISTINCT FROM (to_jsonb(OLD)-ARRAY['state','outcome','last_command_id','owner_replacement_command_id','node_visit_id','last_attempt_id','origin_event_id','stop_code','updated_at','binding_id','managed_instance_id'])
 OR ((to_jsonb(OLD)->>'binding_id') IS NOT NULL AND (to_jsonb(NEW)->>'binding_id') IS DISTINCT FROM (to_jsonb(OLD)->>'binding_id'))
 OR ((to_jsonb(OLD)->>'managed_instance_id') IS NOT NULL AND (to_jsonb(NEW)->>'managed_instance_id') IS DISTINCT FROM (to_jsonb(OLD)->>'managed_instance_id'))
 OR ((to_jsonb(OLD)->>'state') IN ('MATERIALIZED','BLOCKED_KNOWN','UNKNOWN') AND (to_jsonb(NEW)->>'state')='PENDING') THEN
 RAISE EXCEPTION 'responsibility identity cannot be reset' USING ERRCODE='23514'; END IF; END IF;
 RETURN NEW; END $$;
CREATE TRIGGER owner_policy_immutable BEFORE UPDATE OR DELETE ON workflow_owner_policy_approvals FOR EACH ROW EXECUTE FUNCTION owner_fact_immutable();
CREATE TRIGGER owner_managed_immutable BEFORE UPDATE OR DELETE ON workflow_owner_managed_instances FOR EACH ROW EXECUTE FUNCTION owner_fact_immutable();
CREATE TRIGGER owner_origin_immutable BEFORE UPDATE OR DELETE ON workflow_owner_dependency_origins FOR EACH ROW EXECUTE FUNCTION owner_fact_immutable();
CREATE TRIGGER owner_effect_immutable BEFORE UPDATE OR DELETE ON workflow_operation_effects FOR EACH ROW EXECUTE FUNCTION owner_fact_immutable();
CREATE TRIGGER owner_intent_immutable BEFORE UPDATE OR DELETE ON workflow_owner_obligation_intents FOR EACH ROW EXECUTE FUNCTION owner_fact_immutable();
CREATE OR REPLACE FUNCTION assert_command_event_integrity(cmd UUID) RETURNS VOID LANGUAGE plpgsql AS $$
DECLARE
    r workflow_command_receipts%ROWTYPE;
    birth workflow_events%ROWTYPE;
    bound workflow_events%ROWTYPE;
    child workflow_instances%ROWTYPE;
    source workflow_instances%ROWTYPE;
    g workflow_governance_child_bindings%ROWTYPE;
    total BIGINT;
    facts BOOLEAN;
BEGIN
    IF cmd IS NULL THEN RETURN; END IF;
    -- Actual identity lock, before reading cardinality. Native slot uniqueness,
    -- not this snapshot's COUNT alone, also enforces the concurrency invariant.
    SELECT * INTO r FROM workflow_command_receipts WHERE command_id=cmd FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'event command receipt missing' USING ERRCODE='23514'; END IF;
    SELECT count(*) INTO total FROM workflow_events WHERE command_id=cmd;
    IF r.command_type='AUTO_CREATE_OWNER_REVIEW' THEN
        IF r.receipt_status='COMPLETED' AND r.response_status BETWEEN 200 AND 299 THEN
            IF total<>1 OR NOT EXISTS(SELECT 1 FROM workflow_events e JOIN workflow_owner_managed_instances m ON m.workflow_instance_id=e.workflow_instance_id AND m.creation_command_id=e.command_id AND m.kind='REVIEW' WHERE e.command_id=cmd AND e.event_type='INSTANCE_CREATED' AND e.actor_principal_id=r.principal_id AND e.old_workflow_state_version=0 AND e.new_workflow_state_version=1 AND r.response_body->>'workflowInstanceId'=m.workflow_instance_id::text) THEN
                RAISE EXCEPTION 'successful auto review requires one actual managed birth' USING ERRCODE='23514';
            END IF;
        ELSIF total<>0 OR EXISTS(SELECT 1 FROM workflow_owner_managed_instances WHERE creation_command_id=cmd) THEN
            RAISE EXCEPTION 'unsuccessful auto review has business facts' USING ERRCODE='23514';
        END IF;
        RETURN;
    END IF;
    IF r.command_type NOT IN ('CREATE_AND_BIND_GOVERNANCE_CHILD','AUTO_CREATE_AND_BIND_OWNER_INTAKE','ONE_TIME_MIGRATE') THEN
        IF total>1 THEN RAISE EXCEPTION 'ordinary command permits at most one event' USING ERRCODE='23514'; END IF;
        RETURN;
    END IF;
    SELECT EXISTS(SELECT 1 FROM workflow_activations WHERE command_id=cmd)
        OR EXISTS(SELECT 1 FROM workflow_activation_closures WHERE command_id=cmd)
        OR EXISTS(SELECT 1 FROM workflow_dispatch_eligibility_events WHERE command_id=cmd)
        OR EXISTS(SELECT 1 FROM workflow_governance_child_bindings WHERE binding_command_id=cmd)
        INTO facts;
    IF r.receipt_status='COMPLETED' AND r.response_status IS NULL THEN
        RAISE EXCEPTION 'completed composite command requires an explicit HTTP status' USING ERRCODE='23514';
    END IF;
    IF r.receipt_status<>'COMPLETED' OR r.response_status NOT BETWEEN 200 AND 299 THEN
        IF total<>0 OR facts THEN
            RAISE EXCEPTION 'unsuccessful composite command must have zero business facts' USING ERRCODE='23514';
        END IF;
        RETURN;
    END IF;
    IF total<>2 THEN RAISE EXCEPTION 'successful composite command requires exactly two events' USING ERRCODE='23514'; END IF;
    SELECT * INTO birth FROM workflow_events WHERE command_id=cmd AND event_type='INSTANCE_CREATED';
    IF NOT FOUND OR birth.old_workflow_state_version<>0 OR birth.new_workflow_state_version<>1 OR birth.event_sequence<>1 THEN
        RAISE EXCEPTION 'composite birth event invalid' USING ERRCODE='23514';
    END IF;
    SELECT * INTO child FROM workflow_instances WHERE workflow_instance_id=birth.workflow_instance_id;
    SELECT * INTO bound FROM workflow_events WHERE command_id=cmd AND event_id<>birth.event_id;
    SELECT * INTO source FROM workflow_instances WHERE workflow_instance_id=bound.workflow_instance_id;
    IF child.workflow_instance_id=source.workflow_instance_id OR child.domain_id<>source.domain_id
       OR birth.actor_principal_id<>r.principal_id OR bound.actor_principal_id<>r.principal_id
       OR bound.new_workflow_state_version<>bound.old_workflow_state_version+1
       OR NOT EXISTS(SELECT 1 FROM workflow_node_visits v WHERE v.node_visit_id=birth.target_node_visit_id AND v.workflow_instance_id=child.workflow_instance_id AND v.visit_number=1)
       OR NOT EXISTS(SELECT 1 FROM workflow_context_revisions c WHERE c.context_revision_id=birth.context_revision_id AND c.workflow_instance_id=child.workflow_instance_id AND c.revision_number=1) THEN
        RAISE EXCEPTION 'composite event Instance/actor/birth lineage invalid' USING ERRCODE='23514';
    END IF;
    IF r.command_type='ONE_TIME_MIGRATE' THEN
        -- Preserve the separately reserved Legacy -> VAI source/successor scope.
        -- Its writer/event vocabulary remains deferred to its own cutover Spec.
        IF source.semantic_model_version=3 OR child.semantic_model_version<>3 THEN
            RAISE EXCEPTION 'ONE_TIME_MIGRATE must retain Legacy to VAI scope' USING ERRCODE='23514';
        END IF;
        RETURN;
    END IF;
    SELECT * INTO g FROM workflow_governance_child_bindings WHERE binding_command_id=cmd;
    IF NOT FOUND OR (SELECT count(*) FROM workflow_governance_child_bindings WHERE binding_command_id=cmd)<>1
       OR bound.event_type<>'GOVERNANCE_CHILD_BOUND'
       OR g.source_instance_id<>source.workflow_instance_id OR g.governance_child_instance_id<>child.workflow_instance_id
       OR g.binding_event_id<>bound.event_id OR g.binding_principal_id<>r.principal_id
       OR g.child_definition_version_id<>child.definition_version_id
       OR g.child_context_revision_at_bind<>birth.context_revision_id
       OR g.source_node_visit_id IS DISTINCT FROM bound.source_node_visit_id
       OR bound.source_node_visit_id IS DISTINCT FROM bound.target_node_visit_id
       OR bound.event_data->'governanceBinding' IS DISTINCT FROM g.receipt
       OR (r.response_body-'prerequisiteReadiness') IS DISTINCT FROM g.receipt
       OR (r.response_body->>'bindingId') IS DISTINCT FROM g.binding_id::text THEN
        RAISE EXCEPTION 'composite binding/response/event lineage invalid' USING ERRCODE='23514';
    END IF;
    IF r.command_type='AUTO_CREATE_AND_BIND_OWNER_INTAKE' AND NOT EXISTS(SELECT 1 FROM workflow_owner_managed_instances m WHERE m.creation_command_id=cmd AND m.workflow_instance_id=child.workflow_instance_id AND m.kind='INTAKE' AND m.binding_id=g.binding_id) THEN RAISE EXCEPTION 'auto intake managed relation missing' USING ERRCODE='23514'; END IF;
    IF child.semantic_model_version=3 AND NOT EXISTS(SELECT 1 FROM workflow_activations a WHERE a.command_id=cmd AND a.workflow_instance_id=child.workflow_instance_id AND a.node_visit_id=birth.target_node_visit_id) THEN
        RAISE EXCEPTION 'composite initial activation missing' USING ERRCODE='23514';
    END IF;
END $$;

CREATE OR REPLACE FUNCTION derive_command_event_slot() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE kind TEXT;
BEGIN
    NEW.command_event_slot:=0;
    IF NEW.command_id IS NULL THEN RETURN NEW; END IF;
    SELECT command_type INTO kind FROM workflow_command_receipts WHERE command_id=NEW.command_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'actual command receipt required before event' USING ERRCODE='23514'; END IF;
    IF kind IN ('CREATE_AND_BIND_GOVERNANCE_CHILD','AUTO_CREATE_AND_BIND_OWNER_INTAKE') THEN
        IF NEW.event_type='INSTANCE_CREATED' THEN NEW.command_event_slot:=1;
        ELSIF NEW.event_type='GOVERNANCE_CHILD_BOUND' THEN NEW.command_event_slot:=2;
        ELSE RAISE EXCEPTION 'unsupported composite event type' USING ERRCODE='23514'; END IF;
    ELSIF kind='ONE_TIME_MIGRATE' THEN
        NEW.command_event_slot:=CASE WHEN NEW.event_type='INSTANCE_CREATED' THEN 1 ELSE 2 END;
    END IF;
    RETURN NEW;
END $$;
CREATE OR REPLACE FUNCTION check_command_event_integrity() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME='workflow_governance_child_bindings' THEN
        PERFORM assert_command_event_integrity(NEW.binding_command_id);
    ELSIF TG_TABLE_NAME IN ('workflow_activations','workflow_activation_closures','workflow_dispatch_eligibility_events') THEN
        IF EXISTS(SELECT 1 FROM workflow_command_receipts WHERE command_id=NEW.command_id AND command_type IN ('CREATE_AND_BIND_GOVERNANCE_CHILD','AUTO_CREATE_AND_BIND_OWNER_INTAKE','ONE_TIME_MIGRATE')) THEN
            PERFORM assert_command_event_integrity(NEW.command_id);
        END IF;
    ELSE
        PERFORM assert_command_event_integrity(NEW.command_id);
    END IF;
    RETURN NULL;
END $$;

CREATE FUNCTION owner_facts_complete() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE r workflow_command_receipts%ROWTYPE; wi workflow_instances%ROWTYPE; e workflow_operation_effects%ROWTYPE;
BEGIN
 IF TG_TABLE_NAME='workflow_operation_effects' THEN
 IF NOT EXISTS(SELECT 1 FROM workflow_owner_managed_instances m JOIN workflow_owner_policy_approvals p ON p.source_definition_version_id=m.policy_source_version_id JOIN workflow_instances w ON w.workflow_instance_id=m.workflow_instance_id JOIN workflow_node_definitions n ON n.definition_version_id=w.definition_version_id JOIN workflow_node_visits v ON v.workflow_instance_id=w.workflow_instance_id AND v.node_id=n.node_id
 WHERE m.workflow_instance_id=NEW.managed_instance_id AND m.case_id=NEW.case_id AND w.definition_version_id=NEW.definition_version_id AND n.node_id=NEW.node_definition_id AND v.node_visit_id=NEW.first_authorizing_visit_id AND v.assignee_principal_id=NEW.actor_principal_id
 AND NEW.definition_graph_digest=CASE WHEN m.kind='INTAKE' THEN p.intake_graph_digest ELSE p.review_graph_digest END
 AND n.metadata->'workflowOperation'->>'slotKey'=NEW.slot_key AND (n.metadata->'workflowOperation'->'allowedOperations') @> jsonb_build_array(jsonb_build_object('key',NEW.descriptor_key,'version',NEW.descriptor_version,'hash',NEW.descriptor_hash))) THEN RAISE EXCEPTION 'effect published node/visit/descriptor lineage mismatch' USING ERRCODE='23514'; END IF;
 END IF;
 IF TG_TABLE_NAME='workflow_owner_obligation_intents' THEN
 IF NOT EXISTS(SELECT 1 FROM workflow_assistance_cases c JOIN workflow_instances w ON w.workflow_instance_id=c.workflow_instance_id JOIN workflow_events x ON x.command_id=c.request_command_id AND x.event_type='ASSISTANCE_REQUESTED' JOIN workflow_command_receipts rc ON rc.command_id=c.request_command_id JOIN workflow_owner_policy_approvals p ON p.source_definition_version_id=w.definition_version_id
 WHERE c.assistance_case_id=NEW.case_id AND w.workflow_instance_id=NEW.source_instance_id AND w.definition_version_id=NEW.source_definition_version_id AND c.node_visit_id=NEW.source_node_visit_id AND c.request_payload_digest=NEW.request_digest AND x.event_id=NEW.request_event_id AND x.workflow_instance_id=w.workflow_instance_id AND x.context_revision_id=NEW.request_context_revision_id AND x.source_node_visit_id=c.node_visit_id
 AND rc.command_type='REQUEST_WORKFLOW_ASSISTANCE' AND rc.receipt_status='COMPLETED' AND rc.response_status BETWEEN 200 AND 299 AND rc.principal_id=c.requested_by_principal_id AND x.actor_principal_id=rc.principal_id AND p.source_definition_version_id=NEW.policy_source_version_id AND p.policy_digest=NEW.policy_digest AND p.domain_id=w.domain_id
 AND (NEW.kind='STOP' OR (NEW.kind='INTAKE' AND NEW.selected_definition_version_id=p.intake_definition_version_id AND NEW.selected_transition_id=p.intake_acceptance_transition_id) OR (NEW.kind='REVIEW' AND NEW.selected_definition_version_id=p.review_definition_version_id AND NEW.selected_transition_id=p.review_completion_transition_id AND EXISTS(SELECT 1 FROM workflow_governance_child_bindings b WHERE b.binding_id=NEW.binding_id AND b.assistance_case_id=c.assistance_case_id AND b.terminal_event_id=NEW.child_terminal_event_id)))) THEN RAISE EXCEPTION 'responsibility request/policy lineage mismatch' USING ERRCODE='23514'; END IF;
 IF NEW.kind IN ('INTAKE','REVIEW') AND NEW.state IN ('MATERIALIZED','COMPLETED') AND NOT EXISTS(SELECT 1 FROM workflow_owner_managed_instances m WHERE m.workflow_instance_id=NEW.managed_instance_id AND m.case_id=NEW.case_id AND m.kind=NEW.kind AND m.binding_id=NEW.binding_id AND m.policy_source_version_id=NEW.policy_source_version_id AND m.policy_digest=NEW.policy_digest) THEN RAISE EXCEPTION 'materialized responsibility relation missing' USING ERRCODE='23514'; END IF;
 ELSIF TG_TABLE_NAME='workflow_owner_policy_approvals' THEN
 SELECT * INTO r FROM workflow_command_receipts WHERE command_id=NEW.publish_command_id FOR UPDATE;
 IF r.command_type<>'DEFINITION_PUBLISH' OR r.principal_id<>NEW.approved_by_principal_id OR r.receipt_status<>'COMPLETED' OR r.response_status NOT BETWEEN 200 AND 299
 OR NOT EXISTS(SELECT 1 FROM workflow_definition_versions v JOIN workflow_definitions d USING(workflow_definition_id) WHERE v.definition_version_id=NEW.source_definition_version_id AND v.version_status='PUBLISHED' AND v.published_by_principal_id=NEW.approved_by_principal_id AND v.definition_digest=NEW.source_definition_digest AND d.domain_id=NEW.domain_id) THEN RAISE EXCEPTION 'policy approval receipt/version mismatch' USING ERRCODE='23514'; END IF;
 ELSIF TG_TABLE_NAME='workflow_owner_managed_instances' THEN
 SELECT * INTO r FROM workflow_command_receipts WHERE command_id=NEW.creation_command_id FOR UPDATE;
 SELECT * INTO wi FROM workflow_instances WHERE workflow_instance_id=NEW.workflow_instance_id;
 IF r.receipt_status<>'COMPLETED' OR r.response_status NOT BETWEEN 200 AND 299 OR r.command_type<>(CASE WHEN NEW.kind='INTAKE' THEN 'AUTO_CREATE_AND_BIND_OWNER_INTAKE' ELSE 'AUTO_CREATE_OWNER_REVIEW' END)
 OR NOT EXISTS(SELECT 1 FROM workflow_owner_policy_approvals p WHERE p.source_definition_version_id=NEW.policy_source_version_id AND p.policy_digest=NEW.policy_digest AND p.configured_policy_actor_principal_id=r.principal_id AND p.domain_id=wi.domain_id AND wi.definition_version_id=CASE WHEN NEW.kind='INTAKE' THEN p.intake_definition_version_id ELSE p.review_definition_version_id END)
 OR NOT EXISTS(SELECT 1 FROM workflow_events x WHERE x.command_id=r.command_id AND x.workflow_instance_id=wi.workflow_instance_id AND x.event_type='INSTANCE_CREATED') THEN RAISE EXCEPTION 'managed birth lineage mismatch' USING ERRCODE='23514'; END IF;
 ELSIF TG_TABLE_NAME='workflow_operation_effects' AND (to_jsonb(NEW)->>'state') IN ('SUCCEEDED','REJECTED_ZERO_FACTS') THEN
 SELECT * INTO r FROM workflow_command_receipts WHERE command_id=NEW.command_id FOR UPDATE;
 IF r.command_type IS DISTINCT FROM (CASE NEW.descriptor_key WHEN 'workflow.create_instance' THEN 'CREATE_WORKFLOW_INSTANCE' WHEN 'workflow.assistance.resolve_current_review' THEN 'RESOLVE_WORKFLOW_ASSISTANCE' WHEN 'workflow.assistance.escalate_to_human' THEN 'ESCALATE_WORKFLOW_ASSISTANCE_TO_HUMAN' END) OR r.principal_id<>NEW.actor_principal_id OR r.receipt_status<>'COMPLETED' OR ((to_jsonb(NEW)->>'state')='SUCCEEDED') IS DISTINCT FROM (r.response_status BETWEEN 200 AND 299)
 OR ((to_jsonb(NEW)->>'state')='REJECTED_ZERO_FACTS' AND EXISTS(SELECT 1 FROM workflow_events x WHERE x.command_id=r.command_id))
 OR ((to_jsonb(NEW)->>'state')='SUCCEEDED' AND (r.response_body IS DISTINCT FROM NEW.result OR r.response_digest IS DISTINCT FROM NEW.result_digest)) THEN RAISE EXCEPTION 'effect actual receipt mismatch' USING ERRCODE='23514'; END IF;
 ELSIF TG_TABLE_NAME='workflow_owner_dependency_origins' THEN
 SELECT * INTO e FROM workflow_operation_effects WHERE effect_id=NEW.parent_effect_id;
 SELECT * INTO wi FROM workflow_instances WHERE workflow_instance_id=NEW.created_instance_id;
 SELECT * INTO r FROM workflow_command_receipts WHERE command_id=NEW.create_command_id FOR UPDATE;
 IF e.state<>'SUCCEEDED' OR e.descriptor_key<>'workflow.create_instance' OR e.command_id<>r.command_id OR e.managed_instance_id<>NEW.parent_managed_instance_id OR e.case_id<>NEW.case_id OR r.command_type<>'CREATE_WORKFLOW_INSTANCE'
 OR wi.definition_version_id<>NEW.target_definition_version_id OR (r.response_body->>'workflowInstanceId') IS DISTINCT FROM NEW.created_instance_id::text
 OR r.receipt_status<>'COMPLETED' OR r.response_status NOT BETWEEN 200 AND 299 OR r.principal_id<>e.actor_principal_id
 OR NOT EXISTS(SELECT 1 FROM workflow_events x WHERE x.command_id=r.command_id AND x.workflow_instance_id=wi.workflow_instance_id AND x.event_type='INSTANCE_CREATED' AND x.actor_principal_id=r.principal_id AND x.old_workflow_state_version=0 AND x.new_workflow_state_version=1)
 OR NOT EXISTS(SELECT 1 FROM workflow_instances parent JOIN workflow_definition_versions v ON v.definition_version_id=wi.definition_version_id WHERE parent.workflow_instance_id=NEW.parent_managed_instance_id AND parent.domain_id=wi.domain_id AND v.version_status='PUBLISHED' AND NOT (v.context_schema ? 'x-workflow-owner-obligation-role-v1') AND NOT (v.context_schema ? 'x-workflow-owner-obligation-v1'))
 THEN RAISE EXCEPTION 'dependency birth/effect mismatch' USING ERRCODE='23514'; END IF;
 END IF;
 RETURN NULL; END $$;
CREATE CONSTRAINT TRIGGER owner_policy_complete AFTER INSERT ON workflow_owner_policy_approvals DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION owner_facts_complete();
CREATE CONSTRAINT TRIGGER owner_managed_complete AFTER INSERT ON workflow_owner_managed_instances DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION owner_facts_complete();
CREATE CONSTRAINT TRIGGER owner_effect_complete AFTER INSERT OR UPDATE ON workflow_operation_effects DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION owner_facts_complete();
CREATE CONSTRAINT TRIGGER owner_origin_complete AFTER INSERT ON workflow_owner_dependency_origins DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION owner_facts_complete();

CREATE CONSTRAINT TRIGGER owner_intent_complete AFTER INSERT OR UPDATE ON workflow_owner_obligation_intents DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION owner_facts_complete();
