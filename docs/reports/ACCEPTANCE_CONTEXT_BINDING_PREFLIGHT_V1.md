# Acceptance Context binding — DEVELOPMENT_PREFLIGHT

```text
RECORDED_AT = 2026-10-27
SPEC_GOVERNANCE_MODE = AUTHOR
TARGET_REPOSITORY = mayf3/svc-workflow
BASE_HEAD = bfc28c15320acab8cf7d7a36128cd5c9db9a1b1a
ISOLATED_WRITE_SURFACE = /Users/yanfenma/workspace/worktrees/codex-workflow-acceptance-binding-20261027
ISOLATED_REF = codex/workflow-acceptance-binding-20261027
SINGLE_DOCS_AUTHOR = /root/closure_business_audit
GOAL_OR_TARGET = reusable per-instance acceptance-ID and Context-version binding before a designated successful ADVANCE
CURRENT_GAP = submission JSON Schema validates one payload; server Context lineage does not validate references inside that payload
AUTHORITY_ACTION = NEW
PRIMARY_AUTHORITY = SVC_WORKFLOW_ARCHITECTURE_V0_4_3@BASE_HEAD
RELATED_AUTHORITIES = SVC_WORKFLOW_PRODUCT_BOUNDARY_V8; SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1; retained definition/transition contracts @BASE_HEAD
TARGET_PROPOSAL = SVC_WORKFLOW_ACCEPTANCE_CONTEXT_BINDING_V1
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = DURABLE
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = NO
DOCS_FIRST_REQUIRED = YES (repository-local implementation prerequisite)
IMPLEMENTATION_READY = NO
OPERATION_READY = NO
SPEC_GAP_DEPENDENCY = LOAD_BEARING
MUTATION_AUTHORIZATION = VALID for these documents and docs-only commit
```

## Mandate and boundaries

The root coordinator, conveying the user's requested acceptance/version-binding
slice, assigned this isolated author on 2026-10-27 to write a concise proposed
Spec and Brief, persist preflight, and commit documentation only. The user has
requested implementation, but no exact binding Contract has yet been accepted.
The task does not accept this Spec or PR #86's unrelated timestamp proposal.

Allowed: these three new documents, focused static reads, documentation checks,
and a local commit. Forbidden: product source or behavior-defining test changes,
test execution, Definition/Instance or database mutation, runtime activation,
new rights, external messages, push, merge, or self-acceptance.

## Authority classification

At `BASE_HEAD`, Architecture §5.5/CTR-ARCH-008 permits bounded schema/digest
validation and same-Instance Submission integrity, not business truth. The
proposal adds an opt-in, closed typed reference rule: compare report references
with immutable Context facts. It does not introduce an approval engine, execute
scripts, decide evidence sufficiency, mutate Context, or replace existing
unannotated transition semantics. Accordingly the bounded decision is NEW,
not REUSE and not a partial replacement of accepted authority.

The annotation is proposed new contract data, not an already effective JSON
Schema keyword. Standard JSON Schema and old binaries ignore it. Publication,
runtime support, and downgrade containment are therefore load-bearing Contracts.
Any decision to introduce business truth adjudication, ordinary model3 Context
edits, or a different accepted failure/lifecycle meaning
requires RE_PREFLIGHT and the appropriate whole-authority successor; calling it
NEW cannot override the parent.
Model3-only is this V1's chosen scope; the production barrier is not established
by this audit. Expanding this proposal to Legacy requires a separate scope and
authority check, without narrowing existing pre-barrier Legacy authority.

Legacy contracts are `docs/contracts/DEFINITION_SERVICE_CONTRACT_V0_1.md`
and `docs/contracts/WORKFLOW_TRANSITION_CONTRACT_V0_1.md`, both declared
`IMPLEMENTATION_CONTRACT`, retained only in their declared scopes through
Architecture §3.2. Their schema, digest, serialization, Receipt, and immutable
Submission obligations are reused; they do not authorize dynamic binding.

## Evidence and stop

Static source at the exact base proves the isolated validator inputs and
server-owned lineage. Existing schema/CAS tests are test definitions, not new
executed results. The prior independent a1 HTTP 201/internal Receipt 200 failure
record supports preserving distinct transport/Receipt facts and no replay of an
unknown mutation; it does not prove this feature works.

```text
EVIDENCE_NEEDED = independent exact-head authority review; then Owner acceptance; after that, new implementation and executed acceptance evidence
DONE_WHEN = concise proposed Spec + Brief + this preflight are committed docs-only and exact head/hashes returned to root
EXPANSION_TRIGGER = parent conflict, change of scope, new data/permission/runtime effect, or changed relevant base
NEXT_REAL_ACTION = independent review of the proposed per-instance binding contract
NEXT_ACTION = REVIEW
```
