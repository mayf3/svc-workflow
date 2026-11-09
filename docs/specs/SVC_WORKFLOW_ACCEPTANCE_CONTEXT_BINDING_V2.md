---
spec_id: SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V2
title: Accepted whole-authority successor to SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1
status: accepted
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
repo: mayf3/svc-workflow
scope:
  - opt-in submission references to the same instance acceptance Context
  - bounded managed gated-entry Context revision
date: 2026-10-29
revision: proposal-1
candidate_base: e8cc025341abb338e1a636c5a6d7e92aab77af69
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V13
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_7
paired_authority: AGENT_CORE_WORKFLOW_OWNER_OBLIGATION_WORKFLOWS_V2
external_authorities: []
supersedes:
  - SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1
superseded_by: null
owners:
  - mayf3
accepted_proposal_head: 39587c33c8bcf26ad5689127f5c92a6abc8b562e
accepted_proposal_spec_sha256: 15c17ae6c3720edc4f76bfc6e7d6bca736a4ee9b6ce67f8caec060df185ded0d
acceptance_record: ART/node-context-producer-spec-20261029/ROOT_OWNER_ACCEPTANCE.json
acceptance_record_sha256: f9fd9a3e8d5578dc9a1a309c0290f975fff8bb68861c3644eb02166616d572a4
---

# SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V2 — accepted whole-authority successor

**Accepted whole-authority successor.** Root acceptance under the user-delegated authority, recorded in `ROOT_OWNER_ACCEPTANCE.json` (SHA-256 `f9fd9a3e8d5578dc9a1a309c0290f975fff8bb68861c3644eb02166616d572a4`), binds reviewed semantic heads Svc `39587c33c8bcf26ad5689127f5c92a6abc8b562e` and Core `ac83fe5cea3701eb9436f74d1b5f6ed0013256a9`. This lifecycle-only transaction records accepted status and reciprocal direct-predecessor links; the reviewed semantics are unchanged. `implementation_authority: contracts` authorizes source implementation only within these Contracts after this paired lifecycle commit pair is merged to the relevant authority main and the accepted revisions are present in the implementation base. `production_apply_authority: none`; no production apply, migration execution, deployment, runtime changes, or A75 mutation is authorized. No implementation or runtime conformance is claimed. The carried predecessor body remains historical.

## ACB V2 bounded successor delta

This successor preserves every ACB V1 Contract and rejected alternative except the explicitly bounded new-flow Context exception. CTR-ACB-002 now states that create-time immutability has one managed gated-entry append; CTR-ACB-007 fixes its exact scope, JCS preservation, producer validation, source Submission and Event lineage, receipt/version/event cardinality, error mapping, absence/null compatibility, and future-report UUID rule. The existing `/acceptanceCriteria` catalog does not change during this append. No report accepted for the previous Context UUID is copied, migrated, or treated as valid for a later revision. Existing model3 combined/revise/revise-and-transition rejection remains 422.

## Complete predecessor body carried forward from `SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1`

# SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1

## 1. Goal

One reusable Definition Version serves different Instances' acceptance catalogs.
A designated successful review ADVANCE must refer to the exact current Context
and every acceptance ID in that Instance, rather than an Agent-selected list.
This checks reference integrity and declared results; it does not decide whether
evidence proves those results. The Owner accepted the exact independently
reviewed proposal; see the [acceptance record](../reports/ACCEPTANCE_CONTEXT_BINDING_V1_ACCEPTANCE.md).
This is accepted on the branch; implementation still requires accepted authority
in its base, and production permission remains absent.

## 2. Scope and non-goals

In scope: a closed opt-in annotation, catalog shape, transaction-local reference
validation, existing failure Receipts, focused compatibility and downgrade gates.
The first slice supports model3 TASK primary ADVANCE only, using existing APIs.
This is this V1 proposal's deliberate scope choice, not a claim that the
production cutover barrier has occurred. Architecture §5.13/§10.2 makes the
no-new-Legacy rule post-barrier; pre-barrier Legacy operations retain their
existing accepted authority, which this Spec neither expands nor narrows.

Out of scope: new Context mutation, business truth/evidence authenticity,
artifact fetching, new roles or grants, new ledgers/tables, arbitrary guards,
per-task Definition constants, changes to existing Instances, new Legacy intake,
and production deployment. PR #86's timestamp proposal is unrelated and remains
outside this authority.

## 3. Authority and dependencies

All local parents are pinned at main
`bfc28c15320acab8cf7d7a36128cd5c9db9a1b1a` (2026-10-27): Product V8
§§5.2–5.5, Architecture v0.4.3 §5.5 / CTR-ARCH-005/008/040, and accepted
`SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1` CTR-VAI-001/005/011/012.
The retained `DEFINITION_SERVICE_CONTRACT_V0_1` §§3.7/4 and
`WORKFLOW_TRANSITION_CONTRACT_V0_1` §§3/6 retain their declared
IMPLEMENTATION_CONTRACT scope: schema/digest, locks, lineage and Receipts.

`AUTHORITY_ACTION=NEW`: a new optional typed reference contract, not a new
meaning for ordinary unannotated submissions. Parent shape/digest and
same-Instance integrity remain intact: the validator compares local facts and
literal declarations, not business truth. It adds no node kind, script guard,
Context editing privilege or evidence-approval authority. No accepted authority
is partially superseded. Independent review must test this interpretation; a
parent meaning change requires RE_PREFLIGHT and the proper successor, not this
child's acceptance by implication. Implementation requires this exact Spec
accepted in its base; `production_apply_authority` remains `none`.

## 4. Current State

**STATE-ACB-001**, as of the pinned main, static source review on 2026-10-27:
dynamic catalog binding is absent. Existing JSON Schema validates an isolated
payload; Submission is separately bound to the locked Instance's Context UUID.
No implementation or runtime acceptance for this proposal has run (OBS-ACB-001).

## 5. Observations

- **OBS-ACB-001** — At the pinned main,
  `transition_transaction.rs:470–475` passes only schema and submitted payload
  to `transition_validation.rs:343–363`; `:597–608` records authoritative
  `current_context_revision_id`. `definition/lifecycle/validation.rs:270–317`
  compiles schemas and restricts references; publication digest covers the full
  schema. Method: read-only source/contract inspection in the isolated checkout.
- **OBS-ACB-002** — Existing test definitions cover required/type/schema errors,
  wrong-version transitions, CAS, idempotency and Context lineage under
  `tests/17_workflow_runtime/`. They were read, **not executed** in this task.
- **OBS-ACB-003** — The prior isolated a1 Create returned HTTP 201 while its
  internal Receipt recorded 200; harness mismatch did not authorize repeating
  Create. Preserved dsh-agent-core artifact:
  `docs/evidence/workflow-governance-closure-20261026/weak-model-failure-cases-audit-20261027.json`,
  SHA256 `01f85ab2dcd3260ba20a6bef8f45c282d5dc21271893bc5ad845f5a311a20c0f`.
  This is non-authoritative historical harness evidence, not this feature's test.

## 6. Claims and assumptions

**CLM-ACB-001 (SUPPORTED)** — Schema constants can enforce one shared catalog,
but do not dynamically compare each Instance's Context with its submission.
Reference validation is needed for the reusable per-instance goal (EVD-ACB-001).

**CLM-ACB-002 (INFERRED)** — An annotation inside the fully digested submission
schema is a bounded zero-migration carrier. Old binaries ignore it, making the
runtime/rollback gate necessary (EVD-ACB-002). No live inventory is asserted.

## 7. Evidence relations

- **EVD-ACB-001**: OBS-ACB-001/002 → SUPPORTS CLM-ACB-001 and STATE-ACB-001,
  at the pinned source/test coordinates. Sufficient to identify the code gap;
  insufficient to claim deployed behavior or executed rejection tests.
- **EVD-ACB-002**: OBS-ACB-001 → SUPPORTS CLM-ACB-002. Full schema digest and
  the payload-only compiler support the carrier and old-runtime risk; future
  build-bound conformance and runtime inventory remain required.
- **EVD-ACB-003**: OBS-ACB-003 → SUPPORTS separate HTTP/Receipt recording in
  ACC-ACB-005. Limited to the historical harness failure; not new product proof.

## 8. Decisions

**DEC-ACB-001 — Owner choice:** use one shared model3 Definition with
per-Instance immutable `acceptanceCriteria`, versioned by the server's actual
Context UUID. Reserve one closed annotation; require complete all-PASS reports
only on explicitly configured successful ADVANCE. RETURN/TERMINATE remain
available under their own contracts. No new endpoint or database migration.
These choices were accepted by Owner mayf3 at the exact proposal head identified
in the acceptance record; the author records that decision without changing it.

## 9. Contracts

### CTR-ACB-001 — Explicit publication opt-in

The reserved key `x-workflow-acceptance-binding-v1` MUST appear only at the root
of a Transition's non-null `submissionSchema`, with exactly this value:

```json
{
  "contextCatalogPointer": "/acceptanceCriteria",
  "submissionReportPointer": "/acceptanceReport"
}
```

These are literal, fixed JSON Pointers, not expressions, external references or
caller-selected paths. Validation and publication MUST reject malformed values,
unknown members, nested occurrences, and use anywhere except a model3 TASK's
primary ADVANCE. A DRAFT **source node kind** (not the draft publication lifecycle),
non-model3, RETURN and TERMINATE opt-ins are rejected.
The full annotation remains inside the existing canonical Definition digest;
published immutability and existing publish CAS/Owner authorization apply.
Absence means existing behavior. A malformed present annotation never means
absence. This Spec authorizes no in-place editing of published schemas.

Definition authors MUST place the opt-in only at a complete-acceptance boundary
where the entire Context catalog is due to be satisfied. An early QA or merge
step MUST NOT require all-PASS for catalog obligations that belong to future
deployment or later work. Configuration review establishes this placement; the
kernel does not infer stages from node names. V1 has no phased/subset-catalog
engine and does not waive future obligations to make an early gate pass.

### CTR-ACB-002 — Catalog identity and lifecycle

For a Definition with an opt-in transition, formal Context at
`/acceptanceCriteria` MUST be an array of 1–128 objects, each with exactly `id`
and `description`. IDs MUST fully match `[A-Za-z0-9][A-Za-z0-9_-]{0,63}` and be unique
by exact case-sensitive value. Descriptions MUST be nonblank strings of at most
4096 UTF-8 bytes. Existing total payload limits also apply. Creation MUST check
this after ordinary Context Schema validation and before workflow-fact writes;
malformed/empty/duplicate catalogs fail with existing `context_validation_failed`
422 and existing deterministic failure Receipt semantics.

The catalog is workflow input supplied through existing creation authority,
not proof of Owner approval of its substance. Independent governance review
must establish that substance; this gate never substitutes for it. Model3 Context remains create-time immutable except for the exact transition-entry Context append in ACB V2 / Product V13: the existing managed INTAKE ADVANCE or RETURN into a declared gated TASK may add the verified real `workflowPrerequisite` binding as a new immutable revision. This does not edit an existing revision or change the acceptance catalog. This slice adds no Agent permission or API to edit standards, no Legacy revise or repair permission, and no general Context lifecycle. Metadata or report-provided catalogs MUST NOT replace formal Context.
The exact Context UUID is the binding version: it cannot be replaced with a
Definition version number, client version string or artifact SHA. A report
cannot transfer to any different current Context UUID, regardless of equal IDs.

### CTR-ACB-007 — Gated-entry Context revision and report identity

The only post-create model3 Context append permitted by this successor is the existing managed INTAKE transition that enters a declared `workflowPrerequisiteGate` TASK by ADVANCE or RETURN. An optional transition `contextPayload` MUST be present as an object (null/non-object is HTTP 422 `invalid_input`) and MUST differ from the current Context only at the exact four-field `workflowPrerequisite` object, compared by JCS. Svc verifies the four fields against the exact target gate's real earlier producer effect, original effect/create receipt and result, actual created Instance, pinned DefinitionVersion and selected terminal Transition inside the existing transition transaction. The bound supplier may be WAITING; its eventual terminal Event/Submission/digest remains the later live-readiness proof. A scope violation uses existing 422 `invalid_input`; invalid schema/JCS/producer facts use existing 422 `context_validation_failed` with a specific rule.

The current transition's source Submission, ACB evaluation, and Event `context_revision_id` remain bound to the pre-transition Context. The new Context is a fresh immutable revision and its UUID is the current Context returned after transition; the existing transition EventData carries actual `previousContextRevisionId` and `newContextRevisionId` only when a new revision was appended. The catalog at `/acceptanceCriteria` and all other JSON is preserved; the report for every later gated-node acceptance MUST bind the new UUID. A prior report is never migrated or credited against the new UUID. Omission preserves the exact old request-hash/result route; if effective Context is unchanged, no revision/EventData pair is added. No new command, receipt, Event, permission, role, table, or ACB bypass is introduced.

### CTR-ACB-003 — Atomic successful-ADVANCE predicate

After the existing Instance lock, authorization, state CAS and transition gates,
and before any workflow-fact write, ordinary Transition execution MUST:

1. Resolve the exact stored transition/schema under `instance.definition_version_id`;
   apply ordinary submission-schema validation and CTR-ACB-001 configuration checks.
2. Fetch Context by the locked Instance's `current_context_revision_id` **and**
   Instance ID in the same transaction; recheck CTR-ACB-002 catalog validity.
3. Require `/acceptanceReport` to be an object containing `contextRevisionId`
   (the canonical UUID string of that exact current Context) and `results` (array).
4. Require every result to contain a string `id` and literal `verdict="PASS"`;
   result IDs MUST equal the catalog ID set exactly once each. Order is irrelevant;
   missing, unknown, substituted or duplicate IDs fail. Existing submission Schema
   governs any additional report/result/evidence fields; the binding validator
   does not interpret their truth or fetch their resources.

No client-supplied catalog, Context payload copy or UUID alone is authority.
Two Instances with identical catalogs still cannot exchange reports. Successful
Submission MUST retain the existing server-set Context FK, author, transition,
source Visit, payload/digest, Event and Receipt; no second acceptance ledger is
created. A true `PASS` remains the reviewer's evidence-backed responsibility.
For example, a correct Context UUID, complete IDs and declared PASS can still
carry only canary evidence for a C2 that requires regression testing. This may
pass the structural gate; independent QA MUST NOT infer substantive business
PASS from it. UUID/ID binding refers to immutable descriptions but does not
validate evidence meaning and does not claim to prevent 8121's original same-ID
semantic/evidence substitution.

### CTR-ACB-004 — Failure and bypass closure

Missing payload retains existing `submission_required` 422. Present but invalid
configuration/catalog/report/reference/result fails as existing
`submission_validation_failed` 422, preserving the existing public error envelope
without leaking Context text. Missing/corrupt authoritative Context linkage
remains an internal consistency failure, never an unchecked success.

For an annotated definition, no ordinary command may evade the check by calling
combined revise-and-transition: publication disallows its DRAFT/model scopes,
and the repository's combined transaction MUST fail closed if the selected
persisted schema, resolved under that Instance's exact Definition Version,
contains the reserved annotation at any depth, including malformed/null values.
Existing model3 legacy-command rejection stays
in force. RETURN/TERMINATE contain no annotation and keep their own schemas,
reference validation, authorization and failure semantics; they do not require
this report or all-PASS. Cancel/admin paths retain their existing distinct
authority and MUST NOT be represented as successful acceptance ADVANCE.

### CTR-ACB-005 — Transaction, CAS and outcome discipline

Binding failure MUST commit no Context, Submission, Visit, Event, activation or
Instance-state delta. Existing deterministic failure Receipts may persist; they
are not business progress. Storage/commit failure preserves existing rollback
and UNKNOWN semantics. Existing request hashing, same-key same-payload replay,
same-key changed-payload conflict, and state CAS remain authoritative; successful
idempotent replay returns the original result without re-executing the gate or
creating new facts. No retry with a new key follows an UNKNOWN outcome before
reconciliation. HTTP status and internal Receipt status are recorded separately.

### CTR-ACB-006 — Compatible rollout and safe downgrade

Unannotated versions and historical submissions MUST retain existing behavior.
No migration or retroactive business-acceptance claim is permitted. Before any
opt-in version is published/used, the operator MUST bind every serving runtime
to an implementation/build whose executed conformance covers this contract,
and inspect existing versions for the reserved key. Unexplained existing use
blocks activation; this document is not permission to reinterpret it silently.
An unverified, old or mixed runtime fleet MUST NOT receive opt-in traffic.

Because old binaries ignore this annotation, downgrade is allowed only after
ordinary execution of **all** opt-in versions is durably blocked through existing
authorized controls, such as REVOKED, with readback proving the old binary also
rejects those versions. DEPRECATED, disabling only new creation, or read-only
schema compatibility is insufficient. If this cannot be proved, do not downgrade;
use authorized containment. Preserve all Instances/history and assigned work
responsibility; blocked work is not completed. Re-enable only under a supported
verified runtime and existing lifecycle authority, never by removing annotation
or mutating history. This is a compatibility obligation, not deployment authority.

## 10. Acceptance

Every row below is **NOT_RUN**. Use an isolated PostgreSQL environment and exact
candidate source/binary; live canaries require a separate operation mandate and
existing DOMAIN_OWNER rights for `NON_BUSINESS_TEST`. Record requests, independent
HTTP and Receipt results, IDs/digests, and before/after workflow-fact counts/state.
The common failure condition is any expected rejection committing business facts,
any accepted wrong reference, or a claimed result without executed evidence.

| Acceptance / Contracts | Method and inputs | Expected result / distinguishing failure |
|---|---|---|
| ACC-ACB-001 / CTR-001 | Review catalog obligations against the chosen complete-acceptance boundary; publish annotation matrix: valid root value; changed/extra/missing pointer; nested key; DRAFT, model1/2, nonprimary, RETURN, TERMINATE; digest comparison. | Configuration review rejects a gate that demands future deployment obligations at early QA/merge; only valid model3 primary ADVANCE publishes; digest binds exact annotation; malformed or ignored annotation fails the contract. |
| ACC-ACB-002 / CTR-002 | Actual Create with valid catalog, missing/empty array, empty/invalid ID, duplicate ID, empty/oversize description, extra catalog member; verify metadata cannot substitute. | Valid input creates one immutable Context; every malformed catalog rejects with zero workflow-fact delta and retained failure Receipt as applicable. |
| ACC-ACB-003 / CTR-003 | Two Instances, same Definition and same IDs, different Context UUIDs; then a third different catalog. Submit correct report, other Instance UUID, arbitrary cached/incorrect UUID, omitted UUID, missing/replaced/unknown/duplicate ID, and non-PASS verdict. Include correct UUID/IDs/PASS with schema-valid but semantically wrong evidence, as in the C2 example. | Only exact current UUID plus exact ID coverage/all-PASS advances. The wrong-evidence example may pass this structural gate; independent QA must not infer business PASS or claim the original 8121 same-ID substitution is prevented. Incorrect/foreign UUID tests do not imply model3 can revise Context; no real Context-change test is invented. |
| ACC-ACB-004 / CTR-004 | Call combined with annotation using an isolated malformed/drift fixture, and the legal model3 command; after a rejected report, execute valid RETURN with its required reference fields; exercise configured TERMINATE separately. | Combined never commits; ordinary RETURN/TERMINATE remain usable without all-PASS; normal authorization/reference checks still hold. A fixture is not new production Legacy authority. |
| ACC-ACB-005 / CTR-005 | Stale/future CAS; concurrent same-state requests; success and deterministic-failure same-key replay; changed payload same key; injected storage/commit interruption and Receipt reconciliation. | At most one state command commits, version/Event increments once; replay creates no facts; changed payload conflicts; UNKNOWN never triggers blind new-key execution. |
| ACC-ACB-006 / CTR-006 | Existing unannotated regression; pinned old binary in isolated environment; support/mixed-runtime activation gate; then block every annotated test version and attempt old-binary ordinary execution. | Unannotated behavior retained; old-runtime exposure blocks activation; after durable containment old binary rejects all annotated versions. Health/readback alone is insufficient. |
| ACC-ACB-007 / CTR-ACB-007 | Managed INTAKE enters multiple declared gated TASKs through ADVANCE/RETURN with exact successful producer effect; omitted, null/non-object, altered catalog, stale/foreign producer, changed-body replay, and source Submission/ACB/Event references. | Only omitted or exact valid object can follow its declared route; success uses one old transition receipt/Event/version; new revision only on effective change; later ACB reports require new Context UUID; old report is not migrated. |
Coverage is seven Contracts mapped to seven Acceptance rows, with ACC-ACB-003/004/005
also checking the failure-delta rule. Required evidence includes exact published
Definition digest, same-version multiple-Instance readbacks, Context/Submission
lineage, executed command output/exit status, and independent review. Test source,
schema compilation, generic build health or author assertion cannot mark PASS.

## 11. Alternatives and disposition

- **ALT-ACB-001 — Per-task `const` schemas:** rejected for this goal; creates
  per-task versions or changes the shared template's catalog.
- **ALT-ACB-002 — Agent-only precheck or metadata convention:** rejected as a
  server invariant; direct Transition calls can bypass it.
- **ALT-ACB-003 — General script/JSON-expression guard or new ledger:** rejected;
  unnecessary permission, storage and policy-engine expansion.
- **ALT-ACB-004 — Add model3 Context revision now:** deferred to separate authority;
  not necessary to bind reports to create-time standards.

## 12. Migration, compatibility, and rollback

No database migration, automatic adoption, active Definition rewrite or existing
Instance rebinding. After acceptance and implementation, a separately authorized
new model3 Definition Version may opt in while reusing the existing governance
workflow pattern. Existing Instances retain their exact prior version and
responsibility. CTR-ACB-006 governs activation/downgrade; no production operation
is authorized here. Superseded binaries must not reopen this guard accidentally.

## 13. Open questions

```text
OPEN_OWNER_DECISIONS = NONE (exact-head Owner acceptance recorded)
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE identified by the exact-proposal independent review
PARTIAL_SUPERSESSION = NONE
IMPLEMENTATION_STATE = NOT_IMPLEMENTED
VERIFICATION_STATE = NOT_RUN
ACCEPTANCE_STATE = ACCEPTED_ON_BRANCH
FINAL_ACCEPTED_HEAD_RECHECK = PENDING
ACTIVE_ON_MAIN = NO at this lifecycle transaction; activation requires merge
PRODUCTION_APPLY_AUTHORITY = NONE
```
