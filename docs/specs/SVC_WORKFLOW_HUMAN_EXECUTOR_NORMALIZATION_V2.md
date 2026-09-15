---
spec_id: SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V2
status: accepted
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
scope: [mayf3/svc-workflow, exact-17-human-executor-normalization]
governed_by: [SVC_WORKFLOW_PRODUCT_BOUNDARY_V8, SVC_WORKFLOW_ARCHITECTURE_V0_4_1]
related_authorities: [SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V1, IDENTITY_PROVISIONING_API_V0]
supersedes: [SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V1]
superseded_by: null
owners: [mayf3]
accepted_by: mayf3
accepted_date: 2026-09-15
accepted_reviewed_spec_commit: e6e0af5cbadfe46dbf9b18d16ba12819a0d26880
accepted_reviewed_spec_sha256: 7f1e6178841c4eb2d6e9895abe25752b0367d479971f46b0bc498420a7d2f61b
accepted_plan_sha256: 57146935b5aef4a6d737cc3d709d1f8967052616bd730ec10dea367b2f94d0c5
acceptance_review_verdict: PASS
acceptance_record: docs/reports/HUMAN_EXECUTOR_NORMALIZATION_V2_ACCEPTANCE.md
title: Human Executor Normalization V2 exact-17 successor
repo: mayf3/svc-workflow
base_head: 7c3beec0ee058aa896b86e58443c502a48d23d11
production_apply_authorized_now: false
merge_required_for_activation: true
---

# SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V2

## 1. Goal

This whole-authority successor preserves the accepted V1 normalization model
and narrows its stale exact-18 target to the exact 17 Workflows that still have
the accepted active Agent-owned preimage. It excludes the additional Workflow
that advanced after V1's snapshot and MUST NOT rewrite, reopen, or otherwise
touch any of the three terminal exclusions.

```text
PLAN_PATH = docs/evidence/human-executor-normalization-v2/exact-17-plan.tsv
PLAN_SHA256 = 57146935b5aef4a6d737cc3d709d1f8967052616bd730ec10dea367b2f94d0c5
TARGET_COUNT = 17
TARGET_AUTH_USER_ID = 8902db0d-429a-4e37-985c-f8b92d4b78fb
TARGET_WORKFLOW_PRINCIPAL_ID = 8902db0d-429a-4e37-985c-f8b92d4b78fb
TARGET_PRINCIPAL_TYPE = HUMAN
EXCLUDED_WORKFLOW_COUNT = 3
```

V1 remains active until an independently reviewed, Owner-accepted lifecycle
transaction merges V2 as `accepted` and marks V1 `superseded` with the
reciprocal backlink. This accepted successor activates implementation authority
only after this atomic lifecycle transaction is merged to `main`; it grants no
production apply authority.

## 2. Scope and non-goals

Scope is only the exact-plan identity/cardinality successor and the bounded
update of the already merged goal-specific operator/tests. The executor model,
Human UUID, append-only mechanism, transaction/replay rules, and HR consequence
are unchanged from V1.

Out of scope: repair/reopen of the three excluded terminal Workflows; any other
Workflow or Principal; title/content inference; generic migration; HR filters;
schema/API/Definition changes; deployment behavior; and production apply.

## 3. Authority and dependencies

Product Boundary V8 supplies the mechanical current-executor relation.
Architecture v0.4.1 supplies immutable Visit identity and append-only
administrative successor structure. V1 is the complete predecessor authority;
V2 replaces it as a whole because changing the frozen target set narrows
accepted normative meaning and cannot be an in-place amendment. The existing
Human projection authority remains unchanged and has already produced the
enabled exact Workflow Principal observed below.

## 4. Current State

- `STATE-HEN2-001` — At source base
  `7c3beec0ee058aa896b86e58443c502a48d23d11` and production observation time
  2026-09-15 10:45 Asia/Shanghai, V1 is accepted and implemented but its
  production apply is blocked by one additional post-snapshot transition. The
  remaining 17 are active, Agent-owned, and exactly represented by the V2 plan; the target Human
  is enabled. Basis: `OBS-HEN2-001`..`OBS-HEN2-004`, `EVD-HEN2-001`..`002`.

## 5. Observations

### OBS-HEN2-001 — V1 exact-plan preflight

- Subject: V1's frozen 18-row plan against production
  `svc_workflow_dogfood_clean`.
- Source/artifact coordinate: merged implementation
  `7c3beec0ee058aa896b86e58443c502a48d23d11`, plan SHA
  `bba710b9790fed4c0136b9a0f33186f87f11be9e5da3f76e08784bfbce8dd871`.
- Observed at: 2026-09-15 10:45 Asia/Shanghai.
- Method: release-build V1 operator `--plan`; followed by PostgreSQL
  `BEGIN READ ONLY` exact-ID inspection.
- Result: the operator returned `CONFLICT` with `writes=0`; 17 rows retain the
  exact V1 preimage, zero V1 normalization artifacts exist, and one row no
  longer matches.
- Provenance:
  `docs/evidence/human-executor-normalization-v2/production-preflight-20260915.md`.

### OBS-HEN2-002 — Additional post-snapshot transition

- Subject: Workflow `0dbf2597-c6f5-4446-aef6-4a5232bc8a1e`.
- Environment: same production database; read-only inspection on 2026-09-15.
- Result: it moved from state version 2 to 3 through a persisted
  `WORKFLOW_TRANSITION_COMMITTED` / `ADVANCE` event at 2026-09-15 10:38 +08:00,
  using transition key `advance-to-completed`; its current node is `completed` of
  type `TERMINAL` with no current assignee.
- Limitation: persisted completion does not prove the real-world action was
  performed.
- Provenance:
  `docs/evidence/human-executor-normalization-v2/production-preflight-20260915.md`.

### OBS-HEN2-003 — Three terminal exclusions

- Subject: the two V1 exclusions plus the Workflow in OBS-HEN2-002.
- Source: explicit Owner statement in the 2026-09-15 Lane G execution thread.
- Result: all three are now mechanically terminal and unassigned. The Owner has
  stated the two V1 exclusions are not complete in reality; no equivalent
  real-world claim is made for the newly terminal repair Workflow. All three
  are excluded because this operator cannot rewrite terminal history.
- Limitation: repair/reopening of any terminal Workflow requires a separate
  authority and is not part of V2.
- Provenance: Owner decision in the Lane G thread; the resulting bounded
  exclusions and machine preimages are persisted in the V2 plan and preflight
  record.

### OBS-HEN2-004 — Human projection and remaining preimage

- Environment: production read-only inspection on 2026-09-15.
- Result: Principal `8902db0d-429a-4e37-985c-f8b92d4b78fb` exists as enabled
  `HUMAN`; each row in the V2 plan still matches its exact current Visit,
  version, Agent assignee, DefinitionVersion, node, Context digest, and unused
  target Visit UUID.
- Provenance:
  `docs/evidence/human-executor-normalization-v2/production-preflight-20260915.md`.

## 6. Claims and assumptions

### CLM-HEN2-001 — Exact-17 replacement is required

- Support state: SUPPORTED.
- Basis: OBS-HEN2-001..003.
- Claim: V1 cannot lawfully apply because its all-or-zero target is stale;
  excluding the additional transitioned row and freezing the remaining 17 is the
  smallest correction that preserves current business history.

### CLM-HEN2-002 — The exact Human and remaining preimages are ready

- Support state: SUPPORTED.
- Basis: OBS-HEN2-004 and EVD-HEN2-002.
- Claim: the already projected same-UUID Auth User/Workflow Human and the exact
  17 mechanical preimages satisfy the identity and target prerequisites for a
  later apply-time revalidation; they do not themselves authorize writes.

No normative open assumption remains.

## 7. Evidence relations

### EVD-HEN2-001

- Source observations: OBS-HEN2-001, OBS-HEN2-002.
- Target type and IDs: Claim CLM-HEN2-001; State STATE-HEN2-001.
- Relation: SUPPORTS.
- Bound coordinates: production `svc_workflow_dogfood_clean`, observed
  2026-09-15 10:45 +08:00; source main `7c3beec0ee058aa896b86e58443c502a48d23d11`;
  V1 and V2 plan hashes stated in section 1.
- Strength/sufficiency: strong for exact stale rows, persisted transitions,
  zero normalization artifacts, and required whole-scope replacement.
- Limitations: read-only observation proves neither implementation conformance
  nor future apply-time state and grants no writes.
- Provenance:
  `docs/evidence/human-executor-normalization-v2/production-preflight-20260915.md`.

### EVD-HEN2-002

- Source observations: OBS-HEN2-003, OBS-HEN2-004.
- Target type and IDs: Claim CLM-HEN2-002; State STATE-HEN2-001.
- Relation: SUPPORTS.
- Bound coordinates: exact V2 plan SHA, target Human UUID, production database,
  and 2026-09-15 observation recorded above.
- Strength/sufficiency: strong for the selected exact exclusion, same-UUID
  enabled Workflow Human, 17/17 preimage match, and zero target collisions.
- Limitations: Owner statement establishes business intent but not repository
  acceptance; all facts remain subject to apply-time revalidation.
- Provenance: V2 exact plan, the production preflight record, and the Owner
  decision recorded in this execution thread.

## 8. Decisions

### DEC-HEN2-001 — Supersede exact 18 with exact 17

- Decision owner: repository Owner `mayf3` through the 2026-09-15 Lane G
  ruling.
- Selected: exclude the additional ID in OBS-HEN2-002, retain both V1
  exclusions, and normalize only the exact 17 rows in the V2 plan.
- Rejected: applying stale V1; dynamic subset flags; title/content selection;
  silently reopening terminal rows; treating persisted completion as real-world
  proof.
- Reason: current facts must be preserved while the still-valid Human backlog
  is corrected.
- Remaining Owner input: acceptance of the exact reviewed candidate.

### DEC-HEN2-002 — Preserve V1 correction mechanism

- Decision owner: repository Owner `mayf3`.
- Selected: retain V1's exact-plan-bound, append-only same-node successor Visit,
  SERIALIZABLE all-or-zero transaction, deterministic receipt/Event/audit,
  fail-closed prevalidation, and exact replay model, with only plan identity and
  cardinality changed to 17.
- Rejected: same-Visit mutation, business transition, new executor field,
  generic migration framework, HR filter, activation/Dispatch Intent creation,
  or per-row best effort.
- Remaining Owner input: none.

## 9. Contracts

### CTR-HEN2-001 — Exact finite scope

The operator MUST process only the 17 rows and target Visit UUIDs in the V2
plan. It MUST embed and verify the exact plan bytes and SHA. It MUST accept no
plan path, Workflow/owner UUID, row selector, subset, or content-derived input.
The three excluded Workflow IDs MUST be absent from its plan and writes.

### CTR-HEN2-002 — Canonical Human and operator gates

Before any Workflow write, the target MUST be exact Principal
`8902db0d-429a-4e37-985c-f8b92d4b78fb`, mechanically bound to the exact active
Auth User of the same UUID and projected as an enabled Workflow `HUMAN`
Principal. The supplied operator actor MUST exist, be enabled, and type
`AGENT`; every target Instance MUST retain Legacy `semantic_model_version=1`;
and the actual database name MUST equal the separately supplied execution
coordinate. Missing, disabled, conflicting, differently bound, or mismatched
state causes zero writes.

### CTR-HEN2-003 — Append-only executor correction

For each exact row, append the preassigned same-Instance, same-node Human Visit
with `visit_number = source + 1` and `entered_by_transition_id = NULL`. CAS
update only current Visit pointer, state version `expected + 1`, and timestamp.
Never update or delete a source Visit.

### CTR-HEN2-004 — No fabricated business action

Preserve Instance identity, DefinitionVersion, node, Context, creator, external
reference, metadata, artifact references, lifecycle, cancellation/archive
state, semantic model, and business payload. Create no Submission,
transition/effect, completion, activation, Dispatch Intent, or human evidence.
Modify no historical row and no excluded Workflow.

### CTR-HEN2-005 — Atomic fail-closed group

One SERIALIZABLE advisory-locked transaction MUST cover all 17 Visits, 17 CAS
updates, 17 Events, 17 completed Receipts, and one group audit. Lock exact
Instances in deterministic order and completely prevalidate every row plus all
target Visit, receipt command/idempotency, Event ID/command/sequence, and audit
collisions before the first write. Drift, collision, assistance, or mismatch
aborts with zero operator writes.

### CTR-HEN2-006 — Exact replay and unknown outcome

Each row MUST use deterministic command, Event, and idempotency identities that
bind Spec, plan, implementation SHA, database, actor, Human, and row
coordinates. Exactly one Event MUST link to each completed Receipt. Exact rerun
MUST verify all 17 linked pairs, one audit, and full poststate before zero-write
`NOOP`. Partial/asymmetric/corrupt state conflicts. Commit outcome uncertainty
MUST reconcile by fresh readback and MUST NOT blindly retry.

### CTR-HEN2-007 — Mechanical HR consequence

After apply, the exact 17 MUST remain lifecycle `ACTIVE`, be mechanically
`current executor type = HUMAN`, and be absent from `ACTIVE + AGENT` solely by
the current Visit assignee Principal type. No HR title, test, identity, Domain,
readiness, or special-case filter is authorized.

### CTR-HEN2-008 — Separation and production boundary

V2 authoring/acceptance, implementation, merge, and production apply remain
separate gates. This Spec does not itself authorize production apply. Repair of
the three excluded terminal Workflows is separate follow-up debt and MUST NOT be
performed by the normalization operator.

## 10. Acceptance

### ACC-HEN2-001 — Plan and target closure

- Contracts: CTR-HEN2-001, CTR-HEN2-002.
- Method: exact plan digest/cardinality/exclusion checks plus target/operator
  type matrix in disposable PostgreSQL.
- Expected: 17 unique rows; all three excluded IDs absent; invalid identity or DB
  coordinate produces zero writes.

### ACC-HEN2-002 — Append-only preservation

- Contracts: CTR-HEN2-003, CTR-HEN2-004.
- Method: full before/after projections including a lawful non-null source
  `entered_by_transition_id`, unrelated row, and all three excluded Workflow
  fixtures.
- Expected: only 17 appended Human Visits and exact current projections change;
  business/source/excluded facts are unchanged.

### ACC-HEN2-003 — Atomicity and replay

- Contracts: CTR-HEN2-005, CTR-HEN2-006.
- Method: success, one-row drift, Visit/command/Event/audit collision, open
  assistance, injected mid-group failure, rerun, corrupted poststate, and
  commit-acknowledgement-loss reconciliation.
- Expected: counts 17 Visits/Events/Receipts and one audit on success; all
  rejected cases have zero operator writes; replay is exact `NOOP`.

### ACC-HEN2-004 — Executor result

- Contracts: CTR-HEN2-007, CTR-HEN2-008.
- Method: exact-ID current Visit/Principal query and excluded/unrelated diffs.
- Expected: exact 17 `ACTIVE + HUMAN`, zero exact 17 `ACTIVE + AGENT`, excluded
  rows unchanged, and no HR classifier.

| Contract | Acceptance |
|---|---|
| CTR-HEN2-001 | ACC-HEN2-001 |
| CTR-HEN2-002 | ACC-HEN2-001 |
| CTR-HEN2-003 | ACC-HEN2-002 |
| CTR-HEN2-004 | ACC-HEN2-002 |
| CTR-HEN2-005 | ACC-HEN2-003 |
| CTR-HEN2-006 | ACC-HEN2-003 |
| CTR-HEN2-007 | ACC-HEN2-004 |
| CTR-HEN2-008 | ACC-HEN2-004 |

## 11. Alternatives and disposition

- `ALT-HEN2-001` apply stale exact-18 authority: rejected because one row no
  longer matches and committed business history cannot be overwritten.
- `ALT-HEN2-002` runtime subset/exclusion flags: rejected because the operator
  must remain closed to one reviewed finite plan.
- `ALT-HEN2-003` reopen the three terminal Workflows here: rejected as a separate
  lifecycle/business repair requiring its own evidence and authority.
- `ALT-HEN2-004` infer remaining Human work from titles: rejected; membership
  comes only from exact UUIDs and fresh mechanical preimages.

## 12. Migration, compatibility, and rollback

Implementation is limited to updating the existing goal-specific offline Rust
binary and its existing disposable PostgreSQL test module. No API, schema,
field, generic migration framework, deployment behavior, HR code, credential,
Grant, Definition, or runtime selection mechanism may change.

Pre-commit failure rolls back the entire group. Post-commit uncertainty is
contained and reconciled through exact readback; committed facts are never
deleted or reversed without new authority. All three excluded Workflows remain
untouched; the Owner's real-world incompletion statement applies only to the
two earlier exclusions.

## 13. Open questions and author output

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE AFTER WHOLE SUPERSESSION
PARTIAL_SUPERSESSION = NONE
```

```text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V2
SPEC_KIND = implementation
STATUS = accepted
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
