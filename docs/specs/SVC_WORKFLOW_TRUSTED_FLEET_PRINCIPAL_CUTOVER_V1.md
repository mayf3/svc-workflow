---
spec_id: SVC_WORKFLOW_TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: none
scope:
  - mayf3/svc-workflow
  - trusted-fleet-principal-cutover
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
authority_chain:
  authority_id: SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
  fleet_exception_section: §17A
  accepted_main_revision: f4bfbb7cbc1dbcdb29c1caa472408adc41378fbf
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
title: Trusted Fleet Principal Cutover V1
repo: mayf3/svc-workflow
base_head: f4bfbb7cbc1dbcdb29c1caa472408adc41378fbf
production_apply_authorized_now: false
merge_required_for_activation: true
---

# SVC_WORKFLOW_TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1

## 0. Authority alignment and provenance

This proposed child implementation Spec is governed by the accepted Product Direction on `github/main`:

```text
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
BOUND_EXCEPTION = §17A trusted-fleet exact-plan-bound bounded successor exception
PARENT_ACCEPTED_MAIN_REVISION = f4bfbb7cbc1dbcdb29c1caa472408adc41378fbf
AUTHORITY_COMPATIBILITY = PASS
```

The parent exception authorizes only the exact frozen plan artifact and its reviewed rows. It keeps ordinary reassignment, handoff, delegation, arbitrary Principal-pair migration, runtime OLD/NEW parameters, dynamic roster expansion, general successor APIs, history rewrite, and terminal-task reactivation unauthorized.

Lifecycle state of this proposal:

```text
STATUS = proposed
implementation_authority = none
production_apply_authority = none
GOVERNING_AUTHORITY_REQUIRED_BEFORE_IMPLEMENTATION = accepted_on_main
```

This authoring round creates the Spec only. It performs no implementation, runs no production plan, writes no database, does not modify the frozen plan artifact, and does not close, modify, or merge PR #9.

## 1. Decision summary

This Spec proposes one bounded, offline, fleet-wide successor cutover for exactly the 86 exact successor pairs frozen in one canonical read-only plan artifact. It creates the 85 missing NEW Workflow Principal projections, transfers exactly the 760 reviewed Domain tuples and 80 reviewed active responsibilities, and rewrites zero history.

```text
ONE_TIME_OFFLINE_FLEET_CUTOVER = YES
GENERAL_PRINCIPAL_REASSIGNMENT_API = NO
HTTP_API_ADDED = NO
PRODUCT_CODE_CHANGED_BY_THIS_SPEC_PR = NO
DATABASE_SCHEMA_CHANGED = NO
PRODUCTION_APPLY_AUTHORIZED_BY_THIS_SPEC = NO
```

The frozen executable shape is:

```text
--plan (default; production read-only recheck, zero writes)
--apply <per authorized canary/batch>
--verify
exact rerun => per-pair NOOP
```

Acceptance of this Spec, when later authorized, activates only the bounded implementation Contracts of §11 at the exact accepted head on `main`. Acceptance and merge perform no implementation and authorize no production execution. Production apply requires a separate explicit owner authorization over an exact implementation Git SHA and the exact `PLAN_SHA256`.

## 2. Exact frozen inputs

The only authority input is the exact frozen local evidence artifact:

```text
PLAN_SCHEMA = workflow_trusted_fleet_successor_plan_v2
PLAN_PATH = /Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json
PLAN_SIZE_BYTES = 540472
PLAN_SHA256 = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606
ROSTER_SHA256 = f046d18f76da838ba94775af7c960d0ee548f2e392c22e6c7b0e3add36cb8e5f
PLAN_MODE = READ_ONLY_CANONICAL_PLAN
SNAPSHOT_OBSERVED_AT_UTC = 2026-08-24T01:03:53.192875+00:00
```

Frozen scope bound to `PLAN_SHA256`:

```text
TOTAL_NEW_AGENTS = 86
EXACT_SUCCESSOR_PAIR_COUNT = 86
AMBIGUOUS_COUNT = 0
CONFLICT_COUNT = 0

WORKFLOW_PROJECTION_CREATE_COUNT = 85
WORKFLOW_PROJECTION_PRESENT_COUNT = 1

DOMAIN_OWNER_TRANSFER_COUNT = 8
DOMAIN_MEMBER_TRANSFER_COUNT = 752
DOMAIN_TRANSFER_COUNT = 760

ACTIVE_RESPONSIBILITY_TRANSFER_COUNT = 80

DRAFT_OWNERSHIP_CANDIDATE_COUNT = 99
DRAFT_OWNERSHIP_MIGRATION_COUNT = 0
```

These counts are review scope, never live truth and never permission to select rows by count alone. Every phase re-verifies exact rows against the artifact and fails closed on drift.

The operator MUST NOT accept runtime OLD/NEW parameters and MUST NOT dynamically enumerate an 87th identity. All 86 pairs, 760 Domain tuples, 80 responsibilities, and the excluded identity are compiled from the exact artifact bytes verified against `PLAN_SHA256`.

## 3. Exact operator model

The future operator is one offline binary with exactly three modes; `--plan` is the default when no mode is given:

```text
OPERATOR_MODES = --plan | --apply | --verify
DEFAULT_MODE = --plan
```

`--plan` connects read-only, re-reads the live production state for all 86 pairs, compares it against the frozen artifact, emits a canonical recheck report (including the projection 85/1 state, all Domain tuples, all active responsibilities, and any drift), and reports `WRITES = 0`. `--apply` executes only rows covered by the separate production apply authorization, per pair, in the §9 canary order. `--verify` validates terminal state against the artifact with zero writes.

The operator must bind, before any database access:

- the accepted V4 revision;
- the accepted Child revision;
- the exact implementation Git SHA with exact HEAD equality and a clean checkout;
- the exact `PLAN_SHA256` and exact roster SHA;
- the exact 86 pairs compiled from the artifact;
- the exact excluded identity.

The operator must reject, before writes:

- a byte-different plan or any digest drift;
- arbitrary OLD/NEW arguments;
- display-name mapping;
- deriving an OLD identity by stripping the `agt_` prefix from a NEW agent_id (prefix stripping is never mapping evidence);
- fuzzy or prefix matching;
- dynamic roster expansion;
- any online generic migration API surface.

The plan file path must remain outside the Git checkout, so a reviewed plan cannot make the checkout dirty. The operator must never set or replace `DATABASE_URL` and must never add fallback database discovery.

## 4. Projection phase

The projection phase resolves the recorded systemic failure: 85 of 86 NEW Auth Principals are absent from the svc-workflow Principal projection, so assigned-to-me reads return `404 principal_not_found`.

Allowed operations only:

- create exactly the 85 missing NEW Workflow Principal projections named in the frozen artifact;
- for the one already-present projection `agt_build-in-public-agent`, perform an exact-match NOOP.

Before creating each projection, the operator must verify against the artifact and Auth that the NEW Auth Principal has:

- UUID exact;
- external identity exact;
- status `active`.

Forbidden:

- creating the excluded identity's projection or binding it anywhere;
- replacing any Auth Principal UUID;
- creating an 87th projection by dynamic enumeration, display-name pairing, prefix stripping, or fuzzy/prefix matching.

Terminal state after this phase:

```text
with current tasks    -> HTTP 200 + items
without current tasks -> HTTP 200 + items = []
residual 404 principal_not_found = FORBIDDEN
```

## 5. Domain successor phase

Only the 760 exact tuples in the artifact are processed.

`DOMAIN_OWNER` (8 tuples):

- exact OLD -> NEW transfer;
- atomic owner replacement using the repository's existing owner-replacement semantics;
- Domain unchanged;
- no committed dual Owner (the partial unique index preserving one enabled owner per Domain must hold).

`DOMAIN_MEMBER` (752 tuples):

- enable NEW;
- disable OLD;
- Domain and Role unchanged;
- no committed long-lived dual authority.

Each pair executes its Domain changes inside that pair's own SERIALIZABLE transaction. Any tuple drift — missing, additional, disabled, role-changed, Principal-changed, or otherwise mismatched against the artifact — yields:

```text
PAIR_WRITES = 0
OUTCOME = CONFLICT
```

The operator must never widen selection or force the authoring counts.

## 6. Active responsibility phase

Only the 80 exact responsibility tuples are processed, each re-validated at apply time inside the same transaction:

- current Visit is the instance's current visit;
- active;
- non-terminal;
- not cancelled;
- not archived;
- current assignee = OLD;
- expected workflow state version matches the artifact.

For each validated responsibility the operator must:

- append a same-node successor Visit assigned to NEW;
- append a dedicated successor Event;
- append a Receipt;
- append a durable Audit;
- CAS the workflow state version;
- keep the Instance unchanged;
- keep the node unchanged;
- keep historical Visits immutable;
- keep historical assignments immutable.

Records already completed, terminal, cancelled, or archived get zero migration and zero reactivation.

## 7. Draft creator boundary

The 99 creator-owned draft tuples in the artifact keep `created_by_principal_id` byte-preserved:

```text
DRAFT_SUCCESSOR_MIGRATION = FORBIDDEN
```

The operator must never rewrite a historical creator to NEW. If a current-maintainer concept is ever needed, a separate draft-stewardship authority must be established first; this Spec creates none.

## 8. Excluded identity

Exactly one identity is excluded:

```text
EXCLUDED_AGENT_ID = efficiency-agent
EXCLUDED_PRINCIPAL_ID = d09f8849-073c-484a-978c-f375113c28b2
EXCLUDED_CLASSIFICATION = EXCLUDED_DUPLICATE_IDENTITY
EXCLUDED_MIGRATION_CANDIDATE = false
EXCLUDED_FUTURE_OPERATOR_WRITES = 0
```

The only canonical efficiency pair:

```text
efficiency-manager / 95eab282-22c7-46a2-8580-abfef4942cdc
  -> agt_efficiency-agent / b21ddb23-42f6-47c4-a27f-bc44950e554c
```

Also frozen as two fully independent pairs:

```text
build-in-public-agent -> agt_build-in-public-agent
blog-agent -> agt_blog-agent
```

The two must never cross: `blog-agent` pairs only with `agt_blog-agent`, never with `agt_build-in-public-agent`, and vice versa.

## 9. Canary / fleet order

The future execution order is fixed:

1. production read-only plan recheck (`--plan`, zero writes);
2. exact `PLAN_SHA256` review of the recheck and artifact;
3. separate production apply authorization;
4. canary 1: `agt_build-in-public-agent`;
5. canary 2: `agt_efficiency-agent`;
6. remaining exact 84 pairs;
7. `--verify`;
8. exact rerun NOOP.

Both canaries must be verified before the remaining 84 pairs proceed:

- `workflow_my_tasks` no longer returns 404;
- `workflow_my_domains` matches the plan terminal state;
- active responsibilities match the plan terminal state;
- history is unchanged.

Each pair commits in its own independent SERIALIZABLE transaction. One pair's failure never fabricates another pair's success; already-committed pairs keep their committed outcomes, and the failed pair reports its exact outcome without repair attempts.

## 10. Outcomes

Every pair is reported with at least one of the frozen per-pair outcomes:

```text
PLANNED
NOOP
COMMITTED
CONFLICT
ROLLED_BACK
OUTCOME_UNKNOWN
```

Exact rerun after success returns per pair:

```text
OUTCOME = NOOP
WRITES = 0
NEW_AUDITS = 0
```

The rerun decision anchors on the per-pair immutable receipt/event/audit chain and exact post-state match; missing or inconsistent post-state is `CONFLICT`, never self-healing.

Prohibited everywhere:

```text
ordinary reassignment
handoff
delegation
general successor API
manual SQL
history rewrite
terminal-task reactivation
```

## 11. Implementation closure

The closure is mechanically derived from the existing CTO successor implementation/audit/recovery pattern in accepted `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` (§7 transaction order, §10 audit and exact-rerun NOOP, §12 three-file closure, §14 production run authorization), which is authorized on `main` but not yet implemented; no fleet-specific expansion beyond that pattern is proposed.

Candidate files and classification:

```text
1. src/bin/trusted_fleet_principal_cutover_v1.rs
   CLASSIFICATION = PROVEN_NECESSARY
   (offline --plan/--apply/--verify operator; compiled artifact constants:
   PLAN_SHA256, roster SHA, 86 pairs, excluded identity; per-pair
   SERIALIZABLE transactions; projection/domain/responsibility phases;
   audit; per-pair exact NOOP)

2. scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
   CLASSIFICATION = PROVEN_NECESSARY
   (disposable PostgreSQL runner: isolated database, existing migrations,
   focused scenarios from a clean fixed SHA, always drops the database)

3. tests/27_trusted_fleet_principal_cutover_v1.rs
   CLASSIFICATION = PROVEN_NECESSARY
   (focused integration/conformance tests invoking the actual binary and
   inspecting PostgreSQL facts)

Cargo.toml
   CLASSIFICATION = NOT_NECESSARY
   (cargo auto-discovers src/bin/*.rs; no [[bin]] or feature entry needed)

new SQL migration file
   CLASSIFICATION = NOT_NECESSARY
   (DATABASE_SCHEMA_CHANGED = NO; projection creation uses existing tables)

any HTTP surface, online management API, or additional per-phase binary
   CLASSIFICATION = NOT_NECESSARY
   (forbidden general migration capability)
```

```text
IMPLEMENTATION_FILES = 3
OWNER_DECISION_REQUIRED = NO
```

If implementation cannot be completed in these three files, work stops before creating a fourth file and reports:

```text
OWNER_DECISION_REQUIRED = YES
PROPOSED_FOURTH_FILE = <exact path>
WHY_UNAVOIDABLE = <specific reason>
SCOPE_IMPACT = <specific impact>
```

No implementation agent may self-authorize that expansion, and until such an OWNER decision is recorded the implementation remains blocked.

## 12. PR #9 disposition

```text
PR_9_DISPOSITION = SUPERSEDED_BY_FLEET_LOCAL_CHILD
```

PR #9 (single Build in Public pair Child) is superseded by this fleet-local Child. This task and this Spec must not close, modify, or merge PR #9; PR #9 remains OPEN at Head `3056263c3fc964a2b225720dd2b859b47e296c2e` until its Owner disposes of it.

## 13. Final frozen fields

```text
TASK_NAME = 全迁 执行

SPEC_ID = SVC_WORKFLOW_TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1
SPEC_FILE = docs/specs/SVC_WORKFLOW_TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1.md
BASE_HEAD = f4bfbb7cbc1dbcdb29c1caa472408adc41378fbf
COMMIT = REPORTED_EXTERNALLY_AFTER_DOCS_ONLY_COMMIT (not self-embedded)

PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
BOUND_EXCEPTION = §17A trusted-fleet exact-plan-bound bounded successor exception
PARENT_ACCEPTED_MAIN_REVISION = f4bfbb7cbc1dbcdb29c1caa472408adc41378fbf

PLAN_SHA256_BOUND = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606
ROSTER_SHA256_BOUND = f046d18f76da838ba94775af7c960d0ee548f2e392c22e6c7b0e3add36cb8e5f
EXACT_FLEET_PAIR_COUNT = 86
WORKFLOW_PROJECTION_CREATE_COUNT = 85
DOMAIN_TRANSFER_COUNT = 760
ACTIVE_RESPONSIBILITY_TRANSFER_COUNT = 80
DRAFT_OWNERSHIP_MIGRATION_COUNT = 0

HISTORY_REWRITE_ALLOWED = NO
GENERAL_MIGRATION_CAPABILITY = NO
RUNTIME_OLD_NEW_PARAMETERS = FORBIDDEN
TRANSACTION_MODEL = PER_PAIR_POSTGRESQL_SERIALIZABLE
NOOP_MODEL = PER_PAIR_EXACT_RECEIPT_EVENT_AUDIT_CHAIN_AND_POSTSTATE_MATCH
IMPLEMENTATION_FILES = 3

STATUS = proposed
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
IMPLEMENTATION_PERFORMED = NO
PRODUCTION_PLAN_EXECUTED = NO
PRODUCTION_APPLY_AUTHORIZED_NOW = NO
MERGE_REQUIRED_FOR_ACTIVATION = YES
PRODUCTION_CHANGE = NONE
```

End of proposed Spec. This document authorizes nothing by itself; implementation authority activates only through independent review, Owner acceptance, and merge of this exact head to `main`, and production apply remains a separate later gate.
