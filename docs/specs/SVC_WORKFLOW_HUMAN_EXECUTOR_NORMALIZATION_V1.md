---
spec_id: SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
scope: [mayf3/svc-workflow, exact-18-human-executor-normalization]
governed_by: [SVC_WORKFLOW_PRODUCT_BOUNDARY_V8, SVC_WORKFLOW_ARCHITECTURE_V0_4_1]
related_authorities: [SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V0, IDENTITY_PROVISIONING_API_V0]
supersedes: [SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V0]
superseded_by: null
owners: [mayf3]
accepted_by: null
accepted_date: null
accepted_reviewed_spec_commit: null
acceptance_review_verdict: null
title: Human Executor Normalization V1 exact-18 successor
repo: mayf3/svc-workflow
base_head: b1c9a02fbffc7386d428863d0a91bcea98499f64
production_apply_authorized_now: false
merge_required_for_activation: true
---

# SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V1

## 1. Goal

This whole-authority successor preserves the accepted V0 normalization model
and narrows its stale exact-20 target to the exact 18 Workflows that still have
the accepted active Agent-owned preimage. It excludes two Workflows that
advanced after V0's snapshot and MUST NOT rewrite, reopen, or otherwise touch
them.

```text
PLAN_PATH = docs/evidence/human-executor-normalization-v1/exact-18-plan.tsv
PLAN_SHA256 = bba710b9790fed4c0136b9a0f33186f87f11be9e5da3f76e08784bfbce8dd871
TARGET_COUNT = 18
TARGET_AUTH_USER_ID = 8902db0d-429a-4e37-985c-f8b92d4b78fb
TARGET_WORKFLOW_PRINCIPAL_ID = 8902db0d-429a-4e37-985c-f8b92d4b78fb
TARGET_PRINCIPAL_TYPE = HUMAN
EXCLUDED_WORKFLOW_COUNT = 2
```

V0 remains active until an independently reviewed, Owner-accepted lifecycle
transaction merges V1 as `accepted` and marks V0 `superseded` with the
reciprocal backlink. This proposed candidate grants no implementation or
production apply authority.

## 2. Scope and non-goals

Scope is only the exact-plan identity/cardinality successor and the bounded
update of the already merged goal-specific operator/tests. The executor model,
Human UUID, append-only mechanism, transaction/replay rules, and HR consequence
are unchanged from V0.

Out of scope: repair/reopen of the two excluded terminal Workflows; any other
Workflow or Principal; title/content inference; generic migration; HR filters;
schema/API/Definition changes; deployment behavior; and production apply.

## 3. Authority and dependencies

Product Boundary V8 supplies the mechanical current-executor relation.
Architecture v0.4.1 supplies immutable Visit identity and append-only
administrative successor structure. V0 is the complete predecessor authority;
V1 replaces it as a whole because changing the frozen target set narrows
accepted normative meaning and cannot be an in-place amendment. The existing
Human projection authority remains unchanged and has already produced the
enabled exact Workflow Principal observed below.

## 4. Current State

- `STATE-HEN1-001` — At source base
  `b1c9a02fbffc7386d428863d0a91bcea98499f64` and production observation time
  2026-09-15 Asia/Shanghai, V0 is accepted and implemented but its production
  apply is blocked by two post-snapshot transitions. The remaining 18 are
  active, Agent-owned, and exactly represented by the V1 plan; the target Human
  is enabled. Basis: `OBS-HEN1-001`..`OBS-HEN1-004`, `EVD-HEN1-001`..`002`.

## 5. Observations

### OBS-HEN1-001 — V0 exact-plan preflight

- Subject: V0's frozen 20-row plan against production
  `svc_workflow_dogfood_clean`.
- Source/artifact coordinate: merged implementation
  `b1c9a02fbffc7386d428863d0a91bcea98499f64`, plan SHA
  `b349e203c00ac82e286666a89dbedd6a17f77e0221090a1a9f2db51d8a253199`.
- Observed at: 2026-09-15 Asia/Shanghai.
- Method: release-build V0 operator `--plan`; followed by PostgreSQL
  `BEGIN READ ONLY` exact-ID inspection.
- Result: the operator returned `CONFLICT` with `writes=0`; 18 rows retain the
  exact V0 preimage, zero V0 normalization artifacts exist, and two rows no
  longer match.
- Provenance:
  `docs/evidence/human-executor-normalization-v1/production-preflight-20260915.md`.

### OBS-HEN1-002 — Two post-snapshot transitions

- Subject: Workflows `2edf5b53-1dd9-4c93-b356-4029d3fe1adb` and
  `f0ebdef1-8cab-4b97-82ac-af92b8ed3e12`.
- Environment: same production database; read-only inspection on 2026-09-15.
- Result: both moved from state version 1 to 2 through persisted
  `WORKFLOW_TRANSITION_COMMITTED` / `ADVANCE` events on 2026-09-14, using
  transition key `advance-to-completed`; their current node is `completed` of
  type `TERMINAL` with no current assignee.
- Limitation: persisted completion does not prove the real-world action was
  performed.
- Provenance:
  `docs/evidence/human-executor-normalization-v1/production-preflight-20260915.md`.

### OBS-HEN1-003 — Owner business correction

- Subject: the two post-snapshot Workflows.
- Source: explicit Owner statement in the 2026-09-15 Lane G execution thread.
- Result: neither real-world item is complete; Owner directs this Goal to
  exclude both rows and continue Human normalization only for the other 18.
- Limitation: repair/reopening of the two terminal Workflows requires a
  separate authority and is not part of V1.
- Provenance: Owner decision in the Lane G thread; the resulting bounded
  exclusion and machine preimage are persisted in the V1 plan and preflight
  record.

### OBS-HEN1-004 — Human projection and remaining preimage

- Environment: production read-only inspection on 2026-09-15.
- Result: Principal `8902db0d-429a-4e37-985c-f8b92d4b78fb` exists as enabled
  `HUMAN`; each row in the V1 plan still matches its exact current Visit,
  version, Agent assignee, DefinitionVersion, node, Context digest, and unused
  target Visit UUID.
- Provenance:
  `docs/evidence/human-executor-normalization-v1/production-preflight-20260915.md`.

## 6. Claims and assumptions

### CLM-HEN1-001 — Exact-18 replacement is required

- Support state: SUPPORTED.
- Basis: OBS-HEN1-001..003.
- Claim: V0 cannot lawfully apply because its all-or-zero target is stale;
  excluding the two transitioned rows and freezing the remaining 18 is the
  smallest correction that preserves current business history.

### CLM-HEN1-002 — The exact Human and remaining preimages are ready

- Support state: SUPPORTED.
- Basis: OBS-HEN1-004 and EVD-HEN1-002.
- Claim: the already projected same-UUID Auth User/Workflow Human and the exact
  18 mechanical preimages satisfy the identity and target prerequisites for a
  later apply-time revalidation; they do not themselves authorize writes.

No normative open assumption remains.

## 7. Evidence relations

### EVD-HEN1-001

- Source observations: OBS-HEN1-001, OBS-HEN1-002.
- Target type and IDs: Claim CLM-HEN1-001; State STATE-HEN1-001.
- Relation: SUPPORTS.
- Bound coordinates: production `svc_workflow_dogfood_clean`, observed
  2026-09-15; source main `b1c9a02fbffc7386d428863d0a91bcea98499f64`;
  V0 and V1 plan hashes stated in section 1.
- Strength/sufficiency: strong for exact stale rows, persisted transitions,
  zero normalization artifacts, and required whole-scope replacement.
- Limitations: read-only observation proves neither implementation conformance
  nor future apply-time state and grants no writes.
- Provenance:
  `docs/evidence/human-executor-normalization-v1/production-preflight-20260915.md`.

### EVD-HEN1-002

- Source observations: OBS-HEN1-003, OBS-HEN1-004.
- Target type and IDs: Claim CLM-HEN1-002; State STATE-HEN1-001.
- Relation: SUPPORTS.
- Bound coordinates: exact V1 plan SHA, target Human UUID, production database,
  and 2026-09-15 observation recorded above.
- Strength/sufficiency: strong for the selected exact exclusion, same-UUID
  enabled Workflow Human, 18/18 preimage match, and zero target collisions.
- Limitations: Owner statement establishes business intent but not repository
  acceptance; all facts remain subject to apply-time revalidation.
- Provenance: V1 exact plan, the production preflight record, and the Owner
  decision recorded in this execution thread.

## 8. Decisions

### DEC-HEN1-001 — Supersede exact 20 with exact 18

- Decision owner: repository Owner `mayf3` through the 2026-09-15 Lane G
  ruling.
- Selected: exclude exactly the two IDs in OBS-HEN1-002 and normalize only the
  exact 18 rows in the V1 plan.
- Rejected: applying stale V0; dynamic subset flags; title/content selection;
  silently reopening terminal rows; treating persisted completion as real-world
  proof.
- Reason: current facts must be preserved while the still-valid Human backlog
  is corrected.
- Remaining Owner input: none for candidate semantics.

### DEC-HEN1-002 — Preserve V0 correction mechanism

- Decision owner: repository Owner `mayf3`.
- Selected: retain V0's exact-plan-bound, append-only same-node successor Visit,
  SERIALIZABLE all-or-zero transaction, deterministic receipt/Event/audit,
  fail-closed prevalidation, and exact replay model, with only plan identity and
  cardinality changed to 18.
- Rejected: same-Visit mutation, business transition, new executor field,
  generic migration framework, HR filter, activation/Dispatch Intent creation,
  or per-row best effort.
- Remaining Owner input: none.

## 9. Contracts

### CTR-HEN1-001 — Exact finite scope

The operator MUST process only the 18 rows and target Visit UUIDs in the V1
plan. It MUST embed and verify the exact plan bytes and SHA. It MUST accept no
plan path, Workflow/owner UUID, row selector, subset, or content-derived input.
The two excluded Workflow IDs MUST be absent from its plan and writes.

### CTR-HEN1-002 — Canonical Human and operator gates

Before any Workflow write, the target MUST be exact Principal
`8902db0d-429a-4e37-985c-f8b92d4b78fb`, mechanically bound to the exact active
Auth User of the same UUID and projected as an enabled Workflow `HUMAN`
Principal. The supplied operator actor MUST exist, be enabled, and type
`AGENT`; every target Instance MUST retain Legacy `semantic_model_version=1`;
and the actual database name MUST equal the separately supplied execution
coordinate. Missing, disabled, conflicting, differently bound, or mismatched
state causes zero writes.

### CTR-HEN1-003 — Append-only executor correction

For each exact row, append the preassigned same-Instance, same-node Human Visit
with `visit_number = source + 1` and `entered_by_transition_id = NULL`. CAS
update only current Visit pointer, state version `expected + 1`, and timestamp.
Never update or delete a source Visit.

### CTR-HEN1-004 — No fabricated business action

Preserve Instance identity, DefinitionVersion, node, Context, creator, external
reference, metadata, artifact references, lifecycle, cancellation/archive
state, semantic model, and business payload. Create no Submission,
transition/effect, completion, activation, Dispatch Intent, or human evidence.
Modify no historical row and no excluded Workflow.

### CTR-HEN1-005 — Atomic fail-closed group

One SERIALIZABLE advisory-locked transaction MUST cover all 18 Visits, 18 CAS
updates, 18 Events, 18 completed Receipts, and one group audit. Lock exact
Instances in deterministic order and completely prevalidate every row plus all
target Visit, receipt command/idempotency, Event ID/command/sequence, and audit
collisions before the first write. Drift, collision, assistance, or mismatch
aborts with zero operator writes.

### CTR-HEN1-006 — Exact replay and unknown outcome

Each row MUST use deterministic command, Event, and idempotency identities that
bind Spec, plan, implementation SHA, database, actor, Human, and row
coordinates. Exactly one Event MUST link to each completed Receipt. Exact rerun
MUST verify all 18 linked pairs, one audit, and full poststate before zero-write
`NOOP`. Partial/asymmetric/corrupt state conflicts. Commit outcome uncertainty
MUST reconcile by fresh readback and MUST NOT blindly retry.

### CTR-HEN1-007 — Mechanical HR consequence

After apply, the exact 18 MUST remain lifecycle `ACTIVE`, be mechanically
`current executor type = HUMAN`, and be absent from `ACTIVE + AGENT` solely by
the current Visit assignee Principal type. No HR title, test, identity, Domain,
readiness, or special-case filter is authorized.

### CTR-HEN1-008 — Separation and production boundary

V1 authoring/acceptance, implementation, merge, and production apply remain
separate gates. This Spec does not itself authorize production apply. Repair of
the two excluded terminal Workflows is separate follow-up debt and MUST NOT be
performed by the normalization operator.

## 10. Acceptance

### ACC-HEN1-001 — Plan and target closure

- Contracts: CTR-HEN1-001, CTR-HEN1-002.
- Method: exact plan digest/cardinality/exclusion checks plus target/operator
  type matrix in disposable PostgreSQL.
- Expected: 18 unique rows; both excluded IDs absent; invalid identity or DB
  coordinate produces zero writes.

### ACC-HEN1-002 — Append-only preservation

- Contracts: CTR-HEN1-003, CTR-HEN1-004.
- Method: full before/after projections including a lawful non-null source
  `entered_by_transition_id`, unrelated row, and both excluded Workflow
  fixtures.
- Expected: only 18 appended Human Visits and exact current projections change;
  business/source/excluded facts are unchanged.

### ACC-HEN1-003 — Atomicity and replay

- Contracts: CTR-HEN1-005, CTR-HEN1-006.
- Method: success, one-row drift, Visit/command/Event/audit collision, open
  assistance, injected mid-group failure, rerun, corrupted poststate, and
  commit-acknowledgement-loss reconciliation.
- Expected: counts 18 Visits/Events/Receipts and one audit on success; all
  rejected cases have zero operator writes; replay is exact `NOOP`.

### ACC-HEN1-004 — Executor result

- Contracts: CTR-HEN1-007, CTR-HEN1-008.
- Method: exact-ID current Visit/Principal query and excluded/unrelated diffs.
- Expected: exact 18 `ACTIVE + HUMAN`, zero exact 18 `ACTIVE + AGENT`, excluded
  rows unchanged, and no HR classifier.

| Contract | Acceptance |
|---|---|
| CTR-HEN1-001 | ACC-HEN1-001 |
| CTR-HEN1-002 | ACC-HEN1-001 |
| CTR-HEN1-003 | ACC-HEN1-002 |
| CTR-HEN1-004 | ACC-HEN1-002 |
| CTR-HEN1-005 | ACC-HEN1-003 |
| CTR-HEN1-006 | ACC-HEN1-003 |
| CTR-HEN1-007 | ACC-HEN1-004 |
| CTR-HEN1-008 | ACC-HEN1-004 |

## 11. Alternatives and disposition

- `ALT-HEN1-001` apply stale exact-20 authority: rejected because two rows no
  longer match and committed business history cannot be overwritten.
- `ALT-HEN1-002` runtime subset/exclusion flags: rejected because the operator
  must remain closed to one reviewed finite plan.
- `ALT-HEN1-003` reopen the two terminal Workflows here: rejected as a separate
  lifecycle/business repair requiring its own evidence and authority.
- `ALT-HEN1-004` infer remaining Human work from titles: rejected; membership
  comes only from exact UUIDs and fresh mechanical preimages.

## 12. Migration, compatibility, and rollback

Implementation is limited to updating the existing goal-specific offline Rust
binary and its existing disposable PostgreSQL test module. No API, schema,
field, generic migration framework, deployment behavior, HR code, credential,
Grant, Definition, or runtime selection mechanism may change.

Pre-commit failure rolls back the entire group. Post-commit uncertainty is
contained and reconciled through exact readback; committed facts are never
deleted or reversed without new authority. The two excluded Workflows remain
untouched even though the Owner reports their real-world work incomplete.

## 13. Open questions and author output

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE AFTER WHOLE SUPERSESSION
PARTIAL_SUPERSESSION = NONE
```

```text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V1
SPEC_KIND = implementation
STATUS = proposed
AUTHORITY_LEVEL = governing_spec
IMPLEMENTATION_AUTHORITY = contracts AFTER ACCEPTANCE AND MERGE
PRODUCTION_APPLY_AUTHORITY = none
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V8
EXTERNAL_AUTHORITIES = NONE
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
PARTIAL_SUPERSESSION = NONE
CONTRACT_COUNT = 8
CONTRACTS_WITH_ACCEPTANCE = 8
AUTHORING_READY_FOR_REVIEW = YES
```
