---
spec_id: SVC_WORKFLOW_DISPATCH_ASSIGNEE_RESOLUTION_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: conditional_controlled_operation
scope:
  - mayf3/svc-workflow
  - dispatch-bound historical-assignee to resolver-Principal read surface
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V7
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_1
  - SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
  - SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2
external_authorities:
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_EXACT_PRINCIPAL_AGENT_RESOLUTION_V2
    revision: 5a5395246e7cbd7412101167d8a99042c15db1aa
    relation: constrained_by
supersedes: []
superseded_by: null
owners:
  - mayf3/svc-workflow maintainers
---

# SVC_WORKFLOW_DISPATCH_ASSIGNEE_RESOLUTION_V1

## 0. Route and authority boundary

```text
SPEC_GOVERNANCE_MODE = AUTHOR
AUTHORITY_ACTION = NEW
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO
BASE_HEAD = f525d5575906bcdb46a246194f77a67c21a19604
CURRENT_BASE_HEAD = f525d5575906bcdb46a246194f77a67c21a19604
PLAN_LEVEL = EXEC_PLAN
ASSURANCE_LEVEL = CONTROLLED
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = NO
IMPLEMENTATION_ALLOWED = NO
MERGE_READY = NO
OPERATION_ALLOWED = NO
SPEC_GAP_DEPENDENCY = LOAD_BEARING
OWNER_DECISION_REQUIRED = YES
```

The active Goal requests the first real Workflow dispatch-chain closure. It is
an Execution Mandate and evidence source, not Product Authority. This new
identity/trust/public-protocol decision is docs-first. No implementation,
deployment, wake or business-state mutation is authorized until this exact
authority is independently reviewed, accepted by an authorized Owner and
merged into the implementation base.

## 1. Goal

Preserve the immutable assignment Principal on an existing NodeVisit while
allowing a trusted dispatcher to discover which exact Workflow Principal must
be submitted to the external Auth exact-Principal resolver after a formally
recorded canonical-identity successor repair.

```text
historical assigned Principal
  -> exact current Dispatch Intent / NodeVisit binding
  -> immutable Workflow successor lineage, zero or one edge
  -> resolverPrincipalId
  -> external Auth exact Principal resolver
  -> canonical enabled Agent
```

`resolverPrincipalId` is not an Agent ID, not an authorization alias and not a
substitute for the Auth resolver. The historical NodeVisit, activation and
owner Principal are never rewritten.

## 2. Scope and non-goals

In scope is one additive read-only internal endpoint bound to an exact active,
due `DISPATCH_INTENT`. It composes existing activation facts with the accepted,
append-only `workflow_identity_successor_lines` surface.

Out of scope: changing the seven-field due-feed response; changing wake;
reassignment; Principal/Agent/Client/Grant creation; editing lineage; display
name or stored `canonical_agent_id` routing; generic Principal lookup; implicit
redirect; dispatch, Session admission, Transition, retry, or replacement
Instance creation. This authority adds no database table or migration.

## 3. Authority and dependencies

```text
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
IDENTITY_LINEAGE_AUTHORITY = SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2
PRODUCT_DIRECTION = SVC_WORKFLOW_PRODUCT_BOUNDARY_V7
ARCHITECTURE = SVC_WORKFLOW_ARCHITECTURE_V0_4_1
IMPLEMENTATION_AUTHORITY = contracts only after acceptance and merge
EXTERNAL_AUTHORITY = mayf3/dsh-agent-core
  AGENT_CORE_EXACT_PRINCIPAL_AGENT_RESOLUTION_V2
  @ 5a5395246e7cbd7412101167d8a99042c15db1aa
AUTHORITY_CONFLICT = NONE after selecting a new additive endpoint
```

This Spec governs only svc-workflow behavior. It neither changes nor accepts
the external resolver authority. A dsh consumer change requires its own
accepted local authority after this contract is accepted and pinned.

## 4. Current State

### STATE-DAR-001 — first real subject is blocked before delivery

- Source main: `f525d5575906bcdb46a246194f77a67c21a19604`.
- Production observation time: 2026-09-09.
- Instance: `cebf4816-c664-40cb-9b61-3fa330ad1c39`.
- Current NodeVisit: `75a134e8-bd5f-4d3a-af2d-469da433e0c5`.
- Dispatch Intent: `9ccd0359-d23b-4122-939b-9180a722fd7f`.
- Historical assignee: `61819256-07e1-4bd0-adea-e93e51243fa1`.
- Recorded successor: `9e3adced-575f-4fb2-b351-f7698b59127d`.
- Result: the intent is active and due; no successful delivery fact was found.
- Basis: `OBS-DAR-001`, `OBS-DAR-002`, `OBS-DAR-003`, `EVD-DAR-001`.

## 5. Observations

### OBS-DAR-001 — due feed preserves the historical owner

- Subject: due Dispatch Intent projection.
- Repository/source: `mayf3/svc-workflow`,
  `src/store/postgres/workflow_instance_repository/query_dispatch_intents.rs`.
- Commit: `f525d5575906bcdb46a246194f77a67c21a19604`.
- Environment: source main.
- Observed at: 2026-09-09.
- Method: exact source and accepted-Contract read.
- Result: the query projects the activation's `owner_principal_id`; accepted
  `CTR-VAI-009` and `CTR-DKC-002/003` freeze the seven-field feed and its
  no-cursor compatibility.
- Provenance: named source file and accepted Specs in this repository.

### OBS-DAR-002 — an accepted exact lineage already exists

- Subject: Workflow identity successor projection.
- Repository/source: `mayf3/svc-workflow`, migration 0025 and
  `src/store/postgres/identity_successor.rs`.
- Commit: `f525d5575906bcdb46a246194f77a67c21a19604`.
- Environment: source main plus production database read-only census.
- Observed at: 2026-09-09.
- Method: exact source read and read-only production query.
- Result: one immutable row per stale source is supported; the helper returns
  the exact successor for one edge and otherwise the source. Query visibility
  and detail already consume the lineage without rewriting the NodeVisit.
- Provenance: migration/source paths above and the Goal runtime census record.

### OBS-DAR-003 — direct Auth resolution of the historical subject fails closed

- Subject: exact Auth Principal-to-Agent resolution for the frozen real subject.
- Repository/source: production Auth read surface plus accepted dsh resolver.
- Commit/artifact: dsh main
  `5a5395246e7cbd7412101167d8a99042c15db1aa`; production read at the time below.
- Environment: production loopback Auth and deployed dsh runtime.
- Observed at: 2026-09-09.
- Method: exact Principal UUID lookup and deployed resolver call.
- Result: historical Principal stores a non-canonical Agent ID; recorded
  successor is active with the expected canonical Agent ID; the dsh exact
  resolver rejects the historical stored-ID grammar.
- Provenance: sanitized Goal census; no secret or credential bytes retained.

## 6. Claims and assumptions

### CLM-DAR-001 — the missing seam is dispatch-bound lineage projection

- Support state: SUPPORTED.
- Supported by evidence: `EVD-DAR-001`.
- Contradicted by evidence: none known.
- Uncertainty: production observations are time-bound; all identity and
  dispatch predicates require fresh read-back before controlled apply.

## 7. Evidence relations

### EVD-DAR-001 — current facts support a bounded missing-seam claim

- Source observations: `OBS-DAR-001`, `OBS-DAR-002`, `OBS-DAR-003`.
- Target: `CLM-DAR-001`, `STATE-DAR-001`.
- Relation: SUPPORTS.
- Bound coordinates: svc main `f525d5575906bcdb46a246194f77a67c21a19604`,
  dsh main `5a5395246e7cbd7412101167d8a99042c15db1aa`, production
  subject and read-back observed 2026-09-09.
- Strength/sufficiency: sufficient to select a new dispatch-bound read rather
  than mutate the due feed, Auth mapping or historical assignment.
- Limitations: does not prove implementation, deployment, caller credentials,
  Grant, Session delivery or business completion.
- Provenance: named source/authority paths and sanitized Goal census.

## 8. Decisions

### DEC-DAR-001 — add a dispatch-bound resolution read

- Decision owner: `mayf3/svc-workflow` maintainers.
- Decision: add one internal read endpoint that binds Instance, NodeVisit and
  Dispatch Intent and returns historical plus resolver Principal identities.
- Rejected alternatives: `ALT-DAR-001`, `ALT-DAR-002`.
- Reason: preserve existing due-feed and assignment meanings while exposing the
  already accepted lineage only at the delivery obligation where it is needed.

### DEC-DAR-002 — keep Auth as the Agent identity authority

- Decision owner: `mayf3/svc-workflow` maintainers.
- Decision: Workflow returns only Principal UUIDs and lineage kind; the caller
  must invoke the external Auth exact resolver and Agent-definition validation.
- Rejected alternatives: `ALT-DAR-003`, `ALT-DAR-004`.
- Reason: Workflow lineage is provenance, not current Agent identity authority.

### DEC-DAR-003 — fail closed on drift

- Decision owner: `mayf3/svc-workflow` maintainers.
- Decision: if coordinates are not the current active due Dispatch Intent or
  lineage is inconsistent, return no identity and perform no repair.
- Rejected alternative: stale positive reuse or automatic lineage repair.
- Reason: a wrong-target admission is worse than a visible no-delivery outcome.

## 9. Contracts

### CTR-DAR-001 — exact endpoint and closed schema

Expose exactly:

```text
GET /internal/v1/workflow-instances/{workflowInstanceId}
    /node-visits/{nodeVisitId}/dispatch-assignee-resolution
    ?dispatchIntentId=<UUID>
```

The three coordinates are required canonical UUIDs. Unknown query parameters,
malformed UUIDs or duplicate parameters return `422 invalid_input` before a
database read. Success is exactly:

```json
{
  "workflowInstanceId": "<UUID>",
  "nodeVisitId": "<UUID>",
  "dispatchIntentId": "<UUID>",
  "assignedPrincipalId": "<UUID>",
  "resolverPrincipalId": "<UUID>",
  "resolutionKind": "DIRECT | SUCCESSOR"
}
```

No Agent ID, display name, metadata, lineage evidence blob, Context, business
payload, token, credential, Grant or transition option is returned.

### CTR-DAR-002 — authorization and snapshot binding

Require a direct token with `workflow.read` and one enabled server-side
`GLOBAL_SCHEDULER_READ` binding, identical in authority to the due feed. No
Reader, Coordinator, Domain role or possession of an ID implies this role.
Role check and fact resolution occur in one read-only `REPEATABLE READ`
transaction. Missing scope follows normal Auth denial; missing binding returns
`403 scheduler_read_role_required`. The response never discloses whether supplied
coordinates exist before both authorization gates pass.

Every success and every authenticated denial MUST append one durable,
non-sensitive protected-read audit record before publishing the HTTP response,
as required by `SVC_WORKFLOW_ARCHITECTURE_V0_4_1#CTR-ARCH-039`. The audit binds
the authenticated actor, operation, request/correlation identity, timestamp and
closed outcome class (`SUCCESS`, `AUTHORIZATION_DENIED`,
`DISPATCH_INTENT_NOT_CURRENT`, or `DISPATCH_IDENTITY_UNAVAILABLE`). A denial
record MUST NOT disclose resolved identity fields or distinguish which hidden
coordinate predicate failed. Required audit is retained for exactly 365 days
through the existing audit lifecycle. If the audit append or its durable
confirmation is unavailable, publication fails closed with `503
audit_unavailable` and no identity body; the response MUST NOT be published from
a cached positive result. The business/identity snapshot itself remains
read-only; the audit is the only write authorized by this read operation.

Inside that snapshot, all predicates are conjunctive:

1. Instance exists, is not cancelled/archived, and its current NodeVisit equals
   the supplied NodeVisit;
2. NodeVisit belongs to that Instance and has one non-null assignee;
3. supplied Dispatch Intent identifies the unique activation for that NodeVisit,
   whose kind is `DISPATCH_INTENT` and owner equals the NodeVisit assignee;
4. no activation closure exists;
5. current `nextEligibleAt <=` the same authoritative transaction time.

Any failed predicate returns `409 dispatch_intent_not_current` with no identity
fields and no writes. The endpoint never calls wake.

### CTR-DAR-003 — exact zero-or-one-edge lineage

Resolve `assignedPrincipalId` with the existing immutable successor surface:

- no lineage row: `resolverPrincipalId = assignedPrincipalId`,
  `resolutionKind = DIRECT`;
- exactly one row: `resolverPrincipalId = successor_principal_id`,
  `resolutionKind = SUCCESSOR`.

Require the selected resolver Principal to exist in the Workflow Principal
projection, be type `AGENT`, be enabled, and have an enabled membership in the
Instance Domain. Require no second lineage edge from the selected successor and
no self-edge. Any missing, disabled, wrong-type, second-hop or inconsistent row
returns `409 dispatch_identity_unavailable`, never DIRECT and never a guessed
success. The database unique source constraint is necessary but not by itself a
success proof.

Do not read a display name, normalize an Agent ID, traverse multiple edges,
consult `external_ref`, infer from Domain membership, or use the lineage row's
recorded `canonical_agent_id` as current identity authority.

### CTR-DAR-004 — trust and use boundary

The response is a time-indexed Workflow observation. It grants no authority to
act as either Principal and is not a lease. svc-workflow MUST publish both
`assignedPrincipalId` and `resolverPrincipalId` with their distinct meanings,
even when their UUID values are equal for `DIRECT`; it MUST NOT collapse one into
the other or claim an Agent resolution. Assignment ownership, target own-context
visibility and historical events continue to use Workflow authority.

External consumers remain governed by their own accepted authority. This Spec
only declares interoperability: `resolverPrincipalId` is the exact Principal
UUID intended for a separately authorized external exact-Principal resolution;
the response itself never proves an Agent, Session admission or delivery.

### CTR-DAR-005 — compatibility and zero mutation

The existing due-feed request/response, seven fields, ordering, continuation,
wake behavior, activation facts, NodeVisit rows, worklist/detail responses and
error meanings remain byte-for-meaning unchanged. This endpoint performs zero
business, identity-repair, eligibility, receipt or lineage writes. Its only
write is the required protected-read audit in `CTR-DAR-002`. It introduces no
schema migration and no generic alias semantics.

### CTR-DAR-006 — controlled release

Implementation is limited to one query/repository method, one handler/route,
closed response types, focused tests and generated contract artifacts already
required by repository policy. No unrelated refactor.

Production apply requires: accepted authority in main; exact implementation
head independently reviewed with blocker union empty; current main impact
recheck; existing migrations through 25; svc binary preimage and rollback
artifact; deployment serialization gate free; fresh protected call proving the
real scheduler Principal's scope and role; read-before/read-after; health and
ready checks; and a durable sanitized deployment receipt. Deployment alone does
not authorize wake or dsh activation.

Rollback restores the exact binary/config preimage. Because the endpoint is
read-only and adds no storage, rollback deletes no facts. After any unknown
deployment outcome, inspect process, binary and health state before retrying.

## 10. Acceptance

### ACC-DAR-001 — closed wire surface

- Contracts: `CTR-DAR-001`, `CTR-DAR-005`.
- Method/environment: handler plus HTTP contract tests in an isolated database.
- Expected: exact success keys; malformed/unknown/duplicate inputs rejected;
  existing due/wake snapshots unchanged.
- Failure: any extra field, changed existing response, business/identity/
  eligibility/lineage write, or missing required protected-read audit.

### ACC-DAR-002 — authorization matrix

- Contracts: `CTR-DAR-002`.
- Method/environment: real scope verifier with role fixtures.
- Expected: only `workflow.read` plus enabled `GLOBAL_SCHEDULER_READ` succeeds;
  role-less/disabled/other-role authenticated callers fail with zero identity
  disclosure; success and authenticated denial each append one sanitized audit
  before response publication; forced audit outage returns `503
  audit_unavailable` with no identity body and no cached publication.
- Failure: client-side-only gate, implicit role equivalence, response before
  durable audit, sensitive denial audit, missing audit or fail-open audit outage.

### ACC-DAR-003 — lineage and drift matrix

- Contracts: `CTR-DAR-002`, `CTR-DAR-003`.
- Method/environment: PostgreSQL integration test.
- Expected: DIRECT and one-edge SUCCESSOR positives; rejects mismatched
  Instance/Visit/activation/owner, closed/not-due intent, missing/disabled/
  wrong-type/non-member resolver Principal, self-edge and second hop.
- Failure: any guessed, transitive or stale success.

### ACC-DAR-004 — no Agent-ID shortcut

- Contracts: `CTR-DAR-003`, `CTR-DAR-004`.
- Method/environment: repository/HTTP response fixture with misleading lineage
  `canonical_agent_id` and display-name collision.
- Expected: Workflow never returns or uses an Agent ID; both Principal UUID
  fields retain their named meanings, including an equal-valued DIRECT result.
- Failure: any Agent-ID field, routing claim, display-name dependency or collapse
  of assignment identity into resolver identity.

### ACC-DAR-005 — production read-only proof

- Contracts: `CTR-DAR-006`.
- Method/environment: controlled production deployment then one protected read
  for the frozen real subject, before any wake or dsh poller enablement.
- Expected: exact assigned Principal preserved; exact recorded successor returned
  as resolver Principal; zero Workflow version/Event/eligibility/receipt delta;
  exactly one sanitized protected-read success audit precedes the response;
  health and ready remain green; rollback remains executable.
- Failure: wrong coordinates, mutation, missing receipt, unhealthy service or
  unresolved deployment outcome.

### Contract coverage

| Contract | Acceptance | Covered |
|---|---|---|
| `CTR-DAR-001` | `ACC-DAR-001` | YES |
| `CTR-DAR-002` | `ACC-DAR-002`, `ACC-DAR-003` | YES |
| `CTR-DAR-003` | `ACC-DAR-003`, `ACC-DAR-004` | YES |
| `CTR-DAR-004` | `ACC-DAR-004` | YES |
| `CTR-DAR-005` | `ACC-DAR-001` | YES |
| `CTR-DAR-006` | `ACC-DAR-005` | YES |

## 11. Alternatives and disposition

### ALT-DAR-001 — replace due-feed owner with successor

- Disposition: rejected.
- Reason: it collapses immutable assignment history into delivery identity and
  changes an accepted existing surface.
- What would reopen: whole-authority supersession explicitly choosing new feed
  semantics; not needed for this Goal.

### ALT-DAR-002 — add a resolver field to the due feed

- Disposition: rejected for V1.
- Reason: it changes the exact seven-field contract for every consumer.
- What would reopen: a separately justified feed-versioning requirement.

### ALT-DAR-003 — route from recorded canonical Agent ID

- Disposition: rejected.
- Reason: Workflow evidence is not current Auth/Agent authority.
- What would reopen: none under the current authority split.

### ALT-DAR-004 — normalize or alias identity outside accepted lineage

- Disposition: rejected.
- Reason: Auth-side display-name normalization, hard-coded UUID mapping or
  multi-hop aliases create wrong-target and privilege-confusion risk.
- What would reopen: a separately accepted Auth identity model; not present.

### ALT-DAR-005 — mutate existing business facts

- Disposition: rejected.
- Reason: rebinding the Visit or creating a replacement Instance is historical
  mutation/business substitution and explicitly outside the Goal.
- What would reopen: none for the frozen real subject.

## 12. Migration, compatibility, and rollback

```text
MIGRATION = NONE; migration 0025 is reused read-only
COMPATIBILITY = additive endpoint; due feed, wake, worklist and detail unchanged
ROLLBACK = restore exact svc binary/config preimage; delete no facts
EMERGENCY_CONTAINMENT = disable or roll back the new endpoint/consumer only
```

No accepted authority is superseded. This new bounded surface refines existing
activation and lineage authorities without changing their stable Contract IDs.

## 13. Open questions and author status

```text
OPEN_OWNER_DECISIONS = accept or reject DEC-DAR-001..003 at exact reviewed head
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
CONTRACT_COUNT = 6
CONTRACTS_WITH_ACCEPTANCE = 6
AUTHORING_READY_FOR_REVIEW = YES
READY_TO_MARK_ACCEPTED = NO
IMPLEMENTATION_READY = NO
PRODUCTION_READY = NO
NEXT_ACTION = independent exact-head semantic review
```
