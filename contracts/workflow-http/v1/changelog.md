# Changelog

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
