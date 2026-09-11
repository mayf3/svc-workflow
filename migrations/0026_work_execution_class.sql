-- SVC_WORKFLOW_WORK_EXECUTION_CLASS_V1 (CTR-WEC-001)
--
-- Explicit work execution class on every workflow instance:
--   BUSINESS          — normal business work (universal default)
--   NON_BUSINESS_TEST — explicitly marked test/canary work; excluded from
--                       the normal BUSINESS automated dispatch due feed
--                       (CTR-WEC-003) by the query predicate, never by a
--                       blocked state.
--
-- The column is NOT NULL DEFAULT 'BUSINESS': the default IS the
-- compatibility story. ZERO_SEPARATE_BACKFILL; no historical row is ever
-- reclassified. The class is written exactly once, by the instance-create
-- transaction (CTR-WEC-002); no UPDATE path exists.

CREATE TYPE workflow_execution_class AS ENUM ('BUSINESS', 'NON_BUSINESS_TEST');

ALTER TABLE workflow_instances
    ADD COLUMN execution_class workflow_execution_class NOT NULL DEFAULT 'BUSINESS';
