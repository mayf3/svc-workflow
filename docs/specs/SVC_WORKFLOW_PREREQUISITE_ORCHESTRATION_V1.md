---
spec_id: SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1
title: Atomic governance-child creation and typed workflow prerequisite readiness
status: accepted
accepted_date: 2026-10-29
accepted_by: Codex root (explicitly delegated by mayf3)
accepted_reviewed_head: 19593b4dc4d48b8341c8a759d07b9e575afeb08e
acceptance_review_verdict: PASS
independent_semantic_reviewer: /root/structure_v2_semantic_review
acceptance_delta_class: new_standalone_contract
semantic_delta_after_review: NONE
acceptance_record: docs/reports/WORKFLOW_PREREQUISITE_ORCHESTRATION_ACCEPTANCE_20261029.md
acceptance_authority_basis: >-
  Root accepted the exact independently reviewed semantic head under mayf3's
  explicit delegation to complete source work and isolated validation only.
  The final semantic review SHA-256 is
  4baa1f24202e3d9f1544d0ebd95eb5dc42ce9febedb243c45819f751aa09204f. This is
  the paired lifecycle acceptance; implementation still requires both
  accepted authorities in the implementation base and a separate mandate.
  Production apply authority is none.
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
repo: mayf3/svc-workflow
date: 2026-10-29
revision: r1
candidate_base: c8937e567a429160779ee7db0cb7b53efbe8ed3e
scope:
  - src/http/handlers/assistance.rs
  - src/application/workflow_instance/assistance.rs
  - src/store/postgres/workflow_instance_repository/create_transaction.rs
  - src/store/postgres/workflow_instance_repository/assistance_transaction/governance_child/
  - src/store/postgres/workflow_instance_repository/query_dispatch_intents.rs
  - src/store/postgres/workflow_instance_repository/current_task_input/
  - src/http/handlers/dispatch_intents.rs
  - src/http/handlers/current_task_input.rs
  - src/http/handlers/instances.rs
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V9
  - SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V1
  - SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
  - SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V2
supersedes: []
superseded_by: null
owners:
  - mayf3
external_authorities: []
---

# SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1

This accepted Contract authorizes a later bounded Svc implementation of (1) an
atomic GCR child create-and-bind command, (2) one explicit immutable Context
prerequisite referencing an already-existing same-Domain WorkflowInstance and
its exact terminal Submission transition, and (3) typed readiness and a
READY-only scheduler projection. It applies to ordinary `create_instance`
workflows and GCR children alike. The paired Core Spec is required. This
candidate creates no production permission and makes no claim that an
indefinitely waiting or closed workflow will recover automatically.

## 1. Goal

Provide one atomic GCR child create-bind operation and one shared, typed
prerequisite readiness projection so ordinary workflows and GCR children wait
on an exact, already-existing same-Domain upstream terminal Submission without
creating an Agent attempt before dispatch is eligible.

## 2. Scope and non-goals

Scope is the existing workflow create/GCR transaction boundary, immutable
Context Schema validation, authorized Instance/current-task reads, and a
separate scheduler due projection. This is one fixed
`TERMINAL_SUBMISSION` dependency type. Non-goals are an arbitrary dependency
DSL, selector/policy registry, prerequisite table, wait ledger, timer, outbox,
new role/scope/Grant, new recovery/escalation flow, historical migration,
business-truth validation, or production apply.

## 3. Authority and dependencies

This is cross-repository `ASSURANCE_LEVEL=CONTROLLED`; route stage is
`AUTHORITY_AUTHORING`; the parent successors and paired implementation Specs
are accepted in these candidate branches and become effective after merge. The
parent-authority route is `AUTHORITY_ACTION=SUPERSEDE` for whole successors Product V9,
Architecture V0_4_4, and WEP V2. The separate additive implementation-child
route is `AUTHORITY_ACTION=NEW` for Svc prerequisite orchestration V1 and paired
Core prerequisite orchestration V1. No child route partially supersedes a
parent, and no route authorizes implementation before exact acceptance in the
implementation base plus a separate mandate. Local parents are accepted Product V9, Architecture V0_4_4, GCR V1, VAI V1,
and WEP V2. Paired Core
`AGENT_CORE_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1` is required; the paired
acceptance record must bind both exact reviewed companion heads. Acceptance in
a docs-only branch does not activate implementation authority; production apply
stays none.

## 4. Current State

`STATE-SWPO-001` — At Svc candidate base
`c8937e567a429160779ee7db0cb7b53efbe8ed3e`, GCR V1 accepts only
pre-created-child binding; VAI V1 defines a time-only due-intent read; WEP V1
defines only time eligibility. No accepted local Contract defines
cross-instance prerequisites or a READY-only dispatch projection. No
implementation or runtime acceptance for this proposal has run.

## 5. Observations

- `OBS-SWPO-001` — Read-only inspection of GCR V1 §1 at the pinned Svc base:
  the current bind accepts an existing child and excludes unattended
  create-before-bind orchestration.
- `OBS-SWPO-002` — Read-only inspection of VAI V1 CTR-VAI-009 at the pinned
  Svc base: the raw due read returns active, time-due rows in due-time order,
  without a prerequisite filter.
- `OBS-SWPO-003` — Read-only inspection of WEP V1 §§2–4: the canonical
  classification is `ACTIONABLE_NOW | WAITING_FOR_TIME`, read-only, and
  explicitly has no `BLOCKED` state or cross-instance input.
- `OBS-SWPO-004` — Read-only inspection of accepted ACB V1 CTR-ACB-001/002:
  its opt-in binds a submission report to the same Instance's Context catalog;
  it does not define cross-instance dependency semantics and is not reused as
  a carrier here.

No database, service, HTTP, model, or runtime was invoked for these
observations.

## 6. Claims and assumptions

- `CLM-SWPO-001` (SUPPORTED) — Existing immutable Context plus exact
  Instance/Version/Transition/Event/Submission rows are sufficient inputs for
  read-time readiness without a new dependency ledger. Supported as a design
  inference by `OBS-SWPO-001/002` and VAI V1's canonical activation facts.
- `CLM-SWPO-002` (SUPPORTED) — Existing `GLOBAL_SCHEDULER_READ` can guard only minimal due/readiness-status coordinates. The bounded receipt projection is explicitly authorized by accepted Product V9 as a same-Domain permission for authorized dependent assignee/Owner readers; Product V8 did not grant it. It exposes no source payload. Supported by the existing scheduler and per-Instance read boundaries, with the narrow permission delta stated explicitly.

Assumptions to validate during implementation: the immutable published
Context Schema can express the root opt-in annotation without reinterpreting
existing business fields; and existing create/GCR transaction helpers can be
composed under one SERIALIZABLE transaction. Failure means stop and revise,
not weaken the Contract.

## 7. Evidence relations

- `EVD-SWPO-001` — `OBS-SWPO-001/002/003` support `STATE-SWPO-001` and
  `CLM-SWPO-001` at the exact pinned authority revisions. Sufficient to
  identify the gap; insufficient to assert runtime readiness behavior.
- `EVD-SWPO-002` — `OBS-SWPO-004` supports the decision not to reuse ACB's
  same-Instance report carrier for this cross-Instance Contract.
- `EVD-SWPO-003` — `OBS-SWPO-002` and VAI V1 support reuse of the existing
  scheduler authorization and time eligibility as separate gates; no new
  permission is proposed.

## 8. Decisions

- `DEC-SWPO-001` — Add a new Svc authority; do not amend GCR, VAI, WEP or ACB.
- `DEC-SWPO-002` — Use one fixed, root-schema-opted-in
  `TERMINAL_SUBMISSION` Context reference to a preexisting same-Domain
  Instance/Version/Transition. Do not predict a future Submission ID.
- `DEC-SWPO-003` — Derive `READY | WAITING | CLOSED | UNKNOWN` from Svc-owned
  facts on each normal poll/read; add no second wait truth or wake writer.
- `DEC-SWPO-004` — Keep the existing raw VAI time-only due API unchanged and
  add a separate READY-only projection filtered before pagination.

## 9. Contracts

### CTR-SWPO-000 — Existing-authority boundary

The paired parent-authority acceptance mapping for this Contract is Product
V9 `DEC-V9-001/002`, Architecture V0_4_4 `CTR-ARCH-041/042/043`, and WEP V2's
selected time-projection exception. The `CTR-SWPO-000` row of `ACC-SWPO-001` must review these exact identities as one bounded parent delta; it must not cite the retained
Architecture `CTR-ARCH-019/020/021` IDs, whose predecessor meanings remain
unchanged.

GCR V1 accepts only an existing child and explicitly leaves atomic
create-and-bind outside its scope. Its current same-Domain/current-Owner,
schema, one-child, serialization, receipt, and unknown-readback predicates
remain binding. VAI V1 CTR-VAI-009 defines a time-only raw due-intent read;
WEP V1 exposes time eligibility without a dependency `BLOCKED` state. Neither
contract defines cross-instance prerequisite semantics. This Spec adds a new
authority and preserves those existing meanings and response shapes.

The design reuses immutable create-time Context, published Definition/Context
Schemas and digests, existing Instance/Visit/Event/Submission facts, current
Owner and membership checks, existing idempotency receipts, and the ordinary
poll path. It adds no prerequisite table, wait ledger, retry counter, timer,
outbox, new role, scope, Grant, or arbitrary predicate language.

### CTR-SWPO-001 — Explicit Context opt-in

A prerequisite exists only when a published Definition Version's root
Context JSON Schema contains the exact annotation
`"x-workflow-prerequisite-v1": true` **and** declares the top-level
`workflowPrerequisite` property as this closed schema:

```json
{
  "type": "object",
  "required": ["kind", "workflowInstanceId", "definitionVersionId", "transitionId"],
  "properties": {
    "kind": { "const": "TERMINAL_SUBMISSION" },
    "workflowInstanceId": { "type": "string", "format": "uuid" },
    "definitionVersionId": { "type": "string", "format": "uuid" },
    "transitionId": { "type": "string", "format": "uuid" }
  },
  "additionalProperties": false
}
```

The opt-in annotation is legal only at the root of a Context Schema and the
property MUST be required at that root. A present malformed annotation, nested
annotation, annotation without the exact property/schema, or attempt to
reinterpret a similarly named property without the annotation MUST fail
Definition publication/validation closed. Absence of the annotation means
ordinary Context data: it is never parsed as a prerequisite. Published Version
immutability and its existing digest cover both marker and schema. Existing
published Definitions are not retroactively reinterpreted or edited.

The Context value itself contains exactly the four fields above. No
Submission ID, Event ID, future identifier, caller predicate, URI, JSONPath,
list, nested prerequisite, timeout, or readiness status is accepted. Creation
stores the ordinary immutable Context revision; later readiness derives from
that exact Context and Svc-owned facts. This is the single supported
non-GCR/GCR dependency shape in this Contract, not a general dependency DSL.

### CTR-SWPO-002 — Admission and source validation

For either ordinary create or GCR create-and-bind, Svc MUST validate the
request against the exact published Definition Version and its Context
Schema. An opted-in create is authorized only when the authenticated actor is
the current Owner of the dependent enabled Domain and has fresh existing
visibility to the exact source Instance in the same Domain. This explicitly
authorizes the bounded receipt projection in CTR-SWPO-004; it does not grant
source payload access or any additional role, scope, or Grant. The Domain Owner
check and source visibility check are revalidated within the create transaction.

The prerequisite MUST identify an already-existing WorkflowInstance in that
same enabled Domain, with exactly the submitted immutable Definition Version.
That Version MUST contain the exact `transitionId`, declare it terminal, and
define a Submission Schema. Only the VAI semantic model with canonical
Instance, Visit, Event, and Submission facts is supported. Legacy instances,
foreign or not-visible references, disabled Domains, unavailable/unpublished
Versions, wrong-version/transition pairs, and malformed or missing Context
references fail closed before workflow facts commit. Foreign/not-visible facts
MUST NOT be distinguished in an error response. A UUID alone grants no
visibility or authorization.

The source Instance and Version MUST preexist; a not-yet-created ID cannot be
referenced. The create transaction rechecks and locks the source Instance,
Domain and exact Version/transition under the repository's serialization
discipline. It checks exact terminal proof before active-Visit reachability:
if the exact selected terminal Event and Submission already exist with valid
lineage, the reference is accepted even if the source was later archived or
otherwise terminalized. If that positive proof does not exist, the source must
still be active and the selected transition must be reachable from its current
active Visit in the immutable published graph; otherwise the create fails as
an impossible prerequisite. Source terminal writer and create share one
serializable order. If create commits first while the transition remains
reachable, the dependent is WAITING; if the terminal proof commits first, the
dependent is READY. Self-reference and concurrent mutual references are
impossible because referenced instances must already exist. The dependent
Context is immutable after creation.

No future Submission ID is predicted or stored. Svc derives actual terminal
Event, Submission ID, and canonical Submission digest from authoritative rows
once committed. The source Submission's business truth is not evaluated by
this mechanism.

### CTR-SWPO-003 — Atomic governance-child create and bind

Svc exposes one new closed command on the GCR/Assistance boundary:

```http
POST /internal/v1/assistance-cases/{assistanceCaseId}/governance-child:create-and-bind
```

The existing bind endpoint and its body semantics remain unchanged. The new
command body contains only the exact child Definition Version, ordinary
immutable Context payload, exact child acceptance Transition, and existing
expected source-state CAS. The case ID is taken from the path. Core maps its
`create_and_bind_governance_child` operation to this new endpoint under the
existing authenticated route.

Svc derives Domain, source Instance/Visit/DefinitionVersion, requester, Owner,
request Context/event/digest and every generated child/binding coordinate from
authoritative rows. No caller may submit derived provenance or an Owner,
Domain, binding, child Instance, terminal Event, Submission ID or digest.

Within one SERIALIZABLE transaction and the existing command-idempotency
boundary, Svc MUST:

1. acquire/reconcile the command receipt for the authenticated actor, exact
   idempotency key and canonical request body;
2. lock and revalidate the exact open OWNER_PENDING case, source Instance and
   current Visit/state version, and current Domain Owner using existing GCR
   checks;
3. validate the child Version, acceptance Transition and closed GCR
   acceptance envelope under GCR V1, plus the prerequisite under CTR-SWPO-002;
4. create the child Instance, immutable Context, first Visit and initial VAI
   activation using the existing create predicates;
5. create the immutable GCR binding and `GOVERNANCE_CHILD_BOUND` source Event
   using the existing bind predicates; and
6. commit the success receipt with all business facts and return exact child,
   binding, acceptance-contract and prerequisite coordinates.

All steps commit or roll back together. No child, activation or dispatchable
intent may become visible before the binding is committed. Concurrent duplicate
commands with the same key/body yield one child and one binding receipt;
same key with a changed body deterministically conflicts. Deterministic
validation/authorization failures commit no child, visit, activation, binding
or GCR event and retain existing deterministic failure-receipt semantics.
Transaction deadlock/serialization retry remains internal and bounded under
existing repository rules; after that bound, return a safe rejection only if
rollback is known. A commit whose outcome is unknown is never retried under a
new key.

For an ambiguous response, linked case detail is the authoritative readback.
A matching case, child, Definition Version, binding, acceptance digest and
prerequisite proves commit. An authoritative rejection proves no commit.
Otherwise the result remains `UNKNOWN`: do not create a replacement or infer
outcome. The existing external caller may replay only the byte-equivalent
command under the same idempotency key. This command is not a second approval;
it uses the existing `workflow.execute` caller path and revalidates the
current Domain Owner for the case.

Ordinary non-GCR creation continues through existing `create_instance` with
its existing command and receipt semantics. If the Definition opts into this
prerequisite shape, the same current Domain Owner and fresh source visibility
checks above are required, and the immutable Context is validated in the
ordinary create transaction. This Spec does not change ordinary command
idempotency or create a case for it.

### CTR-SWPO-004 — Typed prerequisite readiness projection

Readiness is a read-time projection of the stored immutable Context and
authoritative source Domain, Instance, Visit, terminal Event and Submission
facts. It is recomputed on every readiness read and ready-page poll; there is
no readiness writer or durable wait state. Readiness is independent of the
dependent's time eligibility, which remains the distinct raw VAI/WEP time
projection. The closed JSON object is:

```json
{
  "status": "WAITING",
  "reason": "PREREQUISITE_PENDING",
  "sourceInstanceId": "<uuid>",
  "sourceDefinitionVersionId": "<uuid>",
  "transitionId": "<uuid>",
  "eventId": "<uuid, READY only>",
  "submissionId": "<uuid, READY only>",
  "submissionSha256": "<lowercase hex digest, READY only>"
}
```

This is a closed object (`additionalProperties: false`). All four status values
are exact uppercase strings. `sourceInstanceId`, `sourceDefinitionVersionId`,
and `transitionId` are always present. `reason` is omitted only for READY and
required for WAITING/CLOSED/UNKNOWN. `eventId`, `submissionId`, and
`submissionSha256` are present together only for READY; they are never
predicted. No Submission body or caller-provided readiness value is returned.

For `READY`, `reason` is absent and the exact immutable terminal Event and
Submission lineage must match the stored source Instance, Version and selected
transition. A valid exact proof has positive precedence over later source
archive/cancellation/terminalization; later lifecycle changes cannot erase an
immutable receipt. For `WAITING`, `reason` is exactly
`PREREQUISITE_PENDING`: no exact proof exists yet and the source remains active
with the selected terminal transition reachable. `CLOSED` is returned only
when no valid exact proof exists and authoritative facts establish that the
selected proof can no longer be produced; reason is one of
`SOURCE_CANCELLED`, `SOURCE_ARCHIVED`, `SOURCE_TERMINATED`,
`OTHER_TERMINAL_TRANSITION`, `TRANSITION_UNREACHABLE`, or
`SOURCE_DOMAIN_DISABLED`. `UNKNOWN` means
missing, unavailable, inconsistent or not-visible required facts and uses
`STORAGE_UNAVAILABLE`, `LINEAGE_INCONSISTENT`, or `SOURCE_NOT_VISIBLE`.
Unknown/missing status never becomes READY by timeout or model assertion.

Dependency-free Instances have no prerequisite object. Time eligibility is
never serialized as a prerequisite reason/status: not-yet-due work is absent
from the ready scheduler projection, while a readiness read may still report
`READY` for its satisfied prerequisite. A malformed persisted opted-in Context
or impossible/missing reference is `UNKNOWN`, never dependency-free `READY`.

The object is available on the read-only
`GET /internal/v1/workflow-instances/{instanceId}/readiness` view and existing
authorized current-task input. For a linked GCR case, authorized case detail
may return the same object. It is also exposed for ordinary workflows without
a case. The containing response field is exactly `prerequisiteReadiness` on
current-task input and linked case detail; the readiness-view response is
`{ "prerequisiteReadiness": <closed object> }`. Only a reader currently
authorized for the dependent and who is its
current assignee or current Domain Owner may receive it. The bounded object
contains the receipt IDs/digest shown above but no Submission body, source
Context, source task payload or history. No new role/scope/Grant is created;
UUIDs alone grant no access. A global scheduler is not a reader of this object
and receives only `readinessStatus: "READY"` with normal due coordinates in
CTR-SWPO-005. Other workflow readers, including an unrelated same-Domain
principal such as Bob, cannot obtain the receipt projection unless they
independently meet the dependent reader predicate.

`CLOSED` and `UNKNOWN` remain visible to authorized readers, whether or not a
GCR case exists. This slice adds no automatic recovery, case creation, retry,
Owner wake, timeout or indefinite-wait escalation. A closed ordinary workflow
without a case remains a visible stop; it is not silently marked PASS or
repaired.
### CTR-SWPO-005 — READY-only scheduler projection and pagination

Add a separate read-only scheduler projection; do not change the route,
bytes, filtering, or meaning of the VAI V1 raw due-intent endpoint or WEP time
projection. Its route is:

```http
GET /internal/v1/dispatch-intents/ready?limit=N[&afterNextEligibleAt=T&afterDispatchIntentId=ID]
scope workflow.read + direct token + enabled GLOBAL_SCHEDULER_READ binding
response { items: [{ ...existing due-intent coordinates, readinessStatus: "READY" }] }
```

The existing `GET /internal/v1/dispatch-intents` remains the raw time-only
API for its supported consumers. Broker operation `workflow_dispatch_intents.list_ready`
uses only the new `/ready` route; existing `workflow_dispatch_intents.list`
continues to use the raw route. No transparent fallback is allowed.

The page limit is 1..100. The optional keyset pair is both-or-neither and
uses the exact returned `(nextEligibleAt, dispatchIntentId)` tuple; malformed
or repeated tuples fail closed. The response row contains no Context or
Submission body, and `readinessStatus` is the literal `READY`; any other status
is excluded. The existing scheduler authorization and current service route
authentication apply. The new projection returns only active, time-due intents
whose derived prerequisite is `READY`, using the existing
`GLOBAL_SCHEDULER_READ` authorization and deterministic due ordering. For
dependency-free intents, no prerequisite blocks the ordinary time-due
predicate. For opted-in Contexts, time due and the exact prerequisite receipt
in CTR-SWPO-004 are both required.

The database query MUST derive/filter readiness before applying `LIMIT`; it
MUST NOT fetch the first N raw due rows and then filter in Core. Stable ordering
uses the existing due tuple plus activation ID as a unique tie-breaker. The
both-or-neither `afterNextEligibleAt` and `afterDispatchIntentId` pair is bound
to the last returned tuple for one poll scan. Core starts every normal poll at
the first page and drains pages by passing the exact last tuple returned. Rows
that are WAITING/CLOSED/UNKNOWN do not consume page capacity, so a blocked
prefix cannot starve later READY work. A later normal poll starts over and
reevaluates all candidates, so a formerly waiting row can become READY without
an outbox or persisted cursor. Invalid or repeated cursors fail closed; the
caller MUST NOT fall back to raw due rows.

The new Core consumer uses only this READY projection. Endpoint/storage error,
unknown readiness or malformed pagination produces no intent for that item
and no alternate raw-due route. Existing activation/wake, retry and attempt
ledgers remain unchanged.

### CTR-SWPO-006 — Failure, restart, compatibility

Source completion and readiness are derived from committed facts, not a wake
message. After any process restart, the same stored Context and source rows
derive the same status; the next normal poll reevaluates status and can return
the same dependent activation as READY. Poll interval remains the existing
Scheduler behavior. This Spec does not promise instantaneous wake or add an
outbox.

Lost create-bind response, duplicate concurrency, terminal-transition race,
source cancellation/archive, time-not-due, disabled/foreign source, storage
unavailability, and corrupt lineage fail closed per the typed statuses and
receipt rules above. Existing GCR review outbox, terminal acceptance proof,
Owner replacement, explicit resolve/escalate, and WEC resume remain unchanged.
Existing unlinked Assistance cases and dependency-free workflows retain their
current behavior. No historical Instance or Definition is migrated.

## 10. Acceptance — ACC-SWPO-001

The implementation PR MUST provide exact-head Svc tests plus the paired Core
integration acceptance covering:

| Contract | Method / environment | Required evidence | Expected result | Failure condition |
|---|---|---|---|---|
| CTR-SWPO-000 | Exact-base authority and compatibility review | Diff, endpoint/request identities, permission checks, Product V9 DEC-V9-001/002, Architecture CTR-ARCH-041/042/043, WEP V2 mapping | New authority adds bounded behavior without changing GCR/VAI/WEP/ACB meanings | Any changed raw due, eligibility, GCR, or existing permission semantics |
| CTR-SWPO-001 | Definition publication/schema tests | Exact published Schema digest and accepted/rejected schema examples | Only root marker plus required closed property opts in; unmarked fields stay business data | Nested/malformed/ambiguous marker is accepted or old Definition reinterpreted |
| CTR-SWPO-002 | Isolated create transaction tests | Owner/source visibility checks, source IDs/version/transition, transaction order and rollback receipts | Existing exact proof is admitted before reachability; otherwise only active reachable source is admitted | Foreign/not-visible source, invalid transition, or unauthorized actor commits facts |
| CTR-SWPO-003 | Isolated Postgres transaction and HTTP idempotency tests | One command receipt, DB rows, event/activation visibility, same-key replay and race outcomes | Child, activation, binding, event and receipt commit atomically exactly once | Partial child visibility, duplicate, fresh-key replay, or unknown treated as success |
| CTR-SWPO-004 | Query/read-model tests plus restart | Typed wire fields, terminal lineage, archive precedence, Owner/Alice/Bob visibility matrix | READY includes actual receipt; exact proof survives later archive; status/receipt limited to authorized dependent readers | Predicted ID, unrelated Bob receipt access, scheduler receipt leakage, or model-derived readiness |
| CTR-SWPO-005 | Scheduler DB query/pagination tests | SQL rows, closed route and Broker op, role check, tokens and blocked-prefix fixture | Time due AND prerequisite READY before limit; raw VAI route unchanged | WAITING prefix starves READY, receipt leaks to Scheduler, or raw fallback is possible |
| CTR-SWPO-006 | Restart and compatibility suite | Same source rows before/after restart; existing GCR/VAI/WEC receipts | Poll recomputes status; old flows retain behavior; no new wait writer/outbox | Restart loses readiness, duplicate event, changed existing lifecycle, or recovery claim |

1. publication accepts only the explicit root marker plus exact closed schema;
   malformed, nested, unknown-field and unmarked lookalike Contexts do not
   acquire prerequisite meaning; old published Definition behavior is
   unchanged;
2. ordinary create accepts a valid preexisting same-Domain reference and
   rejects foreign/not-visible, disabled, missing, wrong-Version,
   nonterminal/missing-transition, non-VAI and malformed references before
   workflow facts commit;
3. atomic GCR create-bind yields exactly one child, Visit, activation, binding,
   source Event and receipt; no dispatchable child exists before commit;
4. same-key/same-body replay, same-key/different-body conflict, concurrent
   duplicate, deterministic rollback, lost-response readback and unresolved
   `UNKNOWN` cause no duplicate child or fresh-key retry;
4a. source terminal proof before create is admitted even after archive; create
   before terminal writer is WAITING then READY; run both transaction race
   orderings and prove exact one durable receipt;
4b. current Domain Owner and authorized dependent Alice receive only the
   closed receipt object; unrelated Bob and cross-Domain readers are denied;
   global Scheduler receives only `readinessStatus` and no receipt IDs/digest;
5. each readiness status is derived correctly, exact proof remains READY after
   later source archive, CLOSED only appears without valid proof, and restart
   reproduces status without a wait table;
6. a distinct non-GCR dependent ordinary workflow created through ordinary
   `create_instance` accepts the same immutable prerequisite, stays WAITING
   with zero Core attempts and Router deliveries through repeated polls, then
   becomes READY and dispatches after its normal upstream supplier workflow
   commits the exact terminal Submission;
7. more WAITING rows than one page cannot starve a later READY row; new poll
   starts from page one and sees a newly READY row; raw VAI due and WEP
   time-only compatibility remain intact; Core uses `list_ready` with no
   fallback and performs fresh dependent tuple/config/Domain/authorization
   validation before WEC planning;
8. existing GCR terminal review/resolve, VAI wake/time due, WEC retry/unknown
   fence and dependency-free dispatch regression checks pass.

The isolated native run begins from one trigger and no precreated child. To
make the WAITING interval observable, its Owner Agent may first create an
ordinary supplier through existing `create_instance`, then use the returned
actual Instance ID while selecting an exact published child schema and making
the atomic GCR create-bind call. The supplier has normal Agent-owned task
nodes and completes through ordinary runtime execution; this is only a bounded
test arrangement, not product-specific selection behavior or an external
node-by-node intervention. A separate non-GCR dependent created through
ordinary `create_instance` proves that the same generic Context/readiness path
works without any Assistance case. Record actual receipts and exact Core/Svc
heads. An indefinite `WAITING`, `CLOSED`, or `UNKNOWN` is not PASS and has no
automatic recovery claim in this slice.

## 11. Alternatives and disposition

- `ALT-SWPO-001` — Reuse ACB V1's `acceptanceCriteria` payload/reference:
  rejected. ACB binds a submission report to the same Instance's Context and
  is not a cross-Instance readiness authority.
- `ALT-SWPO-002` — Add a generalized predicate/relationship DSL or
  per-business dependency handler: rejected. This slice supports one exact
  terminal-Submission input shape and applies it uniformly to ordinary and
  GCR-created Instances.
- `ALT-SWPO-003` — Add a durable wait ledger/outbox or change raw due meaning:
  rejected. Existing poll recomputes from canonical facts; the separate
  READY-only query preserves VAI and WEP contracts.

## 12. Migration, compatibility, and rollback

No schema migration or historical Instance/Definition rewrite is authorized.
The opt-in is part of a new immutable published Definition Version and its
existing digest; absence preserves all existing Context meanings. The raw VAI
due endpoint, WEP time-only classification, VAI activation/wake, existing
GCR binding/review/resolve, and dependency-free create/dispatch remain
unchanged. API rollout/rollback is an implementation/deployment concern under
separate authority; rollback must not route dependent intents through raw due.

## 13. Open questions

No normative choice remains open in this proposal. Implementation preflight
must demonstrate schema-marker compatibility, serializable transaction
composition, source graph reachability, and READY filtering before pagination.
Failure of any check stops for authority review; there is no silent fallback.

### Implementation bounds

Expected Svc surfaces are the Assistance HTTP/application transaction,
transaction-scoped create/GCR helpers, prerequisite Context validation,
current-task/Instance readiness reads, and a separate dispatchable-intent
query/route. Existing tables and events are sufficient; no migration is
authorized by this Contract. If atomic composition cannot be achieved without
an externally visible unbound child, stop and return for authority review. If
the readiness query cannot filter before page sizing without changing raw due
semantics, stop rather than permit starvation or a bypass.

This Spec and the paired Core Spec are accepted in the paired candidate
branches. Implementation requires both exact accepted authorities and their
parent successors in the implementation base, then a separate exact-scope
mandate. Production apply authority remains `none`.
