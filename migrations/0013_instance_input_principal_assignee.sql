-- Migration 0013: Extend assignee_ref_type enum with INSTANCE_INPUT_PRINCIPAL.
--
-- This is a standalone ALTER TYPE so the new value is committed before any
-- CHECK constraint or schema change references it. The subsequent schema
-- changes (column + constraint) live in migration 0014.

ALTER TYPE assignee_ref_type ADD VALUE IF NOT EXISTS 'INSTANCE_INPUT_PRINCIPAL';
