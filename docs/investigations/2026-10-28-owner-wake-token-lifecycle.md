# Owner Assistance wake token lifecycle — source investigation

Status: non-authoritative investigation. It does not create an implementation
Contract or deployment permission.

## Coordinates

- Repository: `mayf3/svc-workflow`
- Fresh fetched `origin/main`: `784ad4bbea6e4f6bdde1a10ce4efc80f815c145f`
- Candidate worktree: `/Users/yanfenma/workspace/worktrees/codex-wec-kick-token-renewal-20261028`
- Pinned implementation comparison only: `/Users/yanfenma/workspace/worktrees/codex-workflow-svc-combined-20261027` at `f4a1ed85e96936155e551eea6ac8b209ef360cda`; not modified.
- Cross-repo accepted decision: `mayf3/dsh-agent-core` `AGENT_CORE_WORKFLOW_EXECUTION_CONTROL_V1`, accepted revision `50d8a69bc1996eb28029272f2fa7f8edb4f3baca`; `CTR-WEC1-005` requires durable Domain Owner wake and same request id on retry.

## Observations

### OBS-001 — accepted svc authority omits Owner Assistance sender auth lifecycle

At the fresh svc base, `docs/specs/SVC_WORKFLOW_EXECUTION_CONTROL_V1.md` is accepted revision r1 and defines CTR-SWEC-001..008. It specifies forum reconciliation and static `WORKFLOW_EXECUTION_KICK_TOKEN` for execution kicks; it does not specify `OWNER_ASSISTANCE_WAKE` token acquisition or renewal.

### OBS-002 — source has durable owner wake but reuses one static token

At base `784ad4b`, `src/store/postgres/outbox.rs:76–115` queues BUSINESS-only `OWNER_ASSISTANCE_WAKE` keyed by assistance case. `src/forum_sync.rs:43–92` reads `WORKFLOW_EXECUTION_KICK_URL/TOKEN` once into `ExecutionKickConfig`; `src/forum_sync.rs:546–580` reuses that token on each owner-wake POST and leaves every non-2xx row pending. `src/forum_sync.rs:136–193` separately obtains Forum client-credentials tokens, but requests Forum audience plus `forum.read forum.write`; it cannot authorize the product-api owner-wake endpoint.

`src/forum_sync.rs:473–481` marks only success delivered and calls `mark_attempt_failed` otherwise. `src/store/postgres/outbox.rs:187–203` persists failed attempts and caps the exponential delay at 600 seconds without deleting/dead-lettering the row.

### OBS-003 — consumer verifier rejects expired and overlong machine tokens

At Agent Core source revision `b38a4ecb8d245becbec12044f7a60b8ef8f5aceb`, `packages/product-api/src/scheduler-auth.js:27,217–228` caps direct-machine JWT lifetime at 600 seconds and rejects expiry. `packages/product-api/test/scheduler-auth.test.js:94–105` verifies rejection of a 601-second lifetime. The same verifier is mounted in the production runtime through `packages/production-runtime/src/scheduler/history-runtime.js:1–2,21–31`.

### OBS-004 — actual route short-expiry reproduction

Task artifact `/Users/yanfenma/workspace/artifacts/workflow-unattended-mechanisms-20261027/jwt-static-expiry-repro-luna/repro.mjs`, SHA-256 `ffe782a65988534b09ee5428dbfa48a0496a91b037fa060599a76132fe31c0d0`. Command: `node --test /Users/yanfenma/workspace/artifacts/workflow-unattended-mechanisms-20261027/jwt-static-expiry-repro-luna/repro.mjs`; executed result 1/1 PASS. It sends a synthetic RS256 token through actual `handleWorkflowExecutionRequest` and actual verifier over a random loopback route/JWKS pair: first request is 200; after advancing injected time beyond exp plus verifier tolerance, two identical-static-bearer retries are 401 and only one wake is admitted.

This test proves product-api route expiry behavior and static replay failure. It does **not** run the svc Rust reconciler, PostgreSQL outbox, or real Auth service, and is not evidence of an end-to-end svc outbox failure or production state.

## Claims and limits

- `CLM-001` — a fixed bearer can stop making progress on a durable Owner Assistance outbox after expiry while the product-api verifier continues to reject it. Support: `OBS-002`, `OBS-003`, `OBS-004`. This is a source-supported mechanism risk; no live production assertion is made.
- `CLM-002` — existing `client_credentials` is a viable token acquisition primitive if an active client has the exact configured audience and `workflow.execute` grant. Auth V1 `src/lib/oauth/v1/direct.ts:92–134` authorizes exact active client/principal/resource/scope grants; `docs/contracts/minimal-auth-v1/claims-and-profiles.md:137–148` says direct machine tokens have no refresh token and max 10-minute TTL. Agent Core Broker `packages/broker/src/transport/token.js:66–135` and `packages/broker/src/transport/http.js:45–58` already implement that acquisition and a client/audience/scope keyed expiry cache. Whether a production poller identity currently has the exact grant is `UNKNOWN`.

## Scope / next step

The proposed additive `CTR-SWEC-009` defines a dedicated owner-wake credential path and leaves CTR-SWEC-007 execution-kick authentication unchanged. This is authoring only. Credential custody, exact production audience/grant, deployment configuration, and restart remain separate gates. After independent Contract acceptance, implementation must run against fresh isolated Auth/service fixtures and the actual svc outbox path; no production credentials or writes are required for source/isolated conformance.

## Proposed same-Spec AMEND delta

The exact additive diff is stored in sibling `2026-10-28-owner-wake-token-lifecycle.patch`.
It appends `ACC-SWEC-009-001..005` and `CTR-SWEC-009` only; it leaves existing
CTR-SWEC-001..008, frontmatter, accepted `SVC_WORKFLOW_EXECUTION_CONTROL_V1`
status, parent authorities, and external-authority revisions untouched. This
patch is a review artifact only and MUST NOT be applied until the Owner accepts
the exact amendment; it is not itself Product Authority.

The proposed Contract requires dedicated `WORKFLOW_OWNER_ASSISTANCE_WAKE_*`
configuration (exact endpoint URL, Auth base URL, client ID/secret, and exact
product-api audience), Auth `client_credentials` with only `workflow.execute`,
an in-memory `(client_id, audience, scope)` cache with renewal margin exactly
`min(60 seconds, 10% of expires_in)`. Expiry is measured from the monotonic
instant immediately before each Auth request begins; a delayed response that
arrives at or beyond the renewal point is neither cached nor used, and the row
remains pending under existing backoff. Acceptance adds a controlled delayed
Auth/short-TTL negative case. The Contract also specifies 5-second Auth/product-api
request bounds, 401 cache invalidation without same-pass replay, and existing
durable outbox retry/backoff with no drop/dead-letter.
The existing `WORKFLOW_EXECUTION_KICK_TOKEN` cannot be used as owner-wake
fallback; CTR-SWEC-007 execution-kick authentication remains unchanged.

The repository transition validator was run against the fresh base and a
same-ID candidate with `status: proposed`; it rejected the transition as
`accepted -> proposed`. For that reason the accepted Spec stays byte-identical
in this authoring branch. The proposal is carried as an unapplied patch rather
than falsely retaining `status: accepted` for unaccepted new text or weakening
the old accepted-head binding. Owner acceptance can apply the exact patch and
record the accepted final revision in the usual docs-only transaction.

## Patch integrity correction

The amendment patch is generated as a complete unified diff against the exact
`origin/main` accepted Spec body, rather than editing hunk counts by hand. The
hunk is `@@ -277,3 +277,120 @@`. A temporary checkout-shaped copy was patched
with `git apply`; the complete resulting Spec body matched the generated
candidate byte-for-byte, including the final production-authority boundary.
Applied candidate body SHA-256: `6774a1481563f5059bf774e8dae05365e80bb1f4327b55ce4fec83096a3b01c0`.
The contradictory one-request use rule for tokens without a safe window was
removed: those responses fail closed without caching or POST, leaving the row
pending under existing backoff.
