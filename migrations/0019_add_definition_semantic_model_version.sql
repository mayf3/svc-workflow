-- Migration 0019: Add immutable semantic model version to definition versions
--
-- PURPOSE:
-- WorkflowDefinitionVersion must carry a first-class, immutable semantic
-- model version so the runtime can mechanically decide whether a version
-- is interpreted under Legacy (1) or Minimal (2) semantics. This version
-- boundary is established NOW; Minimal semantics (2) is NOT implemented
-- yet and is only a defined, accepted value.
--
--   semantic_model_version = 1  -> Legacy semantics (all existing versions)
--   semantic_model_version = 2  -> Minimal semantics (defined, not yet
--                                   production-implemented)
--
-- All existing definition versions are backfilled to 1 (they were all
-- authored and interpreted under Legacy rules). NOT NULL is enforced at the
-- DB layer and the value is constrained to the currently defined set.
--
-- No WorkflowInstance data, node data, or existing definition content is
-- touched. 0015..0018 are frozen and unmodified.

ALTER TABLE workflow_definition_versions
    ADD COLUMN semantic_model_version SMALLINT NOT NULL DEFAULT 1;

ALTER TABLE workflow_definition_versions
    ADD CONSTRAINT workflow_definition_versions_semantic_model_version_check
    CHECK (semantic_model_version IN (1, 2));

-- NOTE: EXPECTED_MIGRATION_VERSION / SCHEMA_VERSION must be updated from
-- 18/0018 to 19/0019 in src/http/mod.rs for /readyz to return "ready".
