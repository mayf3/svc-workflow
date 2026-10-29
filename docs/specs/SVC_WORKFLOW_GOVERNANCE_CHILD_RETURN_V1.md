---
spec_id: SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V1
title: Owner-bound governance child and verified return to the originating visit
status: superseded
accepted_date: 2026-10-28
accepted_at: 2026-10-28T06:04:48Z
accepted_by: Codex root (explicitly delegated by mayf3)
accepted_reviewed_head: 1f191156a03d9397b2fb2fdb6c28d6a9621ace95
accepted_candidate_file_sha256: 7169e46bf86a9c9ff024a8a198ac6cf9e63827d43a2da364b4f837ace3d711aa
acceptance_review_verdict: PASS
independent_semantic_reviewer: /root/assistance_resume_core_impl_luna
acceptance_delta_class: new_standalone_contract
semantic_delta_after_review: NONE
acceptance_authority_basis: >-
  Root accepted the exact independently reviewed contract under mayf3's explicit
  delegation to complete source work and isolated validation only. The Core
  reviewer returned PASS on accepted_reviewed_head. The provisioning scope entry
  makes explicit the existing Owner-replacement hook required by §5 and does not
  alter its contract. This lifecycle commit records acceptance only; authority
  is effective after both companion Specs merge. Production apply authority is none.
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
repo: mayf3/svc-workflow
date: 2026-10-28
revision: r1
base_head: ed9344f4701b416e8cf6d7cd10cc4ae28969ff3f
scope:
  - svc-workflow Assistance case binding and review outbox
  - src/application/provisioning/mod.rs::replace_owner
  - src/store/postgres/provisioning_repository/mod.rs::replace_domain_owner
  - exact governance child acceptance provenance
  - Owner-controlled resolution and existing same-visit resume evidence
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V8
  - SVC_WORKFLOW_EXECUTION_CONTROL_V1
  - SVC_WORKFLOW_CURRENT_TASK_INPUT_V1
external_authorities:
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_WORKFLOW_EXECUTION_CONTROL_V2
    revision: 599107b6909e5f3153afd29bc846a122e50ebfd7
    relation: interoperates_with
supersedes: []
superseded_by: SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V2
owners:
  - mayf3
architecture_authority: SVC_WORKFLOW_ARCHITECTURE_V0_4_3
---

# SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V1

This accepted additive contract binds one already-existing, same-Domain
governance child WorkflowInstance to one open Owner Assistance case. A child
terminal event creates a second, separately deduplicated Owner review wake.
Only an explicit Owner resolve, carrying service-verifiable evidence from the
child's exact acceptance submission, may use the existing CTR-SWEC-010 resume
path. A child terminal or an Agent assertion never resolves the original
problem by itself. This accepted candidate becomes active when it and the
accepted Core companion merge to their authority branches. Implementation
remains separately authorized; production apply authority is none.

## 0. Current state and evidence

`STATE-GCRS-001` — At Svc `ed9344f4701b416e8cf6d7cd10cc4ae28969ff3f`,
accepted `SVC_WORKFLOW_EXECUTION_CONTROL_V1` provides the initial durable
Owner Assistance wake and exact explicit-resolve evidence for same-visit
resume. No governance-child relation, second review phase, or linked-resolve
proof is authorized at this cited base; this accepted candidate is not active
until merge. No implementation or runtime acceptance is claimed.

`OBS-GCRS-001` — Read accepted `CTR-SWEC-009/010`, current task input, and
the Svc assistance transaction, Owner-resume evidence query, and outbox
contracts at the pinned base. Existing case records carry source instance,
visit, requester, request payload/digest, command and event provenance;
existing outbox identity is keyed to the case. Method: local source and Spec
inspection only; no database or service call.

`CLM-GCRS-001` — SUPPORTED: extending the existing case/outbox and explicit
resolve path is narrower than introducing a second state authority or resume
mechanism. Supported by `OBS-GCRS-001` and accepted `CTR-SWEC-009/010`.

`EVD-GCRS-001` — `OBS-GCRS-001` supports `STATE-GCRS-001` and
`CLM-GCRS-001` only at the cited source snapshot. It does not establish that
the accepted relation or review wake exists in a deployed system.

## 1. Authority and compatibility

Svc remains the sole business-state authority. Existing case creation,
`workflow_assistance_read`, `resolve`, `escalate_to_human`, and
CTR-SWEC-010 evidence remain authoritative. The accepted Core companion adds a
typed Broker operation and distinct review-wake delivery; it does not grant a
new permission. The caller continues to require the existing `workflow.execute`
authorization, and this service independently checks the current Domain Owner
for the exact case.

Existing unlinked cases retain byte-for-byte behavior. This Spec adds no
automatic transition, no historical migration, no general obligation engine,
no arbitrary graph rewrite, no cross-case/global budget and no production
authorization. The relationship is exactly one layer; a failed child uses
ordinary Assistance or human escalation and cannot recursively bind another
governance child.

This binding operation accepts an already-created child instance. The existing
Core `workflow_execute.create_instance` operation can create an instance from
an exact published DefinitionVersion using its declared entry Context and
server-resolved assignee, but this contract does not define governance-graph
discovery, child Definition selection, or an unattended create-before-bind
planner. Passing this bind contract does not prove that the originating error
was automatically routed into a suitable governance Definition.

## 2. Typed binding operation — `CTR-GCRS-001`

Add one authenticated operation to the existing assistance-action boundary:

```http
POST /internal/v1/assistance-cases/{assistanceCaseId}/governance-child
```

The request requires the existing `Idempotency-Key` header, scope
`workflow.execute`, and this closed body:

```json
{
  "governanceChildInstanceId": "<uuid>",
  "acceptanceTransitionId": "<uuid>",
  "expectedWorkflowStateVersion": 7
}
```

The route requires the existing `workflow.execute` scope and server-side
current-Domain-Owner authorization. Caller-supplied source instance, visit,
Context revision, source DefinitionVersion, requester, Owner, Domain,
request-payload digest, child DefinitionVersion, terminal event, submission,
or evidence digest is forbidden. The service derives them from the locked
case/instance/visit, persisted assistance request/command/event, child
instance/version/transition and submission facts. Extra fields fail closed.

The response is an immutable binding receipt containing at least:

```json
{
  "bindingId": "<uuid>",
  "assistanceCaseId": "<uuid>",
  "source": {
    "workflowInstanceId": "<uuid>", "nodeVisitId": "<uuid>",
    "requestContextRevisionId": "<uuid>", "definitionVersionId": "<uuid>",
    "requestPayloadDigest": "<sha256>", "requestCommandId": "<uuid>",
    "requestEventSequence": 12
  },
  "child": {
    "workflowInstanceId": "<uuid>", "definitionVersionId": "<uuid>",
    "contextRevisionAtBind": "<uuid>", "acceptanceTransitionId": "<uuid>"
  },
  "acceptanceContractDigest": "<sha256>",
  "status": "BOUND"
}
```

The service may return additional exact persisted coordinates, but Core must
forward them transparently. The acceptance contract digest is the JCS
SHA-256 of the service-derived source tuple, child tuple, and the selected
published transition's complete submission schema. It is not caller input.

Persist one immutable relation row keyed by `binding_id`, unique on both
`assistance_case_id` and `governance_child_instance_id`, with the exact source
tuple, child tuple, selected acceptance transition, canonical contract digest,
binding principal, and creation event/time. The row references existing case,
instance, visit, Context and DefinitionVersion facts. Terminal-event
coordinates are added only by the child-terminal transaction; they are
immutable once set. The table stores
provenance, not a semantic repair verdict.

## 3. Admission, uniqueness and transaction — `CTR-GCRS-002`

Binding is permitted only when the case is open and `OWNER_PENDING`, the
requester/source visit remain current, the source and child are in the same
enabled Domain, and neither instance is canceled or archived. The child must
already exist and reference an exact published DefinitionVersion. The
selected transition must belong to that version, target a node whose
published `node_type` is `TERMINAL`, and have a non-null submission schema
that declares the closed governance-acceptance envelope in §6. Its transition
effect is not constrained; both a graph-defined `ADVANCE` into a terminal node
and another valid transition effect remain governed by the published graph.
The child must still be nonterminal in the binding transaction's locked
read. This is required because binding creates the binding ID and contract
digest which the later acceptance envelope must cite. If terminal transition
commits first, bind returns a terminal-state conflict; it does not
retroactively create proof or a review wake.

Reject self-link; a second child for the case; a child already linked to any
case; an origin already used as a child; a candidate child already used as an
origin; source/child Domain mismatch; stale workflow-state version, visit,
assignee or DefinitionVersion; wrong Owner; and any ambiguous or incomplete
provenance. A newer Context revision alone is not stale when it remains
attached to the same visit and instance.
These one-layer rules prevent cycles and nested relation chains without
introducing a general graph or budget policy. A single source instance may
still have distinct cases; each such case independently has only one child.

The source request Context revision is derived from the exact
`ASSISTANCE_REQUESTED` event joined by the case's persisted
`request_command_id`; the existing event writer records `context_revision_id`,
source/target visit, event sequence and case/request digest. It is immutable
request-time provenance, not a requirement that the source Context remain
current. `CONTEXT_REVISED` may legitimately change Context while retaining the
same visit. Bind and later resolve must still prove the exact source instance,
visit, DefinitionVersion, current assignee and request event/digest; they must
not reinterpret the original request using a later Context. The service records
the child Context revision observed at bind as historical provenance, but does
not require it to remain current. At the selected terminal transition it also
records the exact child terminal Context revision with the terminal event and
Submission. The Domain Owner reviews the current source Context and exact child
terminal evidence before explicit resolve. The source DefinitionVersion is
derived from the bound source instance's version and verified against the
event's source node/visit. If the request event or its matching Context is
missing or ambiguous, binding fails closed.

In one SERIALIZABLE transaction, lock the case, source instance/current visit,
and child instance/current visit/version in deterministic UUID order; recheck
all admission predicates; derive and persist the immutable relation; append a
`GOVERNANCE_CHILD_BOUND` event and advance the source instance's
`workflow_state_version` using the caller's `expectedWorkflowStateVersion`
CAS. The child terminal writer and binder serialize on the same child
instance/relation lock: if bind commits first, the later terminal transaction
queues exactly one review wake; if terminal commits first, bind refuses.
At the HTTP boundary, a caller that retained its `Idempotency-Key` may
reconcile or retry only the byte-equivalent command under that same key; it
must never use a new key to create a second relation. In the existing Core
Agent tool path, the trusted HTTP transport generates the key and does not
expose it to the Agent or provide a receipt-query tool. After a lost/unknown
bind response, the Agent must read case detail and match the exact child and
contract digest. If this readback proves neither commit nor authoritative
rejection, the outcome remains UNKNOWN and no new bind/create invocation is
permitted.

## 4. Service-derived child task input — `CTR-GCRS-003`

The authorized current-assignee `workflow_current_task_input` projection for a
bound child adds a `governanceParent` object containing the exact immutable
binding receipt from §2 and the acceptance-contract digest. It is returned
only to the existing authorized reader of that child; the service derives it
from the binding row, not child Context or model text. Unbound instances have
the unchanged response shape. This is a reference to the original request
facts, not a business conclusion or an expansion of child write authority.

The Owner's existing case read and ordinary authorized Workflow instance and
submission read surfaces remain the review mechanism. Svc does not infer that
the Owner read a message or that evidence is substantively true. Core's
isolated protocol acceptance must observe Owner reads of the exact child
terminal event/submission before the resolve call; this is an integration
acceptance condition, not a new service permission or a persisted read claim.
The existing Owner-authorized `GET /internal/v1/assistance-cases/{caseId}`
detail projection is extended with optional `governanceBinding` and
`governanceTerminalProof` references. The relationship coordinates and terminal
proof are immutable; the nested current-review delivery projection is mutable
as recipient ownership changes. They use the existing requester/Domain
Owner visibility predicate; the existing Broker `workflow_assistance_read.detail`
forwards them transparently. The binding reference includes source request
coordinates, child coordinates, selected transition and contract digest, plus
a current-review projection containing `reviewWakeId`, recipient Owner
Principal, recipient Owner-binding reference and delivery/review status. The
terminal reference includes the exact terminal event, child
visit, DefinitionVersion, Context revision, Submission ID/digest, submitter and
verdict when present. It contains no child Submission body; the Owner follows
existing authorized instance/submission reads. This is the readback path after
a lost bind response or Owner change, not a new permission.

## 5. Distinct review outbox and stale handling — `CTR-GCRS-004`

Reuse the existing `OWNER_ASSISTANCE_WAKE` outbox kind with a closed typed
phase discriminator. Initial rows remain unchanged. Each review delivery has
an immutable `reviewWakeId`, recipient Principal, and current Owner-binding
reference. Its event key is unique by binding, terminal event, recipient and
that wake ID:

```text
owner-assistance-review:<bindingId>:<terminalEventId>:<ownerPrincipalId>:<reviewWakeId>
```

It is drained through the existing CTR-SWEC-009 authenticated Owner-wake
transport and existing Core route, but carries the closed phase
`GOVERNANCE_CHILD_REVIEW` and exact binding/event/submission coordinates. It
must not reuse or mutate the
initial `owner-assistance:<caseId>` outbox key, request ID, or correlation.
Retries of the same `reviewWakeId` coalesce for the same recipient Principal.
A later Owner assignment gets a recipient-specific event identity, so an earlier recipient's
acknowledgement cannot suppress the new Owner's wake. The initial case wake
cannot suppress this review phase. No new OAuth scope, client grant, or
credential class is added.

The review HTTP payload is a closed additive union of the existing initial
payload and these required phase fields:

```json
{
  "phase": "GOVERNANCE_CHILD_REVIEW",
  "workflowInstanceId": "<source uuid>",
  "nodeVisitId": "<source visit uuid>",
  "assistanceCaseId": "<case uuid>",
  "ownerPrincipalId": "<current owner uuid>",
  "ownerBindingId": "<current owner binding uuid; informational, not dedupe identity>",
  "reviewWakeId": "<uuid>",
  "bindingId": "<uuid>",
  "governanceChildInstanceId": "<uuid>",
  "childNodeVisitId": "<uuid>",
  "childDefinitionVersionId": "<uuid>",
  "childTerminalEventId": "<uuid>",
  "acceptanceTransitionId": "<uuid>",
  "acceptanceContractDigest": "<sha256>",
  "childSubmissionId": "<uuid, only when exact submission exists>",
  "childSubmissionDigest": "<sha256, only with childSubmissionId>"
}
```

The initial payload retains its current shape and has no `phase` property.
The terminal event and selected acceptance transition are mandatory for a
review row; if there is no exact terminal event, no review row is created.
When a valid selected-transition submission is absent, only
`childSubmissionId` and `childSubmissionDigest` are omitted; the event still
wakes the Owner to inspect or escalate. Producer and Core consumer reject
unknown fields, invalid conditional combinations, and representing the initial
phase as this review phase or vice versa.

Immediately before delivery, the sender rereads the case, binding, source
instance/visit/assignee, current Domain Owner binding, and child terminal facts.
Request-time, bind-time and terminal Context revisions are immutable historical
coordinates; a later `CONTEXT_REVISED` alone does not invalidate the case or
suppress review. If the case is resolved, source visit/DefinitionVersion/
assignee changed, binding differs, or terminal event is no longer the exact
persisted event, the row is suppressed with a structured reason. If only the
Domain Owner changed, the sender atomically retires the old-recipient pending
row and enqueues a new review row for the current Owner; it must not
acknowledge the only outstanding review responsibility. Each successful,
actual Owner-replacement transaction enqueues a recipient-bound review row for
every unresolved linked case that already has terminal proof. A no-op request
that leaves the Owner unchanged creates no replacement wake. This covers Owner
changes before delivery and after an earlier successful delivery. The exact
`workflow_command_receipts.command_id` of the successful Owner-replacement
command participates in the new `reviewWakeId`, together with the recipient
Principal; the role-binding ID alone is insufficient because an existing
binding row may be reactivated. Event-key creation is idempotent within that
transaction. Switching away and later back to the same Principal therefore
creates a new wake because the successful replacement command ID differs,
while replaying one replacement command does not. `reviewWakeId` and current
recipient are persisted and appear in the case detail and wake DTO. This handoff is durable
and requires no external watcher. Review remains outstanding until explicit
resolve or escalation; a 2xx wake delivery records delivery only, not
completion. The current Owner rereads case detail and child evidence before
acting. Transient read/database/transport failures use existing bounded
retry/backoff. Binding requires a nonterminal child, so only a later terminal
event can queue review. Ambiguous terminal or submission provenance fails
closed and cannot produce a resolve proof.

Owner replacement and child terminal-event/outbox creation MUST serialize on
the enabled Domain's current-owner coordinate (or an equivalent serializable
transactional guard). If terminal commits first, a later successful Owner
replacement sees its unresolved terminal proof and creates the new recipient
wake. If Owner replacement commits first, the terminal producer resolves and
records the then-current Owner recipient. Concurrent ordering may not leave an
unresolved terminal proof without a review wake for the current Owner. A stale
HTTP delivery already in flight may reach the former Owner, but it cannot
resolve the case; the replacement transaction still creates the current
Owner's distinct wake.

## 6. Terminal acceptance proof — `CTR-GCRS-005`

The selected child terminal submission schema must require this closed
envelope, with the ordinary business evidence fields remaining governed by
that Definition:

```json
{
  "governanceAcceptance": {
    "bindingId": "<uuid>",
    "acceptanceContractDigest": "<sha256>",
    "sourceRequestPayloadDigest": "<sha256>",
    "verdict": "ACCEPTED"
  }
}
```

The reserved object is `additionalProperties:false`; it requires exactly
`bindingId`, `acceptanceContractDigest`, `sourceRequestPayloadDigest`, and
`verdict`, where `verdict` is the enum `ACCEPTED|NOT_ACCEPTED`. The selected
Definition may add its own ordinary business-evidence properties outside this
reserved object, and their semantic truth remains that workflow's acceptance
responsibility.

On the selected terminal transition, ordinary published schema validation
first requires a structurally valid envelope. A malformed or missing required
envelope is rejected by the existing submission-validation path and does not
commit the transition, terminal event, or review wake. Svc then validates the
envelope against the immutable binding and persists the exact transition
event, Submission ID/digest, submitted-by Principal, child visit, child
DefinitionVersion, terminal Context revision, and contract digest. A
structurally valid `NOT_ACCEPTED` verdict is a real terminal outcome but cannot
authorize resolution. Another legal terminal transition may also produce a
real terminal outcome and review wake, but cannot authorize linked resolve
unless it is the selected transition with the exact `ACCEPTED` proof. Schema
validity is only structural; this Spec does not claim business truth. The exact
current child assignee at the terminal visit is the service-verifiable
submitting actor. The current Domain Owner makes the explicit final case
decision; no distinct-Principal condition is imposed.

## 7. Explicit linked resolve — `CTR-GCRS-006`

For a linked case only, extend the existing `resolve` payload's
`supportingPayload` with a required closed `governanceChildReview` reference.
The enclosing request retains the existing `expectedWorkflowStateVersion`,
`resolution.message`, and `Idempotency-Key`:

```json
{
  "bindingId": "<uuid>",
  "terminalEventId": "<uuid>",
  "submissionId": "<uuid>",
  "submissionDigest": "<sha256>",
  "acceptanceContractDigest": "<sha256>"
}
```

This typed object is nested in `resolution.supportingPayload`, not a new
top-level route or grant. Duplicate/missing keys and unknown members are
rejected for this reserved object. A retry with the same idempotency key and
byte-equivalent canonical command returns the original receipt; reusing the
key with a different body conflicts. The Agent-facing Core tool does not
expose its transport-generated key or provide a receipt-query operation. After
an unknown resolve outcome, the Owner reads case detail and existing WEC
resume evidence. If those authoritative reads do not establish the result,
retain UNKNOWN and do not invoke resolve again with a new key. A trusted HTTP
caller that retained the original key may only reconcile/retry the same
byte-equivalent command under it.

Within the existing resolution transaction, Svc locks and rechecks the exact
case, source visit/DefinitionVersion/request digest, current Domain Owner,
immutable binding and child. It verifies the request-time Context as immutable
provenance but does not require it to equal the current source Context. The
Owner's explicit resolve is based on the fresh current source Context and exact
child terminal evidence, including that terminal event's Context revision. It
requires the selected terminal event
and Submission to be on the exact bound child visit and DefinitionVersion,
the Submission digest and envelope to match persisted facts, the submitting
Principal to have been the authorized current child assignee, and verdict
`ACCEPTED`. It rejects missing, duplicated, foreign, stale, mismatched or
ambiguous references. The resolve command and event persist the exact binding,
terminal event, Submission and digests as provenance. The Owner's action—not
the child event—remains the cause of RESOLVED.

For unlinked cases the current CTR-SWEC-010 request and behavior are unchanged.
`escalate_to_human` remains available for linked cases without acceptance
proof and does not resume the source. After a valid linked resolve, the
existing CTR-SWEC-010 evidence continues to authorize only the exact
same-active-dispatch, same-instance, same-visit, same-assignee Core resume;
this Spec adds no second resume mechanism and no automated transition.

## 8. Required acceptance — `ACC-GCRS-001`

Service tests must cover exact derived source/child tuples and digests;
requester/Owner/Domain/visit/Context/version drift; all uniqueness and
one-layer rejections; child terminal-before-bind and bind-before-terminal
serialization; one review outbox event under duplicate terminal/worker retry;
distinct initial/review dedupe identities; Owner changes before delivery, after
successful delivery, and away-and-back to the same Principal; no-op Owner
replacement; old-recipient suppression and current-recipient redelivery;
detail recovery after an unknown bind response; inconclusive bind/resolve
readback remains UNKNOWN without a second mutation; same-visit source Context
revision preserving request provenance; stale case/visit suppression; retry
after transient failure; acceptance-envelope and resolve mismatch matrices;
explicit linked resolve; unlinked resolve compatibility; and exact
CTR-SWEC-010 same-visit evidence preservation. A partial or unknown write must
never be retried under a new relation key.

The isolated cross-repository acceptance must exercise the real native Owner
typed bind action, actual child task input and terminal Submission, durable
review outbox delivery to Core, Owner reads, explicit resolve, and original
same-visit resume without external per-node intervention. It must also retain
one initial trigger and exact source/runtime pins. Service tests alone cannot
claim unattended business completion.

## 9. Compatibility, rollout and open decisions

This is additive; no existing case or instance is migrated. Both companion
Specs are accepted in their candidate commits; this authority becomes active
after both merge into their authority branches. Implementation remains
separately authorized. Core and
Svc must be deployed compatibly before the typed action is exposed. If either
side is unavailable or the review phase is unknown, the original case stays
open and existing human escalation remains available. Rollback disables only
new binding/review operations; persisted relations and receipts remain
readable and are never rewritten as unlinked history.

No normative decisions are delegated to implementers. This Spec grants no
production permission.

## 10. Decision and alternatives

`DEC-GCRS-001` — Keep the existing Assistance case and explicit resolve as
the source workflow's authority; add only one immutable child relation, one
distinct terminal-review outbox phase, and exact linked-resolve evidence.
Decision owner: root accepted this contract under mayf3's explicit delegation
after independent semantic review. Implementation remains separately
authorized. Rejected:
terminal-as-proof, automated resolve/transition, a new grant, arbitrary
workflow migration, recursive governance, and a second resume engine.

## 11. Migration and rollback

No existing case or WorkflowInstance is backfilled. Unlinked cases are
unchanged. A rollback disables new bind and review events but retains linked
records and their provenance for authorized reads. The ordinary initial wake
and current human escalation continue. The new Core action must remain
disabled unless the compatible Core companion authority/implementation is
present. No production action is authorized.

## 12. Open questions

None. Implementation remains forbidden until both the Core companion and
this Svc Spec are independently reviewed, accepted and merged.
