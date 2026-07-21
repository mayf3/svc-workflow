-- Migration 0011: Add archived columns to workflow_definitions
-- Enables Domain Owner to archive (soft-disable) a workflow definition

ALTER TABLE workflow_definitions
    ADD COLUMN archived              BOOLEAN     NOT NULL DEFAULT FALSE,
    ADD COLUMN archived_at           TIMESTAMPTZ,
    ADD COLUMN archived_by_principal_id UUID REFERENCES principals(principal_id);

-- Allow filtering archived definitions
CREATE INDEX idx_wf_def_archived ON workflow_definitions (domain_id, archived);
