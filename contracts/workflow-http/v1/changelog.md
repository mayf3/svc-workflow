# Changelog

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
