---
spec_id: SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V0
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: none
scope: [mayf3/svc-workflow, exact-20-human-executor-normalization]
governed_by: [SVC_WORKFLOW_PRODUCT_BOUNDARY_V8, SVC_WORKFLOW_ARCHITECTURE_V0_4_1]
related_authorities: [SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1, IDENTITY_PROVISIONING_API_V0]
supersedes: []
superseded_by: null
owners: [mayf3]
title: Human Executor Normalization V0
repo: mayf3/svc-workflow
base_head: a1000604e2182947978cf5d475e8fa723caff7c4
production_apply_authorized_now: false
merge_required_for_activation: true
---

# SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V0

## 1. Goal

Correct the exact 20 Owner-confirmed human-work Workflow instances from a
mechanically false `ACTIVE + AGENT` current executor to `ACTIVE + HUMAN`, while
leaving every real-world task unresolved and every historical fact intact.

```text
PLAN_PATH = docs/evidence/human-executor-normalization-v0/exact-20-plan.tsv
PLAN_SHA256 = b349e203c00ac82e286666a89dbedd6a17f77e0221090a1a9f2db51d8a253199
TARGET_COUNT = 20
TARGET_AUTH_USER_ID = 8902db0d-429a-4e37-985c-f8b92d4b78fb
TARGET_WORKFLOW_PRINCIPAL_ID = 8902db0d-429a-4e37-985c-f8b92d4b78fb
TARGET_PRINCIPAL_TYPE = HUMAN
```

## 2. Scope and non-goals

Scope is one offline, exact-plan-bound administrative correction. It appends
one preassigned same-Instance, same-node Human Visit per row, then changes the
current projection by compare-and-swap. It never updates the source Visit or
executes a Definition transition.

Out of scope: HTTP APIs; schema or executor-field changes; generic migration or
reassignment; runtime row selection; title/content inference; HR filters;
readiness classification; activation or Dispatch Intent creation; leases;
future-chain validation; the excluded test canary; nine invalid Agent owners;
Lane D/F repair; deployment; and production apply.

## 3. Authority and dependencies

Accepted Product Boundary V8 supplies the executor relation and
`ACTIVE + HUMAN` / `ACTIVE + AGENT` boundary. Accepted Architecture v0.4.1
supplies immutable Visit identity and append-only same-node administrative
successor structure. Accepted `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1`
is a storage precedent only; its frozen OLD/NEW scope is not reused.
`IDENTITY_PROVISIONING_API_V0` remains the sole authority for the separate
Workflow Principal projection.

This candidate has `implementation_authority: none` and
`production_apply_authorized_now: false`. If an authorized Owner accepts the
exact reviewed candidate and merges it to main, acceptance may set
`implementation_authority: contracts`. Acceptance and merge still perform no
identity projection and no Workflow mutation.

## 4. Current State

- `STATE-HEN-001` — In production DB `svc_workflow_dogfood_clean`, observed
  `2026-09-14T07:43:29+08:00`, the exact plan has 20 active model-1 instances
  whose current Visits are assigned to AGENT `b21ddb23-42f6-47c4-a27f-bc44950e554c`.
  Basis: `OBS-HEN-001`, `EVD-HEN-001`.
- `STATE-HEN-002` — At auth-service production readback after Owner-authorized
  registration, exact User `8902db0d-429a-4e37-985c-f8b92d4b78fb` is enabled
  and active; its Workflow Principal projection is absent. Basis:
  `OBS-HEN-002`, `OBS-HEN-003`, `EVD-HEN-002`.
- `STATE-HEN-003` — At source base `a1000604e2182947978cf5d475e8fa723caff7c4`,
  storage permits an immutable same-node successor Visit, while the Event index
  permits at most one Event per command ID. Basis: `OBS-HEN-004`, `EVD-HEN-003`.

## 5. Observations

### OBS-HEN-001 — Exact production row snapshot

- Subject: Owner-confirmed human-work row set.
- Source revision: `a1000604e2182947978cf5d475e8fa723caff7c4`.
- Environment: production `svc_workflow_dogfood_clean`.
- Observed at: `2026-09-14T07:43:29+08:00`.
- Method: read-only PostgreSQL exact UUID regeneration.
- Result: 20 rows match current Visit/version/assignee/Definition/node/Context;
  20 preassigned target Visit UUIDs are unique with zero production conflicts.
- Provenance: `docs/evidence/human-executor-normalization-v0/`.

### OBS-HEN-002 — Canonical Auth Human readback

- Subject: exact Owner-authorized Auth User.
- Environment: deployed auth-service production database.
- Observed at: `2026-09-14`.
- Method: exact UUID and email cardinality readback after normal registration.
- Result: one enabled active User, no same-email second UUID, `agentId` null.
- Provenance: Lane G2 registration/readback record; no credential value is
  included in this repository.

### OBS-HEN-003 — Workflow projection preimage

- Subject: Workflow Principal `8902db0d-429a-4e37-985c-f8b92d4b78fb`.
- Environment: production `svc_workflow_dogfood_clean`.
- Observed at: `2026-09-14T07:43:29+08:00`.
- Method: read-only exact Principal lookup.
- Result: no local Principal row exists.
- Provenance: exact-plan generation readback.

### OBS-HEN-004 — Successor and command cardinality

- Subject: svc-workflow migrations and accepted successor implementation.
- Source revision: `a1000604e2182947978cf5d475e8fa723caff7c4`.
- Environment: repository source.
- Observed at: `2026-09-14`.
- Method: read Visit/Instance/Event/Receipt constraints and accepted Spec.
- Result: same-node successor Visit preserves source history;
  `idx_wf_event_unique_command` allows at most one Event per command ID.
- Provenance: migrations 0003-0006 and principal-successor Spec section 9.

## 6. Claims and assumptions

### CLM-HEN-001 — Executor correction needs no business completion

- Support state: SUPPORTED.
- Supported by evidence: `EVD-HEN-001`, `EVD-HEN-003`.
- Contradicted by evidence: none known.
- Uncertainty: production coordinates require apply-time revalidation.

### CLM-HEN-002 — The registered User is the lawful Human target

- Support state: SUPPORTED.
- Supported by evidence: `EVD-HEN-002`.
- Contradicted by evidence: none known.
- Uncertainty: the separate Workflow projection remains unapplied.

### CLM-HEN-003 — Row commands can remain group-atomic

- Support state: SUPPORTED.
- Supported by evidence: `EVD-HEN-003`.
- Contradicted by evidence: none known.
- Uncertainty: isolated conformance must qualify transaction and replay at the
  implementation revision.

No normative open assumption remains.

## 7. Evidence relations

### EVD-HEN-001 — Snapshot supports bounded correction

- Source observations: `OBS-HEN-001`.
- Target: `CLM-HEN-001`, `STATE-HEN-001`.
- Relation: SUPPORTS.
- Bound coordinates: plan SHA
  `b349e203c00ac82e286666a89dbedd6a17f77e0221090a1a9f2db51d8a253199`.
- Strength/sufficiency: strong for exact identity and preimage.
- Limitations: no future production or implementation proof.

### EVD-HEN-002 — Registration supports canonical target

- Source observations: `OBS-HEN-002`, `OBS-HEN-003`.
- Target: `CLM-HEN-002`, `STATE-HEN-002`.
- Relation: SUPPORTS.
- Bound coordinates: exact Auth/Workflow UUID above, `2026-09-14`.
- Strength/sufficiency: strong for identity and missing projection.
- Limitations: no projection mutation authority or result.

### EVD-HEN-003 — Constraints support row-command model

- Source observations: `OBS-HEN-004`.
- Target: `CLM-HEN-001`, `CLM-HEN-003`, `STATE-HEN-003`.
- Relation: SUPPORTS.
- Bound coordinates: svc-workflow `a1000604e2182947978cf5d475e8fa723caff7c4`.
- Strength/sufficiency: strong for current schema constraints.
- Limitations: executed results remain future conformance evidence.

## 8. Decisions

### DEC-HEN-001 — Exact same-node successor Visits

- Decision owner: repository Owner `mayf3` through the Lane G2 ruling.
- Decision: append one preassigned same-Instance, same-node Human Visit per
  exact row and update only the current pointer/version projection.
- Rejected alternatives: same-Visit update, business transition, successor
  Instance, generic reassignment, and HR filtering.
- Reason: preserve business state and immutable history while correcting actor.
- Remaining Owner input: none for candidate semantics.

### DEC-HEN-002 — One canonical Owner Human

- Decision owner: repository Owner `mayf3`.
- Decision: use only UUID `8902db0d-429a-4e37-985c-f8b92d4b78fb` for all 20
  confirmed same-executor rows.
- Rejected alternatives: guessing an existing User, Agent conversion,
  per-Workflow Humans, or a new identity framework.
- Reason: explicit Owner-authorized registration supplies the binding.
- Remaining Owner input: none for identity selection.

### DEC-HEN-003 — Preserve one Event per command

- Decision owner: repository Owner `mayf3`, bounded by current Architecture
  and schema.
- Decision: execute 20 exact row commands in one SERIALIZABLE transaction,
  each with one Receipt and one Event, plus one group audit.
- Rejected alternative: one group command ID shared by 20 Events.
- Reason: that alternative violates `idx_wf_event_unique_command`.
- Remaining Owner input: none.

## 9. Contracts

### CTR-HEN-001 — Exact finite scope

The operator MUST process only the exact 20 rows and target Visit UUIDs in the
frozen plan. It MUST NOT accept plan path, Workflow/owner UUID, row selector,
arbitrary subset, or title/content-derived row inputs.

### CTR-HEN-002 — Canonical Human projection gate

Before Workflow writes, the target MUST be the exact active Auth User projected
under the same UUID as an enabled Workflow `HUMAN` Principal. Projection MUST
use the existing API as a prior separate idempotent mutation. Missing,
disabled, AGENT, conflicting, or differently bound state MUST cause zero
Workflow writes.

### CTR-HEN-003 — Append-only executor correction

Each row MUST append the preassigned same-Instance, same-node Human Visit with
`visit_number = source + 1` and `entered_by_transition_id = NULL`. The operator
MUST CAS-update only current Visit pointer, state version `expected + 1`, and
timestamp. It MUST NOT update the source Visit.

### CTR-HEN-004 — No fabricated business action

Normalization MUST preserve Instance, DefinitionVersion, node, Context,
creator, external reference, artifacts, lifecycle, cancellation/archive, and
business payload. It MUST create no Submission, transition/effect, completion,
activation, Dispatch Intent, or human evidence, and modify no historical row.

### CTR-HEN-005 — Group atomicity and fail-closed drift

One bounded SERIALIZABLE transaction MUST cover all 20 Visits and CAS writes,
20 row Events, 20 completed row Receipts, and one group audit. Deterministic
locks and complete prevalidation MUST precede the first write. Any row drift,
duplicate, target collision, conflicting command, or open visit assistance MUST
abort the group with zero writes.

### CTR-HEN-006 — Row linkage and exact replay

Each row MUST have a unique fixed command/idempotency identity binding Spec,
plan, DB, code, actor, Human, and exact source/target coordinates; exactly one
Event MUST link to its completed Receipt. Exact rerun MUST verify all 20 pairs,
one audit, and full poststate before zero-write `NOOP`. Asymmetry or changed
identity/poststate MUST conflict. Unknown outcome MUST be re-observed under the
same identities and MUST NOT be blindly retried.

### CTR-HEN-007 — HR consequence without classifier

After apply, the exact 20 MUST remain `ACTIVE + HUMAN` and be absent from
`ACTIVE + AGENT`. No HR classifier, special/title/identity/Domain/test filter
may be added.

### CTR-HEN-008 — Lane and production separation

Canary cleanup, invalid Agent owners, Lane D/F, Human projection, exact-20
apply, code merge, and deployment MUST remain separate gates. This candidate
MUST perform and authorize no production Workflow mutation.

## 10. Acceptance

### ACC-HEN-001 — Scope and target

- Contracts: `CTR-HEN-001`, `CTR-HEN-002`.
- Method: isolated plan parser, digest, and target-type matrix.
- Environment: disposable PostgreSQL.
- Required evidence: implementation/plan SHA, 20 rows, zero-write rejections.
- Expected result: only exact rows and exact enabled HUMAN pass.
- Failure condition: dynamic/content selection or invalid target writes.

### ACC-HEN-002 — Append-only preservation

- Contracts: `CTR-HEN-003`, `CTR-HEN-004`.
- Method: full before/after row projections around success.
- Environment: production-shaped disposable PostgreSQL.
- Required evidence: Visit/Instance/Context/Submission/Event/Receipt/business
  diffs at exact implementation SHA.
- Expected result: 20 Human Visits and projection/version changes only.
- Failure condition: old-row mutation or fabricated business action.

### ACC-HEN-003 — Atomicity and replay

- Contracts: `CTR-HEN-005`, `CTR-HEN-006`.
- Method: success, one-row drift, collision, injected mid-group failure, exact
  rerun, corruption, and unknown-outcome cases.
- Environment: disposable PostgreSQL under current migrations.
- Required evidence: counts of 20 Visits, 20 Events, 20 Receipts, one audit;
  zero-write failure and NOOP traces.
- Expected result: all-or-zero apply and exact zero-write replay.
- Failure condition: partial commit, duplicate Event/command, or blind retry.

### ACC-HEN-004 — Executor and lane postconditions

- Contracts: `CTR-HEN-007`, `CTR-HEN-008`.
- Method: exact-ID worklist queries and unrelated-row diff.
- Environment: disposable PostgreSQL.
- Required evidence: exact 20 ACTIVE+HUMAN, zero exact ACTIVE+AGENT, unchanged
  canary/invalid-owner/Lane D/F fixtures.
- Expected result: natural exclusion through Principal type only.
- Failure condition: special filter, wrong count, or unrelated mutation.

| Contract | Acceptance | Covered |
|---|---|---|
| CTR-HEN-001 | ACC-HEN-001 | yes |
| CTR-HEN-002 | ACC-HEN-001 | yes |
| CTR-HEN-003 | ACC-HEN-002 | yes |
| CTR-HEN-004 | ACC-HEN-002 | yes |
| CTR-HEN-005 | ACC-HEN-003 | yes |
| CTR-HEN-006 | ACC-HEN-003 | yes |
| CTR-HEN-007 | ACC-HEN-004 | yes |
| CTR-HEN-008 | ACC-HEN-004 | yes |

## 11. Alternatives and disposition

- `ALT-HEN-001` direct old-Visit update: rejected as history rewrite.
- `ALT-HEN-002` HR title/content inference: rejected as false data plus business
  classification in discovery.
- `ALT-HEN-003` new executor field/identity framework: rejected; existing
  canonical types suffice.
- `ALT-HEN-004` one command ID for 20 Events: rejected by current unique index.
- `ALT-HEN-005` successor Instances or auto-completion: rejected as business
  identity/state change or fabricated real-world action.

## 12. Migration, compatibility, and rollback

Implementation is frozen to one goal-specific offline Rust binary, one
disposable PostgreSQL runner, one focused integration-test module, and
manifest/build registration only if mechanically required. No production apply
script, API, schema migration, reusable framework, dynamic plan loader, or HR
code is authorized.

Before Workflow apply, a separately authorized call through existing
`POST /internal/v1/admin/principals` must create and read back the exact HUMAN
projection. Later exact-20 run authorization must separately bind clean code
SHA, plan SHA, DB identity, actor, preimage, containment, and readback. One
production mutation is verified before another begins.

Pre-commit rollback is transaction abort. Post-commit rollback is containment:
preserve facts and repair only under new authority. Deleting successor facts,
rewriting source Visits, or restoring an AGENT current projection is forbidden.
Existing list, transition, visibility, Domain, identity, and error contracts
remain unchanged. The 20 Workflows remain Legacy model 1 and ACTIVE.

## 13. Open questions

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
```

The later production run authorization selects the exact migration actor and
window; these are execution coordinates, not unresolved semantics.
