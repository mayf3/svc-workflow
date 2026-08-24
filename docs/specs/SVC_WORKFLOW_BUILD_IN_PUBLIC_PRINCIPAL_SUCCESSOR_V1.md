---
spec_id: SVC_WORKFLOW_BUILD_IN_PUBLIC_PRINCIPAL_SUCCESSOR_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: none
production_apply_authority: none
scope:
  - mayf3/svc-workflow
  - exact-build-in-public-principal-successor-plan
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V3
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_BUILD_IN_PUBLIC_PRINCIPAL_SUCCESSOR_V1

## 1. Goal

Establish a reviewable, exact-pair proposal for restoring the current Domain authority and only the still-active current workflow responsibility of the historical Build in Public identity to its canonical successor:

```text
OLD_AGENT_ID = build-in-public-agent
OLD_PRINCIPAL = bb9d8f48-7962-4321-8fb1-554bb428c159
NEW_AGENT_ID = agt_build-in-public-agent
NEW_PRINCIPAL = d5b3aeb2-e754-49a9-9914-b963521c0985
MIGRATION_KIND = ONE_TIME_SUCCESSOR
```

`NEW` is the only proposed canonical Build in Public Workflow identity after a future separately authorized apply. This proposal does not itself establish identity linkage outside `svc-workflow`, implement an operator, query production, or authorize production apply.

```text
STATUS = proposed
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
IMPLEMENTATION_PERFORMED = NO
PRODUCTION_CHANGE = NONE
```

## 2. Scope and non-goals

### 2.1 In scope after all authority gates are satisfied

A future accepted implementation-authorizing successor may permit exactly one offline operator, fixed at compile time to the exact pair above, with these modes:

```text
--plan
--apply
--verify
DEFAULT_MODE = --plan
```

The intended operation is limited to:

1. read-only planning from the live svc-workflow PostgreSQL database;
2. transfer of the exact reviewed enabled Domain bindings from `OLD` to `NEW`;
3. append-only transfer of only reviewed current, active, non-terminal, non-cancelled responsibility still assigned to `OLD` at apply time;
4. exact rerun `NOOP` and fail-closed conflict handling.

### 2.2 Explicit exclusions

The proposal MUST NOT authorize modification, migration, selection as successor, or identity inference for:

```text
EXCLUDED_AGENT_ID = blog-agent
EXCLUDED_PRINCIPAL = 81c7fc7e-c696-4b47-bfd6-f12a9ecb68a6
```

It also excludes:

- completed, terminal, cancelled, or archived tasks;
- historical Visits, historical assignments, and ever-assigned records as mutation targets;
- the legacy 5,583 archive;
- every other Principal pair;
- every Domain or workflow responsibility absent from the reviewed canonical plan;
- credentials, tokens, secrets, Auth identity/client mappings, or `DATABASE_URL` contents;
- changing `workflow_instances.created_by_principal_id`;
- deleting or disabling the `OLD` Principal;
- representing the operation as `ADMIN_EMERGENCY_OVERRIDE` or emitting an emergency-override event;
- selecting, switching, rewriting, or fallback-discovering another database;
- any schema change, SQL migration, or migration-file change for this one-time operator;
- a general reassign API, HTTP/SDK capability, handoff, delegation, or reusable arbitrary-pair migration;
- manual SQL or `psql` mutation;
- use or impersonation of an `OLD` credential;
- treating a historical snapshot as live evidence;
- production apply under this proposed Spec.

## 3. Authority and dependencies

### 3.1 Current authority coordinates

```text
REPOSITORY = mayf3/svc-workflow
AUTHORING_BASE = 327b74f138151a7f4d9d88e3881e54d203f1e8f6
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V3
PARENT_STATUS = accepted
PATTERN_REFERENCE = SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1
PATTERN_REFERENCE_REVISION = 1055b711f8f07a173126b6488b554466a707e899
PATTERN_REFERENCE_STATUS = accepted
RECOVERY_PATTERN_REFERENCE_REVISION = a7f8d26b7a8f57da773bd7b05879ee485841fa58
RECOVERY_PATTERN_REFERENCE_STATUS = proposed_unmerged
```

`SVC_WORKFLOW_PRODUCT_BOUNDARY_V3` §§17, 20 freezes the retained one-time migration to a different exact Principal pair. `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` likewise authorizes only that CTO pair. Its offline, append-only, atomic, audit, fail-closed, and NOOP shapes are accepted precedent, but its exact-pair authority MUST NOT be expanded by analogy. The Admin Recovery replay closure at `a7f8d26b7a8f57da773bd7b05879ee485841fa58` is explicitly identified by V3 as proposed and unmerged; it is evidence for proposal design, not accepted authority.

Therefore this proposed child is non-implementable at this revision:

```text
AUTHORITY_SUFFICIENT_FOR_REQUESTED_PAIR = NO
AUTHORITY_CONFLICT = V3_EXACT_PAIR_DOES_NOT_MATCH_BUILD_IN_PUBLIC_PAIR
REQUIRED_BEFORE_ACCEPTANCE = LAWFUL_PARENT_AUTHORITY_RECONCILIATION
CHILD_MAY_EXPAND_PARENT = NO
```

A lawful Product Direction successor or other valid higher-authority reconciliation must explicitly authorize this separate exact pair before this child can become implementation-authorizing. Merely accepting this file without that reconciliation MUST NOT activate implementation or production execution.

### 3.2 Existing semantics to preserve

Any later authority and implementation must preserve:

- `SVC_WORKFLOW_ARCHITECTURE_V0_3_1` single-current-node/single-current-assignee shape and prohibition on ordinary reassign/handoff/delegate;
- immutable historical Visit/Event/Submission facts;
- formal Domain Owner replacement semantics and single-enabled-owner invariant;
- accepted append-only successor Visit/Event/Receipt/Audit precedent;
- the proposed Admin Recovery replay pattern only after recovery semantics receive their own lawful accepted authority; this proposal MUST NOT represent PR #7 as accepted;
- one PostgreSQL atomic outcome or zero committed writes;
- separate implementation, independent review, and production execution gates.

## 4. Current State

### STATE-BIP-001 — Authority state

- Subject: `mayf3/svc-workflow` authority tree.
- As of commit: `327b74f138151a7f4d9d88e3881e54d203f1e8f6`.
- Environment: source repository, not production runtime.
- Observed at: `2026-08-23T23:53:56Z`.
- State: accepted V3 and the accepted CTO migration child freeze another exact pair; no accepted authority found for this Build in Public pair.
- Basis: `OBS-BIP-001`, `OBS-BIP-002`, `EVD-BIP-001`.

### STATE-BIP-002 — Requested production facts remain unverified

- Subject: production svc-workflow persistence for `OLD` and `NEW`.
- Environment: production database.
- Observed at: not queried in this authority-gated round.
- State: nine enabled `OLD` Domain bindings are Owner-supplied expected inputs; all six task/history counts and active responsibility tuples remain unknown until an authorized read-only `--plan` run.
- Basis: `OBS-BIP-004`, `CLM-BIP-002`.

## 5. Observations

### OBS-BIP-001 — Accepted Product Direction freezes another pair

- Subject: `docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V3.md` §§17, 20.
- Source revision: `327b74f138151a7f4d9d88e3881e54d203f1e8f6`.
- Environment: repository source.
- Observed at: `2026-08-23T23:53:56Z`.
- Method: inspect accepted authority text.
- Result: the retained exception is exact-pair-only and names neither requested Build in Public Principal.
- Provenance: repository path above.

### OBS-BIP-002 — Accepted implementation Spec freezes the CTO pattern

- Subject: `docs/specs/SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1.md`.
- Source revision: `1055b711f8f07a173126b6488b554466a707e899`.
- Environment: repository source.
- Observed at: `2026-08-23T23:53:56Z`.
- Method: inspect exact pair, plan, apply, audit, NOOP, and production gate Contracts.
- Result: the accepted implementation authority is intentionally incapable of accepting another Principal pair.
- Provenance: repository Spec and its accepted revision history.

### OBS-BIP-003 — Recovery closure is proposed, not accepted

- Subject: successor-event Admin Recovery replay addition to `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1`.
- Source revision: `a7f8d26b7a8f57da773bd7b05879ee485841fa58`.
- Environment: repository Draft PR history referenced by accepted V3.
- Observed at: `2026-08-23T23:53:56Z`.
- Method: compare the replay addition with V3 §3 and §17 lifecycle statements.
- Result: replay fail-closed semantics are a useful proposed pattern, but the revision is unmerged and non-authoritative.
- Provenance: Draft PR revision above and `docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V3.md`.

### OBS-BIP-004 — Owner supplied a distinct exact identity ruling

- Subject: Build in Public Workflow identity.
- Source: direct Owner task ruling for this docs-only round.
- Environment: requested future production migration.
- Observed at: `2026-08-23T23:53:56Z`.
- Method: record the exact OLD/NEW identities, nine expected Domain keys/roles, transfer-all decision, exclusions, and plan-only limit.
- Result: the requested pair differs from the accepted CTO pair; `blog-agent` is explicitly unrelated and excluded.
- Provenance: this Spec persists the supplied ruling without treating it as already accepted repository authority or live database evidence.

## 6. Claims and assumptions

### CLM-BIP-001 — Existing authority is insufficient

- Support state: SUPPORTED.
- Supported by evidence: `EVD-BIP-001`.
- Contradicted by evidence: none known.
- Uncertainty: none for exact UUID comparison.

### CLM-BIP-002 — Owner-supplied counts are planning expectations, not live facts

- Support state: SUPPORTED.
- Supported by evidence: `EVD-BIP-002`.
- Contradicted by evidence: none known.
- Uncertainty: exact production tuples and counts require a later authorized consistent read-only transaction.

### CLM-BIP-003 — The CTO pattern can be reused only as a bounded semantic pattern

- Support state: INFERRED.
- Supported by evidence: `EVD-BIP-003`.
- Contradicted by evidence: none known.
- Uncertainty: parent authority reconciliation and independent review may require narrower Contracts before acceptance.

## 7. Evidence relations

### EVD-BIP-001 — Exact-pair mismatch supports the authority blocker

- Source observations: `OBS-BIP-001`, `OBS-BIP-002`, `OBS-BIP-004`.
- Target: `CLM-BIP-001`, `STATE-BIP-001`.
- Relation: SUPPORTS.
- Coordinates: repository commit `327b74f138151a7f4d9d88e3881e54d203f1e8f6`, observed `2026-08-23T23:53:56Z`.
- Strength: sufficient for the docs-only authority gate.
- Limitations: does not establish production data state.

### EVD-BIP-002 — No live query limits production State

- Source observations: `OBS-BIP-004`.
- Target: `CLM-BIP-002`, `STATE-BIP-002`.
- Relation: SUPPORTS.
- Coordinates: this plan-only authority round.
- Strength: sufficient to prohibit claiming live completeness.
- Limitations: expected Domain list remains to be verified.

### EVD-BIP-003 — Accepted predecessor and proposed replay closure supply constrained patterns

- Source observations: `OBS-BIP-001`, `OBS-BIP-002`, `OBS-BIP-003`.
- Target: `CLM-BIP-003`.
- Relation: SUPPORTS.
- Coordinates: accepted source authorities at the authoring base plus proposed replay revision `a7f8d26b7a8f57da773bd7b05879ee485841fa58`.
- Strength: sufficient for proposal authoring, insufficient for implementation.
- Limitations: pattern reuse does not transfer exact-pair authority; proposed replay text is not accepted authority.

## 8. Decisions

### DEC-BIP-001 — Exact one-time successor pair

- Decision owner: `mayf3`.
- Selected direction: propose one offline successor migration fixed to `bb9d8f48-7962-4321-8fb1-554bb428c159` → `d5b3aeb2-e754-49a9-9914-b963521c0985`.
- Rejected: arbitrary Principal input, generic migration, alias/name-based selection, and reuse of the accepted CTO pair authority.
- Owner input remaining: lawful parent authority reconciliation and later acceptance.

### DEC-BIP-002 — Transfer all and only reviewed enabled OLD Domain bindings

- Decision owner: `mayf3`.
- Selected direction: expected scope is exactly one `DOMAIN_OWNER` and eight `DOMAIN_MEMBER` tuples listed in the `CTR-BIP-DOMAIN-001` table, provided a live plan re-reads an exact matching tuple set.
- Rejected: count-only selection, discovery of extra Domains as implicit scope, role normalization, or long-lived dual authority.
- Owner input remaining: none on desired nine-key scope; live tuple evidence remains required.

### DEC-BIP-003 — Transfer only current effective responsibility

- Decision owner: `mayf3`.
- Selected direction: only current Visit responsibility assigned to `OLD` that remains active, non-terminal, not cancelled, not archived, and is frozen in the reviewed plan may move to `NEW`.
- Rejected: completed/terminal/archive migration, historical attribution rewrite, reactivation, and tasks absent from the plan.
- Owner input remaining: none.

### DEC-BIP-004 — Plan-only in this round

- Decision owner: `mayf3`.
- Selected direction: create only this proposed child Spec because authority is insufficient.
- Rejected: implementation, production read, production apply, manual SQL, or claiming a canonical live plan without live evidence.
- Owner input remaining: none.

## 9. Contracts

These Contracts are proposed requirements only. With `implementation_authority: none`, they authorize no code or runtime action.

### CTR-BIP-AUTH-001 — Exact pair and excluded Principal

Any future operator MUST compile in only the exact `OLD` and `NEW` UUIDs in §1, MUST expose no arbitrary Principal flags, and MUST reject any appearance of `81c7fc7e-c696-4b47-bfd6-f12a9ecb68a6` in selected Domain/responsibility scope or successor metadata. Names are explanatory labels, never identity selectors.

### CTR-BIP-MODE-001 — Offline modes and safe default

The one-time operator MUST support exactly `--plan`, `--apply`, and `--verify`; omission of a mode MUST behave as `--plan`. `--plan` and `--verify` MUST use read-only database transactions and perform zero `INSERT`, `UPDATE`, `DELETE`, DDL, receipt, event, or audit writes. No HTTP reassign surface may be added.

### CTR-BIP-LIVE-001 — Exact consistent live read

`--plan` MUST use the already supplied production read-only connection only after source/checkout gates, execute only `SELECT` in one consistent read-only PostgreSQL transaction, and derive live facts from current repository predicates rather than historical snapshots. It MUST neither print nor persist secrets, tokens, credentials, or `DATABASE_URL`.

### CTR-BIP-COUNT-001 — Six exact count families

For both `OLD` and `NEW`, the canonical plan MUST report these exact live counts with explicit predicate/version metadata:

1. `active_worklist_tasks` — rows visible under the actual assigned-worklist predicate;
2. `current_node_tasks` — instances whose `current_node_visit_id` points to a Visit assigned to the Principal;
3. `non_terminal_current_responsibilities` — current-node tasks whose node is non-terminal and instance is active, non-cancelled, and non-archived;
4. `distinct_tasks_ever_assigned` — distinct workflow instances with any immutable Visit assigned to the Principal;
5. `historical_node_visit_assignment_records` — immutable Visit rows assigned to the Principal;
6. `completed_or_terminal_history` — distinct historically assigned instances that are completed/terminal at snapshot time, reported only as read-only evidence.

The plan MUST include the exact row identity arrays or deterministic digests necessary to independently reproduce each count. History counts are evidence, not migration selectors.

### CTR-BIP-DOMAIN-001 — Exact live Domain tuples

The plan MUST read and print each selected binding as:

```text
domain_id
domain_key
role
enabled
principal_id
created_at
updated_at
binding_id
```

The expected `OLD` enabled set is exactly:

| domain_key | role |
|---|---|
| `build-in-public-dogfood` | `DOMAIN_OWNER` |
| `adc-v2-dogfood` | `DOMAIN_MEMBER` |
| `commercial-exploration-dogfood` | `DOMAIN_MEMBER` |
| `game-dev` | `DOMAIN_MEMBER` |
| `hr-onboarding` | `DOMAIN_MEMBER` |
| `journal-submission` | `DOMAIN_MEMBER` |
| `knowledge-curation` | `DOMAIN_MEMBER` |
| `okr-dogfood` | `DOMAIN_MEMBER` |
| `workflow-todo-dogfood` | `DOMAIN_MEMBER` |

Any missing, extra, duplicated, disabled, role-changed, Principal-changed, or identity/timestamp-drifted tuple relative to the reviewed plan MUST yield:

```text
DOMAIN_WRITES = 0
RESULT = CONFLICT
FAIL_LOUD = YES
```

### CTR-BIP-DOMAIN-002 — Formal atomic successor semantics

A future apply MAY transfer only the nine reviewed tuples. The `DOMAIN_OWNER` tuple MUST use formal owner replacement semantics. Each `DOMAIN_MEMBER` tuple MUST use successor enable/disable semantics. NEW enable/re-enable and OLD disable MUST occur in the same serializable transaction as all responsibility, Receipt, Event, and Audit changes. No committed state may retain dual OLD/NEW authority for a transferred role. Exact successful rerun MUST be `NOOP` with zero writes.

### CTR-BIP-RESP-001 — Exact active responsibility predicate and output

A plan candidate MUST satisfy all of:

```text
current Visit assignee = OLD
instance.current_node_visit_id = Visit.id
node = active and non-terminal
instance cancelled = false
instance archived = false
Domain enabled = true
visible under actual assigned-worklist semantics
```

For each candidate the plan MUST output at least:

```text
workflow_instance_id
workflow_title
current_status
current_node_id
current_node_title
current_visit_id
current_assignee
workflow_state_version
domain_id
```

It MUST also explicitly freeze `definition_version_id`, `visit_number`, `entered_by_transition_id`, Visit `created_at`, current context revision, current workflow status, `cancelled`, `archived_at`, node type, Domain enabled state, and open visit-scoped assistance count, plus every identity/state field needed for CAS and recovery validation. Zero eligible candidates is valid and MUST report `ACTIVE_RESPONSIBILITY_TRANSFER_COUNT = 0` without reactivating history.

### CTR-BIP-RESP-002 — Append-only successor facts and CAS

A future apply MAY transfer only reviewed candidates that still exactly match the plan. For each candidate it MUST append a dedicated successor Visit assigned to `NEW`, append a dedicated successor Event, complete a dedicated Receipt, and append a durable security Audit. It MUST CAS the exact expected `workflow_state_version` and current Visit identity. Every pre-existing Visit and assignment remains immutable. Conflict MUST roll back all Domain and responsibility changes.

### CTR-BIP-RECOVERY-001 — Fail-closed replay compatibility

Admin Recovery replay MUST recognize only the dedicated Build in Public successor event shape authorized by a later implementation Spec. Replay MUST validate exact migration ID, exact OLD/NEW pair, command/Visit identities, same-instance/same-node successor Visit, event digest, unchanged context revision, and expected state-version progression. Replay advances only reconstructed current projection; it MUST NOT issue migration writes, update history, manufacture events, or reinterpret the event as normal Transition, ordinary reassignment, or emergency override.

### CTR-BIP-HISTORY-001 — Historical immutability and exclusions

Future apply MUST update or delete zero pre-existing Visits, assignments, Events, Submissions, Context revisions, Receipts, and historical Audits. Completed, terminal, cancelled, and archived tasks; ever-assigned records; the legacy 5,583 archive; `blog-agent`; and every unplanned Domain/task remain untouched. Appending a successor Visit may increase `NEW.historical_node_visit_assignment_records`; that is new provenance, not rewriting OLD history.

### CTR-BIP-PLAN-001 — Canonical redacted plan

The plan MUST be canonical RFC 8785/JCS UTF-8 bytes with no BOM or trailing newline and MUST contain:

- exact Spec/migration/tool source identities and clean-tree assertion;
- exact OLD/NEW and explicit excluded Principal;
- redacted database identity evidence with no credential material;
- exact nine Domain before tuples and deterministic expected after tuples;
- exact active responsibility before tuples, expected state versions, and deterministic expected after tuples;
- all six OLD/NEW before counts and expected after counts;
- immutable history identity arrays/digests sufficient for non-rewrite proof;
- `snapshotDigest` over the plan object with that field omitted.

`PLAN_SHA256` / `planFileSha256` MUST NOT be embedded in the plan object. After writing the final canonical bytes, the operator MUST compute SHA-256 over those exact bytes and print it separately as the review envelope/output. Expected-after calculations MUST preserve OLD historical counts, add only appended NEW successor Visit provenance, leave completed/terminal history unchanged, and compute distinct-ever-assigned changes from exact candidate membership rather than assuming transfer count equals distinct-count delta.

### CTR-BIP-CONFLICT-001 — Plan/apply exact match and fail closed

Before any future apply write, the operator MUST re-read database identity, both Principals, all Domain tuples, all candidate responsibilities, all expected state versions, and all historical identity/digest evidence. Any drift, ambiguity, unexpected role, excluded Principal, extra/missing row, open assistance case, or digest mismatch MUST return conflict with zero committed writes. Selection MUST never widen to make expected counts match.

### CTR-BIP-ATOMIC-001 — One outcome and exact NOOP

All nine Domain transfers, zero-or-more reviewed active-responsibility successor chains, projection CAS changes, completed Receipt(s), and durable Audit MUST commit as one PostgreSQL serializable transaction or not commit at all. Exact metadata rerun after success MUST verify immutable Receipt/Event anchors, Audit metadata, all post-state, and historical invariants before returning `NOOP`; mismatch MUST fail closed and MUST NOT self-heal.

### CTR-BIP-GATE-001 — Separate acceptance and production gates

Even after lawful parent reconciliation, this child requires independent semantic review, Owner acceptance, merge to `main`, and a later implementation-authorizing authority before coding. Production apply additionally requires a separate reviewed execution record pinning implementation full Git SHA, clean checkout, canonical plan and `PLAN_SHA256`, database identity, operator/migration actor, backup/recovery readiness, command transcript, and independent approval. No Spec acceptance or merge alone authorizes production apply.

## 10. Acceptance

### ACC-BIP-AUTH-001 — Authority and pair closure

- Contracts: `CTR-BIP-AUTH-001`, `CTR-BIP-GATE-001`.
- Method/environment: independent review of exact parent and child commits.
- Expected: lawful parent authority explicitly permits only this new pair; child remains non-implementing until a separately accepted implementation authority exists; excluded Principal cannot enter scope.
- Failure condition: child expands V3 by itself, accepts arbitrary pairs, or implies production apply.

### ACC-BIP-PLAN-001 — Read-only canonical plan conformance

- Contracts: `CTR-BIP-MODE-001`, `CTR-BIP-LIVE-001`, `CTR-BIP-COUNT-001`, `CTR-BIP-DOMAIN-001`, `CTR-BIP-RESP-001`, `CTR-BIP-PLAN-001`.
- Method/environment: disposable PostgreSQL conformance fixture plus separately authorized production read-only plan evidence.
- Expected: default plan; SELECT-only; exact six count families; exact tuple arrays/digests; canonical redacted bytes and reproducible SHA-256; zero writes.
- Failure condition: any mutation, secret disclosure, snapshot substitution, count-only approval, non-canonical bytes, or omitted tuple/version.

### ACC-BIP-APPLY-001 — Atomic exact successor behavior

- Contracts: `CTR-BIP-DOMAIN-002`, `CTR-BIP-RESP-002`, `CTR-BIP-HISTORY-001`, `CTR-BIP-CONFLICT-001`, `CTR-BIP-ATOMIC-001`.
- Method/environment: disposable PostgreSQL tests with mutation-phase fault injection; no production apply during implementation review.
- Expected: one owner + eight member transfers; only reviewed eligible current responsibility; append-only facts; CAS; all-phase rollback; exact NOOP; zero history rewrite.
- Failure condition: partial commit, dual authority, rewritten history, widened selection, non-NOOP rerun, or any excluded row touched.

### ACC-BIP-RECOVERY-001 — Recovery replay

- Contracts: `CTR-BIP-RECOVERY-001`.
- Method/environment: disposable recovery/replay fixtures for valid and malformed dedicated successor events.
- Expected: exact valid event reconstructs the successor projection; every metadata/identity/version mismatch fails closed without mutation.
- Failure condition: unknown event breaks valid recovery, malformed event is accepted, or replay creates/rewrites business facts.

```text
CONTRACT_COUNT = 14
CONTRACTS_WITH_ACCEPTANCE = 14
```

## 11. Alternatives and disposition

| Alternative | Disposition | Reason |
|---|---|---|
| Reuse accepted CTO successor authority for this pair | Rejected | Exact UUID authority cannot be expanded by analogy. |
| Amend the accepted CTO implementation Spec in place | Rejected | Accepted meaning and fixed pair are immutable; this is a new independent normative meaning. |
| General Principal migration/reassign API | Rejected | Violates Product Direction and creates a durable arbitrary-pair capability. |
| Manual SQL/`psql` mutation | Rejected | Bypasses plan review, CAS, append-only facts, audit, recovery, atomicity, and NOOP. |
| Rewrite OLD historical assignments to NEW | Rejected | Destroys immutable provenance and falsely changes completed history. |
| Migrate `blog-agent` with Build in Public | Rejected | Explicitly unrelated Principal and outside exact-pair authority. |
| Run production `--plan` before authority closure in this round | Rejected | Owner instructed Spec-only fallback when authority is insufficient. |

## 12. Migration, compatibility, and rollback

This proposed file performs no migration and needs no runtime rollback. Its rollback is deletion/revert while still proposed.

A future accepted implementation must provide:

- forward-only one-time migration with no schema, SQL migration, or migration-file change in this operator; any separate schema proposal remains outside this Spec and cannot be consumed as implicit authority;
- exact pre-state plan and deterministic expected post-state;
- one serializable transaction;
- fault-injection proof for every mutation phase;
- recovery replay compatibility for the dedicated event;
- exact-rerun NOOP anchored by immutable Receipt/Event plus verified Audit/post-state;
- rollback by transaction rollback before commit, never compensating historical rewrites after commit;
- post-commit recovery/investigation procedure that fails closed and preserves the canonical plan and audit chain.

## 13. Open questions

```text
OPEN-BIP-001 = Which lawful Product Direction successor or higher-authority transition will authorize this second exact one-time pair without converting the exception into a reusable capability?
OPEN-BIP-002 = What exact implementation Spec/revision will grant implementation_authority=contracts after OPEN-BIP-001 closes?
OPEN-BIP-003 = What are the live production Domain tuples, six OLD/NEW count families, and eligible responsibility tuples from a later authorized read-only plan?
OPEN-BIP-004 = What exact implementation Git SHA, migration actor, database identity, backup evidence, and independent approvals will a later production execution record pin?

OPEN_OWNER_DECISIONS = OPEN-BIP-001, OPEN-BIP-002
NORMATIVE_TBD = parent authority reconciliation; implementation-authorizing revision
PARTIAL_SUPERSESSION = NONE
AUTHORING_READY_FOR_REVIEW = YES
IMPLEMENTATION_READY = NO
LIVE_PLAN_COMPLETE = NO
```

## 14. Final proposal fields

```text
TASK_NAME = 归属 执行
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = SVC_WORKFLOW_BUILD_IN_PUBLIC_PRINCIPAL_SUCCESSOR_V1
SPEC_KIND = implementation
STATUS = proposed
AUTHORITY_LEVEL = governing_spec
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V3
EXTERNAL_AUTHORITIES = NONE

OLD_PRINCIPAL = bb9d8f48-7962-4321-8fb1-554bb428c159
NEW_PRINCIPAL = d5b3aeb2-e754-49a9-9914-b963521c0985
EXCLUDED_PRINCIPAL = 81c7fc7e-c696-4b47-bfd6-f12a9ecb68a6
EXPECTED_OLD_ENABLED_DOMAIN_BINDING_COUNT = 9
EXPECTED_DOMAIN_OWNER_TRANSFER_COUNT = 1
EXPECTED_DOMAIN_MEMBER_TRANSFER_COUNT = 8

AUTHORITY_SUFFICIENT = CHILD_SPEC_REQUIRED
LIVE_PLAN_COMPLETE = NO
IMPLEMENTATION_PERFORMED = NONE
PRODUCTION_CHANGE = NONE
NEXT_TASK = 归属 审计
```
