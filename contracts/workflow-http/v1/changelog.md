# Changelog

## V1.0.0 (2026-07-18) — Current-State Freeze

- Initial contract recording based on commit `2dff132`
- Records all runtime endpoints as currently implemented
- Domain list endpoint (`/internal/v1/workflow-instances/domain`) added as the newest API
- All behavior recorded as-is, no redesign
- Corrected the freeze artifacts after independent audit: valid OpenAPI 3.0
  security/nullability, strict request schemas, snake_case Timeline events, and
  the actual authentication/error wire behavior. Runtime behavior is unchanged.
