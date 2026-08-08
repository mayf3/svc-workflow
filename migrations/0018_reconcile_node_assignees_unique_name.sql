-- Migration 0018: Reconcile UNIQUE constraint/index name on node assignees
--
-- CONTEXT (final catalog-name divergence):
-- On the live svc_workflow_dogfood_clean database the table
-- workflow_instance_node_assignees was created out-of-band with the
-- historical B DDL, so its UNIQUE (workflow_instance_id, node_key)
-- constraint received a different auto-generated (truncated) name:
--
--   legacy:  workflow_instance_node_assignees_workflow_instance_id_node_key_
--
-- The canonical schema (0016 fresh create) names the same constraint:
--
--   canonical: workflow_instance_node_assign_workflow_instance_id_node_key_key
--
-- Both are UNIQUE (workflow_instance_id, node_key), validated, backed by a
-- unique index with identical semantics; only the catalog name differs.
--
-- This migration's ONLY responsibility: if the legacy name exists AND the
-- canonical name does not, RENAME the constraint to the canonical name.
-- PostgreSQL renames the backing UNIQUE index automatically as part of
-- RENAME CONSTRAINT (no table rewrite, no data change).
--
-- Target tri-state:
--   live (legacy present, canonical absent)        -> rename
--   fresh (canonical present, legacy absent)       -> NO_OP
--   non-live svc_workflow (same as fresh)          -> NO_OP
--
-- Explicitly NOT doing: DROP+recreate, definition changes, data changes,
-- column changes, database-name special cases, checksum bypass, runtime
-- special cases. 0015/0016/0017 are frozen and untouched.

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'workflow_instance_node_assignees_workflow_instance_id_node_key_'
          AND conrelid = 'workflow_instance_node_assignees'::regclass
    ) AND NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'workflow_instance_node_assign_workflow_instance_id_node_key_key'
          AND conrelid = 'workflow_instance_node_assignees'::regclass
    ) THEN
        ALTER TABLE workflow_instance_node_assignees
            RENAME CONSTRAINT
                workflow_instance_node_assignees_workflow_instance_id_node_key_
            TO
                workflow_instance_node_assign_workflow_instance_id_node_key_key;
    END IF;
END $$;

-- NOTE: EXPECTED_MIGRATION_VERSION / SCHEMA_VERSION must be updated from
-- 17/0017 to 18/0018 in src/http/mod.rs for /readyz to return "ready".
