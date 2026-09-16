---
spec_id: SVC_WORKFLOW_ACTIVE_AGENT_LIST_V1
status: superseded
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
title: Canonical Active Agent global-list projection and filter V1
repo: mayf3/svc-workflow
date: 2026-09-16
candidate_base: ed99fa06a3067fe1d230699e9fed2b19542ab190
scope:
  - GET /internal/v1/workflow-instances/global
  - currentExecutorType query parameter
  - current_executor_type summary projection
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V8
external_authorities: []
related_authorities:
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_3
  - SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1
  - SVC_WORKFLOW_WORK_EXECUTION_CLASS_V1
supersedes: []
superseded_by: SVC_WORKFLOW_ACTIVE_AGENT_LIST_V2
owners:
  - mayf3
accepted_by: mayf3
accepted_date: 2026-09-16
accepted_reviewed_head: 34b2c6e90d5a7f02a6690b189f97cb901a47dd43
acceptance_review_verdict: PASS
acceptance_record: docs/reports/WORKFLOW_ACTIVE_AGENT_LIST_AUTHORITY_ACCEPTANCE_V1.md
---

# SVC_WORKFLOW_ACTIVE_AGENT_LIST_V1

This is a docs-only implementation-authority candidate. It changes no source,
schema, deployment, role, grant, Workflow Instance, or production data. Its
Contracts remain inert while status is `proposed`. Implementation may begin
only after this Spec and its Architecture parent are independently reviewed,
Owner-accepted, merged to `main`, and present as accepted authority in the
implementation base.

## 1. Goal

Implement the exact Product Boundary V8 global-list contract so HR can express:

```text
lifecycle=active
&status=active
&currentExecutorType=AGENT
```

The server derives executor type only from the exact current Visit assignee's
canonical svc-workflow Principal projection. The result is a read projection
and filter, not stored state, dispatch readiness, identity repair, or a new work
authority.

## 2. Scope and non-goals

The implementation closure is limited to:

```text
src/http/dto.rs
src/http/handlers/instances.rs
src/application/workflow_instance/query_types.rs
src/store/postgres/workflow_instance_repository/query_global_instances.rs
tests/23_global_coordinator.rs
```

A later implementation review may replace the single test path above with an
already-existing dedicated global-list test module only if the reviewed
candidate proves identical behavioral coverage and no production-file scope
expansion.

In scope:

- optional camelCase query parameter `currentExecutorType`;
- accepted input values exactly `HUMAN | AGENT`;
- nullable snake_case response member `current_executor_type`;
- derivation from current Visit `assignee_principal_id -> principals.principal_type`
  in the same repeatable-read list snapshot;
- exact equality filtering when the parameter is present;
- stable downstream error `invalid_current_executor_type` for any other value;
- focused repository and HTTP tests for projection, filtering, pagination, and
  existing lifecycle/status composition.

Out of scope:

- a database field, migration, index, materialized view, cache, mapping table, or
  second identity authority;
- any role, permission, scope, grant, credential, or route-gate change;
- any Domain-list or assigned-to-me contract change;
- title, content, display name, session ID, external reference, agent ID,
  successor-line, activation, eligibility, execution class, node key, or
  metadata inference;
- Auth runtime-health validation, task readiness, future-node inspection, test
  suppression, quarantine suppression, or business disposition;
- Workflow mutation, transition, cancellation, archival, cleanup, deployment,
  or HR delivery;
- changing the meaning or defaults of `lifecycle` or `status`.

## 3. Authority and dependency graph

```text
SVC_WORKFLOW_PRODUCT_BOUNDARY_V8 (accepted product authority)
  -> SVC_WORKFLOW_ARCHITECTURE_V0_4_3 (proposed whole Architecture successor)
       -> SVC_WORKFLOW_ACTIVE_AGENT_LIST_V1 (this proposed implementation Spec)
            -> AGENT_CORE_WORKFLOW_GLOBAL_INSTANCES_CAPABILITY_V3
               (separate dsh-agent-core proposed Broker companion)
```

`SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1` remains the route authorization
authority and is not superseded. This Spec adds no reader. The separately
proposed Architecture must be accepted first or atomically with this Spec's
acceptance dependency closure. The dsh companion must pin the final accepted
svc-workflow coordinate before its own acceptance.

This Spec supersedes no implementation Spec: the existing global Reader Spec
owns role gating, while this Spec owns only the additive V8 query/projection
contract.

## 4. Current evidence

At production observation
`2026-09-16T07:05:58.285738+08:00`, database
`svc_workflow_dogfood_clean` had a frozen product-ACTIVE denominator of 32:

```text
ACTIVE + AGENT = 15
ACTIVE + HUMAN = 17
AGENT_ACTIONABLE = 6
INVALID_OR_BROKEN_AGENT = 7
OWNER_BOUND_AGENT = 2
UNKNOWN = 0
```

Nine later ACTIVE rows were recorded separately and do not alter that frozen
denominator. At `2026-09-16 08:30:03.549822+08`, the current product-ACTIVE
set was 41 = 24 AGENT + 17 HUMAN.

The deployed split query semantics were also mechanically observed:

```text
node_type <> TERMINAL                         = 270
cancelled=false AND archived_at IS NULL
  AND node_type <> TERMINAL                  = 41
cancelled=true AND node_type <> TERMINAL     = 229
archived_at IS NOT NULL AND non-terminal     = 22
```

Therefore `lifecycle=active` alone is not the product ACTIVE predicate.
Product V8 already freezes the required conjunction and current-executor
authority. Current main and the deployed source expose neither
`currentExecutorType` nor `current_executor_type`.

## 5. Decisions

### DEC-AAL-001 — Server owns executor projection

The sole executor-type authority for this surface is the exact current Visit
assignee's svc-workflow Principal `principal_type`. The service derives it
inside the list query snapshot. Callers and Brokers do not reproduce it.

### DEC-AAL-002 — Preserve split lifecycle/status vocabulary

No existing parameter changes meaning. Product ACTIVE remains the explicit
composition `lifecycle=active&status=active`. Omitting `status` while
supplying `lifecycle` retains the existing `status=all` behavior.

### DEC-AAL-003 — Exact current-step type, no readiness classifier

The filter is exact `HUMAN | AGENT` equality only. Disabled Auth clients,
missing external runtime, stale business data, deprecated Definitions,
quarantine, future-chain health, activation, eligibility, and execution class
do not alter membership. Such facts are repaired outside the list contract.

### DEC-AAL-004 — Nullable projection outside executable work

A non-terminal current Visit with an allowed HUMAN or AGENT Principal projects
that value. Terminal summaries project null. Any non-terminal current Visit
whose Principal type is neither HUMAN nor AGENT is an internal consistency
error; it is never emitted as a third executor value or guessed.

### DEC-AAL-005 — Production equality is a data-plus-contract gate

The query contract proves exact `ACTIVE + current executor AGENT`. It does not
hide invalid Agent-owned rows. Production claims that HR-visible equals
business-dispatchable require a fresh data gate proving every returned current
Agent owner is valid and every invalid/historical row has been repaired or
lawfully removed from the ACTIVE work surface.

## 6. Contracts

### CTR-AAL-001 — Input contract

`GlobalInstanceQuery` accepts optional `currentExecutorType`. Values are
case-sensitive `HUMAN` and `AGENT`. Invalid or empty values return HTTP 422
with code `invalid_current_executor_type`; they never broaden to all.

### CTR-AAL-002 — Canonical projection

Each summary adds nullable `current_executor_type`. Its only non-null values
are `HUMAN | AGENT`, derived through:

```text
workflow_instances.current_node_visit_id
  -> workflow_node_visits.assignee_principal_id
  -> principals.principal_type
```

The derivation and result page use one repeatable-read snapshot. No successor
mapping, Auth lookup, display-name lookup, or content inference participates.

### CTR-AAL-003 — Filter contract

When supplied, `currentExecutorType` applies server-side before ordering,
limit, and cursor construction. Pagination over all pages is complete and
duplicate-free for the filtered set. The filter never changes caller identity,
authorization, or current assignee identity.

### CTR-AAL-004 — Exact ACTIVE Agent expression

The only HR V0 expression authorized by this Spec is:

```text
lifecycle=active&status=active&currentExecutorType=AGENT
```

It equals:

```sql
wi.cancelled = false
AND wi.archived_at IS NULL
AND current_node.node_type <> 'TERMINAL'
AND current_assignee_principal.principal_type = 'AGENT'
```

No one parameter is a synonym for this conjunction.

### CTR-AAL-005 — Compatibility

All existing route gates, response fields, ordering, cursor encoding,
pagination bounds, lifecycle/status defaults, error envelopes, and filters
remain unchanged when the new parameter is absent. No migration is produced.

### CTR-AAL-006 — Invalid owners remain data defects

An ACTIVE row projected as AGENT remains in the AGENT result even when external
Agent resolution would later fail. The service must not hide it using runtime,
identity-health, Domain-enabled, test, quarantine, title, or future-chain
filters. Production activation is blocked until the data invariant is clean.

### CTR-AAL-007 — Single delivery authority mode

This read contract does not itself dispatch. Architecture V0_4_3 requires the
activation-driven dispatcher and HR V0 list-driven sender to be mutually
exclusive production delivery-authority modes for the same environment. This
Spec adds no mode field, lease, claim, tombstone, consumption key, or dispatch
ledger.

### CTR-AAL-008 — Separation of gates

Spec acceptance, implementation review, merge, deployment, data cleanup, HR
activation, and real dispatch are separate gates. No earlier gate authorizes a
later one.

## 7. Acceptance

### ACC-AAL-001 — Projection matrix

Create isolated fixtures for ACTIVE HUMAN, ACTIVE AGENT, terminal, cancelled,
archived, and forbidden SERVICE ownership. Prove exact projection values,
terminal null, and fail-closed non-terminal SERVICE behavior.

### ACC-AAL-002 — Filter matrix

For both executor values and omitted filter, prove exact membership across
multiple Domains and pages. Invalid enum values return the exact 422 code.

### ACC-AAL-003 — ACTIVE Agent conjunction

Prove the three-filter query includes every valid ACTIVE Agent fixture and
excludes HUMAN, cancelled, archived, and terminal fixtures. Independently query
the canonical DB relation in the same test dataset and compare exact ID sets.

### ACC-AAL-004 — No false positive or false negative

At production activation time, after separately authorized instance repair and
cleanup, compare complete paginated API IDs with the frozen canonical
dispatchable-Agent ledger:

```text
HR_FALSE_POSITIVE_COUNT=0
HR_FALSE_NEGATIVE_COUNT=0
HR_VISIBLE_SET_EQUALS_CANONICAL_DISPATCHABLE_AGENT_SET=YES
```

A mismatch blocks activation; it never authorizes a new filter.

### ACC-AAL-005 — Compatibility and authorization

Run existing global-list authorization, lifecycle/status default, cursor,
ordering, and error-envelope suites with the parameter absent. Prove no role,
scope, route, or response field regression beyond the additive nullable field.

### ACC-AAL-006 — No forbidden derivation

Static and behavioral checks prove the implementation does not read title,
description, metadata, session ID, display name, external reference,
canonical-agent successor enrichment, activation, eligibility, execution class,
or any external identity/runtime source to derive executor type.

### ACC-AAL-007 — Cross-repository conformance

Against the separately accepted dsh Broker companion, send the exact three
filters and prove the downstream request bytes and returned IDs/types are
unchanged by Broker processing.

## 8. Rejected alternatives

- Reuse or accept historical dispatchability PR #19: rejected by Product V8; it
  is stale V5-bound authority and includes broader classifier semantics.
- Broker/HR-side HUMAN/AGENT inference: rejected because svc-workflow owns the
  canonical current executor relation.
- Store a new executor column or mapping: rejected because the relation already
  exists and storage would create drift.
- Infer from title, session, display name, external reference, node, activation,
  eligibility, or execution class: rejected as a second business authority.
- Change `lifecycle=active` to include status semantics implicitly: rejected
  because existing split vocabulary is preserved.
- Filter invalid Agent owners from this query: rejected because it hides
  Workflow data defects and creates false negatives.

## 9. Migration, compatibility, rollback

There is no data migration. Before deployment, rollback is code reversion.
After deployment, rollback removes the optional input/projection only after the
dsh companion and HR caller are disabled or reverted in dependency order.
Rollback never mutates Workflow data.

## 10. Candidate gate

```text
SPEC_STATUS=proposed
ARCHITECTURE_STATUS_REQUIRED=accepted
IMPLEMENTATION_AUTHORITY=none
PRODUCT_CODE_CHANGE=none
MERGE_AUTHORIZED=NO
DEPLOY_AUTHORIZED=NO
PRODUCTION_MUTATION_AUTHORIZED=NO
NEXT_GATE=INDEPENDENT_SEMANTIC_REVIEW
```
