---
spec_id: SVC_WORKFLOW_DANGLING_INSTANCE_CANCEL_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: none
scope:
  - mayf3/svc-workflow
  - workflow-instance-cancel-dangling-branch-m1b
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V8
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_3
external_authorities: []
related_authorities:
  - SVC_WORKFLOW_CANCEL_ARCHIVE_GOVERNANCE_V0_3_2
  - SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
supersedes: []
superseded_by: null
owners:
  - mayf3
title: Dangling-instance cancel branch and fail-closed runtime-fact invariant (M1B closure)
repo: mayf3/svc-workflow
base_head: 07d888205001708080aa50720e428c6f1c775c7b
date: 2026-09-18
production_apply_authority: none
product_code_changed_by_this_spec_pr: true
owner_directive: WORKFLOW_DANGLING_INSTANCE_CANCEL_M1B_CLOSURE_V1 (2026-09-18, SOURCE CLOSURE ONLY)
predecessor_evidence: PR #45 (fix/dangling-instance-cancel-v1-r2 @ 161bc9ee, OPEN draft, stale base bd47668, AUTHORITY_GATE=PENDING; approach transplanted, bytes not reused)
---

# SVC_WORKFLOW_DANGLING_INSTANCE_CANCEL_V1

> Proposed-stage note: `implementation_authority: none` while proposed. This
> candidate is authored under the Owner directive
> `WORKFLOW_DANGLING_INSTANCE_CANCEL_M1B_CLOSURE_V1`, which freezes the
> semantics below, orders source closure after independent review PASS, and
> separately forbids production deploy, production data mutation, and any
> parent-pool mutation in this goal. Acceptance remains an Owner act.

## 0. Problem

Four production instances (parent-goal frozen bucket F4,
snapshot `ACTIVE_POOL_SNAPSHOT_20260916_0709`) have
`workflow_instances.current_node_visit_id IS NULL` while
`cancelled = false AND archived_at IS NULL`. The deployed cancel path raises
`InternalConsistency("instance has no current node visit")` for this shape
(cancel_transaction.rs:318 pre-change), leaving no lawful disposition
surface; direct SQL mutation is forbidden. Fresh preflight at 2026-09-18
confirms all four rows: `F4_PREFLIGHT_MATCH_COUNT = 4`,
`F4_WITH_OPEN_RUNTIME_FACT = 0`.

## 1. Semantic delta (bounded to one branch)

The existing cancel command gains exactly one branch: when the locked
instance row has `current_node_visit_id IS NULL`:

1. FAIL-CLOSED: if any open runtime fact exists — any row of
   `workflow_activations` without a matching `workflow_activation_closures`
   row (the canonical model merges dispatch intents and human work items
   into this one activation family, Product Boundary V7 §5.4 canonical-activation
   enumeration) — cancel refuses with `InternalConsistency` before any
   mutation. The instance row, its runtime facts, and the state version stay
   unchanged; zero CANCELLED events are written. Such rows are data
   inconsistencies requiring separately accepted recovery; this branch never
   closes, repairs, or guesses them.
2. PASS-THROUGH: with no open runtime fact, cancel proceeds through the
   unchanged endpoint, authorization, receipt/idempotency machinery, and
   transaction shape: `cancelled = true`, `workflow_state_version += 1`
   (guarded update), exactly one `WORKFLOW_INSTANCE_CANCELLED` event whose
   `source_node_visit_id` is NULL — meaning "this instance never had a
   current visit". No synthetic visit is created; the frozen EFAP visit
   immutability discipline is untouched. The event payload field
   `cancelled_from_node_key` is the empty string for a dangling cancel
   (there is no current node key).
3. No activation closure step runs for a dangling instance (there is no
   current visit to close). For non-dangling instances nothing changes:
   same authorization (enabled DOMAIN_OWNER of the instance's Domain or
   enabled GLOBAL_WORKFLOW_COORDINATOR), same assistance voiding, same
   SMV3 activation closure, same guarded version increment, same event
   binding to the current visit.

Out of scope (explicitly unchanged): who can cancel; what else can be
cancelled; archive semantics (archive still requires terminal-or-cancelled;
a dangling instance must be cancelled first); coordinator permissions;
recovery/admin-override APIs; wire contract; schema; migration.

## 2. Contracts

### CTR-M1B-001 — Dangling eligibility
A dangling instance (locked row shows `current_node_visit_id IS NULL`) is
cancellable iff it has zero open runtime facts (no row of
`workflow_activations` lacking a closure). Violation: refuse with
`InternalConsistency`, zero mutation, provable unchanged instance/runtime
fact/state-version/event counts.

### CTR-M1B-002 — Event and projection shape
A successful dangling cancel writes exactly one `WORKFLOW_INSTANCE_CANCELLED`
event with `source_node_visit_id = NULL`, increments the state version
exactly once via the existing guarded update, and creates no visit, no
context revision, no submission, and no activation closure.

### CTR-M1B-003 — Normal-path zero regression
For `current_node_visit_id IS NOT NULL`, all pre-existing cancel semantics
are preserved byte-for-byte in behavior: authorization, assistance voiding,
SMV3 activation closure (including its `closed = false` fail-closed error),
version bump, event `source_node_visit_id` binding, idempotency replay and
conflict behavior.

### CTR-M1B-004 — Idempotency unchanged
Dangling cancels use the existing receipt machinery: same key + same request
replays the original outcome exactly once; same key + different request
conflicts with zero mutation. Deterministic cancel failures return without
persisting failure receipts (house cancel behavior shared by
NotDomainOwner/AlreadyCancelled/SourceNodeTerminal/InstanceArchived);
retrying while the refusing condition holds reproduces the same outcome
deterministically.

### CTR-M1B-005 — Atomicity
Every path (success, fail-closed refusal, infrastructure fault) commits all
or nothing: no partial cancelled flag, no orphan event, no runtime-fact
mutation, no state-version change.

### CTR-M1B-006 — Bounded branch
No code path outside the `current_node_visit_id IS NULL` branch changes
behavior. The implementation delta is confined to the cancel transaction;
review must confirm no authz widening, no synthetic visit, and no new
disposition surface.

## 3. Acceptance

```text
ACC-M1B-001 (T1): dangling + no runtime facts -> cancel succeeds; event
  source_node_visit_id NULL; exactly one CANCELLED event; version +1.
ACC-M1B-002 (T2/T3/T4): dangling + open DISPATCH_INTENT / open
  HUMAN_WORK_ITEM (the canonical model's complete activation-kind set) ->
  refuse; instance/runtime facts/version/event counts unchanged.
ACC-M1B-003 (T5): normal current-visit cancel unregressed (existing suites
  plus explicit binding assertion).
ACC-M1B-004 (T6/T7): same-key replay exactly once; same-key different
  request conflicts.
ACC-M1B-005 (T8): injected fault mid-transaction -> complete rollback, no
  partial state.
```

## 4. Relation to predecessor evidence

PR #45 (r1 33f03ce + r2 161bc9ee) demonstrated the same shape on base
`bd47668` with a pending authority gate. Its approach is transplanted as
minimal delta onto fresh `main` @ `07d8882`; its draft branch is superseded
as implementation evidence and should be closed when this candidate lands.
No historical bytes are carried: the fail-closed check, the guard shape, and
all tests are re-authored against the current canonical activation model
and current test helpers.
