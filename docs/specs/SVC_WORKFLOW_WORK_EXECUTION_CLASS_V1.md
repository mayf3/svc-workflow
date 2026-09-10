---
spec_id: SVC_WORKFLOW_WORK_EXECUTION_CLASS_V1
title: Work Execution Class — explicit BUSINESS / NON_BUSINESS_TEST classification with governed marking
status: accepted
spec_kind: implementation
authority_level: governing_spec
date: 2026-09-11
type: implementation-spec (one DB classification fact + one create-request field + one due-set conjunct + one summary field, with explicit amendments to two accepted feed contracts and one versioned HTTP contract)
repo: mayf3/svc-workflow
base_head: dd235dcf755e5007061874f19a3f1552b786c8c4 (github/main, re-fetched 2026-09-11)
accepted_date: 2026-09-11
accepted_by: mayf3
accepted_reviewed_head: a62e12afad423515a06c0c99b9a4c3f7a0b82a00
independent_review_result: PASS
independent_review_blockers: 0
independent_review_record: mayf3/svc-workflow#39 / pullrequestreview-5173123572
scope:
  - mayf3/svc-workflow (workflow_instances classification fact, create marking, due-feed predicate, domain/global summary projection, 0026 migration)
  - mayf3/dsh-agent-core (declared broker companion delta only, CTR-WEC-006 — authored and governed in that repository)
implementation_authority: contracts
production_apply_authority: none
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V7
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_1
external_authorities:
  - repository: mayf3/svc-workflow
    authority_id: SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1 (accepted — CTR-VAI-004
      activation-kind derivation and CTR-VAI-009 due-set selection; amended by
      AMENDMENT A in §3 of this Spec)
    relation: constrained_by
  - repository: mayf3/svc-workflow
    authority_id: SVC_WORKFLOW_DISPATCH_INTENT_KEYSET_CONTINUATION_V1 (accepted —
      CTR-DKC-002 result-set freeze; amended by AMENDMENT B in §3 of this Spec)
    relation: constrained_by
  - repository: mayf3/svc-workflow
    authority_id: SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1 (accepted — two-variant
      eligibility projection; NOT amended; this Spec adds no blocked state)
    relation: constrained_by
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_WORKFLOW_AGENT_EXECUTION_V2 (accepted @ b1fb7c0 — the due
      -feed consumer; consumed UNCHANGED)
    relation: interoperates_with
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_WORKFLOW_BROKER_ERROR_PRESERVATION_V1 (accepted @ b1fb7c0 —
      governs 403 passthrough on the declared broker companion delta, CTR-WEC-006)
    relation: depends_on
supersedes: []
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_WORK_EXECUTION_CLASS_V1 — explicit BUSINESS / NON_BUSINESS_TEST classification

> **ACCEPTED (2026-09-11, Owner exact-head acceptance).** Owner decision ACCEPT EXACT
> HEAD = YES at `a62e12afad423515a06c0c99b9a4c3f7a0b82a00` (base
> `dd235dcf755e5007061874f19a3f1552b786c8c4`, HEAD_UNCHANGED_SINCE_REVIEW = YES,
> CURRENT_MAIN_DRIFT = NO); final independent exact-head review = PASS /
> SHIP_BLOCKERS = 0 (record: mayf3/svc-workflow#39 / pullrequestreview-5173123572).
> `implementation_authority: none -> contracts`; `production_apply_authority` stays
> **none**. This acceptance transaction is lifecycle/provenance only — every §1–§13
> semantic byte below is preserved verbatim from the accepted head.

## 1. Goal

Make test/canary workflow work a first-class, machine-readable classification so that
it is structurally excluded from the normal BUSINESS automated dispatch path, while
ordinary business creation regresses zero. Governing directive:
WORKFLOW_ASSIGNEE_ADMISSION_GUARD_V1 LANE_TEST_CLASSIFICATION (Owner ruling r3,
2026-09-11); frozen minimal design:
dsh-agent-core `docs/investigations/WORKFLOW_ASSIGNEE_ADMISSION_GUARD_TEST_CLASSIFICATION_DESIGN_V1.md`
at dsh commits 8dc489e..e9699ae (MINIMAL_DESIGN_REVIEW = PASS / LOAD_BEARING_GAPS = 0,
record in header of that document).

The invariant served, and nothing else:

```text
NON_BUSINESS_TEST work → MUST NOT appear in the normal BUSINESS due feed
NORMAL BUSINESS work   → regresses zero
SUPPORTED_NEW_TEST_CANARY_CREATED_AS_BUSINESS = 0 (structural once implemented)
```

HUMAN_REQUIRED safety is NOT this Spec's subject: activation_kind (CTR-VAI-004) already
derives HUMAN_WORK_ITEM from the resolved principal type, and a HUMAN owner can never
mint a DISPATCH_INTENT. This Spec adds no human/agent semantics.

## 2. Scope and non-goals

In scope: one enum type + one NOT NULL DEFAULT 'BUSINESS' column on
`workflow_instances`; one optional create-request field whose non-default value requires
in-transaction enabled DOMAIN_OWNER on the target domain; one due-set predicate
conjunct; one class-only field on the shared domain/global instance summary; the
explicit amendments listed in §3.

Out of scope (frozen): no definition-level or version-level classification;
no general taxonomy/framework (exactly two enum values); no blocked state and no
WorkEligibility change (its "NO BLOCKED state" freeze stands); no activation_kind
change; no name/substring/title/domain-key inference anywhere (in code, tests, or
tooling); no new role, no new grant, no identity authority, no retry engine, no
consumer/protocol change (WAE V2 consumes the narrowed feed UNCHANGED);
no worklist filtering (marked instances stay fully visible and executable through
their assignee worklist — the targeted execution path); no historical reclassification
of existing rows; no HR identity or coordinator authority expansion.

## 3. Authority and dependencies — EXPLICIT AMENDMENTS (no silent override)

This Spec is the single authority artifact for the semantic delta; the accepted
contracts it narrows are amended HERE, in the open, per the repository rule that a
semantic change to accepted contract meaning names and amends its authority. The
governance form is the repo's accepted named-amendment precedent:
SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1 (accepted, dd235dc) carries
`supersedes: []` while amending GLOBAL_WORKFLOW_READER_V1 §3 in-body with an explicit
non-supersession declaration — the same form used here. This Spec DOES NOT supersede
VISIT_ACTIVATION_IMPL_V1 or DISPATCH_INTENT_KEYSET_CONTINUATION_V1: both survive in
full except for the exact deltas named below (whole-authority supersession would
retire entire accepted feed contracts for a one-conjunct delta).

### AMENDMENT A — SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1 / CTR-VAI-009

The CTR-VAI-009 due Dispatch-Intent read ("active DISPATCH_INTENT activations with
current nextEligibleAt <= authoritative now") gains exactly one predicate conjunct:

```text
AND wi.execution_class = 'BUSINESS'
```

Everything else in CTR-VAI-009/010 is preserved verbatim: endpoint, scope +
direct-token + enabled GLOBAL_SCHEDULER_READ fail-closed gate, limit 1..100, ordering
by `(current nextEligibleAt, activation_id)`, EXACTLY the same 7 fields, zero-write
feed. The class conjunct only removes rows; it never adds, reorders, or reshapes them.

### AMENDMENT B — SVC_WORKFLOW_DISPATCH_INTENT_KEYSET_CONTINUATION_V1 / CTR-DKC-002

CTR-DKC-002 freezes "the result set is the CTR-VAI-009 due set (unchanged predicate …)
INTERSECTED with the exclusive keyset filter." With AMENDMENT A, the result set is the
CLASS-NARROWED CTR-VAI-009 due set intersected with the same exclusive keyset filter.
Preserved verbatim: cursor key == order key expression identity, both-or-neither cursor
parameters and 422 validation precedence, LIMIT 1..100, short-page exhaustion,
CTR-DKC-003 response contract (the conjunct adds no field), CTR-DKC-004 feed-safety
argument (exclusion cannot move keys; class immutability preserves the
"caught by the next cursorless sweep" reasoning), and the deployment-order dependency
with the dsh consumer (CTR-WAE-001b: svc deploys before the poller is enabled — the
narrowing is consumer-invisible by construction because excluded rows were never
admissible business work).

### AMENDMENT C — versioned HTTP contract `contracts/workflow-http/v1/openapi.yaml`

`DomainInstanceSummary` (:1397, `additionalProperties: false`) gains one property
`execution_class` (string enum `BUSINESS | NON_BUSINESS_TEST`). WIRE NAMING IS
TWO-CONVENTION BY DESIGN and must not be "normalized" by implementation: the summary
object is snake_case on the wire (`DomainInstanceSummary` derives plain `Serialize`
with no rename attribute — `workflow_instance_id`, `current_assignee_canonical_agent_id`,
`eligibility`; openapi properties snake_case), so the summary property is
`execution_class`; the CREATE request body stays camelCase
(`CreateWorkflowInstanceRequest` is `#[serde(rename_all = "camelCase")]`, so the
request field is `executionClass`), matching the existing per-endpoint conventions
frozen by compatibility.md rule 2. The schema marks the new summary property REQUIRED:
the column is NOT NULL DEFAULT 'BUSINESS', so every post-deploy binary emits it on
every summary; old binaries omit it (wire-tolerant in both directions — new fields are
ignored by existing clients; strict validators run against conformance fixtures served
by the new binary), and the changelog entry states that pre-deploy binaries omit the
field explicitly (the dual-timeline pattern applied to a schema field). changelog.md
gains the entry; conformance fixtures and digests are refreshed in the same
implementation closure. This amendment DELIBERATELY breaks the silence precedent of
WORK_ELIGIBILITY_PROJECTION_V1 (which added `eligibility` without touching this
contract): the contract is amended in the same transaction instead of being left
stale.

### PRODUCT_BOUNDARY_V7 reconciliation

`executionClass` is a STORED CLASSIFICATION stamped at creation — not a computed
dispatchable flag, not a blocked-state code, and not a second scheduler subject. The
V7 exclusivity clause on the `Page<DomainInstanceSummary>` family (only the exact
opt-in pair and closed non-sensitive codes) is not engaged by a stored class field;
V7's `CANONICAL_SCHEDULER_SUBJECT` remains exactly the (now class-narrowed)
DISPATCH_INTENT due feed — the class narrows the feed, it does not become an
alternative scheduler surface.

### SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1 — non-interference

READER_V1 freezes the role matrix, the gate predicate, and wire errors — not a summary
field list. No section of it is amended; READER holders see `executionClass` like any
other summary field under the unchanged gate.

## 4. Current state (census, dd235dc)

- `workflow_instances` carries NO classification of any kind (whole-tree grep for
  work_class/execution_class/is_test/is_canary: zero hits). WorkEligibility is
  two-variant with BLOCKED-state invention forbidden by its accepted direction.
- Instance writers, whole tree: HTTP create transaction
  (`create_transaction.rs:373`), legacy import (`legacy_import_repository/
  transaction.rs:85`), and the out-of-production canary seeding script
  (`scripts/canary/seed_canary_test_data.sql:75,151`, which mints ZERO
  workflow_activations rows and is therefore dispatch-inert by construction).
- Instance create requires only domain membership
  (`validate_domain_membership`, create_transaction.rs:248) and accepts
  FIXED_PRINCIPAL entry assignees — an ordinary member can create work assigned to
  another principal, which is exactly why uncontrolled marking was rejected (Owner
  ruling B1).
- The enabled-DOMAIN_OWNER in-tx governance predicate is established repo practice:
  `cancel_transaction.rs:281-294` (wire family `not_domain_owner` :502),
  `archive_transaction.rs:264`, `query_visibility.rs:43`, `query_detail.rs:69`.
- The create path is receipt-first with durable deterministic-failure replay
  (`create_transaction.rs:89-130`); the canary write guard is an env-global write
  gate that classifies nothing (`canary_guard.rs:25-40`).

## 5. Observations

OBS-WEC-001: the automated business dispatcher's only svc-side ingress is the due
feed; work absent from the feed can never be admitted, mint a Run, or be woken into
the business path (wake only re-times intents; the feed never returns excluded rows).
OBS-WEC-002: the only supported producer of new test/canary instances is the HTTP
create surface; a governance-level marking authority on that surface therefore
covers the whole supported producer set (design §3 census, review-verified).
OBS-WEC-003: marking by an ordinary create caller would let legitimate work assigned
to OTHER principals be silently suppressed from dispatch — rejected by Owner ruling B1.

## 6. Claims

CLM-WEC-001 SUPPORTED: a creation-stamped, immutable, DB-owned class with a single
feed predicate conjunct structurally severs marked work from the automated business
dispatcher (OBS-001/002). CLM-WEC-002 SUPPORTED: DOMAIN_OWNER marking reuses the
exact existing lifecycle governance authority and cannot escalate (marking only
excludes the marker's own new instance from automated dispatch). CLM-WEC-003
SUPPORTED (design review round 2, PASS / 0 gaps): the amendment map above enumerates
every frozen surface the delta touches; nothing is narrowed silently.

## 7. Evidence relations

EVD-WEC-001 binds OBS-001/002/003 to CLM-001/002/003 at dd235dc + dsh b1fb7c0
coordinates via the frozen minimal design and its two-round independent review record
(dsh `8dc489e..e9699ae`, MINIMAL_DESIGN_REVIEW round-2 PASS / LOAD_BEARING_GAPS=0).
Evidence is design/census authority only — no test, receipt, or production claim.

## 8. Decisions

DEC-WEC-001: AUTHORITATIVE_OBJECT = the workflow instance; stamped once in the
create transaction; immutable thereafter; definition/version-level classification
rejected (one definition legitimately produces both business and controlled test
instances; a per-version class would add authoring semantics and a hotter feed join
for zero added safety). DEC-WEC-002: MARKING_AUTHORITY = enabled DOMAIN_OWNER of the
target domain, checked in-transaction with the exact cancel/archive predicate;
WORKFLOW_ADMIN and GLOBAL_WORKFLOW_COORDINATOR excluded (neither accepted scope covers
creation-time classification; no permission-magnitude derivation); canary env guard
not reused (classifies nothing). DEC-WEC-003: exactly two enum values;
BUSINESS is the universal default (forward-only compatibility; ZERO_SEPARATE_BACKFILL;
existing rows — including historical test garbage — are NOT reclassified; that
inventory belongs to WORKFLOW_DATA_HYGIENE_V1). DEC-WEC-004: the only dispatch delta
is the single AMENDMENT A conjunct; no consumer, protocol, gate, or row-shape change.
DEC-WEC-005: read-side closure is class-only positive visibility in the shared
summary struct (T11), wire name `execution_class` (snake_case summary convention;
create body stays `executionClass`), with the versioned contract amended
in-transaction (AMENDMENT C).
DEC-WEC-006: the broker companion delta (one optional create-argument passthrough +
summary field passthrough; zero authority logic in the broker) is REQUIRED and
declared now, because broker `workflow_execute.create_instance` is the normal
production create surface and the production tool face must close with the semantic.

## 9. Contracts

### CTR-WEC-001 — Classification fact and migration

One migration `0026_work_execution_class.sql`: `CREATE TYPE workflow_execution_class
AS ENUM ('BUSINESS','NON_BUSINESS_TEST');` and `ALTER TABLE workflow_instances ADD
COLUMN execution_class workflow_execution_class NOT NULL DEFAULT 'BUSINESS';`
EXPECTED_MIGRATION_VERSION 25 → 26. No backfill pass exists or is permitted; the
column default IS the compatibility story. The class is written exactly once, by the
instance-create transaction, and no UPDATE, cancel, archive, transition, recovery,
repair, import, or admin path may modify it. The fact is Workflow-DB-owned (no
cross-service semantics); canonical identity remains untouched.

### CTR-WEC-002 — Governed marking at create

`POST /internal/v1/workflow-instances` accepts an optional body field `executionClass`
(closed enum; unknown value → 422 `invalid_input` before any persistence). Absent or
`BUSINESS` ⇒ exactly today's behavior. `NON_BUSINESS_TEST` ⇒ the create transaction
additionally requires `EXISTS(SELECT 1 FROM domain_role_bindings WHERE domain_id =
<instance domain> AND principal_id = <caller> AND role_key = 'DOMAIN_OWNER' AND
enabled = TRUE)` — the identical in-tx predicate family as cancel/archive. Failure ⇒
403 `not_domain_owner` as a deterministic failure class on the receipt-first create
path (byte-identical retry replays the stored 403; changed body ⇒ idempotency
conflict; fresh key ⇒ fresh attempt), checked after request identity/schema
authorization and before the first runtime-fact write. The class applies at the single
shared create transaction for ALL semantic model versions; on non-model-3 instances
(which never write activation facts, CTR-VAI-012 lineage) it is classification-only.
No new role, grant, scope, or credential exists; the class field carries no identity
and selects no caller.

### CTR-WEC-003 — Due-feed narrowing (implements AMENDMENTS A+B)

`query_dispatch_intents` due selection adds the literal conjunct
`AND wi.execution_class = 'BUSINESS'` and changes nothing else: same ordering
expression, same keyset filter, same 7-field projection, same GLOBAL_SCHEDULER_READ
in-snapshot gate, same zero-write read semantics. Excluded rows are absent from every
business consumer view; a wake against a NON_BUSINESS_TEST activation remains a
receipted 200 no-op-or-eligibility-event whose row can still never re-enter the
business feed (class immutability makes the exclusion permanent).

### CTR-WEC-004 — Class-only positive visibility (implements AMENDMENT C)

`DomainInstanceSummary` (shared by the domain and global list surfaces) exposes
`execution_class` (snake_case wire, per AMENDMENT C's two-convention split) and
nothing else new: no private detail, no cross-domain widening, no eligibility/blocked
semantics, no second scheduler subject. Worklists, detail visibility, and all other
read surfaces are unchanged.

### CTR-WEC-005 — Ingress dispositions and forbidden authorities

Supported ingress = the HTTP create surface only (AUTHORIZED_TEST_CREATOR_MUST_
EXPLICITLY_MARK under CTR-WEC-002). Legacy import stays out of scope (creates
BUSINESS; writes no activation facts). The canary seeding script stays out of
production scope; any future reuse MUST stamp `NON_BUSINESS_TEST` in its INSERT
statements directly. SERVER-FORCED classification is unused (no accepted mechanism).
FORBIDDEN everywhere: name/substring/title/domain-key inference; a third enum value;
a generic taxonomy framework; any blocked state; any retry of excluded work; any
second dispatch gate.

### CTR-WEC-006 — Broker companion delta (declared counterpart)

The dsh-side implementation closure adds one optional passthrough argument
(`executionClass`) to broker `workflow_execute.create_instance` and the
`execution_class` summary field to the domain/global summary passthroughs. The broker
performs zero authority logic — svc enforces CTR-WEC-002 and fails closed; denials
preserve the exact svc error family through the broker per AGENT_CORE_WORKFLOW_BROKER_
ERROR_PRESERVATION_V1. This Spec's acceptance does not authorize the dsh delta; the
dsh counterpart follows that repository's own governance, referencing this Spec as its
svc-side authority.

## 10. Acceptance mapping

All rows require executed evidence bound to this Spec's exact accepted head and the
implementation head; tests alone do not prove deployment. `production_apply_authority:
none` — production activation is a separately gated deployment step.

| ACC | Contracts | Method / environment | Expected / failure condition |
|---|---|---|---|
| ACC-WEC-001 | CTR-WEC-001 | migration rehearsal on isolated DB; information_schema readback (T5d) | enum + column exist with NOT NULL DEFAULT 'BUSINESS'; all pre-migration rows read BUSINESS (T5d); EXPECTED_MIGRATION_VERSION = 26; zero row rewrites |
| ACC-WEC-002 | CTR-WEC-002 | T5a/T5c/T5e/T5f/T5g/T5k matrix, isolated | owner+mark ⇒ NON_BUSINESS_TEST persisted; member/admin/coordinator/cross-domain mark ⇒ 403 replayed deterministic zero-delta; unknown value ⇒ 422 zero-delta; unmarked ⇒ byte-identical legacy behavior |
| ACC-WEC-003 | CTR-WEC-003 | T5a/T5b/T5h/T5j, isolated with due and future intents mixed | marked+due row absent from feed; marked+wake ⇒ 200 receipt and still absent; T5h: the SAME marked instance remains present in its assignee worklist and its transition stays executable (targeted path intact, zero worklist filtering) while business-feed presence stays zero; keyset cursor walk over a class-mixed window returns the identical BUSINESS row sequence and key immobility holds |
| ACC-WEC-004 | CTR-WEC-004 | T5i + openapi/conformance suite | summaries expose execution_class on both surfaces; READER (non-scheduler) sees it under the unchanged gate; refreshed openapi validates; conformance digests updated |
| ACC-WEC-005 | CTR-WEC-005 | T5m + seed-script guard test | seed-script and import rows read BUSINESS and produce zero feed entries; grep proves no heuristic classifier exists in the closure |
| ACC-WEC-006 | CTR-WEC-006 | T5l, dsh-side closure tests (its own governance) | broker absent-arg ⇒ BUSINESS; owner+arg ⇒ NON_BUSINESS_TEST; non-owner+arg ⇒ 403 family preserved through broker envelope |

Coverage: CTR-WEC-001..006 all covered; 6/6.

## 11. Alternatives and disposition

ALT-WEC-001 (marking by any workflow.execute caller) — REJECTED by Owner ruling B1:
member-created work assigned to other principals could be suppressed from dispatch.
ALT-WEC-002 (definition/version-level class) — REJECTED: wrong authoritative object;
one definition legitimately yields both classes; extra feed join, new authoring
semantics, zero added safety. ALT-WEC-003 (free-text metadata convention) — REJECTED:
not machine-enforced, not immutable, not an admission authority. ALT-WEC-004
(GLOBAL_WORKFLOW_COORDINATOR as marking authority) — REJECTED: its accepted scope is
domain management/control-plane, not creation-time classification; including it would
derive authority from permission magnitude, which the ruling forbids. ALT-WEC-005
(work_class as a third state or blocked classification) — REJECTED: collides with the
accepted NO-BLOCKED-state direction and expands the taxonomy the ruling capped at two
values. ALT-WEC-006 (filtering in the dsh consumer instead of the svc feed) —
REJECTED: consumer-side narrowing is not structural (other consumers, bypassable),
and it would change the frozen WAE protocol rather than the feed predicate.

## 12. Migration, compatibility and rollback

One additive migration (0026). Rollback before facts exist = restore code preimage
(column may remain, inert). Rollback of the AMENDMENT A conjunct after deployment
RE-ADMITS existing NON_BUSINESS_TEST rows to the business feed until the fix
redeploys — the class column itself is data-inert in both directions (no data
migration is ever required), but the exclusion invariant lives in the query, so
reverting the query reverts the guarantee; containment therefore targets the feed
conjunct, never row rewrites. "Deployment order unconstrained for THIS delta" is
compatible with AMENDMENT B's CTR-WAE-001b preservation because the conjunct adds no
new ordering constraint — an excluded row can never be the mid-sweep-consistent
return of business work — and the keyset build's svc-before-poller ordering continues
to govern unchanged.

## 13. Open questions

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE (AMENDMENTS A/B/C name their authorities and preserve everything except the exact declared deltas; the named-amendment form follows the accepted coordinator-control-plane precedent)
PARTIAL_SUPERSESSION = NONE (supersedes = []; VISIT_ACTIVATION_IMPL_V1 and DISPATCH_INTENT_KEYSET_CONTINUATION_V1 survive in full except the declared conjunct/field)
IMPLEMENTATION_READY = NO (activates on acceptance with implementation_authority: contracts)
PRODUCTION_READY = NO
```
