---
spec_id: SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1
title: Canonical Work Eligibility Read Projection V1
status: accepted
spec_kind: implementation
authority_level: governing_spec
accepted_date: 2026-09-06
date: 2026-09-05
type: implementation-spec (read-only projection delta on existing surfaces)
repo: mayf3/svc-workflow
base_head: c4f1fa8d9bae7c91d9cc09751cfa8e2195c3911a (github/main == production live)
accepted_reviewed_head: 78490a99178c86d648a5aa333f25ddd41dc37888
final_implementation_head: 7c270c768ea37a5f8f20b2a2c56ede93737e79e3
implementation_authority: contracts
production_apply_authority: none
external_authorities:
  - repository: mayf3/svc-workflow
    authority_id: SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1 (accepted, v0.4.0 §5.7-5.9 —
      activation/closure/eligibility-event facts this projection reads)
  - repository: mayf3/svc-workflow
    authority_id: SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1 (accepted — the READER
      authorization under which the global list surface is consumed)
supersedes: []
---

# SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1 — Canonical Work Eligibility Read Projection

> **ACCEPTED (2026-09-06, Owner exact-head acceptance).** Owner decision
> ACCEPT EXACT HEAD = YES at `78490a99178c86d648a5aa333f25ddd41dc37888`
> (remote exact-head stability verified); FINAL_IMPLEMENTATION_HEAD =
> `7c270c768ea37a5f8f20b2a2c56ede93737e79e3`. Review lifecycle: ROUND_1 FAIL
> (BLOCKER B-1 serde WAITING_FOR_TIME wire shape) → FIX_ONCE `7c270c7` →
> RE_AUDIT PASS → FINAL_BLOCKER_UNION = [] (record:
> `docs/audits/WORK_ELIGIBILITY_PROJECTION_REVIEW_RECORD_V1.md`). Frozen wire
> contract: `{"classification":"ACTIONABLE_NOW"}` /
> `{"classification":"WAITING_FOR_TIME","nextEligibleAt":"<RFC3339>"}`.
> `implementation_authority: none -> contracts`;
> `production_apply_authority` stays **none** — production deployment of this
> projection proceeds only under the Owner-dispatched production Goal's slot
> discipline (REAL_AUTONOMOUS_WORKFLOW_LOOP_V1). No other semantic head may be
> substituted without a new Owner acceptance.

## 1. Goal

Answer, server-side and from ONE canonical derivation, the dispatch-eligibility
question for every Workflow read surface that exposes current work:

> Is this item ACTIONABLE_NOW, or WAITING_FOR_TIME (its Visit Activation timer
> has not yet fired)?

Business driver (REAL_AUTONOMOUS_WORKFLOW_LOOP_V1): the HR dispatcher must
select only ACTIONABLE_NOW work. Without this projection the read surfaces
cannot distinguish "waiting for its timer" from "ready to send to an Agent",
while `workflow_execute.transition` (by fresh grep, intentionally unchanged
here) does not enforce eligibility — so the classification is load-bearing for
dispatch decisions, not presentation.

## 2. Frozen semantic (the ONE derivation)

Derived per instance from its CURRENT node visit and existing Visit Activation
tables only (`workflow_activations`, `workflow_activation_closures`,
`workflow_dispatch_eligibility_events`):

| Case | Classification |
|---|---|
| No `workflow_activations` row for the current visit (pre-0023 legacy work) | `ACTIONABLE_NOW` |
| Activation row exists but is closed (row in `workflow_activation_closures`) | `ACTIONABLE_NOW` |
| Open activation, kind `HUMAN_WORK_ITEM` (timerless by schema CHECK) | `ACTIONABLE_NOW` |
| Open activation, kind `DISPATCH_INTENT`, effective instant ≤ now | `ACTIONABLE_NOW` |
| Open activation, kind `DISPATCH_INTENT`, effective instant > now | `WAITING_FOR_TIME` |

- **effective instant** = latest `workflow_dispatch_eligibility_events.new_next_eligible_at`
  by `(created_at, eligibility_event_id)` DESC for the open activation, else
  `workflow_activations.initial_next_eligible_at` (the eligibility-event table
  is the only later writer, per VISIT_ACTIVATION_IMPL_V1).
- `now` = authoritative server clock captured at query execution; never client
  input.
- **NO_ACTIVATION_ROW compatibility rule (frozen)**: legacy/current work without
  an activation record stays discoverable and is classified explicitly
  `ACTIONABLE_NOW`. The existing production work universe (200 items across 14
  domains, measured 2026-09-05) must not disappear behind Visit Activation.
- **NO BLOCKED state**: no canonical Workflow fact distinguishes a distinct
  blocked condition; inventing one is forbidden by the product direction.
- When a WAITING_FOR_TIME activation becomes due, the SAME Workflow work
  becomes ACTIONABLE_NOW — no copy, no backfill, no synthetic task; dispatch
  intents remain the TIME/ELIGIBILITY AUXILIARY ONLY, never a task ledger.
- Classification is a **read projection only**. Transition semantics are NOT
  modified (`workflow_execute.transition` continues not to enforce
  eligibility): `TRANSITION_ELIGIBILITY_ENFORCEMENT = FOLLOW_UP_DEBT`.

## 3. Scope (exact closure)

- `src/application/workflow_instance/eligibility.rs` (NEW): `WorkEligibility`
  enum (`ACTIONABLE_NOW` | `WAITING_FOR_TIME`, serde SCREAMING_SNAKE_CASE with
  `nextEligibleAt` content for the waiting case), `EligibilityFactRow`
  (`sqlx::FromRow`) + `classify(now)`, shared SQL fragment constants.
- `src/application/workflow_instance/mod.rs`: register the module.
- `src/application/workflow_instance/query_types.rs`: add `eligibility` field
  to `DomainInstanceSummary` (shared by domain + global lists) and
  `NodeVisitItem`.
- `src/store/postgres/workflow_instance_repository/query_domain_instances.rs`
  and `query_global_instances.rs`: SELECT gains the three activation-fact
  columns + shared LEFT JOIN/LATERAL; Row structs gain the fields; `From` maps
  classification.
- `src/store/postgres/workflow_instance_repository/query_visibility.rs`
  (`load_base`) + `query_rows.rs` (`QueryBaseRow` fields +
  `current_visit_eligibility()`; both `NodeVisitItem` construction sites).
- NO migration. NO new table. NO new endpoint. NO change to any write path,
  authorization predicate, or the transition engine. NO broker change (the
  field flows through existing passthrough manifests `workflow_domain_instances`
  / `workflow_global_instances` / `workflow_my_tasks` / `workflow_instance_detail`
  untouched).

## 4. Non-goals

- No BLOCKED state; no additional lifecycle states.
- No transition-side eligibility enforcement (FOLLOW_UP_DEBT unless existing
  accepted Authority already requires it — fresh grep shows none does).
- No backfill/synthetic activation rows for legacy work.
- No client-side reconstruction: the server projects the canonical fact
  directly; consumers must not re-derive eligibility from loosely-related calls.
- No change to GLOBAL_WORKFLOW_READER / DOMAIN_OWNER authorization semantics.

## 5. Eligibility-of-the-projection (honest boundaries)

- The `VisitRow::into_item` timeline path (instance visit history) stamps
  `ACTIONABLE_NOW` unconditionally: only the CURRENT visit can carry open
  dispatchable work; any prior visit was necessarily left via a transition, so
  its activation (if any) is closed. Dispatch-decision eligibility is projected
  on current-work surfaces (summaries, detail, worklists), never on the
  historical timeline.
- `classify` fails open (open DISPATCH_INTENT with NULL effective instant →
  ACTIONABLE_NOW): schema CHECK makes NULL impossible for an open
  DISPATCH_INTENT; a NULL can only result from a data anomaly, and an
  actionable classification errs toward visibility, never toward silently
  hiding work.
- Classifying at `chrono::Utc::now()` (row mapping time) vs. transaction
  snapshot time can straddle a due instant by microseconds; acceptable for a
  read projection (the boundary case re-resolves on the next poll).

## 6. Acceptance

- ACC-001: classifier unit tests cover all five table rows plus the exact-due
  boundary and the fail-open case (7 tests, included in the implementation).
- ACC-002: full test suite green except the pre-existing env-dependent
  `upgrade_0012_to_0014` failure, A/B-verified identical on pristine main
  `c4f1fa8` (my diff touches no migration).
- ACC-003: production dry-run of the classifier logic against live data
  (2026-09-05): 200/200 dispatchable items classify ACTIONABLE_NOW
  (legacy-no-activation) — the legacy universe remains visible.
- ACC-004: post-deploy production proof (deferred to the production phase):
  a WAITING_FOR_TIME model-3 instance appears as WAITING_FOR_TIME in
  `workflow_global_instances` / `workflow_domain_instances` /
  `workflow_instance_detail`, and flips to ACTIONABLE_NOW at due time without
  any new row.
