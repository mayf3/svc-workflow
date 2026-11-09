-- CTR-SWPO-007 / Storage V0.2. One real receipt, exact cross-Instance pair.
-- Constant default is metadata-only; no historical business row is UPDATEd.
-- All ordinary commands share slot 0 regardless of event type. Native UNIQUE
-- protects this across RC, RR and SERIALIZABLE, including stale snapshots.
LOCK TABLE workflow_command_receipts IN ACCESS EXCLUSIVE MODE;
LOCK TABLE workflow_events IN ACCESS EXCLUSIVE MODE;
ALTER TABLE workflow_events ADD COLUMN command_event_slot SMALLINT NOT NULL DEFAULT 0
    CHECK (command_event_slot BETWEEN 0 AND 2);

CREATE FUNCTION assert_command_event_integrity(cmd UUID) RETURNS VOID LANGUAGE plpgsql AS $$
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
    IF r.command_type NOT IN ('CREATE_AND_BIND_GOVERNANCE_CHILD','ONE_TIME_MIGRATE') THEN
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
    IF child.semantic_model_version=3 AND NOT EXISTS(SELECT 1 FROM workflow_activations a WHERE a.command_id=cmd AND a.workflow_instance_id=child.workflow_instance_id AND a.node_visit_id=birth.target_node_visit_id) THEN
        RAISE EXCEPTION 'composite initial activation missing' USING ERRCODE='23514';
    END IF;
END $$;

-- Validate historical rows while the former global invariant still exists.
-- No backfill, Event deletion or historical business-value rewrite is permitted.
DO $$ DECLARE c UUID; BEGIN
    IF EXISTS(SELECT 1 FROM workflow_events WHERE command_id IS NOT NULL GROUP BY command_id HAVING count(*)>1) THEN
        RAISE EXCEPTION 'existing global command/event history violates predecessor invariant' USING ERRCODE='23514';
    END IF;
    FOR c IN SELECT command_id FROM workflow_command_receipts
        WHERE command_type IN ('CREATE_AND_BIND_GOVERNANCE_CHILD','ONE_TIME_MIGRATE') LOOP
        PERFORM assert_command_event_integrity(c);
    END LOOP;
END $$;

CREATE FUNCTION derive_command_event_slot() RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE kind TEXT;
BEGIN
    NEW.command_event_slot:=0;
    IF NEW.command_id IS NULL THEN RETURN NEW; END IF;
    SELECT command_type INTO kind FROM workflow_command_receipts WHERE command_id=NEW.command_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'actual command receipt required before event' USING ERRCODE='23514'; END IF;
    IF kind='CREATE_AND_BIND_GOVERNANCE_CHILD' THEN
        IF NEW.event_type='INSTANCE_CREATED' THEN NEW.command_event_slot:=1;
        ELSIF NEW.event_type='GOVERNANCE_CHILD_BOUND' THEN NEW.command_event_slot:=2;
        ELSE RAISE EXCEPTION 'unsupported composite event type' USING ERRCODE='23514'; END IF;
    ELSIF kind='ONE_TIME_MIGRATE' THEN
        NEW.command_event_slot:=CASE WHEN NEW.event_type='INSTANCE_CREATED' THEN 1 ELSE 2 END;
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER command_event_slot BEFORE INSERT ON workflow_events
FOR EACH ROW EXECUTE FUNCTION derive_command_event_slot();
DROP INDEX idx_wf_event_unique_command;
CREATE UNIQUE INDEX idx_wf_event_command_instance ON workflow_events(command_id,workflow_instance_id) WHERE command_id IS NOT NULL;
CREATE UNIQUE INDEX idx_wf_event_command_slot ON workflow_events(command_id,command_event_slot) WHERE command_id IS NOT NULL;

CREATE FUNCTION check_command_event_integrity() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_TABLE_NAME='workflow_governance_child_bindings' THEN
        PERFORM assert_command_event_integrity(NEW.binding_command_id);
    ELSIF TG_TABLE_NAME IN ('workflow_activations','workflow_activation_closures','workflow_dispatch_eligibility_events') THEN
        IF EXISTS(SELECT 1 FROM workflow_command_receipts WHERE command_id=NEW.command_id AND command_type IN ('CREATE_AND_BIND_GOVERNANCE_CHILD','ONE_TIME_MIGRATE')) THEN
            PERFORM assert_command_event_integrity(NEW.command_id);
        END IF;
    ELSE
        PERFORM assert_command_event_integrity(NEW.command_id);
    END IF;
    RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER command_events_complete AFTER INSERT OR UPDATE ON workflow_events
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_command_event_integrity();
CREATE CONSTRAINT TRIGGER command_receipt_events_complete AFTER INSERT OR UPDATE ON workflow_command_receipts
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_command_event_integrity();
-- Command-linked facts cannot be appended later to a completed rejection either.
CREATE CONSTRAINT TRIGGER command_activation_complete AFTER INSERT ON workflow_activations
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_command_event_integrity();
CREATE CONSTRAINT TRIGGER command_activation_closure_complete AFTER INSERT ON workflow_activation_closures
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_command_event_integrity();
CREATE CONSTRAINT TRIGGER command_eligibility_complete AFTER INSERT ON workflow_dispatch_eligibility_events
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_command_event_integrity();
CREATE CONSTRAINT TRIGGER command_binding_complete AFTER INSERT ON workflow_governance_child_bindings
DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION check_command_event_integrity();
