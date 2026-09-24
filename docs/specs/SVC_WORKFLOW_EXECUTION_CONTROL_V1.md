---
spec_id: SVC_WORKFLOW_EXECUTION_CONTROL_V1
title: Workflow Execution Control — canonical forum binding, durable outbox with reconciler, RETURN policy projection with REQUIRE_HUMAN escalation, system execution-escalation ingress, push-first kick, due-set HUMAN_REQUIRED narrowing
status: accepted
accepted_date: 2026-09-24
accepted_by: mayf3
accepted_reviewed_head: 450734f7736efc2d2342499e8bdb63615680ce4e
acceptance_authority_basis: >-
  Owner ACCEPT via GOAL = WORKFLOW_EXECUTION_CONTROL_V1_CLOSURE_AND_DEPLOYMENT_
  READINESS (2026-09-24): "当前整体设计与实现方向接受，可以进入最终 closure / merge /
  deployment-ready 阶段", bound to the implementation head
  450734f7736efc2d2342499e8bdb63615680ce4e (feature branch
  goal/workflow-execution-control-v1 based on github/main d661af5). This commit
  is the acceptance lifecycle transaction ONLY: the contract body is
  byte-identical to the reviewed head except this frontmatter. Preceding
  mandate record (proposal): GOAL = WORKFLOW_EXECUTION_CONTROL_V1 (2026-09-24),
  Scopes A/C/D/F/G svc-workflow side; §11 authority invariants; §12 Cases 1-10.
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
scope:
  - svc-workflow
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V8
external_authorities:
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_WORKFLOW_EXECUTION_CONTROL_V1
    revision: 50d8a69bc1996eb28029272f2fa7f8edb4f3baca
    relation: interoperates_with
  - repository: mayf3/agent-forum
    authority_id: AGENT_FORUM_WORKFLOW_INSTANCE_CONTEXT_V1
    revision: 9734f6c5e6d378087008f4573a1efc01b00568e3
    relation: interoperates_with
supersedes: []
superseded_by: null
owners:
  - mayf3
architecture_authority: SVC_WORKFLOW_ARCHITECTURE_V0_4_3
local_authority_relations:
  - authority_id: SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
    relation: amended_by
    note: due-set narrowing in Amendment A section 6
  - authority_id: SVC_WORKFLOW_DISPATCH_INTENT_KEYSET_CONTINUATION_V1
    relation: preserved_untouched
    note: cursor/order semantics byte-identical
production_apply_authority: none
repo: mayf3/svc-workflow
date: 2026-09-24
base_head: d661af5caa135d783e33718bfa2da9e8aafdd9d5 (github/main)
revision: r1
companion_specs:
  - repository: mayf3/dsh-agent-core
    spec_id: AGENT_CORE_WORKFLOW_EXECUTION_CONTROL_V1 (proposed, same date)
  - repository: mayf3/agent-forum
    spec_id: AGENT_FORUM_WORKFLOW_INSTANCE_CONTEXT_V1 (proposed, same date)
---

# SVC_WORKFLOW_EXECUTION_CONTROL_V1

## 0. Intent

svc-workflow stays the ONLY business authority. This Spec adds: (A) a
canonical WorkflowInstance → ForumThread binding owned by svc-workflow and
materialized through a durable outbox + reconciler (forum is never a
business authority and never blocks a business transaction); (D) a
deterministic RETURN-policy projection computed from `workflow_events`
(system-counted, never agent-counted) with REQUIRE_HUMAN escalation through
the EXISTING assistance machinery; (E/F) a system execution-escalation
ingress for the execution runtime and a push-first kick outbox — the
scheduler/due-feed poll stays the correctness path; and a due-set narrowing
that stops dispatching into a visit that is HUMAN_REQUIRED. No separate
execution ledger is stored here; no attempt facts are mirrored.

## 1. Data model (migration `0027_execution_control_v1.sql`)

```
workflow_forum_bindings (
  workflow_instance_id UUID PK REFERENCES workflow_instances,
  forum_thread_id      TEXT,            -- NULL until reconciler resolves it
  binding_state        TEXT NOT NULL CHECK (binding_state IN ('PENDING','BOUND')),
  created_at / updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (forum_thread_id)
)

workflow_outbox (
  outbox_id      UUID PK,
  workflow_instance_id UUID NOT NULL REFERENCES workflow_instances,
  outbox_kind    TEXT NOT NULL CHECK (outbox_kind IN ('FORUM_EVENT','EXECUTION_KICK')),
  event_key      TEXT NOT NULL,         -- dedupe identity (see §3)
  payload        JSONB NOT NULL CHECK (jsonb_typeof(payload) = 'object'),
  attempt_count  INT NOT NULL DEFAULT 0,
  next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  last_error     TEXT,
  delivered_at   TIMESTAMPTZ,
  created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (outbox_kind, event_key)
)
CREATE INDEX ... ON workflow_outbox (next_attempt_at) WHERE delivered_at IS NULL
```

No historical backfill: existing instances get bindings opportunistically
through the same reconciler ONLY if a FORUM_EVENT row exists for them (V1
keeps zero write-backfill; instances predating this Spec keep no binding).

## 2. CTR-SWEC-001 — binding fact at creation (BUSINESS class only)

Inside the EXISTING create transaction (`create_transaction.rs`), for
`execution_class = 'BUSINESS'` instances, the tx additionally inserts:

- `workflow_forum_bindings (PENDING)` row;
- one `FORUM_EVENT` outbox row `event_key = wf_created:<instanceId>`
  (payload: eventType `workflow_created`, title, definitionKey, createdAt);
- when the first activation is a `DISPATCH_INTENT`, one `EXECUTION_KICK`
  outbox row `event_key = kick:<activationId>` (payload: the 3 ids).

The transaction's success never depends on forum/agent-core availability —
rows are durable facts written in the same tx, drained later.

## 3. CTR-SWEC-002 — outbox reconciler (tokio background task)

- Every `WORKFLOW_OUTBOX_INTERVAL_MS` (default 5000), take up to 20 rows
  with `delivered_at IS NULL AND next_attempt_at <= now()` ordered by
  `created_at, outbox_id`.
- `FORUM_EVENT` rows: ensure the canonical thread (see §4), then post the
  event as `kind=comment` with metadata
  `{workflowInstanceId, eventKey, eventType}` to the bound thread; mark
  `delivered_at`. Order per instance is (created_at, outbox_id) — oldest
  first. On failure: `attempt_count += 1`,
  `next_attempt_at = now() + min(2^attempt_count, 600s)`, `last_error`
  recorded (sanitized). Rows are NEVER deleted or dead-lettered (no silent
  loss); delivery is at-least-once with the forum-side dedupe below.
- `EXECUTION_KICK` rows: POST to the agent-core kick endpoint (§5); mark
  `delivered_at` on 2xx/4xx (4xx = permanent, the poll loop owns
  correctness; only 5xx/network retry). Same backoff.
- The reconciler NEVER writes business tables. It only drains its own
  outbox and updates `workflow_forum_bindings`.

## 4. CTR-SWEC-003 — canonical binding protocol (idempotent)

Config env (all required to enable; otherwise the reconciler sleeps and
logs honestly): `WORKFLOW_FORUM_SYNC_ENABLED`, `WORKFLOW_FORUM_AUTH_BASE_URL`,
`WORKFLOW_FORUM_CLIENT_ID`, `WORKFLOW_FORUM_CLIENT_SECRET`,
`WORKFLOW_FORUM_ORIGIN` (default `http://127.0.0.1:3460`), audience `svc-forum`.

The forum principal is an agent-type machine principal with `forum.read` +
`forum.write` (client_credentials from auth-service; token cached until
expiry). It must NOT be listed in svc-forum's operator exclusion.

Find-or-create, per PENDING binding:

1. `GET /api/threads?contextType=workflow_instance&contextId=<id>`
2. empty ⇒ `POST /api/threads {title, type: 'discussion', contextType,
   contextId}`; a 409 (lost race — the companion forum Spec makes this a
   DB-guaranteed unique) ⇒ re-run step 1.
3. Store `forum_thread_id`, set `binding_state = 'BOUND'`.

Idempotent by construction; two reconcilers (or a retry after a timeout
that actually succeeded) converge on ONE thread (forum-side unique index).

## 5. CTR-SWEC-004 — execution-escalation ingress

`POST /internal/v1/workflow-instances/{workflowInstanceId}/execution-escalations`

Gate mirrors wake exactly: `workflow.execute` scope + direct token +
server-side `GLOBAL_SCHEDULER_READ` binding; denied attempts get the same
durable security audit.

Body: `{ nodeVisitId, reason, attemptCount, lastAttemptId?, dispatchIntentId? }`
with `reason ∈ ('ATTEMPTS_EXHAUSTED','STALE_LOOP_EXHAUSTED')`.

Transaction (one tx, mirroring assistance write discipline):

1. Acquire command receipt (`SYSTEM_EXECUTION_ESCALATION` command type).
2. Lock instance; visit must be the CURRENT visit; instance not
   cancelled/archived/terminal.
3. An open case on the visit ⇒ 200 replay `{escalated: false,
   assistanceCaseId: <existing>}` (idempotent).
4. Else insert the assistance case as OWNER_PENDING
   (`requested_by = caller`, request payload = `{message, supportingPayload:
   {reason, attemptCount, lastAttemptId, dispatchIntentId, source:
   'execution_policy'}}`), then in the SAME tx escalate it to
   HUMAN_REQUIRED (escalation fields set by the same command id — satisfies
   the 0021 CHECK constraints), event `ASSISTANCE_ESCALATED_TO_HUMAN` via
   the existing `increment_instance_and_event` (version bump ⇒ the
   engine's stale probe mechanically sees `progressed`).
5. Queue a `FORUM_EVENT` outbox row `event_key =
   assistance_escalated:<caseId>` (Goal Scope G: HUMAN_REQUIRED visible on
   the canonical thread).

Response: `{escalated, assistanceCaseId, workflowStateVersion, eventSequence}`.

## 6. CTR-SWEC-005 — RETURN policy projection (Goal Scope D)

`maxReturnsPerEdge`: env `WORKFLOW_POLICY_MAX_RETURNS_PER_EDGE`, default 3,
min 1. Canonical key: `(workflow_instance_id, transition_definition_id)`.
Counter source: `workflow_events` rows with `transition_effect = 'RETURN'`
and the event's `transition_definition_id` (deterministic rebuild from
authoritative events; no counter table, nothing agent-maintained).

In `execute_workflow_transition_atomically`, for `effect == "RETURN"`, after
existing effect validation:

- `returnCount = count(prior RETURN events for this instance + transition)`
- `returnCount >= max` ⇒ deterministic failure 409
  `return_policy_exhausted` (no side effects; the escalation for the
  exhausted edge happened when the max-th RETURN committed — see below).
- `returnCount + 1 == max` ⇒ the committing tx ALSO escalates: create +
  escalate an assistance case on the transition's TARGET visit (the visit
  the instance is about to sit on), in the same tx, using the same
  mechanics as §5 step 4 (caller = transitioning principal; provenance
  `source: 'return_policy'`), plus the FORUM_EVENT outbox row
  `return_policy_reached:<instanceId>:<transitionId>`.

Default disposition is REQUIRE_HUMAN; nothing force-advances, ever.

## 7. CTR-SWEC-006 — AMENDMENT A to CTR-VAI-009 (due-set narrowing)

The due-set predicate gains ONE conjunct:

```
AND NOT EXISTS (
  SELECT 1 FROM workflow_assistance_cases ac
  WHERE ac.node_visit_id = a.node_visit_id
    AND ac.status IN ('OWNER_PENDING','HUMAN_REQUIRED'))
```

A visit with an open assistance case is not offered to any scheduler —
system-enforced stop of meaningless loops (the open case also fail-closes
transitions per existing semantics). Keyset continuation, ordering, record
shape (7 fields) and role gate are byte-identical; this Spec explicitly
amends the CTR-VAI-009 due-set selection accordingly.

## 8. CTR-SWEC-007 — push-first kick outbox (Goal Scope F)

Every transition/create that inserts a `DISPATCH_INTENT` activation also
inserts the `EXECUTION_KICK` outbox row (§2/§9). Fast path = reconciler
drains it to agent-core within seconds; fallback = the existing 30s due
poll (unchanged). Both paths meet the same one-attempt fence in the
execution runtime, so push + poll can never double-execute. `HUMAN_WORK_ITEM`
activations mint no kick (humans are not kicked by the execution runtime).

## 9. CTR-SWEC-008 — transition-forum events (Goal Scope G)

The transition transaction additionally queues FORUM_EVENT outbox rows:
`transition_committed:<eventId>` (ADVANCE / RETURN #n / TERMINATE, with
node ids and effect) and `workflow_completed:<instanceId>` on TERMINAL
arrival. Cancel/archive transactions queue `workflow_cancelled` /
`workflow_archived` rows. Assistance request/escalate/resolve and the §5/§6
escalations queue their rows. All message text is a projection of
committed business facts only.

## 10. Invariants (Goal §11 restated for this slice)

- svc-workflow stays the sole business authority; the forum binding is a
  projection pointer, never a business fact.
- Forum write failures can NEVER roll back a committed business transition
  (outbox discipline).
- The agent cannot write attempt counts, return counts, escalations, or
  session facts; every counter is computed from authoritative events.
- No second execution ledger: attempt/session facts stay in dsh-agent-core.

## 11. Test obligations (Goal §12 mapping)

- Case 1: create/transition queues kick row; reconciler delivers to a mock
  core; due intent unaffected.
- Case 2: kick endpoint unreachable ⇒ row retries; poll loop still admits
  exactly one attempt (covered agent-side).
- Case 5: RETURN commits; per-edge count query returns 1; forum row queued.
- Case 6: max-th RETURN auto-escalates (HUMAN_REQUIRED on target visit);
  further RETURN ⇒ 409 `return_policy_exhausted`.
- Case 4: escalation endpoint creates HUMAN_REQUIRED assistance case,
  bumps state version, is idempotent on a second call; due feed excludes
  the escalated visit (§7).
- Case 8: forum sync disabled ⇒ create/transition still 2xx; enabling
  later drains the backlog in order.
- Case 10: two ensure-thread passes ⇒ exactly one thread (mock forum 409
  race); binding UNIQUE holds.
