-- Migration 0017: Reconcile missing CHECK constraints for B columns
--
-- CONTEXT (constraint reconciliation after canonical 0016):
-- The canonical 0016 lands the historical B columns via
-- `ADD COLUMN IF NOT EXISTS ... CHECK (...)`; on databases where the B
-- columns already exist out-of-band (live svc_workflow_dogfood_clean), the
-- `IF NOT EXISTS` branch skips the ADD COLUMN entirely, so the columns stay
-- WITHOUT the canonical CHECK constraints. Result:
--
--   ledger = 16
--   actual schema != fresh canonical schema (4 missing CHECKs)
--
-- This migration's ONLY responsibility is to idempotently add the 4
-- canonical CHECK constraints on workflow_instances where the columns
-- exist but the constraints are missing.
--
-- Guarantees:
--   * constraint names match the fresh canonical schema exactly
--     (workflow_instances_{subject_id,artifact_id,artifact_version,artifact_digest}_check)
--   * constraint definitions match what 0016 creates on a fresh database
--     (validated CHECK, same expressions)
--   * existing constraints -> NO_OP (guarded via pg_constraint probe)
--   * no business data touched, no columns altered, no new product
--     semantics, no artifact functionality changes
--   * no database-name special cases
--
-- 0015 and 0016 are frozen and are NOT modified by this migration.

DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'workflow_instances_subject_id_check'
          AND conrelid = 'workflow_instances'::regclass
    ) THEN
        ALTER TABLE workflow_instances
            ADD CONSTRAINT workflow_instances_subject_id_check
            CHECK (subject_id IS NULL OR char_length(subject_id) <= 512);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'workflow_instances_artifact_id_check'
          AND conrelid = 'workflow_instances'::regclass
    ) THEN
        ALTER TABLE workflow_instances
            ADD CONSTRAINT workflow_instances_artifact_id_check
            CHECK (artifact_id IS NULL OR char_length(artifact_id) <= 512);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'workflow_instances_artifact_version_check'
          AND conrelid = 'workflow_instances'::regclass
    ) THEN
        ALTER TABLE workflow_instances
            ADD CONSTRAINT workflow_instances_artifact_version_check
            CHECK (artifact_version IS NULL OR char_length(artifact_version) <= 512);
    END IF;

    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'workflow_instances_artifact_digest_check'
          AND conrelid = 'workflow_instances'::regclass
    ) THEN
        ALTER TABLE workflow_instances
            ADD CONSTRAINT workflow_instances_artifact_digest_check
            CHECK (artifact_digest IS NULL OR artifact_digest ~ '^[0-9a-f]{64}$');
    END IF;
END $$;

-- NOTE: EXPECTED_MIGRATION_VERSION / SCHEMA_VERSION must be updated from
-- 16/0016 to 17/0017 in src/http/mod.rs for /readyz to return "ready".
