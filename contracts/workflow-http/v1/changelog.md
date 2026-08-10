# Changelog

## Unreleased — domain-list status filter

- **Breaking-ish (internal API):** `GET /internal/v1/workflow-instances/domain` now accepts a `status` query parameter (`active`/`cancelled`/`archived`/`all`).
- Default behavior change: when `status` is omitted and `lifecycle` is also omitted, only **active** instances are returned (`cancelled = FALSE AND archived_at IS NULL`). Previously, all instances were returned.
- When `lifecycle` is provided but `status` is omitted, `status` defaults to `all` to preserve backward compatibility for existing `lifecycle` callers.
- Use `status=all` to restore the previous "return everything" behavior.
- Added `status` to the TypeScript SDK `DomainInstanceQuery` schema.
- Invalid `status` values return 422 `invalid_status`.

## Unreleased — agent domain discovery

- **New:** `GET /internal/v1/principals/me/domains` — caller-scoped domain membership discovery. Returns every domain where the verified caller has an enabled `DOMAIN_OWNER` / `DOMAIN_MEMBER` binding (`domain_id` / `domain_key` / `display_name` / `caller_role` / `binding_created_at`). Disabled bindings and disabled domains are excluded. Requires `workflow.read` scope; accepts direct and OBO tokens.

## V1.4.1 (2026-07-26) — Current-State Re-freeze

- Re-frozen Contract Bundle against runtime mainline commit `c133118` (Repository Truth Cleanup)
- Aligned OpenAPI VersionResponse examples with runtime: `version="0.3.1"`, `schemaVersion="0014"`
- Updated owner HEAD/Tree to reference the runtime snapshot (`c133118` / `0a12962`)
- Added `canary_read_only` (403) error code to errors.json — emitted by all write endpoints when canary write guard is active
- Explicitly excluded 10 provisioning/admin-only error codes from runtime contract scope
- Updated all version references to Contract Bundle Version `1.4.1`
- Added `/version` endpoint black-box assertions to conformance suite
- Fixed digest spec and recomputed SCHEMA_DIGEST / BUNDLE_DIGEST

## V1.4.0 (2026-07-24) — Instance Input Principal Blockers Fix

- Split migration 0013 into 0013 (enum `INSTANCE_INPUT_PRINCIPAL` only) and 0014 (schema: `assignee_input_key` column + updated `chk_node_assignee_shape`), fixing the PostgreSQL 55P04 unsafe-enum-value error on fresh migrations
- Updated `SCHEMA_VERSION` to "0014" and `EXPECTED_MIGRATION_VERSION` to 14

## V1.3.0 (2026-07-22) — Canary Seed & Schema Repair

- Added migration 0012: restores `workflow_instances_workflow_state_version_check` constraint (CHECK workflow_state_version >= 1), with data integrity gate
- Fixed canary seed (`seed_canary_test_data.sql`): now creates `WORKFLOW_INSTANCE_CREATED` event for each seed instance, matching runtime event semantics
- Canary seed made idempotent: re-execution does not produce duplicate rows
- Updated `SCHEMA_VERSION` to "0012" and `EXPECTED_MIGRATION_VERSION` to 12

## V1.2.0 (2026-07-21) — Domain Owner Workflow Definition Governance

- Added `GET /internal/v1/domains/{domainId}/definitions` for listing definitions (paginated)
- Added `GET /internal/v1/domains/{domainId}/definitions/{definitionId}` for detail + versions
- Added `POST /internal/v1/domains/{domainId}/definitions` for creating definitions (idempotent)
- Added `POST /internal/v1/domains/{domainId}/definitions/{definitionId}/versions` for draft versions
- Added `PUT /internal/v1/domains/{domainId}/definitions/{definitionId}/draft` for replacing draft graph
- Added `POST /internal/v1/domains/{domainId}/definitions/{definitionId}/publish` for publishing versions
- Added `POST /internal/v1/domains/{domainId}/definitions/{definitionId}/archive` for archiving definitions
- All write endpoints require `token_use=access` (Direct tokens only, no OBO)
- Added error codes: `definition_key_conflict`, `definition_version_immutable`, `revision_conflict`, `definition_not_editable`
- Added migration 0011: `archived`, `archived_at`, `archived_by_principal_id` columns on `workflow_definitions`
- All existing definition authorization (DOMAIN_OWNER) is unchanged; handlers delegate to DefinitionService

## V1.1.0 (2026-07-20) — Agent Self-Projection & Domain Member Management

- Added `PUT /internal/v1/principals/me` for Agent self-projection using Direct Machine Tokens
- Added `GET /internal/v1/domains/{domainId}/members` for listing domain members
- Added `PUT /internal/v1/domains/{domainId}/members/{principalId}` for adding members (idempotent)
- Added `DELETE /internal/v1/domains/{domainId}/members/{principalId}` for removing members
- All new endpoints require `token_use=access` (Direct tokens only, no OBO)
- Added error codes: `direct_token_required`, `principal_not_registered`, `principal_projection_conflict`, `not_domain_owner`, `principal_is_owner`, `member_not_found`
- Updated TypeScript SDK with new methods, types, and schemas
- Frozen architecture decision document: `docs/architecture/AUTH_PRINCIPAL_SELF_PROJECTION_AND_DOMAIN_MEMBERSHIP_V1.md`

## V1.0.0 (2026-07-18) — Current-State Freeze

- Initial contract recording based on commit `2dff132`
- Records all runtime endpoints as currently implemented
- Domain list endpoint (`/internal/v1/workflow-instances/domain`) added as the newest API
- All behavior recorded as-is, no redesign
- Corrected the freeze artifacts after independent audit: valid OpenAPI 3.0
  security/nullability, strict request schemas, snake_case Timeline events, and
  the actual authentication/error wire behavior. Runtime behavior is unchanged.
