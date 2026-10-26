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
GOAL = workflow_instances.updated_at reliably means the time of the instance's
       latest committed state change, maintained transactionally at every write
       that changes the authoritative instance projection
SUCCESS_OUTCOME = no committed instance state change leaves updated_at stale;
                  no failed or side-effect-free request changes updated_at
```

Provenance of the convention (historical, cited as consistency anchors, not as
active authority): superseded SVC_WORKFLOW_PRODUCT_BOUNDARY_V2 §(l.208) / V3
(l.281, l.831) / V4 (l.346, l.1133) / V5 (l.451) defined instance-projection
`updatedAt` as "the latest committed Workflow Instance state change … not read,
cache, scheduler, audit, Assistance, or external-system time", with "uses another
clock" as a failure condition. Accepted scope-bounded precedents already write it
this way: SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1 step 5 and
ADMIN_RECOVERY_CONTRACT_V0_1 rebuild rule; six of ten instance UPDATE sites on
main already conform (see docs/reports/UPDATED_AT_AUTHORITY_LOCALIZATION_V1.md §3).

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
write paths inherit the rule.

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

No schema migration and no backfill. Existing rows with
`updated_at = created_at` remain historical observations and MUST NOT be
rewritten by this Spec's implementation. Any consumer-side correction of
historical summaries is a separate decision outside this Spec.

### CTR-UAM-006 — Compatibility and rollback

No wire, SDK, OpenAPI, or schema change; the column already exists since the
initial schema. Rollback of an implementation under this Spec is a git revert of
that implementation commit; no data rollback path exists.

### CTR-UAM-007 — Verification

Implementation under this Spec MUST be verified per path family (transition,
combined, revise, and any other path it changes) with integration tests on an
isolated database selected by explicit `TEST_DATABASE_URL` (dedicated throwaway
database; the production `svc_workflow` database and other tenant data are never
connected). Tests MUST assert both the bump on success and the no-bump on
version conflict. PASS claims require executed runs at the exact candidate head;
unexecuted coverage stays unknown.

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
3. Whether the context-repair path (4th non-conforming site, untouched by
   2c135ef and by PR #85) is fixed together with the three 2c-parity sites or in
   a separate step.
4. Any consumer that relied on stale `updated_at` values in global summaries
   (informational; no platform action proposed here).
