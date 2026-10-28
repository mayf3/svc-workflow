---
spec_id: SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1
title: Atomic governance-child creation and typed workflow prerequisite readiness
status: proposed
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
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V8
  - SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V1
  - SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
  - SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1
supersedes: []
superseded_by: null
owners:
  - mayf3
external_authorities:
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1
    revision: 01c1cd0c4a980e1a37878ea5ad05f4d46fb3ba99
    relation: interoperates_with
---

# SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1

This proposed Contract authorizes a later bounded Svc implementation of (1) an
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

This is `AUTHORITY_ACTION=NEW`, `PLAN_LEVEL=EXEC_PLAN`,
`ASSURANCE_LEVEL=CONTROLLED`; route stage is `AUTHORITY_AUTHORING` and the new
authority is not accepted in this base. Local parents are Product V8,
Architecture v0.4.3, GCR V1, VAI V1, and WEP V1. Paired Core
`AGENT_CORE_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1` at candidate base
`01c1cd0c4a980e1a37878ea5ad05f4d46fb3ba99` is required. Acceptance in a
docs-only branch does not activate implementation authority; production apply
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
- `CLM-SWPO-002` (SUPPORTED) — Existing `GLOBAL_SCHEDULER_READ` can guard a
  separate scheduler projection while existing Instance/current-assignee
  visibility can guard product readiness reads; no wider permission is
  necessary. Supported by VAI V1 CTR-VAI-009/010 and GCR V1's read boundary.

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
Schema. If opted in, the reference MUST identify a WorkflowInstance that
already exists at transaction admission, belongs to the same enabled Domain,
and has exactly the submitted immutable DefinitionVersion. That Version MUST
contain the exact `transitionId`, declare it terminal, and define a
Submission Schema. Only the VAI semantic model with canonical Instance,
Visit, Event, and Submission facts is supported. Legacy instances, foreign or
not-visible references, disabled Domains, unavailable/unpublished Versions,
wrong-version/transition pairs, and malformed or missing Context references
fail closed before workflow facts commit. Foreign/not-visible facts MUST NOT
be distinguished in an error response.

The source Instance and Version MUST preexist; a not-yet-created ID cannot be
referenced. The create transaction rechecks and locks the source Instance,
Domain and exact Version/transition under the repository's serialization
discipline. The upstream terminal writer and creation therefore observe one
serializable order. Self-reference and mutual references between concurrently
created instances are impossible because submitted references must already
exist. At creation, the selected transition MUST be reachable from the
upstream Instance's current active Visit in its immutable published graph;
otherwise reject as an impossible prerequisite. The dependent Context remains
immutable after creation.

No future Submission ID is predicted or stored. When the source later commits
the selected terminal transition, Svc derives the actual terminal Event,
Submission ID and canonical Submission digest from their authoritative rows.
The source Submission's business truth is not evaluated by this mechanism.

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

Ordinary non-GCR creation continues through existing `create_instance`, with
existing active-membership/Owner checks and existing receipt semantics. Its
Context may carry the same explicitly opted-in prerequisite, validated in the
ordinary create transaction. This Spec does not change ordinary command
idempotency or create a case for it.

### CTR-SWPO-004 — Typed readiness projection

Readiness is a read-time projection of the stored immutable Context and
authoritative Domain, Instance, Visit, terminal Event and Submission facts. It
is recomputed on each ordinary scheduler poll and readiness read; there is no
new readiness writer or durable wait state. The closed wire status is exactly
`READY | WAITING | CLOSED | UNKNOWN`, with a closed reason enum:

| Status | Meaning |
|---|---|
| `READY` | The dependent activation is time-due and, when opted in, the exact referenced Instance/Version/Transition has committed its terminal Event and Submission with consistent lineage. Include actual `submissionId`, `eventId`, `transitionId`, and canonical digest when there is a prerequisite. |
| `WAITING` | Dispatch is not yet eligible because the dependent activation is not time-due or the exact upstream terminal Submission is pending. `reason` is exactly `NOT_YET_DUE` (checked before pending) or `PREREQUISITE_PENDING`. |
| `CLOSED` | Authoritative source facts show the referenced transition cannot now produce the required receipt. `reason` is exactly `SOURCE_CANCELLED`, `SOURCE_ARCHIVED`, `SOURCE_TERMINATED`, `OTHER_TERMINAL_TRANSITION`, `TRANSITION_UNREACHABLE`, or `SOURCE_DOMAIN_DISABLED`. |
| `UNKNOWN` | Required rows are unavailable, inconsistent, ambiguous, or their lineage cannot be proven. `reason` is exactly `STORAGE_UNAVAILABLE`, `LINEAGE_INCONSISTENT`, or `SOURCE_NOT_VISIBLE`. It is never dispatchable and cannot become `READY` by timeout or model assertion. |

Dependency-free active Instances are `READY` once ordinarily time-due; their
existing time semantics do not change. Before their time eligibility they are
`WAITING / NOT_YET_DUE`. For every dependent, authoritative source `CLOSED` or
`UNKNOWN` takes precedence over time waiting so those stop states remain
visible. Otherwise, if the dependent activation is not time-due, return
`WAITING / NOT_YET_DUE`; once due, return `WAITING / PREREQUISITE_PENDING`
until the exact source receipt exists. A malformed persisted opted-in Context
or impossible/missing authoritative reference is `UNKNOWN`, never
dependency-free `READY`. A reference rejected before creation is a
deterministic create failure, not a persisted readiness status.

The typed projection is available on a new read-only
`GET /internal/v1/workflow-instances/{instanceId}/readiness` view and is
included in the existing authorized current-task input for an active
dependent. It uses the existing Instance/current-assignee visibility rules;
it adds no scope or visibility for foreign Instances. If the Instance has a
linked GCR case, authorized case detail includes the same typed status and
receipt projection. Ordinary workflows without a case still expose
`CLOSED`/`UNKNOWN` to their existing authorized Instance/current-task readers.
`READY` contains actual receipt references, not Submission body or a business
PASS claim.

`CLOSED` and `UNKNOWN` are readable stop states. This slice adds no automatic
recovery, case creation, retry, Owner wake, timeout, or indefinite-wait
escalation. Existing case resolve/escalate semantics remain available only
where a case already exists. Missing Owner visibility is not silently treated
as success.

### CTR-SWPO-005 — READY-only scheduler projection and pagination

Add a separate read-only scheduler projection; do not change the bytes,
filtering, or meaning of the VAI V1 raw due-intent endpoint or WEP time
projection. Its route is:

```http
GET /internal/v1/dispatchable-intents?limit=N[&cursor=<opaque>]
scope workflow.read + direct token + enabled GLOBAL_SCHEDULER_READ binding
response { items: [{ ...existing due-intent coordinates, readiness: { status: "READY" } }], nextCursor: <opaque|null> }
```

The page limit is 1..100, the response row contains no Context or Submission
body, and `readiness.status` is the literal `READY`; any other status is
excluded. The existing scheduler authorization and current service route
authentication apply. The new projection returns only active, time-due intents whose
derived readiness is `READY`, using the existing `GLOBAL_SCHEDULER_READ`
authorization and deterministic due ordering. For dependency-free intents,
`READY` means the ordinary due predicate holds. For opted-in Contexts, it also
requires the exact prerequisite receipt in CTR-SWPO-004.

The database query MUST derive/filter readiness before applying `LIMIT`; it
MUST NOT fetch the first N raw due rows and then filter in Core. Stable ordering
uses the existing due tuple plus activation ID as a unique tie-breaker. The
opaque continuation token is bound to that tuple and one poll scan. Core
starts every new normal poll from the first page; it drains all pages for that
scan. Rows that are WAITING/CLOSED/UNKNOWN do not consume page capacity, so a
blocked prefix cannot starve later READY work. A later ordinary poll starts
over and reevaluates all candidates, so a formerly waiting row can become
READY without an outbox or persisted cursor. Invalid, repeated, or expired
tokens fail closed; the caller MUST NOT fall back to raw due rows.

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
| CTR-SWPO-000 | Exact-base authority and compatibility review | Diff, endpoint/request identities, permission checks | New authority adds bounded behavior without changing GCR/VAI/WEP/ACB meanings | Any changed raw due, eligibility, GCR, or existing permission semantics |
| CTR-SWPO-001 | Definition publication/schema tests | Exact published Schema digest and accepted/rejected schema examples | Only root marker plus required closed property opts in; unmarked fields stay business data | Nested/malformed/ambiguous marker is accepted or old Definition reinterpreted |
| CTR-SWPO-002 | Isolated create transaction tests | Source IDs/version/transition, Domain visibility, rollback receipts | Only preexisting reachable same-Domain VAI source is admitted | Foreign/stale/unreachable/malformed reference commits any workflow fact |
| CTR-SWPO-003 | Isolated Postgres transaction and HTTP idempotency tests | One command receipt, DB rows, event/activation visibility, same-key replay and race outcomes | Child, activation, binding, event and receipt commit atomically exactly once | Partial child visibility, duplicate, fresh-key replay, or unknown treated as success |
| CTR-SWPO-004 | Query/read-model tests plus restart | Typed state/reason and exact terminal Event/Submission/digest lineage | READY includes actual receipt; other statuses never dispatch; authorized readers can see status with or without case | Predicted ID, hidden status, unauthorized visibility, or arbitrary/model-derived readiness |
| CTR-SWPO-005 | Scheduler DB query/pagination tests | SQL rows, role check, page tokens and blocked-prefix fixture | READY filter precedes page limit; raw VAI endpoint unchanged | WAITING prefix starves READY or any raw-due fallback is possible |
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
5. each readiness status is derived correctly, including actual future
   Submission/Event/digest only after upstream terminal commit, and restart
   reproduces status without a wait table;
6. a distinct non-GCR dependent ordinary workflow created through ordinary
   `create_instance` accepts the same immutable prerequisite, stays WAITING
   with zero Core attempts and Router deliveries through repeated polls, then
   becomes READY and dispatches after its normal upstream supplier workflow
   commits the exact terminal Submission;
7. more WAITING rows than one page cannot starve a later READY row; new poll
   starts from page one and sees a newly READY row; raw VAI due and WEP
   time-only compatibility remain intact;
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

This is a proposed new authority, not accepted implementation permission.
Implementation requires this exact Spec and the paired Core Spec accepted in
the relevant bases, then a separate exact-scope mandate. Production apply
authority remains `none`.
