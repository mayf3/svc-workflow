-- Migration 0014: INSTANCE_INPUT_PRINCIPAL schema updates.
--
-- Depends on migration 0013 which committed the INSTANCE_INPUT_PRINCIPAL enum
-- value. Runs in a separate transaction so the CHECK constraint referencing
-- the new enum value is valid.

-- 1. Add the input-key column. NULL for every existing assignee shape; only
--    INSTANCE_INPUT_PRINCIPAL nodes carry a non-null value. Defaults to NULL
--    so existing rows are untouched and backfilled implicitly.
ALTER TABLE workflow_node_definitions
    ADD COLUMN IF NOT EXISTS assignee_input_key TEXT
    CHECK (assignee_input_key IS NULL OR (
        char_length(assignee_input_key) >= 1
        AND char_length(assignee_input_key) <= 128
        AND assignee_input_key ~ '^[A-Za-z_][A-Za-z0-9_]*$'
    ));

-- 2. Replace chk_node_assignee_shape so it also describes the new shape.
--    INSTANCE_INPUT_PRINCIPAL requires a non-null assignee_input_key and a
--    null fixed_principal_id (the principal is supplied per-instance, not on
--    the definition). NOT VALID keeps existing (already-published) rows valid.
ALTER TABLE workflow_node_definitions DROP CONSTRAINT IF EXISTS chk_node_assignee_shape;

ALTER TABLE workflow_node_definitions
    ADD CONSTRAINT chk_node_assignee_shape
    CHECK (
        (node_type = 'TERMINAL'
            AND assignee_ref_type IS NULL
            AND fixed_principal_id IS NULL
            AND assignee_input_key IS NULL)
        OR
        (node_type <> 'TERMINAL'
            AND assignee_ref_type IS NOT NULL
            AND (
                (assignee_ref_type = 'FIXED_PRINCIPAL'
                    AND fixed_principal_id IS NOT NULL
                    AND assignee_input_key IS NULL)
                OR
                (assignee_ref_type IN ('WORKFLOW_CREATOR', 'DOMAIN_OWNER')
                    AND fixed_principal_id IS NULL
                    AND assignee_input_key IS NULL)
                OR
                (assignee_ref_type = 'INSTANCE_INPUT_PRINCIPAL'
                    AND fixed_principal_id IS NULL
                    AND assignee_input_key IS NOT NULL)
            ))
    ) NOT VALID;
