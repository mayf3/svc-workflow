-- Migration 0027: WORKFLOW_EXECUTION_CONTROL_V1 (SVC_WORKFLOW_EXECUTION_CONTROL_V1)
-- Canonical forum binding fact + durable outbox (forum events + execution kicks).

CREATE TABLE workflow_forum_bindings (
    workflow_instance_id UUID NOT NULL PRIMARY KEY
        REFERENCES workflow_instances(workflow_instance_id),

    -- NULL until the outbox reconciler resolves the canonical thread.
    forum_thread_id TEXT,

    binding_state TEXT NOT NULL CHECK (binding_state IN ('PENDING', 'BOUND')),

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- One forum thread belongs to at most one workflow instance (the forum
    -- side carries the mirrored uniqueness: CTR-FWIC-001 partial unique).
    CONSTRAINT uq_forum_binding_thread UNIQUE (forum_thread_id)
);

CREATE TABLE workflow_outbox (
    outbox_id UUID NOT NULL PRIMARY KEY,

    workflow_instance_id UUID NOT NULL
        REFERENCES workflow_instances(workflow_instance_id),

    outbox_kind TEXT NOT NULL CHECK (outbox_kind IN ('FORUM_EVENT', 'EXECUTION_KICK')),

    -- Dedupe identity (e.g. wf_created:<instanceId>,
    -- transition_committed:<eventId>, kick:<activationId>).
    event_key TEXT NOT NULL,

    payload JSONB NOT NULL CHECK (jsonb_typeof(payload) = 'object'),

    attempt_count INT NOT NULL DEFAULT 0,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_error TEXT,
    delivered_at TIMESTAMPTZ,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT uq_outbox_kind_event_key UNIQUE (outbox_kind, event_key)
);

-- Only IMMUTABLE predicates are allowed in a partial index; the time
-- comparison stays in the draining query's WHERE clause.
CREATE INDEX idx_outbox_pending
    ON workflow_outbox (created_at, outbox_id)
    WHERE delivered_at IS NULL;
