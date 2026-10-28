# Workflow prerequisite orchestration — Svc preflight

```text
GOAL/TARGET = Propose Svc authority for atomic GCR child create-bind, an
  explicitly opted-in immutable same-Domain terminal-Submission prerequisite,
  typed readiness, and a separate READY-only scheduler projection.
CURRENT_GAP = GCR V1 binds only pre-created children; VAI V1 due read and WEP
  eligibility are time-only and define no cross-instance prerequisite state.
BASE = c8937e567a429160779ee7db0cb7b53efbe8ed3e
AUTHORITY_ACTION = NEW
PLAN_LEVEL = EXEC_PLAN
ASSURANCE_LEVEL = CONTROLLED
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO
SPEC_GAP_DEPENDENCY = LOAD_BEARING
EXECUTION = docs only in isolated worktree; no code, acceptance, merge, push,
  live execution, or production apply.
```

Direct authority evidence: `SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V1` §1
excludes create-and-bind; VAI V1 CTR-VAI-009 preserves a time-only raw due
read; WEP V1's eligibility contract has no dependency status. This candidate
does not amend those meanings. It reuses published immutable Context Schema,
Instance/Visit/Event/Submission rows, current Owner and membership validation,
and the normal poll. The paired Core Spec is required. Proposed
`implementation_authority: contracts` takes effect only after acceptance in
the implementation base plus a separate mandate; production authority remains
none.

## Bounded outcome

Add one proposed Svc Spec, one proposed index row, and this preflight plus a
bounded implementation plan. No wait ledger, prerequisite table, outbox,
arbitrary predicate DSL, new role/scope/Grant, or raw-due semantic change.

```text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1
STATUS = proposed
IMPLEMENTATION_AUTHORITY = contracts
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_GOVERNANCE_CHILD_RETURN_V1
EXTERNAL_AUTHORITIES = mayf3/dsh-agent-core AGENT_CORE_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1 @ 01c1cd0c4a980e1a37878ea5ad05f4d46fb3ba99
DOCS_FIRST_REQUIRED = YES
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
PARTIAL_SUPERSESSION = NONE
CONTRACT_COUNT = 7
CONTRACTS_WITH_ACCEPTANCE = 7
AUTHORING_READY_FOR_REVIEW = YES
NEXT_ACTION = REVIEW
```
