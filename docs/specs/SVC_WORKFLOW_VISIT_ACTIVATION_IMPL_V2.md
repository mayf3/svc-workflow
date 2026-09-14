---
spec_id: SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V2
status: accepted
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
accepted_date: 2026-09-15
date: 2026-09-15
scope:
  - svc-workflow VISIT_ACTIVATION_V1 runtime core (canonical activation facts, DISPATCH_INTENT, wake, due-intent read)
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_0
supersedes:
  - SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V2 — Visit-Activation Runtime Core (Slice D phase 1, INSTANCE_INPUT owners)

```text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V2
STATUS = accepted
BASE_REPOSITORY = mayf3/svc-workflow
BASE_COMMIT = 6c05e0f (github/main at successor authoring; accepted V1 remains authoritative base text)
CHANGE_CLASS = NON_MECHANICAL
PREFLIGHT_MODE = SUPERSEDE (SPEC_GOVERNANCE_V0 §9.2 — whole-authority successor)
AUTHORITY_HANDLING = supersedes accepted SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1 (atomic docs-only
  transition in this PR: V2.status=accepted; V1.status=superseded, superseded_by=V2)
SUPERSESSION_BASIS = Owner ruling E1 (PRESERVE_INSTANCE_INPUT_PRINCIPAL_SEMANTICS, 2026-09-14):
  the V1 normative owner closed set (CTR-VAI-002/004/011) rewrites normative meaning, therefore
  AMEND is invalid per SPEC_GOVERNANCE_V0 §9.1 and SUPERSEDE is the only legal handling.
PRODUCT_CODE_CHANGE = THIS SPEC AUTHORIZES IT
PRODUCTION_CHANGE = NONE
PRODUCTION_APPLY_AUTHORITY = none
ACCEPTANCE_BASIS = Owner ruling KEEP_ACCEPTED_V6 (2026-09-02) delegates the
  autonomous authority->audit->acceptance->implementation->audit chain;
  independent spec review performed at authoring head before acceptance.
```

> Implementation-authorizing child of accepted `SVC_WORKFLOW_ARCHITECTURE_V0_4_0`
> (Slice D, per V6 §18). It freezes the phase-1 runtime core for the
> VISIT_ACTIVATION_V1 semantic model. Production apply, cutover barrier
> selection, and Legacy migration remain separately gated and are NOT
> authorized here. `IMPLEMENTATION_READY` becomes YES for the scoped surface
> below only.

## 1. Goal

Implement the minimum complete, tested runtime core that makes the accepted
v0.4.0 activation model real for the `VISIT_ACTIVATION_V1` semantic model:

1. explicit immutable semantic-model identity `VISIT_ACTIVATION_V1` on
   Definition Versions and Instances with DB-enforced equality;
2. new-model graph validation (`TASK | TERMINAL`);
3. canonical activation facts (`HUMAN_WORK_ITEM | DISPATCH_INTENT`) created
   and closed atomically with Visit entry/closure;
4. server-authored initial `nextEligibleAt` in the activation transaction;
5. authorized wake (early wake to server-now) and the singular due read for
   the Scheduler;
6. strict Legacy protection: no Legacy instance ever gains an activation; no
   new-model instance takes Legacy-only semantics.

## 2. Exact coordinates and inputs

```text
GOVERNING_PRODUCT_DIRECTION = SVC_WORKFLOW_PRODUCT_BOUNDARY_V6 (accepted)
GOVERNING_ARCHITECTURE = SVC_WORKFLOW_ARCHITECTURE_V0_4_0 (accepted, merged b5bb7eec)
BASE_COMMIT = b5bb7eecdb9bfdf41b96e470df9c845c538edcad
PRIOR_ARCHITECTURE = SVC_WORKFLOW_ARCHITECTURE_V0_3_1 (superseded, historical)
RETAINED_REFINEMENT = SVC_WORKFLOW_ARCHITECTURE_V0_3_2 (unchanged)
EXPECTED_MIGRATION_VERSION_BEFORE = 23
EXPECTED_MIGRATION_VERSION_AFTER = 23 (no schema change in this successor;
the delta is validator + create/transition guards only)
```

## 3. Storage contracts (migration `0023_visit_activation_v1.sql`)

### CTR-VAI-001 — Semantic-model identity

`semantic_model_version` gains value `3` = `VISIT_ACTIVATION_V1`:

- `workflow_definition_versions` check widened to `(1, 2, 3)`; existing rows
  and values `1`/`2` keep their exact meaning (CTR-ARCH-003: `2` Minimal is
  NOT aliased to `3`);
- `workflow_instances` gains `semantic_model_version SMALLINT NOT NULL`
  backfilled from its immutable Definition Version;
- equality is DB-enforced: `UNIQUE (definition_version_id,
  semantic_model_version)` on versions + composite FOREIGN KEY from
  `workflow_instances (definition_version_id, semantic_model_version)`;
- the Rust runtime rejects unknown values fail-closed (existing `other` arm).

### CTR-VAI-002 — Node model encoding

`node_type` enum gains `'TASK'`. `VISIT_ACTIVATION_V1` graphs use exactly
`TASK | TERMINAL`; the graph validator (CTR-VAI-011) rejects `DRAFT`/`NORMAL`
node kinds in new-model graphs. TASK owner refs come from the closed set
`WORKFLOW_CREATOR | DOMAIN_OWNER | FIXED_PRINCIPAL | INSTANCE_INPUT_PRINCIPAL`
(CTR-VAI-015 extends the set with INSTANCE_INPUT_PRINCIPAL, Owner ruling E1).
Legacy rows and validators are unchanged. TERMINAL nodes carry no resolved owner (visits are
never created for TERMINAL).

### CTR-VAI-003 — Immutable activation fact tables

~~~sql
activation_kind := ENUM('HUMAN_WORK_ITEM','DISPATCH_INTENT')

workflow_activations (
  activation_id UUID PK,
  workflow_instance_id FK NOT NULL,
  node_visit_id  FK NOT NULL UNIQUE,          -- exactly-one, DB-enforced
  activation_kind activation_kind NOT NULL,
  owner_principal_id FK (principals) NOT NULL,
  activation_at TIMESTAMPTZ NOT NULL,         -- server-authored, same tx
  initial_next_eligible_at TIMESTAMPTZ,       -- NOT NULL iff DISPATCH_INTENT
  command_id UUID NOT NULL,                   -- owning command receipt
  CHECK (activation_kind = 'DISPATCH_INTENT' ==> initial_next_eligible_at IS NOT NULL),
  CHECK (activation_kind = 'HUMAN_WORK_ITEM' ==> initial_next_eligible_at IS NULL)
)

workflow_activation_closures (
  activation_id UUID PK FK(workflow_activations),
  closed_at TIMESTAMPTZ NOT NULL,
  closure_reason TEXT NOT NULL (1..128),
  command_id UUID NOT NULL,
  event_id UUID NULL                          -- set when the closure command
                                              )   -- writes its Event
)

workflow_dispatch_eligibility_events (
  eligibility_event_id UUID PK,
  activation_id FK NOT NULL,                  -- DISPATCH_INTENT activations only
  previous_next_eligible_at TIMESTAMPTZ NOT NULL,
  new_next_eligible_at TIMESTAMPTZ NOT NULL,
  cause_class TEXT NOT NULL (1..64),          -- e.g. WAKE | SCHEDULER_DEFER
  command_id UUID NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
)
~~~

All three tables are append-only immutable fact families: UPDATE and DELETE
are rejected by database triggers (same enforcement style as the existing
immutability triggers). `workflow_activations.node_visit_id` UNIQUE is the
mechanical exactly-one invariant (one activation fact may ever exist per
Visit; repair of a provably missing activation is out of scope for phase 1,
see §8). Active/closed and current `nextEligibleAt` are derived:

```text
active(activation)         = no workflow_activation_closures row exists
current nextEligibleAt(a)  = value of the latest
                             workflow_dispatch_eligibility_events row by
                             (created_at, eligibility_event_id), else
                             initial_next_eligible_at
```

A partial index on `workflow_activations (activation_kind) WHERE
activation_kind = 'DISPATCH_INTENT'` plus the closure PK supports the bounded
due poll. The activation tables are the durable delivery-obligation surface
of v0.4.0 §5.11 (dedicated-table poll); no Workflow Instance/global-summary
scan is used.

### CTR-VAI-004 — Create atomic closure

For `VISIT_ACTIVATION_V1`, `create_workflow_instance_atomically` extends its
existing single transaction with: entry `TASK` Visit (resolved owner
snapshot) and one `workflow_activations` row whose `kind` is derived only
from the resolved canonical Principal type (`HUMAN -> HUMAN_WORK_ITEM`,
`AGENT -> DISPATCH_INTENT`), whose `activation_at` is the server transaction
timestamp, and whose `initial_next_eligible_at = activation_at` iff kind is
`DISPATCH_INTENT`. The INSTANCE_CREATED Event `event_data` records
`activationId`/`activationKind`/`initialNextEligibleAt`. Owner resolution
failure (missing/disabled/SERVICE/not HUMAN-or-AGENT) commits nothing with a
deterministic `owner_resolution_failed` class. No caller field can supply the
owner or timestamp.

Create-time safety gate (CTR-VAI-015): every INSTANCE_INPUT_PRINCIPAL owner
key declared by the Definition must resolve at creation — key present in the
initial context payload, value a string UUID, Principal canonical, enabled,
and of type HUMAN or AGENT — otherwise creation commits nothing with the same
`owner_resolution_failed` class. Half-valid instances are never created.

### CTR-VAI-005 — Transition atomic closure

A successful Transition on a `VISIT_ACTIVATION_V1` Instance closes the source
Visit activation (one `workflow_activation_closures` row, reason
`TRANSITIONED`) and creates the target Visit + its activation in the same
transaction; TERMINAL targets create no target activation. A target TASK
with `INSTANCE_INPUT_PRINCIPAL` resolves its owner from the current
authoritative context revision (assignee_input_key -> UUID -> canonical,
enabled, HUMAN|AGENT) inside the SAME transaction, and the created activation
kind derives only from that resolved Principal type. Existing
version/Event/Receipt/audit invariants are unchanged (one increment, one
Event).

### CTR-VAI-006 — Cancel and Archive closure semantics

Cancel of a `VISIT_ACTIVATION_V1` Instance atomically closes the current
activation (reason `CANCELLED`) while preserving v0.3.2 Visit semantics.
Archive of a `VISIT_ACTIVATION_V1` Instance with an active activation fails
closed (deterministic `active_activation_exists` class).

### CTR-VAI-007 — Administrative move and terminate closure semantics

Admin Recovery `MOVE_TO_NODE` and `TERMINATE_INSTANCE` on a
`VISIT_ACTIVATION_V1` Instance atomically close the source activation
(reasons `ADMIN_MOVE` / `ADMIN_TERMINATE`) and create the correct target
TASK activation / no terminal activation, per CTR-ARCH-019. Legacy behavior
is unchanged.

### CTR-VAI-008 — Wake command

New command `WAKE_DISPATCH_INTENT`:

```text
POST /internal/v1/workflow-instances/{workflowInstanceId}/node-visits/{nodeVisitId}/wake
scope workflow.execute + direct token + enabled GLOBAL_SCHEDULER_READ binding
request: { expectedWorkflowStateVersion, idempotencyKey, cause? }
```

- binds the exact Instance + nodeVisitId; resolves the unique activation;
- applied only when the activation exists, is `DISPATCH_INTENT`, and is
  active: appends one `workflow_dispatch_eligibility_events` row
  (`previous = current`, `new = server now`, `cause_class = WAKE`),
  increments `workflowStateVersion` once, writes exactly one
  `WAKE_DISPATCH_INTENT` Event, one Receipt, and audit — atomically;
- stale/closed/current-mismatch/Version-mismatch wake commits a durable
  receipt + attempt audit and returns `200 {wakeApplied: false, reason}`
  with NO version increment, NO Event, NO fact row;
- idempotency: same principal+key+request replays the original outcome;
  different request conflicts (existing receipt machinery);
- it never mutates node/owner, creates an activation, performs a Transition,
  or starts an Agent.

### CTR-VAI-009 — Due Dispatch Intent read

```text
GET /internal/v1/dispatch-intents?limit=N
scope workflow.read + direct token + enabled GLOBAL_SCHEDULER_READ binding
```

Returns at most `N` (1..100) active DISPATCH_INTENT activations with
`current nextEligibleAt <= authoritative now`, ordered by `(current
nextEligibleAt, activation_id)`. Each record contains exactly:

```text
dispatchIntentId, nodeVisitId, workflowInstanceId,
ownerPrincipalId, nextEligibleAt, createdAt, updatedAt
```

(`createdAt` = activation_at; `updatedAt` = latest eligibility change time or
activation_at). It excludes Context/title/labels/keys/Submission/EventData/
Assistance/credentials/audit payloads/Transition options/metadata. The role
gate is the fail-closed server-side mapping of the V6
`GLOBAL_SCHEDULER_READ` product capability: an enabled
`GLOBAL_SCHEDULER_READ` binding satisfies it; nothing else does; no existing
Reader/Coordinator binding is automatically mapped and no Grant is created by
this Spec. Missing gate = 403 `scheduler_read_role_required`.

### CTR-VAI-010 — GLOBAL_SCHEDULER_READ role value

`GLOBAL_SCHEDULER_READ` becomes an accepted `global_role_bindings.role_key`
value in the admin provisioning API validation set. No binding is created by
migration or code; provisioning remains via the existing admin API under its
existing authority (production supply stays with auth-service Slice B/C).
Fail-closed when absent.

### CTR-VAI-011 — New-model graph validation

`validate_definition_graph` dispatches on semantic model: model `3` runs the
new `VISIT_ACTIVATION_V1` validator: node kinds exactly `TASK | TERMINAL`;
exactly one entry TASK (no incoming primary ADVANCE); every TASK has exactly
one primary ADVANCE; primary ADVANCE edges form one acyclic deterministic
path ending at a TERMINAL; RETURN edges target a strictly earlier reachable
TASK; TERMINATE edges target a TERMINAL; TERMINAL has no outgoing edge; all
nodes reachable from the entry TASK; `DRAFT`/`NORMAL` node kinds are
rejected; TASK owner refs come from the closed set `WORKFLOW_CREATOR |
DOMAIN_OWNER | FIXED_PRINCIPAL | INSTANCE_INPUT_PRINCIPAL`, where an
`INSTANCE_INPUT_PRINCIPAL` TASK must declare an `assignee_input_key` covered
by the Definition's context schema (required-key coverage per CTR-VAI-015).
Draft-time and publish-time validation both run it. Model `1`/`2` paths are
byte-for-meaning unchanged.

### CTR-VAI-012 — Legacy protection (mutual fail-closed)

- LEGACY (`1`) and Minimal (`2`) Instances never read or write activation
  tables (their command paths are unchanged);
- `VISIT_ACTIVATION_V1` Instances reject Legacy-only commands
  (`revise`, `revise-and-transition`) with deterministic 422
  `legacy_command_not_supported_for_semantic_model`;
- `REBUILD_PROJECTION` on a `VISIT_ACTIVATION_V1` Instance additionally
  validates the activation fact families for that Instance (exactly-one
  activation per Visit, closures follow creation, eligibility rows reference
  DISPATCH_INTENT activations with sane previous/new ordering) and fails
  closed on drift without mutating facts;
- unknown semantic model values fail closed everywhere they are dispatched.

### CTR-VAI-013 — Compatibility

Existing Domain/global lists, worklists, details, timeline, Submission
history, HTTP envelope, error codes, pagination, and the Global Reader/
Coordinator gates are byte-for-meaning unchanged. No `dispatchable` field,
`dispatch_blocked_reasons`, or `dispatchableOnly` parameter is added to any
existing surface (Owner ruling KEEP_ACCEPTED_V6). Additive wire surface is
exactly CTR-VAI-008/009. `EXPECTED_MIGRATION_VERSION` becomes 23.

### CTR-VAI-014 — Audit and idempotency

Wake and due-read are protected operations: successful and
authenticated-denied attempts write non-sensitive audit rows (existing
attempt-audit/security-audit machinery). Wake keeps existing receipt
idempotency semantics (same key+request replays; changed request conflicts).
The due read performs its role check inside the same read snapshot as the
query.

### CTR-VAI-015 — INSTANCE_INPUT_PRINCIPAL owner semantics (E1)

`VISIT_ACTIVATION_V1` TASK nodes may declare
`INSTANCE_INPUT_PRINCIPAL` + `assignee_input_key`. The semantic is the
instance-scoped dynamic assignee: each Instance resolves its own owner from
its context payload at Visit creation.

- Owner resolution happens at NODE VISIT CREATION (entry or transition),
  always from the current authoritative context revision, inside the same
  authoritative transaction that writes the Visit and activation:
  context value -> string UUID -> canonical Principal -> enabled check ->
  type check (HUMAN | AGENT; SERVICE never owns new-model work).
- The resolved Principal is written to `NodeVisit.assignee_principal_id`
  and becomes the activation `owner_principal_id`; the activation kind
  derives ONLY from that resolved Principal's canonical type
  (`AGENT -> DISPATCH_INTENT`, `HUMAN -> HUMAN_WORK_ITEM`) — never from any
  caller hint, node name, or context self-report.
- Resolved Visits are immutable: later context or identity changes never
  rewrite an existing Visit's assignee (subsequent INSTANCE_INPUT TASKs
  resolve from the then-current authoritative context).
- The Definition stores only `assignee_input_key`; no principal UUID is
  frozen into the graph, and the context key is NOT a legacy-identity
  successor slot (create/admission-time canonical identity gating applies to
  every concrete context value).
- All other V1 contracts (immutable activation facts, wake, due read,
  legacy protection, audit/idempotency) are unchanged.

## 4. Explicit out of scope (FOLLOW_UP_DEBT, not blocking this Spec)

| Item | Authority home |
|---|---|
| ONE_TIME_MIGRATE, DRAIN/HISTORICAL_REPLAY enforcement, cutover barrier application | later cutover/migration Spec (CTR-ARCH-028..031) |
| missing-activation repair command | later repair Spec (CTR-ARCH-027) |
| delivery obligation acknowledgement/transport protocol | later delivery Spec + external Scheduler authority (Slice F) |
| `GLOBAL_SCHEDULER_READ` grant supply to any Principal | auth-service Slice B/C + V6 designation authority |
| 365-day audit retention enforcement | separate audit-authority round |
| SCHEDULER_DEFER (future-dated eligibility) command | first external Scheduler-management authority (attempt-bound) |

Where a deferred item could otherwise be reached, the implementation fails
closed (e.g., no SCHEDULER_DEFER command exists in phase 1; wake is the only
eligibility writer).

## 5. Acceptance

Each ACC names its owning contract; executable evidence is collected by the
implementation candidate test suite (`tests/28_visit_activation_v1.rs` +
validator unit tests) against the run-scoped test database. An ACC is not
production authorization.

- **ACC-VAI-001 (CTR-VAI-001)** — Migration 0023 applies; versions accept 3;
  instance/model equality enforced (composite FK rejects mismatch);
  Minimal(2) rows unchanged. Environment: run-scoped test DB.
  Required evidence: migration run + mismatch-insert rejection + Minimal regression.
- **ACC-VAI-002 (CTR-VAI-004)** — Create with AGENT-owned entry TASK yields
  instance+visit+activation(`DISPATCH_INTENT`, initial nextEligibleAt =
  activation_at) in one commit; HUMAN owner yields `HUMAN_WORK_ITEM` with
  NULL initial eligibility; SERVICE/disabled/missing owner fails with zero
  facts. Required evidence: row traces + rollback traces.
- **ACC-VAI-003 (CTR-VAI-005)** — Transition closes source activation,
  creates target activation (or none for TERMINAL), exactly one version/Event.
  Required evidence: before/after activation state + Event count.
- **ACC-VAI-004 (CTR-VAI-006/007)** — Cancel closes activation; Archive with
  active activation fails; admin MOVE/TERMINATE handle activation correctly.
  Required evidence: traces per command.
- **ACC-VAI-005 (CTR-VAI-008)** — Wake on active due intent applies
  eligibility fact + one Event + one version + receipt; stale/closed wake is
  a no-op with receipt/audit; unauthorized caller gets 403 and an audit row;
  same key+request replays. Required evidence: response bodies + row counts.
- **ACC-VAI-006 (CTR-VAI-009/010)** — Due endpoint returns only active due
  intents with exactly the 7 minimum fields; role gate fail-closed
  (`scheduler_read_role_required`) for role-less callers, including
  GLOBAL_WORKFLOW_READER holders. Required evidence: schema-key snapshot +
  negative-role matrix.
- **ACC-VAI-007 (CTR-VAI-011)** — Validator accepts a conformant
  TASK/TERMINAL graph and rejects each malformed shape (multi-entry, missing
  primary, cycle, RETURN forward/into TERMINAL, TERMINATE into TASK, DRAFT/
  NORMAL nodes, INSTANCE_INPUT_PRINCIPAL ref, unreachable node). Required
  evidence: positive + negative fixtures.
- **ACC-VAI-008 (CTR-VAI-012/013)** — Legacy instance lifecycle produces zero
  activation rows; new-model revise returns
  `legacy_command_not_supported_for_semantic_model`; global list response
  shape is byte-identical pre/post. Required evidence: row counts + wire diff.
- **ACC-VAI-009 (CTR-VAI-014)** — Wake attempts (success, denial, no-op) all
  leave audit rows; sensitive payloads absent. Required evidence: audit rows.
- **ACC-VAI-010 (CTR-VAI-003)** — Direct UPDATE/DELETE on the three fact
  tables is rejected by triggers. Required evidence: rejected-statement traces.
- **ACC-VAI-011 (CTR-VAI-012)** — REBUILD_PROJECTION on a new-model Instance
  validates activation cardinality/consistency and fails closed on injected
  drift; Legacy instance rebuild behavior is unchanged. Required evidence:
  rebuild traces for clean and drifted fixtures.
- **ACC-VAI-012 (CTR-VAI-015)** — E1 acceptance matrix (A–L, no reductions):
  (A) validator accepts TASK+INSTANCE_INPUT_PRINCIPAL with a covered key;
  (B) missing `assignee_input_key` fails publish; (C) missing context value
  fails create; (D) non-UUID value fails create; (E) disabled Principal
  fails; (F) SERVICE Principal fails; (G) resolved AGENT yields
  DISPATCH_INTENT; (H) resolved HUMAN yields HUMAN_WORK_ITEM; (I) two
  Instances of the same Definition with context assignees A/B resolve their
  Visits independently to A/B; (J) an existing Visit assigned A remains A
  after later context/identity changes; (K) a later INSTANCE_INPUT TASK
  resolves from the current authoritative context revision; (L) the legacy
  retro migration maps `INSTANCE_INPUT_PRINCIPAL(assignee)` -> SMV3
  `INSTANCE_INPUT_PRINCIPAL(assignee)` with machine diff C_COUNT=0.
  Required evidence: positive/negative fixtures + row traces per case.

## 6. Test gate

The implementation candidate ships with: `cargo test --lib` green,
`cargo test --test 28_visit_activation_v1` green,
`cargo test --test 30_visit_activation_v2_instance_input` green (E1 A–L),
full workspace
`cargo test --workspace` green against the run-scoped test database
(known pre-existing failures outside scope remain as documented baseline).
`git diff --check` clean.
