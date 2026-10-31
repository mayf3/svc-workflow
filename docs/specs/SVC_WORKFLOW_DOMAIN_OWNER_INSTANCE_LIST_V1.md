---
spec_id: SVC_WORKFLOW_DOMAIN_OWNER_INSTANCE_LIST_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: none
production_apply_authority: none
scope:
  - owner-scoped read-only Domain workflow instance inventory API
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
  - SVC_WORKFLOW_ARCHITECTURE_V0_3_1
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_DOMAIN_OWNER_INSTANCE_LIST_V1

> **PROPOSED / DOCS ONLY.** This document freezes a candidate service contract. It does not
> authorize implementation, merge, deployment, restart, migration, database writes, or any
> production action.

## 1. Goal

```text
GOAL = let a DOMAIN_OWNER obtain a complete, bounded, read-only inventory of Workflow
       instances in exactly one Domain they currently own
SUCCESS_OUTCOME = a dedicated owner-scoped API has explicit identity, authorization,
                  projection, pagination, filtering, error, zero-write, and isolation contracts
STATUS = proposed
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
DATABASE_CHANGE_AUTHORIZED = NO
```

The existing Domain-list endpoint is reusable evidence and implementation material, but its
current response and failure semantics do not satisfy this Goal. This Spec therefore selects a
new dedicated endpoint rather than silently changing the existing endpoint's Member visibility
semantics.

## 2. Scope and non-goals

### In scope

- one new read-only endpoint: `GET /internal/v1/domains/{domainId}/workflow-instances`;
- authenticated caller identity derived only from the verified bearer token;
- current enabled `DOMAIN_OWNER` binding authorization for the path Domain;
- owner-safe summary projection, opaque page-token pagination, and bounded lifecycle/status
  filters;
- exact error codes and downstream `x-request-id` propagation already supplied by HTTP
  middleware;
- conformance tests proving authorization, isolation, pagination, required fields, and zero
  writes.

### Out of scope

- changing or removing `GET /internal/v1/workflow-instances/domain`;
- changing that existing endpoint's current non-owner `404
  workflow_instance_not_found_or_not_visible` behavior;
- arbitrary Principal impersonation, global/coordinator views, Member views, assignee override,
  Definition mutation, Instance mutation, assignment mutation, or Domain-binding mutation;
- returning Context payloads, submissions, event bodies, credentials, tokens, secrets, or raw
  Authorization material;
- schema migration, data backfill, deployment, restart, production grant, production database
  access, or Itops cutover changes.

## 3. Authority and dependencies

```text
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
PARENT_REVISION = 5cdd5eeb9895ce0bb4df1989f01806ca25b8ecff
ARCHITECTURE_AUTHORITY = SVC_WORKFLOW_ARCHITECTURE_V0_3_1
ARCHITECTURE_REVISION = 661842f3d86993de1b81c4d9f19ca6793f436088
AUTHORING_BASE = 2ff81ae47ab068216bd0012fa0e76a45dd2fb572
IMPLEMENTATION_AUTHORITY = none
EXTERNAL_AUTHORITIES = NONE
AUTHORITY_CONFLICT = NONE FOUND
```

This child refines the accepted Product Boundary's Workflow Instance and Domain-isolation scope.
It does not widen the Product Boundary and does not redefine Agent Core Broker behavior, which is
owned by `mayf3/dsh-agent-core`.

## 4. Current State

### STATE-DIL-001 — Existing Domain list is not the requested contract

- Subject: `mayf3/svc-workflow` source at authoring base
  `2ff81ae47ab068216bd0012fa0e76a45dd2fb572`.
- Environment: source tree only; no runtime or production claim.
- Observed at: `2026-08-27T01:36:49Z`.
- Projection: a Domain-scoped list endpoint already exists, but it (a) returns a camouflage 404
  for non-owners, (b) has no distinct `domain_not_found` path, (c) accepts an assignee filter, and
  (d) omits `workflow_state_version`, `cancelled`, and `archived` from each summary.
- Basis: `OBS-DIL-001` through `OBS-DIL-004`, `CLM-DIL-001`, `EVD-DIL-001`.

### STATE-DIL-002 — Current route has useful read-side machinery

- Subject, base, environment, and observation time: same as `STATE-DIL-001`.
- Projection: current code already has `workflow.read` scope enforcement, repeatable-read list
  queries, lifecycle/status filters, deterministic tuple pagination, and summary-only projection;
  these may be reused without preserving the old endpoint's incompatible wire contract.
- Basis: `OBS-DIL-001`, `OBS-DIL-003`, `CLM-DIL-002`, `EVD-DIL-002`.

## 5. Observations

### OBS-DIL-001 — Existing route and caller identity

- Subject: existing Domain instance list HTTP adapter.
- Repository/source: `mayf3/svc-workflow`.
- Commit: `2ff81ae47ab068216bd0012fa0e76a45dd2fb572`.
- Environment: source inspection.
- Observed at: `2026-08-27T01:36:49Z`.
- Method: inspect `src/http/mod.rs:114-117` and
  `src/http/handlers/instances.rs:96-155`.
- Result: `GET /internal/v1/workflow-instances/domain` takes `domainId` from query, takes actor
  Principal from `AuthenticatedPrincipal`, requires `workflow.read`, and calls
  `list_domain_instances`.
- Provenance: exact paths and symbols above.

### OBS-DIL-002 — Existing authorization cannot produce required errors

- Subject/commit/environment/time: same coordinates as `OBS-DIL-001`.
- Method: inspect `src/application/workflow_instance/query_service.rs:69-99`,
  `src/http/error.rs:503-529`, and test
  `tests/17_workflow_runtime/http/domain_list.rs::non_domain_owner_returns_404`.
- Result: a failed owner check becomes `WorkflowInstanceNotFoundOrNotVisible`, mapped to HTTP 404
  `workflow_instance_not_found_or_not_visible`; Domain absence and non-owner are not classified as
  `domain_not_found` and `not_domain_owner` for this endpoint.
- Provenance: exact files and test symbol above.

### OBS-DIL-003 — Existing projection and pagination are narrower than required

- Subject/commit/environment/time: same coordinates as `OBS-DIL-001`.
- Method: inspect `src/application/workflow_instance/query_types.rs:306-349` and
  `src/store/postgres/workflow_instance_repository/query_domain_instances.rs`.
- Result: the existing summary includes ids, definition key/version, assignee, current node,
  terminal boolean, optional Context-derived title, and timestamps; it does not include workflow
  state version, cancelled, or archived. It pages by `(created_at, workflow_instance_id)` using two
  caller-visible cursor fields and accepts `assigneePrincipalId`.
- Provenance: exact files above.

### OBS-DIL-004 — Existing contract explicitly preserves non-owner 404

- Subject/commit/environment/time: same coordinates as `OBS-DIL-001`.
- Method: inspect `contracts/workflow-http/v1/contract.md:272` and
  `contracts/workflow-http/v1/openapi.yaml` Domain-list operation.
- Result: the current internal API contract explicitly says non-owners get 404.
- Provenance: exact contract paths above.

## 6. Claims and assumptions

### CLM-DIL-001 — Silent mutation of the existing route is incompatible

- Support state: SUPPORTED.
- Supported by evidence: `EVD-DIL-001`.
- Contradicted by evidence: none known.
- Uncertainty: none material to the selected route boundary.

### CLM-DIL-002 — Existing machinery is reusable behind a new wire contract

- Support state: SUPPORTED.
- Supported by evidence: `EVD-DIL-002`.
- Contradicted by evidence: none known.
- Uncertainty: implementation structure remains bounded by the exact closure in
  `CTR-DIL-010`.

### CLM-DIL-003 — Opaque tuple pagination is safer than offset pages

- Support state: INFERRED.
- Supported by evidence: `EVD-DIL-003`.
- Contradicted by evidence: none known.
- Uncertainty: token encoding is an implementation detail only if it remains opaque, bounded,
  non-secret, and validates to the frozen tuple.

## 7. Evidence relations

### EVD-DIL-001 — Current errors and projection support a dedicated endpoint

- Source observations: `OBS-DIL-002`, `OBS-DIL-003`, `OBS-DIL-004`.
- Target: `CLM-DIL-001`.
- Relation: SUPPORTS.
- Bound coordinates: service authoring base `2ff81ae47ab068216bd0012fa0e76a45dd2fb572`.
- Strength/sufficiency: direct source and contract evidence; sufficient for endpoint selection.
- Limitations: no deployed-runtime claim.
- Provenance: observations above.

### EVD-DIL-002 — Current query design supports bounded reuse

- Source observations: `OBS-DIL-001`, `OBS-DIL-003`.
- Target: `CLM-DIL-002`.
- Relation: SUPPORTS.
- Bound coordinates: same authoring base.
- Strength/sufficiency: direct source evidence.
- Limitations: does not prove a future implementation conforms.
- Provenance: observations above.

### EVD-DIL-003 — Existing tuple cursor supports opaque page-token design

- Source observations: `OBS-DIL-003`.
- Target: `CLM-DIL-003`.
- Relation: SUPPORTS.
- Bound coordinates: same authoring base.
- Strength/sufficiency: sufficient for a keyset pagination decision.
- Limitations: does not select a serialization library.
- Provenance: observation above.

## 8. Decisions

### DEC-DIL-001 — Add a dedicated owner-scoped endpoint

- Decision owner: `mayf3` (direct task instruction).
- Decision: add `GET /internal/v1/domains/{domainId}/workflow-instances`; preserve the existing
  route and its behavior byte-for-meaning.
- Rejected alternatives: `ALT-DIL-001`, `ALT-DIL-002`.
- Reason: required error and projection semantics are incompatible with the existing contract.
- Owner decision remaining: NONE.

### DEC-DIL-002 — Authenticate caller; never accept substitute identity

- Decision owner: `mayf3`.
- Decision: actor identity comes only from verified bearer claims. Neither path, query, body, nor
  header may accept `principalId`, `actorPrincipalId`, or an equivalent override.
- Rejected alternatives: `ALT-DIL-003`.
- Reason: prevents Principal impersonation and preserves Domain authority.
- Owner decision remaining: NONE.

### DEC-DIL-003 — Use opaque keyset page tokens

- Decision owner: `mayf3`.
- Decision: request parameters are `page` and `limit`; `page` is an opaque server-issued token for
  `(updated_at, workflow_instance_id)`, not an offset or caller-assembled half cursor.
- Rejected alternatives: `ALT-DIL-004`.
- Reason: deterministic keyset pagination avoids offset omission/duplication under concurrent
  updates and prevents partial cursor input.
- Owner decision remaining: NONE.

## 9. Contracts

### CTR-DIL-001 — Route, method, scope, and identity

The service MUST expose exactly `GET /internal/v1/domains/{domainId}/workflow-instances` and MUST
require `workflow.read`. `domainId` MUST be a UUID path parameter. Actor identity MUST come only
from the authenticated bearer Principal. Unknown query fields MUST fail closed. The operation
MUST NOT accept a body or any Principal/actor override.

### CTR-DIL-002 — Ordered authorization classification

Within one read-only repeatable-read transaction, authorization MUST classify in this order:

1. caller Principal missing -> HTTP 404 `principal_not_found`;
2. caller Principal disabled -> HTTP 403 `principal_disabled`;
3. target Domain missing -> HTTP 404 `domain_not_found`;
4. no enabled binding matching `(domain_id, caller_principal_id, role_key='DOMAIN_OWNER')` ->
   HTTP 403 `not_domain_owner`;
5. otherwise continue.

An enabled `DOMAIN_MEMBER` or any other role is insufficient. Owning another Domain is
insufficient. The owner binding check MUST include `enabled = TRUE` and exact Domain equality.

### CTR-DIL-003 — Cross-Domain isolation

Every instance query MUST constrain `workflow_instances.domain_id` to the authorized path Domain.
No result, count, page token, error detail, timing-dependent secondary lookup, or joined row may
include another Domain's instance data. The endpoint MUST NOT offer an assignee Principal filter
or any other override that changes authorization scope.

### CTR-DIL-004 — Safe summary projection

Each item MUST contain at least:

```text
workflow_instance_id: UUID
definition_id: UUID                 # workflow_definition_id, not caller supplied
definition_version_id: UUID
definition_key: string              # stable display field; no Context title required
lifecycle: active | terminal
current_node: {
  node_id: UUID,
  node_key: string,
  display_name: string,
  node_type: string
}
current_assignee: UUID | null       # current Visit assignee only
state_version: positive integer     # workflow_state_version
cancelled: boolean
archived: boolean                   # archived_at IS NOT NULL
updated_at: RFC3339 timestamp
```

The response MUST NOT include Context payload/body, submissions, event payloads, instructions,
Credentials, tokens, secrets, Authorization values, or unrestricted joined objects. `definition_key`
is the required stable display field; a Context-derived business title MUST NOT be added by V1.

### CTR-DIL-005 — Lifecycle and status filters

Allowed filters are closed enums:

```text
lifecycle = active | terminal | all     (default all)
status    = active | cancelled | archived | all (default all)
```

`active` lifecycle means current node type is non-terminal; `terminal` means current node type is
terminal. `active` status means `cancelled = FALSE AND archived_at IS NULL`; `cancelled` means
`cancelled = TRUE`; `archived` means `archived_at IS NOT NULL`. Invalid lifecycle/status MUST
return HTTP 422 `invalid_lifecycle` / `invalid_status` without returning items.

### CTR-DIL-006 — Pagination contract

`limit` is optional, defaults to 20, and MUST be an integer from 1 through 100 inclusive. `page` is
optional and MUST be either absent or one complete opaque server-issued token. Ordering MUST be
`updated_at DESC, workflow_instance_id DESC`. The token MUST decode only to the last emitted
`(updated_at, workflow_instance_id)` tuple and MUST NOT carry identity, credential, secret,
Authorization, or business content. Invalid, oversized, partial, or mismatched tokens and invalid
limits MUST return HTTP 422 `invalid_pagination`.

Success MUST return:

```json
{"items": [], "next_page": null}
```

with `items` populated as applicable and `next_page` set only when another page exists. An
authorized empty Domain MUST return HTTP 200 with exactly an empty `items` array and no next page.

### CTR-DIL-007 — Error and request-id contract

The endpoint MUST preserve this closed service error contract:

| HTTP | code | condition |
|---:|---|---|
| 400 | `invalid_path_parameter` | malformed Domain UUID |
| 401 | `unauthenticated` | missing/invalid bearer authentication |
| 403 | `forbidden` | missing `workflow.read` scope |
| 403 | `principal_disabled` | caller projection disabled |
| 403 | `not_domain_owner` | caller lacks exact enabled OWNER binding |
| 404 | `principal_not_found` | caller projection absent |
| 404 | `domain_not_found` | target Domain absent |
| 422 | `invalid_lifecycle` | lifecycle outside closed enum |
| 422 | `invalid_status` | status outside closed enum |
| 422 | `invalid_pagination` | page/limit invalid |
| 500 | `internal_consistency_error` | impossible projection invariant |
| 503 | `service_unavailable` | storage unavailable |

Every response MUST retain the service middleware's `x-request-id`. Error messages MUST be
sanitized and MUST NOT echo page-token bytes, bearer material, SQL, raw database errors, or
business content.

### CTR-DIL-008 — Strict read-only and zero writes

The complete operation, including success and all failures, MUST execute no `INSERT`, `UPDATE`,
`DELETE`, DDL, event append, audit-row append, Receipt write, assignment change, binding change,
or workflow-state mutation. A read-only transaction and transaction commit are not business
writes. Frozen outcomes:

```text
DATABASE_WRITE_COUNT = 0
WORKFLOW_STATE_CHANGE = NONE
ASSIGNMENT_CHANGE = NONE
DOMAIN_BINDING_CHANGE = NONE
```

### CTR-DIL-009 — Existing API compatibility

`GET /internal/v1/workflow-instances/domain` MUST remain unchanged in route, accepted parameters,
projection, status defaults, pagination shape, and non-owner 404 behavior. Existing worklist,
instance-detail, submission-history, global-list, and Domain-discovery APIs MUST remain unchanged.

### CTR-DIL-010 — Exact implementation closure

A future implementation is limited to exactly these repository paths; adding another product,
test, migration, script, or contract path requires a separately reviewed Spec revision before
implementation:

```text
src/http/mod.rs
src/http/dto.rs
src/http/error.rs
src/http/handlers/instances.rs
src/application/workflow_instance/query_types.rs
src/application/workflow_instance/query_service.rs
src/store/postgres/workflow_instance_repository/mod.rs
src/store/postgres/workflow_instance_repository/query_owner_domain_instances.rs   # new
tests/17_workflow_runtime.rs
tests/17_workflow_runtime/http/owner_domain_list.rs                               # new
contracts/workflow-http/v1/openapi.yaml
contracts/workflow-http/v1/contract.md
contracts/workflow-http/v1/changelog.md
contracts/workflow-http/v1/conformance/run.sh
```

```text
MIGRATION_FILE_COUNT = 0
NEW_SERVER_ROUTE_COUNT = 1
EXISTING_SERVER_ROUTE_MUTATION_COUNT = 0
```

Acceptance of this proposed Spec while `implementation_authority: none` remains unchanged still
does not authorize edits to this closure.

## 10. Acceptance

### ACC-DIL-001 — Owner success and safe projection

- Contracts: `CTR-DIL-001`, `CTR-DIL-003`, `CTR-DIL-004`.
- Method: HTTP integration test with instances in owned and foreign Domains.
- Expected result: enabled owner receives 200; only owned-Domain items appear; every required field
  is present; forbidden payload/secret fields are absent.
- Failure condition: missing required field, foreign data, or sensitive/body content.

### ACC-DIL-002 — Authorization and error matrix

- Contracts: `CTR-DIL-002`, `CTR-DIL-007`.
- Method: distinct integration fixtures for owner of another Domain, enabled DOMAIN_MEMBER,
  nonexistent Domain, absent Principal projection, disabled Principal, and disabled owner binding.
- Expected result: exact status/code matrix, including 403 `not_domain_owner`, 404
  `domain_not_found`, and 404 `principal_not_found`; each response has `x-request-id`.
- Failure condition: 200, camouflage 404 for Member, wrong code/status, or missing request-id.

### ACC-DIL-003 — Empty Domain and filter behavior

- Contracts: `CTR-DIL-005`, `CTR-DIL-006`.
- Method: query an authorized empty Domain and a mixed active/terminal/cancelled/archived fixture.
- Expected result: empty Domain is `200 {items:[],next_page:null}`; each closed filter returns only
  matching rows; invalid filters fail 422.
- Failure condition: non-200 empty result, filter leak, or accepted unknown value.

### ACC-DIL-004 — Pagination boundaries

- Contracts: `CTR-DIL-006`.
- Method: exercise default, limits 1 and 100, limits 0 and 101, malformed/partial page tokens, and
  multi-page traversal with tie timestamps.
- Expected result: stable order, no duplicates/omissions in a stable fixture, correct next-page
  termination, and exact `invalid_pagination` failures.
- Failure condition: unsafe limit, offset semantics, half cursor, duplicate/omitted stable rows, or
  token content leak.

### ACC-DIL-005 — Zero-write proof

- Contracts: `CTR-DIL-008`.
- Method: instrument SQL execution/write paths and compare row counts plus immutable state/version,
  event, assignment, and binding digests before and after every success/failure scenario.
- Expected result: write-path call count and all business-row deltas are zero.
- Failure condition: any DML/DDL invocation or durable row/value change.

### ACC-DIL-006 — Existing endpoint non-regression

- Contracts: `CTR-DIL-009`, `CTR-DIL-010`.
- Method: run existing Domain-list/workflow HTTP tests and contract conformance suite; inspect Git
  diff path closure.
- Expected result: all existing tests pass; old Member behavior remains 404; changed paths are a
  subset of the exact closure; no migration exists.
- Failure condition: regression, path expansion, old wire change, or migration.

### Contract coverage

| Contract | Acceptance | Evidence class | Covered |
|---|---|---|---|
| `CTR-DIL-001` | `ACC-DIL-001` | executed integration | YES |
| `CTR-DIL-002` | `ACC-DIL-002` | executed integration | YES |
| `CTR-DIL-003` | `ACC-DIL-001` | executed isolation | YES |
| `CTR-DIL-004` | `ACC-DIL-001` | executed projection/security | YES |
| `CTR-DIL-005` | `ACC-DIL-003` | executed integration | YES |
| `CTR-DIL-006` | `ACC-DIL-003`, `ACC-DIL-004` | executed pagination | YES |
| `CTR-DIL-007` | `ACC-DIL-002` | executed error/request-id | YES |
| `CTR-DIL-008` | `ACC-DIL-005` | instrumented zero-write | YES |
| `CTR-DIL-009` | `ACC-DIL-006` | regression suite | YES |
| `CTR-DIL-010` | `ACC-DIL-006` | diff/closure inspection | YES |

## 11. Alternatives and disposition

### ALT-DIL-001 — Change the existing Domain-list endpoint in place

- Disposition: rejected.
- Reason: silently changes accepted non-owner visibility/error semantics and response shape.
- Evidence/Claims considered: `OBS-DIL-002` through `OBS-DIL-004`, `CLM-DIL-001`.
- What would reopen: a whole-contract successor explicitly migrating every existing caller.

### ALT-DIL-002 — Broker-only adaptation of the existing endpoint

- Disposition: rejected.
- Reason: Broker cannot manufacture missing state fields or distinguish absent Domain from
  non-owner without redefining service truth.
- Evidence/Claims considered: `OBS-DIL-002`, `OBS-DIL-003`.
- What would reopen: an existing service response already satisfying every frozen field/error.

### ALT-DIL-003 — Accept `principalId` for delegated lookup

- Disposition: rejected.
- Reason: enables identity substitution and bypasses authenticated caller authority.
- Evidence/Claims considered: direct Owner security boundary.
- What would reopen: a separate audited delegation authority and service trust model.

### ALT-DIL-004 — Offset page numbers or split caller cursors

- Disposition: rejected.
- Reason: offsets are unstable under mutation; split cursors permit partial/mismatched input.
- Evidence/Claims considered: `CLM-DIL-003`.
- What would reopen: no V1 reopening; any pagination replacement requires a successor Spec.

## 12. Migration, compatibility, and rollback

```text
MIGRATION = NONE; no schema or data migration
COMPATIBILITY = additive new route; old route byte-for-meaning unchanged
ROLLBACK = remove the additive route and its closed implementation paths; no data rollback
EMERGENCY_CONTAINMENT = disable/unroute only under separately authorized incident action
DATABASE_CHANGE = NONE
PRODUCTION_CHANGE = NONE
```

## 13. Open questions

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
READY_TO_MARK_ACCEPTED = NO
READY_FOR_INDEPENDENT_REVIEW = YES
NEXT_TASK = 域查 审计
```
