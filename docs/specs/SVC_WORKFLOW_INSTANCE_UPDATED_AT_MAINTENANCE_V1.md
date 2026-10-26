---
spec_id: SVC_WORKFLOW_INSTANCE_UPDATED_AT_MAINTENANCE_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
scope:
  - mayf3/svc-workflow
  - workflow_instances.updated_at write maintenance only
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V8
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_3
supersedes: []
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_INSTANCE_UPDATED_AT_MAINTENANCE_V1 (PROPOSED — NOT ACCEPTED)

> Status: proposed. This Spec has no acceptance record and grants no implementation
> authority until independently reviewed and accepted by an authorized acceptance
> actor. While `status: proposed`, implementation on the `main` authority branch is
> forbidden under it.

## 1. Goal

```text
GOAL = workflow_instances.updated_at records the database transaction-start
       timestamp assigned by the most recent committed transaction that changes
       the authoritative instance projection
SUCCESS_OUTCOME = each committed instance state change assigns that transaction's
                  database timestamp; no failed or side-effect-free request
                  changes the persisted timestamp
```

Provenance of the convention (historical, cited as consistency anchors, not as
active authority): superseded SVC_WORKFLOW_PRODUCT_BOUNDARY_V2 §(l.208) / V3
(l.281, l.831) / V4 (l.346, l.1133) / V5 (l.451) defined instance-projection
`updatedAt` as "the latest committed Workflow Instance state change … not read,
cache, scheduler, audit, Assistance, or external-system time", with "uses another
clock" as a failure condition. Accepted scope-bounded precedents already write it
this way: SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1 step 5 and
ADMIN_RECOVERY_CONTRACT_V0_1 rebuild rule. Static inspection found timestamp
assignments in six of ten path families on main; this is an implementation
observation, not verified conformance to all Contracts below (see
docs/reports/UPDATED_AT_AUTHORITY_LOCALIZATION_V1.md §3).

Decision D-UAM-001: preserve PostgreSQL `now()` / `transaction_timestamp()`
semantics. This is **not** commit wall-clock time, a duration, a unique event
identifier, or a strictly increasing sequence. Two changes in one transaction
may have equal timestamps; transaction start order need not equal commit order.
Use the existing `workflow_state_version` for the ordering semantics it already
owns. A commit-time or globally monotonic clock would require a separate decision.
See [PostgreSQL 16 §9.9.5](https://www.postgresql.org/docs/16/functions-datetime.html#FUNCTIONS-DATETIME-CURRENT).

## 2. Scope and non-goals

In scope: the `workflow_instances.updated_at` column of the authoritative
instance projection row, and only it.

Out of scope (different objects or dimensions, unaffected):

- `updatedAt` of the Scheduler-facing Dispatch Intent record
  (SVC_WORKFLOW_PRODUCT_BOUNDARY_V8, Dispatch Intent surface);
- activation-record `updatedAt` (SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1),
  execution-control / forum-binding tables (SVC_WORKFLOW_EXECUTION_CONTROL_V1),
  `domains`, definition/version tables, outbox, provisioning tables;
- read-time fields of any projection; event/audit timestamps; timeline semantics;
- historical data backfill or recomputation of global summaries;
- the identity/admission semantics of revise (owned by
  SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2, unchanged).

## 3. Definitions

```text
INSTANCE_STATE_CHANGE = a committed change of the authoritative instance
                        projection: current_node_visit_id,
                        current_context_revision_id, workflow_state_version,
                        or lifecycle fields (cancelled*/archived*)
```

## 4. Contracts

### CTR-UAM-001 — Transactional maintenance rule

Every INSTANCE_STATE_CHANGE MUST set `workflow_instances.updated_at = now()` in
the same database transaction (same statement or same transaction) as the
projection change. One database clock only; no application-supplied timestamp,
no deferred or post-commit write, no second Scheduler/wait semantics attached to
this column.

### CTR-UAM-002 — Path enumeration (normative minimum)

The rule applies at least to: ADVANCE, RETURN and TERMINATE (transition
projection update), combined revise+transition, revise-only context revise,
context repair, cancel, archive, admin recovery rebuild/override, assistance
activation, execution escalation, and wake/push-kick state bumps. New instance
write paths inherit the rule. Context repair is included in this proposed
Contract. An implementation covering only transition/combined/revise is partial
conformance; sequencing repair later cannot remove that remaining obligation or
produce a full-Spec PASS.

### CTR-UAM-003 — No-bump events

A request that produces zero committed instance delta MUST NOT change
`updated_at`: failed or aborted commands, optimistic
`workflow_state_version` conflicts, read/query paths, and idempotent replays
served from an existing receipt without a new write.

### CTR-UAM-004 — created_at preservation

`created_at` is written only by the creating INSERT; `updated_at` equals
`created_at` immediately after creation. No later statement may write
`created_at`.

### CTR-UAM-005 — Historical data and migration

No schema migration and no backfill. Installing this change MUST NOT rewrite
existing rows solely to repair historical timestamps. A later legitimate
INSTANCE_STATE_CHANGE on an existing row follows CTR-UAM-001; this is not a
backfill. Untouched historical rows remain unchanged. Any consumer-side
correction of historical summaries is a separate decision outside this Spec.

### CTR-UAM-006 — Compatibility and rollback

No wire, SDK, OpenAPI, or schema change; the column already exists since the
initial schema. Source rollback is a git revert of the implementation commit;
this does not install or load an old artifact. Runtime adoption and rollback
require their separately authorized deployment profile and verified artifact
binding. Reverting source or deploying an old binary MUST NOT rewrite stored
timestamps or other business data. This Spec grants no production operation
authority and does not claim the old binary satisfies these Contracts.

### CTR-UAM-007 — Verification

Implementation under this Spec MUST bind each applicable Contract to executed
evidence at the exact candidate head, isolated database identity, command,
result and limitations. Use explicit `TEST_DATABASE_URL` for a dedicated
throwaway database; never connect production `svc_workflow` or other tenant
data. A functional suite without timestamp assertions cannot prove timestamp
maintenance. The matrix below defines minimum evidence; it is not a claim that
these tests exist or have run.

| Contract | Required evidence and negative control |
|---|---|
| CTR-UAM-001 | For each changed path, a committed projection change and the assigned DB transaction timestamp; use separately started transactions with controlled separation for `after > before` checks. That fixture assertion is not a general monotonicity guarantee. Roll back a transaction after its candidate write and verify persisted projection and timestamp are unchanged. |
| CTR-UAM-002 | Explicit cases for ADVANCE, RETURN, TERMINATE, combined, revise, repair; audit and test existing cancel, archive, admin rebuild/override, assistance, escalation and wake families for a full-Spec claim. Shared helpers can justify code coverage but do not silently substitute for named behavior cases. Missing cases remain NOT_VERIFIED. |
| CTR-UAM-003 | Before/after projection and timestamp for rejected version conflict, invalid/failed command, aborted transaction, reads and an actual idempotent replay. Exercise the replay-capable surfaces affected by the change; distinguish a replay receipt from a new successful write. |
| CTR-UAM-004 | Creation records equal timestamps; subsequent success and negative-control cases retain original `created_at`. |
| CTR-UAM-005 | Diff contains no schema/backfill changes; an untouched seeded historical row remains byte-equivalent through the test run while a separate row changes legitimately. |
| CTR-UAM-006 | Review public/schema diff and source revert scope. Any claimed runtime rollback additionally identifies the authorized profile, exact old/new artifacts and before/after database readback; source revert alone cannot satisfy that claim. |
| CTR-UAM-007 | A conformance record joins Contract IDs, exact Spec revision, candidate HEAD, environment and actual results; cross-head historical runs and unexecuted plans cannot be reported as current PASS. |

Acceptance of this proposed normative contract is distinct from implementation
conformance. Currently every new main-bound test in this matrix is NOT_RUN.
Partial implementations may record passed named cases while retaining all
unverified obligations; they MUST NOT claim complete conformance.

## 5. External owner items (explicit; not decidable by the writer)

1. Acceptance of this Spec by an authorized acceptance actor (mayf3 or an
   explicitly authorized maintainer), with independent review, per
   `.agents/local/README.md`.
2. Sequencing between this Spec's future implementation on `main` and the
   in-flight candidate line (PR #85 head e068dd06… on base
   fix/history-listener-error-resilience c819124c…, ahead 9 / behind 14 vs
   main): which merges first, and whether the main-bound minimal fix
   (docs/reports/UPDATED_AT_MAIN_INTEGRATION_PREP_V1.md) supersedes or follows
   #85. This Spec neither retro-authorizes nor alters #85, whose stated basis
   (CTR-CIR-004/005 revise operability on that line) remains as recorded.
3. This proposal recommends fixing context repair with the three 2c-parity
   paths on the main-bound line. An authorized sequencing decision may split
   delivery, but repair remains required by CTR-UAM-002 and full conformance
   remains open until its evidence exists. Excluding repair from the Contract
   requires changing and reviewing this proposal before acceptance.
4. Any consumer that relied on stale `updated_at` values in global summaries
   (informational; no platform action proposed here).
