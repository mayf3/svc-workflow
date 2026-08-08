-- Migration 0016: Per-Instance Node Assignees & Artifact Binding
--
-- LINEAGE RECONCILIATION (0015 B -> 0016):
-- Historically, an alternate "0015 instance_assignees_and_artifact" (commit
-- a50602d) was applied to the legacy dogfood database as an uncommitted
-- migration. The canonical lineage is frozen as:
--
--   0001 ... 0014, 0015_add_instance_cancel_archive, 0016_instance_assignees_and_artifact
--
-- 0015_add_instance_cancel_archive is immutable and is NOT modified here.
-- This migration lands the historical B DDL under its canonical number 0016
-- so that databases carrying the canonical lineage converge on the same
-- schema. All statements are idempotent (IF NOT EXISTS) so applying 0016 to
-- a database that already has the B schema via the legacy path is a NO_OP.
--
-- Content is a faithful reconciliation of the audited a50602d migration:
--   migrations/0015_instance_assignees_and_artifact.sql (historical B)
--
-- No new Workflow functionality is introduced; the artifact /
-- explicit-assignee product logic is NOT re-enabled.

-- ============================================================
-- Per-Instance Node Assignee Overrides
-- ============================================================

CREATE TABLE IF NOT EXISTS workflow_instance_node_assignees (
    workflow_instance_id     UUID        NOT NULL REFERENCES workflow_instances(workflow_instance_id) ON DELETE CASCADE,
    node_key                 TEXT        NOT NULL CHECK (char_length(node_key) >= 1 AND char_length(node_key) <= 128),
    assignee_principal_id    UUID        NOT NULL REFERENCES principals(principal_id),
    created_at               TIMESTAMPTZ NOT NULL DEFAULT now(),

    -- One override per node per instance
    UNIQUE (workflow_instance_id, node_key)
);

CREATE INDEX IF NOT EXISTS idx_wf_inst_node_assignees_instance
    ON workflow_instance_node_assignees (workflow_instance_id);

-- ============================================================
-- Artifact Binding Columns on workflow_instances
-- ============================================================

ALTER TABLE workflow_instances
    ADD COLUMN IF NOT EXISTS require_explicit_node_assignees BOOLEAN NOT NULL DEFAULT false;

ALTER TABLE workflow_instances
    ADD COLUMN IF NOT EXISTS subject_id TEXT CHECK (subject_id IS NULL OR char_length(subject_id) <= 512);

ALTER TABLE workflow_instances
    ADD COLUMN IF NOT EXISTS artifact_id TEXT CHECK (artifact_id IS NULL OR char_length(artifact_id) <= 512);

ALTER TABLE workflow_instances
    ADD COLUMN IF NOT EXISTS artifact_version TEXT CHECK (artifact_version IS NULL OR char_length(artifact_version) <= 512);

ALTER TABLE workflow_instances
    ADD COLUMN IF NOT EXISTS artifact_digest TEXT CHECK (
        artifact_digest IS NULL OR artifact_digest ~ '^[0-9a-f]{64}$'
    );

ALTER TABLE workflow_instances
    ADD COLUMN IF NOT EXISTS require_artifact_binding BOOLEAN NOT NULL DEFAULT false;

-- ============================================================
-- Update EXPECTED_MIGRATION_VERSION in readyz check
-- ============================================================

-- NOTE: The src/http/mod.rs EXPECTED_MIGRATION_VERSION constant must be
-- updated from 15 to 16 for /readyz to return status "ready".
