# WORKFLOW_EXECUTION_CONTROL_V1 — Deployment Package

Owner closure: GOAL = WORKFLOW_EXECUTION_CONTROL_V1_CLOSURE_AND_DEPLOYMENT_READINESS (2026-09-24).
CLOSURE = PASS (see final closure report; SHIP_BLOCKERS resolved in this package's step 0).

System Product API binding amendment accepted by mayf3 on 2026-09-24:
`/Users/yanfenma/workspace/artifacts/DEPLOYMENT_BACKLOG/wec-live-base-candidate-20260924/RECOMMENDED-SERVICE-BINDING-DELTA.md`
(SHA-256 `2cabc4515fa45aec048ea43b9b6221905fcd9c4ec397c5dda73de19afab7d9a5`).
The canonical system/authsvc runtime serves this release's Product API on
`127.0.0.1:8788`; the existing GUI/502 runtime remains on `127.0.0.1:8787`.

Feature branches (all `goal/workflow-execution-control-v1`):

| repo | base (canonical main) | FINAL_SHA | acceptance commit |
|---|---|---|---|
| mayf3/agent-forum | 2af4a716da482b572e653938f60a90540be30274 | 9734f6c5e6d378087008f4573a1efc01b00568e3 | b1cdb05 |
| mayf3/dsh-agent-core | 2a85d0659157a3bab649239158744d5222d86afe | 50d8a69b (impl e28c7666) | 50d8a69b |
| mayf3/svc-workflow | d661af5caa135d783e33718bfa2da9e8aafdd9d5 (github/main) | 39d5020 (impl 450734f) | 39d5020 |

Governing specs (all `status: accepted` 2026-09-24, accepted_by mayf3):
- `AGENT_FORUM_WORKFLOW_INSTANCE_CONTEXT_V1` (CTR-FWIC-001..004)
- `AGENT_CORE_WORKFLOW_EXECUTION_CONTROL_V1` (CTR-WEC1-001..007)
- `SVC_WORKFLOW_EXECUTION_CONTROL_V1` (CTR-SWEC-001..008)

## Deployment order (strict)

```text
0. forum data/migration preflight  (inside the forum migration itself — see step 1)
1. agent-forum
2. dsh-agent-core
3. svc-workflow
```

Rationale: Forum first provides the binding constraint (unique canonical thread);
DSH second provides the trace/kick consumer + execution policy; svc last starts
producing new outbox / kick / escalation facts.

## Step 0 — forum migration safety (already verified)

Migration `svc-forum/prisma/migrations/20260924000000_workflow_instance_context/migration.sql`
is SELF-CONTAINED and was verified end-to-end against a `pg_dump` clone of the
live forum database:

- guarded data remediation: the free-form context field was hand-used pre-WEC;
  duplicate `workflow_instance` contextIds exist in real data. OLDEST thread per
  contextId stays canonical; newer duplicates keep all content, lose only the
  canonical binding (UPDATE, nothing deleted).
- index `uq_forum_threads_workflow_instance_context` created on the QUOTED
  camelCase columns (`"contextType"` / `"contextId"` — Prisma naming).
- post-conditions verified on the clone: one canonical thread per contextId;
  canonical query returns the oldest thread; duplicate INSERT rejected
  (route maps the P2002 to 409 WORKFLOW_CONTEXT_THREAD_EXISTS).

Rollback of this step: `DROP INDEX uq_forum_threads_workflow_instance_context;`
(demoted threads keep NULL context — restorable only from backups; acceptable:
forum is a projection, never a business authority).

## Step 1 — agent-forum

```bash
cd svc-forum
npm ci && npx prisma migrate deploy && npm run typecheck && npm test
# restart svc-forum (PORT 3460 unchanged)
curl -s http://127.0.0.1:3460/api/health   # expect 200
```

## Step 2 — dsh-agent-core

The accepted one-time system service binding enables the canonical authsvc
Product API on `127.0.0.1:8788`. Without
`WORKFLOW_EXECUTION_POLLER_AGENT_ID` the engine stays evidence-only; the new
routes 401 fail-closed without the token verifier. Deploy the new bytes
(Phase 1). Before authenticated route acceptance, bind the existing trusted
`SCHEDULER_AUTH_JWKS_URL` verifier; the SAME verifier also gates:

- `GET  /workflow-execution/traces`   (bearer + workflow.execute scope)
- `POST /workflow-execution/kicks`    (bearer + workflow.execute scope)

Phase 3 enablement (kick consumer + execution policy) requires the pre-existing
`WORKFLOW_EXECUTION_POLLER_AGENT_ID` principal to ALSO hold auth-service
`forum.read` + `forum.write` scopes (agent-type principal, NOT in
`FORUM_OPERATOR_AGENT_IDS`) for the forum execution-event projection.

## Step 3 — svc-workflow

```bash
svc-workflow --migrate     # applies 0027_execution_control_v1.sql (two NEW tables only)
# then roll the new binary (WORKFLOW_PORT 8989 unchanged)
```

Phase 2 enablement (forum sync) env:

```text
WORKFLOW_FORUM_SYNC_ENABLED=1
WORKFLOW_FORUM_AUTH_BASE_URL=http://127.0.0.1:4001
WORKFLOW_FORUM_CLIENT_ID=<machine principal client id>
WORKFLOW_FORUM_CLIENT_SECRET=<secret>
WORKFLOW_FORUM_ORIGIN=http://127.0.0.1:3460
WORKFLOW_FORUM_AUDIENCE=svc-forum
```

Phase 3 enablement (push kick) env:

```text
WORKFLOW_EXECUTION_KICK_URL=http://127.0.0.1:8788/workflow-execution/kicks
WORKFLOW_EXECUTION_KICK_TOKEN=<token for the poller principal, audience per
                             SCHEDULER_AUTH_* verifier config, scope workflow.execute>
```

Unset ⇒ reconciler sleeps, outbox rows queue safely (never lost), catch-up is
automatic in `(created_at, outbox_id)` order after enablement.

## Production prerequisites (verify before Phase 2/3 — read-only checks)

```sql
-- forum bot principal exists & enabled (auth-service DB / API):
--   principal_type='agent', enabled, client has scopes forum.read forum.write
--   and is NOT listed in svc-forum FORUM_OPERATOR_AGENT_IDS
-- execution poller GLOBAL_SCHEDULER_READ still bound (svc DB):
SELECT * FROM global_role_bindings
 WHERE role_key='GLOBAL_SCHEDULER_READ' AND enabled;
```

Health gates per phase:

- Phase 1: svc `/healthz` 200, `version.schemaVersion = 0023`, migrations = 27;
  canonical system/authsvc product-api `http://127.0.0.1:8788/health` 200
  with the listener PID/UID/source generation bound to the deployed system
  runtime; existing GUI/502 `127.0.0.1:8787` unchanged; forum
  `/api/health` 200.
- Phase 2: create one BUSINESS workflow → canonical thread appears exactly once
  (`GET /api/threads?contextType=workflow_instance&contextId=<id>` → 1 item);
  forum events post in order; forum outage during the phase must NOT fail any
  workflow command (outbox retries).
- Phase 3: transition triggers a kick (observed in svc-workflow runtime log
  `outbox reconciler delivered rows`) and exactly ONE attempt appears in
  `GET /workflow-execution/traces` despite push+poll both firing.
- Phase 4 canary (small real workflows): attempt retry after
  run_ended_no_submission; RETURN-limit → HUMAN_REQUIRED assistance case +
  forum incident; session trace chain complete
  (workflowInstanceId → nodeVisitId → attemptId → generation → agent →
  sessionId → reconciliation evidence). Keep HR / scheduler fallback ON.

## Rollback

- svc: Owner accepted the exact
  `WORKFLOW_EXECUTION_CONTROL_V1_DEPLOYMENT_ROLLBACK_AMENDMENT_DRAFT.md`
  (SHA-256 `b76fa3aabc51d572d0bfaaa426e9acc273181c2f1e185636787bd4b240f50398`)
  and its `0027-empty-only-recovery.sql` (SHA-256
  `49cf0921202ec1fc0bf33212985a535dd643ccede7cdec56b01d802f8ce6ec96`)
  on 2026-09-24. Before migration 0027 commits, retain or restore the exact
  preimage binary and 26-file migration bundle without schema recovery. After
  0027 commits, **empty-only schema rollback is permitted only before any WEC
  candidate process has launched**, and only with a durable
  `NO_WEC_WRITE_FENCE` receipt proving continuous production mutex and journal,
  candidate launch count zero, no direct WEC writer, the exact preimage service
  generation or continuously stopped state, exact database and actor, and the
  reviewed SQL digest. That continuous fence must prove both new tables were
  never written. Fresh SQL gates must prove both are currently empty, their
  exact definitions, and the exact successful `1..27` migration ledger,
  including the reviewed version-27 checksum. Current emptiness alone is
  never no-write proof. The sole
  destructive effect is to drop `public.workflow_outbox` and
  `public.workflow_forum_bindings` without `CASCADE` and delete exactly the
  matching successful version-27 ledger row. Then restore the exact preimage
  binary and 26-file bundle; require ledger `1..26`, both WEC tables absent,
  `/version.gitSha = f6a74001b71af106777045b596b30c3a7e99934d`,
  `/readyz = 200`, and a durable receipt before releasing the mutex. If the
  candidate ever launched, a WEC write may have occurred, any prerequisite is
  unknown, or the journal/fence is incomplete, preserve 0027 and its data and
  recover forward. An unknown SQL transaction outcome must never be replayed.
  Normal successful forward deployment never invokes the recovery SQL.
- dsh: roll back the binary; ledger files replay byte-identically on either
  version (additive event fields only).
- forum: `DROP INDEX uq_forum_threads_workflow_instance_context;` + roll back
  bytes. No workflow data is affected at any point (forum is projection-only).

## Follow-up debt (registered, non-blocking)

1. svc contract debt: `POST /internal/v1/workflow-instances/{id}/execution-escalations`
   + error `return_policy_exhausted` not yet in
   `contracts/workflow-http/v1/{openapi.yaml,errors.json}` — precedent allows
   (wake + dispatch-intents shipped accepted outside the bundle; no CI gate).
2. Forum message-level idempotency unique key (at-least-once poster may rarely
   duplicate a system comment; display-only).
3. `ELIGIBLE` execution state from svc activation facts (trace currently serves
   ledger-derived states only).
