-- Migration 0024: Add deprecation_reason to workflow definition versions
--
-- PURPOSE:
-- Records WHY a PUBLISHED version was deprecated without a successor.
-- Governing authority:
-- docs/specs/SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2.md, CTR-CIR-003
-- (final paragraph): for exactly the two held versions of parent
-- CTR-WACI-010, the reviewed plan may deprecate without publishing a
-- replacement, "atomically recording SOURCE_IDENTITY_UNRESOLVED and
-- preserving every task/history fact". The existing lifecycle columns
-- (deprecated_at / deprecated_by_principal_id) record WHEN and BY WHOM, but
-- no mechanism recorded WHY; this column carries the reason token written in
-- the same deprecation transaction.
--
--   deprecation_reason = NULL            -> ordinary deprecation (default;
--                                          existing behavior unchanged)
--   deprecation_reason = 'SOURCE_IDENTITY_UNRESOLVED'
--                                        -> source-only stop-new provenance
--                                          for a held version (see
--                                          docs/runbooks/SOURCE_ONLY_STOP_NEW_V1.md)
--
-- Nullable, no backfill: existing DEPRECATED/REVOKED rows keep NULL. The
-- write path is atomic_deprecate only; revocation and publish never touch
-- this column. No instance, visit, context or history row is modified.
--
-- NOTE: EXPECTED_MIGRATION_VERSION must be updated from 23 to 24 in
-- src/http/mod.rs for /readyz to return "ready".

ALTER TABLE workflow_definition_versions
    ADD COLUMN deprecation_reason TEXT
    CHECK (deprecation_reason IS NULL OR char_length(deprecation_reason) <= 256);
