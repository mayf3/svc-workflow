# DEVELOPMENT_PREFLIGHT — prerequisite composite-contract closure

```text
GOAL/TARGET = Resolve FEAS-001 atomic create-bind event cardinality and FEAS-002 typed UNKNOWN coordinate closure through docs-only whole-authority successors.
CURRENT_GAP = Accepted Svc CTR-SWPO-003 requires one receipt/transaction but accepted Storage V0.1 + Architecture CTR-ARCH-020 require one Event per command globally; actual PG blocks the second required Event. Accepted CTR-SWPO-004 promises UNKNOWN for malformed references but its DTO always requires a full UUID triple.
AUTHORITY_ACTION = SUPERSEDE
PLAN_LEVEL = EXEC_PLAN
ASSURANCE_LEVEL = CONTROLLED
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO (the selected successor bundle is proposed only)
BASE = afb73d83c4804c894582a81af6907f48c13fa0bb
FEASIBILITY_REPORT_SHA256 = 8a9ad9365ec8186cd0c1e1b4e4f9797d60dcff203220db6b85c856ffbcf99e7f
ATOMIC_FEASIBILITY_BLOCKER_SHA256 = acf9064f83cd642a0111147d47fa880d942892454ce80d9d25e3376ee7bbb641
IMPLEMENTATION_TREE = dirty in-progress author tree at stated base; excluded from this docs-only target
ACCEPTED_PARENTS = SVC_WORKFLOW_PRODUCT_BOUNDARY_V9; SVC_WORKFLOW_ARCHITECTURE_V0_4_4; SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1; POSTGRES_STORAGE_CONTRACT_V0_1; SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V2
PROPOSED_WHOLE_SUCCESSORS = SVC_WORKFLOW_PRODUCT_BOUNDARY_V10; SVC_WORKFLOW_ARCHITECTURE_V0_4_5; POSTGRES_STORAGE_CONTRACT_V0_2; SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V2
PAIRED_CORE_SUCCESSOR = AGENT_CORE_WORKFLOW_PREREQUISITE_ORCHESTRATION_V2
DOCS_FIRST_REQUIRED = YES
IMPLEMENTATION_ALLOWED = NO
MIGRATION_ALLOWED = NO
DATABASE_OR_SERVICE_OPERATION_ALLOWED = NO
PRODUCTION_APPLY = NONE
OPEN_OWNER_DECISIONS = NONE (root selected FEAS-001/FEAS-002 closure)
NORMATIVE_TBD = NONE
PARTIAL_SUPERSESSION = NONE
```

## Evidence and selected decision

The read-only feasibility record at the exact Svc base reports a real PostgreSQL
16.15 failure: `CREATE_AND_BIND_GOVERNANCE_CHILD` inserts the required child
`INSTANCE_CREATED` and source `GOVERNANCE_CHILD_BOUND` Events under one actual
receipt/command ID; the global `idx_wf_event_unique_command` rejects the second
Event and the atomic request returns 503. The transaction rolls back. The
independent written blocker confirms the DDL and accepted Contract conflict;
no implementation, database or production mutation is authorized here.

FEAS-002 is likewise concrete: the persisted opted-in Context can have a
missing/unparseable coordinate or missing Context row, while the accepted
readiness DTO unconditionally requires all three UUID fields. Returning an
internal-consistency error avoids fabricated IDs but does not fulfill the
accepted typed UNKNOWN promise. Root selected one closed alternative:
`UNKNOWN / LINEAGE_INCONSISTENT` with all three coordinates omitted only when
the complete real triple is unrecoverable; otherwise all three real UUIDs stay
present. Database unavailability remains the existing transport/storage error.

## Proposed authority route and bounded plan

1. Author complete proposed whole successors Product V10, Architecture 0.4.5,
   Storage Contract 0.2 and Svc prerequisite Spec V2. The successors preserve
   their complete predecessor bodies and make only the FEAS-001/002 changes.
2. Author Core prerequisite Spec V2 as a whole successor that changes only the
   active Svc companion reference, lifecycle and acceptance-evidence mapping;
   Core behavior, typed READY row and WEC/WAE contracts remain unchanged.
3. Update navigation/preflight material to identify the proposed successor
   bundle while keeping all accepted V1/V9/0.4.4/V0.1 authorities current.
4. Stop for independent paired semantic and migration-safety review. Acceptance
   is a later explicit Root lifecycle action; this task does not accept, merge,
   implement, create a migration, or operate PostgreSQL.

The later implementation test plan must prove the completed-receipt matrix:
successful 2xx requires the exact two-Event/fact pair; completed deterministic
non-2xx requires zero business Events/facts and preserves same-key replay;
zero/one/three/wrong Events and non-2xx with any fact fail closed. It must
exercise the invariant on receipt completion and Event mutation, demonstrate
real receipt-row locking under command-identity races (not deferred COUNT-only
validation), and preserve concurrent ordinary-command at-most-one cardinality
and the unchanged `ONE_TIME_MIGRATE` pair. It must also cover migration
preservation and fail-closed rollback, command-to-Event reader compatibility,
unknown-commit/replay/whole-transaction rollback, and all-or-none
malformed-reference coordinates plus the unchanged READY-only Scheduler row.
No hidden receipt, NULL or invented command ID, omitted initial Event, new
table/wait ledger, or production database write is permitted.

```text
SPEC_GOVERNANCE_MODE = AUTHOR
AUTHORING_READY_FOR_REVIEW = YES after artifact/scope/index checks
NEXT_ACTION = REVIEW
```
