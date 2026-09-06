---
spec_id: SVC_WORKFLOW_DEFINITION_GRAPH_DIAGNOSTICS_V1
status: accepted
accepted_date: 2026-09-06
accepted_by: mayf3
accepted_reviewed_head: 78323394c6c6d82a14657bdfd6589419fdbb6dff
independent_review_result: ACCEPT
independent_review_blockers: NONE
acceptance_delta_class: lifecycle_provenance_only
semantic_delta_from_reviewed_head: none
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
scope:
  - mayf3/svc-workflow
  - definition-governance-graph-validation-diagnostics
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_0
  - SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
production_apply_authority: none
---

# Definition graph diagnostics V1

## 1. Goal

Within WORKFLOW_AUTHORING_USABILITY_PRODUCTION_V1, let an authorized author
correct an invalid graph using the existing canonical validator's rule identity.
This is a new bounded HTTP diagnostic obligation; it is not validator redesign.

## 2. Scope and non-goals

Only Definition governance draft replacement and publication failures already
represented as `DefinitionError::GraphValidationFailed` are in scope, including
safe receipt replay. No new route, validation command, rule, assignee resolver,
schema model, role, graph semantics, lifecycle transition, or database migration.
Other errors, legacy direct mappings, raw parser failures, and execution remain
unchanged. The service does not define or accept external Broker behavior.

## 3. Authority and dependencies

Repository `mayf3/svc-workflow`; base `github/main`
`e297ff1f3913133058d97bb30bcf8f63b3e137f9`, observed 2026-09-06.
Active authority blobs at this base:

| Authority/path | Blob | Relationship |
|---|---|---|
| docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V6.md | fecca7168b8a9e043664842cd92557fd09615c82 | parent; serial kernel and privacy preserved |
| docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_4_0.md | 34a833bae2d0adc25e5699e9403326d5ba0793c0 | parent; canonical graph and wire compatibility retained |
| docs/specs/SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1.md | e1504d00ca587e4d193d5b5e46653cd0c54204b2 | accepted CTR-VAI-011 canonical model dispatch retained |
| docs/contracts/DEFINITION_SERVICE_CONTRACT_V0_1.md | b23ed1340ad26480ae919e56fb25c8e08abeed7a | legacy IMPLEMENTATION_CONTRACT; §8 domain code/message retained |
| contracts/workflow-http/v1/contract.md | 9d81acb167567d9309846da504af2a5b73b86390 | retained wire contract §2.7 authorization/errors and §4.3 envelope |

The legacy service contract specifies domain diagnostics, not their governance
HTTP exposure. The HTTP contract lists Definition lifecycle/visibility errors
and an optional details envelope, not a graph diagnostic obligation. No accepted
text requires graph errors to be HTTP 500. Therefore `PREFLIGHT = NEW`,
`LANE_CLASSIFICATION = NEW_SEMANTIC_AUTHORITY_REQUIRED`; no existing Contract is
superseded. The accepted invalid RETURN references HTTP 422 Spec is an analogous
precedent for treating error-detail exposure as new semantic authority; its
transition/RETURN scope is not reused here.

Implementation is forbidden while proposed. Owner acceptance changes status to
accepted and implementation_authority to contracts in a docs-only lifecycle
transaction, with exact reviewed head and final-head independent recheck; only
presence on main/in implementation base activates these Contracts. This Spec
neither deploys nor grants production authority.

## 4. Current State

STATE-GD-001: at the pinned local source base on 2026-09-06, canonical graph errors
exist before HTTP mapping; Definition governance erases their vector and reports
500. Basis OBS-GD-001/002, CLM-GD-001. Production state is NOT verified by this
source observation. User-reported production symptoms motivate, but do not prove,
deployed source identity. Production source/artifact/readback is required later.

## 5. Observations

All observations use the repository/base above, isolated worktree
`/Users/yanfenma/workspace/worktrees/svc-authoring-usability-v1`, local source,
2026-09-06, read-only `sed`, `rg`, and `git rev-parse HEAD:<path>`.

- OBS-GD-001: `src/domain/definition/error.rs` declares GraphValidationError
  `{code,message}`. `src/application/definition/draft_graph.rs` collects canonical
  graph/schema errors and returns before `repo.replace_draft_graph`; publication
  revalidates in `src/application/definition/lifecycle/publish.rs`.
- OBS-GD-002: `src/application/definition_governance/mod.rs` From conversion maps
  GraphValidationFailed to InternalConsistency and drops errors; status is 500.
  `run_governance_write` stores only `{error:label}` for completed domain errors.
  `receipt.rs` has no graph-diagnostic decoding. By contrast direct handler mapping
  in `src/http/handlers/definitions.rs` already uses 422 and details.errors.
- OBS-GD-003: source rule inventory in Appendix A contains the literal codes
  emitted by current canonical graph validators and graph/schema assembly.
  Raw messages interpolate caller node keys and JSON-schema engine strings.
- OBS-GD-004: existing authorization maps FixedPrincipalInvalid and permission or
  existence failures to opaque definition_not_found. This is separate from safe
  structural graph rules. Unknown assignees must not become a principal census.

## 6. Claims and assumptions

CLM-GD-001 (SUPPORTED): a typed graph-error mapping plus bounded safe presentation
can close the diagnosed seam without changing validator behavior. EVD-GD-001.
CLM-GD-002 (SUPPORTED): returning raw messages is unnecessarily unsafe and may
produce nondeterministic message order; a static code catalog and sorted unique
rule list avoid trusting raw server strings. EVD-GD-002. These claims establish
local design suitability, not production conformance.

## 7. Evidence relations

- EVD-GD-001: OBS-GD-001/002 SUPPORTS CLM-GD-001 at the exact base/local source/date
  above. Strong source evidence; not an executed HTTP or production proof.
  Provenance: the four application/HTTP source paths in OBS-GD-001/002.
- EVD-GD-002: OBS-GD-003/004 SUPPORTS CLM-GD-002 at the same coordinates. Strong
  source evidence for message origins/privacy separation; boundedness and replay
  require executable tests. Provenance: graph directory and governance conversion.

## 8. Decisions

DEC-GD-001, owner mayf3, proposed: expose 422 graph_validation_failed only for typed
canonical graph failures, with a deterministic static diagnostic projection.
Reject exposing raw strings, generic 500, or a second validator: those respectively
risk disclosure, preserve the usability defect, or change authority ownership.

DEC-GD-002, owner mayf3, proposed: freeze bounded completed-error receipt replay for
these newly classified client failures, preserving existing actor/operation/key/
request-hash identity and authorization gates. No automatic correction or retry.

## 9. Contracts

### CTR-GD-001 — Typed client response

For an authorized in-scope call rejected with GraphValidationFailed, the HTTP
adapter MUST return 422 with existing error envelope and:

- `error.code = "graph_validation_failed"`;
- `error.details = {"errors":[{"code":C,"message":M},...],"truncated":B}`;
- `error.message = "graph validation failed (rule: " + T + "): " + M0`, where
  C0/M0 are the first returned error and T is C0 with every underscore replaced
  by one ASCII space. Case and remaining characters are preserved. This is a
  lossless human-readable rule-name encoding for this catalog (codes have no
  spaces); structured codes remain exact.

The top-level message MUST be at most 512 UTF-8 bytes. Normal catalog messages
M MUST be static ASCII, at most 350 bytes, with no token matching
`[A-Za-z0-9._~+/-]{24,}`; the encoded rule names also satisfy that bound. This
keeps rule identity readable through conservative opaque-token sanitizers.

### CTR-GD-002 — Safe deterministic projection

The projection MUST accept ONLY literal rule codes from Appendix A, sort codes
lexicographically by ASCII bytes, deduplicate by exact code, and return the first
32 unique codes. `truncated` MUST indicate whether unique projected codes were
omitted. Each error code MUST remain its exact case-sensitive canonical value.
Each message MUST be a static actionable correction specific to that rule;
raw validator messages, caller data, SQL, stacks, database identifiers, schema
engine dumps, credentials, and private authorization data MUST NOT be copied.

Unrecognized codes and an empty error vector MUST produce safe static fallback
`GRAPH_VALIDATION_REJECTED` / `The canonical graph was rejected; inspect the full
graph against its semantic model rules.` Unknown codes themselves MUST NOT be
exposed. A fallback does not turn failure into success. Projection/deduplication
changes display only, never the validator's error vector or rejection decision.

Representative required correction meanings:

| Canonical rules | Required correction meaning |
|---|---|
| NO_DRAFT_NODE, MULTIPLE_DRAFT_NODES | Legacy graph needs exactly one DRAFT start |
| v1_entry_task_required, v1_multiple_entry_tasks | model 3 needs exactly one entry TASK |
| TRANSITION_SOURCE_MISSING, TRANSITION_TARGET_MISSING | reference an existing source/target node key |
| PRIMARY_NOT_FROM_NODE | primary transition must originate at its owning node |
| PRIMARY_NOT_ADVANCE, v1_primary_advance_invalid | select an existing ADVANCE transition as primary |
| MISSING_PRIMARY, v1_primary_advance_required | provide primary ADVANCE for each required work node |
| SELF_LOOP | remove transition whose source and target are the same |
| NODE_NOT_REACHABLE, v1_unreachable_node | connect every node to the model's entry path |
| PRIMARY_CYCLE, v1_primary_path_cycle | remove cycle in primary path |
| ASSIGNEE_REQUIRED, v1_task_owner_required | supply the required canonical owner reference |
| TERMINAL_HAS_ASSIGNEE, v1_terminal_owner_forbidden | remove terminal owner reference |
| v1_owner_ref_forbidden | use only model 3's accepted owner reference types |
| INVALID_CONTEXT_SCHEMA, INVALID_SUBMISSION_SCHEMA | fix the named schema using supported local references |

Other catalog rules MUST receive equally specific static corrections; exact prose
is editorial, while the represented rule identity and correction meaning are not.

### CTR-GD-003 — Replay and no partial invalid state

The canonical graph, version identity, revision, publication status, and digest
MUST remain unchanged on graph rejection. Only the existing command receipt may
record the completed 422 diagnostic. Same identity/key/hash replay MUST return
the identical bounded status, code, message, and details without revalidating or
reexecuting the mutation. Store the safe diagnostic projection, not raw messages;
validate its closed shape and bounds when decoding. Invalid stored diagnostic
shape MUST fail closed through existing opaque consistency handling, never be
returned verbatim. Historical receipt formats remain decodable unchanged.
Same key with changed input MUST remain an idempotency conflict. Existing pending
or ambiguous historical attempts MUST NOT be converted/replayed automatically.

### CTR-GD-004 — Authority and privacy preservation

Canonical validator implementation, rule predicates, semantic dispatch, identity,
Domain permission, token/scope gates, and graph lifecycle MUST remain unchanged.
Existing FixedPrincipalInvalid/permission/existence opaque response behavior MUST
remain unchanged. Raw storage, schema-only, digest, and parser errors MUST NOT be
reclassified by string matching or passed through this graph catalog. No graph
validation call may be skipped merely because input was generated by an adapter.

### CTR-GD-005 — Contract artifacts and release evidence

Implementation MUST update affected HTTP contract/error catalog/OpenAPI artifacts
consistently, preserve unrelated routes, and pass repository contract checks.
The service release evidence MUST bind source/artifact, deployed preimage, health,
HTTP diagnostics, unchanged rejected graph readback, and normal authorized caller
behavior. Source tests alone MUST NOT be labeled production PASS.

## 10. Acceptance

Each item records exact Spec/code SHA, executed command, environment, timestamp,
result, and provenance. Definitions below are requirements, not executed evidence.

- ACC-GD-001 → CTR-GD-001/002: unit and real HTTP tests for invalid start,
  unknown transition target, invalid primary relationship, self-loop/connectivity,
  and missing/forbidden assignee representation. Use both relevant Legacy and model
  3 canonical rules. Require 422, expected stable code, corrective text and visible
  encoded top-level rule name. Include a valid graph positive control.
- ACC-GD-002 → CTR-GD-002: oversized/malicious raw messages, unknown codes, empty
  vectors, duplicate/permuted errors, and >32 unique rules. Require exact bounds,
  deterministic unique sorting, correct truncated flag, no raw leakage. A consumer
  sanitizer equivalent to the regex in CTR-GD-001 must preserve rule and correction.
- ACC-GD-003 → CTR-GD-003: real DB/HTTP failure plus graph/version readback; exact
  same-key replay must match all diagnostic fields, changed input conflicts; corrupt
  receipt shape fails opaque, and a historical receipt replays unchanged. Tests use
  a disposable database, not production. Invalid graph is never stored/published.
- ACC-GD-004 → CTR-GD-004: existing focused canonical validator suites remain green
  with zero validator diff; forbidden Domain, unknown assignee, invalid lifecycle,
  storage, and authorization regressions retain prior status and privacy behavior.
- ACC-GD-005 → CTR-GD-005: affected HTTP bundle checks plus production artifact,
  health, authorized diagnostic proof and unchanged graph readback. Deployment is
  under the owning Goal's separate authority and serialized production boundary.

## 11. Alternatives and disposition

Raw details rejected for disclosure risk. A new validator or graph model rejected
as outside Goal. Generic 500 rejected because it hides the existing rule. Reusing
the RETURN Spec rejected because its stable scope is unrelated. New routes or
standalone validate API deferred because replacement/publication already validate.

## 12. Migration, compatibility, and rollback

No database schema/data migration. Only previously opaque typed graph failures
become 422; all success shapes and unrelated failures remain unchanged. New safe
receipt payload is additive for this code only; historical receipt entries remain
unchanged. Release rollback must account for new completed graph diagnostic
receipts: an older binary lacks the decoder, so raw downgrade is not a proven
compatible rollback. Prepare a rollback artifact retaining the safe receipt decoder
or quiesce authoring and document bounded pending diagnostics; never erase receipt
history or replay unknown attempts. Production activation/rollback evidence is
separate from acceptance of this docs-only Spec.

## 13. Open questions

OPEN_OWNER_DECISIONS = NONE beyond exact-head acceptance.
NORMATIVE_TBD = NONE. PARTIAL_SUPERSESSION = NONE.
IMPLEMENTATION_STATE = NOT_STARTED. CONFORMANCE = UNKNOWN.

## Appendix A — Closed source rule catalog

Source inventory described in OBS-GD-003, at the exact base. This freezes public
rule identities for this projection only; it does not alter validator rules.

- `ASSIGNEE_REQUIRED`
- `CONTEXT_SCHEMA_REQUIRED_FOR_INSTANCE_INPUT`
- `CONTEXT_SCHEMA_REQUIRED_MISSING_ASSIGNEE_KEY`
- `DRAFT_NOT_WORKFLOW_CREATOR`
- `DUPLICATE_ORDER_INDEX`
- `FIXED_PRINCIPAL_MISSING_ID`
- `INSTANCE_INPUT_PRINCIPAL_INVALID_KEY`
- `INSTANCE_INPUT_PRINCIPAL_MISSING_KEY`
- `INVALID_CONTEXT_SCHEMA`
- `INVALID_SUBMISSION_SCHEMA`
- `MIN_NODES`
- `MISSING_PRIMARY`
- `MULTIPLE_DRAFT_NODES`
- `NODE_NOT_REACHABLE`
- `NO_DRAFT_NODE`
- `NO_TERMINAL_NODE`
- `PRIMARY_CYCLE`
- `PRIMARY_NOT_ADVANCE`
- `PRIMARY_NOT_ADVANCING`
- `PRIMARY_NOT_FROM_NODE`
- `PRIMARY_TRANSITION_MISSING`
- `PRIMARY_TRUNK_NO_TERMINAL`
- `RETURN_IS_PRIMARY`
- `RETURN_NOT_BACKWARD`
- `RETURN_TO_TERMINAL`
- `SELF_LOOP`
- `TASK_NODE_IN_LEGACY_GRAPH`
- `TERMINAL_HAS_ASSIGNEE`
- `TERMINAL_HAS_OUTGOING`
- `TERMINAL_HAS_PRIMARY`
- `TERMINATE_IS_PRIMARY`
- `TERMINATE_TO_NON_TERMINAL`
- `TRANSITION_SOURCE_MISSING`
- `TRANSITION_TARGET_MISSING`
- `UNEXPECTED_ASSIGNEE_INPUT_KEY`
- `UNEXPECTED_FIXED_PRINCIPAL`
- `v1_entry_task_required`
- `v1_fixed_principal_missing`
- `v1_multiple_advance_forbidden`
- `v1_multiple_entry_tasks`
- `v1_node_kind_forbidden`
- `v1_owner_ref_forbidden`
- `v1_primary_advance_invalid`
- `v1_primary_advance_required`
- `v1_primary_path_cycle`
- `v1_return_target_not_earlier`
- `v1_return_target_not_task`
- `v1_task_owner_required`
- `v1_terminal_outgoing_forbidden`
- `v1_terminal_owner_forbidden`
- `v1_terminal_primary_advance_forbidden`
- `v1_terminate_primary_forbidden`
- `v1_terminate_target_not_terminal`
- `v1_unreachable_node`
- `v2_advance_cycle`
- `v2_assignee_domain_owner_forbidden`
- `v2_context_principal_key_must_be_single_segment`
- `v2_context_principal_key_required`
- `v2_entry_task_required`
- `v2_fixed_principal_missing`
- `v2_multiple_entry_tasks`
- `v2_node_draft_forbidden`
- `v2_node_task_forbidden`
- `v2_primary_advance_forbidden`
- `v2_return_target_not_strict_ancestor`
- `v2_task_assignee_required`
- `v2_terminal_assignee_forbidden`
- `v2_terminal_outgoing_forbidden`
- `v2_terminate_effect_forbidden`
- `v2_unreachable_task`
