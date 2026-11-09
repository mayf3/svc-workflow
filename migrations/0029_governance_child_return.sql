-- GCRS additive immutable relation and recipient-bound review delivery facts.
-- No historical case or instance is backfilled.
CREATE TABLE workflow_governance_child_bindings (
    binding_id UUID PRIMARY KEY,
    assistance_case_id UUID NOT NULL UNIQUE REFERENCES workflow_assistance_cases(assistance_case_id),
    source_instance_id UUID NOT NULL REFERENCES workflow_instances(workflow_instance_id),
    source_node_visit_id UUID NOT NULL,
    request_context_revision_id UUID NOT NULL,
    source_definition_version_id UUID NOT NULL REFERENCES workflow_definition_versions(definition_version_id),
    request_command_id UUID NOT NULL REFERENCES workflow_command_receipts(command_id),
    request_event_id UUID NOT NULL REFERENCES workflow_events(event_id),
    request_payload_digest TEXT NOT NULL CHECK (request_payload_digest ~ '^[0-9a-f]{64}$'),
    governance_child_instance_id UUID NOT NULL UNIQUE REFERENCES workflow_instances(workflow_instance_id),
    child_definition_version_id UUID NOT NULL REFERENCES workflow_definition_versions(definition_version_id),
    child_context_revision_at_bind UUID NOT NULL,
    acceptance_transition_id UUID NOT NULL REFERENCES workflow_transition_definitions(transition_id),
    acceptance_contract_digest TEXT NOT NULL CHECK (acceptance_contract_digest ~ '^[0-9a-f]{64}$'),
    binding_principal_id UUID NOT NULL REFERENCES principals(principal_id),
    binding_command_id UUID NOT NULL REFERENCES workflow_command_receipts(command_id),
    binding_event_id UUID NOT NULL REFERENCES workflow_events(event_id) DEFERRABLE INITIALLY DEFERRED,
    receipt JSONB NOT NULL,
    terminal_event_id UUID UNIQUE REFERENCES workflow_events(event_id),
    terminal_proof JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (source_instance_id <> governance_child_instance_id),
    CHECK ((terminal_event_id IS NULL) = (terminal_proof IS NULL)),
    FOREIGN KEY (source_node_visit_id, source_instance_id) REFERENCES workflow_node_visits(node_visit_id,workflow_instance_id),
    FOREIGN KEY (request_context_revision_id, source_instance_id) REFERENCES workflow_context_revisions(context_revision_id,workflow_instance_id),
    FOREIGN KEY (child_context_revision_at_bind, governance_child_instance_id) REFERENCES workflow_context_revisions(context_revision_id,workflow_instance_id)
);
CREATE TABLE workflow_governance_child_reviews (
    review_wake_id UUID PRIMARY KEY,
    binding_id UUID NOT NULL REFERENCES workflow_governance_child_bindings(binding_id),
    terminal_event_id UUID NOT NULL REFERENCES workflow_events(event_id),
    owner_principal_id UUID NOT NULL REFERENCES principals(principal_id),
    owner_binding_id UUID NOT NULL REFERENCES domain_role_bindings(binding_id),
    owner_replacement_command_id UUID REFERENCES workflow_command_receipts(command_id),
    outbox_id UUID NOT NULL UNIQUE REFERENCES workflow_outbox(outbox_id),
    retired_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE NULLS NOT DISTINCT (binding_id, terminal_event_id, owner_principal_id, owner_replacement_command_id)
);
CREATE INDEX idx_governance_child_source ON workflow_governance_child_bindings(source_instance_id);
CREATE FUNCTION guard_governance_child_binding() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'governance binding history is immutable'; END IF;
    IF TG_OP = 'UPDATE' THEN
        IF (to_jsonb(OLD) - 'terminal_event_id' - 'terminal_proof') IS DISTINCT FROM
           (to_jsonb(NEW) - 'terminal_event_id' - 'terminal_proof') OR
           (OLD.terminal_event_id IS NOT NULL AND
             (OLD.terminal_event_id IS DISTINCT FROM NEW.terminal_event_id OR OLD.terminal_proof IS DISTINCT FROM NEW.terminal_proof)) THEN
            RAISE EXCEPTION 'governance binding provenance is immutable';
        END IF;
        RETURN NEW;
    END IF;
    -- Instance locks held by the service serialize competing one-layer edges.
    IF EXISTS (SELECT 1 FROM workflow_governance_child_bindings
      WHERE governance_child_instance_id=NEW.source_instance_id OR source_instance_id=NEW.governance_child_instance_id) THEN
        RAISE EXCEPTION 'governance relationship must have one layer';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER governance_child_binding_guard BEFORE INSERT OR UPDATE OR DELETE
ON workflow_governance_child_bindings FOR EACH ROW EXECUTE FUNCTION guard_governance_child_binding();

CREATE FUNCTION guard_governance_child_review() RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN RAISE EXCEPTION 'governance review history is immutable'; END IF;
    IF (to_jsonb(OLD) - 'retired_reason') IS DISTINCT FROM (to_jsonb(NEW) - 'retired_reason') THEN
        RAISE EXCEPTION 'governance review recipient identity is immutable';
    END IF;
    IF OLD.retired_reason IS NOT NULL AND OLD.retired_reason IS DISTINCT FROM NEW.retired_reason THEN
        RAISE EXCEPTION 'governance review retirement is immutable';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER governance_child_review_guard BEFORE UPDATE OR DELETE
ON workflow_governance_child_reviews FOR EACH ROW EXECUTE FUNCTION guard_governance_child_review();
