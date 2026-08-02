# svc-workflow JWKS / OBO Verifier + Dual Auth Mode V0 — Independent Audit

```text
Audit Agent       : independent security audit (ZCode)
Repository        : svc-workflow
Audit worktree    : svc-workflow-jwks-verifier
Implementation PR : feat/jwks-obo-verifier-v0
Base SHA          : f3306a5d387aa4159a995b7477e4c9da1a7193b7
Audited HEAD      : 300818f06efa7b090fcbeca81d9a8919a289df89
Date              : 2026-07-16
Contract          : docs/contracts/JWKS_OBO_AUTH_V0.md (FROZEN_FOR_STAGE_1_AUTHENTICATED_SMOKE)
```

> Note on scope: the task brief referenced `docs/contracts/INTERNAL_HTTP_API_V0.md`,
> which does not exist in this worktree (the internal API contract lives at
> `docs/contracts/INTERNAL_API_CONTRACT_V0_1.md` from a prior PR and is
> unchanged here). This is a documentation-name mismatch in the brief, not an
> implementation defect. The audit proceeded against `JWKS_OBO_AUTH_V0.md`,
> which is the contract this PR introduces.

---

## 1. Files changed (17)

```
M  Cargo.lock
M  Cargo.toml
A  docs/contracts/JWKS_OBO_AUTH_V0.md
A  src/auth/auth_context.rs
A  src/auth/auth_mode.rs
A  src/auth/claims.rs
A  src/auth/jwks_verifier.rs
M  src/auth/mod.rs
M  src/auth/principal.rs
M  src/auth/verifier.rs
M  src/http/handlers/health.rs
M  src/http/state.rs
M  src/main.rs
M  tests/17_workflow_runtime.rs
M  tests/17_workflow_runtime/http/e2e/server.rs
A  tests/17_workflow_runtime/http/jwks_auth.rs
M  tests/17_workflow_runtime/http/smoke.rs
```

`migrations/` diff: **empty** (verified). Domain/application layers: **untouched**.

---

## 2. Contract conformance

| Contract clause                                | Status | Evidence |
|------------------------------------------------|--------|----------|
| Dual mode via `WORKFLOW_AUTH_MODE`             | ✅ | `auth_mode.rs:21-38` |
| test_hs256 gates (secret req, no JWKS, loopback) | ✅ | `auth_mode.rs:135-167` |
| jwks gates (URL/iss/aud req, no secret)        | ✅ | `auth_mode.rs:157-167`, `auth_mode.rs:73-119` |
| RS256 only; HS256/none/other rejected          | ✅ | `jwks_verifier.rs:119-130`, `jwks_verifier.rs:136-145` |
| kid required in header                         | ✅ | `jwks_verifier.rs:127-130` |
| Claims: iss/aud/exp/nbf/sub/principal_type/token_use/scope | ✅ | `jwks_verifier.rs:136-198` |
| OBO: act.sub UUID, azp/jti non-empty           | ✅ | `claims.rs:111-130` |
| Actor = JWT.sub always                         | ✅ | `jwks_verifier.rs:174-176`, dynamic exp. 5 |
| Scope enforcement per endpoint                 | ✅ | `handlers/mod.rs:12-21` + handlers |
| Cache TTL / max-stale / controlled refresh     | ✅ | `jwks_verifier.rs:233-303` |
| Refresh mutex (single-flight)                  | ✅ | `jwks_verifier.rs:278-303` |
| Fail-closed table (503/401/200)                | ✅ | `jwks_verifier.rs:256-275` |
| JWKS key filtering (RSA/sig/RS256/kid/n,e)     | ✅ | `jwks_verifier.rs:336-369` |
| No private key material accepted               | ✅ | `RawJwk` omits private fields; exp. 8 |
| readyz auth check                              | ✅ | `health.rs:49-55` |
| Error envelope (401/403/503)                   | ✅ | `error.rs` + verifier mappings |
| Structured audit log fields                    | ✅ | `auth_context.rs:46-60` |
| Never log JWT/signature/header/secret          | ✅ | log field audit |

All Stage-1 contract clauses are satisfied.

---

## 3. Actor audit (the central security invariant)

**Invariant: domain `principal_id` == verified `JWT.sub`, exclusively.**

Traced end-to-end:
- `AuthenticatedPrincipal` is constructible only inside the `auth` module
  (`new_with_context` is `pub(crate)`, `principal.rs:28`). `scopes` is private.
- JWKS path: `parse_subject(&claims.sub)` runs **after** signature, alg, kid,
  iss, aud, exp, nbf are all verified (`jwks_verifier.rs:116-176`). The parsed
  `principal_id` is the sole value passed to the constructor.
- HS256 path: identical, `Uuid::parse_str(sub)` → `PrincipalId` (`verifier.rs:136-161`).
- The `FromRequestParts` extractor (`principal.rs:45-96`) reads **only** the
  `Authorization: Bearer` header; it never consults body/path/query/other headers
  for identity.
- All four handlers consume `principal.principal_id` and nothing else from the
  principal: `instances.rs:42,84`, `transitions.rs:32`, `timeline.rs:27`.
- Command structs carry `principal_id: PrincipalId`, populated solely from
  `principal.principal_id`.
- **`act.sub` / `delegating_principal_id` / `authorized_party` / `token_id` are
  referenced only inside `src/auth/`** (definition + `log_audit`). Zero handler,
  application, domain, or store code reads them. Confirmed by repo-wide grep.

Body/path/query cannot supply identity: request DTOs use `deny_unknown_fields`
and declare no identity field (`dto.rs:13,46,80`); a unit test
`actor_fields_are_rejected` pins this (`dto.rs:124-134`). Path params carry only
`workflowInstanceId`; query only `after`/`limit`.

**Dynamic proof (experiment 5):** an OBO token with `sub=A`, `act.sub=B` yields
`principal_id == A` and `delegating_principal_id == Some(B)`. `act.sub` never
becomes the actor. **PASS.**

---

## 4. Algorithm audit

- Header `alg` is pinned to `RS256` **before** any key lookup
  (`jwks_verifier.rs:119-125`). Non-RS256 header → `401 invalid_token`.
- `Validation::new(RS256)` plus `validation.algorithms = vec![RS256]`
  (`jwks_verifier.rs:136-137`) makes `jsonwebtoken` reject any token whose actual
  signing algorithm differs, independent of the JWK `alg`.
- **Dynamic (exp 1):** RS384 and RS512 tokens rejected (`invalid_token`).
- **Dynamic (exp 2):** hand-crafted `alg=none` token rejected.
- **Dynamic (exp 3):** RSA-public-key-as-HMAC-secret confusion token (HS256
  signed with the JWK modulus as secret) rejected — fails at header-alg check.

No algorithm-confusion vector found. **PASS.**

---

## 5. kid / JWK audit

- Missing/empty `kid` in header → rejected (`jwks_verifier.rs:127-130`).
- Unknown `kid` triggers exactly one coordinated refresh
  (`refresh_and_find`, single-flight under `refresh_lock`); still unknown after
  refresh → `401` (`jwks_verifier.rs:256-275`).
- JWK acceptance filter (`fetch_jwks:336-369`): `kty=RSA`, `use∈{sig,∅}`,
  `alg∈{RS256,∅}`, non-empty `kid`, non-empty `n`/`e`, and
  `DecodingKey::from_rsa_components` must succeed. Invalid keys skipped.
- **Private params:** `RawJwk` declares no `d`/`p`/`q`/`dp`/`dq`/`qi` fields, so
  serde ignores them; they are never stored, logged, or used. **Dynamic (exp 8):**
  a JWK carrying bogus `d`/`p`/`q` still validates the real signature. **PASS.**
- **Duplicate kid:** two JWKs sharing a `kid` are both stored; `find_key` returns
  the first. **Dynamic (exp 9):** a wrong-signature token is still rejected under
  duplicate kids — no bypass. Deviation from contract wording ("duplicate keys
  silently skipped with a warning") → **Low** (see §25).

---

## 6. Cache and max-stale

- Cache state: `{ keys, fetched_at: Instant }`. `is_ready` uses
  `fetched_at.elapsed() <= max_stale` as the readiness anchor (`jwks_verifier.rs:96-104`).
- Within TTL: known kid served from cache, no fetch (`lookup_key:240-245`).
- Beyond TTL, within max-stale: known kid still served; unknown kid triggers one
  refresh (`lookup_key:246-253`, `refresh_and_find`).
- Beyond max-stale: cache treated as miss → forced refresh; fetch failure → 503.
- max-stale is anchored to `fetched_at` (last successful load), **not** extended
  by ongoing access. No infinite-stale-life bug.
- Refresh failure path (`fetch_jwks` returns `Err`) does **not** overwrite the
  existing cache (`write` only happens on full success, `jwks_verifier.rs:376-381`),
  does **not** update `fetched_at`, and does **not** lock permanently (mutex is
  scoped, released on drop). **PASS.**

**Fail-closed table verified** against contract §5: no-cache+fail→503,
stale+known+fail→200, stale+unknown+fail→401, unknown-after-refresh→401.

---

## 7. Rotation / key deletion

**This was the audit's focus and the implementation is correct.**

A successful `fetch_jwks` **replaces** the entire key set
(`*guard = Some(JwksCacheState { keys, fetched_at: Instant::now() })`,
`jwks_verifier.rs:376-381`). Therefore a key removed from the server's JWKS is
**immediately** removed from the client cache on the next successful refresh —
it is **not** retained until max-stale.

**Dynamic (exp 6):** after rotating the mock JWKS to a brand-new kid and forcing
a refresh, a token signed with the *old* kid is rejected immediately. This is the
secure behavior the brief required ("once successfully refreshed, deleted keys
must not linger"). The contract's "old key usable in max-stale window" applies
**only** while no successful refresh has occurred — the correct interpretation.

No long-term retention of deleted keys. **PASS (no High).**

---

## 8. Auth mode gating

- `WORKFLOW_AUTH_MODE` required, only `test_hs256`/`jwks` accepted; unknown →
  startup error (`auth_mode.rs:21-38`).
- test_hs256: `WORKFLOW_JWKS_URL` must be unset; bind must be loopback
  (`auth_mode.rs:138-156`). Failure → `Err(String)`.
- jwks: `WORKFLOW_JWKS_URL`/`ISSUER`/`AUDIENCE` required & non-empty;
  `WORKFLOW_JWT_SECRET` must be unset — **no HS256 fallback**
  (`auth_mode.rs:73-119`, `auth_mode.rs:157-167`).
- All gate failures propagate through `HttpConfig::from_env` → `main.rs:21-23`
  as an `io::Error` that **aborts startup** (not a warning). Verified.
- `IpAddr::is_loopback()` accepts only `127.0.0.1` and `::1`; rejects `0.0.0.0`,
  `::`, public IPs. No string/case bypass (parsing goes through `IpAddr::parse`).

No Blocker/High. See §23 (Medium) for the defense-in-depth note on production
discrimination.

---

## 9. readyz semantics

- `healthz`: pure liveness, returns `{"status":"ok"}`, no JWKS/DB access
  (`health.rs:12-14`). ✅ matches contract.
- `readyz`: DB + migration ledger checks, then `auth_verifier.is_ready()`
  (`health.rs:49-55`). Failure → `503 auth_verifier_unavailable` /
  `"authentication verifier is not ready"` — **no URL, key, or network detail**.
- `is_ready` states: first-load success→200, first-load fail→503, cache
  valid→200, beyond max-stale→503. Recovery (next successful fetch) → 200.
- Contract nuance (recorded, not a defect): "TTL expired, within max-stale, JWKS
  temporarily down" → `is_ready()==true` → readyz 200, while per-token unknown-kid
  verifications still 401. Consistent staged behavior.

**PASS.**

---

## 10. Claim validation

Dynamically/exhaustively verified the contract's claim matrix:
wrong iss, wrong aud, expired exp, missing exp, nbf-in-future, missing sub,
non-UUID sub, wrong principal_type, missing/unknown token_use, OBO missing
act/azp/jti — all rejected with stable `401` codes (PR tests + experiments).

`aud` is matched as a single configured string via `set_audience`; `scope` is
space-split into a `HashSet`. `iat` is in `required_spec_claims`. Legacy
`type=access`/`version=v1` enforced by `require_legacy_claims` (shared by both
verifiers). No unfrozen claim encoding is hard-wired in a way that would block
auth-service; iss/aud are operator-configurable. **No cross-repo protocol
conflict (no High).**

---

## 11. Error semantics and leakage

- All verifier errors are `ApiError` with `&'static str` messages
  (`invalid_token`, `token_expired`, `auth_verifier_unavailable`, …). No JWT
  body, signature, header, JWKS URL, JWK, network detail, file path, or Rust
  debug string is returned.
- `MissingRequiredClaim` detail echoes only the **claim name**
  (`{"claim":"sub"}`), never its value — insufficient to enumerate key state.
- 401/403/503 mapping is stable across unknown-kid, JWKS-network-fail,
  cache-expired, and scope-insufficient paths.
- Internal diagnostics (DB/storage errors) go to `tracing::error!`, not the
  response body (`error.rs`).

**PASS.**

---

## 12. Structured audit logging

`log_audit` (`auth_context.rs:46-60`) emits exactly: `request_id, jti, sub,
principal_type, token_use, act_sub, azp, audience, scope, endpoint, result`.
None of: full JWT, signature, Authorization header, secret, full payload. Failed
authentications return before `log_audit` is reached (the extractor calls
`log_audit` only on the success path, `principal.rs:89-92`).

**Medium (pre-existing, contract-acknowledged):** persistent delegation audit is
**not** implemented in this PR — the contract §9/§10 explicitly defer it to
before Write Shadow, and structured logs are the Stage-1 mechanism. Acceptable
for this stage.

---

## 13. Domain regression

The PR touches **no** `domain/`, `application/`, `store/`, or migration file.
The only domain-adjacent change is that `principal_id` now originates from the
verified `sub` (previously from the HS256-verified `sub`) — same field, same
type, same downstream consumers. requestHash computation
(`compute_request_hash(... &command.principal_id ...)`) is unchanged.

Golden / regression suites ran green in all 3 rounds:
- `request_hash_contract` (create/transition/context/legacy) — all golden tests pass.
- `transition_idempotency` (replay, same-key-same-hash, same-key-diff-payload conflict) — pass.
- `transition_authorization` (assignee/creator/disabled/domain-owner checks) — pass.
- `transition_concurrency`, `transition_success` (stateVersion, event sequence) — pass.

**No domain semantic change. PASS.**

---

## 14. Dependencies

- Runtime: `reqwest = { version = "0.12", default-features = false, features =
  ["json", "rustls-tls"] }` — `default-features=false` disables the default
  native-tls backend; only **rustls** is pulled in.
- `cargo tree -e features` confirms a single TLS backend (rustls) for the
  runtime reqwest; **no native-tls/openssl duplication**.
- Dev-only `reqwest 0.13` (test helper, `json` feature) also resolves to rustls;
  not linked into the release binary.
- No high-risk features, no unknown dependencies. `Cargo.lock` changes are the
  expected rustls/hyper-rustls additions. **PASS.**

---

## 15. Tests

Independent execution (real Postgres 16 on `localhost:5432`, DB `svc_workflow`):

| Check | Result |
|-------|--------|
| `cargo fmt --check` | ✅ clean |
| `cargo build` | ✅ exit 0 |
| `cargo clippy --all-targets --all-features -- -D warnings` | ✅ exit 0, no warnings |
| `cargo test -- --test-threads=1` (round 1, serial) | ✅ **487 passed, 0 failed** |
| `cargo test` (round 2, parallel) | ✅ **487 passed, 0 failed** |
| `cargo test` (round 3, parallel) | ✅ **487 passed, 0 failed** |
| `cargo test -- --list` count | **487** (matches implementation claim exactly) |
| `git diff --check` | ✅ no whitespace errors |

Per-binary breakdown (all rounds agree): lib unit 80, main 0, migration 2,
domain_owner 2, context_revision 4, node_visit 3, submission 3, event 4,
command 9, instance 8, deferred_fk 2, def_version 5, size_limit 8,
graph_immutability 16, def_service 9, def_lifecycle 6, parent_move 5,
def_service_audit 32, **17_workflow_runtime 289** (includes the 18 new
`http_jwks_auth` tests), doctests 0. **Total 487.**

---

## 16. Dynamic security experiments (independent harness)

A standalone, temporary experiment binary (`tests/zz_jwks_audit_experiments.rs`,
**deleted after the run**, never committed) drove the verifier with crafted
inputs. 9 experiments, all **PASS**:

| # | Experiment | Result |
|---|-----------|--------|
| 1 | RS384 / RS512 tokens rejected | ✅ `invalid_token` |
| 2 | hand-crafted `alg=none` token rejected | ✅ `invalid_token` |
| 3 | RSA-pubkey-as-HMAC-secret confusion rejected | ✅ `invalid_token` |
| 4 | `access` token w/ spurious `act` does not elevate; actor stays `sub` | ✅ |
| 5 | OBO actor = `sub`, `act.sub` captured for audit only | ✅ |
| 6 | **deleted key invalid immediately after successful refresh** | ✅ |
| 7 | `principal_type` not overridden by `act.sub` | ✅ |
| 8 | JWK private params (`d`/`p`/`q`) ignored | ✅ |
| 9 | duplicate kid does not bypass signature check | ✅ |

Concurrency: the single-flight `refresh_lock` was code-verified; the existing
`cache_hit_multiple_verifications` test plus the rotation experiment exercise the
refresh path. (A synthetic many-thread unknown-kid storm was not run because the
mock server serializes accepts; the mutex design is sound and the rotation
experiment confirms replacement semantics.)

---

## 17. Migration guards

```bash
git diff --name-status <base>..<head> -- migrations/
```
→ **empty**. No schema change, no new SQL, no migration version bump. ✅

---

## 18. Structure guards

| Guard | Limit | Actual | Status |
|-------|-------|--------|--------|
| Handwritten Rust file length | ≤ 500 | max 433 (`jwks_verifier.rs`) | ✅ |
| Directory direct children | ≤ 20 | `src/auth/` = 7 | ✅ |
| Directory depth (new src files) | ≤ 4 | depth 2 (`src/auth/*.rs`) | ✅ |
| `tests/` top-level entries | < 21 (no 21st) | 17 (new module nested under `17_workflow_runtime/http/`) | ✅ |
| Whitespace (`git diff --check`) | clean | clean | ✅ |

Residue checks: no test triggers/functions, no temp DB, no orphaned mock-JWKS
process, no occupied ports after the run. ✅

---

## 19. Blocker

**None.**

---

## 20. High

**None.**

Algorithm pinning, kid trust, actor sourcing, max-stale, key-deletion-on-refresh,
and fail-closed semantics all hold and were dynamically proven.

---

## 21. Medium

1. **No explicit JWKS redirect policy.** The reqwest client builder
   (`jwks_verifier.rs:126-129`) sets only `.timeout(...)`. reqwest defaults to
   following up to 10 redirects. The JWKS URL is operator-configured (not
   attacker-controlled), so this is not directly exploitable, but a
   compromised/rogue JWKS endpoint could redirect the verifier to an internal
   address. *Fix:* `.redirect(reqwest::redirect::Policy::none())` (a JWKS
   endpoint should not redirect) — 1 line.

2. **No explicit URL scheme allow-list.** `WORKFLOW_JWKS_URL` is not validated
   to be `http://`/`https://` before use. reqwest rejects non-HTTP schemes
   internally at request time, so this is defense-in-depth, not a live hole.
   *Fix:* parse the URL and reject non-http(s) schemes at `JwksConfig::from_env`.

3. **Production discrimination for `test_hs256` is loopback-only.** There is no
   `WORKFLOW_ENV`/production flag; the only barrier to running HS256 in a
   "production-like" setting is the loopback-bind gate. A server bound to
   `127.0.0.1` behind a reverse proxy could still run HS256 with a shared secret.
   This matches the V0 contract's stated mechanism but is weaker than an explicit
   environment guard. Track for hardening before cutover.

4. **Persistent delegation audit not implemented** (contract-deferred to
   pre-Write-Shadow). Structured logs are the Stage-1 mechanism. Acknowledged,
   not a regression.

None of these block merge.

---

## 22. Low

1. **Duplicate-kid handling diverges from contract wording.** Contract §5 says
   duplicate/malformed keys are "silently skipped with a warning log"; the
   implementation keeps the first matching key and warns nothing on duplicates.
   Security impact is negligible (an attacker controlling the JWKS endpoint
   already controls all keys), but the behavior should match the doc. *Fix:*
   dedupe on `kid` (keep first) and emit a `tracing::warn!` on collision.

2. **`token_use` defaults to `access` when absent** (`claims.rs:73-79`). This is
   intentional backward-compat, but a future strict mode could require the claim
   explicitly. Documented; no action needed for V0.

3. **`regression_existing_tests_unchanged` test is a no-op meta-assertion**
   (`jwks_auth.rs:736-740`). Harmless but adds no coverage; could be removed.

---

## 23. Verdict

The JWKS/OBO Verifier + Dual Auth Mode V0 implements every Stage-1 contract
clause correctly. The critical security invariants — algorithm pinning to RS256,
`kid` trust with controlled refresh, `JWT.sub` as the sole domain actor,
`act.sub` audit-only, fail-closed cache semantics, and **immediate eviction of
deleted keys on successful refresh** — all hold and were verified both by
code trace and by independent dynamic experiments. No Blocker or High severity
findings. Three full test rounds (serial + 2× parallel) pass with exactly 487
tests and zero failures; `fmt`, `clippy -D warnings`, and `diff --check` are
clean. Domain/application/migration layers are untouched; golden requestHash and
idempotency suites remain green.

The four Medium findings are hardening items (redirect policy, scheme
allow-list, production guard, deferred persistent audit) consistent with a
Stage-1 frozen contract and do not weaken the authentication security envelope.

**Recommendation: APPROVE for merge into `svc-workflow/main`.**

---

## 24. Minimum fix suggestions (optional, post-merge or follow-up)

1. `jwks_verifier.rs` `JwksVerifier::new`: add
   `.redirect(reqwest::redirect::Policy::none())` to the client builder.
2. `auth_mode.rs` `JwksConfig::from_env`: after reading `jwks_url`, parse it and
   reject any scheme other than `http`/`https` with a clear startup error.
3. `jwks_verifier.rs` `fetch_jwks`: on duplicate `kid`, keep the first entry and
   log `tracing::warn!(kid = %kid, "duplicate kid in JWKS, keeping first")`.
4. (Process) Consider an explicit `WORKFLOW_ENV` gate so `test_hs256` cannot be
   selected outside local/staging, as defense-in-depth before cutover.

None of these are required to merge.

---

## 25. Can this merge into main?

**Yes.**

```text
SVC_WORKFLOW_JWKS_VERIFIER_AUDIT_PASS
```
