# DEVELOPMENT_PREFLIGHT — bounded source implementation

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
TARGET_REPOSITORY = mayf3/svc-workflow
REVIEW_TARGET_HEAD = afb73d83c4804c894582a81af6907f48c13fa0bb
BASE_HEAD = afb73d83c4804c894582a81af6907f48c13fa0bb
CURRENT_BASE_HEAD = afb73d83c4804c894582a81af6907f48c13fa0bb
ROUTE_STAGE = IMPLEMENTATION
AUTHORITY_ACCEPTED_IN_BASE = YES
GOAL_OR_TARGET = CTR-SWPO-001..006 source implementation and isolated executable evidence
CURRENT_GAP = accepted fixed prerequisites and atomic create-bind have no implementation
OBSERVATIONS = clean isolated codex/prerequisite-orchestration-impl worktree; PR97 and paired PR577 merged; PR573 merged
WORKING_GUESS = NOT_APPLICABLE
AUTHORITY_ACTION = REUSE
PRIMARY_AUTHORITY = SVC_WORKFLOW_PREREQUISITE_ORCHESTRATION_V1@r1
RELATED_AUTHORITIES = Product V9 DEC-V9-001/002; Architecture V0_4_4 CTR-ARCH-041/042/043; WEP V2; GCR V1; VAI V1; paired Core V1 at b8e58012de75f98825f2d1fd734e9eb0dfe8cdfa
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
BASE_IMPACT = NONE
IMPLEMENTATION_ALLOWED = YES
MERGE_READY = NO
OPERATION_ALLOWED = NO
EVIDENCE_NEEDED = focused Rust schema tests; isolated PostgreSQL atomicity/idempotency/races/readiness/pagination; authenticated HTTP visibility; diff/format/governance checks
DONE_WHEN = clean committed exact source candidate plus reproducible evidence/report for root independent review
EXPANSION_TRIGGER = missing contract; inability to compose atomic transaction, narrow visibility, or pre-LIMIT readiness; need migration or production operation
NEXT_REAL_ACTION = execute existing WORKFLOW_PREREQUISITE_ORCHESTRATION_PLAN.md phases 1..4
NEXT_ACTION = CONTINUE
```

Mandate: `/Users/yanfenma/workspace/artifacts/workflow-unattended-mechanisms-20261027/ART/child-wait-contract-recon-20261029/paired-implementation-mandate.md` and `svc-implementation-brief.md`, issued by root under mayf3 delegation.

Controlled source/test runbook: sole writer prerequisite_svc_impl; exact owned worktree `/Users/yanfenma/workspace/worktrees/codex-autocreate-wait-svc-20261029`; source, tests and review artifacts only. Validate exact closed schema, then transaction-scoped create/bind composition, then projections/routes. All database execution uses only a fresh explicitly named localhost test database; preserve prior databases. No secrets in evidence. Abort on any expansion trigger above. Record executed command/result/log and final source hash; root performs independent review and any integration. No push, merge, native launch, production deployment, restart, real Grant/credential/permission mutation, migration or old-run mutation.

Convergence: RETAIN raw due API and existing bind (distinct supported interfaces); share create/admission/readiness logic instead of introducing another workflow truth. No bridge or wait writer.


## Implementation checkpoint — not ready

The real atomic expected-success test fails on existing global event-command uniqueness (`idx_wf_event_unique_command`): one receipt owns both required birth and bind events. A separate rollback test proves zero child/binding/receipt/outbox delta and unchanged source version. Atomic implementation is frozen pending root's independently reviewed, accepted and merged docs-first closure. No null command ID, second receipt, dropped birth event, index change or migration was introduced.

Malformed opted-in stored Context whose exact reference coordinates cannot be recovered currently returns an internal-consistency transport error, with no READY item; no placeholder/null/fake coordinate DTO is emitted. The final typed unidentifiable-UNKNOWN choice is frozen pending the same authority review. Valid-reference UNKNOWN remains exact and tested.

The source candidate is WIP, not merge-ready, not accepted conformance, and not a runtime result. Existing fresh test databases are preserved. Root owns subsequent authority acceptance, independent source review and integration. Proposed V2 semantic changes have not been implemented here.
