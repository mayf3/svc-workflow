-- Migration 0021: Workflow Assistance V1
-- Visit-scoped, side-band assistance without changing instance lifecycle or assignee.

CREATE TABLE workflow_assistance_cases (
    assistance_case_id UUID NOT NULL PRIMARY KEY,
    workflow_instance_id UUID NOT NULL
        REFERENCES workflow_instances(workflow_instance_id),
    node_visit_id UUID NOT NULL,

    status TEXT NOT NULL CHECK (
        status IN ('OWNER_PENDING', 'HUMAN_REQUIRED', 'RESOLVED', 'VOIDED')
    ),

    requested_by_principal_id UUID NOT NULL REFERENCES principals(principal_id),
    request_payload JSONB NOT NULL CHECK (jsonb_typeof(request_payload) = 'object'),
    request_payload_digest TEXT NOT NULL
        CHECK (request_payload_digest ~ '^[0-9a-f]{64}$'),
    request_command_id UUID NOT NULL REFERENCES workflow_command_receipts(command_id),

    escalated_by_principal_id UUID REFERENCES principals(principal_id),
    escalation_payload JSONB CHECK (
        escalation_payload IS NULL OR jsonb_typeof(escalation_payload) = 'object'
    ),
    escalation_payload_digest TEXT CHECK (
        escalation_payload_digest IS NULL
        OR escalation_payload_digest ~ '^[0-9a-f]{64}$'
    ),
    escalation_command_id UUID REFERENCES workflow_command_receipts(command_id),
    escalated_at TIMESTAMPTZ,

    resolved_by_principal_id UUID REFERENCES principals(principal_id),
    resolution_payload JSONB CHECK (
        resolution_payload IS NULL OR jsonb_typeof(resolution_payload) = 'object'
    ),
    resolution_payload_digest TEXT CHECK (
        resolution_payload_digest IS NULL
        OR resolution_payload_digest ~ '^[0-9a-f]{64}$'
    ),
    resolution_command_id UUID REFERENCES workflow_command_receipts(command_id),
    resolved_at TIMESTAMPTZ,

    voided_by_principal_id UUID REFERENCES principals(principal_id),
    void_reason_code TEXT CHECK (
        void_reason_code IS NULL
        OR void_reason_code IN (
            'INSTANCE_CANCELLED',
            'INSTANCE_ARCHIVED',
            'ADMIN_EMERGENCY_OVERRIDE',
            'ADMIN_PROJECTION_REBUILD'
        )
    ),
    voided_by_command_id UUID REFERENCES workflow_command_receipts(command_id),
    voided_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    UNIQUE (request_command_id),
    FOREIGN KEY (node_visit_id, workflow_instance_id)
        REFERENCES workflow_node_visits(node_visit_id, workflow_instance_id)
        DEFERRABLE INITIALLY DEFERRED,

    CHECK (pg_column_size(request_payload) <= 65536),
    CHECK (escalation_payload IS NULL OR pg_column_size(escalation_payload) <= 65536),
    CHECK (resolution_payload IS NULL OR pg_column_size(resolution_payload) <= 65536),

    CHECK (
        (escalated_by_principal_id IS NULL
         AND escalation_payload IS NULL
         AND escalation_payload_digest IS NULL
         AND escalation_command_id IS NULL
         AND escalated_at IS NULL)
        OR
        (escalated_by_principal_id IS NOT NULL
         AND escalation_payload IS NOT NULL
         AND escalation_payload_digest IS NOT NULL
         AND escalation_command_id IS NOT NULL
         AND escalated_at IS NOT NULL)
    ),
    CHECK (
        (resolved_by_principal_id IS NULL
         AND resolution_payload IS NULL
         AND resolution_payload_digest IS NULL
         AND resolution_command_id IS NULL
         AND resolved_at IS NULL)
        OR
        (resolved_by_principal_id IS NOT NULL
         AND resolution_payload IS NOT NULL
         AND resolution_payload_digest IS NOT NULL
         AND resolution_command_id IS NOT NULL
         AND resolved_at IS NOT NULL)
    ),
    CHECK (
        (voided_by_principal_id IS NULL
         AND void_reason_code IS NULL
         AND voided_by_command_id IS NULL
         AND voided_at IS NULL)
        OR
        (voided_by_principal_id IS NOT NULL
         AND void_reason_code IS NOT NULL
         AND voided_by_command_id IS NOT NULL
         AND voided_at IS NOT NULL)
    ),
    CHECK (
        (status = 'OWNER_PENDING'
         AND escalation_command_id IS NULL
         AND resolution_command_id IS NULL
         AND voided_by_command_id IS NULL)
        OR
        (status = 'HUMAN_REQUIRED'
         AND escalation_command_id IS NOT NULL
         AND resolution_command_id IS NULL
         AND voided_by_command_id IS NULL)
        OR
        (status = 'RESOLVED'
         AND resolution_command_id IS NOT NULL
         AND voided_by_command_id IS NULL)
        OR
        (status = 'VOIDED'
         AND resolution_command_id IS NULL
         AND voided_by_command_id IS NOT NULL)
    )
);

CREATE UNIQUE INDEX uq_assistance_one_open_per_visit
    ON workflow_assistance_cases(node_visit_id)
    WHERE status IN ('OWNER_PENDING', 'HUMAN_REQUIRED');

CREATE UNIQUE INDEX uq_assistance_escalation_command
    ON workflow_assistance_cases(escalation_command_id)
    WHERE escalation_command_id IS NOT NULL;

CREATE UNIQUE INDEX uq_assistance_resolution_command
    ON workflow_assistance_cases(resolution_command_id)
    WHERE resolution_command_id IS NOT NULL;

CREATE INDEX idx_assistance_requester
    ON workflow_assistance_cases(
        requested_by_principal_id, created_at DESC, assistance_case_id DESC
    );

CREATE INDEX idx_assistance_owner_open
    ON workflow_assistance_cases(
        workflow_instance_id, created_at DESC, assistance_case_id DESC
    )
    WHERE status IN ('OWNER_PENDING', 'HUMAN_REQUIRED');

CREATE INDEX idx_assistance_human_required
    ON workflow_assistance_cases(escalated_at DESC, assistance_case_id DESC)
    WHERE status = 'HUMAN_REQUIRED';

CREATE OR REPLACE FUNCTION fn_check_assistance_case_change()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP = 'DELETE' THEN
        RAISE EXCEPTION 'immutable record: workflow_assistance_cases does not allow DELETE'
            USING ERRCODE = '23000';
    END IF;

    IF OLD.assistance_case_id IS DISTINCT FROM NEW.assistance_case_id
       OR OLD.workflow_instance_id IS DISTINCT FROM NEW.workflow_instance_id
       OR OLD.node_visit_id IS DISTINCT FROM NEW.node_visit_id
       OR OLD.requested_by_principal_id IS DISTINCT FROM NEW.requested_by_principal_id
       OR OLD.request_payload IS DISTINCT FROM NEW.request_payload
       OR OLD.request_payload_digest IS DISTINCT FROM NEW.request_payload_digest
       OR OLD.request_command_id IS DISTINCT FROM NEW.request_command_id
       OR OLD.created_at IS DISTINCT FROM NEW.created_at THEN
        RAISE EXCEPTION 'immutable assistance case identity/request fields changed'
            USING ERRCODE = '23000';
    END IF;

    IF OLD.status = 'OWNER_PENDING' AND NEW.status = 'HUMAN_REQUIRED' THEN
        IF OLD.escalation_command_id IS NOT NULL
           OR NEW.escalation_command_id IS NULL
           OR NEW.resolution_command_id IS NOT NULL
           OR NEW.voided_by_command_id IS NOT NULL THEN
            RAISE EXCEPTION 'invalid assistance escalation fields'
                USING ERRCODE = '23000';
        END IF;
    ELSIF OLD.status IN ('OWNER_PENDING', 'HUMAN_REQUIRED')
          AND NEW.status = 'RESOLVED' THEN
        IF NEW.resolution_command_id IS NULL
           OR NEW.voided_by_command_id IS NOT NULL
           OR OLD.resolution_command_id IS NOT NULL THEN
            RAISE EXCEPTION 'invalid assistance resolution fields'
                USING ERRCODE = '23000';
        END IF;
        IF OLD.escalated_by_principal_id IS DISTINCT FROM NEW.escalated_by_principal_id
           OR OLD.escalation_payload IS DISTINCT FROM NEW.escalation_payload
           OR OLD.escalation_payload_digest IS DISTINCT FROM NEW.escalation_payload_digest
           OR OLD.escalation_command_id IS DISTINCT FROM NEW.escalation_command_id
           OR OLD.escalated_at IS DISTINCT FROM NEW.escalated_at THEN
            RAISE EXCEPTION 'immutable assistance escalation fields changed'
                USING ERRCODE = '23000';
        END IF;
    ELSIF OLD.status IN ('OWNER_PENDING', 'HUMAN_REQUIRED')
          AND NEW.status = 'VOIDED' THEN
        IF NEW.voided_by_command_id IS NULL
           OR NEW.resolution_command_id IS NOT NULL
           OR OLD.voided_by_command_id IS NOT NULL THEN
            RAISE EXCEPTION 'invalid assistance void fields'
                USING ERRCODE = '23000';
        END IF;
        IF OLD.escalated_by_principal_id IS DISTINCT FROM NEW.escalated_by_principal_id
           OR OLD.escalation_payload IS DISTINCT FROM NEW.escalation_payload
           OR OLD.escalation_payload_digest IS DISTINCT FROM NEW.escalation_payload_digest
           OR OLD.escalation_command_id IS DISTINCT FROM NEW.escalation_command_id
           OR OLD.escalated_at IS DISTINCT FROM NEW.escalated_at THEN
            RAISE EXCEPTION 'immutable assistance escalation fields changed'
                USING ERRCODE = '23000';
        END IF;
    ELSE
        RAISE EXCEPTION 'invalid assistance status transition: % -> %', OLD.status, NEW.status
            USING ERRCODE = '23000';
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_assistance_case_change
    BEFORE UPDATE OR DELETE ON workflow_assistance_cases
    FOR EACH ROW EXECUTE FUNCTION fn_check_assistance_case_change();
