# Workflow prerequisite orchestration — Svc preflight

```text
GOAL/TARGET = Propose Svc authority for atomic GCR child create-bind, an
  explicitly opted-in immutable same-Domain terminal-Submission prerequisite,
  typed readiness, and a separate READY-only scheduler projection.
CURRENT_GAP = GCR V1 binds only pre-created children; VAI V1 due read and WEP
  eligibility are time-only and define no cross-instance prerequisite state.
BASE = c8937e567a429160779ee7db0cb7b53efbe8ed3e
AUTHORITY_ACTION = SUPERSEDE (parent whole successors: Product V9, Architecture V0_4_4, WEP V2)
CHILD_AUTHORITY_ACTION = NEW (Svc and paired Core implementation Specs)
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
Instance/Visit/Event/Submission rows, current Owner and source visibility validation, and the normal poll. The
receipt projection is an explicit, narrowly bounded Product V9 permission
delta; no source payload or general read is added. The paired Core Spec is required. Proposed
`implementation_authority: contracts` takes effect only after acceptance in
the implementation base plus a separate mandate; production authority remains
none.

## Bounded outcome

Add proposed Svc Spec plus whole-authority Product V9, Architecture V0_4_4, and WEP V2 successors, proposed index/readme entries, and this preflight plus a bounded implementation plan. No wait ledger, prerequisite table, outbox,
arbitrary predicate DSL, new role/scope/Grant, or raw-due semantic change.

```text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1
STATUS = proposed
IMPLEMENTATION_AUTHORITY = contracts
PARENT_SUPERSEDE_TARGETS = Product V8; Architecture V0_4_3; WEP V1
CHILD_NEW_AUTHORITIES = SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1; AGENT_CORE_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1
CHILD_DEPENDS_ON = GCR V1; VAI V1; proposed Product V9; proposed Architecture V0_4_4; proposed WEP V2
PAIRED_ACCEPTANCE_PIN = exact reviewed Core/Svc companion heads recorded after independent review (no stale predecessor-base pin)
DOCS_FIRST_REQUIRED = YES
OPEN_OWNER_DECISIONS = NONE; parent decisions are explicit in proposed successor Contracts; exact authority acceptance remains later
NORMATIVE_TBD = NONE
PARTIAL_SUPERSESSION = NONE
CONTRACT_COUNT = 7
CONTRACTS_WITH_ACCEPTANCE = 7
AUTHORING_READY_FOR_REVIEW = YES (after paired docs commits; no implementation or acceptance)
NEXT_ACTION = REVIEW
```
