---
authority_id: SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
status: proposed
authority_kind: product_direction
owning_repository: mayf3/svc-workflow
implementation_authority: none
production_apply_authority: none
supersedes:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_PRODUCT_BOUNDARY_V5

## 1. Goal and authority status

This document is the complete Product Direction for `svc-workflow`. It is a whole-authority successor to `SVC_WORKFLOW_PRODUCT_BOUNDARY_V4`, not an amendment and not a reader-side composition with V4.

```text
AUTHORITY_ID = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
AUTHORITY_KIND = product_direction
STATUS = proposed
SUPERSEDES = SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
PRODUCT_BOUNDARY_ACTION = SUPERSEDE
WHOLE_AUTHORITY_SUPERSESSION = YES
PARTIAL_SUPERSESSION = NONE
OWNER_USE_CASE = V4_COMPLETE_RESTATEMENT_PLUS_BOUNDED_WORKFLOW_DISPATCHABILITY
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PRODUCT_DIRECTION_AUTHORIZES_IMPLEMENTATION_DIRECTLY = NO
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
CHILD_IMPLEMENTATION_SPEC_REQUIRED = YES
```

V5 is proposed and non-active. V4 remains the accepted Product Direction on `main`; this authoring round does not change V4 lifecycle/backlinks or the repository authority map. Only an independent fixed-head review followed by an explicit Owner whole-authority acceptance transaction may atomically accept V5, supersede V4, update the backlink/map, and permit V5-governed child-Spec review. This document never authorizes implementation or production apply directly.

The Goal is to preserve every V4 product boundary, Decision, Contract, exclusion, security invariant, capability Slice, retained trade-off, conformance-debt statement, original CTO bounded successor exception, and trusted-fleet exact-plan exception byte-for-meaning, while adding one bounded Current Product Direction:

```text
Workflow owns = whether an Instance is objectively suitable to begin ordinary current-assignee work now
HR/Scheduler owns = which objectively suitable assignee to contact in this scheduling round
```

The addition authorizes a read-time, non-persisted dispatchability projection and opt-in filter on the existing Domain/global list family. It does not create dispatch state, scheduling policy, Agent mapping, wake/message/session behavior, execution authority, or an operational-lock convention.

## 2. Scope and non-goals

### 2.1 In scope

V5 governs:

- the serial workflow engine, its immutable facts, Domain isolation, worklists, Definition and Instance lifecycle, Transition boundary, idempotency, and failure behavior;
- the two independent global permissions `GLOBAL_SCHEDULER_READ` and `GLOBAL_DOMAIN_ADMIN`;
- one dedicated administrative Agent Principal as the daily runtime actor in a single-user deployment;
- a repository-owned docs-only designation root for the exact Agent Principal, Client, and granted split permissions;
- direct-token execution by that Agent Principal;
- one exact Feishu app/tenant/conversation/sender command ingress as provenance and an Agent-core gate, never as `svc-workflow` authority;
- durable audit using the actual authenticated Agent Principal;
- capability-scoped child-authority sequencing and current conformance debt;
- the original CTO bounded one-time Principal successor migration already added to V2;
- the additive exact-plan-bound trusted fleet bounded successor exception in §17A, frozen to 86 exact successor pairs by `PLAN_SHA256`;
- one bounded, Workflow-owned dispatchability Product Direction in §8A, with exact ownership, authorization, pagination, compatibility, privacy, and child-authority boundaries.

### 2.2 Explicit non-goals

This Product Direction does not create or select an Agent, Principal UUID, Client, credential, permission Grant, designation root instance, database row, migration, API, HTTP Contract, OpenAPI surface, SDK, test, deployment, production change, auth-service change, or dsh-agent-core change. It does not accept, merge, mark Ready, or activate any child authority or external PR. It does not commit the frozen fleet plan artifact into this repository, and the frozen plan is not live database truth.

It does not add parallel workflow nodes, dynamic forward branching, claim/pull assignment, ordinary reassignment, handoff, delegation, timers, external signals, automatic retry, SLA orchestration, arbitrary script guards, built-in LLM execution, cross-Domain shared templates, in-flight template replacement, in-flight Domain transfer, physical Instance deletion, unrestricted global workflow content access, or a runtime break-glass grant.

It does not create an HR Dispatcher product, a Scheduler, cron job, wake mechanism, Agent Session, message transport, notification ledger, fairness/priority/quota/retry policy, Principal-to-Agent mapping, lease/reservation/claim token, or durable dispatchability/ops-lock column. It does not change Grants, role bindings, credentials, allowlists, production data, or runtime.

Long-term multi-Human governance is deferred, not forbidden. It may be introduced only by a lawful later Product Direction successor or an independent higher authority; no implementation or runtime configuration may silently reinterpret V5 to add it.

## 3. Authority and exact coordinates

```text
SVC_WORKFLOW_BASE_COMMIT = f0c74eefd63ca71a1fcb670ad31ac35f19f69539
CURRENT_PRODUCT_DIRECTION = SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
CURRENT_PRODUCT_DIRECTION_ACCEPT_COMMIT = 5cdd5eeb9895ce0bb4df1989f01806ca25b8ecff
SUCCESSOR_PRODUCT_DIRECTION = SVC_WORKFLOW_PRODUCT_BOUNDARY_V5
DISPATCHABILITY_CHILD_CANDIDATE = SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1
DISPATCHABILITY_CHILD_CANDIDATE_COMMIT = af450aa39e446683b8ae2b2edf99c4febdcfb068
DISPATCHABILITY_CHILD_REMOTE_REF = ABSENT
GLOBAL_READER_SPEC = SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1
GLOBAL_READER_ACCEPT_COMMIT = ea9ab2df0da7e58328ce5018164a2d2b6d6c14a9
GLOBAL_READER_IMPLEMENTATION_MERGE = bf875c265843b3e07570a96b734051e9cfe27a43
AUTH_SERVICE_REFERENCE_COMMIT = 0855dc5161309196ef0cddbf9142e22726961956
DSH_AGENT_CORE_REFERENCE_COMMIT = 6ec83fa7ef0565959f26c7112de423bf5aa65680
BLOCKED_CHILD_PR = https://github.com/mayf3/svc-workflow/pull/9
BLOCKED_CHILD_HEAD = 3056263c3fc964a2b225720dd2b859b47e296c2e
FLEET_PLAN_SCHEMA = workflow_trusted_fleet_successor_plan_v2
FLEET_PLAN_SIZE_BYTES = 540472
FLEET_PLAN_SHA256 = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606
FLEET_ROSTER_SHA256 = f046d18f76da838ba94775af7c960d0ee548f2e392c22e6c7b0e3add36cb8e5f
FLEET_PLAN_CANONICAL = YES
FLEET_PLAN_MODE = READ_ONLY_CANONICAL_PLAN
FLEET_PLAN_SNAPSHOT_UTC = 2026-08-24T01:03:53.192875+00:00
```

Authority precedence remains Product Direction, then accepted Architecture/long-lived invariant authority, then accepted governing child Specs, then descriptive code/tests/runtime/operations. V5 changes no external repository authority. V4 remains active until an atomic accepted V5 transition is merged.

The accepted one-time child authority `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` and both V4 successor exceptions remain governed through the byte-for-meaning restatement in §§17/17A. Open svc-workflow Draft PR #7 remains independent. PR #9 retains its V4 disposition and is neither modified nor merged by this round.

The dispatchability child commit is an immutable local candidate, not a GitHub ref or active authority. It is evidence input only. It must be revised after an accepted V5 is present on `main`, then independently audited and accepted before implementation. The accepted `SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1` is a lower-level current implementation authority; V5 reconciles its exact server role with the Product Direction concept in §8A without granting or changing any role.

External Draft PRs remain classified exactly as in V4:

```text
AUTH_SERVICE_PR_15_DISPOSITION = KEEP_DRAFT_DEFERRED
AUTH_SERVICE_PR_15_REQUIRED_FOR_AGENT_FIRST_V1 = NO
AUTH_SERVICE_PR_15_ACTIVE_ON_MAIN = NO
AUTH_SERVICE_PR_2_BLOCKS_AGENT_ADMIN_ROUTE = NO
AUTH_SERVICE_PR_2_RELATION = INDEPENDENT_LEGACY_SHUTDOWN_PROGRAM
```

PR #15 is a possible future input to multi-Human governance, not an Agent-first V1 prerequisite. PR #2 is an independent legacy shutdown Program and MUST NOT become a common mutable gate.

## 4. Current State, Observations, Claims, and Evidence

All State below is descriptive. Drift does not rewrite this Product Direction.

### 4.1 Observations

#### OBS-V5-001 — Active repository Product Direction and governance

- Subject: `mayf3/svc-workflow` source tree.
- Source revision: `f0c74eefd63ca71a1fcb670ad31ac35f19f69539`.
- Method: fresh-fetch `github/main`; inspect Product Direction frontmatter, `.agents/local/README.md`, governance lock, governing Spec index, and run the governance verifier.
- Result: V4 is accepted and active on `main`; V3/V2 are superseded; governance adoption is accepted; V5 does not exist on the base.
- Provenance: repository paths and authoring preflight record.

#### OBS-V5-002 — Existing global scheduler surface remains broader than V5

- Subject: svc-workflow current source/contract state.
- Source revision: `f0c74eefd63ca71a1fcb670ad31ac35f19f69539`.
- Method: inspect the global query, current-state HTTP Contract, accepted Global Reader Spec, V4 §8, and the dispatchability child candidate.
- Result: the existing global query remains a `Page<DomainInstanceSummary>` with broader legacy populations/fields than V4's canonical scheduler view, lacks dispatchability, and uses the accepted `GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR` read gate. V5 treats this as bounded current-state compatibility plus remaining Slice-D conformance debt.
- Provenance: `src/application/workflow_instance/`, `src/store/postgres/workflow_instance_repository/`, `contracts/workflow-http/v1/`, accepted reader Spec, and candidate commit in §3.

#### OBS-V5-003 — Existing global Domain administration is not V4-complete

- Subject: svc-workflow provisioning surface at the base commit.
- Method: inspect `docs/contracts/IDENTITY_PROVISIONING_API_V0.md` and current source.
- Result: the surface is allowlist/scope based and broader than the V4-preserved minimum; separated permission lifecycle, comprehensive no-self-grant/no-self-owner enforcement, minimum Human/Agent selection directory, and narrowed existing Domain-admin behavior remain unestablished.
- Provenance: `docs/contracts/IDENTITY_PROVISIONING_API_V0.md` and source tree.

#### OBS-V5-004 — auth-service reusable identity primitives and missing child permission supply

- Subject: `mayf3/auth-service` at `0855dc5161309196ef0cddbf9142e22726961956`.
- Method: inspect accepted/frozen identity, Machine Principal, Client, direct-token, resolution, and revoke contracts and compare them with the V4-preserved designated-permission requirement.
- Result: Agent Principal, Client, direct token, identity resolution, rotation, and revocation primitives are reusable; an accepted capability-scoped child authority supplying the designated Agent's required `svc-workflow` audience/scope/grant is still required. Human Principal administration is not a prerequisite.
- Provenance: exact external revision and the later auth-service child-authority review record.

#### OBS-V5-005 — dsh-agent-core command-route gaps

- Subject: `mayf3/dsh-agent-core` at `6ec83fa7ef0565959f26c7112de423bf5aa65680`.
- Method: inspect Feishu connector, Router/binding, Broker capability manifests, Scheduler, Notification Ingress, and their authority records.
- Result: no implemented fleet-level svc-workflow scheduler query and no workflow Domain-admin capability manifest exist. The shipped workflow manifest has four caller-scoped `workflow.read` capabilities only. Feishu admission has runtime app credentials and a dynamic prebound-conversation gate, but no committed exact tenant or sender-identity allowlist; replay handling is SDK-owned and message-ID based. Notification Ingress V0 is an implemented loopback thin delivery adapter without auth/durable idempotency/workflow authority; its V1 auth/idempotency document is accepted design authority but explicitly does not authorize implementation. Router/Broker identity discipline ignores self-reported fields, selects credentials by trusted process `agentId`, and relies on auth-service to resolve the actual Agent Principal; the V4-preserved exact administrative route still requires its own authority and conformance evidence.
- Provenance: `packages/scheduler/src/`, `packages/broker/src/capabilities/workflow.js`, `packages/feishu-connector/src/`, `packages/production-runtime/src/v2-ingress-gate.js`, `packages/notification-ingress/src/index.js`, `docs/specs/NOTIFICATION_INGRESS_SERVICE_AUTH_AND_IDEMPOTENCY_V1.md`, and Router/Broker credential-path sources at the exact external revision.

#### OBS-V5-006 — Open and historical authority inventory

- Subject: svc-workflow refs, Product Direction, Specs, and the dispatchability candidate.
- Method: fresh-fetch GitHub `main`; inspect local branches/worktrees without reusing them; audit immutable child commit `af450aa39e446683b8ae2b2edf99c4febdcfb068`.
- Result: the child candidate is not published on GitHub and has no PR/ref; it correctly reports the V4 parent conflict. No competing V5 or accepted dispatchability Product Direction exists.
- Provenance: fixed-SHA `可派 审计` record and exact §3 coordinates.

#### OBS-V5-007 — Build in Public child is blocked by V3 exact-pair authority

- Subject: `mayf3/svc-workflow` Draft PR #9.
- Source revision: `3056263c3fc964a2b225720dd2b859b47e296c2e` against main `327b74f138151a7f4d9d88e3881e54d203f1e8f6`.
- Method: inspect the blocked child's exact pair, authority analysis, and proposed Contracts; compare with accepted V3 §17 and `CTR-V5-032` through `CTR-V5-034`.
- Result: PR #9 is fixed to Build in Public OLD `bb9d8f48-7962-4321-8fb1-554bb428c159` and NEW `d5b3aeb2-e754-49a9-9914-b963521c0985`, while V3 authorizes only the distinct CTO pair. The child correctly remains non-implementing and reports a lawful-parent blocker.
- Provenance: exact GitHub PR and commit coordinates above; no production database was queried.

#### OBS-V5-008 — Frozen trusted-fleet successor plan artifact

- Subject: local canonical plan `workflow_trusted_fleet_successor_plan_v2.json`.
- Source: file at `/Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json`, byte size 540472, `PLAN_SHA256 = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606`, frozen roster digest `ROSTER_SHA256 = f046d18f76da838ba94775af7c960d0ee548f2e392c22e6c7b0e3add36cb8e5f`, snapshot `2026-08-24T01:03:53.192875+00:00`, mode `READ_ONLY_CANONICAL_PLAN`, superseding plan v1 digest `57f769d0bc9f0a4494dd37685da3cc8657b2dc5845f020858457dbecc35ce9b7` (538625 bytes).
- Method: `shasum -a 256` byte verification plus structural count verification of the artifact summary and row arrays.
- Result: 86 `EXACT_SUCCESSOR_PAIR` rows with 86 `active` NEW Auth Principals, 0 ambiguous, 0 conflict; 760 Domain tuples (8 `DOMAIN_OWNER` + 752 `DOMAIN_MEMBER`); 80 active responsibility tuples; 99 creator-owned draft tuples with 0 migration candidates; 85 missing plus 1 present NEW Workflow projection, the present one being exactly `agt_build-in-public-agent`; and exactly one excluded duplicate identity `efficiency-agent`/`d09f8849-073c-484a-978c-f375113c28b2` (disabled, zero enabled Domain bindings, zero Visits, zero future operator writes). The roster source file verifies at sha256 `32d0b23753370156150babcaf0b108ad1d8c2b28f952e9586cf700142f9ec852` under `docs/evidence/account-recovery-phase-a-20260823/` in dsh-agent-core.
- Provenance: artifact bytes verified locally; roster evidence file digest verified against the artifact's recorded `frozen_roster_file_sha256`.

#### OBS-V5-009 — Systemic assigned-to-me 404 for unprojected NEW fleet principals

- Subject: svc-workflow worklist behavior for the 85 NEW fleet principals absent from the Principal projection.
- Method: read-only diagnosis recorded in the frozen artifact's `broker_4xx_diagnosis`.
- Result: `/internal/v1/worklists/assigned-to-me` returns `404 principal_not_found` when the actor Principal is absent from the projection, and the model renderer flattens it to a generic HTTP 4xx; the one projected principal `agt_build-in-public-agent` returns HTTP 200 with `items = []`.
- Provenance: artifact `broker_4xx_diagnosis` with its recorded in-production sample evidence path; no production write was performed.

#### OBS-V5-010 — Dispatchability child conflicts with accepted V4

- Subject: `SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1`.
- Source revision: `af450aa39e446683b8ae2b2edf99c4febdcfb068`, parent `f0c74eefd63ca71a1fcb670ad31ac35f19f69539`.
- Method: independent fixed-SHA semantic audit against V4 §8.
- Result: V4 forbids blocked flags/reason codes and Assistance-derived blocking status on the global scheduling surface; the candidate proposes `dispatchable`, `dispatch_blocked_reasons`, and `ASSISTANCE_OPEN`. The conflict requires whole Product Direction supersession, not a child edit.
- Provenance: fixed-SHA audit; candidate §3.3 and `OBS-013`.

#### OBS-V5-011 — Accepted current global read-role gate

- Subject: `SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1` plus merged implementation.
- Source revisions: acceptance `ea9ab2df0da7e58328ce5018164a2d2b6d6c14a9`; implementation merge `bf875c265843b3e07570a96b734051e9cfe27a43`.
- Method: inspect accepted Spec, query visibility SQL, handler errors, and role-binding API.
- Result: the global list is authorized by enabled server-side role `GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR`; the reader role grants only the global GET surface and no write capability. This current gate postdates V4 and must be reconciled explicitly rather than ignored.
- Provenance: accepted Spec and source paths recorded in §3.

#### OBS-V5-012 — Strict wire compatibility forbids silent always-present fields

- Subject: `contracts/workflow-http/v1/compatibility.md`.
- Source revision: `f0c74eefd63ca71a1fcb670ad31ac35f19f69539`.
- Method: inspect current compatibility policy.
- Result: existing valid requests must retain the same response structure. Adding two always-present fields to requests that omit any dispatchability opt-in would violate the current policy; an explicit opt-in representation preserves it.
- Provenance: compatibility policy rules 1-3.

### 4.2 Claims

#### CLM-V5-001 — Dispatchability requires whole-authority supersession

- Support state: SUPPORTED.
- Supported by: `EVD-V5-001`.
- Claim: adding a global eligibility flag, closed reason codes including one Assistance-derived fact, and a current read-role relationship changes V4's complete allowlist and prohibition meaning; V0 therefore requires a whole V5 successor.

#### CLM-V5-002 — Existing implementation is conformance debt, not authority

- Support state: SUPPORTED.
- Supported by: `EVD-V5-002`.
- Claim: current global read, Domain administration, Agent-core ingress, and external permission supply do not already conform merely because partial mechanisms exist.

#### CLM-V5-003 — Capability-scoped sequencing avoids a common global gate

- Support state: SUPPORTED.
- Supported by: `EVD-V5-003`.
- Claim: separate child authorities can close identity, designation, permission supply, scheduler read, Domain admin, and Feishu routing without allowing one Slice to authorize another or making Human governance a common prerequisite.

#### CLM-V5-004 — Complete restatement can preserve V4 while adding bounded dispatchability

- Support state: SUPPORTED.
- Supported by: `EVD-V5-004`.
- Claim: a complete V5 restatement can preserve every V4 boundary and both exact successor exceptions while adding only the bounded §8A dispatchability direction without creating scheduler or execution authority.

#### CLM-V5-005 — Exact-plan binding prevents a general migration capability

- Support state: SUPPORTED.
- Supported by: `EVD-V5-005`.
- Claim: binding the fleet exception to one digest-frozen artifact with 86 exact pairs, closed counts, per-pair SERIALIZABLE apply, and fail-closed drift prevents any caller-parameterized or dynamically expanded migration capability.

#### CLM-V5-006 — Objective projection belongs to Workflow without scheduling policy

- Support state: SUPPORTED.
- Supported by: `EVD-V5-006`.
- Claim: lifecycle, current Visit, assignee, Principal enabled state, Definition Version state, cancellation/archive, and open current-Visit Assistance are Workflow-owned formal facts; projecting them at read time avoids duplicating Workflow semantics in HR while leaving caller selection policy outside Workflow.

#### CLM-V5-007 — Explicit opt-in preserves legacy response shape

- Support state: SUPPORTED.
- Supported by: `EVD-V5-007`.
- Claim: keeping dispatchability fields absent when `dispatchableOnly` is omitted preserves existing request response structure, while explicit `true|false` selects the new representation and permits the bounded additive fields.

### 4.3 Evidence relations

#### EVD-V5-001 — Parent, child, role, and compatibility observations support supersession

- Source observations: `OBS-V5-001`, `OBS-V5-010`, `OBS-V5-011`, `OBS-V5-012`.
- Target: `CLM-V5-001`.
- Relation: SUPPORTS.
- Bound coordinates: svc-workflow base and candidate revisions in §3.
- Strength/sufficiency: sufficient for whole-authority classification.
- Limitations: does not accept V5 or authorize child implementation.

#### EVD-V5-002 — Current-source observations support drift classification

- Source observations: `OBS-V5-002`, `OBS-V5-003`, `OBS-V5-004`, `OBS-V5-005`.
- Target: `CLM-V5-002`.
- Relation: SUPPORTS.
- Bound coordinates: exact repository revisions in §3.
- Strength/sufficiency: sufficient to reject claims of present conformance.
- Limitations: child implementation review must refresh source/runtime observations on its own base.

#### EVD-V5-003 — Split ownership supports capability-scoped children

- Source observations: `OBS-V5-003`, `OBS-V5-004`, `OBS-V5-005`.
- Target: `CLM-V5-003`.
- Relation: SUPPORTS.
- Bound coordinates: exact repository revisions in §3.
- Strength/sufficiency: sufficient for Product Direction decomposition.
- Limitations: each external repository retains acceptance authority over its own child.

#### EVD-V5-004 — V4 restatement and bounded delta support preservation

- Source observations: `OBS-V5-001`, `OBS-V5-007`, `OBS-V5-008`, `OBS-V5-010`, accepted V4 at `5cdd5eeb9895ce0bb4df1989f01806ca25b8ecff`, and this file's explicit preservation of §§5-17A.
- Target: `CLM-V5-004`.
- Relation: SUPPORTS.
- Bound coordinates: §3 blocked-child and current-parent commits.
- Strength/sufficiency: sufficient for bounded proposal authoring and independent semantic review.
- Limitations: does not accept V5, revise the child, implement an operator/projection, establish live scope, or authorize production apply.

#### EVD-V5-005 — Frozen artifact counts and exclusion support exact-plan binding

- Source observations: `OBS-V5-008`, `OBS-V5-009`.
- Target: `CLM-V5-005`.
- Relation: SUPPORTS.
- Bound coordinates: §17A.1 artifact digests.
- Strength/sufficiency: sufficient to bind the proposal to exact artifact rows.
- Limitations: future apply must re-verify live state; the artifact itself grants no write authority.

#### EVD-V5-006 — Formal Workflow facts support projection ownership

- Source observations: `OBS-V5-002`, `OBS-V5-010` and candidate `OBS-004` through `OBS-012`.
- Target: `CLM-V5-006`.
- Relation: SUPPORTS.
- Bound coordinates: svc-workflow `f0c74ee` and candidate `af450aa`.
- Strength/sufficiency: sufficient for Product Direction ownership; exact predicates remain child-Spec obligations.
- Limitations: no formal durable ops-lock primitive exists; it remains excluded.

#### EVD-V5-007 — Compatibility policy supports opt-in representation

- Source observations: `OBS-V5-012`.
- Target: `CLM-V5-007`.
- Relation: SUPPORTS.
- Bound coordinates: current HTTP compatibility policy at `f0c74ee`.
- Strength/sufficiency: sufficient to reject silent always-present fields on legacy requests.
- Limitations: child review must still test exact clients and endpoint field naming.

## 5. Product positioning and qualifying workflow shape

`svc-workflow` is a platform-level, serial, governed workflow engine for fixed Agent, Human, and Service Principals. It owns versioned Workflow Definition governance, Workflow Instance lifecycle, legal Transition execution, immutable event-sourced history, strict normal-data-plane Domain isolation, assignee/creator worklists, Domain-local administration, bounded global scheduling metadata and Domain bootstrap/Owner replacement, and idempotent concurrency-safe commands.

It guarantees that a known Principal acts on an authorized workflow at its current node against an explicit Definition Version and workflow state version, and that every committed state change has immutable history. It validates workflow structure and JSON protocol shape; it does not decide payload business meaning or truth and does not run an LLM.

```text
one current node per Workflow Instance
one concrete current assignee for every active non-terminal current task
one deterministic normal forward path
JSON stage delivery
configured backward RETURN paths
configured or governed termination paths
```

### 5.1 Workflow Definition

A Definition is a Domain-owned versioned template containing node/Transition graphs, assignee references, Context and Submission schemas, and the deterministic normal path.

```text
DRAFT -> PUBLISHED -> DEPRECATED -> REVOKED
```

A Draft may be edited and validated but cannot create normal production Instances. A Published version may create Instances and is immutable. A Deprecated version creates no new Instances while existing Instances continue only as accepted child authority allows. Revoked behavior is governed by accepted Architecture/child Specs. Publication freezes graph, schemas, assignee references, ordering, validator semantics, and digest inputs. Archive/discovery is non-destructive. A Definition belongs to exactly one Domain; another Domain uses a separate Definition unless a later Product Direction changes that rule.

### 5.2 Workflow Instance

An Instance is one independent execution of one immutable Definition Version in one Domain. It owns its current Context Revision, current Node Visit, workflow state version, lifecycle/governance metadata, and references to immutable history. It is not an upper-layer business object. Optional external references may correlate it to one, but the upper layer owns that object's identity and full data.

Lifecycle includes creation, authorized Context revision, Transition, graph-external Domain Owner cancellation, and non-destructive archive. Normal product APIs do not physically delete Instances. Cancel/archive retain facts and remain governed by accepted Architecture and child Specs.

### 5.3 Transition, Context, Node Visit, and Submission

`ADVANCE` follows the normal configured direction, including normal terminal completion. `RETURN` moves to an allowed earlier non-terminal node and creates a new Visit. `TERMINATE` follows a configured graph edge to an exceptional terminal. Domain Owner `CANCEL` is graph-external governance, not a Transition effect.

Only the authorized current assignee performs a normal Transition. Submission, target Visit, current projection, one state-version increment, Workflow Event, command outcome, Receipt, and required audit commit atomically. Domain ownership, broad scope, global permission, or Agent designation does not imply Transition authority.

Context is versioned workflow input, not the complete upper-layer business record. Immutable revisions form one chain and mutation remains bounded by workflow rules. A Visit immutably records node entry and assignee snapshot; later Owner/Definition changes do not rewrite it. Submission is immutable JSON stage delivery; large resources may be URI/digest references, and schema validity is shape rather than business truth.

### 5.4 Authoritative history

The authoritative workflow history consists of immutable `WorkflowContextRevision`, `NodeVisit`, `Submission`, and `WorkflowEvent` facts. Current Context, current Visit, and workflow state version are projections over them. A successful state command changes the version once and records its Event once; partial workflow-fact commits are forbidden. Timeline is a projection, not a second authority. Global scheduling grants neither timeline nor `EventData` access.

## 6. Domain isolation, worklists, and Domain-local administration

A Domain is the workflow business-ownership, Definition-management, permission, and audit boundary. Each Definition and Instance belongs to one Domain. Canonical Domain role binding is the only Domain Owner authority; no unrelated owner field duplicates it.

```text
NORMAL_DATA_PLANE_DOMAIN_ISOLATION = STRICT
GLOBAL_CONTROL_PLANE_EXCEPTION = AUTHORIZED_AND_BOUNDED
```

An ordinary Agent/member or Domain Owner cannot see another Domain merely because it exists. Domain Owner authority is Domain-local. Current assignees receive only authorized Instance-local access; historical participants receive only explicitly governed participation history. Scope, allowlist, service/Feishu identity, UI role, or combinations of Domain-local roles do not create cross-Domain authority. Lookup, list, count, cursor, denial, and serialization behavior must not leak another Domain's existence or facts. Cross-Domain authority exists only through §§7-9's two explicit permissions and enumerated data/operations. No Architecture, child Spec, API, SDK, migration, code, test, deployment, legacy role, or UI label may broaden it.

The product owns assigned-to-me current tasks, authorized creator-owned drafts, Domain-local Instance/audit views for the effective Owner, and authorized feedback about a Principal's own Submissions. A worklist item is a workflow projection, not a Todo/business object. The global scheduler is separate from ordinary worklists and full Instance views.

`svc-workflow` owns local Principal/Domain/membership/ownership/workflow-role projections and bindings; global identity/authentication remain external. A Domain Owner remains Domain-local, cannot become global through role composition, and cannot rewrite Visit snapshots. Verified authentication supplies actor identity; request bodies do not.

## 7. Deployment actor and split global permission model

```text
DEPLOYMENT_MODE = SINGLE_USER
OWNER = mayf3
DAILY_EXECUTION_ACTOR = DEDICATED_ADMIN_AGENT_PRINCIPAL
HUMAN_PRINCIPAL_AS_RUNTIME_ACTOR = NOT_REQUIRED
ADMIN_AGENT_STRATEGY = NEW_DEDICATED_AGENT
EXISTING_BUSINESS_OR_CANARY_AGENT_REUSE = FORBIDDEN_BY_DEFAULT

GLOBAL_PERMISSIONS =
  GLOBAL_SCHEDULER_READ
  GLOBAL_DOMAIN_ADMIN
PERMISSION_MODEL = SPLIT
SAME_AGENT_MAY_HOLD_BOTH = YES
ONE_PERMISSION_IMPLIES_THE_OTHER = NO
PRODUCT_DIRECTION_GLOBAL_WORKFLOW_COORDINATOR = UI_LABEL_ONLY
```

The dedicated Agent is an actual auth-service Agent Principal and daily runtime actor. V5 selects no UUID or Client ID. Existing business/canary Agents are not reused by default; a later designation authority must identify a newly dedicated Agent and exact Client.

There are exactly two independent Product Direction permissions. The same designated Agent may hold either or both, but one never implies the other and both do not form a third authorization capability. V5 creates no composite runtime role. `GLOBAL_WORKFLOW_COORDINATOR` may be presentation text only in a conformant Product Direction surface; it is not a Product Direction permission, migration target, or authorization alias. The pre-existing server role with that name remains only as the explicitly bounded route-compatibility debt in §8; V5 neither denies that observed role exists nor authorizes any new binding for it.

Neither permission grants workflow content, Transition, reassignment, cancel/archive, Definition management, membership management, Assistance body, credentials, or audit-content access.

## 8. `GLOBAL_SCHEDULER_READ`

This Product Direction keeps `GLOBAL_SCHEDULER_READ` as the canonical product capability: deployment-wide read-only scheduling metadata. It is not timer, dispatch, retry, SLA, signal, Transition, or orchestration authority.

```text
PRODUCT_CAPABILITY = GLOBAL_SCHEDULER_READ
CURRENT_SERVER_ROLE_KEY = GLOBAL_WORKFLOW_READER
CURRENT_GLOBAL_ROUTE_COMPATIBILITY_GATE =
  GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR
GLOBAL_WORKFLOW_READER_IS_THIRD_PRODUCT_PERMISSION = NO
NEW_GLOBAL_WORKFLOW_COORDINATOR_GRANTS_AUTHORIZED = NO
FULL_CONTENT_ACCESS_REQUIRED = NO
SCHEDULING_VIEW_SCOPE = ACTIVE_CURRENT_TASK_METADATA_ONLY
TASK_LABEL = NOT_INCLUDED
CONTEXT_TITLE_AS_METADATA = FORBIDDEN
```

`GLOBAL_WORKFLOW_READER` is the accepted current server-side role key that realizes the read-only global-list surface. It is not a third Product Direction permission and grants no write. The legacy `GLOBAL_WORKFLOW_COORDINATOR` branch of the current route gate is bounded compatibility debt: V5 creates no new Coordinator grant, does not let Coordinator imply Domain-admin Product Direction authority, and requires a later accepted Slice-D migration before that legacy gate can be removed. The dispatchability child MUST retain the current gate and error semantics; it MUST NOT change roles, grants, scopes, identities, or allowlists.

The fully conformant canonical scheduler record retains V4's core field allowlist:

```text
principalId
principalDisplayName
activeTaskCount
workflowInstanceId
domainId
domainKey
domainDisplayName
definitionKey
definitionVersionId
currentNodeId
currentNodeKey
currentNodeDisplayName
currentNodeType
currentAssigneePrincipalId
lifecycle
status
nodeEnteredAt
updatedAt
```

Each logical task record has `principalId == currentAssigneePrincipalId`; `activeTaskCount` is that Principal's count in the same projection snapshot. Node type/lifecycle/status use closed non-sensitive code sets. `updatedAt` is the latest committed Workflow Instance state-change time represented by the current authoritative projection.

V5 additionally authorizes an exact dispatchability-aware extension on the existing Domain/global `Page<DomainInstanceSummary>` family:

```text
query opt-in = dispatchableOnly=true|false
response extension (pair, required together when opted in):
  dispatchable
  dispatch_blocked_reasons
pagination = items + next_cursor
```

When `dispatchableOnly` is omitted, the request is legacy and the response structure MUST remain byte-for-meaning compatible with the current endpoint: neither new field is emitted. Explicit `false` opts into the extended response without filtering. Explicit `true` opts into the extension and adds an `AND` predicate selecting only rows whose reason set is empty. Filtering occurs before `ORDER BY`, `limit + 1`, and cursor creation; callers traverse `next_cursor` until null. No `page`, `totalPages`, `totalCount`, offset cursor, or count query is added.

`dispatchable` means only: at one read snapshot, no child-authorized objective blocker is true for beginning ordinary current-assignee work. It is derived from existing Workflow-owned formal facts in the same read snapshot, is never persisted/evented/leased/reserved, and never replaces command-time validation. The child freezes the exact closed reason enum and predicates. It may include a closed `ASSISTANCE_OPEN` code because open current-Visit Assistance already blocks ordinary transition, but MUST expose no Assistance status/body/request/escalation/resolution/supporting payload and MUST NOT derive any other Assistance content.

Only active ordinary current-assignee work may pass `dispatchableOnly=true`. Archived, cancelled, terminal, current DRAFT, no-assignee, disabled-assignee, blocked Definition Version, and child-authorized open-Assistance facts fail closed. Domain-disabled and Definition-archived meanings remain deferred where current authorities disagree. `OPS_LOCKED` is forbidden: no authoritative durable ops-lock primitive exists, and no metadata/free-text key such as `"ops-lock"` may be inferred.

The V4 sensitive-field exclusions remain. Context/title, task label, Submission/history, timeline `EventData`, Assistance content, credentials/tokens, Receipt/command attempt/SecurityAudit/audit payload, Transition options, write capability, and content-derived summaries remain forbidden. Generic `blockedFlag`, `blockedReasonCode`, `waitingAssistance`, and `assistanceStatus` remain unauthorized; only the exact opt-in pair and closed non-sensitive codes above are authorized.

Workflow owns objective dispatchability. HR/Scheduler owns whom to contact, round size, fairness, deduplication, notification history, retry interval/backoff, priority, Principal-to-Agent mapping, message delivery, Session selection/reuse, and wake policy. `svc-workflow` MUST NOT send or wake an Agent.

## 8A. Dispatchability child authority and sequencing

V5 does not itself authorize implementation. After V5 is independently reviewed, Owner-accepted, and merged, a revised `SVC_WORKFLOW_DISPATCHABILITY_PROJECTION_V1` may define exact predicates and wire Contracts. It MUST:

1. pin accepted V5 on its base;
2. retain Domain Owner authorization for the Domain list and the current global route compatibility gate;
3. adopt the opt-in response-shape rule above rather than always-present fields;
4. preserve `items + next_cursor` and filter before limit/cursor;
5. keep HR policy, Agent mapping, messaging, Sessions, wake, retries, and ops-lock outside Workflow;
6. require independent Spec review before implementation;
7. leave the broader V5 Slice-D core-field/permission conformance debt explicit rather than claiming that two new fields complete it.

## 9. `GLOBAL_DOMAIN_ADMIN`

Allowed operations are only:

1. idempotent Domain creation;
2. atomic initial Owner assignment during creation, or disabled retention until a valid Owner is ready;
3. atomic Domain Owner replacement;
4. the minimum Domain/Principal selection directory required for those operations.

Creation/replacement preserves one effective Owner, canonical Principal identity, idempotency, transaction integrity, and durable audit. An enabled Domain cannot be left ownerless by partial success.

Complete minimum directory fields:

```text
Domain selection:
  domainId
  domainKey
  domainDisplayName
  domainEnabled
  currentOwnerPrincipalId
  currentOwnerPrincipalDisplayName

Principal selection:
  principalId
  principalDisplayName
  principalType
  principalEnabled
```

There is one logical record per Domain/Principal. The directory is only for create/replace selection and excludes membership, workflow counts/facts, email, Feishu identifiers, credentials, scopes, permission bindings, audit data, and content-derived fields. Child Specs may freeze search/pagination/nullability/grouping but not add globally readable data.

Excluded authority includes Workflow Instance/Context/Submission/Visit/timeline/Assistance/worklist content; Transition/Context revision/reassignment/cancel/archive; Definition lifecycle/assignment; membership; Domain-local audit content; any other Domain data write; self-grant; self-Owner; or actor derivation from body, Feishu, display name, service identity, scope, or allowlist.

```text
SELF_GRANT = FORBIDDEN
SELF_DOMAIN_OWNER = FORBIDDEN
SELF_DOMAIN_OWNER_RULE = CANONICAL_PRINCIPAL_EQUALITY
NO_ACCEPTED_LINKAGE_AUTHORITY = TREAT_DISTINCT_CANONICAL_PRINCIPALS_AS_DISTINCT
IMPLICIT_COMMON_CONTROL_INFERENCE = FORBIDDEN
FAIL_CLOSED_WHEN_LINKAGE_IS_ABSENT = NO
```

If authenticated actor Principal equals the new Owner Principal, reject. Distinct canonical UUIDs are treated as distinct; svc-workflow neither invents common control nor rejects solely because linkage proof is absent. Only an exact accepted external identity authority explicitly establishing linkage may extend the prohibition, and a child Spec must pin it. Runtime heuristics, Agent claims, request fields, or Feishu/service identity cannot.

## 10. Authentication, execution identity, and authorization

`svc-workflow` trusts only auth-service-verified token identity. The execution actor is the Agent Principal in verified `token.sub`, which must be the exact designated Admin Agent Principal UUID. The Client/intermediary, Feishu sender, conversation, display name, request-body actor, scope, allowlist, and Agent self-claim are not actor or permission facts.

Global role/binding state remains server-side and is not accepted from Agent-reported JWT claims or tool arguments. Each permission is independently evaluated on every protected request against the active repository designation and server-side binding. A disabled/revoked Principal or Client, invalid/expired token, inactive designation/binding, or missing permission fails closed.

```text
HUMAN_PRINCIPAL_ADMINISTRATION_REQUIRED_FOR_V1 = NO
HUMAN_OBO_REQUIRED_FOR_V1 = NO
TWO_PERSON_APPROVAL_REQUIRED_FOR_V1 = NO
HUMAN_ROOT_REQUIRED_FOR_V1 = NO
LONG_TERM_MULTI_HUMAN_GOVERNANCE = DEFERRED_NOT_FORBIDDEN
```

## 11. Repository-owned trusted Admin Agent designation

A later docs-only authority must be established:

```text
AUTHORITY_ID = SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1
AUTHORITY_KIND = repository_owned_security_invariant
IMPLEMENTATION_AUTHORITY = none
OWNER = mayf3
```

It must record exactly:

```text
agentPrincipalId
clientId
grantedPermissions
activationTime
repository owner acceptance
supersedes
superseded_by
owners
```

`grantedPermissions` is a subset of the closed set `{GLOBAL_SCHEDULER_READ, GLOBAL_DOMAIN_ADMIN}`. No other permission or composite role is legal. The authority is inert while proposed/unmerged. It activates only after independent review, Owner acceptance, and merge to `main`. Runtime API, Feishu message, request body, Agent self-claim, token claim, database-only mutation, or deployment config cannot establish it.

```text
RUNTIME_SELF_GRANT = FORBIDDEN
RUNTIME_GRANT_API_REQUIRED = NO
DESIGNATION_EXPIRY_REQUIRED = NO
DESIGNATION_ROTATION = WHOLE_AUTHORITY_SUCCESSOR
DESIGNATION_REVOCATION = WHOLE_AUTHORITY_SUCCESSOR_OR_EMERGENCY_DISABLE
BREAK_GLASS_GRANT = NOT_SUPPORTED
```

Designation need not auto-expire. Access Tokens remain short-lived; Client secrets remain rotatable; Clients remain revocable; Principals and svc-workflow global bindings remain disableable. Revocation/disablement fails closed and creates a publication/commit barrier: the old Agent cannot release protected data or commit protected writes afterward.

Credential leak response order is:

1. revoke/rotate the auth-service Client;
2. disable the compromised global binding;
3. disable the Agent Principal if necessary;
4. write durable security audit;
5. grant no replacement Agent permission until a new designation successor is merged.

Agent replacement order is:

```text
old Agent revoke
-> new Agent Principal/Client verified
-> new root authority successor independently reviewed/Owner accepted/merged
-> new Agent active
```

Emergency disablement is containment, not a replacement grant. There is no runtime break-glass Agent.

## 12. Feishu single-user command ingress

```text
FEISHU_IS_AUTHORITY_SOURCE = NO
FEISHU_COMMAND_PROVENANCE = YES
SVC_WORKFLOW_EXECUTION_ACTOR = ADMIN_AGENT_PRINCIPAL
```

The Agent-core ingress gate must verify at least:

- exact trusted Feishu app;
- exact tenant;
- exact prebound single-user conversation;
- exact allowed sender identifier;
- verified event/message ID;
- timestamp, nonce, and replay protection.

These facts gate command admission and provide provenance in Agent core. They never become Human permissions or `svc-workflow` actor identity. The dedicated Agent calls svc-workflow with a direct token whose `sub` is its exact designated Agent Principal UUID.

Durable audit correlation must associate, without confusing authorization sources:

```text
Agent Principal UUID
Feishu tenant
conversation ID
sender ID
event/message ID
ingress correlation ID
svc-workflow request/receipt ID
```

Authorization uses only the verified Agent Principal and server-side active binding/designation. A Feishu sender ID is not `token.sub`; message text, mention, membership, and conversation binding cannot impersonate the actor. Feishu outage or rejected ingress creates no weaker fallback path.

## 13. Audit, failure, revocation barrier, retention, and idempotency

Every successful or authenticated-denied protected global operation requires durable audit, including scheduler and directory reads, Domain create/Owner replace, designation/grant activation, revoke/disable, and lifecycle/security actions. Unauthenticated denial follows existing authentication/security-audit semantics and never promotes unverified fields into actor facts.

Audit identifies actual authenticated Agent Principal, target/subject, independent permission/operation, decision/result, time, idempotency/correlation IDs, and non-sensitive reason codes. It carries Feishu provenance only when present. It does not copy Context, Submission, Assistance/supporting payload, `EventData`, credentials, tokens, Receipt bodies, unrestricted request/response bodies, or other sensitive content.

```text
AUDIT_RETENTION = 365_DAYS
FAILURE_POLICY = FAIL_CLOSED
AUDIT_PRODUCT_READ_API = NOT_SUPPORTED
EXTERNAL_AUDIT_EXPORT = NOT_SUPPORTED
```

A protected successful read must durably commit audit before data publication. If audit or authorization state is unavailable, no data is released. A protected write and required audit commit atomically. Revocation/disablement is rechecked at the publication/commit barrier; an older in-flight request cannot publish or commit after authority ends.

State-changing workflow/control-plane commands require client idempotency identities and canonical request comparison. Keys bind authenticated actor and complete command meaning. Same key/same request replays the original outcome; same key/different request conflicts without changing it. Conflicting writes serialize and workflow state versions are enforced where applicable. Facts/projections/events/receipts/audits commit atomically.

When a client cannot know whether the authoritative write committed, return `outcome_unknown` (or child-wire equivalent). Reconciliation retries only the exact same request with the same key; generating a new key and blindly retrying is forbidden. Revocation, Owner-replacement races, retries, and compatibility routes cannot bypass current authorization.

## 14. External ownership, technology, and retained trade-offs

### 14.1 auth-service

Auth-service owns global identity, authentication, Agent Principals, Clients/credentials, token issuance, resolution, revocation, and signing keys. svc-workflow neither signs tokens nor treats Feishu/body fields as identity. Direct machine access uses auth-service RS256/JWKS verification, `aud=svc-workflow`, and canonical Agent Principal UUID in `token.sub`, subject to the accepted bounded JWKS cache trade-off.

Agent-first V1 requires a separate accepted auth-service child authority supplying only the designated Agent's needed audience/scope/grant. It must not grant a Human or any other Agent, place business role authority in a self-reported JWT claim, or depend on PR #15/PR #2. This Product Direction references but does not govern auth-service.

### 14.2 dsh-agent-core and Feishu

Agent-core/integration layers own Feishu transport, ingress verification, Agent routing, capability manifests, credential brokering, request/receipt correlation, and Agent dispatch. They must resolve actual Agent identity from trusted credentials/process binding, never model/tool/body self-report. svc-workflow owns final authorization against its Principal and server-side permissions.

### 14.3 Upper layers and UI

`adc-v2` and other business products own Requirement, Todo, project, priority, task label, Article, Campaign, business rules, and long-lived business state. They may correlate `workflowInstanceId` and use accepted contracts but cannot mutate workflow storage or persist competing workflow state authority.

UI products own presentation/navigation/interaction and may show `GLOBAL_WORKFLOW_COORDINATOR` only as a label. Labels do not authorize. External message/email/webhook/integration delivery belongs to adapters/business services.

### 14.4 Explicit non-ownership

svc-workflow does not own UI rendering, upper-layer business objects/content, business-specific decision logic, identity proofing, credential/token issuance, Feishu identity/permission administration, outbound delivery, built-in LLM/Agent dispatch, generic task labels/Context-title scheduling, or unrestricted cross-Domain content.

Shared PostgreSQL infrastructure changes no ownership: workflow tables may be written only through accepted svc-workflow command boundaries.

### 14.5 Technology and layers

```text
Rust
Axum
PostgreSQL
sqlx
tokio
tower-http
```

- `domain`: typed IDs/entities/enums/commands/events/permissions/errors, without HTTP ownership.
- `application`: use-case orchestration and authorization over ports, without Axum request mechanics.
- `http`: routes, authentication adaptation, strict DTOs, wire validation, response/error mapping.
- `store`: PostgreSQL repositories, atomic transactions, concurrency, immutable facts/projections/audit, migrations.
- `auth`: RS256/JWKS verification and accepted credential adaptation, never issuance.

Global control-plane behavior cannot be UI-only filtering, post-read handler redaction, or adapter bypass of application/storage authorization.

### 14.6 Retained trade-offs

No normal physical DELETE exists for Instances; cancel/archive preserve history. Single PostgreSQL without read/write separation remains allowed. Offline JWKS has the accepted bounded revocation-cache window. Global schedulers intentionally lack full content, task label, and Context title. No break-glass grant exists. These trade-offs never weaken fail-closed authorization, audit, or Domain isolation.

## 15. Requirement ownership guide

| Requirement language | Owning product boundary |
|---|---|
| workflow, Instance, current node, advance, approval flow | svc-workflow |
| state machine, `ADVANCE`, `RETURN`, `TERMINATE` | svc-workflow |
| event sourcing, timeline, immutable workflow Event | svc-workflow |
| Definition/template/version publication/graph validation | svc-workflow |
| Domain and normal cross-Domain isolation | svc-workflow |
| assigned worklist and creator-owned draft | svc-workflow |
| Instance cancel/archive | svc-workflow |
| Requirement/Todo/task board/article/campaign/business rule | owning upper-layer product |
| UI presentation/interaction | UI product |
| identity proofing/token issuance | auth-service |
| Feishu transport/notification/external delivery | integration layer |

## 16. Conformance debt at the authoring coordinates

V5 preserves V4's record of, and does not excuse, these gaps:

### svc-workflow

- global query still reads Context title and may return terminal/archived records;
- protected global read lacks the complete disabled-Principal gate and durable audit-before-publication;
- legacy composite Coordinator role/binding remains;
- no-self-grant and no-self-owner are not comprehensively enforced;
- separated permission lifecycle is absent;
- minimum Human/Agent selection directory is absent;
- existing Domain-admin surface still requires narrowing and verification.

### dsh-agent-core

- no accepted fleet-level svc-workflow scheduler capability;
- no accepted workflow Domain-admin capability manifest;
- no complete exact Feishu sender/app/tenant/conversation allowlist and replay gate for this route;
- Notification Ingress is currently a thin delivery adapter, not authority for administrative command ingress;
- actual actor must come from trusted credential/process resolution; Agent self-report is never authority.

### auth-service

- Agent Principal, Client, direct token, identity resolution, rotation, and revoke are reusable;
- designated global permission supply still requires an independent accepted child authority;
- Human Principal PR #15 is not a prerequisite;
- Agent-first V1 must not modify the Human route.

No current code, contract bundle, runtime, or report is deemed conforming by this list. Child compliance must use exact implementation and environment coordinates.

## 17. Original CTO bounded one-time Principal successor migration retained unchanged

V5 preserves and fully restates V4’s retained CTO bounded exception without changing its pair, scope, counts, child authority, or production gate and without creating ordinary reassignment:

```text
AMENDMENT_ID = PBV2-ONE-TIME-SUCCESSOR-001
MIGRATION_KIND = ONE_TIME_SUCCESSOR
OLD_PRINCIPAL = 3e2439d2-fb54-44f5-afee-77aa17c40d22
NEW_PRINCIPAL = 4e5a4578-0645-4133-bd35-b80e453dfee9
ORDINARY_REASSIGNMENT = NOT_AUTHORIZED
HANDOFF = NOT_AUTHORIZED
DELEGATION = NOT_AUTHORIZED
GENERAL_REASSIGNMENT_API = NOT_AUTHORIZED
LONG_LIVED_PRODUCT_CAPABILITY = NO
PRODUCTION_APPLY_AUTHORIZED = NO
```

The offline tooling is fixed to that pair, cannot accept arbitrary UUIDs, and fails closed on scope drift. It may transfer only the independently reviewed nine currently enabled Domain authorities and the execution-time workflow responsibility that remains active, non-terminal, currently assigned to OLD, and exactly present in the reviewed plan. Any live mismatch conflicts with zero writes.

Historical attribution is immutable. Existing Visits/assignments are never updated, deleted, or relabeled. Exactly zero of the known 58 historical assignments and zero of the known 111 historical Visits are migrated/rewritten. Successor responsibility is represented only through newly appended Visit, Event, Receipt, and Audit facts.

Nine Domain transfers, eligible current-responsibility facts, projection changes, Receipt completion, and audit commit atomically. Partial success is forbidden. Exact rerun after success is NOOP with zero writes and zero new audits; mismatched metadata/post-state fails closed. No HTTP/SDK/reusable reassignment, handoff, delegation, or arbitrary-pair surface is created.

Implementation remains bounded by accepted `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1`. Production apply still requires a separate independently reviewed execution gate pinning implementation SHA, clean checkout, reviewed plan/digest, database identity, operator identity, and fail-closed preconditions. Open PR #7 affects only its stated replay closure and remains independently governed.

## 17A. Additive trusted-fleet exact-plan-bound bounded successor exception

### 17A.1 Frozen canonical plan artifact

V5 preserves V4's authority for only the exact frozen local evidence artifact and its reviewed contents:

```text
PLAN_SCHEMA = workflow_trusted_fleet_successor_plan_v2
PLAN_PATH = /Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json
PLAN_SIZE_BYTES = 540472
PLAN_SHA256 = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606
ROSTER_SHA256 = f046d18f76da838ba94775af7c960d0ee548f2e392c22e6c7b0e3add36cb8e5f
PLAN_MODE = READ_ONLY_CANONICAL_PLAN
PLAN_CANONICAL = YES
SNAPSHOT_OBSERVED_AT_UTC = 2026-08-24T01:03:53.192875+00:00
PLAN_PRODUCTION_CHANGE = NONE
PLAN_SUPERSEDES_SCHEMA = workflow_trusted_fleet_successor_plan_v1
PLAN_SUPERSEDES_SHA256 = 57f769d0bc9f0a4494dd37685da3cc8657b2dc5845f020858457dbecc35ce9b7
```

The artifact is a read-only canonical production snapshot; it performs and authorizes zero writes. Any future operator binds to `PLAN_SHA256` exactly. A byte-different plan, an unverified digest, or any runtime-supplied `OLD`/`NEW` parameter is outside authority. The artifact remains a local evidence file and is not committed by this PR; only its digest is frozen here. The frozen roster source is the dsh-agent-core account-recovery evidence file `docs/evidence/account-recovery-phase-a-20260823/plan-production-20260823015745Z.json` bound by `ROSTER_SHA256` and file sha256 `32d0b23753370156150babcaf0b108ad1d8c2b28f952e9586cf700142f9ec852`; the roster is itself frozen and is never dynamically expanded.

### 17A.2 Exact fleet authority and frozen counts

The exception authorizes only the exact successor pairs, Domain tuples, and active responsibility tuples contained in the frozen artifact:

```text
TOTAL_NEW_AGENTS = 86
EXACT_SUCCESSOR_PAIR_COUNT = 86
AMBIGUOUS_COUNT = 0
CONFLICT_COUNT = 0

WORKFLOW_PROJECTION_CREATE_COUNT = 85
WORKFLOW_PROJECTION_PRESENT_COUNT = 1

DOMAIN_OWNER_TRANSFER_COUNT = 8
DOMAIN_MEMBER_TRANSFER_COUNT = 752
DOMAIN_TUPLE_COUNT = 760

ACTIVE_RESPONSIBILITY_TRANSFER_COUNT = 80

DRAFT_OWNERSHIP_CANDIDATE_COUNT = 99
DRAFT_OWNERSHIP_MIGRATION_COUNT = 0
```

These counts are frozen authoring scope bound to the artifact digest — never live database truth and never permission to force counts at apply time. Every future apply re-verifies live state against the exact artifact rows and fails closed on drift.

For each exact pair, the only permitted future transaction shape is:

1. verify the NEW Auth Principal is exact and `active`;
2. NEW Workflow projection: create when missing, or require exact match when present;
3. transfer the reviewed Domain authority tuples;
4. transfer the reviewed active responsibility tuples;
5. write Event / Receipt / Audit;
6. verify the exact terminal state.

No step accepts arbitrary `OLD`/`NEW` input; identities come only from the frozen artifact rows.

### 17A.3 Duplicate identity exclusion and canonical pairs

Exactly one identity is excluded as a duplicate:

```text
EXCLUDED_AGENT_ID = efficiency-agent
EXCLUDED_PRINCIPAL_ID = d09f8849-073c-484a-978c-f375113c28b2
EXCLUDED_CLASSIFICATION = EXCLUDED_DUPLICATE_IDENTITY
EXCLUDED_MIGRATION_CANDIDATE = false
EXCLUDED_FUTURE_OPERATOR_WRITES = 0
```

The excluded identity is a disabled, non-canonical duplicate with an existing projection, zero enabled Domain bindings, and zero Visits. A future operator encountering it commits zero writes. The only canonical efficiency pair is:

```text
efficiency-manager / 95eab282-22c7-46a2-8580-abfef4942cdc
  -> agt_efficiency-agent / b21ddb23-42f6-47c4-a27f-bc44950e554c
```

The artifact also fixes, as two fully independent pairs:

```text
build-in-public-agent -> agt_build-in-public-agent
blog-agent -> agt_blog-agent
```

`blog-agent` pairs only with `agt_blog-agent`, never with `agt_build-in-public-agent`; agent labels remain explanatory provenance and are never runtime identity selectors.

### 17A.4 Workflow projection creation

Future apply must first resolve the current systemic worklist failure: 85 of 86 NEW Auth Principals are absent from the svc-workflow Principal projection, so assigned-to-me reads return `404 principal_not_found`. The one present projection is exactly `agt_build-in-public-agent`.

Each missing projection is created only for the exact NEW Principal in the frozen artifact. Forbidden:

- dynamically enumerating an 87th identity from Auth;
- display-name pairing;
- deriving an OLD identity by stripping the `agt_` prefix from a NEW agent_id;
- fuzzy or prefix matching;
- creating the excluded identity's projection or binding it anywhere;
- replacing any Auth Principal UUID.

Stripping the `agt_` prefix never constitutes mapping evidence; display-name, prefix, and fuzzy matching remain forbidden; exact pairs come only from the frozen artifact.

After creation, verification must show `workflow_my_tasks` no longer returns `principal_not_found`; with no current tasks, the correct terminal state is exactly HTTP 200 with `items = []`.

### 17A.5 Domain transfer

Only the 760 exact tuples in the artifact are processed.

`DOMAIN_OWNER`:

- exact OLD -> NEW transfer;
- atomic Owner replacement;
- never commits dual Owner;
- Domain unchanged.

`DOMAIN_MEMBER`:

- enable NEW;
- disable OLD;
- Domain and Role unchanged;
- never commits long-lived dual authority.

Any tuple drift — missing, additional, disabled, role-changed, Principal-changed, or otherwise mismatched against the artifact — sets that pair's writes to 0 and the outcome to `CONFLICT`.

### 17A.6 Active responsibility transfer

Only the 80 exact responsibility tuples are processed, and each is re-validated at apply time: the current Visit is current, active, non-terminal, not cancelled, not archived, currently assigned to OLD, and its expected workflow state version matches the artifact.

Apply must:

- append a same-node successor Visit;
- append a dedicated Event;
- append a Receipt;
- append durable Audit;
- CAS the workflow state version;
- preserve Instance and node;
- keep historical Visits immutable;
- keep historical assignments immutable.

Terminal, completed, cancelled, or archived records are never migrated or reactivated.

### 17A.7 Creator-owned drafts

The 99 creator-owned draft tuples keep `created_by_principal_id` unchanged:

```text
DRAFT_CREATOR_HISTORY_IMMUTABLE = YES
DRAFT_SUCCESSOR_MIGRATION = FORBIDDEN
```

If a "current maintainer" concept is ever needed, it requires a separate future draft-stewardship capability; this boundary never silently overwrites creator attribution.

### 17A.8 Canary and execution sequence

The complete future sequence is:

1. accepted fleet Product Boundary;
2. accepted local implementation Child;
3. independently reviewed operator;
4. production read-only plan recheck;
5. exact `PLAN_SHA256` review;
6. separate production apply authorization;
7. canary 1: `agt_build-in-public-agent`;
8. canary 2: `agt_efficiency-agent`;
9. remaining exact 84 pairs;
10. verify;
11. exact rerun NOOP.

```text
SPEC_ACCEPTED != PRODUCTION_APPLY_AUTHORIZED
```

Each pair commits in its own independent SERIALIZABLE transaction. One pair's failure never fabricates another pair's success.

### 17A.9 No durable general capability

```text
CTO_BOUNDED_SUCCESSOR_AUTHORITY = PRESERVED_UNCHANGED
FLEET_BOUNDED_SUCCESSOR_AUTHORITY = ADDITIVE_EXACT_PLAN_BOUND_ONLY
ORDINARY_REASSIGNMENT = STILL_FORBIDDEN
HANDOFF = STILL_FORBIDDEN
DELEGATION = STILL_FORBIDDEN
GENERAL_SUCCESSOR_API = FORBIDDEN
GENERAL_MIGRATION_CAPABILITY = NO
RUNTIME_OLD_NEW_PARAMETERS = FORBIDDEN
DYNAMIC_ROSTER_EXPANSION = FORBIDDEN
ONLINE_MANAGEMENT_API = FORBIDDEN
HISTORICAL_REWRITE = FORBIDDEN
TERMINAL_TASK_REACTIVATION = FORBIDDEN
PRODUCTION_APPLY_AUTHORIZED = NO
```

The retained CTO exception and the exact fleet mappings, including the Build in Public pair, are never abstracted into a caller-parameterized general migration mechanism.

### 17A.10 Blocked Child and PR disposition

Draft PR #9 is now dispositioned:

```text
PR_9_DISPOSITION = SUPERSEDED_BY_FLEET_LOCAL_CHILD
```

This round must not close, modify, or merge PR #9. Its single-pair Child meaning is superseded by the fleet boundary above; the future local implementation Child (sequence step 2) supersedes it and must carry its own independent review. This proposed V5 Head must receive a fresh independent fixed-head audit before any acceptance consideration; no earlier V4 review result transfers to V5.

## 18. Capability-scoped child authorities and ordering

No common all-Slices global gate is created. Each child authorizes only its own capability.

### Slice A — Dedicated Admin Agent identity

Create/select a new dedicated Agent; establish exact Agent Principal UUID and exact Client; verify enabled status, credential ownership, rotation, and revoke. This Product Direction performs none of these actions.

### Slice B — Trusted Agent designation root

Independently review, Owner-accept, and merge `SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1` with exact Agent/Client and split permissions.

### Slice C — auth-service permission supply

An independent accepted auth-service child authority supplies only the designated Agent's required audience/scope/grant. It grants no Human or other Agent, is versioned/auditable/idempotent/revocable, and does not put business role authority in self-reported JWT claims.

### Slice D — svc-workflow global scheduler read

One implementation-authorizing child may close the complete canonical scheduler field/permission/audit conformance debt. Separately, the bounded dispatchability child in §8A may add only the opt-in projection/filter to existing Domain/global lists. It must retain the accepted `GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR` compatibility gate, create no role/grant change, and cannot claim the full Slice is complete. A later role migration may remove legacy Coordinator compatibility only under its own accepted authority and compatibility plan.

### Slice E — svc-workflow Domain admin

An implementation-authorizing child Spec freezes Domain create, initial Owner, Owner replacement, no-self-grant, no-self-owner, atomic audit, minimum directory, and reconciliation of conflicting `IDENTITY_PROVISIONING_API_V0` semantics.

### Slice F — dsh-agent-core / Feishu command route

An independently owned authority freezes exact Feishu ingress gates, fleet scheduler capability, Domain-admin capability, Agent credential broker, and request/receipt correlation. It uses neither Human OBO nor body actor.

Slices may have dependency edges necessary for their own execution, but no Slice silently activates another. Assistance and Admin Recovery remain independent unless a child actually changes their data/semantics. Current HTTP/OpenAPI/SDK surfaces change only with their own accepted implementation authority.

## 19. Decisions

### DEC-V5-001 — Single-user dedicated Agent operation

- Decision owner: `mayf3`.
- Decision: daily global administration uses one new dedicated Agent Principal with direct token; Human runtime Principal/OBO/administration and two-person approval are not V1 prerequisites.
- Rejected: reuse an ordinary business/canary Agent; retain Human-root/two-approver V1 prerequisite.
- Owner input remaining: none.

### DEC-V5-002 — Repository designation replaces runtime grant governance

- Decision owner: `mayf3`.
- Decision: exact Agent/Client/split permissions are activated only by merged docs-only `SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1`; replacement uses whole-authority successor, with emergency disablement only for containment.
- Rejected: runtime self-grant, runtime grant API requirement, auto-expiring designation, break-glass grant.
- Owner input remaining: none.

### DEC-V5-003 — Split global permissions remain independent

- Decision owner: `mayf3`.
- Decision: preserve exactly the two split permissions and V2's bounded scheduler/Domain-admin semantics; the same Agent may hold both without union or third role.
- Rejected: composite Coordinator runtime role or alias.
- Owner input remaining: none.

### DEC-V5-004 — Feishu is gated provenance, never authority

- Decision owner: `mayf3`.
- Decision: exact single-user Feishu ingress gates command admission in Agent core; svc-workflow actor remains the designated Agent Principal.
- Rejected: Feishu sender as Human Principal or body actor; Human OBO prerequisite.
- Owner input remaining: none.

### DEC-V5-005 — Preserve V2 workflow and one-time successor boundaries

- Decision owner: `mayf3`.
- Decision: retain all V2 workflow, Domain, scheduler, Domain-admin, audit/idempotency, ownership, technology, trade-off, and exact one-time successor limits unless explicitly replaced by DEC-V5-001/002/004.
- Rejected: partial supersession or reader-side composition with V2.
- Owner input remaining: none.

### DEC-V5-006 — Preserve the exact fleet bounded exception

- Decision owner: `mayf3`.
- Decision: preserve the CTO exception unchanged and add only §17A's trusted-fleet exception bound to the exact frozen plan artifact — 86 exact successor pairs, 85 projection creations, 760 exact Domain tuples, 80 exact active responsibilities, 99 immutable creator-owned drafts — with per-pair SERIALIZABLE apply, canary-first ordering, fail-closed drift, exact NOOP rerun, and separate production authorization.
- Rejected: modifying the CTO pair; keeping only the single Build in Public pair; arbitrary Principal migration with runtime OLD/NEW arguments; dynamic roster expansion; ordinary reassignment/handoff/delegation; general successor API; online management API; historical rewrite/reactivation; count-forcing.
- Owner input remaining: none.

### DEC-V5-007 — Workflow owns objective dispatchability

- Decision owner: `mayf3`.
- Decision: Workflow derives objective current eligibility from existing formal facts at read time; HR/Scheduler selects whom to contact and owns all scheduling/message/session policy.
- Rejected: duplicate Workflow predicates in HR; persisted dispatch status; Workflow-owned dispatcher/wake policy.
- Owner input remaining: none.

### DEC-V5-008 — Reconcile current read role without grant expansion

- Decision owner: `mayf3`.
- Decision: `GLOBAL_WORKFLOW_READER` is the current server role implementing the `GLOBAL_SCHEDULER_READ` read surface; the bounded child retains the current global route gate and changes no role/grant. Legacy Coordinator acceptance is compatibility debt, not a third permission or new grant path.
- Rejected: ignore the accepted reader Spec; grant Coordinator; introduce a new role in the dispatchability child.
- Owner input remaining: none.

### DEC-V5-009 — Dispatchability representation is explicit opt-in

- Decision owner: `mayf3`.
- Decision: omitted `dispatchableOnly` preserves legacy response structure; explicit `true|false` selects the paired projection fields, and `true` additionally filters before pagination.
- Rejected: always-present fields on legacy requests; version an entire endpoint before the bounded capability; post-page filtering.
- Owner input remaining: none.

## 20. Normative Contracts

### CTR-V5-001 — Whole-authority lifecycle
V5 MUST remain non-active while proposed/unmerged, MUST replace all V4 meaning only through an atomic accepted transition with V4 backlink/authority-map updates, and MUST NOT authorize implementation or production apply directly.

### CTR-V5-002 — Serial workflow product shape
svc-workflow MUST preserve §5's single-current-node/current-assignee deterministic serial workflow shape and MUST NOT add the excluded orchestration capabilities without later authority.

### CTR-V5-003 — Definition lifecycle and immutability
Definition ownership, lifecycle, publication immutability, Domain locality, and non-destructive archive MUST satisfy §5.1.

### CTR-V5-004 — Instance and immutable history
Instance ownership, non-physical deletion, immutable Context/Visit/Submission/Event facts, projection semantics, and one-version/one-Event atomic state command MUST satisfy §§5.2-5.4.

### CTR-V5-005 — Transition actor and atomicity
Only the authorized current assignee MAY perform normal Transition; global permission/designation MUST NOT imply it, and all resulting facts/outcome/audit MUST commit atomically.

### CTR-V5-006 — Strict normal Domain isolation
Ordinary Agent/member/Owner access, lookup/list/count/cursor/denial/serialization MUST preserve §6 isolation; only enumerated global permissions MAY cross Domains.

### CTR-V5-007 — Domain-local views and administration
Ordinary worklists, history, Owner views, membership, Definition governance, and Visit snapshots MUST remain bounded as in §6 and MUST NOT inherit global authority.

### CTR-V5-008 — Exactly two split global permissions
Authorization MUST preserve the two Product Direction capabilities `GLOBAL_SCHEDULER_READ` and `GLOBAL_DOMAIN_ADMIN` independently. `GLOBAL_WORKFLOW_READER` is the accepted current server role for the scheduler-read surface, not a third product permission; neither it nor legacy Coordinator compatibility may imply Domain-admin or write authority.

### CTR-V5-009 — Scheduler core allowlist plus opt-in pair
The fully conformant canonical scheduler record MUST use only and all §8 core fields. A dispatchability-aware Domain/global response MAY additionally expose exactly `dispatchable` and `dispatch_blocked_reasons`, required together only when `dispatchableOnly` is explicitly present.

### CTR-V5-010 — Scheduler population and compatibility
`dispatchableOnly=true` MUST return only objectively eligible active ordinary current-assignee rows. Omitted `dispatchableOnly` MUST preserve the existing endpoint population and response structure; this compatibility rule does not declare broader legacy populations fully conformant with the canonical scheduler Slice.

### CTR-V5-011 — Scheduler sensitive-content exclusion
Global/Domain dispatchability MUST expose only the closed non-sensitive codes authorized by §8. Context/title, task label, Submission/history, EventData, Assistance content/status/body, credentials, Receipt/audit content, transition options, and content-derived fields remain forbidden. `ASSISTANCE_OPEN` MAY reveal only the existence of the formal blocker.

### CTR-V5-012 — Domain-admin allowed surface
`GLOBAL_DOMAIN_ADMIN` MUST authorize only idempotent Domain create, atomic initial Owner/disabled fallback, atomic Owner replacement, and §9's minimum selection directory.

### CTR-V5-013 — Domain-admin excluded surface
The permission MUST NOT grant workflow content/write/Transition/cancel/archive/Definition/membership/audit-content/other Domain writes or infer actor from body/Feishu/display/service/scope/allowlist.

### CTR-V5-014 — No self-grant and no self-Owner
The Agent MUST NOT grant itself permission or set its own canonical Principal as Domain Owner, directly or through aliases/chains/retries/migrations; distinct UUIDs remain distinct absent exact accepted linkage authority.

### CTR-V5-015 — Dedicated actual Agent actor
The runtime actor MUST be the exact designated Admin Agent Principal in verified direct-token `sub`; ordinary Agents, Humans, Clients/intermediaries, Feishu senders, bodies, display names, claims, and self-report MUST NOT substitute.

### CTR-V5-016 — Server-side independent authorization and fail closed
Each protected request MUST evaluate active designation and server-side split binding; disabled/revoked Principal or Client, invalid/expired token, missing permission, inactive designation/binding, or unavailable authorization MUST fail closed.

### CTR-V5-017 — Trusted designation root contents and activation
`SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1` MUST contain every field and only closed permissions in §11 and MUST activate only after independent review, Owner acceptance, and merge to main; runtime inputs cannot create it.

### CTR-V5-018 — Designation rotation and emergency lifecycle
Designation has no required expiry; replacement/revocation MUST follow whole-authority successor or emergency disable containment, retain short token/rotatable secret/revocable Client/disableable Principal and binding controls, and provide no break-glass grant.

### CTR-V5-019 — Compromise and replacement order
Credential compromise and Agent replacement MUST follow §11's exact containment and successor order; no replacement Agent MAY receive authority before its merged root successor.

### CTR-V5-020 — Exact Feishu ingress gate
Agent core MUST admit administrative commands only for the exact app, tenant, prebound conversation, allowed sender, verified event/message ID, timestamp/nonce, and replay checks in §12.

### CTR-V5-021 — Feishu provenance is not authorization
Feishu/message facts MUST remain provenance only; svc-workflow MUST authorize only actual Agent Principal/server binding and MUST NOT treat sender ID as actor or Human permission.

### CTR-V5-022 — End-to-end correlation
Durable records MUST correlate the Agent Principal and every Feishu/request/receipt identifier enumerated in §12 without storing sensitive bodies or confusing provenance with authorization.

### CTR-V5-023 — Durable audit coverage
Successful and authenticated-denied global reads, Domain create/Owner replace, designation/grant/revoke/disable actions MUST produce durable non-sensitive audit identifying the actual authenticated Agent Principal.

### CTR-V5-024 — Audit-before-read and atomic-write failure policy
Protected read audit MUST be durable before data publication; protected writes and audit MUST be atomic; audit/authorization failure MUST release/commit nothing.

### CTR-V5-025 — Revocation/disablement publication barrier
Revocation or disablement before publication/commit MUST prevent older in-flight reads from releasing data and writes from committing; the old Agent MUST cease operating.

### CTR-V5-026 — Idempotency and outcome reconciliation
Same-key/same-request MUST replay; same-key/different-request MUST conflict; `outcome_unknown` MUST reconcile only by exact same-key/same-request retry, never a blind new key.

### CTR-V5-027 — Retention and sensitive audit exclusion
Required audit MUST be retained exactly 365 days and MUST exclude §13 sensitive content; no product audit-read API or external export is authorized.

### CTR-V5-028 — External ownership and direct-token supply
Auth-service/Agent-core/upper-layer/UI ownership MUST remain as §14 states. Agent-first permission supply requires independent child authority and MUST NOT depend on Human PR #15 or legacy PR #2.

### CTR-V5-029 — Layer and storage trust boundary
The technology/layer/storage boundary in §14 MUST be preserved; global security MUST NOT rely on UI filtering, post-read redaction, adapter bypass, or shared-database access.

### CTR-V5-030 — Conformance debt remains unimplemented
No debt listed in §16 MAY be represented as compliant or implementation-authorized until exact accepted child Contracts and Contract-by-Contract evidence establish it.

### CTR-V5-031 — Capability-scoped child authority
Each Slice in §18 MUST have its own accepted authority before implementation and MUST NOT activate, broaden, or waive another Slice; no Human-governance common gate exists.

### CTR-V5-032 — Exact one-time successor scope
The retained migration MUST be offline and fixed to §17's exact pair, nine reviewed enabled Domain authorities, and exact live eligible current responsibility; drift MUST commit zero writes.

### CTR-V5-033 — Successor historical immutability and append-only transfer
The migration MUST rewrite zero historical assignments/Visits, preserve the known 58/111 exclusions, and represent successor responsibility only through new Visit/Event/Receipt/Audit facts.

### CTR-V5-034 — Successor atomic NOOP and no durable product surface
The retained migration MUST commit atomically, exact-rerun NOOP with zero writes/audits, fail closed on mismatched metadata/post-state, create no general API/capability, and retain separate implementation/production gates.

### CTR-V5-035 — Fleet exception binds only to the frozen plan
The additive exception MUST authorize only the exact rows of the artifact with `PLAN_SHA256 = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606` (§17A.1); any other bytes, digest mismatch, runtime `OLD`/`NEW` parameter, label-based selection, or roster expansion MUST be rejected before writes.

### CTR-V5-036 — Excluded duplicate identity and canonical pairs
The efficiency duplicate `efficiency-agent`/`d09f8849-073c-484a-978c-f375113c28b2` MUST remain excluded with `EXCLUDED_FUTURE_OPERATOR_WRITES = 0`; only the canonical pair `efficiency-manager` -> `agt_efficiency-agent` MAY transfer efficiency authority, and `blog-agent` MUST pair only with `agt_blog-agent`.

### CTR-V5-037 — Projection creation and worklist terminal state
Each of the 85 missing NEW Workflow projections MUST be created only from the artifact's exact NEW Principal; the already-present `agt_build-in-public-agent` projection MUST exactly match; after creation `workflow_my_tasks` MUST stop returning `principal_not_found` and MUST return HTTP 200 with `items = []` when no current tasks exist.

### CTR-V5-038 — Exact Domain tuple transfer
Only the artifact's 760 exact Domain tuples MAY transfer: `DOMAIN_OWNER` by atomic OLD->NEW replacement without dual Owner, `DOMAIN_MEMBER` by enable-NEW/disable-OLD with Domain and Role unchanged; any tuple drift MUST yield zero writes for that pair with outcome `CONFLICT`.

### CTR-V5-039 — Append-only active responsibility transfer
Only the artifact's 80 exact responsibility tuples MAY transfer, each re-validated at apply time (current, active, non-terminal, not cancelled, not archived, assignee OLD, expected state version matching); apply MUST append same-node successor Visit, dedicated Event, Receipt, and Audit, CAS the state version, preserve Instance and node, and rewrite zero historical facts.

### CTR-V5-040 — Creator-owned draft immutability
All 99 creator-owned draft tuples MUST keep `created_by_principal_id` unchanged (`DRAFT_CREATOR_HISTORY_IMMUTABLE = YES`, `DRAFT_SUCCESSOR_MIGRATION = FORBIDDEN`); any maintainer concept requires a separate future draft-stewardship capability.

### CTR-V5-041 — Per-pair transaction isolation, canary order, exact NOOP
Each pair MUST commit in one independent SERIALIZABLE transaction following the §17A.8 sequence (canary 1 `agt_build-in-public-agent`, canary 2 `agt_efficiency-agent`, then the remaining exact 84); one pair's failure MUST NOT fabricate another pair's success; an exact successful rerun MUST be NOOP with zero writes and zero new audits.

### CTR-V5-042 — Fleet plan-first separate production gate
The complete §17A.8 sequence MUST be enforced before any fleet write: accepted fleet Product Boundary, accepted local implementation Child, independently reviewed operator, production read-only plan recheck, and exact `PLAN_SHA256` review occur before a separate explicit production apply authorization; no earlier milestone authorizes apply.

### CTR-V5-043 — PR disposition without lifecycle change
PR #9 MUST retain disposition `SUPERSEDED_BY_FLEET_LOCAL_CHILD` without being closed, modified, or merged this round; V5 MUST receive a fresh independent audit and remain proposed with `implementation_authority = none` and `production_apply_authority = none`.

### CTR-V5-044 — Read-time derivation and no second state
Dispatchability fields, filter decision, returned row, and cursor MUST be derived from existing Workflow-owned facts in one list-query snapshot. V5 authorizes no persisted dispatch status, Event, lease, reservation, claim, execution token, notification state, or schema column.

### CTR-V5-045 — Closed reasons, privacy, and ops-lock gap
The child MUST freeze an exact closed reason enum backed by formal facts; `dispatchable` is true iff that ordered reason set is empty. It MUST NOT expose Assistance content, infer free-text/metadata, or emit `OPS_LOCKED` until a separate accepted authority creates a Workflow-owned primitive.

### CTR-V5-046 — Explicit opt-in wire compatibility
Omitted `dispatchableOnly` MUST preserve the legacy response structure with neither new field. Explicit `false` MUST emit both fields without filtering; explicit `true` MUST emit both fields and filter to empty-reason rows. Invalid values MUST fail before returning a page.

### CTR-V5-047 — Pagination and filter ordering
Dispatchability filtering MUST compose by `AND` with existing filters before order/limit/cursor selection. Response pagination remains `items + next_cursor`; callers walk until null. `page`, `totalPages`, `totalCount`, offset pagination, and post-page filtering are forbidden.

### CTR-V5-048 — Authorization and no grant expansion
Domain list authorization remains Domain Owner scoped. Global list authorization retains the accepted current `GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR` compatibility gate and error behavior. The child MUST NOT add/change roles, grants, scopes, identities, credentials, allowlists, or write reach, and MUST NOT authorize a new Coordinator grant.

### CTR-V5-049 — Advisory snapshot, final command authority, and caller policy
`dispatchable=true` is advisory for the observed snapshot only. It MUST NOT authorize/bypass Transition, assignee, state-version, idempotency, lock, or command validation. Workflow MUST NOT choose contact order, fairness, quota, priority, retries, Agent mapping, messaging, Session, or wake behavior.

### CTR-V5-050 — Parent/child lifecycle
V5 authorizes no implementation directly. The dispatchability child MUST be revised against accepted V5, independently audited, accepted, and merged before code starts; it MUST keep unrelated full Slice-D conformance debt explicit.

## 21. Acceptance

Every item requires executed evidence at the implementation/authority revision named by its child; a test definition or prose assertion alone is not evidence.

### ACC-V5-001 — Lifecycle and supersession check
- Contracts: `CTR-V5-001`.
- Method/environment: repository frontmatter/backlink/map review on proposed and any later acceptance commits.
- Expected: proposed V5 is inactive; any later acceptance changes V4 backlink/map atomically while implementation and production apply authority remain none.
- Required evidence: exact Git diffs, reviewed commits, Owner receipt, final-head recheck, and main merge coordinate.
- Failure condition: proposed V5 is called active/accepted, V4 is partially superseded, or implementation/production apply is authorized.

### ACC-V5-002 — Serial-shape negative matrix
- Contracts: `CTR-V5-002`.
- Method/environment: child-spec contract and executable capability matrix.
- Expected: only §5 serial shape is available.
- Required evidence: exact endpoint/domain tests and implementation mapping.
- Failure condition: parallel/dynamic/claim/timer/retry/SLA/script/LLM/general reassignment capability becomes available.

### ACC-V5-003 — Definition lifecycle matrix
- Contracts: `CTR-V5-003`.
- Method/environment: integration tests over all Definition states and cross-Domain attempts.
- Expected: §5.1 lifecycle/immutability/Domain ownership holds.
- Required evidence: executed state matrix and storage diff.
- Failure condition: published bytes mutate, invalid state creates Instances, archive rewrites facts, or Definition is shared cross-Domain.

### ACC-V5-004 — Instance/history integrity
- Contracts: `CTR-V5-004`.
- Method/environment: transactional and history-replay integration tests.
- Expected: no physical delete; immutable facts and projection agree; one version/Event per success.
- Required evidence: executed queries and commit coordinates.
- Failure condition: fact rewrite/delete, partial commit, duplicate/missing Event, or projection/history divergence.

### ACC-V5-005 — Transition authority matrix
- Contracts: `CTR-V5-005`.
- Method/environment: test current assignee, Owner, globally authorized Agent, and unrelated Principal.
- Expected: only current assignee succeeds and transaction is atomic.
- Required evidence: executed auth matrix and database audit.
- Failure condition: Admin Agent/global permission transitions solely by that status or any partial fact commits.

### ACC-V5-006 — Cross-Domain noninterference
- Contracts: `CTR-V5-006`.
- Method/environment: ordinary Agent/Owner lookup/list/count/cursor/error/serialization matrix across two Domains.
- Expected: no cross-Domain fact or existence leak.
- Required evidence: executed responses and query/audit traces.
- Failure condition: an ordinary Agent or role combination obtains global access or any observable cross-Domain leak.

### ACC-V5-007 — Domain-local view boundary
- Contracts: `CTR-V5-007`.
- Method/environment: worklist/history/Owner/admin test matrix including Owner replacement.
- Expected: views remain participation/Domain scoped and old Visit snapshots remain unchanged.
- Required evidence: executed responses and immutable rows.
- Failure condition: old Owner retains access, another Domain appears, or Owner change rewrites Visit.

### ACC-V5-008 — Split-permission/current-role matrix
- Contracts: `CTR-V5-008`, `CTR-V5-048`.
- Method/environment: product-permission review plus current server gate tests for Reader, Coordinator, both, and neither.
- Expected: Reader remains global-GET-only; no role implies Domain-admin/write; no new Coordinator grant or role change occurs.
- Required evidence: accepted authority graph, executed allow/deny matrix, and role-binding diff.
- Failure condition: a third product permission appears, Reader/Coordinator gains write, gate changes silently, or grants change.

### ACC-V5-009 — Scheduler schema/representation allowlist
- Contracts: `CTR-V5-009`, `CTR-V5-046`.
- Method/environment: schema/property tests for omitted, explicit false, and explicit true.
- Expected: omitted retains legacy keys; explicit values add exactly the paired fields; fully conformant scheduler records retain all §8 core fields.
- Required evidence: executed response-key snapshots and schema scanner.
- Failure condition: one field appears alone, legacy requests gain fields, or unlisted data appears.

### ACC-V5-010 — Dispatchable population filter
- Contracts: `CTR-V5-010`, `CTR-V5-044`, `CTR-V5-047`.
- Method/environment: interleaved active, DRAFT, cancelled, archived, terminal, no-assignee, disabled-assignee, blocked-version, and open-Assistance fixtures with small pages.
- Expected: `true` returns only empty-reason active ordinary rows with no duplicates/misses; omitted preserves legacy population.
- Required evidence: executed cursor walk and source fixture IDs.
- Failure condition: blocked row passes, eligible row is skipped by post-page filtering, or omitted behavior changes.

### ACC-V5-011 — Scheduler sensitive-field/reason privacy
- Contracts: `CTR-V5-011`, `CTR-V5-045`.
- Method/environment: seeded markers in every forbidden source plus exact reason-enum scan.
- Expected: only closed codes appear; `ASSISTANCE_OPEN` leaks no status/body/content; no metadata-derived or ops-lock reason exists.
- Required evidence: response corpus and marker/key scanner.
- Failure condition: forbidden content/derived field appears or undocumented metadata changes eligibility.

### ACC-V5-012 — Domain-admin allowed operation matrix
- Contracts: `CTR-V5-012`.
- Method/environment: create with Owner, disabled fallback, replace Owner, and directory integration tests.
- Expected: only enumerated operations/fields work; Owner invariant is atomic.
- Required evidence: executed responses, rows, and audits.
- Failure condition: enabled ownerless Domain, partial replacement, extra directory data, or non-idempotent replay.

### ACC-V5-013 — Domain-admin excluded operation matrix
- Contracts: `CTR-V5-013`.
- Method/environment: attempt every excluded read/write using only global Domain-admin permission.
- Expected: all are denied without side effect or sensitive audit content.
- Required evidence: executed denial matrix and unchanged-state proof.
- Failure condition: workflow content/transition/reassignment/cancel/archive/Definition/membership/audit-content/other write succeeds.

### ACC-V5-014 — Self-grant/self-Owner attacks
- Contracts: `CTR-V5-014`.
- Method/environment: direct, alias, chained, retry, migration, same-UUID, distinct-UUID, and unproven-linkage cases.
- Expected: self-grant/same-Principal Owner denied; distinct Principals allowed unless exact accepted linkage applies.
- Required evidence: executed cases and canonical identity traces.
- Failure condition: Agent self-grants, sets itself Owner, evades via alias, or implementation invents common control.

### ACC-V5-015 — Actual actor anti-forgery matrix
- Contracts: `CTR-V5-015`.
- Method/environment: valid dedicated token plus ordinary Agent/Human/Client/Feishu/body/display/JWT-role/tool-argument forgery attempts.
- Expected: only exact token `sub` is actor; only designated Agent can proceed.
- Required evidence: verified claims, authorization trace, and durable audit actor.
- Failure condition: ordinary Agent gains global permission or any request body/self-report/Feishu field substitutes as admin actor.

### ACC-V5-016 — Disabled/revoked fail-closed matrix
- Contracts: `CTR-V5-016`.
- Method/environment: disable/revoke each Principal, Client, token, designation, binding, and permission; inject authorization-store failure.
- Expected: protected operation denies and releases/commits nothing.
- Required evidence: executed responses, publication checks, and state/audit traces.
- Failure condition: disabled Agent still reads/writes or unavailable authorization fails open.

### ACC-V5-017 — Root authority activation gate
- Contracts: `CTR-V5-017`.
- Method/environment: repository lifecycle and malformed-authority negative tests.
- Expected: all exact fields/closed permissions present; proposed/unmerged/runtime-created roots are inert.
- Required evidence: exact review/acceptance/merge and activation trace.
- Failure condition: runtime API/message/self-claim activates designation, missing field passes, or an unmerged authority grants access.

### ACC-V5-018 — Designation lifecycle controls
- Contracts: `CTR-V5-018`.
- Method/environment: token expiry, secret rotation, Client revoke, Principal/binding disable, attempted break-glass and runtime replacement.
- Expected: controls fail closed; only successor activates replacement; no break-glass exists.
- Required evidence: executed lifecycle matrix and audits.
- Failure condition: indefinite token, unrotatable/unrevocable credential, runtime grant/replacement, or break-glass authority.

### ACC-V5-019 — Compromise/replacement sequencing
- Contracts: `CTR-V5-019`.
- Method/environment: staged incident and Agent replacement rehearsal.
- Expected: old Agent stops before replacement; replacement remains denied until merged successor.
- Required evidence: ordered timestamps, revocation/binding/audit/root/main coordinates.
- Failure condition: old Agent continues after replacement/revoke or new Agent acts before successor merge.

### ACC-V5-020 — Feishu exact-ingress matrix
- Contracts: `CTR-V5-020`.
- Method/environment: vary app, tenant, conversation, sender, event signature/ID, timestamp, nonce, and replay.
- Expected: only exact fresh verified single-user ingress is admitted once.
- Required evidence: executed Agent-core gate matrix and ingress audit.
- Failure condition: non-owner sender, wrong app/tenant/conversation, stale/duplicate/unsigned event reaches command execution.

### ACC-V5-021 — Feishu provenance/actor separation
- Contracts: `CTR-V5-021`.
- Method/environment: send valid command while forging sender/body actor and compare token/audit identity.
- Expected: Agent Principal remains actor; Feishu values remain provenance.
- Required evidence: token verification, svc-workflow auth trace, audit record.
- Failure condition: Feishu sender ID becomes svc-workflow actor/permission or Human OBO is required.

### ACC-V5-022 — Correlation completeness
- Contracts: `CTR-V5-022`.
- Method/environment: end-to-end accepted and denied Feishu commands.
- Expected: all enumerated IDs correlate without sensitive bodies.
- Required evidence: joined durable records and redaction scan.
- Failure condition: missing correlation edge, actor/provenance conflation, or sensitive content copied.

### ACC-V5-023 — Protected audit coverage
- Contracts: `CTR-V5-023`.
- Method/environment: success/denial matrix for read/create/replace/designate/revoke/disable.
- Expected: one durable accountability record per required attempt with actual Agent actor.
- Required evidence: executed operation/audit joins.
- Failure condition: required success/denial lacks durable audit or records a self-reported actor.

### ACC-V5-024 — Audit failure and atomicity
- Contracts: `CTR-V5-024`.
- Method/environment: inject audit failure before read publication and during write transaction.
- Expected: read returns no protected data; write and audit both roll back.
- Required evidence: network publication capture and database transaction proof.
- Failure condition: audit fails but data is returned or write commits without audit.

### ACC-V5-025 — Publication/commit revocation race
- Contracts: `CTR-V5-025`.
- Method/environment: pause requests after initial check, revoke/disable, then release.
- Expected: no response data/no protected commit; old Agent cannot operate.
- Required evidence: synchronized race trace and state/audit results.
- Failure condition: prechecked in-flight request publishes/commits after revoke/disable.

### ACC-V5-026 — Idempotency/unknown-outcome matrix
- Contracts: `CTR-V5-026`.
- Method/environment: same/different request-key concurrency and induced lost response after commit.
- Expected: replay/conflict/exact same-key reconciliation semantics.
- Required evidence: executed receipts, hashes, outcomes, and row counts.
- Failure condition: same-key/different-request mutates, same request double-commits, or client retries unknown outcome with a new key.

### ACC-V5-027 — Audit retention/redaction
- Contracts: `CTR-V5-027`.
- Method/environment: retention boundary and seeded-sensitive-marker scan.
- Expected: exact 365-day policy; no forbidden body/credential; no runtime read/export API.
- Required evidence: lifecycle execution and API/schema inventory.
- Failure condition: early deletion, configurable longer retention under V5, sensitive content, or audit read/export surface.

### ACC-V5-028 — External authority and PR disposition
- Contracts: `CTR-V5-028`.
- Method/environment: child-authority dependency graph review at exact revisions.
- Expected: Agent permission child is independent; PR #15 remains deferred/non-prerequisite/non-active; PR #2 independent.
- Required evidence: exact external authority/PR/main coordinates.
- Failure condition: unmerged PR #15 is treated as active/prerequisite, PR #2 blocks route, Human/other Agent is granted, or local Spec governs external behavior.

### ACC-V5-029 — Layer/trust bypass scan
- Contracts: `CTR-V5-029`.
- Method/environment: architecture/source query path and direct-storage/adaptor attack review.
- Expected: authorization/redaction enforced before broad read and through application/store boundaries.
- Required evidence: call graph, query projection, access tests.
- Failure condition: UI/handler-only redaction, adapter bypass, or direct shared-DB mutation authorizes behavior.

### ACC-V5-030 — Drift truth check
- Contracts: `CTR-V5-030`.
- Method/environment: exact-base conformance report against each debt item.
- Expected: unresolved items remain DRIFTED/UNKNOWN, never VERIFIED without qualified evidence.
- Required evidence: Contract-level conformance table.
- Failure condition: existing partial implementation is declared V5-compliant by existence, tests, or runtime alone.

### ACC-V5-031 — Slice non-escalation graph
- Contracts: `CTR-V5-031`.
- Method/environment: authority/dependency/activation review plus partial-deployment tests.
- Expected: each Slice enables only itself; missing unrelated Human/Assistance/Recovery authority does not block; missing own child does.
- Required evidence: accepted authority graph and per-Slice activation results.
- Failure condition: one Slice silently activates another or a common Human gate is imposed.

### ACC-V5-032 — Exact successor scope drift
- Contracts: `CTR-V5-032`.
- Method/environment: reviewed plan, exact pair/nine rows, changed pair/row/live-current fixtures.
- Expected: only exact eligible plan succeeds; every drift commits zero.
- Required evidence: executed plan digest, pre/post rows, Receipt/audit.
- Failure condition: arbitrary pair, non-nine Domain scope, historical/ineligible responsibility, or drift commits.

### ACC-V5-033 — Successor history preservation
- Contracts: `CTR-V5-033`.
- Method/environment: pre/post digest and row-level history comparison.
- Expected: 58 historical assignments and 111 Visits unchanged; only new successor facts appended.
- Required evidence: executed digests, row counts, new fact lineage.
- Failure condition: any historical attribution is updated/deleted/relabeled or successor lacks append-only facts.

### ACC-V5-034 — Successor atomic NOOP/surface/gates
- Contracts: `CTR-V5-034`.
- Method/environment: failure injection, exact rerun, mismatched rerun, API/SDK inventory, production-gate review.
- Expected: all-or-nothing; exact rerun zero writes/audits; no general surface; production remains separately gated.
- Required evidence: executed transactions, counts, surface diff, gate record.
- Failure condition: partial commit, rerun side effect, reusable reassignment surface, or Spec/merge alone authorizes production.

### ACC-V5-035 — Frozen-plan binding negative matrix
- Contracts: `CTR-V5-035`.
- Method/environment: child/operator review using exact artifact bytes, byte-modified plan, mismatched digest, runtime-supplied OLD/NEW, and label/renamed-account fixtures.
- Expected: only the exact frozen artifact rows are selectable; every other input fails before writes.
- Required evidence: executed digest checks, source constants, and negative matrix transcript.
- Failure condition: any non-artifact identity or parameter reaches a write path.

### ACC-V5-036 — Excluded identity and canonical pair check
- Contracts: `CTR-V5-036`.
- Method/environment: operator fixtures for the excluded duplicate, canonical efficiency pair, Build in Public pair, and blog pair.
- Expected: excluded identity commits zero writes; efficiency transfers only via the canonical pair; blog and Build in Public pairs stay independent.
- Required evidence: executed matrix with pre/post row digests.
- Failure condition: the duplicate receives any write, or cross-pair confusion occurs.

### ACC-V5-037 — Projection creation and worklist terminal state
- Contracts: `CTR-V5-037`.
- Method/environment: disposable projection store with the 85 missing and 1 present identities.
- Expected: 85 projections created from exact artifact Principals; the present one exact-matched; `workflow_my_tasks` returns HTTP 200 with `items = []` (or real tasks) and no `principal_not_found`.
- Required evidence: created-row digests and executed worklist responses.
- Failure condition: a 87th/dynamic identity, display-name pairing, excluded-identity creation, or residual 404.

### ACC-V5-038 — Domain tuple exactness and conflict
- Contracts: `CTR-V5-038`.
- Method/environment: disposable Domain fixtures with exact, missing, extra, disabled, role-changed, and Principal-changed tuples.
- Expected: the exact 760 transfer atomically per pair; every drift yields zero writes with `CONFLICT`.
- Required evidence: pre/post tuples, transaction logs, and outcome records.
- Failure condition: dual Owner, long-lived dual member authority, Domain/Role change, or drift commits.

### ACC-V5-039 — Responsibility append-only and history immutability
- Contracts: `CTR-V5-039`.
- Method/environment: disposable current/terminal/cancelled/archived/state-version-mismatch fixtures.
- Expected: only the 80 re-validated exact tuples append successor Visit/Event/Receipt/Audit with CAS; all historical facts remain byte-identical.
- Required evidence: before/after history digests, new fact lineage, and CAS outcomes.
- Failure condition: ineligible reactivation, missing dedicated fact, wrong Instance/node, or any historical rewrite.

### ACC-V5-040 — Draft creator immutability
- Contracts: `CTR-V5-040`.
- Method/environment: all 99 draft tuples in a disposable store with a candidate successor operator run.
- Expected: zero `created_by_principal_id` changes and zero draft migrations.
- Required evidence: pre/post draft digests.
- Failure condition: any creator field rewrite or silent maintainer overwrite.

### ACC-V5-041 — Canary sequence and per-pair isolation
- Contracts: `CTR-V5-041`.
- Method/environment: full fleet rehearsal on a disposable store with injected per-pair failure and exact rerun.
- Expected: canary order holds; each pair commits independently SERIALIZABLE; one failure never fabricates another pair's success; exact rerun is a zero-write NOOP.
- Required evidence: ordered transcripts, per-pair transaction records, and rerun counts.
- Failure condition: pair writes merge, failure leaks into other pairs' outcomes, or rerun mutates.

### ACC-V5-042 — Fleet production gate sequence
- Contracts: `CTR-V5-042`.
- Method/environment: authority/implementation/plan/execution-record lifecycle review for the fleet apply.
- Expected: all ordered gates occur before any write; exact `PLAN_SHA256` is re-reviewed against the live recheck; apply lacks authority until the separate exact production authorization.
- Required evidence: exact commits, plan bytes/SHA, review receipt, execution authorization, and apply/verify/NOOP transcript.
- Failure condition: any earlier milestone implies apply, a write precedes the reviewed rechecked plan, or production apply is derived from acceptance alone.

### ACC-V5-043 — PR disposition and lifecycle invariance
- Contracts: `CTR-V5-043`.
- Method/environment: GitHub PR state plus V5 frontmatter and fresh audit record.
- Expected: PR #9 remains open/unmodified/unmerged with its disposition; V5 stays proposed with no implementation/production authority until Owner acceptance.
- Required evidence: PR snapshots, lifecycle diff, and audit record.
- Failure condition: PR #9 changes, V5 is treated as active, or independent audit is skipped.

### ACC-V5-044 — Read-time/no-state proof
- Contracts: `CTR-V5-044`.
- Method/environment: query snapshot concurrency fixtures and database write census.
- Expected: internally consistent fields/filter/cursor and zero new persistent dispatch state/writes.
- Required evidence: executed transaction trace and schema/write diff.
- Failure condition: mixed snapshot, state write, event, lease, reservation, or new dispatch column.

### ACC-V5-045 — Reason matrix and ops-lock negative
- Contracts: `CTR-V5-045`.
- Method/environment: independent/cumulative formal-fact fixtures plus metadata canaries.
- Expected: exact ordered closed reasons, true iff empty, no `OPS_LOCKED`, no metadata influence, no Assistance content.
- Required evidence: executed matrix and negative scanner.
- Failure condition: missing/extra/reordered/free-text reason or privacy leak.

### ACC-V5-046 — Wire opt-in compatibility
- Contracts: `CTR-V5-046`.
- Method/environment: golden responses for omitted/false/true and invalid values.
- Expected: exact legacy shape when omitted; paired fields for explicit values; invalid value returns stable 422 with no page.
- Required evidence: byte/key comparison and error response.
- Failure condition: legacy structural change, partial pair, coercion, 200, or partial response.

### ACC-V5-047 — Cursor walk/filter ordering
- Contracts: `CTR-V5-047`.
- Method/environment: interleaved blocked/eligible rows, `limit=2`, walk until `next_cursor=null`.
- Expected: no duplicates/misses, filled eligible pages when available, no count/offset fields.
- Required evidence: executed page sequence and query-plan/source proof.
- Failure condition: post-page filtering, discontinuity, or added pagination shape.

### ACC-V5-048 — Role/grant immutability
- Contracts: `CTR-V5-048`.
- Method/environment: pre/post role-binding, scope, credential, identity, and route authorization census.
- Expected: zero changes; Reader and legacy Coordinator behavior remain bounded to their pre-existing surfaces.
- Required evidence: exact before/after diff and denial matrix.
- Failure condition: any role/grant/scope/identity/allowlist change or new Coordinator grant.

### ACC-V5-049 — Advisory and caller-boundary matrix
- Contracts: `CTR-V5-049`.
- Method/environment: stale-after-read command test and source/capability scan for HR policy, Agent mapping, message/Session/wake behavior.
- Expected: later command revalidates and may reject; Workflow owns none of the caller policies.
- Required evidence: executed stale test and negative source/surface inventory.
- Failure condition: projection bypasses command gate or Workflow sends/selects/schedules/wakes.

### ACC-V5-050 — Child authority gate
- Contracts: `CTR-V5-050`.
- Method/environment: exact revision authority graph and implementation-base preflight.
- Expected: accepted V5 on main precedes revised accepted child; code begins only from that base; unrelated Slice-D debt remains explicit.
- Required evidence: merge commits, reviewed child head, acceptance record, and implementation base.
- Failure condition: current `af450aa` is accepted/implemented directly, chat fills authority, or two-field work claims full Slice-D completion.

```text
CONTRACT_COUNT = 50
CONTRACTS_WITH_ACCEPTANCE = 50
ACCEPTANCE_COUNT = 50
DANGLING_CONTRACT_REFERENCES = 0
UNCOVERED_CONTRACTS = 0
ACCEPTANCE_WITHOUT_FAILURE_CONDITION = 0
```

## 22. Alternatives and disposition

- Retain V2 Human root/two-person grant/Human OBO prerequisite: rejected for single-user Agent-first V1; future multi-Human governance remains possible through later authority.
- Reuse an existing business/canary Agent: forbidden by default because administrative authority needs a dedicated identity and credential lifecycle.
- Add `GLOBAL_WORKFLOW_COORDINATOR` composite authorization: rejected; it destroys independent permission reasoning and enables alias bypass.
- Treat Feishu owner identity as svc-workflow Human actor: rejected; transport provenance is not token identity or permission.
- Runtime grant API/Agent self-grant: rejected; designation is repository-owned and reviewed.
- Auto-expiring designation: not required; short token/rotation/revoke/disable controls plus whole-authority replacement are selected.
- Partial V2 amendment: rejected in V3; identity/governance meaning changed and V0 permits only whole-authority supersession.
- Directly amend accepted V3 for the fleet: rejected; successor-scope meaning changes, so V4 is a whole-authority successor.
- Generalize CTO/fleet into arbitrary Principal migration: rejected; each exception remains independently exact and bounded, and the fleet exception stays digest-frozen.
- Keep the single Build in Public pair as the entire exception: superseded; the frozen plan v2 artifact supersedes plan v1, and Draft PR #9 is dispositioned `SUPERSEDED_BY_FLEET_LOCAL_CHILD`.
- Accept runtime OLD/NEW arguments or dynamic roster expansion: rejected; a future operator binds only to exact artifact rows.
- Use PR #15 or mutable PR #2 as common prerequisites: rejected; both are independent from Agent-first V1.
- Directly amend accepted V4 for dispatchability: rejected; V4's complete field allowlist and explicit blocked/Assistance prohibition change, so V5 is whole-authority.
- Let HR infer Workflow blockers: rejected; it duplicates Workflow semantics and creates divergent authorities.
- Treat `metadata contains "ops-lock"` as a blocker: rejected; no authoritative primitive/writer/lifecycle exists.
- Add the two fields to every legacy response: rejected; it violates the current strict response-structure policy.
- Add page/total counts or post-filter pages: rejected; keyset `items + next_cursor` remains authoritative.

## 23. Migration, compatibility, containment, and rollback

V5 authoring is docs-only and mutates no runtime. It preserves V4's migration, trusted-fleet, containment, and rollback meaning. Existing `GLOBAL_WORKFLOW_COORDINATOR` bindings do not become Product Direction permission or gain new power; no role/grant migration occurs here.

Dispatchability requires no persisted state or data migration. Wire compatibility is explicit: requests omitting `dispatchableOnly` retain their current response structure; explicit `true|false` selects the new paired fields. A later implementation rollback removes the query parameter behavior and paired fields together without changing Workflow facts. Product Direction reversal requires a lawful whole successor; it is not inferred from code rollback.

Capability rollout remains Slice-scoped. Security containment may revoke/rotate Client, disable binding, disable Principal, and record durable audit under existing authority. V5 authorizes no production action, transition, canary, Grant, message, Session, wake, or Scheduler creation.

## 24. Open Questions and authoring readiness

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE (V5 lawfully proposes whole-authority resolution)
PARTIAL_SUPERSESSION = NONE
DUPLICATE_AUTHORITY_RISK = NONE
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
CHILD_IMPLEMENTATION_SPEC_REQUIRED = YES
AUTHORING_READY_FOR_REVIEW = YES
OPS_LOCK_AUTHORITY = GAP
OPS_LOCK_DISPOSITION = DEFERRED
WIRE_COMPATIBILITY_DECISION = EXPLICIT_QUERY_OPT_IN
CURRENT_GLOBAL_READ_GATE = GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR
ROLE_OR_GRANT_CHANGE_AUTHORIZED = NO

TRUSTED_AGENT_ROOT_REQUIRED = YES
ROOT_AUTHORITY_ID = SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1
ADMIN_AGENT_STRATEGY = NEW_DEDICATED_AGENT
HUMAN_PRINCIPAL_ADMINISTRATION_REQUIRED_FOR_V1 = NO
HUMAN_OBO_REQUIRED_FOR_V1 = NO
TWO_PERSON_APPROVAL_REQUIRED_FOR_V1 = NO
```

Exact dispatch blocker predicates and the final closed enum belong to the revised child Spec, within §8/§8A. They are not open Product Direction choices. The exact Admin Agent Principal UUID and Client ID remain owned by the later designation authority.

## 25. Lifecycle record

```text
ACCEPTANCE_STATUS = proposed
STATUS = proposed
AUTHORING_BASE = f0c74eefd63ca71a1fcb670ad31ac35f19f69539
CURRENT_PARENT = SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
CURRENT_PARENT_STATUS_ON_MAIN = accepted
CURRENT_PARENT_ACCEPT_COMMIT = 5cdd5eeb9895ce0bb4df1989f01806ca25b8ecff
DISPATCHABILITY_CHILD_CANDIDATE = af450aa39e446683b8ae2b2edf99c4febdcfb068
DISPATCHABILITY_CHILD_STATUS = proposed / blocked_on_parent
PLAN_SHA256_BOUND = 0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606
EXACT_FLEET_PAIR_COUNT = 86
WORKFLOW_PROJECTION_CREATE_COUNT = 85
DOMAIN_TRANSFER_COUNT = 760
ACTIVE_RESPONSIBILITY_TRANSFER_COUNT = 80
GENERAL_MIGRATION_CAPABILITY = NO
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
PRODUCTION_APPLY_AUTHORIZED = NO
INDEPENDENT_REVIEW_REQUIRED = YES
OWNER_ACCEPTANCE_REQUIRED = YES
FINAL_HEAD_RECHECK_REQUIRED = YES
MERGE_PERFORMED = NO
PRODUCT_CODE_CHANGE = NONE
PRODUCTION_CHANGE = NONE
```

This authoring round adds only the proposed V5 file. It does not alter V4, the local authority map, the child candidate, PR #9, code, tests, contracts, schemas, migrations, data, production, roles, Grants, or external repositories.

## 26. Acceptance Record (pending)

```text
ACCEPTANCE_STATUS = NOT_PERFORMED
REVIEW_BASE = f0c74eefd63ca71a1fcb670ad31ac35f19f69539
REVIEWED_HEAD = NONE
REVIEW_RESULT = PENDING_INDEPENDENT_边界_审计
READY_FOR_ACCEPTANCE_FINALIZE = NO
SEMANTIC_DELTA_AFTER_REVIEW = NOT_APPLICABLE
ACCEPTED_BY = NONE
ACCEPTED_AT = NONE
FINAL_HEAD_RECHECK = NOT_PERFORMED
ACTIVE_ON_MAIN = NO
CHILD_IMPLEMENTATION_SPEC_REQUIRED = YES
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
PRODUCTION_APPLY_AUTHORIZED = NO
```

An independent fixed-head review must first verify complete V4 restatement, the bounded §8/§8A delta, Product Direction/role/wire reconciliation, Contract-Acceptance coverage, and zero partial supersession. Only Owner `mayf3` or an explicitly authorized maintainer may later perform an atomic lifecycle transaction. This authoring act is not review or acceptance.
