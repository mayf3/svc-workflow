-- Migration 0012: Restore workflow_state_version CHECK constraint
--
-- Background: The canary database had the constraint
--   workflow_instances_workflow_state_version_check
-- inadvertently dropped during a manual schema repair.  This migration
-- restores it with a data integrity gate so that instances with an
-- invalid state version are rejected before the constraint is added.
--
-- Idempotent: skips if the constraint already exists.

-- First, verify that no existing data violates the constraint.
-- If any row has workflow_state_version < 1, fail closed — do not
-- silently correct business state.
DO $$
DECLARE
    invalid_count INTEGER;
BEGIN
    SELECT COUNT(*) INTO invalid_count
    FROM workflow_instances
    WHERE workflow_state_version < 1;

    IF invalid_count > 0 THEN
        RAISE EXCEPTION
            'Migration 0012 failed: % workflow_instances have workflow_state_version < 1. '
            'Manual investigation required before constraint can be restored.',
            invalid_count;
    END IF;
END;
$$;

-- Restore the constraint if it was dropped (e.g., during manual repair).
-- On a fresh database the constraint still exists from migration 0003,
-- so we skip it to stay idempotent.
DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1
        FROM pg_constraint
        WHERE conname = 'workflow_instances_workflow_state_version_check'
    ) THEN
        ALTER TABLE workflow_instances
            ADD CONSTRAINT workflow_instances_workflow_state_version_check
            CHECK (workflow_state_version >= 1);
    END IF;
END;
$$;
