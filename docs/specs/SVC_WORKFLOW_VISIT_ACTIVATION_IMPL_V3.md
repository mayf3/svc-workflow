---
spec_id: SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3
title: Accepted whole-authority successor to SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V2
status: accepted
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
repo: mayf3/svc-workflow
scope:
  - visit-activation
  - closed-local-producer-authoring
date: 2026-10-29
revision: proposal-1
candidate_base: 6c5a46433b8ec98b2fe708f44e270fd663cec7af
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V13
  - SVC_WORKFLOW_OWNER_OBLIGATION_WORKFLOWS_V3
  - SVC_WORKFLOW_DEFINITION_GRAPH_DIAGNOSTICS_V1
architecture_authority: SVC_WORKFLOW_ARCHITECTURE_V0_4_7
paired_authority: AGENT_CORE_WORKFLOW_OWNER_OBLIGATION_WORKFLOWS_V2
external_authorities:
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_WORKFLOW_OWNER_OBLIGATION_WORKFLOWS_V2
    revision: 60dea2c350ff2342411d896ca53bd6cf77d884e9
    relation: interoperates_with
supersedes:
  - SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V2
superseded_by: null
owners:
  - mayf3
accepted_proposal_head: c02916db775fb7e7d1e74cb77dd3df7079a7e54c
accepted_proposal_spec_sha256: 2c7543f802389ccb805dab398032b013d5b48bb81ca1839e7fc53b4e723a2b74
acceptance_record: ART/node-reference-authoring-spec-20261029/ROOT_OWNER_ACCEPTANCE.json
acceptance_record_sha256: b09a9ffe268691bee7f2d9b68107b17dfc3f46bbb4d2f1977227790e97d2656b
independent_review_record: ART/node-reference-authoring-spec-20261029/ROOT_INDEPENDENT_SPEC_REVIEW.json
independent_review_record_sha256: 6772ba5bd2a130c968b2298bc19b4ec6a987c38f0222651c1f570b13a75b641a
acceptance_delta_class: lifecycle_provenance_only
semantic_delta_from_reviewed_head: none
---

# SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3 — accepted whole-authority successor

**Accepted whole-authority successor; implementation effective after merge.** Root acceptance under the user's delegated authority is recorded in `ART/node-reference-authoring-spec-20261029/ROOT_OWNER_ACCEPTANCE.json` (SHA-256 `b09a9ffe268691bee7f2d9b68107b17dfc3f46bbb4d2f1977227790e97d2656b`), following the independent review (SHA-256 `6772ba5bd2a130c968b2298bc19b4ec6a987c38f0222651c1f570b13a75b641a`) of exact semantic head `c02916db775fb7e7d1e74cb77dd3df7079a7e54c`. This lifecycle-only transaction accepts both Svc V3 successors and records the direct V2 reciprocal supersession links and discovery indexes without changing reviewed semantics. `implementation_authority: contracts` authorizes only a later scoped implementation after this accepted lifecycle is merged into the authority branch and present in its implementation base. `production_apply_authority: none`. No source, database, service, deployment, run or credentials changed; no implementation or runtime conformance is claimed. The carried predecessor bodies and historical acceptance statements remain unchanged.

## V3 bounded decision and relationship

This whole successor retains every VisitActivationImpl V2 contract and acceptance requirement. The sole delta clarifies and closes Definition authoring for the two producer declarations added by its bounded delta: `workflowPrerequisiteGate.producer` and `workflowOperation.prerequisiteProducer`. Canonical stored, published, read and runtime objects still require exactly `{nodeId,slotKey}`. The existing full draft PUT accepts only the bounded local authoring union and normalization specified by Owner Workflows V3 CTR-ONR-001–006, at the same accepted lifecycle. All older carried authority/acceptance text is historical provenance; it does not make this proposal accepted.

The Product V13/Architecture V0.4.7 subjects and semantics do not change: runtime work identity remains `nodeVisitId`, canonical Definition node identity remains UUID, published graph immutability and activation transactions remain unchanged. Core Owner V2 at `60dea2c350ff2342411d896ca53bd6cf77d884e9` remains compatible; no raw authoring metadata reaches Core. No Core successor, new runtime wire, database relation or migration is authorized.

## V3 Contracts

### CTR-VAI-NR-001 — Authoring normalization before canonical graph use

Apply Owner Workflows V3 CTR-ONR-001–005 as the exact authoring contract. A complete draft PUT may name a producer by closed `{nodeKey,slotKey}` only in the two named metadata positions. All submitted node keys must be unambiguous for resolution; allocate server node UUIDs, resolve both references using the same submitted graph, and preserve the canonical output union as exactly `{nodeId,slotKey}`. Same-Version canonical read-edit resolves its old UUID only through the current locked DRAFT preimage's key and the submitted new map. Foreign/stale IDs, removed targets, duplicates and invalid producer shapes/semantics fail closed. Cross-Version copy explicitly converts a real source readback to local keys before the ordinary target PUT; there is no clone endpoint or cross-Version server lookup.

Within the existing Version serialization and graph replacement boundary, use the actual locked preimage and effective schema, validate the normalized graph with the existing semantic-model/schema and owner producer rules before commit, and persist no temporary invalid graph. Preserve prior graph/schema on failure. Draft success is not publish approval; actual publish revalidates the canonical graph and retains existing Owner, target/digest, schema and lifecycle checks. No follow-up metadata PUT, per-node mutation, direct SQL construction or caller-controlled UUID can satisfy this authoring contract.

Canonical graph validation CTR-VAI-011 and the V2 managed producer/operation rules remain authoritative after normalization. A producer must be earlier, ADVANCE-reachable and a real ordinary-create slot in this graph; role, slot and gate placement restrictions remain. This changes neither accepted Context materialization nor runtime producer proof: actual effects/receipts still establish supplier lineage. Closed authoring syntax adds no Model3 graph node/transition type, required-work catalog, selector, generic execution step or new runtime semantics. Preserve metadata outside the two rewritten identity members under JCS equality and retain existing DTO/schema patch behavior.

### CTR-VAI-NR-002 — Runtime isolation, receipt compatibility and lifecycle

Persisted graph/read/publish/runtime boundaries never accept an unresolved `nodeKey` producer. Runtime validators and consumers do not infer identity from key/name/metadata; activation, Dispatch Intent, wake, attempt, replay and RETURN identities remain those in the full predecessor contracts. Core V2 only receives existing canonical coordinates; Svc owns authoring resolution. Ordinary graphs and semantic models 1/2 keep their existing behavior.

Use the existing Direct-token, scope, Domain Owner, enabled actor/Domain, visibility, lifecycle and opaque error rules. Normalization errors join the existing graph-validation failure/receipt path, HTTP 422 `graph_validation_failed` and current safe diagnostic mapping; unlisted owner internal codes map to `GRAPH_VALIDATION_REJECTED`. No public code or diagnostic disclosure is added. Keep the historical version-only replacement receipt request hash and existing successful same-key replay behavior. Keep full parsed pre-normalization graph input hash only in its existing graph-rejection role, preserving failed same-input replay and changed-input conflict. Generated IDs never become request identity. Known rejection leaves zero graph/schema/business success facts; failure receipts/audit retain their existing meaning, and uncertain commit remains UNKNOWN. CTR-ONR-005 supplies the exact distinction; this successor does not expand idempotency or redesign transactions.

This V3 and Owner Workflows V3 are whole successors, accepted together with reciprocal direct-predecessor links and local index changes after independent review. No predecessor normative body is silently edited. Implementation begins only after accepted authority merges into its relevant base. No production apply, activation or unattended business conformance follows from authoring/source/HTTP test success. Stable VAI contract IDs, all unchanged V2 bounds and acceptance obligations remain carried below.

## V3 Acceptance

### ACC-VAI-NR-001 — Actual author/read/replace/copy/publish path

- Contracts: CTR-VAI-NR-001 and CTR-VAI-NR-002; CTR-ONR-001–005.
- Method: execute Owner V3 ACC-ONR-001–005 against the actual HTTP service and retained synthetic PostgreSQL; include both metadata paths, one-PUT initial authoring, same-DRAFT canonical read-edit, explicit cross-Version key-copy, negatives and concurrent replace/publish. This is shared evidence for the same affected contract, not a second duplicate suite.
- Environment: exact candidate binary/source/authority, isolated authorized test Domain/Owner/database; no production or old-run changes.
- Required evidence: real sanitized request/response, fresh UUID/readback/digest maps, actual validation/publication and diagnostic receipts, atomic pre/post facts and original-input hash comparisons.
- Expected result: every persisted/published producer is canonical and valid, runtime behavior unchanged, rollback and historical receipt semantics preserved, all Owner V3 cases pass.
- Failure condition: only direct SQL or mocks exercise authoring, invalid/stale reference persists, opaque metadata changes, public diagnostics/hash behavior expands, or a follow-up PUT is needed to attach future UUIDs.

### ACC-VAI-NR-002 — Canonical runtime and whole-authority compatibility

- Contracts: CTR-VAI-NR-001 and CTR-VAI-NR-002; CTR-ONR-004 and CTR-ONR-006.
- Method: carry-check the complete V2 body byte-for-byte, inspect the lifecycle pair and unchanged parents/Core consumer, and execute affected existing canonical graph/producer/readiness tests plus unmanaged and semantic-model controls. Trace a real HTTP-published graph to the existing canonical runtime decoder; reject unresolved authoring reference in that decoder.
- Environment: exact docs/source heads and authorized test environment; shared Owner V3 ACC-ONR-006 evidence is permitted.
- Required evidence: source/carry hashes, final acceptance/merge coordinates, canonical published graph, focused executed checks, zero source changes in this docs proposal.
- Expected result: two complete Svc successors only; NodeVisit runtime identity, closed UUID producers, activation/RETURN/STOP/effect semantics and Core V2 compatibility retained.
- Failure condition: partial carry, new runtime key fallback, unaccepted implementation, unrelated parent/Core change, or static tests falsely claimed as actual runtime/unattended success.

## V3 migration, rollback and retained authority

There is no storage migration or backfill. This authoring input form is normalized once, so rollback does not leave a second runtime representation. Existing canonical graphs/history remain unchanged; complete draft replacement retains its existing fresh-ID behavior. Product V13, Architecture V0.4.7, graph diagnostics V1 and Core Owner V2 are retained. No clone API, per-node patch, authoring DSL or generic graph migration is included. Expansion requires re-PREFLIGHT.

## Complete predecessor body carried forward verbatim

Direct predecessor: `docs/specs/SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V2.md` at `6c5a46433b8ec98b2fe708f44e270fd663cec7af`. Full file SHA-256: `21070c61efa9eca1b44558b67c5b078dd3f5c1df2f386ad5a8fa185e5f5228c3`. Body bytes after YAML frontmatter SHA-256: `6e39ebec75b292e2fa4833e09601d335aa9cf05805c3e141891983bcd8731dd9`. The following body is carried byte-for-byte; only the V3 bounded delta above changes current authority. Its historical lifecycle statements do not confer V3 acceptance.

<!-- BEGIN COMPLETE V2 PREDECESSOR BODY -->

# SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V2 — accepted whole-authority successor

Accepted whole-authority successor. Root acceptance under the user-delegated authority, recorded in `ROOT_OWNER_ACCEPTANCE.json` (SHA-256 `43eef302fb86cf131106fd6d39c0e338abc0acf9f14fb0210a5e51918d91448e`), binds the independently reviewed semantic heads Svc `b115c0641e4c2b6b965a6a3cd24252734754c14f` and Core `0629858c08029ebcaed39b012b4c4a2a2a8078fb`. This lifecycle-only transaction records acceptance and reciprocal predecessor links. Accepted governing Specs with `implementation_authority: contracts` authorize later source implementation within their Contracts once present in its base; Product, Architecture, and Storage authority retain `implementation_authority: none`. Production apply, migration execution, deployment, and A75 mutation remain unauthorized. It carries forward the predecessor contracts with the explicitly stated bounded additions below; the predecessor source remains unchanged.

## Bounded contract delta

Add CLOSED `node.metadata.workflowOperation` with version 1, immutable per-node `slotKey`, closed operation descriptors, `requiredForAdvance=true`, and optional earlier producer reference `{nodeId,slotKey}`. Ordinary-create descriptors pin exact target Version and digest; managed INTAKE/REVIEW roles are not ordinary create targets. Add CLOSED `node.metadata.workflowPrerequisiteGate={version:1,kind:TERMINAL_SUBMISSION,producer:{nodeId,slotKey}}` for managed intake TASKs only. Svc derives effect identity; all required actions join the actual business command receipt/result before ADVANCE or terminal acceptance. Before ADVANCE or RETURN enters a declared gated TASK, the same atomic effective-Context-plus-transition transaction requires and verifies the exact real supplier binding and the earlier successful producer slot; missing/mismatched binding fails before closing the prior visit or committing target visit/Context success facts. A valid real supplier may be WAITING at entry, in which case the new visit has zero dispatch until its live prerequisite is READY. Preserve the existing create-time root prerequisite schema. Owner replacement does not move a visit; it stops/fences and records the actual unsupported disposition, including the existing lifetime-unique INTAKE intent when its managed ID is still absent. The existing WAE attempt limit is three per Visit and the existing Svc RETURN edge limit is three; full managed ADVANCE graph is acyclic. Paired Spec §§4–6,8–9 supplies exact graph, receipt and bounded STOP behavior.

Paired authority reference: `SVC_WORKFLOW_OWNER_OBLIGATION_WORKFLOWS_V1 §§4–6,8–9,13–15; carries all VAI V1 contracts`.
The predecessor contracts are carried forward below with only the bounded successor changes stated above; the predecessor source remains unchanged. This successor is accepted in the same paired lifecycle transaction that records the reciprocal predecessor backlink. Acceptance is not implementation or conformance evidence.

Predecessor source: `docs/specs/SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1.md` at `f5e7f328ccba5f6e0d752abf74e68558ef38ca0f`; original file SHA-256 `6ecd3fe458b09096a40501290a29c216015b2073ade61911384ebe27eb1c9af5`.

## Complete predecessor body carried forward verbatim

# SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1 — Visit-Activation Runtime Core (Slice D phase 1)

```text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1
STATUS = accepted
BASE_REPOSITORY = mayf3/svc-workflow
BASE_COMMIT = b5bb7eecdb9bfdf41b96e470df9c845c538edcad (accepted SVC_WORKFLOW_ARCHITECTURE_V0_4_0 merge)
CHANGE_CLASS = NON_MECHANICAL
PREFLIGHT_MODE = IMPLEMENT (v0.4.0 §10.4 sequencing satisfied)
AUTHORITY_HANDLING = governed_by accepted V6 + accepted v0.4.0
PRODUCT_CODE_CHANGE = THIS SPEC AUTHORIZES IT
PRODUCTION_CHANGE = NONE
PRODUCTION_APPLY_AUTHORITY = none
ACCEPTANCE_BASIS = Owner ruling KEEP_ACCEPTED_V6 (2026-09-02) delegates the
  autonomous authority->audit->acceptance->implementation->audit chain;
  independent spec review performed at authoring head before acceptance.
```

> Implementation-authorizing child of accepted `SVC_WORKFLOW_ARCHITECTURE_V0_4_0`
> (Slice D, per V6 §18). It freezes the phase-1 runtime core for the
> VISIT_ACTIVATION_V1 semantic model. Production apply, cutover barrier
> selection, and Legacy migration remain separately gated and are NOT
> authorized here. `IMPLEMENTATION_READY` becomes YES for the scoped surface
> below only.

## 1. Goal

Implement the minimum complete, tested runtime core that makes the accepted
v0.4.0 activation model real for the `VISIT_ACTIVATION_V1` semantic model:

1. explicit immutable semantic-model identity `VISIT_ACTIVATION_V1` on
   Definition Versions and Instances with DB-enforced equality;
2. new-model graph validation (`TASK | TERMINAL`);
3. canonical activation facts (`HUMAN_WORK_ITEM | DISPATCH_INTENT`) created
   and closed atomically with Visit entry/closure;
4. server-authored initial `nextEligibleAt` in the activation transaction;
5. authorized wake (early wake to server-now) and the singular due read for
   the Scheduler;
6. strict Legacy protection: no Legacy instance ever gains an activation; no
   new-model instance takes Legacy-only semantics.

## 2. Exact coordinates and inputs

```text
GOVERNING_PRODUCT_DIRECTION = SVC_WORKFLOW_PRODUCT_BOUNDARY_V6 (accepted)
GOVERNING_ARCHITECTURE = SVC_WORKFLOW_ARCHITECTURE_V0_4_0 (accepted, merged b5bb7eec)
BASE_COMMIT = b5bb7eecdb9bfdf41b96e470df9c845c538edcad
PRIOR_ARCHITECTURE = SVC_WORKFLOW_ARCHITECTURE_V0_3_1 (superseded, historical)
RETAINED_REFINEMENT = SVC_WORKFLOW_ARCHITECTURE_V0_3_2 (unchanged)
EXPECTED_MIGRATION_VERSION_BEFORE = 22
EXPECTED_MIGRATION_VERSION_AFTER = 23
```

## 3. Storage contracts (migration `0023_visit_activation_v1.sql`)

### CTR-VAI-001 — Semantic-model identity

`semantic_model_version` gains value `3` = `VISIT_ACTIVATION_V1`:

- `workflow_definition_versions` check widened to `(1, 2, 3)`; existing rows
  and values `1`/`2` keep their exact meaning (CTR-ARCH-003: `2` Minimal is
  NOT aliased to `3`);
- `workflow_instances` gains `semantic_model_version SMALLINT NOT NULL`
  backfilled from its immutable Definition Version;
- equality is DB-enforced: `UNIQUE (definition_version_id,
  semantic_model_version)` on versions + composite FOREIGN KEY from
  `workflow_instances (definition_version_id, semantic_model_version)`;
- the Rust runtime rejects unknown values fail-closed (existing `other` arm).

### CTR-VAI-002 — Node model encoding

`node_type` enum gains `'TASK'`. `VISIT_ACTIVATION_V1` graphs use exactly
`TASK | TERMINAL`; the graph validator (CTR-VAI-004) rejects `DRAFT`/`NORMAL`
and `INSTANCE_INPUT_PRINCIPAL` owner refs in new-model graphs. Legacy rows and
validators are unchanged. TERMINAL nodes carry no resolved owner (visits are
never created for TERMINAL).

### CTR-VAI-003 — Immutable activation fact tables

~~~sql
activation_kind := ENUM('HUMAN_WORK_ITEM','DISPATCH_INTENT')

workflow_activations (
  activation_id UUID PK,
  workflow_instance_id FK NOT NULL,
  node_visit_id  FK NOT NULL UNIQUE,          -- exactly-one, DB-enforced
  activation_kind activation_kind NOT NULL,
  owner_principal_id FK (principals) NOT NULL,
  activation_at TIMESTAMPTZ NOT NULL,         -- server-authored, same tx
  initial_next_eligible_at TIMESTAMPTZ,       -- NOT NULL iff DISPATCH_INTENT
  command_id UUID NOT NULL,                   -- owning command receipt
  CHECK (activation_kind = 'DISPATCH_INTENT' ==> initial_next_eligible_at IS NOT NULL),
  CHECK (activation_kind = 'HUMAN_WORK_ITEM' ==> initial_next_eligible_at IS NULL)
)

workflow_activation_closures (
  activation_id UUID PK FK(workflow_activations),
  closed_at TIMESTAMPTZ NOT NULL,
  closure_reason TEXT NOT NULL (1..128),
  command_id UUID NOT NULL,
  event_id UUID NULL                          -- set when the closure command
                                              )   -- writes its Event
)

workflow_dispatch_eligibility_events (
  eligibility_event_id UUID PK,
  activation_id FK NOT NULL,                  -- DISPATCH_INTENT activations only
  previous_next_eligible_at TIMESTAMPTZ NOT NULL,
  new_next_eligible_at TIMESTAMPTZ NOT NULL,
  cause_class TEXT NOT NULL (1..64),          -- e.g. WAKE | SCHEDULER_DEFER
  command_id UUID NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
)
~~~

All three tables are append-only immutable fact families: UPDATE and DELETE
are rejected by database triggers (same enforcement style as the existing
immutability triggers). `workflow_activations.node_visit_id` UNIQUE is the
mechanical exactly-one invariant (one activation fact may ever exist per
Visit; repair of a provably missing activation is out of scope for phase 1,
see §8). Active/closed and current `nextEligibleAt` are derived:

```text
active(activation)         = no workflow_activation_closures row exists
current nextEligibleAt(a)  = value of the latest
                             workflow_dispatch_eligibility_events row by
                             (created_at, eligibility_event_id), else
                             initial_next_eligible_at
```

A partial index on `workflow_activations (activation_kind) WHERE
activation_kind = 'DISPATCH_INTENT'` plus the closure PK supports the bounded
due poll. The activation tables are the durable delivery-obligation surface
of v0.4.0 §5.11 (dedicated-table poll); no Workflow Instance/global-summary
scan is used.

### CTR-VAI-004 — Create atomic closure

For `VISIT_ACTIVATION_V1`, `create_workflow_instance_atomically` extends its
existing single transaction with: entry `TASK` Visit (resolved owner
snapshot) and one `workflow_activations` row whose `kind` is derived only
from the resolved canonical Principal type (`HUMAN -> HUMAN_WORK_ITEM`,
`AGENT -> DISPATCH_INTENT`), whose `activation_at` is the server transaction
timestamp, and whose `initial_next_eligible_at = activation_at` iff kind is
`DISPATCH_INTENT`. The INSTANCE_CREATED Event `event_data` records
`activationId`/`activationKind`/`initialNextEligibleAt`. Owner resolution
failure (missing/disabled/SERVICE/not HUMAN-or-AGENT) commits nothing with a
deterministic `owner_resolution_failed` class. No caller field can supply the
owner or timestamp.

### CTR-VAI-005 — Transition atomic closure

A successful Transition on a `VISIT_ACTIVATION_V1` Instance closes the source
Visit activation (one `workflow_activation_closures` row, reason
`TRANSITIONED`) and creates the target Visit + its activation in the same
transaction; TERMINAL targets create no target activation. Existing
version/Event/Receipt/audit invariants are unchanged (one increment, one
Event).

### CTR-VAI-006 — Cancel and Archive closure semantics

Cancel of a `VISIT_ACTIVATION_V1` Instance atomically closes the current
activation (reason `CANCELLED`) while preserving v0.3.2 Visit semantics.
Archive of a `VISIT_ACTIVATION_V1` Instance with an active activation fails
closed (deterministic `active_activation_exists` class).

### CTR-VAI-007 — Administrative move and terminate closure semantics

Admin Recovery `MOVE_TO_NODE` and `TERMINATE_INSTANCE` on a
`VISIT_ACTIVATION_V1` Instance atomically close the source activation
(reasons `ADMIN_MOVE` / `ADMIN_TERMINATE`) and create the correct target
TASK activation / no terminal activation, per CTR-ARCH-019. Legacy behavior
is unchanged.

### CTR-VAI-008 — Wake command

New command `WAKE_DISPATCH_INTENT`:

```text
POST /internal/v1/workflow-instances/{workflowInstanceId}/node-visits/{nodeVisitId}/wake
scope workflow.execute + direct token + enabled GLOBAL_SCHEDULER_READ binding
request: { expectedWorkflowStateVersion, idempotencyKey, cause? }
```

- binds the exact Instance + nodeVisitId; resolves the unique activation;
- applied only when the activation exists, is `DISPATCH_INTENT`, and is
  active: appends one `workflow_dispatch_eligibility_events` row
  (`previous = current`, `new = server now`, `cause_class = WAKE`),
  increments `workflowStateVersion` once, writes exactly one
  `WAKE_DISPATCH_INTENT` Event, one Receipt, and audit — atomically;
- stale/closed/current-mismatch/Version-mismatch wake commits a durable
  receipt + attempt audit and returns `200 {wakeApplied: false, reason}`
  with NO version increment, NO Event, NO fact row;
- idempotency: same principal+key+request replays the original outcome;
  different request conflicts (existing receipt machinery);
- it never mutates node/owner, creates an activation, performs a Transition,
  or starts an Agent.

### CTR-VAI-009 — Due Dispatch Intent read

```text
GET /internal/v1/dispatch-intents?limit=N
scope workflow.read + direct token + enabled GLOBAL_SCHEDULER_READ binding
```

Returns at most `N` (1..100) active DISPATCH_INTENT activations with
`current nextEligibleAt <= authoritative now`, ordered by `(current
nextEligibleAt, activation_id)`. Each record contains exactly:

```text
dispatchIntentId, nodeVisitId, workflowInstanceId,
ownerPrincipalId, nextEligibleAt, createdAt, updatedAt
```

(`createdAt` = activation_at; `updatedAt` = latest eligibility change time or
activation_at). It excludes Context/title/labels/keys/Submission/EventData/
Assistance/credentials/audit payloads/Transition options/metadata. The role
gate is the fail-closed server-side mapping of the V6
`GLOBAL_SCHEDULER_READ` product capability: an enabled
`GLOBAL_SCHEDULER_READ` binding satisfies it; nothing else does; no existing
Reader/Coordinator binding is automatically mapped and no Grant is created by
this Spec. Missing gate = 403 `scheduler_read_role_required`.

### CTR-VAI-010 — GLOBAL_SCHEDULER_READ role value

`GLOBAL_SCHEDULER_READ` becomes an accepted `global_role_bindings.role_key`
value in the admin provisioning API validation set. No binding is created by
migration or code; provisioning remains via the existing admin API under its
existing authority (production supply stays with auth-service Slice B/C).
Fail-closed when absent.

### CTR-VAI-011 — New-model graph validation

`validate_definition_graph` dispatches on semantic model: model `3` runs the
new `VISIT_ACTIVATION_V1` validator: node kinds exactly `TASK | TERMINAL`;
exactly one entry TASK (no incoming primary ADVANCE); every TASK has exactly
one primary ADVANCE; primary ADVANCE edges form one acyclic deterministic
path ending at a TERMINAL; RETURN edges target a strictly earlier reachable
TASK; TERMINATE edges target a TERMINAL; TERMINAL has no outgoing edge; all
nodes reachable from the entry TASK; `DRAFT`/`NORMAL` node kinds and
`INSTANCE_INPUT_PRINCIPAL` refs are rejected. Draft-time and publish-time
validation both run it. Model `1`/`2` paths are byte-for-meaning unchanged.

### CTR-VAI-012 — Legacy protection (mutual fail-closed)

- LEGACY (`1`) and Minimal (`2`) Instances never read or write activation
  tables (their command paths are unchanged);
- `VISIT_ACTIVATION_V1` Instances reject Legacy-only commands
  (`revise`, `revise-and-transition`) with deterministic 422
  `legacy_command_not_supported_for_semantic_model`;
- `REBUILD_PROJECTION` on a `VISIT_ACTIVATION_V1` Instance additionally
  validates the activation fact families for that Instance (exactly-one
  activation per Visit, closures follow creation, eligibility rows reference
  DISPATCH_INTENT activations with sane previous/new ordering) and fails
  closed on drift without mutating facts;
- unknown semantic model values fail closed everywhere they are dispatched.

### CTR-VAI-013 — Compatibility

Existing Domain/global lists, worklists, details, timeline, Submission
history, HTTP envelope, error codes, pagination, and the Global Reader/
Coordinator gates are byte-for-meaning unchanged. No `dispatchable` field,
`dispatch_blocked_reasons`, or `dispatchableOnly` parameter is added to any
existing surface (Owner ruling KEEP_ACCEPTED_V6). Additive wire surface is
exactly CTR-VAI-008/009. `EXPECTED_MIGRATION_VERSION` becomes 23.

### CTR-VAI-014 — Audit and idempotency

Wake and due-read are protected operations: successful and
authenticated-denied attempts write non-sensitive audit rows (existing
attempt-audit/security-audit machinery). Wake keeps existing receipt
idempotency semantics (same key+request replays; changed request conflicts).
The due read performs its role check inside the same read snapshot as the
query.

## 4. Explicit out of scope (FOLLOW_UP_DEBT, not blocking this Spec)

| Item | Authority home |
|---|---|
| ONE_TIME_MIGRATE, DRAIN/HISTORICAL_REPLAY enforcement, cutover barrier application | later cutover/migration Spec (CTR-ARCH-028..031) |
| missing-activation repair command | later repair Spec (CTR-ARCH-027) |
| delivery obligation acknowledgement/transport protocol | later delivery Spec + external Scheduler authority (Slice F) |
| `GLOBAL_SCHEDULER_READ` grant supply to any Principal | auth-service Slice B/C + V6 designation authority |
| 365-day audit retention enforcement | separate audit-authority round |
| SCHEDULER_DEFER (future-dated eligibility) command | first external Scheduler-management authority (attempt-bound) |

Where a deferred item could otherwise be reached, the implementation fails
closed (e.g., no SCHEDULER_DEFER command exists in phase 1; wake is the only
eligibility writer).

## 5. Acceptance

Each ACC names its owning contract; executable evidence is collected by the
implementation candidate test suite (`tests/28_visit_activation_v1.rs` +
validator unit tests) against the run-scoped test database. An ACC is not
production authorization.

- **ACC-VAI-001 (CTR-VAI-001)** — Migration 0023 applies; versions accept 3;
  instance/model equality enforced (composite FK rejects mismatch);
  Minimal(2) rows unchanged. Environment: run-scoped test DB.
  Required evidence: migration run + mismatch-insert rejection + Minimal regression.
- **ACC-VAI-002 (CTR-VAI-004)** — Create with AGENT-owned entry TASK yields
  instance+visit+activation(`DISPATCH_INTENT`, initial nextEligibleAt =
  activation_at) in one commit; HUMAN owner yields `HUMAN_WORK_ITEM` with
  NULL initial eligibility; SERVICE/disabled/missing owner fails with zero
  facts. Required evidence: row traces + rollback traces.
- **ACC-VAI-003 (CTR-VAI-005)** — Transition closes source activation,
  creates target activation (or none for TERMINAL), exactly one version/Event.
  Required evidence: before/after activation state + Event count.
- **ACC-VAI-004 (CTR-VAI-006/007)** — Cancel closes activation; Archive with
  active activation fails; admin MOVE/TERMINATE handle activation correctly.
  Required evidence: traces per command.
- **ACC-VAI-005 (CTR-VAI-008)** — Wake on active due intent applies
  eligibility fact + one Event + one version + receipt; stale/closed wake is
  a no-op with receipt/audit; unauthorized caller gets 403 and an audit row;
  same key+request replays. Required evidence: response bodies + row counts.
- **ACC-VAI-006 (CTR-VAI-009/010)** — Due endpoint returns only active due
  intents with exactly the 7 minimum fields; role gate fail-closed
  (`scheduler_read_role_required`) for role-less callers, including
  GLOBAL_WORKFLOW_READER holders. Required evidence: schema-key snapshot +
  negative-role matrix.
- **ACC-VAI-007 (CTR-VAI-011)** — Validator accepts a conformant
  TASK/TERMINAL graph and rejects each malformed shape (multi-entry, missing
  primary, cycle, RETURN forward/into TERMINAL, TERMINATE into TASK, DRAFT/
  NORMAL nodes, INSTANCE_INPUT_PRINCIPAL ref, unreachable node). Required
  evidence: positive + negative fixtures.
- **ACC-VAI-008 (CTR-VAI-012/013)** — Legacy instance lifecycle produces zero
  activation rows; new-model revise returns
  `legacy_command_not_supported_for_semantic_model`; global list response
  shape is byte-identical pre/post. Required evidence: row counts + wire diff.
- **ACC-VAI-009 (CTR-VAI-014)** — Wake attempts (success, denial, no-op) all
  leave audit rows; sensitive payloads absent. Required evidence: audit rows.
- **ACC-VAI-010 (CTR-VAI-003)** — Direct UPDATE/DELETE on the three fact
  tables is rejected by triggers. Required evidence: rejected-statement traces.
- **ACC-VAI-011 (CTR-VAI-012)** — REBUILD_PROJECTION on a new-model Instance
  validates activation cardinality/consistency and fails closed on injected
  drift; Legacy instance rebuild behavior is unchanged. Required evidence:
  rebuild traces for clean and drifted fixtures.

## 6. Test gate

The implementation candidate ships with: `cargo test --lib` green,
`cargo test --test 28_visit_activation_v1` green, full workspace
`cargo test --workspace` green against the run-scoped test database
(known pre-existing failures outside scope remain as documented baseline).
`git diff --check` clean.

<!-- END COMPLETE V2 PREDECESSOR BODY -->
