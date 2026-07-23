-- Migration 0013: INSTANCE_INPUT_PRINCIPAL assignee resolution.
--
-- Adds a new assignee_ref_type variant so a node's assignee can be resolved
-- from a stable Principal UUID supplied in the instance's context_payload at
-- creation time. This supports the "Principal A creates for Principal B"
-- pattern without requiring B to be a domain member at creation time.
--
-- The assignee input key (e.g. "assigneePrincipalId") is stored on the node
-- definition so the runtime knows which context_payload field carries the
-- target Principal UUID. It is part of the definition digest.

-- 1. Extend the assignee_ref_type enum with the new variant.
ALTER TYPE assignee_ref_type ADD VALUE IF NOT EXISTS 'INSTANCE_INPUT_PRINCIPAL';

-- 2. Add the input-key column. NULL for every existing assignee shape; only
--    INSTANCE_INPUT_PRINCIPAL nodes carry a non-null value. Defaults to NULL
--    so existing rows are untouched and backfilled implicitly.
ALTER TABLE workflow_node_definitions
    ADD COLUMN IF NOT EXISTS assignee_input_key TEXT
    CHECK (assignee_input_key IS NULL OR (
        char_length(assignee_input_key) >= 1
        AND char_length(assignee_input_key) <= 128
        AND assignee_input_key ~ '^[A-Za-z_][A-Za-z0-9_]*$'
    ));

-- 3. Replace chk_node_assignee_shape so it also describes the new shape.
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
