# svc-workflow Auth-service JWKS Verifier + Principal Context V0 — Design Report

```text
Status: SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN_READY
Generated: 2026-07-16
```

---

## Repository

```
svc-workflow
Path: svc-workflow
```

---

## BASE_SHA

```
4a7b3a324e97410441b3f65c01e3b27f835ad85b
```

---

## Worktree Status

```
?? ADC_SVC_WORKFLOW_INTEGRATION_READINESS_REPORT.md
?? SVC_WORKFLOW_INTERNAL_API_V0_CONTRACT_INVESTIGATION.md
?? SVC_WORKFLOW_JWKS_IDENTITY_PROVISIONING_INVESTIGATION.md
```

All three are untracked investigation/audit report artifacts. No staged or modified tracked files.

---

## Contracts Read

| Document | Path | Status |
|----------|------|--------|
| JWKS / OBO Auth V0 | `docs/contracts/JWKS_OBO_AUTH_V0.md` | FROZEN_FOR_STAGE_1_AUTHENTICATED_SMOKE |
| Identity Provisioning API V0 | `docs/contracts/IDENTITY_PROVISIONING_API_V0.md` | FROZEN_FOR_PROVISIONING_READY |
| Internal API Contract V0.1 | `docs/contracts/INTERNAL_API_CONTRACT_V0_1.md` | FROZEN_FOR_STAGE_1_SMOKE |
| JWKS/OBO Verifier Audit | `SVC_WORKFLOW_JWKS_VERIFIER_AUDIT.md` | AUDIT_PASS |
| Identity Provisioning Audit | `SVC_WORKFLOW_IDENTITY_PROVISIONING_API_V0_AUDIT.md` | AUDIT_PASS |
| Auth-service RS256 JWKS V0 | Cross-repo reference | Merged + Canary PASS |
| Auth-service OBO V0 | Cross-repo reference | Merged + Canary PASS |

---

## Current Auth Entry Point

The auth entry point is `AuthenticatedPrincipal` as an Axum `FromRequestParts` extractor (`src/auth/principal.rs:45-96`):

1. Reads `Authorization: Bearer <token>` from request headers
2. Delegates to `state.auth_verifier.verify(token)` — dispatches to `Hs256Verifier` or `JwksVerifier`
3. Both verifiers return `AuthenticatedPrincipal` containing `principal_id: PrincipalId` and `auth_context: AuthContext`
4. Logs audit entry via `auth_context.log_audit()`
5. Returns principal to handler

**Handler usage**: All authenticated handlers extract `principal: AuthenticatedPrincipal` and use `principal.principal_id` as the command's `principal_id`. No handler re-parses the Authorization header. No handler reads `act.sub` or `azp` for domain decisions.

---

## Current Principal Key

```text
CANONICAL_PRINCIPAL_ID_SOURCE = token.sub  (JWT.sub = UUID)
Domain type: PrincipalId (UUID newtype, src/domain/ids.rs)
```

Status: **Already UUID, no migration needed.**

### Evidence
- `principal_id` in all domain commands is `PrincipalId`
- `PrincipalId::from_uuid(uuid)` is used consistently
- Database `principals.principal_id` is `UUID`
- `AssigneeRef.fixed_principal_id` is `Option<PrincipalId>`
- `WorkflowDefinitionVersion.published_by_principal_id` is `Option<PrincipalId>`
- Provisioning API receives `principal_id: Uuid` and stores it directly
- No email, username, agentId, or client_id is used as a principal identifier anywhere in the domain model

### DB tables using PrincipalId
- `principals.principal_id` (UUID PK)
- `domain_role_bindings.principal_id` (UUID FK)
- `workflow_definition_versions.published_by_principal_id` (UUID nullable)
- `workflow_definition_versions.deprecated_by_principal_id` (UUID nullable)
- `workflow_definition_versions.revoked_by_principal_id` (UUID nullable)
- `workflow_command_receipts.principal_id` (UUID FK)

---

## Migration Judgment

```text
SVC_WORKFLOW_AUTH_PRINCIPAL_BLOCKING_MIGRATION_REQUIRED = false
```

No migration is required because:
1. `PrincipalId` is already a UUID — no type change from `token.sub` to domain ID
2. Owner/Assignee/Reviewer fields are already `PrincipalId` (UUID) — no column conversion
3. No email/username/agentId is used in domain authorization decisions
4. Provisioning API already uses UUID as the identity key
5. The `principals` table already stores UUID as PK
6. No alias mapping or shadow principal table needed
7. No JIT provisioning needed — principals must be pre-provisioned

**If a blocking migration were needed, it would need to:**
1. Add a `principal_id UUID` column to replace any non-UUID identity fields
2. Add an alias table mapping legacy IDs to canonical UUIDs
3. Backfill existing records with auth-service MachinePrincipal IDs
4. Update all FK references

**None of these apply.** The current codebase is already compatible with auth-service's canonical UUID model.

---

## Recommended First PR

```text
PR-C1: JWKS Client + RS256 Verifier hardening
```

### Rationale
- The core JWKS verifier is already merged and audited — PR-C1 primarily addresses hardening items from the audit (redirect policy, URL scheme validation, duplicate-kid warning, Direct/OBO profile checks)
- Principal Context (PR-C2) depends on having a hardened verifier first
- Route Scope Guard (PR-C3) depends on Principal Context
- Audit/Canary (PR-C4) depends on all preceding PRs
- PR-C1 can be merged and tested independently without changing any domain behavior

### What PR-C1 includes
1. `JwksVerifier`: add `.redirect(reqwest::redirect::Policy::none())` to client builder
2. `JwksConfig::from_env`: validate URL scheme is `http` or `https`
3. `JwksVerifier::fetch_jwks`: log `tracing::warn!` on duplicate kid
4. `JwksVerifier::verify`: add Direct token profile check (reject `act`/`azp` for non-OBO tokens)
5. `JwksVerifier::verify`: add OBO `client_id === azp` enforcement
6. `JwksVerifier::verify`: reject nested `act`
7. `JwksVerifier::verify`: freeze `principal_type` to `agent`-only for V0
8. Error contract: add granular error codes
9. Extend test matrix in `jwks_auth.rs`

---

## Next Agent Requirements

The next step must be an **independent audit agent** reviewing:

1. **Design freeze document**: `SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN.md`
   - Verify all sections are present and consistent
   - Verify fixed principles (CANONICAL_PRINCIPAL_ID_SOURCE, JWT_ALGORITHM_ALLOWLIST, etc.)
   - Verify no deviating from the mandatory design template

2. **Migration judgment**: Confirm `SVC_WORKFLOW_AUTH_PRINCIPAL_BLOCKING_MIGRATION_REQUIRED = false`
   - Verify no non-UUID identity fields exist in the domain model
   - Verify no email/username/agentId leak in authorization paths

3. **Gap analysis**: Verify all identified gaps (profile checks, error codes, principal_type) are real and complete

4. **PR-C1 scope**: Validate that PR-C1 as defined is atomic, testable, and does not create a half-authenticated state

---

## Final State

```text
SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN_READY
```

The design freeze is complete. The document has been generated with all 30+ required sections. The codebase is already in a strong starting position — the JWKS verifier, OBO support, PrincipalId model, and provisioning API are all merged and independently audited. No blocking migrations are required.

### Current design gaps vs existing implementation

| Gap | Severity | PR |
|-----|----------|----|
| Direct token must reject `act`/`azp` | High | PR-C1 |
| OBO must enforce `client_id === azp` | High | PR-C1 |
| OBO must reject nested `act` | Medium | PR-C1 |
| `principal_type` frozen to `agent`-only | Medium | PR-C1 |
| Granular error codes | Medium | PR-C1 |
| Redirect policy hardening | Medium | PR-C1 |
| URL scheme validation | Medium | PR-C1 |
| Duplicate kid warning | Low | PR-C1 |
| Formal PrincipalContext extractor | Medium | PR-C2 |

### Fixed principles (unchanged from task specification)

```text
CANONICAL_PRINCIPAL_ID_SOURCE = token.sub
JWT_ALGORITHM_ALLOWLIST = RS256_ONLY
INVALID_RS256_FALLBACK_POLICY = FAIL_CLOSED
USER_TOKEN_SUPPORTED = false
HS256_WORKFLOW_TOKEN_SUPPORTED = false
PRODUCTION_SWITCH_ALLOWED = false
ADC_INTEGRATION_ALLOWED = false
```
