---
authority_id: SVC_WORKFLOW_ARCHITECTURE_V0_4_0
status: proposed
authority_kind: architecture
owning_repository: mayf3/svc-workflow
implementation_authority: none
production_apply_authority: none
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
supersedes:
  - SVC_WORKFLOW_ARCHITECTURE_V0_3_1
superseded_by: null
owners:
  - mayf3
---

# svc-workflow Serial Visit-Activation Architecture v0.4.0

## 1. Goal and lifecycle

This document proposes the complete core Architecture for the V6 Product
Direction. It is a whole-authority successor to
SVC_WORKFLOW_ARCHITECTURE_V0_3_1, not a partial patch and not an instruction
to compose selected paragraphs from both versions.

~~~text
AUTHORITY_ID = SVC_WORKFLOW_ARCHITECTURE_V0_4_0
STATUS = proposed
ARCHITECTURE_ACTION = SUPERSEDE
WHOLE_AUTHORITY_SUPERSESSION = YES
SUPERSEDES = SVC_WORKFLOW_ARCHITECTURE_V0_3_1
GOVERNED_BY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PARTIAL_SUPERSESSION = NONE
~~~

While proposed and unmerged, this document is non-authoritative. It does not
change the status or backlink of v0.3.1, does not alter the local authority
map, and does not authorize implementation, schema changes, migration,
deployment, dispatch cutover, production apply, or work in another
repository.

The Goal is to preserve the reusable serial kernel, Domain isolation,
immutable workflow facts, deterministic Transition authority, idempotency,
audit, recovery, and compatibility boundaries of v0.3.1 while replacing its
Legacy-only node and work-discovery model with the explicit V6
Visit-activation model for new traffic.

## 2. Scope and non-goals

### 2.1 In scope

This Architecture defines:

- one explicit immutable semantic-model discriminator for Definition Versions
  and Instances;
- the Legacy and V6 new-traffic semantic-model boundary;
- Domain, Principal, Definition, Context, Instance, Node Visit, Submission,
  Event, Receipt, audit, and projection invariants;
- TASK and TERMINAL node semantics, deterministic serial graph rules, and
  Human/Agent TASK owner resolution;
- Node Visit as the only runtime work identity;
- exactly one canonical Human Work Item or Dispatch Intent for each active
  non-terminal TASK Visit;
- the immutable activation facts and rebuildable active/closed and
  nextEligibleAt projections;
- create, Transition, RETURN, graph TERMINATE, Cancel, Archive, administrative
  move/terminate, wake, repair, and one-time migration transaction boundaries;
- idempotency, retry, timeout, outcome_unknown, and attempt-scoped lease
  separation;
- activation-driven bounded delivery and recovery-only periodic scans;
- replay, rebuild, repair, event, fact, and projection integrity;
- a one-way new-traffic barrier with bounded Legacy drain, exact one-time
  migration, manual termination, historical replay, containment, and rollback;
- the exact relationship to v0.3.2, accepted child Specs, legacy implementation
  contracts, current list/worklist/HTTP compatibility, and external
  interoperability authority.

### 2.2 Explicit non-goals

This Architecture does not define or authorize:

- parallel nodes, dynamic forward branching, claim/pull, reassignment,
  handoff, delegation, WAIT_EVENT, WAIT_TIMER, HUMAN_TASK, AGENT_TASK,
  SERVICE_TASK, SLA orchestration, arbitrary script guards, or built-in LLM
  execution;
- a generic Scheduler, Dispatcher, Agent mapping, Session, message transport,
  fairness, priority, quota, retry policy, resource manager, event platform,
  Kafka platform, generic Outbox platform, or Operator;
- a second Scheduler wait field, wait status, blocked-reason predicate,
  business-node convention, or lease-backed workflow state;
- new Product Direction permissions, role grants, credentials, identity
  linkage, trust boundaries, cross-Domain authority, or cross-repository
  permission;
- an endpoint, table name, index, DDL encoding, timestamp precision,
  delivery transport, deployment release, production barrier coordinate, or
  migration plan;
- modification, acceptance, implementation, or lifecycle control of any
  dsh-agent-core authority or PR;
- a direct product-code implementation Spec.

Upper-layer Todo, Requirement, Article, Campaign, task label, priority, and
business objects remain outside svc-workflow. Workflow Context is bounded
workflow input, not an upper-layer database.

## 3. Authority and exact coordinates

~~~text
REPOSITORY = mayf3/svc-workflow
AUTHORING_BASE_REF = github/main
AUTHORING_BASE_COMMIT = efdfb7e1a0e6a381b2ab000d48f842991d5c0bb6
ACTIVE_PRODUCT_DIRECTION = SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
ACTIVE_PRODUCT_DIRECTION_ACCEPTED_HEAD = e9f13ace910b2b35037ac62e4d33c9305979ae4e
ACTIVE_PRODUCT_DIRECTION_MAIN_MERGE = efdfb7e1a0e6a381b2ab000d48f842991d5c0bb6
ACTIVE_PRODUCT_DIRECTION_BLOB = fecca7168b8a9e043664842cd92557fd09615c82
CURRENT_PRIMARY_ARCHITECTURE = SVC_WORKFLOW_ARCHITECTURE_V0_3_1
CURRENT_PRIMARY_ARCHITECTURE_BLOB = 98c78dcc8d07fb5b4148860962be080ddac2d182
RETAINED_REFINEMENT = SVC_WORKFLOW_ARCHITECTURE_V0_3_2
RETAINED_REFINEMENT_BLOB = eaa256cbcdf655a1fde72e85aafb12b0a2767648
PREFLIGHT_MODE = SUPERSEDE
IMPLEMENTATION_ALLOWED = NO
~~~

Authority precedence is V6 Product Direction, then the accepted primary
Architecture and compatible refinements, then accepted governing child Specs,
then descriptive code, schemas, tests, runtime, and operations.

### 3.1 Whole-supersession boundary

If accepted through a later lifecycle transaction, v0.4.0 replaces all
v0.3.1 Architecture meaning for new authority. It restates the reusable
v0.3.1 kernel here and does not require a reader to consult v0.3.1 to
understand new implementation obligations. v0.3.1 remains historical
authority for revisions and Legacy facts created while it governed.

SVC_WORKFLOW_ARCHITECTURE_V0_3_2 remains unchanged and separately effective
for Cancel and Archive. This Architecture incorporates its meaning without
editing or superseding that file. The only new-model extension is that Cancel
and every other current-work closure must close the current canonical
activation in the same transaction.

### 3.2 Accepted child and legacy authority relations

| Authority | Exact base object | Relationship under v0.4.0 |
|---|---|---|
| SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1 | a6944973ff8010fdb3ba338ce06143df0d8b3ffc | retain the current route gate, role compatibility, wire/error/pagination meaning, and no-write boundary only; it is not a Dispatch Intent feed or permission grant |
| SVC_WORKFLOW_INVALID_RETURN_REFERENCES_HTTP_422_V1 | e86166fec095ac536b8b188d7c6b891eac36c501 | retain HTTP 422 code/detail and aggregate RETURN-reference validation; it does not choose node or activation semantics |
| SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1 | 67bcdf75c13c092efcea092b9b918c413fe12504 | retain the exact bounded Legacy successor exception; it creates no general reassignment or new-model authority |
| AUTH_PRINCIPAL_SELF_PROJECTION_AND_DOMAIN_MEMBERSHIP_V1 | afeee332776391281548f4f7ffecde913880eeef | retain verified token.sub self-projection and Domain-member limits; projection does not grant TASK ownership or global permission |
| v0.3.1 legacy implementation contracts | exact blobs at the authoring base | remain descriptive/implementation authority for their accepted Legacy scope and compatibility history; they cannot authorize VISIT_ACTIVATION_V1 |
| contracts/workflow-http/v1/contract.md | 9d81acb167567d9309846da504af2a5b73b86390 | retain existing wire surfaces until a later accepted implementation Spec explicitly versions or proves compatibility |

The existing legacy contracts include Definition, Create, Transition, Query,
PostgreSQL storage, Admin Recovery, Legacy Import, identity/auth, and internal
HTTP contracts. Their stable legacy meanings remain historical. Any conflict
with this higher Architecture is a future implementation-Spec reconciliation
item, not permission to silently change either side.

The exact legacy contract coordinates inspected for that relation are:

| Base path | Exact blob at efdfb7e1 | v0.4.0 relationship |
|---|---|---|
| docs/contracts/DEFINITION_SERVICE_CONTRACT_V0_1.md | b23ed1340ad26480ae919e56fb25c8e08abeed7a | retain Definition lifecycle/digest/immutability kernel; reconcile new semantic model and graph |
| docs/contracts/WORKFLOW_INSTANCE_CREATE_CONTRACT_V0_1.md | 853dd60c7f62563cb756578fce428216c4ea40c1 | retain create idempotency/atomicity; replace Legacy initial-node/work semantics for new model |
| docs/contracts/WORKFLOW_TRANSITION_CONTRACT_V0_1.md | 7a9a1cb6602d490fa3bbd008fa679ddc40dc4869 | retain serialization/Submission/Visit/Event closure; add activation closure/creation and new graph rules |
| docs/contracts/WORKFLOW_QUERY_CONTRACT_V0_1.md | a24b08ea5f2e7e9c89fb201e55379faa6f9e454d | retain Legacy/query compatibility only; never canonical dispatch feed |
| docs/contracts/POSTGRES_STORAGE_CONTRACT_V0_1.md | aa9e942878305c3e0e1100b7287d8df9747d430e | retain sole-store/constraint/transaction authority; extend only through later implementation Spec |
| docs/contracts/ADMIN_RECOVERY_CONTRACT_V0_1.md | aa4c436548a4662137b6657c71a3728246a2c44b | retain bounded recovery and Visit immutability; add correct activation outcome |
| docs/contracts/LEGACY_IMPORT_CONTRACT_V0_1.md | 88b96c27b84fd0f6bc6a64f3038aa33f2cca05c5 | retain exact Legacy/import history scope; no new-model or barrier bypass authority |
| docs/contracts/IMPLEMENTATION_CONTRACT_V0_1.md | 0618f60248046e7972970a13ca8d0ee8d87c37cd | historical implementation baseline only; does not authorize V6 code |
| docs/contracts/IDENTITY_PROVISIONING_API_V0.md | 0e01a15b28268a2c99871df1f4408ae22ff4f61b | retain bounded Principal projection/provisioning compatibility; no owner-type inference |
| docs/contracts/JWKS_OBO_AUTH_V0.md | 226c2d9e018bd2fa17b9dbf14593f9b1a35aa0d3 | retain verified authentication/actor boundary; no work or global permission grant |
| docs/contracts/INTERNAL_API_CONTRACT_V0_1.md | c76fb8d28de7590f5c8fcaa4cb30dfe428f0ee20 | retain Legacy internal HTTP adapter compatibility scope; no new-model or canonical dispatch feed authority |

The contracts/workflow-http/v1/contract.md wire contract is blob
9d81acb167567d9309846da504af2a5b73b86390 at the base. Its wire objects remain
compatibility authority only until a later accepted implementation Spec maps
every affected surface.

### 3.3 External authority boundary

svc-workflow defines only local interoperability acceptance conditions:

~~~text
PERIODIC_EXTERNAL_RECOVERY = RECONCILER_ONLY
SCHEDULER_MANAGEMENT = SEPARATE_FROM_NORMAL_DISPATCH
EXTERNAL_AUTHORITY_PATH = EXTERNAL_REPOSITORY_CHOICE
DSH_CONSUMER_DESIGN_IN_THIS_ARCHITECTURE = NONE
~~~

dsh-agent-core may satisfy those conditions through PR 87, a replacement
authority, or another locally lawful authority. This repository does not
prescribe that choice and cannot author, accept, amend, split, close, merge,
supersede, or implement an external authority.

## 4. Current State, Observations, Claims, and Evidence

All State in this section is descriptive and bound to exact coordinates.
Descriptive implementation never rewrites the Architecture.

### 4.1 Current State

Unless a State item names a narrower subject, its subject is
mayf3/svc-workflow; as_of_commit is efdfb7e1a0e6a381b2ab000d48f842991d5c0bb6;
the environment is the clean local authoring worktree plus the fetched GitHub
authority ref; and observed_at is 2026-09-01. Each item names its own basis.

- STATE-ARCH-001 — At svc-workflow efdfb7e1, V6 is active and v0.3.1 remains
  the primary Architecture; v0.4.0 does not yet exist. Basis:
  OBS-ARCH-001, OBS-ARCH-002, EVD-ARCH-001.
- STATE-ARCH-002 — v0.3.1 freezes DRAFT, NORMAL, TERMINAL node kinds and has no
  canonical activation or nextEligibleAt. That conflicts with active V6 for
  new traffic. Basis: OBS-ARCH-003, EVD-ARCH-002.
- STATE-ARCH-003 — v0.3.2 adds graph-external Cancel and non-destructive
  Archive, keeps current Visit unchanged on Cancel, and does not alter node
  semantics. Basis: OBS-ARCH-004, EVD-ARCH-003.
- STATE-ARCH-004 — Current source has a descriptive Definition-Version
  semantic_model_version with values 1 Legacy and 2 Minimal. Descriptive
  Minimal permits multiple ADVANCE edges and forbids TERMINATE, so it is not
  V6 VISIT_ACTIVATION_V1 authority. Basis: OBS-ARCH-006, EVD-ARCH-004.
- STATE-ARCH-005 — Current global list is gated by
  GLOBAL_WORKFLOW_READER or legacy GLOBAL_WORKFLOW_COORDINATOR and returns
  Instance summaries; current assigned-to-me reads current Visit/assignee.
  Neither is a canonical activation feed. Basis: OBS-ARCH-007,
  EVD-ARCH-005.
- STATE-ARCH-006 — Current schema/source has no Human Work Item, Dispatch
  Intent, canonical activation, or nextEligibleAt persistence. Basis:
  OBS-ARCH-008, EVD-ARCH-006.
- STATE-ARCH-007 — Phase-2 preflight classifies the change as SUPERSEDE,
  requires this exact successor/path, retains v0.3.2 unchanged, reports no
  child collision and no Owner decision, and forbids implementation. Basis:
  OBS-ARCH-009, EVD-ARCH-007.

### 4.2 Observations

#### OBS-ARCH-001 — Exact repository base

- Subject: mayf3/svc-workflow authority branch.
- Source revision: efdfb7e1a0e6a381b2ab000d48f842991d5c0bb6.
- Environment: fetched GitHub remote plus clean local authoring worktree.
- Observed at: 2026-09-01, Asia/Shanghai authoring session.
- Method: fetch github/main; inspect Git ref, worktrees, changed paths, V6,
  local authority map, and governance lock.
- Result: requested base equals current github/main; authoring branch started
  clean at that commit.
- Provenance: Git objects and repository files at the exact base.

#### OBS-ARCH-002 — Active V6 authority

- Subject: SVC_WORKFLOW_PRODUCT_BOUNDARY_V6.
- Source revision/blob: accepted head e9f13ace; main efdfb7e1; blob
  fecca7168b8a9e043664842cd92557fd09615c82.
- Environment: repository authority documents at the exact authoring base.
- Observed at: 2026-09-01, Asia/Shanghai authoring session.
- Method: complete document inspection.
- Result: V6 freezes TASK or TERMINAL, Human or Agent owner, Node Visit
  identity, exactly one activation, same-transaction server timestamp,
  activation-driven delivery, one-way cutover, bounded Legacy drain, and
  external authority separation.
- Provenance: docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V6.md.

#### OBS-ARCH-003 — v0.3.1 reusable kernel and conflict

- Subject: SVC_WORKFLOW_ARCHITECTURE_V0_3_1.
- Source revision/blob: efdfb7e1 /
  98c78dcc8d07fb5b4148860962be080ddac2d182.
- Environment: repository authority documents at the exact authoring base.
- Observed at: 2026-09-01, Asia/Shanghai authoring session.
- Method: complete document inspection.
- Result: it freezes the serial kernel, Domain/Definition/Context/Visit/
  Submission/Event/Receipt model, but new instances begin at DRAFT and no
  activation/wait fact exists.
- Provenance: docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_1.md.

#### OBS-ARCH-004 — v0.3.2 compatible refinement

- Subject: SVC_WORKFLOW_ARCHITECTURE_V0_3_2.
- Source revision/blob: efdfb7e1 /
  eaa256cbcdf655a1fde72e85aafb12b0a2767648.
- Environment: repository authority documents at the exact authoring base.
- Observed at: 2026-09-01, Asia/Shanghai authoring session.
- Method: complete document inspection.
- Result: Cancel is graph-external, keeps currentNodeVisitId and creates no
  Visit; Archive is non-destructive metadata; both increment version once and
  append one Event.
- Provenance: docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_3_2.md.

#### OBS-ARCH-005 — Accepted child and legacy contracts

- Subject: accepted governing child Specs and legacy bridge contracts listed
  in section 3.2.
- Source revision: efdfb7e1.
- Environment: repository authority and contract documents at the exact base.
- Observed at: 2026-09-01, Asia/Shanghai authoring session.
- Method: complete inspection of the named child Specs and relevant Definition,
  Create, Transition, Storage, Query, Admin Recovery, Legacy Import, auth, and
  HTTP contracts.
- Result: the reusable transaction, idempotency, visibility, RETURN 422,
  Principal projection, recovery, and compatibility rules are separable from
  the V6 node/activation delta.
- Provenance: exact paths and blobs in section 3.2.

#### OBS-ARCH-006 — Descriptive semantic-model code is not V6 authority

- Subject: migration 0019 and current Definition/runtime code.
- Source revision: efdfb7e1.
- Environment: descriptive repository source at the exact authoring base.
- Observed at: 2026-09-01, Asia/Shanghai authoring session.
- Method: inspect migration, model enum, minimal validator, create, and
  Transition dispatch.
- Result: value 1 means Legacy; value 2 means descriptive Minimal with
  multiple ADVANCE support and TERMINATE rejection. These semantics differ
  from V6 deterministic primary ADVANCE and configured TERMINATE.
- Provenance: migrations/0019_add_definition_semantic_model_version.sql and
  current source at efdfb7e1.
- Limitation: source is descriptive, not accepted Architecture.

#### OBS-ARCH-007 — Current list/worklist compatibility

- Subject: global-list role gate and assigned worklist.
- Source revision: efdfb7e1.
- Environment: descriptive repository source and accepted route authority at
  the exact authoring base.
- Observed at: 2026-09-01, Asia/Shanghai authoring session.
- Method: inspect query_visibility.rs, query_global_instances.rs,
  query_worklists.rs, handlers, and accepted Global Reader Spec.
- Result: global list uses Reader or Coordinator compatibility; assigned
  worklist derives from current Visit assignee; neither reads activation facts.
- Provenance: source and SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1.

#### OBS-ARCH-008 — No canonical activation persistence exists

- Subject: migrations and current source.
- Source revision: efdfb7e1.
- Environment: descriptive repository source at the exact authoring base.
- Observed at: 2026-09-01, Asia/Shanghai authoring session.
- Method: search migrations/source for Dispatch Intent, Human Work Item,
  canonical activation, and nextEligibleAt.
- Result: no persistence or command boundary for these V6 facts exists.
- Provenance: repository search at efdfb7e1.

#### OBS-ARCH-009 — Phase-2 authority investigation

- Subject: coordinator-persisted read-only preflight.
- Source revision: efdfb7e1.
- Environment: local coordinator evidence plus the exact repository base.
- Observed at: 2026-09-01, Asia/Shanghai authoring session.
- Method: read
  /Users/yanfenma/.codex/goal-coordination/GOAL_WORKFLOW_DISPATCH_CUTOVER_V1/state.json
  field LATEST_EVIDENCE.phase2_architecture_authority_investigation.
- Result: SUPERSEDE v0.3.1, implementation NO, required successor and path,
  v0.3.2 reuse unchanged, Global Reader compatibility only, no child
  collision, no Owner decision, no blocker.
- Provenance: coordinator state outside the repository.
- Limitation: this is preflight evidence, not semantic review or acceptance.

### 4.3 Claims and assumptions

#### CLM-ARCH-001 — Whole supersession is required

- Support state: SUPPORTED.
- Supported by evidence: EVD-ARCH-001, EVD-ARCH-002, EVD-ARCH-007.
- Contradicted by evidence: none known.
- Uncertainty: none affecting classification.

#### CLM-ARCH-002 — The serial kernel remains reusable

- Support state: SUPPORTED.
- Supported by evidence: EVD-ARCH-002, EVD-ARCH-003.
- Contradicted by evidence: none known.
- Uncertainty: exact implementation encoding remains child-Spec work.

#### CLM-ARCH-003 — Explicit model identity is mandatory

- Support state: SUPPORTED.
- Supported by evidence: EVD-ARCH-004.
- Contradicted by evidence: none known.
- Uncertainty: storage encoding is not selected here.

#### CLM-ARCH-004 — Immutable activation facts plus projections close replay

- Support state: INFERRED.
- Supported by evidence: EVD-ARCH-002, EVD-ARCH-006.
- Contradicted by evidence: none known.
- Uncertainty: exact table/event encoding requires implementation Spec.

#### CLM-ARCH-005 — Bounded activation delivery removes normal discovery scans

- Support state: INFERRED.
- Supported by evidence: EVD-ARCH-005, EVD-ARCH-006.
- Contradicted by evidence: none known.
- Uncertainty: exact bounded transport is deliberately deferred.

#### CLM-ARCH-006 — Compatibility views cannot grant dispatch authority

- Support state: SUPPORTED.
- Supported by evidence: EVD-ARCH-005.
- Contradicted by evidence: none known.
- Uncertainty: future permission-key mapping requires child authority.

#### CLM-ARCH-007 — One-way routing is compatible with bounded Legacy drain

- Support state: SUPPORTED.
- Supported by evidence: EVD-ARCH-002.
- Contradicted by evidence: none known.
- Uncertainty: exact barrier coordinate and inventory are later gates.

#### CLM-ARCH-008 — External interoperation can be constrained without
governing the consumer

- Support state: SUPPORTED.
- Supported by evidence: EVD-ARCH-002, EVD-ARCH-007.
- Contradicted by evidence: none known.
- Uncertainty: external repository chooses its own authority path.

### 4.4 Evidence relations

#### EVD-ARCH-001 — Base and authority graph

- Source observations: OBS-ARCH-001, OBS-ARCH-002.
- Target: STATE-ARCH-001, CLM-ARCH-001.
- Relation: SUPPORTS.
- Bound coordinates: svc-workflow efdfb7e1, 2026-09-01.
- Strength/sufficiency: direct Git and accepted-authority evidence.
- Limitations: does not accept this proposal.
- Provenance: exact repository refs and blobs.

#### EVD-ARCH-002 — V6 versus v0.3.1 semantic comparison

- Source observations: OBS-ARCH-002, OBS-ARCH-003.
- Target: STATE-ARCH-002, CLM-ARCH-001, CLM-ARCH-002,
  CLM-ARCH-004, CLM-ARCH-007.
- Relation: SUPPORTS.
- Bound coordinates: V6 and v0.3.1 blobs in section 3.
- Strength/sufficiency: complete normative comparison.
- Limitations: implementation state is not conformance.
- Provenance: exact authority files.

#### EVD-ARCH-003 — Cancel/Archive compatibility

- Source observations: OBS-ARCH-004, OBS-ARCH-005.
- Target: STATE-ARCH-003, CLM-ARCH-002.
- Relation: SUPPORTS.
- Bound coordinates: v0.3.2 and legacy contract blobs at efdfb7e1.
- Strength/sufficiency: complete authority/contract inspection.
- Limitations: activation closure is new-model elaboration under V6.
- Provenance: exact local files.

#### EVD-ARCH-004 — Descriptive semantic-model mismatch

- Source observations: OBS-ARCH-006.
- Target: STATE-ARCH-004, CLM-ARCH-003.
- Relation: SUPPORTS.
- Bound coordinates: current source and migration at efdfb7e1.
- Strength/sufficiency: direct code comparison.
- Limitations: code cannot authorize the replacement semantics.
- Provenance: repository source paths named in OBS-ARCH-006.

#### EVD-ARCH-005 — Compatibility route/worklist boundary

- Source observations: OBS-ARCH-005, OBS-ARCH-007.
- Target: STATE-ARCH-005, CLM-ARCH-005, CLM-ARCH-006.
- Relation: SUPPORTS.
- Bound coordinates: accepted Global Reader and source at efdfb7e1.
- Strength/sufficiency: direct contract and query evidence.
- Limitations: no future Dispatch Intent API is selected.
- Provenance: exact child Spec and source paths.

#### EVD-ARCH-006 — Activation implementation gap

- Source observations: OBS-ARCH-008.
- Target: STATE-ARCH-006, CLM-ARCH-004, CLM-ARCH-005.
- Relation: SUPPORTS.
- Bound coordinates: svc-workflow efdfb7e1.
- Strength/sufficiency: complete repository search for named facts.
- Limitations: absence does not select an implementation.
- Provenance: migrations/source inventory.

#### EVD-ARCH-007 — Preflight classification

- Source observations: OBS-ARCH-009.
- Target: STATE-ARCH-007, CLM-ARCH-001, CLM-ARCH-008.
- Relation: SUPPORTS.
- Bound coordinates: coordinator evidence for efdfb7e1.
- Strength/sufficiency: sufficient for authoring route and no-owner-decision
  result.
- Limitations: not semantic review.
- Provenance: coordinator state field named in OBS-ARCH-009.

## 5. Complete core Architecture

### 5.1 Semantic-model identity and routing

Every Workflow Definition Version has one required, immutable, explicit
semantic model ID. Every Workflow Instance records the same model ID at
creation and is immutably bound to a Definition Version carrying that exact
value.

~~~text
LEGACY_V1
  node model = DRAFT | NORMAL | TERMINAL
  authority = historical v0.3.1 and compatible Legacy contracts

VISIT_ACTIVATION_V1
  node model = TASK | TERMINAL
  authority = V6 plus accepted v0.4.0 successor
~~~

The logical IDs above are normative. A later implementation Spec must choose
their wire/storage encoding and prove a database-enforced equality between
the Instance discriminator and its immutable Definition Version. It must not
reuse or reinterpret a pre-existing descriptive numeric value whose meaning
differs. In particular, current descriptive Minimal value 2 is not silently
aliased to VISIT_ACTIVATION_V1.

No graph shape, node key, metadata, created time, migration time, environment
label, global flag, current route, or caller field may infer or override the
model. Unknown, null, mismatched, or unsupported model IDs fail closed before
Definition publication, Instance creation, Transition, migration, repair, or
delivery.

The cutover barrier selects which explicit model new traffic may request; it
does not change the meaning of an existing Definition Version or Instance.

### 5.2 Serial kernel and ownership boundary

svc-workflow owns one serial Workflow Instance state machine:

~~~text
one immutable Definition Version per Instance
one current Node Visit projection
one current Context Revision projection
one workflowStateVersion
one deterministic primary forward path
configured backward RETURN paths
configured graph TERMINATE paths
graph-external Cancel and bounded administrative recovery
~~~

Each Definition and Instance belongs to exactly one Domain. Domain is the
Definition-management, workflow-ownership, authorization, isolation, and
audit boundary. Domain Owner is represented only by the enabled
DOMAIN_OWNER DomainRoleBinding. One enabled Domain has exactly one effective
Owner. Owner replacement is atomic and never rewrites old Visits.

PostgreSQL remains the sole formal workflow database, and only svc-workflow
may write workflow-owned tables. A cache, delivery mechanism, external
consumer, compatibility view, or upper-layer shadow never becomes workflow
fact authority.

Normal data-plane isolation is strict. An ordinary Principal, member, Owner,
scope, allowlist, UI label, or combination of Domain-local roles does not
create cross-Domain authority. Lookup, list, count, cursor, denial, and
serialization must not leak another Domain. Only higher accepted Product
Direction permissions and their independently accepted implementation gates
may cross Domains.

V6 defines exactly two independent cross-Domain Product capabilities:

~~~text
GLOBAL_SCHEDULER_READ = bounded read of canonical active due Dispatch Intents
GLOBAL_DOMAIN_ADMIN   = bounded Domain create/initial-owner/owner-replace and
                        minimum selection directory
ONE_IMPLIES_THE_OTHER = NO
COMPOSITE_PERMISSION  = NONE
~~~

GLOBAL_SCHEDULER_READ grants no execution, Transition, Context, Submission,
reassignment, cancel/archive, Definition, membership, Assistance, credential,
or audit-content authority. GLOBAL_DOMAIN_ADMIN grants no workflow content or
command authority, self-grant, or self-Owner; self-Owner denial compares exact
canonical Principal identity, while distinct canonical identities remain
distinct absent an accepted linkage authority. Its selection directory stays
within V6's minimum Domain/Principal fields. A later implementation Spec must
freeze the fail-closed server mapping for these Product capabilities. Existing
Reader/Coordinator roles receive no automatic mapping or new Grant.

The current TASK owner naturally receives the bounded Instance access and
Transition authority needed for that Visit; membership is not implicitly
required. Historical participation and Domain-local views remain governed by
accepted query contracts.

### 5.3 Definition and publication

A Workflow Definition is Domain-owned. Each Version has lifecycle:

~~~text
DRAFT      -> PUBLISHED
PUBLISHED  -> DEPRECATED | REVOKED
DEPRECATED -> REVOKED
~~~

Here DRAFT is a Definition-Version lifecycle state, never a
VISIT_ACTIVATION_V1 node kind.

- DRAFT versions may be edited and validated but create no normal Instance.
- PUBLISHED versions create Instances and are immutable.
- DEPRECATED versions create no new Instance; existing Instances may continue
  only through their explicit semantic model and accepted lifecycle rules.
- REVOKED versions permit no ordinary create or Transition; only separately
  authorized recovery/containment applies.

Publication freezes semantic model ID, graph, node/Transition identity,
ordering, owner references, Context and Submission schemas, validator
identity, digest inputs, and complete canonical bytes. Canonicalization is
JCS plus SHA-256. Published/Deprecated/Revoked business meaning and child
graph rows are immutable.

VISIT_ACTIVATION_V1 graph rules:

- node kinds are exactly TASK and TERMINAL;
- exactly one entry TASK exists;
- every TASK has exactly one primary ADVANCE Transition;
- primary ADVANCE edges form one acyclic deterministic path ending at a
  TERMINAL node;
- a primary ADVANCE into the success terminal remains ADVANCE;
- RETURN targets only an earlier reachable TASK and never reopens an old
  Visit;
- non-primary TERMINATE targets only a TERMINAL node;
- TERMINAL has no owner and no outgoing edge;
- all nodes are reachable from the entry TASK;
- order is publication structure for validating earlier RETURN and forward
  progress, not runtime work identity.

Dynamic forward branching, multiple primary successors, parallelism, DRAFT
or NORMAL new nodes, WAIT nodes, or metadata-driven execution are invalid.

### 5.4 TASK owner resolution and Principal boundary

Each TASK carries one owner reference from the retained closed set:

~~~text
WORKFLOW_CREATOR
DOMAIN_OWNER
FIXED_PRINCIPAL
~~~

On every Visit entry the server resolves the reference in the locked
transaction and snapshots one canonical principalId. The resolved Principal
must exist, be enabled, and have canonical type HUMAN or AGENT. A later
Principal, Owner, Domain, or Definition change does not rewrite the Visit.

SERVICE remains valid for authentication and separately accepted service
commands. It is authentication-only with respect to TASK ownership. It cannot
be a TASK owner, cannot be auto-converted to AGENT, cannot receive either
activation kind, and cannot enter Scheduler-visible work. Caller bodies,
display names, roles, scopes, service credentials, and external Agent mapping
cannot override canonical Principal type.

The descriptive INSTANCE_INPUT_PRINCIPAL/ContextPrincipal behavior in current
code is not added to this Architecture. A future authority would be required
before it could become a new-model owner reference.

### 5.5 Context and Submission

Workflow Context is bounded, schema-validated input for one Instance. Context
and Submission payloads are JSON; the kernel validates shape and digest, not
business truth.

For VISIT_ACTIVATION_V1:

- initial Context Revision is created with the Instance;
- it forms immutable Revision number 1 with no predecessor;
- post-create Context mutation is not authorized by V6 or this Architecture;
- currentContextRevisionId remains that immutable fact unless a later lawful
  authority explicitly changes the rule.

LEGACY_V1 retains its historical immutable Revision chain and creator-on-DRAFT
mutation only in permitted Legacy operation modes. A new-flow command cannot
invoke Legacy revise-only or revise-and-transition semantics.

Submission is the immutable stage-delivery primitive. At most one committed
Submission exists per source Visit. It is server-bound to the current Context
Revision, author, Transition, payload digest, and Instance. Large resources
use bounded URI/digest references. No cross-Instance reference is valid.

RETURN preserves the accepted required fields and integrity checks:
rootCauseNodeVisitId and every relatedSubmissionId must already exist in and
be readable within the same Instance; reasonCode and reason are required.
Invalid references retain accepted HTTP 422 invalid_return_references with
aggregated details where that HTTP contract applies.

### 5.6 Instance and Node Visit

An Instance is one execution of one immutable Definition Version in one
Domain. Its domainId, definitionVersionId, semanticModelId,
createdByPrincipalId, and creation identity never change.

The current Context, current Visit, active activation, and workflow state
version are projections over authoritative facts. No normal physical DELETE
exists.

A Node Visit is one immutable entry into one definition node:

~~~text
nodeVisitId
workflowInstanceId
nodeId
visitNumber
resolved ownerPrincipalId or null for TERMINAL
enteredByTransitionId or bounded governance cause
createdAt
~~~

nodeVisitId is the sole runtime work identity. nodeId is reusable template
structure. Each RETURN, repeated entry, administrative move, or planned
successor entry creates a distinct Visit and monotonically valid per-node
visitNumber. No old Visit is reopened, retagged current, reassigned, updated,
deleted, or given an authoritative exitedAt/open/closed flag.

Activation, work item, dispatch, attempt idempotency, wake, reconciliation,
repair, and delivery bind nodeVisitId. They never bind nodeKey, display name,
business label, metadata, DRAFT, test_env_deploy, ops-lock, environment, or a
global switch.

### 5.7 Canonical activation facts and projections

Every active non-terminal VISIT_ACTIVATION_V1 TASK Visit has exactly one
canonical activation:

~~~text
resolved HUMAN owner -> HUMAN_WORK_ITEM
resolved AGENT owner -> DISPATCH_INTENT
TERMINAL             -> no activation
~~~

The activation kind derives only from the canonical resolved Principal type.
The caller never supplies it.

The logical immutable fact set is:

1. ActivationCreated — activation identity, Instance, nodeVisitId, kind,
   ownerPrincipalId, server activation timestamp, and, for Dispatch Intent,
   the same timestamp as initial nextEligibleAt.
2. ActivationClosed — at most one closure for the activation, linked to the
   closing command/Event, lifecycle reason, and server time.
3. DispatchEligibilityChanged — each later concrete nextEligibleAt change,
   linked to the exact Dispatch Intent, nodeVisitId, attempt/wake identity,
   command/Event, previous value, new value, and non-sensitive cause class.

These are logical Architecture facts; a later implementation Spec chooses
their tables and wire encoding. It must preserve their immutability,
uniqueness, referential integrity, and replay order.

Active/closed and current nextEligibleAt are rebuildable projections:

~~~text
activation active = ActivationCreated exists and no ActivationClosed exists
current nextEligibleAt = initial value followed by the ordered accepted
                         DispatchEligibilityChanged facts
~~~

Exactly-one means an enforceable mutual-exclusion invariant, not a best-effort
application query. One nodeVisitId cannot have both Human and Dispatch
activation facts, cannot have two of either kind, and cannot be active after
closure.

The minimum Scheduler-facing Dispatch Intent projection is exactly:

~~~text
dispatchIntentId
nodeVisitId
workflowInstanceId
ownerPrincipalId
nextEligibleAt
createdAt
updatedAt
~~~

It excludes Context/title, task label, definition/node/business names or
keys, Submission/history, EventData, Assistance content, credentials, tokens,
Receipt/audit payloads, Transition options, and metadata.

### 5.8 Canonical activation timestamp and wait semantics

The transaction that creates an Agent-owned TASK Visit generates one
canonical server-authored activation timestamp and persists it in that same
transaction both as activation creation time and initial nextEligibleAt.

The initial value:

- is required and non-null;
- is never client-authored;
- is never filled after commit;
- does not require equality to a physical or true commit instant;
- must not weaken Visit/activation atomicity merely to obtain commit time.

The only Scheduler-facing due predicate is:

~~~text
Dispatch Intent is active
AND nextEligibleAt <= authoritative server now
~~~

No status, blocked reason, lease, retry flag, DRAFT convention, node key,
metadata, timer node, event node, or external label is another wait predicate.

An authorized early wake command binds the exact active nodeVisitId and
activation and sets nextEligibleAt to server now. It cannot choose another
timestamp, create an activation, mutate node/owner, perform a Transition, or
start an Agent. Stale/closed/current-mismatch wake is a no-workflow-side-effect
result with durable receipt/audit semantics selected by the child Spec.

After a deterministic non-execution outcome, an independently authorized
Scheduler policy may request one concrete future nextEligibleAt through the
bounded command. The server validates actor, current Visit/activation,
attempt identity, request identity, time bounds, and current state under its
accepted implementation Spec.

### 5.9 Command serialization and atomic closure

All state-changing commands use canonical request hashing, idempotency, fixed
lock order, current Instance serialization, and fail-closed authorization.
The retained lock order is CommandReceipt idempotency identity, then the
existing source WorkflowInstance row, then required Domain/Definition reads;
no command may invert it. A migration successor row does not pre-exist and is
inserted only after its source is locked and every preassigned identity is
validated.
A successful ordinary single-Instance state command increments that Instance's
workflowStateVersion exactly once and creates exactly one WorkflowEvent. The
Event may reference multiple immutable facts created by the same command; the
one-Event rule does not weaken activation fact identity. ONE_TIME_MIGRATE is
the only cross-Instance exception: it atomically creates one source migration
Event and version increment plus the successor's one initial Event and version
1, all linked to one completed command Receipt and audit envelope.

Create VISIT_ACTIVATION_V1 Instance atomically commits:

~~~text
Receipt ownership
Instance with explicit semantic model
Context Revision 1
entry TASK Visit
resolved Human Work Item or Dispatch Intent ActivationCreated
initial Dispatch nextEligibleAt when applicable
current projections and workflowStateVersion 1
INSTANCE_CREATED Event
required audit
completed Receipt
~~~

Any owner, type, timestamp, uniqueness, activation, Event, audit, Receipt, or
constraint failure commits none of those facts.

Normal Transition atomically commits:

~~~text
authorization by current TASK owner
expected workflowStateVersion and exact Transition validation
Submission when required
ActivationClosed for source TASK
new target Visit
ActivationCreated for target TASK, or none for TERMINAL
current projections and one version increment
one WORKFLOW_TRANSITION_COMMITTED Event
required audit
completed Receipt
~~~

ADVANCE uses only the primary Transition. RETURN uses a configured earlier
TASK, creates a new Visit and activation, and never reopens an old Visit.
Graph TERMINATE uses a configured non-primary edge to TERMINAL, requires its
accepted Submission, closes source activation, creates a new terminal Visit,
and creates no target activation.

Cancel preserves v0.3.2 exactly: Domain Owner invokes a graph-external command,
currentNodeVisitId remains the current Visit, no Visit is created, cancelled
metadata/version/Event/Receipt/audit commit atomically, ordinary flow is
blocked, and the current source activation is closed in that same transaction.

Archive preserves v0.3.2: only terminal/cancelled Instance, no Visit,
non-destructive metadata, one version/Event/Receipt/audit, immutable history.
An active activation at Archive time is invariant drift and fails closed.

ADMIN_EMERGENCY_OVERRIDE remains bounded to MOVE_TO_NODE and
TERMINATE_INSTANCE under accepted Admin Recovery authority:

- MOVE_TO_NODE closes source activation, creates one target TASK Visit and
  correct activation, increments once, and appends Event/Receipt/audit;
- TERMINATE_INSTANCE closes source activation, creates one terminal Visit and
  no target activation, increments once, and appends Event/Receipt/audit;
- neither modifies the old Visit or creates a business Submission.

REBUILD_PROJECTION does not change authoritative facts or state version. It
replays and validates all fact families, including activations, and updates
only corrupted projections with durable security audit.

### 5.10 Idempotency, attempts, timeout, and lease boundary

State commands retain:

~~~text
idempotency scope = authenticated principal plus idempotency key
request identity = JCS complete command envelope plus SHA-256
same key + same request = original outcome replay
same key + different request = conflict, no mutation
PROCESSING = not taken over
deterministic failure = stable completed outcome when child contract says so
infrastructure failure = transaction rollback
~~~

Create requires its idempotency identity and has no pre-existing Instance
version. Every state-changing command against an existing Instance also
requires the caller's expected workflowStateVersion and fails without
mutation on mismatch.

Activation/wake/eligibility/repair commands additionally include exact
nodeVisitId, activation identity, expected workflowStateVersion, and complete
operation meaning in the request hash.

An external execution attempt has a stable attemptId owned by the external
Scheduler/Dispatcher authority. Its Workflow-facing identity is
nodeVisitId plus attemptId plus exact request. If a response is lost and
commit status cannot be known, the result is outcome_unknown. Reconciliation
repeats the exact same Visit/attempt/request/idempotency identity. Blind retry
with a new identity is forbidden.

A resource lease is attempt-scoped external mutual exclusion. It is not
Workflow node syntax, Visit identity, activation identity, current status,
nextEligibleAt, or a second wait predicate. Lease acquisition/loss never
creates or closes a Visit/activation and never changes the Definition graph.

### 5.11 Activation-driven bounded delivery

The normal new-work path begins only from committed Dispatch Intent
ActivationCreated. The same state transaction must create either:

- a bounded durable delivery obligation linked one-to-one to that activation;
  or
- another implementation-Spec mechanism proving that committed activation is
  durably discoverable by the Scheduler without scanning Workflow Instances,
  Domain/global summaries, node keys, or metadata.

The bounded delivery subject carries only stable activation/Visit identifiers
and delivery-control metadata. It cannot become a general business event
platform or a second work authority. Delivery retry preserves the same
Dispatch Intent and nodeVisitId; it never creates a duplicate activation.

A dedicated queue/table poll over bounded activation delivery obligations is
activation-driven consumption, not a Workflow Instance discovery scan.
Exact transport, cursor, acknowledgement, and retry scheduling remain for the
implementation and external authorities.

Periodic scans are allowed only for Reconciler, Watchdog, or Repair:

- detect missing, duplicate, stuck, stale, or inconsistent activation/delivery;
- preserve the canonical nodeVisitId and activation identity;
- never branch on nodeKey/business/metadata;
- never dispatch healthy ordinary work;
- repair only through accepted idempotent audited commands;
- never fabricate a second activation.

### 5.12 Fact, Event, replay, and repair integrity

The retained immutable workflow fact families are:

~~~text
WorkflowContextRevision
NodeVisit
Submission
WorkflowEvent
~~~

v0.4.0 adds immutable activation and eligibility fact families described in
section 5.7. CommandReceipt identity/completion and required audit remain
durable command/accountability records.

For each Instance:

- eventSequence equals successful state-command version progression;
- every successful state command has one Event and one completed Receipt;
- every Event reference belongs to the same Instance and exact semantic model;
- every Visit belongs to the Instance fixed Definition Version;
- every Submission binds its source Visit and current Context;
- every activation binds exactly one eligible TASK Visit;
- every closure/update follows an existing activation in command/Event order;
- projections can be rebuilt from facts without inventing facts;
- unknown Event/fact versions, gaps, duplicate command linkage, digest failure,
  orphan fact, semantic-model mismatch, or impossible activation cardinality
  fails closed.

Replay never recreates immutable facts from Events. It validates Events
against facts and recomputes projections. Repair cannot edit or delete facts.
A missing activation may be restored only by a separately accepted repair
command using the original Visit-entry Event's server activation timestamp and
proving no competing activation. If the original timestamp or uniqueness
cannot be proven, repair fails closed and requires an exact plan.

No global Event cursor or generic delivery platform is created here.

### 5.13 One-way cutover and Legacy modes

The later implementation/cutover authority defines one auditable immutable
barrier coordinate. Once lawfully applied, it cannot move, clear, or change
meaning. The caller-routing decision at that coordinate is atomic. After it,
every new-traffic Definition Version and Instance must explicitly use
VISIT_ACTIVATION_V1 or fail closed.

Post-barrier prohibitions apply to new traffic and new Legacy identity:

- no new LEGACY_V1 Definition Version, Definition clone/publication, or
  Instance;
- no route or fallback from new traffic to Legacy;
- no silent fallback, reverse routing, dual authority, or permanent dual
  track;
- no unknown/model-less Instance.

An Instance that existed before the barrier remains explicitly LEGACY_V1 and
may operate only in:

~~~text
DRAIN
ONE_TIME_MIGRATE
MANUALLY_TERMINATE
HISTORICAL_REPLAY
~~~

DRAIN may append only the accepted Legacy Visit, Submission, Event,
Receipt/audit, Context facts where the accepted Legacy flow requires them, and
other necessary accepted Legacy facts to finish that already-existing
Instance. It creates no new Legacy Definition or Instance identity and no
new-model activation. Rejecting a valid bounded drain solely because it
appends facts after the barrier is non-conforming.

HISTORICAL_REPLAY reads/reconstructs only; it writes and schedules nothing.
MANUALLY_TERMINATE uses accepted bounded governance, preserves facts, and
cannot reactivate or create new Legacy identity.

ONE_TIME_MIGRATE is exact-plan, idempotent, fail-closed, append-only, and
separately production-authorized. Because Definition Version and Instance
semantic-model identity are immutable, migration creates one preassigned
VISIT_ACTIVATION_V1 successor Instance with one target Visit/activation and an
immutable mapping from the source Legacy Instance. It atomically:

- validates a plan that preassigns source expected version, successor Instance
  ID and Definition Version, Context digest, target nodeVisitId, target node,
  resolved owner, semantic model, and idempotency identity;

- validates and closes/marks complete the exact source Legacy current work
  under its accepted migration plan;
- creates the successor Instance, Context fact, target TASK or terminal Visit,
  and correct activation;
- records source-to-successor mapping, exactly one source migration Event and
  version increment, exactly one successor initial Event/version 1, one
  Receipt, audit, and command linkage;
- preserves every source Legacy Definition, Instance, Context, Visit,
  Submission, Event, and attribution byte-for-meaning;
- makes exact rerun a no-write replay.

Migration does not mutate a Legacy Instance into another semantic model,
create general reassignment, accept runtime OLD/NEW parameters, or rewrite
history.

Rollback before facts exist may revert candidate code/config. After
VISIT_ACTIVATION_V1 facts exist, rollback is containment: pause intake and
delivery, preserve facts, revoke/disable compromised access, and repair under
accepted authority. It never routes new traffic to Legacy, deletes/relabels
activation, or fabricates Legacy identity.

### 5.14 Compatibility surfaces

Existing Domain/global lists, assigned-to-me, creator-owned drafts, details,
timeline, Submission history, current HTTP envelope, stable error codes,
pagination, and control-plane surfaces retain their accepted compatibility
meaning until separately superseded.

They are diagnostic, Domain/participant worklist, Legacy drain/history, or
compatibility surfaces. They are not Dispatch Intent feeds, permission grants,
or substitutes for ActivationCreated. In particular:

- GLOBAL_WORKFLOW_READER and legacy GLOBAL_WORKFLOW_COORDINATOR remain only
  the current global-list route gate;
- neither role automatically reaches Dispatch Intents;
- assigned-to-me may continue to serve Human/Legacy compatibility but a new
  Scheduler/Dispatcher cannot use it for normal work discovery;
- no existing list field, dispatchable projection, node key, or current
  assignee row becomes canonical activation.

Future wire/schema changes require an accepted implementation-authorizing
Spec. Compatibility does not require preserving a Legacy route as a normal
new-model dispatch mechanism.

### 5.15 Identity, successor, and child sequencing

Auth-service remains global identity/authentication authority. svc-workflow
stores bounded local projections and bindings. Verified token.sub is actor;
act.sub, client, Feishu, body, display name, scope, and self-report cannot
substitute.

Accepted Agent self-projection remains direct-token token.sub projection only
and creates no Domain/global binding. Domain Owner membership management
remains Domain-local. New TASK entry requires an existing enabled canonical
Human/Agent projection; it cannot silently self-provision from request data.

SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1 and the V6 trusted-fleet
exception remain exact Legacy exceptions. They do not authorize ordinary
reassignment, new-model same-Visit owner change, generic migration, or a
caller-parameterized successor API.

SVC_WORKFLOW_INVALID_RETURN_REFERENCES_HTTP_422_V1 retains its accepted
wire/error behavior. It does not authorize DRAFT/NORMAL or alter new-model
Transition selection.

Architecture acceptance, implementation-Spec acceptance, code merge, schema
migration readiness, external Scheduler readiness, barrier selection,
production cutover, and Legacy migration/apply are distinct gates. None
implies the next.

## 6. Decisions

### DEC-ARCH-001 — Whole-supersede v0.3.1

- Decision owner: mayf3 through accepted V6 and later Architecture acceptance.
- Decision: replace v0.3.1 as a whole while fully restating its reusable core.
- Rejected alternatives: partial supersession; reader-side composition;
  silently editing v0.3.1.
- Reason: the V6 node and activation model changes primary Architecture
  semantics while the reusable kernel must remain readable in one authority.
- Owner input remaining: none.

### DEC-ARCH-002 — Explicit immutable semantic models

- Decision owner: mayf3 through V6.
- Decision: use LEGACY_V1 and VISIT_ACTIVATION_V1 logical IDs on Definition
  Version and Instance, never inference.
- Rejected alternatives: graph/metadata/nodeKey/global-flag inference; aliasing
  descriptive Minimal value 2 to V6 without semantic equality.
- Reason: stable replay, routing, and migration require identity that cannot
  change when graph or deployment state changes.
- Owner input remaining: none.

### DEC-ARCH-003 — Preserve the deterministic serial kernel

- Decision owner: mayf3 through V6.
- Decision: one current Visit, one primary forward path, configured RETURN and
  TERMINATE.
- Rejected alternatives: multiple primary ADVANCE, dynamic branching,
  parallelism, claim/pull.
- Reason: V6 preserves one deterministic serial route and excludes those
  product concepts.
- Owner input remaining: none.

### DEC-ARCH-004 — Closed TASK/TERMINAL model and owner set

- Decision owner: mayf3 through V6.
- Decision: VISIT_ACTIVATION_V1 nodes are TASK or TERMINAL; TASK owner reference
  is Creator, Domain Owner, or Fixed Principal and resolves Human/Agent only.
- Rejected alternatives: DRAFT/NORMAL new nodes, typed TASK variants, SERVICE
  ownership, ContextPrincipal without new authority.
- Reason: the V6 closed sets are deliberate product and authorization
  boundaries.
- Owner input remaining: none.

### DEC-ARCH-005 — New-flow Context is create-time immutable

- Decision owner: mayf3 through V6.
- Decision: VISIT_ACTIVATION_V1 creates Context once; Legacy revision rules
  remain only for permitted Legacy modes.
- Rejected alternatives: importing Legacy DRAFT editing into new traffic.
- Reason: new traffic starts at one executable TASK activation and does not
  retain a definition-bound editing phase.
- Owner input remaining: none.

### DEC-ARCH-006 — Node Visit is sole work identity

- Decision owner: mayf3 through V6.
- Decision: every entry has a new immutable nodeVisitId; all work operations
  bind it.
- Rejected alternatives: nodeId/nodeKey/business/environment identity; Visit
  reopen/reassign.
- Reason: re-entry can repeat every weaker label while nodeVisitId remains
  unique and immutable.
- Owner input remaining: none.

### DEC-ARCH-007 — Immutable canonical activation facts

- Decision owner: mayf3 through V6.
- Decision: exactly one Human Work Item or Dispatch Intent per active TASK
  Visit, with immutable creation/closure/eligibility facts and projections.
- Rejected alternatives: read-time synthetic work, mutable fact overwrite,
  dual/duplicate/no activation.
- Reason: atomic closure, delivery, replay, and repair need one durable source
  of activation truth.
- Owner input remaining: none.

### DEC-ARCH-008 — Same-transaction initial timestamp

- Decision owner: mayf3 through V6.
- Decision: server activation timestamp is persisted as initial
  nextEligibleAt in the Visit/activation transaction, without physical commit
  instant equality.
- Rejected alternatives: client time, post-commit fill, split transaction,
  commit-instant atomicity weakening.
- Reason: one server-authored timestamp in the activation transaction closes
  the null and split-brain window without pretending to know commit instant.
- Owner input remaining: none.

### DEC-ARCH-009 — One state command, one atomic closure

- Decision owner: mayf3 through V6 and retained serial kernel.
- Decision: Visit, source activation closure, target activation, facts,
  projection/version, Event, Receipt, and audit commit all-or-nothing.
- Rejected alternatives: eventual activation, partial workflow commit,
  best-effort audit.
- Reason: a visible state change without its activation and evidence would be
  unrecoverably ambiguous.
- Owner input remaining: none.

### DEC-ARCH-010 — Preserve v0.3.2 Cancel/Archive

- Decision owner: existing effective Architecture plus V6.
- Decision: Cancel keeps current Visit/no new Visit and now closes activation
  atomically; Archive remains non-destructive.
- Rejected alternatives: Cancel as TERMINATE edge; new Visit on Cancel;
  rewriting v0.3.2.
- Reason: v0.3.2 already owns Cancel/Archive semantics and V6 only requires
  their current-work activation closure.
- Owner input remaining: none.

### DEC-ARCH-011 — Singular wait and attempt boundary

- Decision owner: mayf3 through V6.
- Decision: due is active plus nextEligibleAt; attempts/leases remain external
  and bind nodeVisitId plus attemptId.
- Rejected alternatives: second wait field/status/reason; lease as Workflow
  syntax; blind new-identity retry.
- Reason: nextEligibleAt is the singular Workflow wait fact while delivery
  attempts are external, recoverable work.
- Owner input remaining: none.

### DEC-ARCH-012 — Activation-driven bounded delivery

- Decision owner: mayf3 through V6.
- Decision: committed activation drives normal delivery; scans are
  Reconciler/Watchdog/Repair only.
- Rejected alternatives: global/Instance/nodeKey periodic discovery; generic
  event platform.
- Reason: canonical activations already identify due work and permit explicit
  concurrency bounds.
- Owner input remaining: none.

### DEC-ARCH-013 — Rebuild from immutable facts

- Decision owner: retained serial-kernel Architecture.
- Decision: validate facts/Event relations and rebuild projections without
  creating or rewriting facts.
- Rejected alternatives: Event-only fact recreation; best-effort repair;
  silent cardinality healing.
- Reason: immutable workflow facts remain authoritative and corruption must be
  visible rather than normalized away.
- Owner input remaining: none.

### DEC-ARCH-014 — One-way new-traffic barrier

- Decision owner: mayf3 through V6.
- Decision: post-barrier new traffic is VISIT_ACTIVATION_V1 or fail closed;
  pre-barrier Legacy may bounded-drain append necessary facts.
- Rejected alternatives: new Legacy identity, fallback, dual authority,
  categorical rejection of valid drain facts.
- Reason: V6 requires deterministic post-barrier routing while expressly
  preserving bounded append-only completion evidence for older work.
- Owner input remaining: none.

### DEC-ARCH-015 — Migration creates an immutable successor Instance

- Decision owner: entailed by V6 migration plus immutable Definition/Instance
  semantic-model identity.
- Decision: exact migration maps immutable Legacy source to one new-model
  successor Instance/Visit/activation.
- Rejected alternatives: mutating source semantic model/Definition; history
  rewrite; general migration/reassignment.
- Reason: both semantic identity and source history are immutable; a uniquely
  mapped successor is the only deterministic migration form.
- Owner input remaining: none.

### DEC-ARCH-016 — Compatibility and external authority stay bounded

- Decision owner: mayf3 for local Architecture; external repository for its
  own design.
- Decision: lists/routes remain compatibility only; external periodic recovery
  is Reconciler-only and Scheduler management is separate.
- Rejected alternatives: implicit permission grant, old list as dispatch feed,
  local dsh consumer design or PR lifecycle command.
- Reason: compatibility and cross-repository ownership are frozen authority
  boundaries, not implementation conveniences.
- Owner input remaining: none.

## 7. Normative Contracts

### CTR-ARCH-001 — Lifecycle and authority

This proposal MUST remain non-authoritative while proposed/unmerged, MUST
whole-supersede v0.3.1 only through a later atomic accepted lifecycle
transaction, and MUST NOT authorize implementation or production apply.

### CTR-ARCH-002 — Explicit semantic-model discriminator

Every Definition Version and Instance MUST carry an immutable explicit
LEGACY_V1 or VISIT_ACTIVATION_V1 identity with enforced equality. Unknown,
missing, mismatched, inferred, or caller-overridden identity MUST fail closed.

### CTR-ARCH-003 — Descriptive Minimal non-reuse

Current descriptive semantic model value 2 MUST NOT be treated as
VISIT_ACTIVATION_V1 unless a later accepted authority proves complete semantic
equality or assigns a distinct non-conflicting encoding. Existing descriptive
rows MUST be inventoried and contained by the implementation/migration Spec.

### CTR-ARCH-004 — Product and Domain boundary

svc-workflow MUST own only the serial workflow kernel and MUST preserve
single-Domain Definition/Instance ownership, unique effective Domain Owner,
strict normal cross-Domain isolation, PostgreSQL as the sole formal workflow
database, and exclusive svc-workflow workflow-table writes.
GLOBAL_SCHEDULER_READ and GLOBAL_DOMAIN_ADMIN MUST remain independent and
bounded exactly to the V6 Scheduler-read and Domain-administration subjects;
neither grants workflow execution/content or implies the other, and no existing
compatibility role receives automatic mapping.

### CTR-ARCH-005 — Definition lifecycle and immutability

Definition lifecycle, publication digest, semantic model, graph, schemas,
owner refs, validator, and published child rows MUST satisfy section 5.3 and
remain immutable after publication.

### CTR-ARCH-006 — VISIT_ACTIVATION_V1 graph

New-model graphs MUST use exactly TASK or TERMINAL, one entry TASK, one
primary ADVANCE per TASK, one deterministic acyclic primary path, earlier TASK
RETURN, configured terminal TERMINATE, reachable nodes, and no excluded
orchestration concepts.

### CTR-ARCH-007 — Owner resolution and SERVICE rejection

TASK entry MUST resolve exactly one enabled canonical HUMAN or AGENT through
the closed owner-reference set. SERVICE remains authentication-only and MUST
NOT own, activate, schedule, or be converted.

### CTR-ARCH-008 — Context and Submission integrity

New-model Context MUST be create-time immutable. Legacy revisions remain only
within accepted Legacy modes. Submission and RETURN references MUST preserve
same-Instance binding, immutability, schema/digest, and accepted RETURN 422
semantics.

### CTR-ARCH-009 — Instance and Visit immutability

Instance identity fields and semantic model MUST be immutable. Each node entry
MUST create a distinct immutable nodeVisitId; old Visits MUST NOT reopen,
reassign, mutate, delete, or become identified by node/business labels.

### CTR-ARCH-010 — Exactly-one activation

Every active new-model TASK Visit MUST have exactly one mutually exclusive
Human Work Item or Dispatch Intent derived from owner type; TERMINAL MUST have
none. Cardinality MUST be mechanically enforceable and fail closed. Activation
creation MUST NOT itself start Human or Agent execution.

### CTR-ARCH-011 — Immutable activation lifecycle

Activation creation, closure, and eligibility-change facts MUST be immutable,
ordered, command/Event-linked, and rebuildable into active/closed and current
nextEligibleAt projections. A closed activation MUST NOT reopen.

### CTR-ARCH-012 — Initial nextEligibleAt

The Agent TASK Visit transaction MUST generate one server activation timestamp
and persist it atomically as initial nextEligibleAt. It MUST NOT accept a
client value, post-commit fill, or physical commit-instant requirement that
weakens atomicity.

### CTR-ARCH-013 — Singular due predicate and allowlist

Scheduler eligibility MUST be exactly active Dispatch Intent with
nextEligibleAt at or before authoritative now. The Scheduler projection MUST
contain only section 5.7 fields and MUST exclude content, business keys, and
every alternate wait predicate.

### CTR-ARCH-014 — Create atomic closure

New-model Instance, Context 1, entry Visit, correct activation/initial time,
projections/version 1, one Event, Receipt, and audit MUST commit in one
transaction or all commit zero.

### CTR-ARCH-015 — Transition authority and closure

Only the current TASK owner MAY execute a normal Transition. Submission,
source activation closure, target Visit/activation or terminal none,
projection/version, one Event, Receipt, and audit MUST commit atomically.

### CTR-ARCH-016 — RETURN and re-entry

RETURN MUST target a configured earlier TASK, validate accepted same-Instance
references, close source activation, create a distinct target Visit and
activation, and never reopen an old Visit.

### CTR-ARCH-017 — Graph TERMINATE

Graph TERMINATE MUST use a configured non-primary edge and accepted reason
Submission, close source activation, create one terminal Visit, create no
target activation, and commit the complete command closure atomically.

### CTR-ARCH-018 — Cancel and Archive

Cancel MUST preserve current Visit and create no Visit while atomically
closing source activation and committing v0.3.2 cancellation facts. Archive
MUST remain non-destructive and MUST fail closed if an active activation
contradicts terminal/cancelled state.

### CTR-ARCH-019 — Administrative move and manual terminate

Accepted administrative MOVE_TO_NODE and TERMINATE_INSTANCE MUST preserve old
Visit immutability, close source activation, create the correct new TASK or
terminal Visit/activation outcome, and atomically commit Event/Receipt/audit
and one version increment.

### CTR-ARCH-020 — State-command version and Event invariant

State-changing commands MUST acquire CommandReceipt identity before source
Instance and required Domain/Definition reads and MUST NOT invert that order.
Every successful ordinary single-Instance workflow/activation command MUST
increment workflowStateVersion once and create exactly one same-Instance
WorkflowEvent linked to one completed Receipt. ONE_TIME_MIGRATE MUST create
exactly one source migration Event/version increment and one successor initial
Event/version 1 linked to its one Receipt. Infrastructure failure MUST roll
back the complete transaction.

### CTR-ARCH-021 — Idempotency and unknown outcome

Commands MUST use authenticated-principal/idempotency-key identity and JCS
complete-request SHA-256. Same request MUST replay, changed request MUST
conflict, PROCESSING MUST not be taken over, and outcome_unknown MUST reconcile
the exact same request identity.

### CTR-ARCH-022 — Wake and later eligibility update

Authorized wake MUST bind the exact current Visit/activation and set server
now only. A later future time MAY be set only through an accepted,
authenticated, attempt-bound command. Neither path may mutate node/owner,
create activation, or start execution.

### CTR-ARCH-023 — Attempt and lease separation

Workflow-facing attempt identity MUST be nodeVisitId plus stable attemptId plus
exact request. Resource leases MUST remain external attempt-scoped mutual
exclusion and MUST NOT become Workflow identity, graph, activation, or wait.

### CTR-ARCH-024 — Activation-driven bounded delivery

Committed Dispatch Intent activation MUST create or prove a durable bounded
delivery obligation without Workflow Instance/global/business-key discovery.
Delivery retry MUST preserve the same activation and nodeVisitId.

### CTR-ARCH-025 — Recovery-only periodic scans

Periodic scans MAY run only as Reconciler, Watchdog, or Repair, MUST detect
anomalies rather than ordinary work, MUST preserve Visit/activation identity,
MUST avoid business keys, and MAY invoke idempotent audited repair only for a
deterministically repairable anomaly; ambiguous/competing facts MUST be
contained and fail closed.

### CTR-ARCH-026 — Fact/Event integrity and rebuild

All workflow and activation facts, digests, references, sequences, semantic
models, command links, and cardinalities MUST validate before projection
publication. Rebuild MUST create/rewrite no fact and unknown/inconsistent
input MUST fail closed.

### CTR-ARCH-027 — Missing-activation repair

A missing activation MAY be repaired only under separate accepted authority
using the original Visit-entry activation timestamp, proving exact owner,
kind, Visit currentness, and absence of a competitor. Otherwise repair MUST
commit zero.

### CTR-ARCH-028 — One-way new-traffic barrier

The later cutover authority MUST apply one auditable immutable caller-atomic
barrier that cannot move or clear. Post-barrier new traffic MUST explicitly
create VISIT_ACTIVATION_V1 or fail closed. It MUST NOT create Legacy identity,
route/fallback to Legacy, silently fallback, reverse-route, or create permanent
dual authority.

### CTR-ARCH-029 — Bounded Legacy drain

Pre-barrier LEGACY_V1 Instances MAY append only necessary accepted Legacy
facts in bounded DRAIN. They MUST create no new Legacy Definition/Instance,
new-model activation, unrelated write, or historical rewrite.

### CTR-ARCH-030 — Exact one-time migration

ONE_TIME_MIGRATE MUST be plan-bound, separately authorized, idempotent, and
append-only. The plan MUST preassign the source expected version and exact
successor/Definition/Context/target nodeVisitId/owner identities. It MUST create
exactly one explicit new-model successor Instance/Visit/activation and
immutable source mapping without mutating Legacy identity/history; drift or
ambiguity MUST commit zero.

### CTR-ARCH-031 — Manual terminate, replay, and rollback

Legacy MANUALLY_TERMINATE MUST use accepted bounded governance.
HISTORICAL_REPLAY MUST write/schedule nothing. Rollback after new facts exist
MUST be containment and MUST NOT create Legacy fallback or delete/relabel
facts.

### CTR-ARCH-032 — Global Reader compatibility only

Current Global Reader/Coordinator route/role/wire/error/pagination meaning
MUST remain compatibility-only. It MUST NOT grant Dispatch Intent access,
Domain-admin/write, or serve as the normal Scheduler feed.

### CTR-ARCH-033 — Existing list/worklist/HTTP compatibility

Existing Domain/global lists, assigned-to-me, creator drafts, details,
timeline, Submission history, HTTP envelope, error codes, and cursor behavior
MUST remain stable until separately superseded. New-model dispatch MUST NOT
use them for ordinary discovery.

### CTR-ARCH-034 — Principal and self-projection relationship

Verified token.sub MUST remain actor. Accepted self-projection MUST create
only a bounded local identity projection and no owner/global authority.
New TASK owner resolution MUST use an existing enabled canonical Human/Agent.

### CTR-ARCH-035 — Successor and RETURN child boundaries

Accepted exact Principal successor and RETURN 422 Specs MUST retain their
bounded meanings and MUST NOT imply general reassignment, new-model authority,
DRAFT/NORMAL reuse, or changed Transition selection.

### CTR-ARCH-036 — Legacy contract reconciliation

Legacy Definition/Create/Transition/Query/Storage/Admin/Import/Auth/HTTP
contracts MUST remain historical/compatibility authority in their declared
scope. They MUST NOT authorize new-model code; a later implementation Spec
MUST explicitly map every changed and retained contract.

### CTR-ARCH-037 — External interoperability ownership

svc-workflow interoperability MUST require external periodic recovery to be
Reconciler-only and Scheduler management separate from normal dispatch. This
Architecture MUST define no dsh consumer design or external PR lifecycle
action.

### CTR-ARCH-038 — Authority and execution sequencing

Implementation MAY begin only from a base containing accepted V6, accepted
v0.4.0, and an independently accepted implementation Spec with contracts
authority. Architecture acceptance, code, schema, external readiness,
cutover, and production apply MUST remain distinct gates.

### CTR-ARCH-039 — Audit, security, and publication barriers

Successful and authenticated-denied protected activation/read/write/repair
operations MUST have durable non-sensitive audit. Protected read audit MUST
precede publication; protected write audit MUST be atomic; revoke/disable and
unavailable authorization/audit MUST fail closed. Required audit MUST be
retained for exactly 365 days. This Architecture authorizes no new audit-read
or external-export surface.

### CTR-ARCH-040 — No hidden expansion

No child Spec, API, schema, migration, code, test, runtime setting, or external
integration MAY add a node/wait/lease/product concept, cross-repository
permission, trust expansion, or production coordinate absent from V6 and this
Architecture without a lawful higher-authority change.

## 8. Acceptance

Every Acceptance below has exactly one owning Contract. Review evidence MAY be
collected by a fresh Architecture reviewer at this proposal Head; executable
evidence belongs to the later accepted implementation Spec and its candidate.
An Acceptance is not production authorization.

### ACC-ARCH-001 — Lifecycle and authority fields

- Contracts: CTR-ARCH-001
- Method: parse frontmatter and lifecycle statements at the exact proposal Head.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: proposed Architecture, whole supersession of v0.3.1, V6 parent, and both implementation and production authority set to none.
- Required evidence: parsed fields plus exact-path diff against the pinned base.
- Failure condition: any accepted claim, partial supersession, authority grant, or ancestor/map mutation.

### ACC-ARCH-002 — Semantic-model discriminator

- Contracts: CTR-ARCH-002
- Method: Architecture model review plus later persistence/constraint tests on DefinitionVersion and Instance.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: both carry explicit immutable LEGACY_V1 or VISIT_ACTIVATION_V1 and equality is enforced.
- Required evidence: model mapping, schema constraints, and negative mutation/mismatch cases.
- Failure condition: missing, mutable, mismatched, or inferred semantic identity.

### ACC-ARCH-003 — No silent Minimal reuse

- Contracts: CTR-ARCH-003
- Method: compare accepted authority with descriptive base source and later migration mapping.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: the current numeric Minimal value is inventoried and contained, never silently treated as VISIT_ACTIVATION_V1.
- Required evidence: source observation, data inventory plan, and negative aliasing test.
- Failure condition: numeric or behavioral coincidence is used as authority for new traffic.

### ACC-ARCH-004 — Product and Domain boundary

- Contracts: CTR-ARCH-004
- Method: Architecture dependency review and later Domain authorization tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: Workflow Product owns Domains; enabled Domain and role authorization are required; cross-Domain references fail; Scheduler-read and Domain-admin capabilities remain independent, bounded, and unmapped from legacy roles by default.
- Required evidence: authority/storage graph and allow/deny/cross-Domain/capability non-implication test matrix.
- Failure condition: orphan Domain, inactive-Domain execution, cross-Domain acceptance, non-PostgreSQL or external write authority, capability composition, or automatic legacy-role grant.

### ACC-ARCH-005 — Definition lifecycle

- Contracts: CTR-ARCH-005
- Method: lifecycle model inspection plus later publication and immutability tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: Definition-Version DRAFT is editable; PUBLISHED is immutable and may create Instances; DEPRECATED and REVOKED reject new creation; REVOKED blocks ordinary commands; new-model nodes never reuse Legacy DRAFT/NORMAL.
- Required evidence: DRAFT/PUBLISHED/DEPRECATED/REVOKED state table, digest proof, and positive/negative lifecycle cases.
- Failure condition: mutable published content, creation from DEPRECATED/REVOKED, ordinary REVOKED command, or Legacy node-lifecycle reuse for new traffic.

### ACC-ARCH-006 — Graph validity

- Contracts: CTR-ARCH-006
- Method: graph-validator review plus later invalid-graph fixtures.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: only TASK and TERMINAL, valid entry, one primary ADVANCE per TASK, optional RETURN, configured TERMINATE, and no wait node.
- Required evidence: validator rules and fixtures for every allowed and rejected edge shape.
- Failure condition: ambiguous ADVANCE, invalid RETURN, unconfigured TERMINATE, or added node concept.

### ACC-ARCH-007 — Owner and SERVICE rules

- Contracts: CTR-ARCH-007
- Method: definition publication review plus later principal-resolution authorization tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: the closed owner-reference set resolves to one enabled Human or Agent; SERVICE is rejected for work ownership and remains auth-only.
- Required evidence: resolution matrix covering enabled, disabled, missing, ambiguous, and SERVICE principals.
- Failure condition: unresolved/disabled owner proceeds, SERVICE owns work, or the owner-reference set expands.

### ACC-ARCH-008 — Context and Submission integrity

- Contracts: CTR-ARCH-008
- Method: command model review plus later mutation and reference-integrity tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: new Context is immutable, Submission is immutable and Visit-scoped, and all references remain in one Instance.
- Required evidence: field immutability map and negative cross-Instance/mutation cases.
- Failure condition: Context mutation, Submission reuse, or cross-Instance linkage succeeds.

### ACC-ARCH-009 — Instance and Visit immutability

- Contracts: CTR-ARCH-009
- Method: state-transition review plus later persistence invariant tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: Instance pins one ACTIVE DefinitionVersion and semantic model; each entry or re-entry creates a distinct immutable nodeVisitId.
- Required evidence: creation/re-entry traces and immutability constraint results.
- Failure condition: DefinitionVersion drift, Visit reuse, or current-node identity substitutes for nodeVisitId.

### ACC-ARCH-010 — Exactly-one activation

- Contracts: CTR-ARCH-010
- Method: activation invariant review plus later Human and Agent Visit transaction tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: each new-model Visit atomically creates exactly one HumanWorkItem or DispatchIntent and never starts execution in that transaction.
- Required evidence: mutually exclusive fixture results, uniqueness proof, and execution-start absence.
- Failure condition: zero, both, duplicate, post-commit fill, or direct start is observed.

### ACC-ARCH-011 — Activation lifecycle facts

- Contracts: CTR-ARCH-011
- Method: fact-model inspection plus later close and immutability tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: creation/closure/eligibility changes are immutable facts; active/closed and current eligibility are projections; closure is terminal.
- Required evidence: fact schemas, projector cases, and duplicate/reopen negative cases.
- Failure condition: in-place historical rewrite, second close, reopen, or mutable status becomes authority.

### ACC-ARCH-012 — Initial eligibility timestamp

- Contracts: CTR-ARCH-012
- Method: transaction-boundary review plus later create/transition transaction tracing.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: server authors activationAt and initial nextEligibleAt in the same activation transaction without requiring the physical commit instant.
- Required evidence: transaction trace and tests rejecting client or post-commit timestamp fill.
- Failure condition: client authority, null-then-fill, post-commit fill, or commit-instant dependency.

### ACC-ARCH-013 — Due predicate and allowlist

- Contracts: CTR-ARCH-013
- Method: query semantics review plus later candidate-selection tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: the singular due predicate is active Dispatch Intent and nextEligibleAt at or before now; the Scheduler record contains exactly the section 5.7 identifiers/timestamps and no sensitive or business-key field.
- Required evidence: response-key/schema snapshot, sensitive-marker scan, and boundary-time/closed-activation cases.
- Failure condition: alternate due definition, closed work delivery, missing/extra record field, sensitive content, or business/node key appears.

### ACC-ARCH-014 — Create closure

- Contracts: CTR-ARCH-014
- Method: command-closure review plus later atomicity and failure-injection tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: one transaction creates Instance, entry Visit, owner, activation, initial Event, receipt, audit, and one version increment.
- Required evidence: committed row/fact set and rollback traces for each injected failure point.
- Failure condition: partial visibility, missing artifact, or version/Event mismatch.

### ACC-ARCH-015 — Transition closure

- Contracts: CTR-ARCH-015
- Method: transition model review plus later ADVANCE/RETURN/TERMINATE transaction tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: authorization and edge selection precede mutation; source activation closes and target Visit/activation is created when applicable in one transaction.
- Required evidence: success and rollback traces with Event, receipt, audit, and version outcomes.
- Failure condition: selection after mutation, partial close/create, or non-atomic evidence.

### ACC-ARCH-016 — RETURN re-entry

- Contracts: CTR-ARCH-016
- Method: graph/Visit review plus later multi-visit RETURN fixtures.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: RETURN follows the configured earlier reachable TASK target and creates a fresh Visit and activation.
- Required evidence: repeated-node history showing distinct nodeVisitId and target resolution.
- Failure condition: Visit reuse, wrong target, no target acceptance, or source activation remains active.

### ACC-ARCH-017 — Graph TERMINATE

- Contracts: CTR-ARCH-017
- Method: graph-validator and command review plus later configured/unconfigured TERMINATE tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: only configured TASK TERMINATE closes the source activation and Instance, creates exactly one terminal Visit, and creates no target activation.
- Required evidence: positive configured trace and negative absent-edge case.
- Failure condition: implicit TERMINATE, target activation creation, missing terminal Visit, or unclosed source activation.

### ACC-ARCH-018 — Cancel and Archive

- Contracts: CTR-ARCH-018
- Method: compare v0.3.2 semantics plus later cancel/archive transaction tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: Cancel stays on the current Visit, creates no Visit, atomically closes the current activation, increments version once, emits one Event, and Archive rejects active activation.
- Required evidence: v0.3.2 relation table and success/denial/rollback traces.
- Failure condition: Cancel moves Visits, leaves active work, or Archive accepts active work.

### ACC-ARCH-019 — Administrative commands

- Contracts: CTR-ARCH-019
- Method: Admin Recovery authority review plus later MOVE and MANUALLY_TERMINATE tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: reason and idempotency are required; MOVE closes source and creates target Visit/activation; terminate creates no target activation; both are atomic.
- Required evidence: authorized/unauthorized traces, reason audit, and rollback results.
- Failure condition: partial mutation, missing reason/audit, Visit reuse, or unauthorized recovery.

### ACC-ARCH-020 — Version and Event invariant

- Contracts: CTR-ARCH-020
- Method: state-command inventory plus later lock-order/concurrency/idempotency tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: each ordinary successful single-Instance command adds exactly one version and one Event; migration adds one source and one successor Event/version outcome; denies/no-ops add neither.
- Required evidence: lock trace plus command-by-command and migration source/successor version/Event matrix under success, retry, conflict, and denial.
- Failure condition: lock inversion, skipped/double per-Instance version, wrong per-Instance Event count, missing Receipt linkage, or denied mutation.

### ACC-ARCH-021 — Idempotency and unknown outcome

- Contracts: CTR-ARCH-021
- Method: retry protocol review plus later same-key/different-key and injected-outcome-unknown tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: one stable command identity returns the same terminal receipt; outcome_unknown is reconciled before any retry decision.
- Required evidence: durable receipt traces and duplicate/conflict/reconciliation cases.
- Failure condition: duplicate state change, regenerated key after ambiguity, or a guessed outcome.

### ACC-ARCH-022 — Wake and eligibility update

- Contracts: CTR-ARCH-022
- Method: eligibility command review plus later authorization-matrix wake/update tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: eligibility mutation requires command-specific authenticated authorization per §5.8 (external event, dependency completion, authorized manual action, or accepted attempt-bound Scheduler policy); it binds the exact current Visit/activation and sets server now, or an accepted bounded future time; the immutable eligibility fact and Event are committed atomically; wake is a hint.
- Required evidence: authorization matrix, fact/Event trace, and lost/duplicate wake cases.
- Failure condition: unauthorized actor update, arbitrary timestamp, mutation of node or owner, activation creation, or partial commit.

### ACC-ARCH-023 — Attempt and lease boundary

- Contracts: CTR-ARCH-023
- Method: interoperability review plus later attempt/lease concurrency tests in the external consumer boundary.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: stable attempt identity binds to one nodeVisitId; lease state is attempt-scoped outside workflow lifecycle authority.
- Required evidence: retry traces, uniqueness mapping, and lease-expiry cases preserving Visit identity.
- Failure condition: retry creates a Visit, attempt spans Visits, or lease state becomes workflow authority.

### ACC-ARCH-024 — Activation-driven delivery

- Contracts: CTR-ARCH-024
- Method: normal-path architecture review plus later bounded worker scheduling tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: normal Agent delivery is triggered only by committed active due Dispatch Intent facts via the bounded delivery obligation, with no Workflow Instance/global/business-key discovery scan.
- Required evidence: trigger trace and scan-source proof.
- Failure condition: table-wide, summary, or business-key scan drives normal delivery, or uncommitted intent is consumed.

### ACC-ARCH-025 — Recovery-only scans

- Contracts: CTR-ARCH-025
- Method: worker-role review plus later Reconciler/Watchdog/Repair fault-injection tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: scans only detect or deterministically repair missed/stuck/inconsistent facts, contain ambiguity, and preserve nodeVisitId and canonical activation identity.
- Required evidence: role-to-query map and lost-wake, stuck-attempt, and missing-projection repair traces.
- Failure condition: scan becomes normal feed, creates replacement Visit, or rewrites canonical facts.

### ACC-ARCH-026 — Fact and Event integrity

- Contracts: CTR-ARCH-026
- Method: replay model review plus later projection deletion and Event/fact corruption tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: projections rebuild deterministically from immutable canonical facts and Events; integrity violations fail closed and audit.
- Required evidence: clean rebuild equality, mismatch detection, and audited containment traces.
- Failure condition: mutable projection is authoritative, corruption is ignored, or rebuild changes facts.

### ACC-ARCH-027 — Missing-activation repair

- Contracts: CTR-ARCH-027
- Method: migration/repair proof review plus later fixture tests for provably absent historical activation.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: repair adds the one deterministically derived missing activation to the same Visit without rewriting history or creating a Visit.
- Required evidence: absence proof, deterministic derivation, idempotent repair receipt, and ambiguity rejection.
- Failure condition: ambiguous repair proceeds, Visit changes, duplicate activation appears, or historical facts are rewritten.

### ACC-ARCH-028 — One-way barrier

- Contracts: CTR-ARCH-028
- Method: barrier protocol review plus later pre/post-barrier creation tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: one immutable barrier exists; post-barrier new traffic requires VISIT_ACTIVATION_V1 and fails closed with no fallback or reverse routing.
- Required evidence: barrier record, boundary-time cases, and prohibited fallback/dual-write tests.
- Failure condition: barrier mutation, Legacy new creation after barrier, dual authority, or rollback across the barrier.

### ACC-ARCH-029 — Legacy drain

- Contracts: CTR-ARCH-029
- Method: Legacy compatibility review plus later pre-barrier drain command tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: only bounded DRAIN completes in-flight Legacy work while appending receipts, Events, audit, and version facts without changing Legacy business semantics.
- Required evidence: allowed-command inventory, append-only traces, and post-barrier new-Legacy rejection.
- Failure condition: Legacy mutation broadens, evidence is absent, or drain becomes new-traffic authority.

### ACC-ARCH-030 — One-time migration

- Contracts: CTR-ARCH-030
- Method: migration design review plus later dry-run and atomic failure-injection fixtures.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: explicit ONE_TIME_MIGRATE uses preassigned source/target identities, closes/marks complete source Legacy work, creates one pinned successor new-model Instance/Visit/activation and immutable mapping, and is idempotent.
- Required evidence: exact plan, eligibility proof, source/successor/mapping/Event/version trace, retry receipt, and rollback cases.
- Failure condition: source semantic identity mutates, migration is implicit, partial visibility occurs, or multiple successors appear.

### ACC-ARCH-031 — Manual terminate, replay, and rollback

- Contracts: CTR-ARCH-031
- Method: containment runbook review plus later MANUALLY_TERMINATE and HISTORICAL_REPLAY fixtures.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: terminate closes work with durable reason; replay is read-only; rollback uses containment and repair only and never reverses the barrier.
- Required evidence: command traces, replay no-write proof, and containment exercise.
- Failure condition: replay mutates, active work survives terminate, new Legacy traffic resumes, or reverse routing occurs.

### ACC-ARCH-032 — Global Reader compatibility

- Contracts: CTR-ARCH-032
- Method: accepted Global Reader relationship review plus later protected-route authorization/query tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: the role remains current route/role compatibility only and grants neither normal dispatch-feed authority nor work permission.
- Required evidence: route matrix and negative candidate/permission tests using only the role.
- Failure condition: role alone exposes dispatch candidates, permits work, or changes lifecycle.

### ACC-ARCH-033 — List, worklist, and HTTP compatibility

- Contracts: CTR-ARCH-033
- Method: interface inventory plus later compatibility endpoint tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: existing Instance/Visit/legacy worklist shapes remain bounded diagnostics or Legacy compatibility surfaces and never become canonical activation delivery.
- Required evidence: endpoint-to-authority map and negative normal-feed tests.
- Failure condition: a compatibility endpoint grants permission, defines due work, or replaces activation identity.

### ACC-ARCH-034 — Principal and self projection

- Contracts: CTR-ARCH-034
- Method: accepted Principal/self-projection authority review plus later authorization and projection tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: Principal identity supplies auth facts only; canonical role reads and same-Principal self filtering are preserved as projections; work eligibility still uses activation owner facts.
- Required evidence: identity/role/owner matrix and stale-projection denial cases.
- Failure condition: projection becomes ownership authority, SERVICE owns work, or self filter expands global access.

### ACC-ARCH-035 — Accepted child boundaries

- Contracts: CTR-ARCH-035
- Method: child-Spec relationship table review plus later regression cases.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: exact Principal successor and RETURN 422 behavior are preserved only in their accepted scopes and do not authorize DRAFT/NORMAL reuse or general reassignment.
- Required evidence: authority matrix and RETURN invalid-target plus successor migration cases.
- Failure condition: child authority is generalized or conflicts with new-model invariants.

### ACC-ARCH-036 — Legacy contract reconciliation

- Contracts: CTR-ARCH-036
- Method: later implementation-Spec contract-by-contract review against the enumerated accepted legacy set.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: every legacy contract is classified retained, compatibility-only, replaced, or inapplicable, with no silent authority carryover.
- Required evidence: complete reconciliation matrix with exact authority IDs and stable references.
- Failure condition: an affected legacy contract is omitted or directly treated as new-model implementation authority.

### ACC-ARCH-037 — External ownership boundary

- Contracts: CTR-ARCH-037
- Method: local interoperability acceptance review without changing an external repository.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: external periodic recovery is Reconciler-only, Scheduler management remains separate, and this Architecture contains no dsh consumer or external lifecycle design.
- Required evidence: local protocol acceptance matrix and explicit external-owner handoff boundary.
- Failure condition: normal polling is authorized, Scheduler and Reconciler collapse, or cross-repo lifecycle action appears.

### ACC-ARCH-038 — Authority sequencing

- Contracts: CTR-ARCH-038
- Method: governance check at Architecture review and again before any implementation work.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: accepted V6, accepted v0.4.0, and independently accepted implementation Spec all precede code; schema, external readiness, cutover, and production apply remain separate gates.
- Required evidence: exact authority heads and independent lifecycle records at each gate.
- Failure condition: proposal authorizes implementation, one gate implies another, or production coordinates appear.

### ACC-ARCH-039 — Audit and publication barriers

- Contracts: CTR-ARCH-039
- Method: security architecture review plus later allow/deny, audit-outage, disable/revoke, and protected-read publication tests.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: non-sensitive durable audit covers required operations for exactly 365 days; read audit precedes publication; write audit is atomic; unavailable auth/audit fails closed; no new read/export surface exists.
- Required evidence: retention configuration, redaction review, transaction traces, and outage/disable cases.
- Failure condition: unaudited publication/mutation, sensitive payload in audit, stale authorization success, retention not exactly 365 days, or a new read/export surface.

### ACC-ARCH-040 — No hidden expansion

- Contracts: CTR-ARCH-040
- Method: fresh Architecture and later implementation-diff review against V6 frozen points.
- Environment: exact proposal Head for Architecture review; later accepted implementation candidate for executable cases.
- Expected result: no new node, wait, lease, product, permission, trust boundary, external lifecycle action, or production coordinate is introduced.
- Required evidence: semantic diff checklist and explicit frozen-point scan.
- Failure condition: any hidden expansion appears without a lawful higher-authority change.

## 9. Alternatives and disposition

The following alternatives were considered and rejected:

- Infer new versus Legacy semantics from graph shape, metadata, node key, numeric
  coincidence, or a global cutover flag. Rejected because replay and migration
  would become nondeterministic and historical identity could drift.
- Mutate an existing Legacy Instance into the new semantic model. Rejected
  because the Instance semantic discriminator is immutable and Legacy history
  must remain truthful.
- Keep two active authorities and route opportunistically. Rejected because V6
  requires a one-way barrier, fail-closed post-barrier creation, and no fallback.
- Use Instance/current-node identity for delivery. Rejected because re-entry
  makes nodeVisitId the only unambiguous runtime work identity.
- Treat mutable status columns or queue messages as canonical activation facts.
  Rejected because reconstruction, repair, and atomic closure require immutable
  facts and deterministic projections.
- Make periodic scans the normal dispatch source. Rejected because normal
  delivery is activation-driven and scans are bounded recovery roles only.
- Use the Global Reader or existing list/worklist HTTP surface as a dispatch
  permission or feed. Rejected because those authorities are compatibility and
  diagnostics only.
- Put lease lifecycle into svc-workflow. Rejected because V6 freezes workflow
  lifecycle ownership locally and attempt-scoped leases externally.

## 10. Migration, compatibility, containment, and rollback

### 10.1 Pre-barrier proof

Before a production authority could select coordinates, a later accepted
implementation Spec MUST define inventory and dry-run evidence for:

1. every DefinitionVersion and Instance semantic-model classification;
2. every current descriptive numeric value and its non-alias mapping;
3. every active Legacy Instance eligible for DRAIN, ONE_TIME_MIGRATE, or
   MANUALLY_TERMINATE;
4. every new-model Visit and its exactly-one activation proof;
5. every Event/fact/projection consistency check;
6. all bounded worker and authorization readiness evidence; and
7. exact rollback-by-containment commands and observability.

This Architecture selects no production time, percentage, tenant, Domain,
Principal, concurrency value, feature flag, or deployment order.

### 10.2 Compatibility periods

Before the barrier, Legacy creation and execution remain governed by existing
accepted Legacy authorities. After the barrier, no new Legacy Instance may be
created. Pre-barrier Legacy Instances may only use the bounded modes in
CTR-ARCH-029 through CTR-ARCH-031. New-model instances never fall back to
Legacy and never use Legacy DRAFT/NORMAL lifecycle semantics.

Existing Global Reader, list, query, worklist, HTTP, Principal, and
self-projection surfaces remain only within their exact accepted compatibility
or diagnostic scope. Compatibility does not grant delivery, ownership, command,
or new-model lifecycle authority.

### 10.3 Containment and rollback

Containment MAY pause new-model creation or delivery, stop individual external
attempt acquisition, fail commands closed, repair deterministic projections,
or manually terminate explicitly selected work through an authorized command.
It MUST preserve canonical facts, nodeVisitId, immutable semantic identity,
receipts, Events, and audit.

Rollback MUST NOT move the barrier, resume post-barrier Legacy creation, reverse
route a new-model Instance, delete accepted evidence, or restore dual
authority. Recovery proceeds forward through replay, deterministic repair,
bounded retry with the same idempotency identity, or lawful superseding
authority.

### 10.4 Required later implementation sequencing

The next fresh author MAY create an implementation Spec only after v0.4.0 is
independently accepted. That Spec must name exact schema/API/command/worker/test
changes and reconcile all legacy contracts in CTR-ARCH-036. Code, schema
migration, external interoperability readiness, production cutover, and
production apply each require their own lawful authority and evidence.

## 11. Open questions and readiness

~~~text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
BLOCKED_DESIGN = NONE
CONTRACT_COUNT = 40
CONTRACTS_WITH_ACCEPTANCE = 40
ACCEPTANCE_COUNT = 40
IMPLEMENTATION_READY = NO
PRODUCTION_READY = NO
~~~

The absence of an Architecture-level open decision does not grant
implementation authority. Implementation remains blocked on independent
acceptance of this Architecture and a later independently accepted
implementation Spec.

## 12. AUTHOR output

~~~text
SPEC_GOVERNANCE_MODE = AUTHOR
SPEC_ID = SVC_WORKFLOW_ARCHITECTURE_V0_4_0
AUTHORITY_ID = SVC_WORKFLOW_ARCHITECTURE_V0_4_0
SPEC_KIND = invariant
STATUS = proposed
AUTHORITY_LEVEL = architecture
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PRIMARY_PARENT_AUTHORITY = SVC_WORKFLOW_PRODUCT_BOUNDARY_V6
EXTERNAL_AUTHORITIES = NONE (local interoperability acceptance only)
WHOLE_SUPERSEDES = SVC_WORKFLOW_ARCHITECTURE_V0_3_1
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
PARTIAL_SUPERSESSION = NONE
CONTRACT_COUNT = 40
CONTRACTS_WITH_ACCEPTANCE = 40
ACCEPTANCE_COUNT = 40
AUTHORING_READY_FOR_REVIEW = YES
~~~

This document remains a non-authoritative, unmerged proposal until an
independent reviewer lawfully accepts it. The author of this proposal MUST NOT
perform that acceptance.
