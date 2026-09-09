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

## 3. Current state and observations

### STATE-DAR-001 — first real subject is blocked before delivery

- Source main: `f525d5575906bcdb46a246194f77a67c21a19604`.
- Production observation time: 2026-09-09.
- Instance: `cebf4816-c664-40cb-9b61-3fa330ad1c39`.
- Current NodeVisit: `75a134e8-bd5f-4d3a-af2d-469da433e0c5`.
- Dispatch Intent: `9ccd0359-d23b-4122-939b-9180a722fd7f`.
- Historical assignee: `61819256-07e1-4bd0-adea-e93e51243fa1`.
- Recorded successor: `9e3adced-575f-4fb2-b351-f7698b59127d`.
- Result: the intent is active and due; no successful delivery fact was found.

### OBS-DAR-001 — due feed preserves the historical owner

At source main, `query_dispatch_intents.rs` projects the activation's
`owner_principal_id`. Accepted `CTR-VAI-009` and `CTR-DKC-002/003` freeze the
seven-field feed and its no-cursor compatibility. Changing that field's meaning
or adding an eighth field would alter an accepted contract.

### OBS-DAR-002 — an accepted exact lineage already exists

Migration `0025_workflow_identity_successor_lines.sql` provides one immutable
row per stale source Principal; `identity_successor::resolve_current_principal`
returns the exact successor for one edge and otherwise the exact source. Query
visibility/detail already consume the same lineage without rewriting the
NodeVisit assignee.

### OBS-DAR-003 — direct Auth resolution of the historical subject fails closed

Fresh Auth read-back showed the historical Principal carries a non-canonical
stored Agent ID, while the recorded successor is active and carries the expected
canonical Agent ID. The accepted dsh exact resolver rejects the historical
stored ID grammar. Replacing that rejection with name normalization or a local
Agent-ID fallback would violate its authority.

## 4. Claims and evidence

### CLM-DAR-001 — the missing seam is dispatch-bound lineage projection

- Support state: SUPPORTED.
- Source observations: `OBS-DAR-001`, `OBS-DAR-002`, `OBS-DAR-003`.
- Relation: the due feed correctly preserves assignment history and Auth
  correctly fails closed; the already accepted Workflow lineage is the only
  formal bridge, but no dispatch-bound read exposes it.
- Sufficiency: enough to select a new bounded read surface.
- Limitations: not implementation, deployment, credential, Grant, Session, or
  business-completion evidence.

## 5. Decisions

### DEC-DAR-001 — add a dispatch-bound resolution read

Add one internal read endpoint that binds Instance, NodeVisit and Dispatch
Intent and returns historical plus resolver Principal identities. It is not a
generic `Principal -> Principal` directory.

### DEC-DAR-002 — keep Auth as the Agent identity authority

Workflow returns only Principal UUIDs and lineage kind. The caller must invoke
the accepted external Auth exact resolver and exact local Agent-definition
validation. `workflow_identity_successor_lines.canonical_agent_id` remains
audit evidence and MUST NOT be returned or used for routing.

### DEC-DAR-003 — fail closed on drift

If the supplied coordinates are not the one current, active, due Dispatch
Intent or lineage is inconsistent, return no identity. Never return a stale
positive result and never repair automatically.

## 6. Contracts

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
act as either Principal and is not a lease. A dispatcher uses only the returned
`resolverPrincipalId` as the UUID input to the separately authorized Auth exact
resolver. Auth and the destination Agent registry must independently return one
enabled canonical Agent before admission. Any later drift fails closed; there is
no cached positive reuse across admission attempts or poll passes.

The dispatcher ledger records both `assignedPrincipalId` and
`resolverPrincipalId` plus `resolutionKind`; they must never be collapsed into
one field. Assignment ownership, target own-context visibility and historical
events continue to use Workflow authority, not the dispatcher's ledger.

### CTR-DAR-005 — compatibility and zero mutation

The existing due-feed request/response, seven fields, ordering, continuation,
wake behavior, activation facts, NodeVisit rows, worklist/detail responses and
error meanings remain byte-for-meaning unchanged. This endpoint performs zero
business, audit-repair, eligibility, receipt or lineage writes. It introduces no
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

## 7. Acceptance

### ACC-DAR-001 — closed wire surface

- Contracts: `CTR-DAR-001`, `CTR-DAR-005`.
- Method/environment: handler plus HTTP contract tests in an isolated database.
- Expected: exact success keys; malformed/unknown/duplicate inputs rejected;
  existing due/wake snapshots unchanged.
- Failure: any extra field, changed existing response or write.

### ACC-DAR-002 — authorization matrix

- Contracts: `CTR-DAR-002`.
- Method/environment: real scope verifier with role fixtures.
- Expected: only `workflow.read` plus enabled `GLOBAL_SCHEDULER_READ` succeeds;
  role-less/disabled/other-role callers fail with zero identity disclosure.
- Failure: client-side-only gate or implicit role equivalence.

### ACC-DAR-003 — lineage and drift matrix

- Contracts: `CTR-DAR-002`, `CTR-DAR-003`.
- Method/environment: PostgreSQL integration test.
- Expected: DIRECT and one-edge SUCCESSOR positives; rejects mismatched
  Instance/Visit/activation/owner, closed/not-due intent, missing/disabled/
  wrong-type/non-member resolver Principal, self-edge and second hop.
- Failure: any guessed, transitive or stale success.

### ACC-DAR-004 — no Agent-ID shortcut

- Contracts: `CTR-DAR-003`, `CTR-DAR-004`.
- Method/environment: composed fixture with misleading lineage
  `canonical_agent_id`, display-name collision and Auth resolver stub.
- Expected: Workflow never returns/uses an Agent ID; only exact returned UUID is
  passed to Auth; Auth failure admits zero Runs.
- Failure: routing from Workflow evidence or fallback.

### ACC-DAR-005 — production read-only proof

- Contracts: `CTR-DAR-006`.
- Method/environment: controlled production deployment then one protected read
  for the frozen real subject, before any wake or dsh poller enablement.
- Expected: exact assigned Principal preserved; exact recorded successor returned
  as resolver Principal; zero Workflow version/Event/eligibility/receipt delta;
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

## 8. Alternatives

- Changing due-feed `ownerPrincipalId` to the successor: rejected; it collapses
  immutable assignment history into delivery identity and changes an accepted
  existing surface.
- Adding `resolverPrincipalId` to the due-feed row: rejected for V1; it changes
  the exact seven-field contract for every consumer.
- Routing from lineage `canonical_agent_id`: rejected; Workflow evidence is not
  current Auth/Agent authority.
- Auth-side display-name normalization, hard-coded UUID mapping or multi-hop
  aliases: rejected; wrong-target and privilege-confusion risk.
- Rebinding the Visit or creating a replacement Instance: rejected; historical
  mutation/business substitution and explicitly outside the Goal.

## 9. Author status

```text
OPEN_OWNER_DECISIONS = DEC-DAR-001 acceptance
NORMATIVE_TBD = NONE
PARTIAL_SUPERSESSION = NONE
CONTRACT_COUNT = 6
CONTRACTS_WITH_ACCEPTANCE = 6
AUTHORING_READY_FOR_REVIEW = YES
IMPLEMENTATION_READY = NO
PRODUCTION_READY = NO
NEXT_ACTION = independent exact-head semantic review
```
