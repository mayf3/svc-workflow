---
spec_id: SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
scope:
  - svc-workflow DomainInstanceSummary dispatchability projection
  - GET /internal/v1/workflow-instances/domain
  - GET /internal/v1/workflow-instances/global
  - existing keyset pagination for those lists
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1

```text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1
STATUS = proposed
BASE_REPOSITORY = mayf3/svc-workflow
BASE_COMMIT = c90d54cace46ff505ac54aa6215587d812cf9a78
SOURCE_CANDIDATE_COMMIT = af450aa39e446683b8ae2b2edf99c4febdcfb068
AUTHORING_OBSERVED_AT = 2026-08-31T22:27:34Z
CHANGE_CLASS = NON_MECHANICAL
PREFLIGHT_MODE = AMEND
AUTHORITY_HANDLING = REVISED_AGAINST_ACCEPTED_PRODUCT_BOUNDARY_V5
PRODUCT_CODE_CHANGE = NONE
PRODUCTION_CHANGE = NONE
```

> This is a docs-only proposed Spec. It does not authorize implementation or
> deployment while proposed. If independently reviewed, Owner-accepted, and
> merged to `main`, its Contracts become implementation authority; production
> apply remains separately gated.

## 1. Goal

Define one generic, Workflow-owned, read-only projection that answers, at the
query snapshot:

> Is this Workflow Instance objectively suitable to hand to its current
> assignee to begin ordinary processing?

The projection removes Workflow-state interpretation from HR, Scheduler, and
other callers. Workflow determines objective eligibility from existing
Workflow-owned facts. A caller remains responsible for which eligible assignee
to contact, fairness, deduplication, rate limits, retries, and message/session
delivery.

The proposed usage is:

```text
GET /internal/v1/workflow-instances/global
  ?lifecycle=active
  &dispatchableOnly=true
  &limit=20

follow next_cursor until null
```

## 2. Scope and non-goals

### 2.1 In scope

- Add `dispatchable: bool` and `dispatch_blocked_reasons: [...]` together to
  `DomainInstanceSummary` only for requests that explicitly provide
  `dispatchableOnly=true|false`.
- Add optional `dispatchableOnly=true|false` to the existing Domain and global
  Instance list queries.
- Derive the projection from authoritative Workflow facts in the same read
  snapshot; create no second mutable dispatch state.
- Preserve existing authorization, filters, ordering, cursor shape, defaults,
  page-size limits, and response envelope.
- Define V1 only from formal primitives that exist at the pinned base.

### 2.2 Explicit non-goals

This Spec does not add or define:

- an HR Dispatcher, Scheduler, cron job, wake mechanism, Agent session, or
  message transport;
- Principal UUID to AgentId mapping;
- notification history, duplicate-send suppression, fairness, round-robin,
  per-round quotas, priority, retry intervals, or backoff;
- claim/pull assignment, reassignment, lease, reservation, lock acquisition, or
  an execution token;
- `page`, `totalPages`, `totalCount`, offset pagination, or a count query;
- Workflow-specific business payload interpretation;
- a metadata string convention for operational locks;
- implementation, migration, deployment, or production mutation in this task.

## 3. Authority and dependencies

### 3.1 Local authorities

The pinned base declares the following precedence:

1. `SVC_WORKFLOW_PRODUCT_BOUNDARY_V5`, accepted Current Product Direction,
   `docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V5.md`, active through merge
   commit `c90d54cace46ff505ac54aa6215587d812cf9a78`;
2. frozen `SVC_WORKFLOW_ARCHITECTURE_V0_3_1`,
   `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md`;
3. effective Cancel/Archive refinement
   `SVC_WORKFLOW_CANCEL_ARCHIVE_GOVERNANCE_V0_3_2`,
   `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_2.md`;
4. accepted governing Specs and explicitly frozen legacy Contracts in their
   declared scopes;
5. code, tests, and migrations as descriptive current-state evidence.

Relevant accepted/legacy authorities include:

- `SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1` for the current read-only global-list
  role gate;
- `WORKFLOW_QUERY_CONTRACT_V0_1` for query/worklist and keyset pagination
  semantics;
- `WORKFLOW_TRANSITION_CONTRACT_V0_1` for ordinary transition gates;
- `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` for the pinned current
  assigned-worklist eligibility facts.

### 3.2 Owner direction represented by this proposal

The requested product split is:

```text
Workflow owns: objective dispatch/execution eligibility
HR owns:       who to contact in this scheduling round
```

This proposal expresses that direction as candidate Decisions and Contracts.
Chat direction is not active repository authority; acceptance must follow the
repository governance process.

### 3.3 Parent-authority reconciliation

Accepted `SVC_WORKFLOW_PRODUCT_BOUNDARY_V5` §§8/8A and
`CTR-V5-044` through `CTR-V5-050` lawfully supersede V4's conflicting global
field prohibition and authorize this child to freeze the exact projection
predicates and wire Contracts. V5 does not itself authorize implementation.

The current HTTP bundle remains `strict_backward_compatible`. V5 resolves that
constraint by requiring an explicit representation opt-in:

- omitted `dispatchableOnly`: legacy response population and structure; neither
  new field is emitted;
- explicit `dispatchableOnly=false`: both fields are emitted without filtering;
- explicit `dispatchableOnly=true`: both fields are emitted and only rows with
  an empty reason set are eligible, before order/limit/cursor selection.

This child retains the current Domain Owner gate and the exact global route
compatibility gate `GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR`,
without adding or changing any role, Grant, scope, identity, credential, or
allowlist. The broader Slice-D core-field/permission conformance debt remains
outside this bounded child and is not declared complete here.

```text
UNRESOLVED_AUTHORITY_CONFLICT = NONE
WIRE_AUTHORITY_RECONCILIATION = EXPLICIT_QUERY_OPT_IN_FROM_V5
PRODUCT_DIRECTION_SUCCESSOR = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5 accepted/current
READY_TO_MARK_ACCEPTED = YES_AFTER_INDEPENDENT_REVIEW
IMPLEMENTATION_ALLOWED = NO
```

No external repository authority is changed or required.

## 4. Current State

All source-state statements below are refreshed against
`mayf3/svc-workflow@c90d54cace46ff505ac54aa6215587d812cf9a78`.
The product-only V5 acceptance merge changes none of the cited source paths;
the original source observations were made at `f0c74ee` and remain applicable
after exact-diff verification from that base to the pinned current base.
No production runtime or production database was queried.

- `STATE-001` — The Domain and global list return `Page<DomainInstanceSummary>`
  with `items` and nullable `next_cursor`. They use descending
  `(created_at, workflow_instance_id)` keyset pagination, default limit 20,
  maximum 100. Basis: `OBS-001`, `OBS-002`, `EVD-001`.
- `STATE-002` — Current global and Domain filters are `definitionKey`,
  `lifecycle`, `currentNodeKey`, `assigneePrincipalId`, `status`, `limit`, and
  the paired `beforeCreatedAt` + `beforeId` cursor; Domain additionally requires
  `domainId`. Basis: `OBS-001`, `OBS-002`.
- `STATE-003` — `lifecycle=active` means only current Node type is not
  `TERMINAL`. It includes current `DRAFT` Nodes. When `lifecycle` is supplied
  and `status` is omitted, `status` defaults to `all`, so cancelled non-terminal
  Instances also remain eligible for that query. Conversely, when both filters
  are omitted, `status` defaults to `active` but lifecycle is unfiltered, so
  non-cancelled/non-archived Terminal Instances remain eligible. Basis:
  `OBS-002`, `OBS-003`, `EVD-002`.
- `STATE-004` — Current Node `DRAFT` is a formal node type. Context revision is
  creator-only, and the combined revise-and-advance command requires the caller
  to be both creator and current assignee. A separate creator-owned-drafts view
  exists, but the current assigned-to-me SQL does not itself exclude DRAFT.
  Basis: `OBS-004`, `OBS-005`, `EVD-003`.
- `STATE-005` — A new or updated non-terminal Visit must have an assignee; a
  Terminal Visit must not. `current_assignee_principal_id = null` is therefore
  expected for Terminal and is a defensive blocking fact for any projected
  non-terminal inconsistency. Basis: `OBS-006`, `EVD-004`.
- `STATE-006` — Cancelled and archived are formal Instance facts. Cancellation
  blocks ordinary and combined transition and hides the Instance from the
  default assigned worklist. Archive is allowed only after terminal or cancel
  and is non-destructive governance metadata. Basis: `OBS-007`, `EVD-005`.
- `STATE-007` — Ordinary transition is blocked when the actor Principal is
  disabled, the Definition Version is `REVOKED` or defensively `DRAFT`, the
  current Node is Terminal, the Instance is cancelled, or the current Visit has
  an open Assistance case. `PUBLISHED` and `DEPRECATED` are allowed. Basis:
  `OBS-008`, `OBS-009`, `EVD-006`.
- `STATE-008` — Formal Assistance state exists in
  `workflow_assistance_cases`; `OWNER_PENDING` and `HUMAN_REQUIRED` are the open
  statuses, and an open current-Visit case fail-closes ordinary and combined
  transition. This is not named or modeled as an ops-lock. Basis: `OBS-009`,
  `OBS-010`, `EVD-007`.
- `STATE-009` — No schema column, enum, metadata contract, Assistance status,
  execution lock, or source symbol named `ops-lock`, `ops_lock`, `OPS_LOCK`, or
  `dispatchable` exists at the pinned base. PostgreSQL row/advisory locks are
  transaction-concurrency mechanisms, not durable dispatch blockers. Basis:
  `OBS-011`, `EVD-008`.
- `STATE-010` — Domain disabled is a formal fact and current worklists exclude
  disabled Domains, but the ordinary transition transaction does not validate
  Domain enabled state. Definition archive is discovery/governance metadata and
  does not block existing Instance transition. Basis: `OBS-012`, `EVD-009`.

## 5. Observations

### OBS-001 — Current wire/query and Page types

- Subject: Domain/global list DTOs and Page response.
- Method: source inspection.
- Result: `DomainInstanceQuery` and `GlobalInstanceQuery` expose the filters
  listed in `STATE-002`; `Page` is `{ items, next_cursor }`.
- Provenance:
  `src/http/dto.rs:102-138`,
  `src/application/workflow_instance/query_types.rs:45-55,309-352`.

### OBS-002 — Current list SQL and pagination

- Subject: Domain/global repository queries.
- Method: source inspection.
- Result: lifecycle is a Node-type predicate; status is an Instance-fact
  predicate; all filters are combined with `AND`; ordering is
  `created_at DESC, workflow_instance_id DESC`; `limit + 1` creates
  `next_cursor`.
- Provenance:
  `src/store/postgres/workflow_instance_repository/query_domain_instances.rs:14-55,99-195`,
  `src/store/postgres/workflow_instance_repository/query_global_instances.rs:14-47,90-178`.

### OBS-003 — Active lifecycle includes DRAFT and can include cancelled

- Subject: HTTP filter behavior.
- Method: inspect executed integration-test definitions and handler defaults.
- Result: the active-lifecycle fixture expects DRAFT + NORMAL; handler defaults
  omitted status to `all` when lifecycle is present; a separate test expects a
  cancelled non-terminal Instance in `lifecycle=active` results.
- Provenance:
  `tests/17_workflow_runtime/http/domain_list.rs:630-665,1720-1770`,
  `src/http/handlers/instances.rs:116-150,180-210`.

### OBS-004 — DRAFT authority and creator-only Context

- Subject: current Node DRAFT semantics.
- Method: architecture/contract/source inspection.
- Result: `node_type` formally contains `DRAFT`; Context mutation requires
  current DRAFT and creator; combined revise-and-advance requires creator and
  current assignee and current DRAFT.
- Provenance:
  `migrations/0001_identity_domain.sql:21-25`,
  `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md:623-652`,
  `docs/contracts/WORKFLOW_TRANSITION_CONTRACT_V0_1.md:331-352`,
  `src/store/postgres/workflow_instance_repository/combined_transaction.rs:159-202`.

### OBS-005 — Two worklist views exist but overlap on DRAFT

- Subject: assigned-to-me and creator-owned-drafts.
- Method: source and Contract inspection.
- Result: creator-owned-drafts selects creator + current `DRAFT`; assigned-to-me
  selects current assignee + non-Terminal and therefore does not exclude DRAFT.
- Provenance:
  `src/store/postgres/workflow_instance_repository/query_worklists.rs:45-71,141-169`,
  `docs/contracts/WORKFLOW_QUERY_CONTRACT_V0_1.md:159-178`.

### OBS-006 — Visit assignee nullability invariant

- Subject: current Visit assignee.
- Method: migration inspection.
- Result: trigger rejects non-terminal Visits without assignee and Terminal
  Visits with assignee.
- Provenance: `migrations/0010_terminal_assignee_nullable.sql:34-80`.

### OBS-007 — Cancel/archive authority

- Subject: Instance cancellation and archive.
- Method: effective Architecture, migration, and transaction inspection.
- Result: `cancelled` and `archived_at` are formal columns; cancellation blocks
  further flow; archive is terminal/cancelled-only governance metadata.
- Provenance:
  `migrations/0015_add_instance_cancel_archive.sql:1-14`,
  `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_2.md:43-103`,
  `src/store/postgres/workflow_instance_repository/transition_transaction.rs:169-187`.

### OBS-008 — Definition and actor transition gates

- Subject: ordinary transition validation.
- Method: source and Contract inspection.
- Result: actor Principal must exist and be enabled; `REVOKED` and defensive
  `DRAFT` Definition Versions are blocked; `PUBLISHED` and `DEPRECATED` are
  allowed.
- Provenance:
  `src/store/postgres/workflow_instance_repository/transition_validation.rs:37-87`,
  `docs/contracts/WORKFLOW_TRANSITION_CONTRACT_V0_1.md:217-224`.

### OBS-009 — Open Assistance blocks transition

- Subject: current-Visit Assistance.
- Method: source inspection.
- Result: ordinary and combined transition both call `has_open_assistance` and
  fail with `AssistanceOpen` before transition writes.
- Provenance:
  `src/store/postgres/workflow_instance_repository/transition_transaction.rs:204-224`,
  `src/store/postgres/workflow_instance_repository/combined_transaction.rs:179-189`.

### OBS-010 — Assistance has formal open statuses

- Subject: Assistance persistence.
- Method: migration inspection.
- Result: formal statuses are `OWNER_PENDING`, `HUMAN_REQUIRED`, `RESOLVED`,
  `VOIDED`; only the first two are open, with at most one open case per Visit.
- Provenance: `migrations/0021_workflow_assistance_v1.sql:4-12,104-127`.

### OBS-011 — No formal ops-lock primitive

- Subject: pinned repository source, migrations, Contracts, and docs.
- Method: exact-token searches for `ops-lock`, `ops_lock`, `OPS_LOCK`,
  `operational lock`, and `dispatchable`, followed by inspection of lock-related
  matches.
- Result: no Workflow-owned durable ops-lock or dispatchability primitive was
  found. Lock matches are PostgreSQL transaction/advisory locks or unrelated
  lifecycle text.
- Provenance: repository search at the pinned base; representative concurrency
  lock provenance includes
  `src/store/postgres/workflow_instance_repository/transition_validation.rs:18-35`.

### OBS-012 — Domain/Definition state does not uniformly gate transition

- Subject: Domain enabled and Definition archive.
- Method: compare worklist, transition, Product Direction, and definition
  lifecycle paths.
- Result: assigned-to-me joins `domains.enabled = TRUE`; the ordinary transition
  path reads `domain_id` but does not validate Domain enabled. Product Direction
  describes Definition archive/discovery as non-destructive; Definition Version
  status, not Definition archive, gates ordinary transition.
- Provenance:
  `src/store/postgres/workflow_instance_repository/query_worklists.rs:45-71`,
  `src/store/postgres/workflow_instance_repository/transition_transaction.rs:139-224`,
  `docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V5.md:326-369`.

### OBS-013 — Accepted V5 authorizes the bounded opt-in projection

- Subject: accepted global scheduler field authority.
- Method: Product Direction inspection.
- Result: V5 preserves the legacy response shape when the query parameter is
  omitted and authorizes exactly the paired projection fields for explicit
  `dispatchableOnly=true|false`; it retains the current read-role gate, forbids
  Grant expansion, requires pre-pagination filtering, and leaves the exact
  closed enum/predicates to this child.
- Provenance:
  `docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V5.md:409-484,1047-1067,1199-1218`.

### OBS-014 — Existing HTTP compatibility freezes response structure

- Subject: current HTTP compatibility bundle.
- Method: contract inspection.
- Result: policy is `strict_backward_compatible`; valid existing requests must
  retain the same response structure, per-endpoint naming, cursor shape, limits,
  authorization, and existing error meanings.
- Provenance: `contracts/workflow-http/v1/compatibility.md:1-25`.

## 6. Claims and assumptions

### CLM-001 — Dispatchability is a projection, not a new lifecycle

- Support state: SUPPORTED
- Supported by: `EVD-001`, `EVD-002`, `EVD-004`, `EVD-005`, `EVD-006`,
  `EVD-007`.
- Claim: V1 eligibility can be computed from existing current facts without a
  mutable `dispatchable` column, event, state machine, or receipt.
- Limitation: no formal ops-lock fact exists, so V1 cannot represent an external
  convention as a Workflow blocker.

### CLM-002 — DRAFT is not ordinary assignee-dispatch work

- Support state: SUPPORTED
- Supported by: `EVD-003` plus the Owner direction represented in §3.2.
- Claim: current Node `DRAFT` is a creator-governed preparation/revision category
  and must not be returned as ordinary dispatchable assignee work, even when
  creator and assignee happen to be the same Principal.
- Limitation: current assigned-to-me implementation still includes DRAFT; this
  proposal intentionally defines a stricter dispatchability projection rather
  than reinterpreting that existing endpoint.

### CLM-003 — Existing command gates are authoritative objective blockers

- Support state: SUPPORTED
- Supported by: `EVD-005`, `EVD-006`, `EVD-007`.
- Claim: cancellation, archive, Terminal, missing assignee, disabled assignee,
  blocked Definition Version status, and open current-Visit Assistance are
  objective Workflow facts that make an Instance unsuitable for ordinary
  assignee dispatch at the snapshot.

### CLM-004 — Domain disabled is not sufficiently authoritative for V1 reason

- Support state: INFERRED
- Supported by: `EVD-009`.
- Claim: Domain disablement affects current worklist publication but is not a
  uniform ordinary-transition blocker at the pinned base. Freezing
  `DOMAIN_DISABLED` as a dispatch-block reason would choose new lifecycle
  semantics rather than merely project one existing authority.
- Disposition: excluded from the V1 enum; requires a separate authority
  reconciliation if desired.

### CLM-005 — Definition archive is not a V1 blocker

- Support state: SUPPORTED
- Supported by: `EVD-009`.
- Claim: Definition archive/discovery metadata does not stop existing Instances;
  Definition Version status is the relevant transition fact.

### CLM-006 — OPS_LOCKED cannot be honestly frozen

- Support state: SUPPORTED
- Supported by: `EVD-008`.
- Claim: no formal Workflow-owned ops-lock fact exists at the pinned base.
  Metadata string matching would invent a second, undocumented authority.

### CLM-007 — V5 permits bounded global publication through this child

- Support state: SUPPORTED
- Supported by: `EVD-010`.
- Claim: accepted V5 permits the exact opt-in field pair and filter on the
  Domain/global list family, provided this child retains the current route
  authorization, closed non-sensitive reasons, compatibility, pagination, and
  caller-policy boundaries.

No open assumption changes the proposed V1 predicate. No parent-authority
conflict remains at the pinned base; independent review and Owner acceptance
remain mandatory before the child becomes active implementation authority.

## 7. Evidence relations

### EVD-001 — Query types support the current-filter State

- Source observations: `OBS-001`, `OBS-002`.
- Target: `STATE-001`, `STATE-002`, `CLM-001`.
- Relation: SUPPORTS.
- Coordinates: repository/base/observation time from the header.
- Strength: direct source evidence.
- Limitation: does not prove production deployment revision.

### EVD-002 — SQL and tests support lifecycle semantics

- Source observations: `OBS-002`, `OBS-003`.
- Target: `STATE-003`, `CLM-001`.
- Relation: SUPPORTS.
- Strength: source plus explicit integration expectations.
- Limitation: tests are definitions, not executed runtime evidence in this
  authoring task.

### EVD-003 — DRAFT authorities support separate dispatch treatment

- Source observations: `OBS-004`, `OBS-005`.
- Target: `STATE-004`, `CLM-002`.
- Relation: SUPPORTS.
- Strength: frozen Architecture/Contract plus source.
- Limitation: exclusion from dispatchability is the new candidate Decision,
  not a claim that current assigned-to-me already excludes DRAFT.

### EVD-004 — Database invariant supports no-assignee handling

- Source observations: `OBS-006`.
- Target: `STATE-005`, `CLM-001`, `CLM-003`.
- Relation: SUPPORTS.
- Strength: trigger-level invariant for new/updated Visits.
- Limitation: legacy or corrupt data remains a defensive concern.

### EVD-005 — Cancel/archive authority supports closure reasons

- Source observations: `OBS-007`.
- Target: `STATE-006`, `CLM-001`, `CLM-003`.
- Relation: SUPPORTS.
- Strength: effective Architecture, schema, and source agree.

### EVD-006 — Transition gates support actor/version reasons

- Source observations: `OBS-008`.
- Target: `STATE-007`, `CLM-001`, `CLM-003`.
- Relation: SUPPORTS.
- Strength: Contract and source agree.

### EVD-007 — Assistance facts support ASSISTANCE_OPEN

- Source observations: `OBS-009`, `OBS-010`.
- Target: `STATE-008`, `CLM-001`, `CLM-003`.
- Relation: SUPPORTS.
- Strength: formal schema plus both ordinary and combined command gates.
- Limitation: V5 authorizes only the exact closed `ASSISTANCE_OPEN` code and
  forbids exposing Assistance content or any other derived Assistance status.

### EVD-008 — Repository search supports OPS lock gap

- Source observations: `OBS-011`.
- Target: `STATE-009`, `CLM-006`.
- Relation: SUPPORTS.
- Strength: repository-wide source/schema/document search at the pinned base.
- Limitation: does not prove no external operator uses an undocumented
  convention; such a convention is precisely not Workflow authority.

### EVD-009 — Mixed Domain behavior supports deferral

- Source observations: `OBS-012`.
- Target: `STATE-010`, `CLM-004`, `CLM-005`.
- Relation: SUPPORTS.
- Strength: direct comparison of read and write paths.
- Limitation: a future parent authority may choose Domain disablement as a
  dispatch blocker.

### EVD-010 — Product Direction supports bounded child activation

- Source observations: `OBS-013`.
- Target: `CLM-007` and the acceptance-readiness statement in §3.3.
- Relation: SUPPORTS.
- Strength: explicit accepted V5 parent Contracts and child sequencing.

### EVD-011 — Compatibility policy requires explicit wire reconciliation

- Source observations: `OBS-014`.
- Target: the wire-authority statement in §3.3 and `CTR-DISP-013`.
- Relation: SUPPORTS.
- Strength: explicit current-state compatibility policy.
- Limitation: accepted V5 has selected explicit query opt-in; this evidence does
  not by itself verify a future implementation's omitted/false/true wire shapes.

## 8. Decisions

### DEC-001 — Workflow owns objective dispatchability

- Decision owner: repository owner `mayf3` through the repository acceptance
  process.
- Decision: Workflow computes objective current eligibility; callers choose whom
  to contact and how.
- Rejected alternative: duplicate Workflow-specific predicates in HR/Scheduler
  prompts or code.
- Reason: avoid semantic leakage and divergent interpretations.

### DEC-002 — Dispatchability is a read-time convenience projection

- Decision: `dispatchable` and reasons are derived in the same read snapshot.
  They are not stored, evented, leased, reserved, or accepted as a substitute
  for command-time validation.
- Rejected alternative: a mutable dispatch status or lock column maintained by
  callers.
- Reason: preserve existing Workflow facts as the sole authority and avoid
  synchronization drift.

### DEC-003 — V1 uses a closed, source-backed reason enum

- Decision: V1 freezes exactly these wire values:

```text
ARCHIVED
CANCELLED
TERMINAL
DRAFT_NODE
NO_ASSIGNEE
ASSIGNEE_DISABLED
DEFINITION_VERSION_REVOKED
DEFINITION_VERSION_DRAFT
ASSISTANCE_OPEN
```

- Rejected for V1:
  - `OPS_LOCKED`: no authoritative primitive;
  - `DOMAIN_DISABLED`: current worklist and transition authorities disagree on
    whether it stops ordinary work;
  - `DEFINITION_ARCHIVED`: archive/discovery does not block existing Instance
    transition;
  - transition-specific reasons such as `TARGET_ASSIGNEE_UNAVAILABLE` or
    `ADVANCE_NOT_PRIMARY`: these answer whether one selected outgoing Transition
    can commit, not whether the current assignee should begin current-node work.
- Reason: freeze only objective facts with a direct current authority.

### DEC-004 — DRAFT is always non-dispatchable for ordinary work

- Decision: current Node type `DRAFT` adds `DRAFT_NODE`, regardless of whether
  creator equals current assignee or a special combined command is currently
  possible.
- Rejected alternative: dispatch DRAFT when creator equals assignee.
- Reason: creator drafting/revision is a separate category from ordinary
  assignee processing.

### DEC-005 — All true reasons are returned in stable order

- Decision: reasons are not mutually exclusive. The response includes every
  true V1 reason in the order listed in `DEC-003`; `dispatchable` is true iff the
  resulting array is empty.
- Rejected alternative: return only the first reason.
- Reason: preserve factual diagnostics without introducing priority semantics.

### DEC-006 — Representation opt-in and filtering compose with existing cursors

- Decision: omitted `dispatchableOnly` preserves the legacy representation;
  explicit `false` selects the paired projection without filtering; explicit
  `true` selects the pair and adds an `AND` predicate evaluated before page
  limiting.
- Rejected alternative: fetch a page then discard blocked rows in the handler.
- Reason: avoid sparse/empty intermediate pages and missed eligible rows.

### DEC-007 — OPS lock is deferred, not simulated

- Decision: no metadata key/value or free-text convention is recognized as an
  ops-lock. A future capability must first define a Workflow-owned authoritative
  primitive, lifecycle, writer authorization, audit, and interaction with
  transition commands.
- Rejected alternative: `metadata contains "ops-lock"`.
- Reason: that would invent an undocumented contract and second authority.

### DEC-008 — Pagination and scheduling boundaries remain unchanged

- Decision: retain `items + next_cursor`; callers traverse until
  `next_cursor = null`. No offset pagination or scheduler policy enters
  Workflow.

## 9. Contracts

The Contracts below are candidate obligations while this Spec is `proposed`.
They become implementation authority only after independent review, Owner
acceptance, and merge to `main`.

### CTR-DISP-001 — Summary fields

When and only when a Domain or global list request explicitly supplies
`dispatchableOnly=true|false`, every returned `DomainInstanceSummary` MUST
include both:

```text
dispatchable: boolean
dispatch_blocked_reasons: array<DispatchBlockedReason>
```

The wire field names MUST remain snake_case, matching the existing summary
surface. `dispatch_blocked_reasons` MUST be present even when empty and MUST NOT
be null. When `dispatchableOnly` is omitted, neither field may be emitted.

### CTR-DISP-002 — Exact derivation

Within one list-query read snapshot, the service MUST add the following reason
when and only when the corresponding fact is true:

| Reason | Authoritative predicate |
|---|---|
| `ARCHIVED` | `workflow_instances.archived_at IS NOT NULL` |
| `CANCELLED` | `workflow_instances.cancelled = TRUE` |
| `TERMINAL` | current Node `node_type = 'TERMINAL'` |
| `DRAFT_NODE` | current Node `node_type = 'DRAFT'` |
| `NO_ASSIGNEE` | current Visit `assignee_principal_id IS NULL` |
| `ASSIGNEE_DISABLED` | current Visit assignee exists and its `principals.enabled = FALSE` |
| `DEFINITION_VERSION_REVOKED` | Instance Definition Version status is `REVOKED` |
| `DEFINITION_VERSION_DRAFT` | Instance Definition Version status is `DRAFT` |
| `ASSISTANCE_OPEN` | an Assistance case for the current Visit has status `OWNER_PENDING` or `HUMAN_REQUIRED` |

The service MUST return all true reasons in the table order. It MUST set
`dispatchable = (dispatch_blocked_reasons is empty)`.

A missing Principal row for a non-null current assignee is an internal
consistency/storage failure under the existing foreign-key model; it MUST NOT be
silently reclassified as dispatchable or as a new uncontracted reason.

### CTR-DISP-003 — Closed enum and no metadata inference

The V1 wire enum MUST contain exactly the values in `DEC-003`. The service MUST
NOT infer a reason from Instance, Domain, Principal, Definition, Context, or
Assistance free-text/JSON metadata. In particular, it MUST NOT recognize
`ops-lock`, `ops_lock`, or similar strings.

### CTR-DISP-004 — Query parameter

Both existing endpoints MUST accept optional camelCase query parameter:

```text
dispatchableOnly=true|false
```

- omitted: no dispatchability predicate is added and neither projection field
  is emitted;
- `false`: no dispatchability predicate is added and both projection fields are
  emitted;
- `true`: only rows whose derived reason array is empty are eligible and both
  projection fields are emitted;
- any other value: reject with HTTP 422 and stable code
  `invalid_dispatchable_only`, without returning a partial page.

The parameter MUST compose by logical `AND` with `definitionKey`, `lifecycle`,
`status`, `currentNodeKey`, `assigneePrincipalId`, Domain scope, and cursor.

### CTR-DISP-005 — Filter before limit

`dispatchableOnly=true` MUST be applied in the repository query before
`ORDER BY`, `limit + 1` page selection, and `next_cursor` construction. The
implementation MUST NOT fetch an unfiltered page and remove blocked rows after
selection.

### CTR-DISP-006 — Existing pagination remains authoritative

The response MUST remain:

```text
{
  "items": [...],
  "next_cursor": {"created_at": "...", "id": "..."} | null
}
```

Ordering MUST remain `(created_at DESC, workflow_instance_id DESC)`. Default and
maximum limits MUST remain 20 and 100. No `page`, `totalPages`, `totalCount`, or
offset cursor may be added. A caller that wants the full eligible set MUST
follow `next_cursor` until null.

### CTR-DISP-007 — Same-snapshot projection

The fields, their reason predicates, filter decision, returned rows, and page
cursor MUST be computed from the same `REPEATABLE READ` repository snapshot used
for that list page. Projection MUST NOT write Workflow, Assistance, Event,
Receipt, audit-content, notification, or scheduling state.

This is a snapshot assertion only. It MUST NOT promise that an Instance remains
eligible after the response.

### CTR-DISP-008 — Command-time authority remains final

`dispatchable=true` MUST mean only that no V1 blocker was true at the list
snapshot. It MUST NOT authorize Transition, bypass current-assignee checks,
bypass expected Workflow state version, reserve the Instance, or guarantee a
later command will succeed. Existing command-time locks and validations remain
the final mutation authority.

### CTR-DISP-009 — Existing authorization and data boundary remain unchanged

The Domain list MUST retain its current Domain Owner/visibility gate. The global
list MUST retain the current accepted compatibility gate
`GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR` and its error behavior
until separately superseded. This Spec MUST NOT add or change any role, Grant,
scope, identity, credential, allowlist, or write reach, and MUST NOT grant
transition, Assistance, cancel/archive, Definition, membership, audit-content,
or payload access.

The new reason fields MUST expose only the closed codes, never Assistance
payload/status detail, metadata, Context, Submission, timeline, or audit data.

### CTR-DISP-010 — DRAFT exclusion is independent of other filters

A current DRAFT Instance MUST have `dispatchable=false` and include
`DRAFT_NODE`. Therefore it MUST never appear under `dispatchableOnly=true`, even
when `lifecycle=active`, status is active, it has an enabled assignee, and its
creator equals its assignee.

### CTR-DISP-011 — OPS lock gap remains explicit

V1 MUST NOT emit `OPS_LOCKED`. If an external system maintains an undocumented
operational-lock convention, V1 makes no assertion about it. Adding an
operational lock later requires a separate accepted authority and a new
compatible enum/version decision; it MUST NOT be smuggled into an existing V1
reason.

### CTR-DISP-012 — Caller scheduling policy stays outside Workflow

Workflow MUST NOT use the projection to select Principals by fairness, priority,
round-robin, notification history, quota, retry interval, AgentId mapping, or
session state. It MUST NOT send or wake an Agent. Those remain caller policy.

### CTR-DISP-013 — Compatibility and rollout

Adding the optional query parameter is additive. Existing requests that omit
`dispatchableOnly` MUST preserve their current population, exact response
structure, defaults, ordering, cursor semantics, authorization, and errors;
neither projection field is emitted. Explicit `false` or `true` opts into the
paired fields. A client that rejects unknown response fields remains compatible
until it explicitly opts into the new representation.

### CTR-DISP-014 — No schema migration required for V1

The V1 projection MUST be derived from existing relational facts and MUST NOT
add a persisted dispatchability or ops-lock column. If an implementation later
proves a schema/index change is needed solely for query performance, it requires
its own reviewed migration and must preserve the exact predicate and wire
semantics here.

## 10. Acceptance

These are future verification mappings, not executed evidence.

### ACC-DISP-001 — Reason matrix

- Contracts: `CTR-DISP-001`, `CTR-DISP-002`, `CTR-DISP-003`,
  `CTR-DISP-010`, `CTR-DISP-011`.
- Method: integration fixtures independently and cumulatively set each formal
  fact; query both Domain and global surfaces.
- Environment: disposable PostgreSQL integration database and real HTTP service
  process at the pinned implementation commit.
- Required evidence: executed fixture matrix, implementation/database revision,
  seeded facts, and exact Domain/global request-response transcript.
- Expected: exact closed codes, all true reasons, table order, empty array iff
  dispatchable, DRAFT always blocked, no metadata-derived reason.
- Failure: missing, extra, reordered, free-text-derived, or incorrectly combined
  reasons.

### ACC-DISP-002 — Dispatchable-only composition

- Contracts: `CTR-DISP-004`, `CTR-DISP-005`, `CTR-DISP-010`.
- Method: create interleaved eligible and blocked rows across all existing
  filters; use small limits.
- Environment: disposable PostgreSQL integration database with interleaved
  fixtures and real repository query/HTTP paths at the pinned implementation.
- Required evidence: executed page sequence, fixture identities, SQL/query-plan
  provenance, implementation commit, and exact responses.
- Expected: every returned item is dispatchable, pages are filled from eligible
  rows when enough exist, and no eligible row is skipped by post-page filtering.
- Failure: sparse pages caused by handler filtering, blocked rows returned, or
  filter composition changed from `AND`.

### ACC-DISP-003 — Cursor walk

- Contracts: `CTR-DISP-005`, `CTR-DISP-006`, `CTR-DISP-007`.
- Method: walk `next_cursor` with `limit=2` until null over stable fixtures.
- Environment: stable disposable PostgreSQL snapshot at the pinned
  implementation, exercised through both Domain and global HTTP endpoints.
- Required evidence: ordered request/cursor/response transcript, fixture census,
  implementation commit, and duplicate/missing-row comparison.
- Expected: no duplicates/misses, existing cursor shape/order, no count or offset
  fields.
- Failure: discontinuity, changed cursor shape, or pagination additions.

### ACC-DISP-004 — Existing requests are compatible

- Contracts: `CTR-DISP-006`, `CTR-DISP-009`, `CTR-DISP-013`.
- Method: run current Domain/global list authorization, default, lifecycle,
  status, assignee, definition, node, and pagination suites without the new
  parameter; compare populations and errors.
- Environment: pinned-base and pinned-implementation real HTTP integration
  services backed by equivalent disposable PostgreSQL fixtures.
- Required evidence: before/after golden JSON key sets and populations, executed
  authorization/error suite, base/implementation commits, and cursor transcript.
- Expected: unchanged rows/order/errors and exact legacy response structure;
  neither projection field is present.
- Failure: either new field appears, or defaults, authorization, population,
  limit, cursor, or any existing field changes.

### ACC-DISP-005 — Invalid query value

- Contracts: `CTR-DISP-004`.
- Method: query `dispatchableOnly=1`, empty, mixed case, and arbitrary text.
- Environment: real HTTP parser/handler integration service at the pinned
  implementation with a disposable PostgreSQL database.
- Required evidence: executed requests, exact HTTP status/error envelopes,
  implementation commit, and proof that no page body was returned.
- Expected: HTTP 422 `invalid_dispatchable_only`, no items.
- Failure: coercion, silent ignore, 200, or partial response.

### ACC-DISP-006 — Snapshot and read-only behavior

- Contracts: `CTR-DISP-007`, `CTR-DISP-008`, `CTR-DISP-014`.
- Method: concurrency integration test around blocker changes plus database write
  census.
- Environment: instrumented disposable PostgreSQL integration database using
  the production-equivalent isolation path at the pinned implementation.
- Required evidence: synchronized concurrency trace, transaction isolation
  record, before/after schema and write census, requests, and command result.
- Expected: each page is internally consistent at one snapshot; zero new
  dispatch state writes; later command still revalidates and can reject stale
  work.
- Failure: mixed-snapshot fields/filtering, a reservation implication, or stored
  dispatch state.

### ACC-DISP-007 — Assistance privacy and role boundaries

- Contracts: `CTR-DISP-009`, `CTR-DISP-012`.
- Method: role matrix for Domain Owner, global reader/coordinator, current
  assignee, and unauthorized caller; use Assistance payload canaries.
- Environment: isolated auth/role integration fixtures plus disposable
  PostgreSQL Workflow/Assistance data at the pinned implementation.
- Required evidence: executed allow/deny matrix, role-binding before/after
  census, seeded canary markers, exact responses, and forbidden-content scan.
- Expected: only the closed `ASSISTANCE_OPEN` code where authorized by the
  accepted V5/child boundary; no Assistance payload/status/body leak and no new
  write capability.
- Failure: payload/content disclosure, role expansion, or scheduling side effect.

### ACC-DISP-008 — Parent authority gate

- Contracts: all.
- Method: semantic review of the exact proposed Spec head against the exact
  accepted Product Direction head.
- Environment: fresh clean repository worktree pinned to the PR base/head and
  the authoritative GitHub PR state at review time.
- Required evidence: exact base/head/blob hashes, V5 lifecycle/merge coordinates,
  changed-file inventory, governance/schema/structure results, and persistent
  independent review record.
- Expected before acceptance: `SVC_WORKFLOW_PRODUCT_BOUNDARY_V5` is
  accepted/current on `main`, this Spec is based on that authority, and every
  V5 child constraint is represented with no unresolved conflict.
- Failure: V5 is absent/superseded, the base predates its activation, or this
  child widens its exact opt-in/authorization/privacy/pagination boundary.

## 11. Alternatives and disposition

### ALT-001 — Let HR/Scheduler infer Workflow blockers

- Disposition: rejected.
- Reason: leaks Workflow semantics and creates divergent authorities.

### ALT-002 — Treat `lifecycle=active` as dispatchable

- Disposition: rejected.
- Reason: active means only non-Terminal and can include DRAFT and, when status is
  omitted, cancelled non-terminal Instances.

### ALT-003 — Parse metadata for ops-lock

- Disposition: rejected.
- Reason: no authoritative key, writer, lifecycle, or command interaction exists.

### ALT-004 — Reuse open Assistance as OPS_LOCKED

- Disposition: rejected.
- Reason: Assistance is a distinct formal capability and reason
  `ASSISTANCE_OPEN`; renaming it would erase its authority and semantics.

### ALT-005 — Make dispatchability the unique execution authority

- Disposition: rejected.
- Reason: list results are snapshots and cannot replace atomic command-time
  validation.

### ALT-006 — Offset pagination and total counts

- Disposition: rejected.
- Reason: current keyset pagination remains sufficient and authoritative.

### ALT-007 — Include Domain disabled in V1

- Disposition: deferred.
- Reason: worklist and transition paths do not currently provide one uniform
  “cannot work” authority. Reconcile that lifecycle meaning first.

### ALT-008 — Include target-transition availability

- Disposition: rejected for this V1.
- Reason: dispatchability concerns beginning current-node work, not selecting and
  validating one outgoing Transition. Transition-specific projections remain
  separate.

## 12. Migration, compatibility, and rollback

### 12.1 Migration

No data migration and no persisted dispatchability state are proposed.
OPS lock remains deferred.

### 12.2 Compatibility

- Existing query behavior and response structure are unchanged when
  `dispatchableOnly` is omitted; the response contains neither new field.
- Explicit `dispatchableOnly=false` opts into both fields without filtering.
- Explicit `dispatchableOnly=true` opts into both fields and filters before
  ordering, limiting, and cursor construction.
- Existing cursor traversal remains mandatory for full enumeration.
- Clients requesting the opt-in representation must understand the paired
  fields; legacy clients receive no unknown field.

### 12.3 Rollback

If a future implementation is authorized and must be rolled back, remove the
query parameter behavior and additive projection fields together at the HTTP
capability boundary, without changing underlying Workflow facts. No data
rollback is needed because V1 stores no new state. Rollback must not be described
as Product Direction reversal; any durable semantic reversal follows governance.

## 13. Open questions and authority readiness

Accepted V5 resolves the former parent conflict and leaves exact predicates and
the closed enum to this child. No Owner policy decision or normative TBD remains
inside this bounded projection. OPS lock and Domain-disabled semantics remain
explicitly deferred rather than inferred.

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE_WITHIN_THIS_CANDIDATE_PREDICATE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
OPS_LOCK_AUTHORITY = GAP
OPS_LOCK_DISPOSITION = DEFERRED_REQUIRES_SEPARATE_AUTHORITATIVE_PRIMITIVE
DOMAIN_DISABLED_DISPOSITION = DEFERRED_REQUIRES_AUTHORITY_RECONCILIATION
SLICE_D_BROADER_CONFORMANCE_DEBT = EXPLICITLY_NOT_CLOSED_BY_THIS_CHILD
READY_FOR_INDEPENDENT_REVIEW = YES
READY_TO_MARK_ACCEPTED = YES_AFTER_INDEPENDENT_REVIEW
IMPLEMENTATION_ALLOWED = NO
```

## 14. Authoring summary

```text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1
SPEC_KIND = implementation
STATUS = proposed
AUTHORITY_LEVEL = governing_spec
IMPLEMENTATION_AUTHORITY = contracts
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
EXTERNAL_AUTHORITIES = NONE
CONTRACT_COUNT = 14
CONTRACTS_WITH_ACCEPTANCE = 14
AUTHORING_READY_FOR_REVIEW = YES
READY_TO_MARK_ACCEPTED = YES_AFTER_INDEPENDENT_REVIEW
PRODUCT_CODE_CHANGE = NONE
PRODUCTION_CHANGE = NONE
```
