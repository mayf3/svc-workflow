-- Migration 0015: Add cancel and archive columns to workflow_instances
-- Enables Domain Owner to cancel active instances and archive terminal instances

ALTER TABLE workflow_instances
    ADD COLUMN cancelled              BOOLEAN     NOT NULL DEFAULT FALSE,
    ADD COLUMN cancelled_at           TIMESTAMPTZ,
    ADD COLUMN cancelled_by_principal_id UUID REFERENCES principals(principal_id),
    ADD COLUMN cancel_reason          VARCHAR(2000),
    ADD COLUMN archived_at            TIMESTAMPTZ,
    ADD COLUMN archived_by_principal_id UUID REFERENCES principals(principal_id),
    ADD COLUMN archive_reason         VARCHAR(2000);

-- Allow filtering cancelled instances in worklists
CREATE INDEX idx_wi_cancelled ON workflow_instances (cancelled) WHERE cancelled = TRUE;
