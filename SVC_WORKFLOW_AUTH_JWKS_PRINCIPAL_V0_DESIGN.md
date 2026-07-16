# svc-workflow Auth-service JWKS Verifier + Principal Context V0 — Design Freeze

```text
Status: SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN_READY
```

---

## BASE_SHA

```
4a7b3a324e97410441b3f65c01e3b27f835ad85b
```

---

## CURRENT_AUTH_MODEL

svc-workflow has a dual-mode authentication layer already merged and independently audited:

| Aspect | Detail |
|--------|--------|
| Auth modes | `test_hs256` (HS256 shared-secret, loopback-only) / `jwks` (RS256 JWKS) |
| Selection | `WORKFLOW_AUTH_MODE` env var, validated at startup |
| HS256 verifier | `src/auth/verifier.rs` — `Hs256Verifier`, rejects OBO markers |
| JWKS verifier | `src/auth/jwks_verifier.rs` — `JwksVerifier`, RS256-only, with caching |
| Dispatch | `src/http/state.rs` — `AuthVerifier` enum wrapping both; `verify()` call |

### HS256 mode gates
- `WORKFLOW_JWT_SECRET` required
- `WORKFLOW_JWKS_URL` must NOT be set
- Server binds to loopback (`127.0.0.1`)

### JWKS mode gates
- `WORKFLOW_JWKS_URL`, `WORKFLOW_JWT_ISSUER`, `WORKFLOW_JWT_AUDIENCE` all required
- `WORKFLOW_JWT_SECRET` must NOT be set
- No loopback restriction

### Verified claims (both modes)
- `sub`: must be a valid UUID, parsed as `PrincipalId`
- `iss`: must match configured issuer
- `aud`: must match configured audience
- `exp`: validated with configurable leeway
- `nbf`: validated if present
- `iat`: required
- `type`: must be `"access"` (legacy)
- `version`: must be `"v1"` (legacy)
- `principal_type`: must be `"human"` or `"agent"`
- `scope`: space-separated string, parsed into `HashSet`
- `token_use`: `"access" | "workflow_obo"` (defaults to `"access"` when absent)
- OBO: `act.sub` (UUID), `azp` (non-empty), `jti` (non-empty)

---

## CURRENT_PRINCIPAL_MODEL

| Aspect | Detail |
|--------|--------|
| Domain identity | `PrincipalId` — UUID newtype (`src/domain/ids.rs`) |
| Primary key | UUID, stored as `principals.principal_id` |
| Fields | `principal_id` (UUID PK), `principal_type` (ENUM: HUMAN/AGENT/SERVICE), `display_name`, `email` (nullable), `enabled` (boolean), `metadata` (JSONB) |
| Provisioning | Identity Provisioning API (`/internal/v1/admin/principals`) creates/reads principals |
| Principal source | `source` field tracks provenance (e.g. `"auth-service"`) |
| Principal uniqueness | Canonical ID is the UUID; no alias/email/username mapping |
| Bootstrap | Allowlisted agent can create its own principal (principal_id = JWT.sub) |
| Token canonical ID | `JWT.sub = PrincipalId.from_uuid(Uuid)` — no translation needed |

### Key: No email/username/agentId in domain authorization
- `PrincipalId` is a UUID throughout the domain layer
- `AssignerRef.fixed_principal_id` is `Option<PrincipalId>` (UUID)
- `WorkflowDefinitionVersion.published_by_principal_id` is `Option<PrincipalId>`
- Commands carry `principal_id: PrincipalId`
- No stored property references email, username, or agentId for permission decisions

**No shadow principal table, no alias mapping, no JIT provisioning path exists.**

---

## CURRENT_DOMAIN_AUTHORIZATION_MODEL

| Aspect | Detail |
|--------|--------|
| Domain owner | `domain_role_bindings` with `role_key='DOMAIN_OWNER'` |
| Domain membership | Role binding existence + enabled = active membership |
| Assignee resolution | `AssigneeRefType`: `WorkflowCreator`, `DomainOwner`, `FixedPrincipal` |
| Create authorization | Principal must exist, be enabled, have domain membership |
| Transition authorization | Principal must be current node visit assignee |
| Scope enforcement | Per-handler `require_scope()` in handler layer |
| Principal disabled | Domain-level error: `PrincipalDisabled` → 403 |

All authorization uses `principal_id: PrincipalId` as the actor key — no client_id, no azp, no act.sub.

---

## AUTH_SERVICE_CONTRACT_INPUTS

The following contracts were read and treated as frozen truth:

| Document | Path |
|----------|------|
| JWKS/OBO Auth V0 (svc-workflow) | `docs/contracts/JWKS_OBO_AUTH_V0.md` |
| Identity Provisioning API V0 | `docs/contracts/IDENTITY_PROVISIONING_API_V0.md` |
| Internal API Contract V0.1 | `docs/contracts/INTERNAL_API_CONTRACT_V0_1.md` |
| JWKS Verifier Audit Report | `SVC_WORKFLOW_JWKS_VERIFIER_AUDIT.md` |
| Identity Provisioning Audit Report | `SVC_WORKFLOW_IDENTITY_PROVISIONING_API_V0_AUDIT.md` |
| Auth-service Workflow RS256 Token + JWKS V0 | `docs/contracts/WORKFLOW_RS256_MACHINE_TOKEN_JWKS_V0.md` (cross-repo reference) |
| Auth-service Workflow Agent OBO Token Exchange V0 | `docs/contracts/WORKFLOW_AGENT_OBO_TOKEN_EXCHANGE_V0.md` (cross-repo reference) |
| Auth-service Controlled Canary reports | Referenced by name (cross-repo artifacts) |

### auth-service contract (current merged main)

Current auth-service main supports:

**Agent Direct Token:**
```json
{
  "alg": "RS256",
  "kid": "<active workflow key>",
  "aud": "svc-workflow",
  "sub": "<Agent MachinePrincipal.id>",
  "principal_type": "agent",
  "type": "access",
  "client_id": "<Agent MachineClient.clientId>",
  "azp": null,
  "act": null,
  "token_use": null
}
```

**Agent OBO Token:**
```json
{
  "alg": "RS256",
  "kid": "<active workflow key>",
  "aud": "svc-workflow",
  "sub": "<delegated Agent MachinePrincipal.id>",
  "principal_type": "agent",
  "type": "access",
  "token_use": "workflow_obo",
  "client_id": "<ADC MachineClient.clientId>",
  "azp": "<ADC MachineClient.clientId>",
  "act.sub": "<ADC MachinePrincipal.id>",
  "scope": "<precise intersection scopes>"
}
```

JWKS endpoint: `GET /.well-known/jwks.json`

---

## JWKS_URL_CONFIGURATION

| Parameter | Env Variable | Default | Required |
|-----------|-------------|---------|----------|
| JWKS URL | `WORKFLOW_JWKS_URL` | — | Yes (in jwks mode) |
| HTTP timeout | `WORKFLOW_JWKS_HTTP_TIMEOUT` | 5s | No |
| Response size limit | Hardcoded | 1 MB | N/A |
| HTTPS requirement | Enforced at config parse | HTTP allowed for test | No |
| URL scheme validation | `http`/`https` only | Rejected otherwise | Config |

### Security rules
1. URL must be parseable and use `http` or `https` scheme.
2. HTTPS is strongly recommended for production; local test may use HTTP.
3. `reqwest::Client` built with `.redirect(reqwest::redirect::Policy::none())` (post-audit fix: currently uses default redirect-following; the audit identified this as Medium finding #1).
4. Response size capped at 1 MB.
5. Only `{"keys": [...]}` shape accepted; non-JSON or oversized is rejected.
6. JWKS URL must not be attacker-controllable — configured via env var only.

---

## JWKS_CACHE_POLICY

```text
JWKS_FAIL_CLOSED_BEHAVIOR = true
JWKS_CACHE_POLICY = TTL_TOLERANT_WITH_MAX_STALE
UNKNOWN_KID_REFRESH_POLICY = CONTROLLED_SINGLE_FLIGHT
```

### Cache state
- `{ keys: Vec<JwkKey>, fetched_at: Instant }`
- Keys are `{ kid: String, decoding_key: DecodingKey }` — no raw private JWK material stored

### Cache behavior

| Condition | Action |
|-----------|--------|
| Cache empty / first request | Trigger JWKS fetch |
| Within TTL (`cache_ttl`) | Use cached keys directly |
| Beyond TTL, within max_stale | Known kid → use cached; Unknown kid → trigger single-flight refresh |
| Beyond max_stale | Cache evicted, forced refresh; failure → 503 |

### Refresh
- Single-flight via `refresh_lock: Arc<Mutex<()>>` — concurrent unknown-kid requests serialize
- Double-check pattern: re-check cache after acquiring lock before fetch
- Failed fetch does **not** update cache, does **not** extend `fetched_at`
- Successful fetch **replaces** entire key set (deleted keys immediately gone)

### Fail-closed outcomes

| Scenario | HTTP | code |
|----------|------|------|
| No cache + fetch fails | 503 | `auth_verifier_unavailable` |
| Stale cache + known kid + fetch fails | 200 (proceed) | n/a |
| Stale cache + unknown kid + fetch fails | 401 | `invalid_token` |
| Unknown kid after successful refresh | 401 | `invalid_token` |

### Startup
- Eager background fetch on `JwksVerifier::new()`; failure does not block startup
- `readyz` checks `is_ready()`: must have cached key within max_stale window

---

## UNKNOWN_KID_REFRESH_POLICY

```text
UNKNOWN_KID_REFRESH_POLICY = CONTROLLED_SINGLE_FLIGHT
```

- Token with kid not in cache → triggers refresh
- Mutex prevents concurrent fetches (single-flight)
- After refresh: if kid still missing → `401 invalid_token`
- If refresh fails: fall back to stale-cache behavior (if within max_stale, return error; if beyond, return 503)
- Max-stale cache is **never** used to honor an unknown kid — only to avoid 503 when cache is stale but known kids still work

---

## JWT_ALGORITHM_ALLOWLIST

```text
JWT_ALGORITHM_ALLOWLIST = RS256_ONLY
```

- Only `RS256` is accepted
- JWT header `alg` checked before key lookup
- `alg=none` → rejected
- `alg=HS256`/`HS384`/`HS512` → rejected
- `alg=RS384`/`RS512` → rejected
- Algorithm confusion (RSA pub key as HMAC secret) → rejected at header check
- `Validation::algorithms = vec![RS256]` provides second layer of defense

---

## DIRECT_TOKEN_PROFILE

Token must satisfy:

```text
type=access
principal_type=agent
aud=svc-workflow
token_use=access  OR  token_use absent  OR  token_use non-workflow_obo
client_id present
azp absent
act absent
```

### Rejection rules
- Token with `act` → reject (direct token must not have delegation claims)
- Token with `azp` → reject (direct token must not have authorized party)
- Token with `token_use=workflow_obo` → treat as OBO, not direct
- `token_use` absent → treat as direct (backward compat)

**Current implementation gap**: The JwksVerifier does not explicitly reject a direct token that carries `act` or `azp`. While `principal_type=agent` is enforced for direct in the HS256 verifier, the JWKS path accepts `principal_type=human` or `agent`. This design freeze formalizes that direct tokens must be `principal_type=agent` only and must reject `act`/`azp`.

---

## OBO_TOKEN_PROFILE

Token must satisfy:

```text
type=access
principal_type=agent
aud=svc-workflow
token_use=workflow_obo
client_id present
azp present
client_id === azp  (same value)
act present
act.sub = <ADC MachinePrincipal.id> (valid UUID)
jti present (non-empty)
```

### Rejection rules
| Condition | Reason |
|-----------|--------|
| Missing `act` | No delegation anchor |
| Missing `azp` | No authorized party |
| `client_id != azp` | Misbound delegation |
| `act.sub` not UUID | Invalid actor reference |
| Nested `act` | Recursive delegation not supported |
| Missing `jti` | Replay prevention required |
| Unknown `token_use` | Must be exactly `workflow_obo` |

**Current implementation gap**: The `client_id === azp` check is not explicitly enforced. The `validate_obo()` function checks `act`, `azp`, and `jti` but not `client_id === azp`. Additionally, `nested_act` is not checked (current `ActClaim` only deserializes `sub`, so extra fields are silently ignored by serde; no recursive `act` detection).

---

## CANONICAL_PRINCIPAL_ID_SOURCE

```text
CANONICAL_PRINCIPAL_ID_SOURCE = token.sub
```

- **DIRECT**: `principalId = token.sub` (Agent MachinePrincipal.id)
- **OBO**: `principalId = token.sub` (delegated Agent Principal.id) — NOT `act.sub`
- `act.sub` is used for audit only
- `client_id`, `azp` are never the domain principal
- `sub` must be a non-empty canonical UUID

### Validation chain
1. JWT header signature + kid → verified
2. Claims: iss, aud, exp, nbf → verified
3. `sub` parsed as UUID → `PrincipalId::from_uuid`
4. Parsed `PrincipalId` is the sole identity passed to domain commands
5. No body/query/path can override `principal_id`

---

## PRINCIPAL_CONTEXT_SCHEMA

```rust
/// Canonical principal context — the sole identity source for domain authorization.
struct PrincipalContext {
    // --- Core identity (both direct & OBO) ---
    principal_id: PrincipalId,          // token.sub (UUID)
    principal_type: String,             // "agent"
    auth_mode: AuthModeVariant,         // Direct | Obo
    scopes: HashSet<String>,            // exact scope set from token
    
    // --- Token metadata ---
    token_jti: Option<String>,          // unique token identifier
    issuer: String,                     // verified iss
    audience: String,                   // verified aud
    expires_at: Option<DateTime<Utc>>,  // exp
}
```

### Rules
1. `principal_id` always comes from `token.sub` — never from `act.sub`, `azp`, `client_id`, body, or query.
2. Context is constructed from **verified token claims only**.
3. Context is immutable after construction.
4. Downstream code must not re-parse the `Authorization` header.
5. Context must not be overridable by request body/query.
6. direct and OBO share the same `PrincipalContext` schema; OBO adds `ActorContext`.
7. Full JWT is not carried in the context.
8. Claims fields not listed in the schema are not propagated to the domain layer.

### Relation to current `AuthContext`
The current `AuthContext` (`src/auth/auth_context.rs`) is close but needs:
- Rename `delegating_principal_id` to `actor_principal_id` for consistency with design terminology
- Add `auth_mode: AuthModeVariant` discriminator
- Add `expires_at` for downstream expiry checks
- Ensure `scope` is exposed as `HashSet<String>` not just raw string

---

## ACTOR_CONTEXT_SCHEMA

Available only when `auth_mode == Obo`:

```rust
struct ActorContext {
    actor_principal_id: PrincipalId,    // token.act.sub (ADC MachinePrincipal.id)
    authorized_client_id: String,       // token.azp / token.client_id (same value)
}
```

### Rules
1. actor_principal_id is for **audit, diagnostics, and policy only**.
2. actor_principal_id must NOT be used as the domain `principal_id`.
3. authorized_client_id is for logging and future rate-limiting / policy.
4. Actor context does NOT grant any scope, role, or authorization rights.

---

## AUTHORIZED_CLIENT_CONTEXT

```rust
struct AuthorizedClientContext {
    client_id: String,           // token.client_id (always present for workflow tokens)
    authorized_party: String,    // token.azp (OBO only)
    client_id_eq_azp: bool,      // always true for OBO, n/a for direct
}
```

- `client_id` is logged for all tokens
- `azp` is logged for OBO tokens
- `client_id === azp` is enforced for OBO as a security invariant
- Neither field is used for domain authorization

---

## SCOPE_AUTHORIZATION_MODEL

### Token scope vocabulary (defined by auth-service)

| Scope | Intended access |
|-------|----------------|
| `workflow.read` | Read-only operations |
| `workflow.execute` | Write operations (create, transition) |
| `workflow.admin` | Provisioning operations |

### Scope-to-route mapping (frozen)

| Endpoint | Required scope |
|----------|---------------|
| `POST /internal/v1/workflow-instances` | `workflow.execute` |
| `POST /internal/v1/workflow-instances/{id}/transitions` | `workflow.execute` |
| `POST /internal/v1/workflow-instances/{id}/context` | `workflow.execute` |
| `POST /internal/v1/workflow-instances/{id}/revise-and-transition` | `workflow.execute` |
| `GET /internal/v1/workflow-instances/{id}` | `workflow.read` |
| `GET /internal/v1/workflow-instances/{id}/timeline` | `workflow.read` |
| `POST /internal/v1/admin/principals` | `workflow.admin` |
| `GET /internal/v1/admin/principals/{id}` | `workflow.admin` |
| `POST /internal/v1/admin/domains` | `workflow.admin` |
| `GET /internal/v1/admin/domains/{id}` | `workflow.admin` |
| `PUT /internal/v1/admin/domains/{id}/role-bindings/{pid}` | `workflow.admin` |
| `DELETE /internal/v1/admin/domains/{id}/role-bindings/{pid}` | `workflow.admin` |
| `PUT /internal/v1/admin/domains/{id}/owner` | `workflow.admin` |
| `GET /internal/v1/admin/definition-versions/{id}` | `workflow.admin` |

### Scope rules
1. Scope matching is **exact** (`HashSet::contains`) — no `includes()` substring match.
2. Unknown scopes in token are ignored (not rejected).
3. Empty scope set → deny all scope-gated routes (401/403 depending on context).
4. Direct and OBO tokens use the same scope rules.
5. Actor (`act.sub`) cannot gain additional scope beyond what the OBO token carries.

---

## DOMAIN_AUTHORIZATION_COMPOSITION

```text
Token scope at auth layer
    AND
Workflow domain authorization (identity membership, assignee check, permission)
```

Both must pass for the request to proceed.

### Auth layer gates (checked in handler before domain call)
| Gate | Implementation |
|------|---------------|
| Token valid (signature, claims) | `JwksVerifier::verify()` or `Hs256Verifier::verify()` |
| Scope sufficient | `require_scope()` in handler |
| Profile valid (direct/obo) | Profile check in verifier |

### Domain layer gates (checked in application/domain service)
| Gate | Implementation |
|------|---------------|
| Principal exists | DB lookup |
| Principal enabled | `enabled` flag check |
| Domain exists | DB lookup |
| Domain enabled | `enabled` flag check |
| Domain membership | Role binding exists |
| Assignee match | Current visit assignee == principal |
| Version published | Definition version status check |

### Security invariant
`act.sub` can never bypass any domain layer gate. The ADC actor's identity is not material to workflow domain authorization.

---

## LEGACY_COEXISTENCE_MODEL

```text
WORKFLOW_AUTH_MODE = required (no default — must be explicitly set)
ACTIVATION_GATE = WORKFLOW_AUTH_MODE=jwks
```

### Phase 1: Dual-stack (current state)
- `WORKFLOW_AUTH_MODE` selects mode at startup
- `test_hs256` → HS256 verifier (loopback only)
- `jwks` → JWKS verifier
- Both modes co-exist in code, mutually exclusive at runtime
- No fallback between modes

### Legacy callers (HS256-dependent)
- Local dev instances
- Isolated integration smoke tests
- ADC direct (pre-JWKS) test flows

### Migration to JWKS-as-default
1. Feature flag: `WORKFLOW_AUTH_MODE` variable
2. Default stays `test_hs256` until operator explicitly sets `jwks`
3. Controlled canary: single instance runs `jwks` mode
4. Shadow: all instances run `jwks`; legacy HS256 traffic continues in parallel
5. Cutover: new deployments default to `jwks`; `test_hs256` decommissioned
6. Production default switch only when:
   - `PRODUCTION_DEPLOYMENT_ALLOWED` gate lifted
   - Auth-service JWKS + OBO in production
   - All legacy callers migrated

### Transition invariants
- A request with `Authorization: Bearer <RS256 token>` that fails in `jwks` mode does **not** fall back to `test_hs256`
- A request in `jwks` mode that fails JWKS verification → `401` with no legacy fallback
- A request in `test_hs256` mode that fails HS256 verification → `401` with no JWKS fallback
- No auto-detection of token type before verification

---

## INVALID_RS256_FALLBACK_POLICY

```text
INVALID_RS256_FALLBACK_POLICY = FAIL_CLOSED
```

- Invalid RS256 token in `jwks` mode → `401` (never fall back to HS256)
- Invalid RS256 token in `test_hs256` mode → `401` (HS256 verifier rejects RS256)
- No token → `401` (in both modes)
- Unreadable token → `401`
- JWKS unavailable → `503`
- Unknown kid after refresh → `401`
- No "try HS256 if RS256 fails" path exists
- The mode gates prevent both configurations being loaded simultaneously

---

## ERROR_CONTRACT

### Auth-layer errors (frozen)

| HTTP | `code` | Meaning | Retryable | Notes |
|------|--------|---------|-----------|-------|
| 401 | `missing_token` | No `Authorization: Bearer` header | No | |
| 401 | `malformed_token` | Unparseable JWT | No | Replaces generic `invalid_token` |
| 401 | `algorithm_not_allowed` | Non-RS256 algorithm | No | |
| 401 | `unknown_kid` | kid not in JWKS after refresh | No | |
| 503 | `jwks_unavailable` | JWKS fetch failed, no usable cache | Yes | |
| 401 | `bad_signature` | Signature verification failed | No | |
| 401 | `wrong_issuer` | `iss` mismatch | No | |
| 401 | `wrong_audience` | `aud` mismatch | No | |
| 401 | `token_expired` | `exp` in the past | No | |
| 401 | `token_not_yet_valid` | `nbf` in the future | No | |
| 401 | `invalid_token_type` | `type` not `"access"` | No | |
| 401 | `invalid_principal_type` | `principal_type` not `"agent"` | No | |
| 401 | `invalid_direct_profile` | Direct token has `act`/`azp` | No | New |
| 401 | `invalid_obo_profile` | OBO missing required claims | No | New |
| 401 | `invalid_actor` | `act.sub` invalid or nested | No | New |
| 401 | `invalid_client_claims` | `client_id !== azp` | No | New |
| 401 | `invalid_scope` | Scope format invalid | No | |
| 403 | `insufficient_scope` | Missing required scope | No | |
| 404 | `principal_not_found` | Principal not provisioned | No | Domain error |
| 403 | `principal_disabled` | Principal is disabled | No | Domain error |
| 403 | `forbidden` | General access denied | No | |

### Error response envelope
```json
{
  "error": {
    "code": "invalid_direct_profile",
    "message": "direct token must not carry delegation claims",
    "details": null
  }
}
```

### Security rules
- Error messages do not leak: key material, token content, JWKS internal state, network details
- `principal_not_found` vs `insufficient_scope`: same HTTP 401/403 category to prevent enumeration (currently 404 vs 403 — evaluate whether to normalize to 401 for auth failures)
- Detailed claim names in `missing_claim` are limited to claim names only, not values

### Current implementation gap
The current error contract uses fewer codes:
- `invalid_token` covers algorithm, kid, signature, issuer, audience, OBO validation failures
- `missing_claim` covers required claim absence
- The design freezes more granular codes for better diagnosability without increasing leakage

---

## AUDIT_MODEL

### Structured audit log record (per authenticated request)

| Field | Source | Always | Notes |
|-------|--------|--------|-------|
| `request_id` | `x-request-id` header | Yes | Correlation ID |
| `jti` | `token.jti` | OBO only | `"-"` for direct |
| `principal_id` | `token.sub` | Yes | Canonical principal |
| `principal_type` | `token.principal_type` | Yes | Always `"agent"` |
| `auth_mode` | Computed | Yes | `"direct"` or `"obo"` |
| `actor_principal_id` | `token.act.sub` | OBO only | `"-"` for direct |
| `authorized_client_id` | `token.azp` | OBO only | `"-"` for direct |
| `scopes` | `token.scope` | Yes | Space-separated |
| `endpoint` | Request method + path | Yes | No query params |
| `result` | Computed | Yes | `"allowed"` or denial code |

### Never logged
- Full JWT
- `Authorization` header value
- JWK modulus (`n`) or exponent (`e`)
- Any private key material
- Raw JWT payload beyond documented fields
- Client secrets

### Persistence model
Current audit is `tracing::info!` → structured log output (stdout/log aggregator). This is **not** a persistent audit ledger (no DB table). This is explicitly acceptable for V0. A persistent audit table must be designed before production cutover.

---

## MIGRATION_REQUIRED

```text
SVC_WORKFLOW_AUTH_PRINCIPAL_BLOCKING_MIGRATION_REQUIRED = false
```

### Assessment
- **Current `PrincipalId` is already UUID** — no type change needed
- **Owner/Assignee/Reviewer fields are already `PrincipalId`** — no column type change needed
- **No email/username/agentId used for authorization** — no alias mapping needed
- **Provisioning API already creates principals from UUID** — no shadow principal table needed
- **No JIT provisioning needed** — principals must be pre-provisioned
- **No `display_name` from token** — display name is set during provisioning, not derived from JWT
- **Existing principals in DB already use UUID** — all existing data compatible

### No migration required for:
- Principal table schema
- Domain table schema
- Role bindings table schema
- Workflow definition/instance tables (store `PrincipalId` as UUID)
- Command receipt table (stores `PrincipalId` as UUID)

### Future optional additions (not migration-blocking)
- Audit table for persistent delegation records (pre-production)
- Actor identity field on workflow events (pre-production)

---

## PROVISIONING_REQUIREMENTS

### Current state (already implemented)
- Identity Provisioning API exists and is audited
- Bootstrap flow: allowlisted agent creates own principal
- Principal types: HUMAN, AGENT
- Principal: UUID PK, type, enabled, source, source_revision
- Domain provisioning with owner replacement
- Role bindings: DOMAIN_OWNER, WORKFLOW_ADMIN

### Token-driven requirements
- Principal must be provisioned **before** token can be used for domain operations
- `principal_not_found` is returned if principal does not exist (not auto-created)
- `principal_disabled` is returned if principal exists but is disabled
- JIT principal creation from token: **NOT supported** for V0
- Actor principal (for OBO `act.sub`) does NOT need local provisioning record
- ADC identity records not required for V0 (audit-only)

### Bootstrap sequence
1. Auth-service creates Agent MachinePrincipal + MachineClient
2. Agent provisions itself in svc-workflow via `POST /internal/v1/admin/principals` with `principal_id = JWT.sub`
3. Agent is granted `WORKFLOW_ADMIN` role binding (or domain owner via separate provisioning)
4. Agent can now create workflow instances and transition

---

## CONFIGURATION_REQUIREMENTS

### New or changed configuration variables

| Variable | Mode | Required | Default | Purpose |
|----------|------|----------|---------|---------|
| `WORKFLOW_AUTH_MODE` | both | Yes | — | `test_hs256` or `jwks` |
| `WORKFLOW_JWKS_URL` | jwks | Yes | — | JWKS endpoint URL |
| `WORKFLOW_JWT_ISSUER` | jwks | Yes | — | Expected `iss` |
| `WORKFLOW_JWT_AUDIENCE` | jwks | Yes | — | Expected `aud` |
| `WORKFLOW_JWKS_CACHE_TTL` | jwks | No | 300 | Cache TTL (seconds) |
| `WORKFLOW_JWKS_HTTP_TIMEOUT` | jwks | No | 5 | HTTP timeout (seconds) |
| `WORKFLOW_JWKS_MAX_STALE` | jwks | No | 600 | Max stale window (seconds) |
| `WORKFLOW_JWT_CLOCK_SKEW` | both | No | 60 | Leeway (seconds) |
| `WORKFLOW_JWT_SECRET` | test_hs256 | Yes | — | HS256 shared secret |
| `WORKFLOW_PROVISIONING_PRINCIPAL_IDS` | both | Yes | — | Provisioning allow-list |
| `WORKFLOW_BIND_ADDR` | both | No | `127.0.0.1` | Bind address |
| `WORKFLOW_PORT` | both | No | 8989 | Port |

### New configuration for PR-C1
*(already partially implemented)*

No new env vars needed for the auth design — all are already defined in `auth_mode.rs`.

### Production gating (env var switches)

| Variable | Current value | Meaning |
|----------|--------------|---------|
| `PRODUCTION_DEPLOYMENT_ALLOWED` | `no` | Blocks production deployment |
| `SVC_WORKFLOW_CONSUMER_SWITCH_ALLOWED` | `no` | Blocks consumer-mode switch |
| `ADC_INTEGRATION_ALLOWED` | `no` | Blocks ADC integration |
| `REAL_PROVISIONING_ALLOWED` | `no` | Blocks real provisioning |
| `USER_OBO_IMPLEMENTATION_ALLOWED` | `no` | Blocks user OBO |

---

## RECOMMENDED_PR_SEQUENCE

### PR-C1: JWKS Client + RS256 Verifier hardening

| Aspect | Detail |
|--------|--------|
| **External behavior** | RS256 JWKS verification with caching, unknown-kid refresh, fail-closed |
| **Changes** | `JwksVerifier` refinements: add redirect policy, URL scheme validation, duplicate-kid warning, Direct/OBO profile checks |
| **Config** | `WORKFLOW_AUTH_MODE=jwks` requirement, `WORKFLOW_JWKS_*` vars |
| **Tests** | Extend `jwks_auth.rs` with Direct/OBO profile rejection tests, `client_id===azp` check, nested act, principal_type=agent-only |
| **Default enabled** | No — requires explicit `WORKFLOW_AUTH_MODE=jwks` |
| **Rollback** | Switch `WORKFLOW_AUTH_MODE` back to `test_hs256` |
| **Depends on** | Auth-service JWKS endpoint deployed |
| **Does not include** | Principal Context, Scope Guard refactor, Audit persistence |

### PR-C2: Direct/OBO Profile + Principal Context

| Aspect | Detail |
|--------|--------|
| **External behavior** | PrincipalContext and ActorContext as typed request extractors; Direct/OBO profile checks at verifier level |
| **Changes** | Formalize `PrincipalContext` extractor, `ActorContext` for OBO, profile validation in JwksVerifier |
| **Config** | No new env vars |
| **Tests** | PrincipalContext assertions, ActorContext audit-only tests |
| **Default enabled** | Always on when `jwks` mode is active |
| **Rollback** | Revert PR-C2 |
| **Depends on** | PR-C1 |
| **Does not include** | Route Scope Guard, legacy dual-stack changes |

### PR-C3: Route Scope Guard + Legacy Dual-Stack

| Aspect | Detail |
|--------|--------|
| **External behavior** | Central scope-to-route guard; explicit legacy coexistence contracts |
| **Changes** | Centralized scope middleware (axum layer), formalize legacy fallback policy |
| **Config** | No new env vars |
| **Tests** | Scope matrix tests, legacy coexistence behavior tests |
| **Default enabled** | Yes — both modes already gated by `WORKFLOW_AUTH_MODE` |
| **Rollback** | Revert scope guard layer |
| **Depends on** | PR-C1, PR-C2 |
| **Does not include** | Audit persistence, Controlled Canary |

### PR-C4: Audit + Controlled Canary

| Aspect | Detail |
|--------|--------|
| **External behavior** | Structured audit with full PrincipalContext/ActorContext fields |
| **Changes** | Audit log completeness verification, Controlled Canary test suite |
| **Config** | No new env vars |
| **Tests** | Full canary matrix (Direct/OBO success, profile attacks, alg confusion, JWKS failure, scope enforcement, legacy coexistence, leak verification) |
| **Default enabled** | Yes |
| **Rollback** | Revert audit changes if needed |
| **Depends on** | PR-C1, PR-C2, PR-C3 |
| **Does not include** | Production deployment, ADC integration |

### Sequence safety
- PR-C1 does not make auth decisions for domain operations alone (still requires scope check)
- PR-C2 does not change scope enforcement (scope still checked in handlers)
- PR-C3 does not change audit semantics
- No intermediate PR creates an "authenticated but open" state
- Each PR is independently testable and auditable

---

## FUTURE_TEST_MATRIX

### Direct success
1. Valid Direct RS256 token → verified, principal_id = sub
2. Token with `workflow.read` scope → read endpoints allowed
3. Token with `workflow.execute` scope → write endpoints allowed
4. canonical `principalId = sub`

### OBO success
5. Valid OBO token → verified, principal_id = sub
6. `actorPrincipalId = act.sub` captured
7. `authorizedClientId = azp = client_id`
8. Actor (act.sub) does NOT replace sub as principal
9. OBO token scopes are the intersection, not the ADC's full scopes

### Signature / JWKS
10. Active kid → verified
11. Previous kid in rotation window → verified (if still in JWKS)
12. Unknown kid after refresh → 401
13. JWKS unavailable, no cache → 503
14. Cache hit (within TTL) → verified
15. Cache expired, within max_stale, known kid → verified
16. Algorithm confusion (HS256 token in jwks mode) → rejected
17. Wrong signature → rejected
18. Wrong issuer / audience → rejected
19. Expired / future nbf token → rejected

### Profile attacks
20. HS256 token → rejected in jwks mode
21. Direct token with `act` → rejected
22. Direct token with `azp` → rejected
23. OBO missing `act` → rejected
24. OBO missing `azp` → rejected
25. OBO `client_id !== azp` → rejected
26. Nested `act` → rejected
27. Human principal_type token → rejected (V0 direct only accepts agent; OBO also agent)
28. Unknown `principal_type` → rejected
29. Unknown `token_use` → rejected
30. Claim type errors (string where UUID expected) → rejected

### Authorization
31. Missing scope → 403
32. Substring scope match attempt → 403
33. `workflow.read` token calling `workflow.execute` endpoint → 403
34. Token scope OK, but principal not in domain → 404/403
35. Token scope OK, but principal disabled → 403
36. ADC actor has domain membership but OBO subject doesn't → subject denied
37. Same subject via direct and OBO → same domain identity

### Legacy
38. Legacy HS256 request unchanged in `test_hs256` mode
39. Bearer RS256 failure does not fallback to HS256 in `jwks` mode
40. Unknown auth scheme → 401
41. Default behavior in dual-stack period
42. Mode switch → immediate effect, no stale sessions

### Leak / Security
43. Error response contains no token material
44. Error response contains no Authorization header
45. Error response contains no JWK private material
46. Audit log fields complete; no sensitive material logged

---

## RISKS_AND_OPEN_DECISIONS

### Open decisions
1. **Error normalization**: Should `principal_not_found` (currently 404) become 401 to prevent ID enumeration? Decision: keep 404 for V0 — principal provisioning is a control-plane operation, 404 vs 403 enumeration is not the primary threat model for V0.

2. **Actor identity in DB**: Should OBO actor (`act.sub`) be stored in a new `security_audit_events` table? Decision: defer to pre-production — V0 uses structured logs only.

3. **Scope middleware vs per-handler**: Should scope enforcement be a centralized axum middleware layer instead of per-handler `require_scope()`? Decision: defer to PR-C3 — current approach is explicit and auditable.

4. **Profile check location**: Should Direct/OBO profile validation live in `JwksVerifier::verify()` or in a separate layer? Decision: live in the verifier for V0 — single responsibility, verified claims produce validated profiles.

5. **`human` principal_type support in V0**: The task says V0 supports `agent` only. Current code accepts both `human` and `agent`. Decision: freeze to `agent`-only for V0 formal contract; human support added as future work.

### Risks
| Risk | Mitigation |
|------|-----------|
| Auth-service JWKS format changes | Contract frozen; svc-workflow only consumes published JWKS shape |
| HS256 shared-secret rotation | Not supported in test_hs256 mode; JWKS mode uses key rotation natively |
| JWKS endpoint network partition | Cached keys serve within TTL/max-stale; 503 on fresh requests |
| Cache poisoning via compromised JWKS | HTTPS + URL allowlist + key filtering (RSA/sig/RS256 only) |
| Concurrent startup with empty cache | Eager fetch non-blocking; first request may block on fetch |
| Feature flag misconfiguration | Mode gates validated at startup; conflicting config aborts startup |

---

## IMPLEMENTATION_READY

```text
SVC_WORKFLOW_AUTH_JWKS_PRINCIPAL_V0_DESIGN_READY
```

### Rationale
- Current codebase already has the core JWKS verifier, OBO support, PrincipalId-as-UUID model, and provisioning API merged and audited
- No blocking migrations required
- All gaps identified with clear PR sequences
- Security invariants (principalId = sub, act audit-only, RS256-only, fail-closed) are already implemented
- The design formalizes hardening items from the previous audit (redirect policy, URL scheme, duplicate-kid handling)
- Profile checks (Direct/OBO) and principal_type=agent-only need implementation but are well-defined

### Remaining gaps to implement
1. Direct token must reject `act` / `azp` — add profile check in `JwksVerifier::verify()`
2. OBO token must enforce `client_id === azp` — add in `validate_obo()` or JWKS path
3. OBO must reject nested `act` — add recursive check
4. `principal_type` frozen to `agent`-only for V0 — update `validate_principal_type()` or JWKS verify
5. Error contract: add granular codes (e.g. `invalid_direct_profile`, `invalid_obo_profile`, `invalid_actor`)
6. Redirect policy: add `reqwest::redirect::Policy::none()` to client builder
7. URL scheme validation: add `http`/`https` check at config parse
8. Duplicate kid: add `tracing::warn!` on collision
9. Formal `PrincipalContext` extractor as typed replacement for raw `AuthenticatedPrincipal` usage in handlers
