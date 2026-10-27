---
spec_id: SVC_WORKFLOW_EXECUTION_CONTROL_V1
title: Workflow Execution Control — canonical forum binding, durable outbox with reconciler, RETURN policy projection with owner-attention assistance, system execution-escalation ingress, push-first kick, open-assistance due-set narrowing
status: accepted
accepted_date: 2026-10-28
accepted_by: Codex root (delegated under user authorization)
accepted_reviewed_head: 2c7e59a14d5d84ecc0688c47c30c5f04d0f05900
acceptance_delta_class: additive_contracts_and_acceptance
semantic_delta_from_reviewed_head: none; body SHA-256 c5b34b713f1c5045ddcf9c23d1d663e572f4c10be4e2a15e001ed7b8e2030194
accepted_proposal_patch_sha256: fd49ca3913ad22695df51cf18b95e94679b1e71fc51c5b677149e29803e33e04
acceptance_authority_basis: >-
  Original acceptance lineage: Owner ACCEPT via GOAL =
  WORKFLOW_EXECUTION_CONTROL_V1_CLOSURE_AND_DEPLOYMENT_READINESS
  (2026-09-24): "当前整体设计与实现方向接受，可以进入最终 closure / merge /
  deployment-ready 阶段", bound to implementation head
  450734f7736efc2d2342499e8bdb63615680ce4e (feature branch
  goal/workflow-execution-control-v1 based on github/main d661af5). That
  acceptance applied to the original r1 body only; its body was byte-identical
  to that reviewed head except the original acceptance frontmatter. Preceding
  mandate record (proposal): GOAL = WORKFLOW_EXECUTION_CONTROL_V1 (2026-09-24),
  Scopes A/C/D/F/G svc-workflow side; §11 authority invariants; §12 Cases 1-10.

  Additive amendment acceptance: root independently accepted proposal commit
  083c52a79efbdb0efc83d6dfaa113a7aa75a0b0d and patch SHA-256
  6deb11e3873a734eb4aa4ffb21317c12e5da36b20ae620e9af57e84b7780b47f after
  verifying the complete applied body SHA-256
  6774a1481563f5059bf774e8dae05365e80bb1f4327b55ce4fec83096a3b01c0;
  acceptance receipt: /Users/yanfenma/workspace/artifacts/workflow-unattended-
  mechanisms-20261027/jwt-static-expiry-repro-luna/root-acceptance-review.json
  (SHA-256 899725e4d0242d158ade812c89a85a79d257b6365cec6c7259b59574a0d3d14d).
  Delegated under user's explicit authorization recorded in that receipt.
  Scope is source and isolated validation only; production apply authority NONE;
  production credential/grant readiness UNKNOWN.

  CTR-SWEC-010 additive amendment acceptance: Codex root, explicitly delegated
  by mayf3, accepted reviewed head 2c7e59a14d5d84ecc0688c47c30c5f04d0f05900 after independent
  /root/recovery_contract_review_luna PASS and complete scratch-apply byte checks.
  The accepted body SHA-256 is c5b34b713f1c5045ddcf9c23d1d663e572f4c10be4e2a15e001ed7b8e2030194.
  Existing CTR-SWEC-001..009 and preceding body bytes are unchanged; this
  lifecycle-only commit records the exact accepted addition. Authority activates
  on merge; source/isolated implementation is a later REUSE task. Production
  apply authority remains NONE.
accepted_at: 2026-10-27T17:37:27.570181+00:00
acceptance_review_verdict: PASS
independent_semantic_reviewer: /root/recovery_contract_review_luna
acceptance_receipt: docs/reports/OWNER_ASSISTANCE_RESUME_CONTRACT_ACCEPTANCE_20261028.json
acceptance_receipt_sha256: 38cad7f44a23d5c10b1af71a1680e9134718f446c1b0b0644d08e0c340b7048c
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
    revision: 71882c48593599c550259892e255ec9c3d73b7f2
    relation: interoperates_with
supersedes: []
superseded_by: null
owners:
  - mayf3
architecture_authority: SVC_WORKFLOW_ARCHITECTURE_V0_4_3
local_authority_relations:
  - authority_id: SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
    relation: amended_by
    note: due-set narrowing in Amendment A section 7
  - authority_id: SVC_WORKFLOW_DISPATCH_INTENT_KEYSET_CONTINUATION_V1
    relation: preserved_untouched
    note: cursor/order semantics byte-identical
production_apply_authority: none
repo: mayf3/svc-workflow
date: 2026-09-24
base_head: d661af5caa135d783e33718bfa2da9e8aafdd9d5 (github/main)
revision: r2
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
(system-counted, never agent-counted) with OWNER_PENDING assistance through
the EXISTING assistance machinery; (E/F) a system execution-escalation
ingress for the execution runtime and a push-first kick outbox — the
scheduler/due-feed poll stays the correctness path; and a due-set narrowing
that stops dispatching into any visit with an open assistance case. No separate
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
   'execution_policy'}}`) and emit `ASSISTANCE_REQUESTED` using the policy's
   synthetic request receipt. The version bump makes the engine's stale probe
   mechanically see `progressed`. The system MUST NOT auto-escalate this case;
   the Domain Owner uses the existing assistance resolve/escalate commands to
   either solve it or explicitly move it to HUMAN_REQUIRED.
5. Queue a `FORUM_EVENT` outbox row `event_key =
   assistance_requested:<caseId>` with `eventType=owner_attention_requested`
   so the owner-attention fact is visible on the canonical thread.

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
- `returnCount + 1 == max` ⇒ the committing tx ALSO opens an OWNER_PENDING
  assistance case on the transition's TARGET visit (the visit the instance is
  about to sit on), in the same tx, using the same owner-attention mechanics as
  §5 step 4 (caller = transitioning principal; provenance
  `source: 'return_policy'`), plus the FORUM_EVENT outbox row
  `return_policy_reached:<instanceId>:<transitionId>`.

Default disposition is REQUIRE_OWNER_ATTENTION; nothing force-advances, ever.
Only an explicit Domain Owner assistance escalation changes the case to
HUMAN_REQUIRED.

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
- Case 6: max-th RETURN opens OWNER_PENDING on the target visit;
  further RETURN ⇒ 409 `return_policy_exhausted`.
- Case 4: escalation endpoint creates OWNER_PENDING assistance case,
  bumps state version, is idempotent on a second call; due feed excludes
  the escalated visit (§7).
- Case 8: forum sync disabled ⇒ create/transition still 2xx; enabling
  later drains the backlog in order.
- Case 10: two ensure-thread passes ⇒ exactly one thread (mock forum 409
  race); binding UNIQUE holds.
- ACC-SWEC-009-001: an owner-wake attempt with complete dedicated Auth config
  calls the Auth token endpoint once with exact `client_credentials`, audience,
  and `workflow.execute`, then POSTs the closed owner-wake payload to the exact
  product-api route; only 2xx marks the outbox row delivered. The response token type and returned scope are checked exactly, and
  `expires_in` is a positive integer no greater than 600 seconds.
- ACC-SWEC-009-002: valid cached owner-wake token is reused only before its
  bounded renewal point; after renewal point, the sender obtains a new token
  before the next owner-wake POST. Simulated short expiry against the actual
  product-api verifier proves the old bearer is rejected and a newly issued
  bearer is accepted without waiting ten minutes.
- ACC-SWEC-009-003: a delayed Auth response whose returned TTL leaves no safe
  window when measured from the monotonic instant before the Auth request starts
  MUST NOT be cached or used for a downstream POST; the owner-wake row remains
  pending under existing backoff. A short-TTL fixture with controlled Auth
  response delay verifies the boundary without waiting for real expiry.
- ACC-SWEC-009-004: Auth timeout, malformed token response, `invalid_client`,
  `invalid_scope`, product-api 401/403/5xx, or transport failure performs at
  most one Auth request and one owner-wake POST per reconciler attempt, records
  only sanitized error metadata, and leaves the durable outbox row pending
  under existing backoff (delay capped at 600 seconds). Each due-row attempt is
  bounded to one Auth request and one downstream POST; a 401 invalidates the
  cached bearer without an immediate same-pass replay. Rows are not dead-lettered.
- ACC-SWEC-009-005: missing/partial new owner-wake configuration never falls
  back to `WORKFLOW_EXECUTION_KICK_TOKEN`; that legacy token remains exclusive
  to CTR-SWEC-007 `/workflow-execution/kicks`. Forum tokens, wrong audience,
  wrong scope, or a token cache entry from another `(client, audience, scope)`
  tuple cannot authenticate an owner wake. Secret/token values never appear in
  logs, `last_error`, receipts, or test output.

## 12. CTR-SWEC-009 — durable Owner Assistance wake authentication

This Contract elaborates the existing durable outbox/reconciler decision and
Agent Core's accepted `CTR-WEC1-005` durable Domain Owner wake. It adds no new
business transition or Agent Core endpoint and does not change CTR-SWEC-001..008.

`OWNER_ASSISTANCE_WAKE` rows remain same-transaction durable facts for
`BUSINESS` workflows only, keyed by `owner-assistance:<assistanceCaseId>`; a
`NON_BUSINESS_TEST` instance MUST NOT enqueue or deliver one. The existing
reconciler sends each due row to the exact configured loopback URL
`WORKFLOW_OWNER_ASSISTANCE_WAKE_URL` using the accepted closed payload and the
existing product-api endpoint `POST /workflow-execution/owner-assistance-wakes`.
The configured URL MUST be absolute HTTP to `127.0.0.1`, with that exact path,
and MUST have no userinfo, query, or fragment.

The owner-wake auth configuration is complete only when every setting below
is non-empty and valid:

```text
WORKFLOW_OWNER_ASSISTANCE_WAKE_URL
WORKFLOW_OWNER_ASSISTANCE_WAKE_AUTH_BASE_URL
WORKFLOW_OWNER_ASSISTANCE_WAKE_CLIENT_ID
WORKFLOW_OWNER_ASSISTANCE_WAKE_CLIENT_SECRET
WORKFLOW_OWNER_ASSISTANCE_WAKE_AUDIENCE
```

The Auth audience MUST exactly equal the audience accepted by the product-api
`SCHEDULER_AUTH_AUDIENCE` verifier configuration. This equality is a
deployment/readiness fact that must be verified from non-secret configuration;
svc-workflow cannot infer it from the token endpoint. The Auth base URL MUST be
an absolute URL without userinfo, query, or fragment, and MUST use HTTPS or HTTP
loopback. The sender MUST reject redirects on both Auth and product-api calls.
The sender MUST request
only `grant_type=client_credentials`, that exact audience as `resource`, and
exactly `workflow.execute` as `scope`. Auth must bind the client to an active
machine principal and explicit grant for that exact audience/scope. The
production poller identity and its grant are deployment gates and are not
asserted by this Spec or by isolated synthetic evidence.

The dedicated client ID and secret MUST be injected through the existing
deployment-managed service-secret channel used for svc-workflow credentials;
this Contract creates no new secret store or provisioning route. The sender
MUST acquire access tokens from the configured Auth service at
`POST <WORKFLOW_OWNER_ASSISTANCE_WAKE_AUTH_BASE_URL>/oauth/token` using the
configured dedicated client credential. It MUST validate `token_type = Bearer`,
a non-empty access token, response scope exactly `workflow.execute`, and integer
`expires_in` in the verifier-supported range `1..=600` seconds.
Token bytes and client secrets MUST remain process-local and MUST NOT be
persisted, included in logs, `last_error`, metrics labels, receipts, or
caller-visible responses. Cache expiry MUST use a monotonic clock.
The cache MUST be keyed by the exact `(client_id, audience, scope)` tuple,
MUST be in process memory only, and MUST reacquire a token before its expiry
renewal point, where the renewal margin is exactly min(60 seconds, 10%
of expires_in). The expiry origin is the monotonic instant immediately before
the Auth request begins, so Auth response latency cannot extend local validity.
If the response arrives at or after request_started + expires_in - renewal_margin,
the token MUST NOT be cached or used for an owner-wake POST; the outbox row stays
pending under existing backoff. Otherwise, expiry MUST be calculated from that
request-start instant. Concurrent misses for the
same key MUST be serialized or coalesced; entries from Forum auth or another
resource/scope MUST never be reused.

`WORKFLOW_EXECUTION_KICK_TOKEN` MUST NOT be used for
`OWNER_ASSISTANCE_WAKE`, including when owner-wake settings are missing,
partial, invalid, or rejected. The owner-wake path MUST use exactly one mode:
the dedicated client-credentials settings above; no static owner-wake token
setting or fallback is supported. CTR-SWEC-007 execution-kick delivery
authentication remains unchanged by this Contract. Absence/partial settings
cause each owner-wake delivery attempt to fail closed with a sanitized
configuration error and the outbox row pending. Deployments separately
provision the owner-wake credential and exact grant; configuring the existing
kick token does not enable, replace, or grant owner-wake authentication.

Each reconciler attempt is bounded to at most one Auth token request and one
owner-wake POST. The Auth and product-api requests each have a 5-second timeout. Auth endpoint failure,
malformed response, authorization denial, wrong/expired bearer (401), or any
non-2xx owner-wake response fails that attempt and MUST NOT mark the row
delivered. On 401 the sender invalidates that cache entry; it MUST NOT retry
inside the same pass. The row remains durable and is eligible only at its
existing `next_attempt_at` using CTR-SWEC-002 exponential backoff (capped at
600 seconds); no row is deleted or dead-lettered. A later pass can obtain a
new token. On a 2xx owner-wake response only, the sender marks the row
`delivered_at`. No change is made to execution-kick 4xx semantics or the
correctness role of Agent Core's due poll.

This Contract does not authorize credential provisioning, Auth grant changes,
production configuration, or service restart. Those require their own
Domain Owner/deployment authority and receipts.

## 13. CTR-SWEC-010 — Owner-resolved workflow execution evidence

This read-only companion preserves CTR-VAI-009's exact seven-field due DTO.
Agent Core consumes it under CTR-WEC1-008; no Core product-api endpoint is
created by this Contract.

```text
GET /internal/v1/dispatch-intents/{dispatchIntentId}/owner-assistance-resume-evidence
query: previousAttemptId=<opaque attempt ID string>&preDeliveryWorkflowStateVersion=<integer>
scope: workflow.read + direct token + enabled GLOBAL_SCHEDULER_READ binding
200 {"evidence": null | {
  "dispatchIntentId": UUID, "workflowInstanceId": UUID, "nodeVisitId": UUID,
  "ownerPrincipalId": UUID, "activationState": "ACTIVE", "workflowStateVersion": integer,
  "assistanceCaseId": UUID, "requestEventSequence": integer,
  "resolutionEventSequence": integer, "requesterPrincipalId": UUID,
  "resolverPrincipalId": UUID, "ingress": "assignee" | "execution_policy",
  "reason": null | "ATTEMPTS_EXHAUSTED" | "STALE_LOOP_EXHAUSTED",
  "lastAttemptId": null | opaque attempt ID string
}}
```

The request and response are closed shapes. `dispatchIntentId` is the exact
path UUID; `previousAttemptId` is the non-empty opaque Core attempt ID
(currently `wfeat-*`), not a UUID. Versions and sequences are integers on the
svc workflow-state-version scale. `reason` and `lastAttemptId` are present
and null for `ingress: assignee`; for `execution_policy` both are non-null,
the reason is one of the two enumerated policy reasons, and `lastAttemptId`
is the persisted policy value, never an echo of the query parameter.

Only the enabled scheduler caller above may query one exact current active
dispatch intent. Svc derives all identities and provenance from persisted
rows in a consistent read; callers supply no requester/resolver identity.
It returns at most the earliest qualifying case by
`(resolutionEventSequence, assistanceCaseId)`, or `{"evidence":null}` when no
case qualifies. Existing authentication/validation/storage error semantics
remain errors, not permission-bearing evidence. No Assistance message,
supporting payload, credential or audit body is returned.

The activation MUST still be ACTIVE for the current nonterminal instance,
current visit and current assignee; the returned `ownerPrincipalId` is that
assignee, not the resolving Domain Owner. The case belongs to that exact
instance/visit and remains `RESOLVED`. An open OWNER_PENDING or HUMAN_REQUIRED
case still excludes the visit under CTR-SWEC-006. Changed activation, visit or
assignee cannot be authorized by an old case.

The request/resolution joins MUST use the case's persisted
`request_command_id` / `resolution_command_id`, completed successful command
receipts of types `REQUEST_WORKFLOW_ASSISTANCE` /
`RESOLVE_WORKFLOW_ASSISTANCE`, and exact workflow event types
`ASSISTANCE_REQUESTED` / `ASSISTANCE_RESOLVED`. Each event must match the
command ID, workflow instance, source/target visit and exact
`event_data.assistanceCaseId`; its actor must match both the receipt Principal
and the case's requested/resolved Principal. Request new status is
OWNER_PENDING and resolution new status is RESOLVED. Missing, conflicting or
ambiguous provenance fails closed. The successful resolution command is the
existing explicit action checked by svc against the exact Domain's Owner
authority at resolution; a timestamp or a claimed role is not a substitute.
No rule requires resolver and executor to be different Principals.

Both ingress paths use those exact joins; classify them from persisted
server-generated provenance, never solely from free supporting JSON:

- `assignee`: the ordinary request command and event attest the authenticated
  requester equals the current activation/visit assignee. `reason` and
  `lastAttemptId` MUST be null; the ordinary API persists no authoritative
  attempt binding, and neither caller payload nor the query's
  `previousAttemptId` may be echoed as one.
- `execution_policy`: the existing system ingress has a completed
  `SYSTEM_EXECUTION_ESCALATION` receipt. Its same-actor synthetic request
  receipt is linked by the exact existing
  `policy-req:<system-command-id>` idempotency key, has the server-generated
  `{"source":"execution_policy"}` receipt body, and both receipts resolve
  to the exact case (the system receipt's `assistanceCaseId` matches).
  The case's persisted `supportingPayload.source` is `execution_policy`,
  its reason is `ATTEMPTS_EXHAUSTED` or `STALE_LOOP_EXHAUSTED`, its non-null
  `lastAttemptId` exactly equals query `previousAttemptId`, and its
  `dispatchIntentId` exactly equals the path ID. Missing legacy policy
  bindings do not qualify. The system requester need not equal the assignee;
  it is not relabeled as executor. Free ordinary payload fields cannot
  manufacture this policy provenance.

Each joined event MUST satisfy
`event_sequence == new_workflow_state_version`, preserving the existing
event writer's shared monotonic scale. Both sequences are strictly after
the supplied preceding attempt baseline, with
`baseline < requestEventSequence < resolutionEventSequence <= workflowStateVersion`.
The route does not invent or verify Core's baseline: Core supplies its
durably recorded pre-delivery fact and remains responsible for the exact
preceding attempt and assignee match. Wall-clock order alone never qualifies.
After Core records the returned current version as its new pre-delivery
baseline, all already-resolved old markers are at or below that baseline;
neither replay nor selecting another old case can authorize another run.
Core's one-append single-case consumption fence remains authoritative for
generation admission; this read creates no consumption record or second
ledger.

This Contract adds no assistance write action, role grant, repair permission,
or automatic resolve. Only the existing explicit authorized Owner resolve
is the authorization fact; underlying repair scope is unchanged.

### Acceptance for CTR-SWEC-010

- ACC-SWEC-010-001: isolated service-route/store tests using actual persisted
  ordinary request/resolve transactions and system-policy request/resolve
  transactions return the exact envelope and one marker with the defined
  nullable fields. Required evidence: request/command/case/event coordinates,
  actual response and exact Core-compatible DTO; fixture-only assertions do
  not establish live production behavior.
- ACC-SWEC-010-002: wrong/missing scheduler binding or scope cannot read;
  wrong command/type/case/instance/visit/actor joins, pending/voided case,
  forged ordinary policy payload, absent or wrong policy attempt/activation
  binding, pre-baseline/reversed sequences and unequal event/version scale
  yield no permission-bearing evidence. Owner and assignee may be the same
  Principal when the existing roles authorize both actions. Tests prove
  no payload/free text leaks and the seven-field due DTO remains unchanged.
- ACC-SWEC-010-003: several eligible resolved cases select the earliest pair;
  repeating the query is read-only, and the next attempt's newer baseline
  excludes every previously resolved marker. Changed active identity rejects
  old evidence. Required evidence: actual store queries and response bodies;
  any unsupported marker, shape mismatch or state mutation fails acceptance.
