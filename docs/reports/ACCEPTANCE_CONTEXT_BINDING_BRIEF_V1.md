# Acceptance Context binding — Change Brief

```text
TASK_ID = acceptance-context-binding-20261027
TARGET_REPOSITORY = mayf3/svc-workflow
BASE_HEAD = bfc28c15320acab8cf7d7a36128cd5c9db9a1b1a
REVIEW_TARGET_HEAD = resolve the docs-only commit containing this Brief
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO
GOAL_OR_TARGET = one reusable Definition; each Instance's acceptance IDs and true Context UUID bound to successful review ADVANCE
CURRENT_GAP = schema shape and server lineage exist; cross-payload Context reference comparison does not
AUTHORITY_ACTION = NEW
PRIMARY_AUTHORITY = SVC_WORKFLOW_ARCHITECTURE_V0_4_3@BASE_HEAD
RELATED_AUTHORITIES = SVC_WORKFLOW_PRODUCT_BOUNDARY_V8; SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1; retained schema/transition contracts
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = NO
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = DURABLE
```

## Proposed choice for Owner review

Accept one narrow typed reference feature: a closed top-level submission-schema
annotation fixes `/acceptanceCriteria` and `/acceptanceReport`; the entire schema
is already in the published Definition digest. Each new model3 Instance provides
its own immutable catalog. The service compares the actual Context UUID and
exact acceptance-ID set inside the existing transition transaction, then requires
declared PASS only on the configured primary ADVANCE. RETURN/TERMINATE preserve
their own contracts. Evidence sufficiency remains independently reviewed.

There is no schema migration, new Context-edit API, new role, or separate ledger.
Existing Instances are not rebound. Choosing model3 preserves the current
no-new-Legacy boundary; it does not upgrade the active governance Instance or
silently publish an 8121-specific catalog into a shared Definition.

## Scope and execution controls

The attributable root assignment on 2026-10-27 authorizes this single author to
write the proposed Spec, this Brief and preflight, check documents, and commit
locally at `codex/workflow-acceptance-binding-20261027` in
`/Users/yanfenma/workspace/worktrees/codex-workflow-acceptance-binding-20261027`.
It does not accept the new Contract, accept PR #86, or authorize implementation.
No product/test/DB/runtime change, push, merge or deployment is part of this task.

`OBSERVATIONS`: main schema-only validation, immutable Definition/Context binding,
existing CAS/idempotency tests; static audit only. `WORKING_GUESS`: the closed
reference rule fits parent shape/digest integrity rather than business truth;
independent review must challenge any parent conflict. Load-bearing evidence is
recorded in the Spec; proposed tests remain `NOT_RUN`.

```text
MUTATION_AUTHORIZATION = VALID (docs-only)
SPEC_GAP_DEPENDENCY = LOAD_BEARING
OWNER_DECISION_REQUIRED = YES, exact proposed contract after independent review
CONTROLLED_RUNBOOK = NOT_APPLICABLE to authoring; required later if an operation mandate calls for it
MODEL_CONVERGENCE_APPLICABLE = YES
RETAIN = unannotated versions and immutable historical Instances under existing authority
RETIRE = old binaries from serving opted-in versions; no historical deletion
MIGRATE = future authorized model3 traffic may select a new opted-in version; no automatic existing-instance migration
TEMPORARY_BRIDGE = NONE
REAL_CONSUMERS = actual adopted Definition/Instance coordinates must be named before runtime activation
BRIDGE_EXIT_CONDITION = NOT_APPLICABLE
```

## Smallest subsequent implementation route

Only after exact acceptance in the implementation base: add pure parsing/catalog
and report-reference checks; call them at Definition validation/publication,
Create, and the existing locked Transition boundary; defend combined commands;
reuse existing error/Receipt/transaction behavior. Bind release support and
downgrade checks to executed evidence. No implementation is performed by this
Brief. It is not a production runbook.

Then execute the six Acceptance rows in isolation: especially two Instances with
the same IDs but different Context UUIDs, wrong references/coverage, malformed
catalogs, CAS/replay/transaction failures, RETURN and unannotated regressions, and
old-binary containment. A bad cached UUID is not evidence of a nonexistent model3
revise capability. Unknown mutation outcomes require reconciliation, not replay.

```text
DONE_WHEN = three docs committed locally, exact head/hashes delivered for independent review
EXPANSION_TRIGGER = parent meaning change, Context mutation, extra policy/ledger/permission, or relevant base drift
NEXT_REAL_ACTION = independent exact-head review, then one attributable Owner decision
NEXT_ACTION = REVIEW
IMPLEMENTATION_READY = NO
OPERATION_READY = NO
```
