# DEVELOPMENT_PREFLIGHT — V2 bounded implementation

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
TARGET_REPOSITORY = mayf3/svc-workflow
REVIEW_TARGET_HEAD = ac4431f944dbce967881dc3c50fffbd7fadfa4a8
BASE_HEAD = d1b744865d796565730d7ca902e218b4a6e7b99a
CURRENT_BASE_HEAD = d1b744865d796565730d7ca902e218b4a6e7b99a
ROUTE_STAGE = IMPLEMENTATION
AUTHORITY_ACCEPTED_IN_BASE = YES
GOAL_OR_TARGET = close CTR-SWPO-007/008 and original atomic acceptance at exact paired accepted V2 bases
CURRENT_GAP = WIP atomic command rejected by old global event index; unidentified corrupt Context has transport error
AUTHORITY_ACTION = REUSE
PRIMARY_AUTHORITY = SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V2@r1
RELATED_AUTHORITIES = Product V10; Architecture V0_4_5; Storage V0_2; WEP V2; GCR V1; VAI V1; Core WPO V2@964a250251d225a43abf93aa899463c7277530b6
IMPLEMENTATION_AUTHORITY = contracts
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = NO
PLAN_LEVEL = EXEC_PLAN
ASSURANCE_LEVEL = CONTROLLED
EXECUTION_MANDATE = VALID
MUTATION_AUTHORIZATION = VALID
ISOLATED_WRITE_SURFACE = YES
CONTROLLED_RUNBOOK_REQUIRED = YES
SPEC_GAP_DEPENDENCY = NONE
EVIDENCE_REVIEWABILITY = PASS
LIVE_AUTHORITY_GAP = NONE
OWNER_DECISION_REQUIRED = NO
EMERGENCY_STATE = NONE
EMERGENCY_ACTION = NONE
INCIDENT_REFERENCE = NOT_APPLICABLE
BASE_IMPACT = BOUNDED
IMPLEMENTATION_ALLOWED = YES
MERGE_READY = NO
OPERATION_ALLOWED = NO
EVIDENCE_NEEDED = one forward migration; both trigger surfaces; exact success/rejection facts; native ordinary uniqueness and real receipt-lock races across RC/RR/SERIALIZABLE; history hashes and rollback refusal; DTO/auth/outage; real HTTP replay/readback
DONE_WHEN = clean source candidate and evidence report for independent review
EXPANSION_TRIGGER = cardinality cannot be guaranteed; historical business values change; missing authority; need production/old database mutation
NEXT_ACTION = CONTINUE
```

Mandate: root's paired-v2-implementation-mandate.md in child-wait-contract-recon-20261029 ART. Actual origin/main verified by fetch at d1b744865d796565730d7ca902e218b4a6e7b99a and merged into WIP ca8593e as ac4431f944dbce967881dc3c50fffbd7fadfa4a8. Sole writer and owned isolated worktree unchanged.

Runbook/plan: (1) preserve V1 failed evidence; write new negative/atomic/DTO tests and observe failures; (2) one forward integrity migration, exact same-command pair, real receipt lock, per-Instance uniqueness and derived native integrity slot (all ordinary events slot0); no historical UPDATE; (3) readiness-only safely authorized corrupt Context representation, no ordinary reader relaxation; (4) actual isolated PostgreSQL/HTTP/upgrade/race checks and directly affected old commands; (5) exact candidate/log hashes and root independent review. New test DB names only; old shared/run DBs read-only. No secrets in reports; no real credentials/grants, production DDL/restart, native launch, push or merge. Abort on expansion triggers. ONE_TIME_MIGRATE retains its deferred authority scope; no writer/event type or native coverage is invented.
