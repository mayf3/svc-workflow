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

Amendment provenance (this revision):

```text
AMENDED_PROPOSAL_HEAD = 9ac2ac79b36fec52b9d81706c66a9bc9f2337a07
LATEST_REVIEW = 全迁 审计 = REVISE
BLOCKERS = 1
REQUIRED_FIXES = 5
OWNER_CLOSURE_RULING = AUTHORIZE_MINIMAL_ADMIN_RECOVERY_REPLAY_CLOSURE
AMENDMENT_SCOPE = DOCS_ONLY
```

The Owner closure ruling authorizes revising the prior three-file closure to the exact N-file closure mechanically necessary for fleet successor event replay. This is a closure correction only, not a product-semantic expansion. The amended closure is fully frozen in §13 with every candidate mechanically classified; no candidate is deferred as "apply later if needed".

Lifecycle state of this proposal:

```text
STATUS = proposed
implementation_authority = none
production_apply_authority = none
GOVERNING_AUTHORITY_REQUIRED_BEFORE_IMPLEMENTATION = accepted_on_main
```

This amendment round modifies the proposed Spec only. It performs no implementation, runs no production plan, writes no database, does not modify the frozen plan artifact or Product Boundary V4, does not acceptance-finalize, does not merge, and does not close, modify, or merge PR #9.

## 1. Decision summary

This Spec proposes one bounded, offline, fleet-wide successor cutover for exactly the 86 exact successor pairs frozen in one canonical read-only plan artifact. For each pair it creates the missing NEW Workflow Principal projection (exact-match NOOP when already present), transfers exactly the reviewed Domain tuples and active responsibilities, and rewrites zero history — all inside one per-pair SERIALIZABLE transaction.

```text
ONE_TIME_OFFLINE_FLEET_CUTOVER = YES
GENERAL_PRINCIPAL_REASSIGNMENT_API = NO
HTTP_API_ADDED = NO
PRODUCT_CODE_CHANGED_BY_THIS_SPEC_PR = NO
DATABASE_SCHEMA_CHANGED = NO
PRODUCTION_APPLY_AUTHORIZED_BY_THIS_SPEC = NO
```

Closed models delivered by this amendment:

```text
APPLY_SCOPE_MODEL = CLOSED (§3)
PROJECTION_PAYLOAD_MODEL = CLOSED (§4; display_name STOP ruling with mandatory new-plan + Product Boundary amendment path)
EVENT_RECEIPT_AUDIT_MODEL = CLOSED (§7; ruling B fleet-specific event type)
ADMIN_RECOVERY_REPLAY_COMPATIBILITY = FROZEN (§8)
OUTCOME_MODEL = CLOSED (§12; six outcomes + OUTCOME_UNKNOWN re-observation protocol)
CANARY_CHECK_COUNT = 6 (§11)
IMPLEMENTATION_CLOSURE = EXACT_FOUR_FILES (§13)
```

Acceptance of this Spec, when later authorized, activates only the bounded implementation Contracts of §13 at the exact accepted head on `main`. Acceptance and merge perform no implementation and authorize no production execution. Production apply requires a separate explicit owner authorization over an exact implementation Git SHA and the exact `PLAN_SHA256`.

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

## 3. Closed apply-scope model

The future CLI exposes exactly one default read-only mode, three closed apply scopes, and one verify mode:

```text
--plan
  default when no mode is given;
  all 86 rows, read-only live recheck against the frozen artifact;
  zero writes.

--apply --scope build-in-public-canary
  exactly the one pair agt_build-in-public-agent.

--apply --scope efficiency-canary
  exactly the one pair agt_efficiency-agent.

--apply --scope remaining-fleet
  exactly the remaining 84 artifact pairs.

--verify
  all 86 rows, terminal-state verification against the frozen artifact;
  zero writes.
```

The scope enum is closed. Forbidden flags and inputs:

```text
--old
--new
--agent-id
any Principal UUID argument
any row index argument
any self-chosen subset
any dynamic scope
```

Canary gating is mandatory and checked before any later scope opens:

```text
Build in Public canary NOT PASS:
  EFFICIENCY_CANARY_WRITES = 0
  REMAINING_84_WRITES = 0

Efficiency canary NOT PASS:
  REMAINING_84_WRITES = 0
```

The operator must additionally bind, before any database access: the accepted V4 revision; the accepted Child revision; the exact implementation Git SHA with exact HEAD equality and a clean checkout; the exact `PLAN_SHA256` and exact roster SHA; the exact 86 pairs compiled from the artifact; the exact excluded identity. The plan file path must remain outside the Git checkout; the operator must never set or replace `DATABASE_URL` and must never add fallback database discovery.

## 4. Projection creation payload model

The projection phase resolves the recorded systemic failure: 85 of 86 NEW Auth Principals are absent from the svc-workflow `principals` projection, so assigned-to-me reads return `404 principal_not_found`.

### 4.1 Frozen column payload

For each missing NEW projection, the complete write model over `principals` (`migrations/0001_identity_domain.sql:48`) is:

```text
principal_id   = exact NEW principal UUID from the frozen artifact row
principal_type = 'AGENT'
enabled        = true
email          = NULL
metadata       = NULL
display_name   = <see §4.2; currently BLOCKED, no lawful value exists>
```

Before creating each projection, the operator must verify against the artifact and Auth that the NEW Auth Principal has UUID exact, external identity exact (`new_principal_external_ref`), and status `active`.

Forbidden derivation for `display_name` (and for any identity mapping):

- constructing from `agent_id`;
- guessing from a display name;
- deriving an OLD identity by stripping the `agt_` prefix from a NEW agent_id (prefix stripping is never mapping evidence);
- prefix matching;
- fuzzy matching.

### 4.2 DISPLAY_NAME_AUTHORITY_SOURCE ruling

`principals.display_name` is `NOT NULL` (1–256 chars), so projection creation cannot commit without a display_name value. The frozen authorities were examined mechanically:

```text
frozen plan v2 fleet_rows keys = {apply_preconditions, classification,
  classification_reason, evidence, future_writes_allowed_now, new_agent_id,
  new_counts, new_principal_external_ref, new_principal_id,
  new_principal_status, old_agent_id, old_counts, old_principal_external_ref,
  old_principal_id, old_principal_status}  -> no display_name
frozen roster (ROSTER_SHA256 = f046d18f...) rows = {agent_id, principal,
  client, principal_type, store, client_id_present, classification,
  reason_code}  -> no display_name
frozen openclaw.json (sha256 3d34b79c...)  -> historical OLD definitions only
frozen primary-workspaces.json (sha256 e3c27c39...)  -> workspace paths only
```

Ruling:

```text
DISPLAY_NAME_AUTHORITY_SOURCE = NONE_AVAILABLE_IN_FROZEN_AUTHORITIES
```

STOP. No display_name value may be invented or derived. A successor plan artifact containing the exact projection payload (including an authoritative display_name per NEW principal) must first be generated and authorized through the corresponding Product Boundary amendment (V4 §17A.1 digest-freeze discipline). Until that amendment lands:

```text
PROJECTION_CREATE = BLOCKED
OPERATOR_DISPLAY_NAME_INPUT = FORBIDDEN
REQUIRED_PATH = NEW_EXACT_PAYLOAD_PLAN + PRODUCT_BOUNDARY_AMENDMENT
```

The payload model is closed by this ruling: the four mechanical columns are frozen in §4.1, and the display_name input is frozen to "no lawful source exists yet; blocked pending the amended authority". Implementation of the projection write path must not proceed against invented values, and conformance fixtures for that path must consume the amended authority once it exists.

### 4.3 Same-transaction rule

For every pair, the following steps all commit inside that pair's single SERIALIZABLE transaction:

```text
projection create / exact-match NOOP
+ Domain transfer
+ responsibility transfer
+ Event / Receipt / Audit
+ terminal-state re-select
```

No step of a pair commits independently; a pair is all-or-nothing.

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

Any tuple drift — missing, additional, disabled, role-changed, Principal-changed, or otherwise mismatched against the artifact — yields `PAIR_WRITES = 0`, `OUTCOME = CONFLICT` for that pair. The operator must never widen selection or force the authoring counts.

## 6. Active responsibility phase

Only the 80 exact responsibility tuples are processed, each re-validated at apply time inside the same pair transaction:

- current Visit is the instance's current visit;
- active;
- non-terminal;
- not cancelled;
- not archived;
- current assignee = OLD;
- expected workflow state version matches the artifact.

For each validated responsibility the operator appends the frozen successor fact chain of §7 and keeps Instance, node, historical Visits, and historical assignments unchanged. Records already completed, terminal, cancelled, or archived get zero migration and zero reactivation.

## 7. Event / Receipt / Audit frozen model

### 7.1 Event-type ruling

Ruling: **B — new fleet-specific event type with a frozen schema and replay semantics.**

```text
EVENT_TYPE = TRUSTED_FLEET_PRINCIPAL_CUTOVER_COMMITTED
EVENT_SCHEMA_VERSION = v1
```

Mechanical grounds, citing `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` §9 and the current Admin Recovery replay implementation (`src/store/postgres/admin_recovery_repository/event_replay.rs`):

- the CTO event's closed data schema is frozen to the CTO migration identity (fixed CTO `migrationId`, nine-tuple semantics) and carries no `pair_index`/`plan_sha256`; reusing `PRINCIPAL_SUCCESSOR_MIGRATION_COMMITTED` with a different data shape would break that frozen closed schema;
- V4 §17A.9 forbids abstracting the CTO and fleet mappings into one shared mechanism; separate frozen types keep each bounded;
- replay recognition in `event_replay.rs` must change under either ruling (unknown types fail loud today), and the Owner closure ruling already classifies that file `PROVEN_NECESSARY`.

The choice is not left to any implementation agent.

### 7.2 Frozen event row shape

Per responsibility successor, one `workflow_events` row (`migrations/0004_workflow_events.sql`) with exactly:

```text
event_type               = TRUSTED_FLEET_PRINCIPAL_CUTOVER_COMMITTED
event_schema_version     = v1
event_sequence           = prior sequence + 1
old_workflow_state_version = artifact expected_state_version
new_workflow_state_version = expected_state_version + 1
source_node_visit_id     = old_visit_id (the immutable OLD-assigned current visit)
target_node_visit_id     = new_visit_id (the appended successor visit)
from_node_id = to_node_id = node_id (same node; no graph movement)
context_revision_id      = current context revision (unchanged)
submission_id            = NULL
transition_effect        = NULL
command_id               = receipt command id
causation_id             = NULL
correlation_id           = NULL
actor_principal_id       = reviewed operator principal (must exist, be enabled, and be projected; replay joins principals)
```

The successor visit has `entered_by_transition_id = NULL`, `visit_number = MAX(visit_number for instance/node) + 1`, and `assignee_principal_id = NEW`.

### 7.3 Closed event_data shape

`event_data` is a closed object with exactly these keys:

```text
migration_id               = "trusted-fleet-principal-cutover-v1" (fixed)
plan_sha256                = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606
pair_index                 = artifact row order, integer 1..86 (pair identity)
old_agent_id / new_agent_id   = artifact row agent ids (pair identity)
old_principal_id / new_principal_id = artifact row principal UUIDs
workflow_instance_id       = the responsibility tuple's instance
old_visit_id / new_visit_id = source visit / appended successor visit
node_id                    = unchanged current node
expected_state_version     = artifact expected workflow state version
resulting_state_version    = expected_state_version + 1
before_projection_digest   = SHA-256 of the canonical BeforeSnapshotV1-shaped object (workflow_instance_id, domain_id, definition_version_id, created_by_principal_id, projection{current_context_revision_id, current_node_visit_id, workflow_state_version}) over the pre-state
after_projection_digest    = same canonical construction over the post-state
causation_id               = null
correlation_id             = null
occurred_at                = UTC RFC 3339 with exactly six fractional digits
```

`event_data_digest` is the repository canonical JSON digest of this object.

### 7.4 Frozen Receipt shape

Per pair, one command receipt with exactly:

```text
command_type     = TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1
idempotency_key  = "trusted-fleet-cutover-v1:" + pair_index
principal_id     = reviewed operator principal
request_hash     = SHA-256 over the canonical object {specId, migrationId,
                   planFileSha256, pairIndex, oldPrincipal, newPrincipal,
                   sourceGitSha, databaseIdentity, operatorPrincipal}
response_status  = 200 on COMMITTED
response_body    = fixed object: command id, pair identity, outcome,
                   before/after projection digests, event id, audit id;
                   zero secret and zero workflow business body
```

### 7.5 Frozen Audit shape

Per pair, one durable audit row with exactly:

```text
action           = TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1_COMMITTED
resource_type    = TRUSTED_FLEET_PRINCIPAL_CUTOVER
resource_id      = "trusted-fleet-principal-cutover-v1:" + pair_index
principal_id     = actual operator principal (never OLD, never NEW, never self-report)
details          = spec id, OLD/NEW, plan SHA, pair identity,
                   affected Domain count, affected responsibility count,
                   decision/outcome, correlation fields (event id, receipt/command id);
                   zero workflow business body
```

## 8. Admin Recovery replay compatibility

Frozen against the current implementation (`event_replay.rs`, `snapshot.rs`):

- **Recognition**: `Replay::apply` gains exactly one new match arm for `"TRUSTED_FLEET_PRINCIPAL_CUTOVER_COMMITTED"` and one bounded handler function. Every existing arm keeps its semantics byte-for-byte; the `_ =>` unknown-event arm stays fail-loud (`"event type is not supported by recovery replay"`); generic unknown-event handling is not loosened and no other event's replay semantics change.
- **Handler validation** (mechanical, mirroring existing handlers): the closed 18-key `event_data` shape via `exact_keys`; `event_schema_version = "v1"` and contiguous sequence/state versions (already enforced by `apply`); source visit equals the replayed current visit; target visit introduced exactly once with `visit_number = MAX(visit_number for node) + 1` and `entered_by_transition_id = NULL`; `from_node_id = to_node_id = source.node_id = target.node_id`; context equals the current context; `submission_id` and `transition_effect` NULL; target assignee equals `new_principal_id`; data OLD/NEW identities equal the compiled artifact constants; `expected_state_version` equals the replayed version; `resulting_state_version` equals the next sequence; `before_projection_digest` equals the digest recomputed from the replayed pre-state; `after_projection_digest` equals the digest of the resulting projection; actor equals the compiled operator principal.
- **Post-replay consistency**: after the arm applies, the replayed projection carries `current_visit = new_visit`, assignee NEW, `workflow_state_version + 1` — exactly the committed terminal state.
- **Unknown/malformed successor events fail loud**: any key drift, digest mismatch, or non-contiguous version returns `RecoveryError::InvalidImmutableFacts` before any recovery write; recovery replay is read-only.
- **History never rewritten**: replay only introduces the successor visit; pre-existing Visits, assignments, Events, Submissions, and receipts are never mutated (database immutability triggers guard additionally).
- **Exact rerun never creates a second successor chain**: the per-pair rerun decision (§12) precedes first-run gates on the frozen receipt/idempotency identity and returns NOOP with zero writes, zero new audits; replay observes exactly one successor chain per responsibility.

## 9. Draft creator boundary

The 99 creator-owned draft tuples in the artifact keep `created_by_principal_id` byte-preserved:

```text
DRAFT_SUCCESSOR_MIGRATION = FORBIDDEN
```

The operator must never rewrite a historical creator to NEW. If a current-maintainer concept is ever needed, a separate draft-stewardship authority must be established first; this Spec creates none.

## 10. Excluded identity

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

## 11. Canary checks and fleet order

The future execution order is fixed:

1. production read-only plan recheck (`--plan`, all 86 rows, zero writes);
2. exact `PLAN_SHA256` review of the recheck and artifact;
3. separate production apply authorization;
4. `--apply --scope build-in-public-canary`;
5. `--apply --scope efficiency-canary`;
6. `--apply --scope remaining-fleet` (the exact 84);
7. `--verify` (all 86 rows);
8. exact rerun NOOP.

Each canary must PASS all six checks before the next scope opens:

1. projection terminal state exact (the canary pair's NEW projection exact against the frozen authority);
2. `workflow_my_tasks` no longer returns 404 for the canary NEW principal;
3. `workflow_my_domains` exact against the plan terminal state;
4. active responsibilities exact against the plan terminal state;
5. historical Visit/assignment digest unchanged;
6. excluded identity `efficiency-agent`/`d09f8849-073c-484a-978c-f375113c28b2` fully unchanged with writes = 0.

```text
ANY_CHECK_NOT_PASS:
  CANARY_RESULT = FAIL
  REMAINING_84_WRITES = 0
```

A successful command invocation alone is never canary PASS. One pair's failure never fabricates another pair's success; committed pairs keep their committed outcomes.

## 12. Outcomes

Every pair is reported with exactly one of the six frozen outcomes:

```text
PLANNED = read-only plan exact match; zero writes.

NOOP = the pair is already in exact terminal state, with receipt, event,
audit, and post-state all precisely matching; zero writes, zero new audit.

COMMITTED = the per-pair transaction is confirmed committed and the
terminal-state re-select passes exactly.

CONFLICT = precondition, tuple, state version, receipt, event, audit, or
post-state drift; that pair commits zero writes.

ROLLED_BACK = the transaction is explicitly not committed; the exact
pre-state is unchanged.

OUTCOME_UNKNOWN = the commit response is lost or the database state is
temporarily unobservable, so whether the pair committed cannot currently
be determined.
```

`OUTCOME_UNKNOWN` handling is frozen:

- no blind replay;
- no second projection creation;
- no repeated append of Visit/Event/Receipt/Audit;
- an exact read-after-failure re-observation MUST be performed first.

Re-observation mapping:

```text
exact committed terminal state -> COMMITTED
exact original pre-state       -> ROLLED_BACK
partial / inconsistent state   -> CONFLICT
database still unobservable    -> OUTCOME_UNKNOWN
```

Exact rerun after success returns per pair `NOOP` with `WRITES = 0`, `NEW_AUDITS = 0`. The rerun decision anchors on the per-pair immutable receipt/event/audit chain and exact post-state match; missing or inconsistent post-state is `CONFLICT`, never self-healing.

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

## 13. Implementation closure

The Owner ruling `AUTHORIZE_MINIMAL_ADMIN_RECOVERY_REPLAY_CLOSURE` revises the prior three-file closure. The closure below is mechanically derived from (a) the CTO successor implementation/audit/recovery pattern (`SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` §7/§9/§10/§12/§14) and (b) the real Admin Recovery replay call graph at base `f4bfbb7` (`event_replay.rs` dispatch; `snapshot.rs` unfiltered event loading with actor-joins-principals; `event_fields.rs` generic helpers; `rows.rs` fact structs; `workflow_events`/`principals` schemas in migrations 0001/0004).

Exact closure — every file classified, none deferred:

```text
1. src/bin/trusted_fleet_principal_cutover_v1.rs
   CLASSIFICATION = PROVEN_NECESSARY
   (offline --plan/--apply --scope/--verify operator; compiled artifact
   constants: PLAN_SHA256, roster SHA, 86 pairs, excluded identity; closed
   scope enum; per-pair SERIALIZABLE transactions covering §4.3; frozen
   E/R/A facts of §7; per-pair exact NOOP)

2. scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
   CLASSIFICATION = PROVEN_NECESSARY
   (disposable PostgreSQL runner: isolated database, existing migrations,
   focused scenarios from a clean fixed SHA, always drops the database)

3. tests/27_trusted_fleet_principal_cutover_v1.rs
   CLASSIFICATION = PROVEN_NECESSARY
   (focused integration/conformance tests invoking the actual binary,
   inspecting PostgreSQL facts, and exercising the replay compatibility
   matrix of §8 and the outcome matrix of §12)

4. src/store/postgres/admin_recovery_repository/event_replay.rs
   CLASSIFICATION = PROVEN_NECESSARY
   (Owner-ruled; exactly one new match arm + one bounded handler for
   TRUSTED_FLEET_PRINCIPAL_CUTOVER_COMMITTED per §8; authorization is
   limited to recognizing only this Spec's frozen successor event/schema)
```

Mechanically confirmed NOT necessary (used as-is, unmodified):

```text
admin_recovery_repository/event_fields.rs
  NOT_NECESSARY — exact_keys/uuid_field/string_field/optional_string_field/
  event_data digest check already cover the closed 18-key string/uuid shape;
  no new helper is required.
admin_recovery_repository/rows.rs
  NOT_NECESSARY — EventFact already loads every column the §8 handler
  validates (type, schema version, versions, visits, nodes, data+digest);
  VisitFact already carries assignee/visit_number/entered_by_transition_id.
admin_recovery_repository/snapshot.rs
  NOT_NECESSARY — load_events selects all events of an instance unfiltered
  and joins the actor to principals; the successor event is an ordinary row
  and the actor projection exists post-commit.
admin_recovery_repository/{rebuild_transaction.rs, override_transaction.rs,
  receipt.rs, authorization.rs, import_event.rs, mod.rs}
  NOT_NECESSARY — no fleet contact point; event_replay is already a declared
  private module.
src/domain/workflow_instance/recovery.rs (WorkflowProjection,
  BeforeSnapshotV1, RecoveryError) and domain digest helpers
  NOT_NECESSARY — reused unmodified.
Cargo.toml
  NOT_NECESSARY — cargo auto-discovers src/bin/*.rs.
new SQL migration file
  NOT_NECESSARY — DATABASE_SCHEMA_CHANGED = NO; projection creation and
  successor facts use existing tables.
any HTTP surface, online management API, or extra per-phase binary
  NOT_NECESSARY — forbidden general migration capability.
```

```text
IMPLEMENTATION_FILES = 4
OWNER_DECISION_REQUIRED = NO
```

If implementation cannot be completed in these four files, work stops before creating a fifth file and reports:

```text
OWNER_DECISION_REQUIRED = YES
PROPOSED_FIFTH_FILE = <exact path>
WHY_UNAVOIDABLE = <specific reason>
SCOPE_IMPACT = <specific impact>
```

No implementation agent may self-authorize that expansion.

Independent gate inherited from §4.2: until the exact-projection-payload plan and its Product Boundary amendment exist, implementation of the projection write path is BLOCKED; the remaining Contracts carry exact frozen inputs already.

## 14. PR #9 disposition

```text
PR_9_DISPOSITION = SUPERSEDED_BY_FLEET_LOCAL_CHILD
```

PR #9 (single Build in Public pair Child) is superseded by this fleet-local Child. This task and this Spec must not close, modify, or merge PR #9; PR #9 remains OPEN at Head `3056263c3fc964a2b225720dd2b859b47e296c2e` until its Owner disposes of it.

## 15. Final frozen fields

```text
TASK_NAME = 全迁 执行

SPEC_ID = SVC_WORKFLOW_TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1
SPEC_FILE = docs/specs/SVC_WORKFLOW_TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1.md
BASE_HEAD = f4bfbb7cbc1dbcdb29c1caa472408adc41378fbf
AMENDED_PROPOSAL_HEAD = 9ac2ac79b36fec52b9d81706c66a9bc9f2337a07
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

APPLY_SCOPE_MODEL = CLOSED
PROJECTION_PAYLOAD_MODEL = CLOSED (DISPLAY_NAME_AUTHORITY_SOURCE = NONE_AVAILABLE_IN_FROZEN_AUTHORITIES; STOP; new exact-payload plan + Product Boundary amendment required before any projection write)
EVENT_RECEIPT_AUDIT_MODEL = CLOSED
EVENT_TYPE = TRUSTED_FLEET_PRINCIPAL_CUTOVER_COMMITTED
EVENT_SCHEMA_VERSION = v1
OUTCOME_UNKNOWN_MODEL = CLOSED
CANARY_CHECK_COUNT = 6
ADMIN_RECOVERY_REPLAY_COMPATIBILITY = FROZEN

HISTORY_REWRITE_ALLOWED = NO
GENERAL_MIGRATION_CAPABILITY = NO
RUNTIME_OLD_NEW_PARAMETERS = FORBIDDEN
TRANSACTION_MODEL = PER_PAIR_POSTGRESQL_SERIALIZABLE
NOOP_MODEL = PER_PAIR_EXACT_RECEIPT_EVENT_AUDIT_CHAIN_AND_POSTSTATE_MATCH

OLD_IMPLEMENTATION_FILE_COUNT = 3
IMPLEMENTATION_FILES = 4

STATUS = proposed
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
IMPLEMENTATION_PERFORMED = NO
PRODUCTION_PLAN_EXECUTED = NO
PRODUCTION_APPLY_AUTHORIZED_NOW = NO
MERGE_REQUIRED_FOR_ACTIVATION = YES
PRODUCTION_CHANGE = NONE
```

End of proposed amended Spec. This document authorizes nothing by itself; implementation authority activates only through independent review, Owner acceptance, and merge of this exact head to `main`, and production apply remains a separate later gate.
