-- Bounded CIR successor-Instance closure surface.
-- Governing authority: SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2,
-- CTR-CIR-003/004/006/007/008. This is not a generic migration API.

CREATE TABLE workflow_instance_successor_cutovers (
    source_instance_id UUID PRIMARY KEY REFERENCES workflow_instances(workflow_instance_id),
    successor_instance_id UUID NOT NULL UNIQUE REFERENCES workflow_instances(workflow_instance_id),
    source_definition_version_id UUID NOT NULL REFERENCES workflow_definition_versions(definition_version_id),
    successor_definition_version_id UUID NOT NULL REFERENCES workflow_definition_versions(definition_version_id),
    source_current_visit_id UUID NOT NULL REFERENCES workflow_node_visits(node_visit_id),
    source_context_revision_id UUID NOT NULL REFERENCES workflow_context_revisions(context_revision_id),
    successor_initial_visit_id UUID NOT NULL REFERENCES workflow_node_visits(node_visit_id),
    successor_context_revision_id UUID NOT NULL REFERENCES workflow_context_revisions(context_revision_id),
    source_node_id UUID NOT NULL REFERENCES workflow_node_definitions(node_id),
    successor_node_id UUID NOT NULL REFERENCES workflow_node_definitions(node_id),
    source_expected_state_version INTEGER NOT NULL CHECK (source_expected_state_version >= 1),
    model SMALLINT NOT NULL CHECK (model = 1),
    closure_reason TEXT NOT NULL CHECK (closure_reason = 'CIR_CANONICAL_SUCCESSOR'),
    plan_sha256 TEXT NOT NULL CHECK (plan_sha256 ~ '^[0-9a-f]{64}$'),
    group_identity_digest TEXT NOT NULL CHECK (group_identity_digest ~ '^[0-9a-f]{64}$'),
    request_hash TEXT NOT NULL CHECK (request_hash ~ '^[0-9a-f]{64}$'),
    command_id UUID NOT NULL UNIQUE REFERENCES workflow_command_receipts(command_id),
    operator_principal_id UUID NOT NULL REFERENCES principals(principal_id),
    original_creator_principal_id UUID NOT NULL REFERENCES principals(principal_id),
    successor_creator_principal_id UUID NOT NULL REFERENCES principals(principal_id),
    approved_identity_changes JSONB NOT NULL,
    database_identity JSONB NOT NULL,
    authority_refs JSONB NOT NULL,
    operator_code_coordinate TEXT NOT NULL,
    receipt JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (source_instance_id <> successor_instance_id)
);

CREATE OR REPLACE FUNCTION fn_successor_cutover_append_only()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'successor_cutover_append_only: cannot % immutable closure fact', TG_OP
        USING ERRCODE = '23000';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_successor_cutover_append_only
    BEFORE UPDATE OR DELETE ON workflow_instance_successor_cutovers
    FOR EACH ROW EXECUTE FUNCTION fn_successor_cutover_append_only();

-- A closure fact is the CIR-defined superseded state. Existing command paths
-- all update the Instance projection before commit, so this guard makes a
-- closed source mechanically non-writable without changing its old row.
CREATE OR REPLACE FUNCTION fn_reject_closed_source_instance_write()
RETURNS TRIGGER AS $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM workflow_instance_successor_cutovers c
         WHERE c.source_instance_id = OLD.workflow_instance_id
    ) THEN
        RAISE EXCEPTION 'workflow instance % is superseded by CIR successor', OLD.workflow_instance_id
            USING ERRCODE = '55000';
    END IF;
    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_reject_closed_source_instance_write
    BEFORE UPDATE OR DELETE ON workflow_instances
    FOR EACH ROW EXECUTE FUNCTION fn_reject_closed_source_instance_write();
