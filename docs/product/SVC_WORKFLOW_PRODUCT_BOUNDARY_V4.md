---
authority_id: SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
status: proposed
authority_kind: product_direction
owning_repository: mayf3/svc-workflow
implementation_authority: none
production_apply_authority: none
supersedes:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V3
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_PRODUCT_BOUNDARY_V4

## 1. Goal and authority status

This document is the complete proposed Product Direction for `svc-workflow`. It is a whole-authority successor to accepted `SVC_WORKFLOW_PRODUCT_BOUNDARY_V3`, not an amendment and not a reader-side composition with V3.

```text
AUTHORITY_ID = SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
AUTHORITY_KIND = product_direction
STATUS = proposed
SUPERSEDES = SVC_WORKFLOW_PRODUCT_BOUNDARY_V3
PRODUCT_BOUNDARY_ACTION = SUPERSEDE
WHOLE_AUTHORITY_SUPERSESSION = YES
PARTIAL_SUPERSESSION = NONE
OWNER_USE_CASE = SINGLE_USER_TRUSTED_ADMIN_AGENT_PLUS_TWO_BOUNDED_SUCCESSOR_EXCEPTIONS
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PRODUCT_DIRECTION_AUTHORIZES_IMPLEMENTATION_DIRECTLY = NO
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
CHILD_IMPLEMENTATION_SPEC_REQUIRED = YES
```

V4 is proposed and inert. V3 remains the accepted active Product Direction on `main`. A later acceptance transition, if authorized after independent review, must atomically mark V4 accepted, mark V3 superseded with its `superseded_by` backlink, and update the repository authority map. This authoring PR performs none of those lifecycle changes and MUST NOT be merged as acceptance by implication.

The Goal is to preserve every V3 product boundary, Decision, Contract, exclusion, security invariant, capability Slice, retained trade-off, conformance-debt statement, and the original CTO bounded successor exception unchanged, while adding exactly one independent bounded Build in Public successor exception for the exact pair in §17A. No other V3 meaning changes.

## 2. Scope and non-goals

### 2.1 In scope

V4 governs:

- the serial workflow engine, its immutable facts, Domain isolation, worklists, Definition and Instance lifecycle, Transition boundary, idempotency, and failure behavior;
- the two independent global permissions `GLOBAL_SCHEDULER_READ` and `GLOBAL_DOMAIN_ADMIN`;
- one dedicated administrative Agent Principal as the daily runtime actor in a single-user deployment;
- a repository-owned docs-only designation root for the exact Agent Principal, Client, and granted split permissions;
- direct-token execution by that Agent Principal;
- one exact Feishu app/tenant/conversation/sender command ingress as provenance and an Agent-core gate, never as `svc-workflow` authority;
- durable audit using the actual authenticated Agent Principal;
- capability-scoped child-authority sequencing and current conformance debt;
- the original CTO bounded one-time Principal successor migration already added to V2;
- the additive exact-pair-only Build in Public bounded successor exception in §17A.

### 2.2 Explicit non-goals

This Product Direction does not create or select an Agent, Principal UUID, Client, credential, permission Grant, designation root instance, database row, migration, API, HTTP Contract, OpenAPI surface, SDK, test, deployment, production change, auth-service change, or dsh-agent-core change. It does not accept, merge, mark Ready, or activate any child authority or external PR.

It does not add parallel workflow nodes, dynamic forward branching, claim/pull assignment, ordinary reassignment, handoff, delegation, timers, external signals, automatic retry, SLA orchestration, arbitrary script guards, built-in LLM execution, cross-Domain shared templates, in-flight template replacement, in-flight Domain transfer, physical Instance deletion, unrestricted global workflow content access, or a runtime break-glass grant.

Long-term multi-Human governance is deferred, not forbidden. It may be introduced only by a lawful later Product Direction successor or an independent higher authority; no implementation or runtime configuration may silently reinterpret V4 to add it.

## 3. Authority and exact coordinates

```text
SVC_WORKFLOW_BASE_COMMIT = 327b74f138151a7f4d9d88e3881e54d203f1e8f6
AUTH_SERVICE_REFERENCE_COMMIT = 0855dc5161309196ef0cddbf9142e22726961956
DSH_AGENT_CORE_REFERENCE_COMMIT = 6ec83fa7ef0565959f26c7112de423bf5aa65680
CURRENT_PRODUCT_DIRECTION = SVC_WORKFLOW_PRODUCT_BOUNDARY_V3
SUCCESSOR_PRODUCT_DIRECTION = SVC_WORKFLOW_PRODUCT_BOUNDARY_V4
BLOCKED_CHILD_PR = https://github.com/mayf3/svc-workflow/pull/9
BLOCKED_CHILD_HEAD = 3056263c3fc964a2b225720dd2b859b47e296c2e
```

Authority precedence remains Product Direction, then accepted Architecture/long-lived invariant authority, then accepted governing child Specs, then descriptive code/tests/runtime/operations. External repositories are referenced at exact revisions and remain owned by those repositories.

The accepted one-time child authority `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` remains governed through the byte-for-meaning V3 restatement in §17 and is preserved unchanged. Open svc-workflow Draft PR #7, exact Head `a7f8d26b7a8f57da773bd7b05879ee485841fa58`, remains an independent proposed closure for successor-event replay and does not alter this Product Direction.

External Draft PRs are classified as follows:

```text
AUTH_SERVICE_PR_15_DISPOSITION = KEEP_DRAFT_DEFERRED
AUTH_SERVICE_PR_15_REQUIRED_FOR_AGENT_FIRST_V1 = NO
AUTH_SERVICE_PR_15_ACTIVE_ON_MAIN = NO
AUTH_SERVICE_PR_2_BLOCKS_AGENT_ADMIN_ROUTE = NO
AUTH_SERVICE_PR_2_RELATION = INDEPENDENT_LEGACY_SHUTDOWN_PROGRAM
```

PR #15 is a possible future input to multi-Human governance, but it is not an Agent-first V1 prerequisite. PR #2 is an independent legacy shutdown Program; the Agent-first route MUST NOT chase or pin its mutable Head as a common gate.

## 4. Current State, Observations, Claims, and Evidence

All State below is descriptive. Drift does not rewrite this Product Direction.

### 4.1 Observations

#### OBS-V4-001 — Active repository Product Direction and governance

- Subject: `mayf3/svc-workflow` source tree.
- Source revision: `327b74f138151a7f4d9d88e3881e54d203f1e8f6`.
- Method: inspect Product Direction frontmatter, `.agents/local/README.md`, governance lock, and governing Spec index; run `python3 .agents/tools/verify_governance.py --target . --require-accepted`.
- Result: V3 is accepted and active on `main`; V2 is superseded; governance adoption is accepted; the integrity verifier passes; V4 does not exist on the base.
- Provenance: repository paths named above and authoring PR record.

#### OBS-V4-002 — Existing global scheduler surface remains broader than V4

- Subject: svc-workflow current source/contract state at the base commit.
- Method: inspect the global/Coordinator query and current-state HTTP Contract and compare them with accepted V3 and the unchanged V4 restatement.
- Result: the existing global query still reads Context title, can expose terminal/archived inventory, lacks the complete disabled-Principal publication gate and durable protected-read audit, and retains legacy composite Coordinator semantics.
- Provenance: source tree, `contracts/workflow-http/v1/contract.md`, `ACTIVE_AGENT_SECRET_DUPLICATE_FORENSICS_V1_REPORT.md`, and V2 conformance-debt record.

#### OBS-V4-003 — Existing global Domain administration is not V4-complete

- Subject: svc-workflow provisioning surface at the base commit.
- Method: inspect `docs/contracts/IDENTITY_PROVISIONING_API_V0.md` and current source.
- Result: the surface is allowlist/scope based and broader than the V4-preserved minimum; separated permission lifecycle, comprehensive no-self-grant/no-self-owner enforcement, minimum Human/Agent selection directory, and narrowed existing Domain-admin behavior remain unestablished.
- Provenance: `docs/contracts/IDENTITY_PROVISIONING_API_V0.md` and source tree.

#### OBS-V4-004 — auth-service reusable identity primitives and missing child permission supply

- Subject: `mayf3/auth-service` at `0855dc5161309196ef0cddbf9142e22726961956`.
- Method: inspect accepted/frozen identity, Machine Principal, Client, direct-token, resolution, and revoke contracts and compare them with the V4-preserved designated-permission requirement.
- Result: Agent Principal, Client, direct token, identity resolution, rotation, and revocation primitives are reusable; an accepted capability-scoped child authority supplying the designated Agent's required `svc-workflow` audience/scope/grant is still required. Human Principal administration is not a prerequisite.
- Provenance: exact external revision and the later auth-service child-authority review record.

#### OBS-V4-005 — dsh-agent-core command-route gaps

- Subject: `mayf3/dsh-agent-core` at `6ec83fa7ef0565959f26c7112de423bf5aa65680`.
- Method: inspect Feishu connector, Router/binding, Broker capability manifests, Scheduler, Notification Ingress, and their authority records.
- Result: no implemented fleet-level svc-workflow scheduler query and no workflow Domain-admin capability manifest exist. The shipped workflow manifest has four caller-scoped `workflow.read` capabilities only. Feishu admission has runtime app credentials and a dynamic prebound-conversation gate, but no committed exact tenant or sender-identity allowlist; replay handling is SDK-owned and message-ID based. Notification Ingress V0 is an implemented loopback thin delivery adapter without auth/durable idempotency/workflow authority; its V1 auth/idempotency document is accepted design authority but explicitly does not authorize implementation. Router/Broker identity discipline ignores self-reported fields, selects credentials by trusted process `agentId`, and relies on auth-service to resolve the actual Agent Principal; the V4-preserved exact administrative route still requires its own authority and conformance evidence.
- Provenance: `packages/scheduler/src/`, `packages/broker/src/capabilities/workflow.js`, `packages/feishu-connector/src/`, `packages/production-runtime/src/v2-ingress-gate.js`, `packages/notification-ingress/src/index.js`, `docs/specs/NOTIFICATION_INGRESS_SERVICE_AUTH_AND_IDEMPOTENCY_V1.md`, and Router/Broker credential-path sources at the exact external revision.

#### OBS-V4-006 — Open and historical authority inventory

- Subject: svc-workflow PRs, branches, worktrees, and uncommitted drafts at authoring start.
- Method: fresh-fetch `github/main` and exact PR #9 Head; inspect Product Direction, Specs, Architecture, contracts, and reports for successor and trusted-Agent authority.
- Result: PR #7 remains an independent proposed replay closure; PR #9 is the exact blocked BIP Child described in `OBS-V4-007`; unrelated uncommitted implementation work remains outside this Product Direction. No competing V4 authority was found.
- Provenance: authoring PR gate record and exact §3 coordinates.

#### OBS-V4-007 — Build in Public child is blocked by V3 exact-pair authority

- Subject: `mayf3/svc-workflow` Draft PR #9.
- Source revision: `3056263c3fc964a2b225720dd2b859b47e296c2e` against main `327b74f138151a7f4d9d88e3881e54d203f1e8f6`.
- Method: inspect the blocked child's exact pair, authority analysis, and proposed Contracts; compare with accepted V3 §17 and `CTR-V4-032` through `CTR-V4-034`.
- Result: PR #9 is fixed to Build in Public OLD `bb9d8f48-7962-4321-8fb1-554bb428c159` and NEW `d5b3aeb2-e754-49a9-9914-b963521c0985`, while V3 authorizes only the distinct CTO pair. The child correctly remains non-implementing and reports a lawful-parent blocker.
- Provenance: exact GitHub PR and commit coordinates above; no production database was queried.

### 4.2 Claims

#### CLM-V4-001 — V4 requires whole-authority supersession

- Support state: SUPPORTED.
- Supported by: `EVD-V4-001`.
- Claim: adding a second exact bounded successor pair changes V3's accepted exact-pair meaning and therefore cannot be a silent edit or partial amendment to accepted V3.

#### CLM-V4-002 — Existing implementation is conformance debt, not authority

- Support state: SUPPORTED.
- Supported by: `EVD-V4-002`.
- Claim: current global read, Domain administration, Agent-core ingress, and external permission supply do not already conform merely because partial mechanisms exist.

#### CLM-V4-003 — Capability-scoped sequencing avoids a common global gate

- Support state: SUPPORTED.
- Supported by: `EVD-V4-003`.
- Claim: separate child authorities can close identity, designation, permission supply, scheduler read, Domain admin, and Feishu routing without allowing one Slice to authorize another or making Human governance a common prerequisite.

#### CLM-V4-004 — Complete restatement can preserve V3 while adding only BIP

- Support state: SUPPORTED.
- Supported by: `EVD-V4-004`.
- Claim: a complete V4 restatement can preserve every V3 boundary and the original CTO exception while adding one separately exact, plan-first BIP exception without creating general migration authority.

### 4.3 Evidence relations

#### EVD-V4-001 — Exact-pair blocker and governance protocol support supersession

- Source observations: `OBS-V4-001`, `OBS-V4-006`, `OBS-V4-007`.
- Target: `CLM-V4-001`.
- Relation: SUPPORTS.
- Bound coordinates: svc-workflow base, blocked Child, and governance snapshot in §3.
- Strength/sufficiency: sufficient for whole-authority classification.
- Limitations: does not itself accept or activate V4.

#### EVD-V4-002 — Current-source observations support drift classification

- Source observations: `OBS-V4-002`, `OBS-V4-003`, `OBS-V4-004`, `OBS-V4-005`.
- Target: `CLM-V4-002`.
- Relation: SUPPORTS.
- Bound coordinates: exact repository revisions in §3.
- Strength/sufficiency: sufficient to reject claims of present conformance.
- Limitations: child implementation review must refresh source/runtime observations on its own base.

#### EVD-V4-003 — Split ownership supports capability-scoped children

- Source observations: `OBS-V4-003`, `OBS-V4-004`, `OBS-V4-005`.
- Target: `CLM-V4-003`.
- Relation: SUPPORTS.
- Bound coordinates: exact repository revisions in §3.
- Strength/sufficiency: sufficient for Product Direction decomposition.
- Limitations: each external repository retains acceptance authority over its own child.

#### EVD-V4-004 — V3 restatement and exact BIP delta support bounded preservation

- Source observations: `OBS-V4-001`, `OBS-V4-007`, accepted V3 at main `327b74f138151a7f4d9d88e3881e54d203f1e8f6`, and this file's explicit §17/§17A separation.
- Target: `CLM-V4-004`.
- Relation: SUPPORTS.
- Bound coordinates: §3 blocked-child and current-parent commits.
- Strength/sufficiency: sufficient for bounded proposal authoring and independent semantic review.
- Limitations: does not accept V4, align PR #9, implement an operator, establish live scope, or authorize production apply.

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
GLOBAL_WORKFLOW_COORDINATOR = UI_LABEL_ONLY
```

The dedicated Agent is an actual auth-service Agent Principal and daily runtime actor. V4 selects no UUID or Client ID. Existing business/canary Agents are not reused by default; a later designation authority must identify a newly dedicated Agent and exact Client.

There are exactly two independent global permissions. The same designated Agent may hold either or both, but one never implies the other and both do not form a third authorization capability. No composite runtime role is permitted. `GLOBAL_WORKFLOW_COORDINATOR` may be presentation text only; it is not a permission, role, migration target, authorization alias, or compatibility bypass.

Neither permission grants workflow content, Transition, reassignment, cancel/archive, Definition management, membership management, Assistance body, credentials, or audit-content access.

## 8. `GLOBAL_SCHEDULER_READ`

This permission supports deployment-wide scheduling with metadata for active current tasks only. It is not timer, dispatch, retry, SLA, signal, Transition, or orchestration authority.

```text
FULL_CONTENT_ACCESS_REQUIRED = NO
SCHEDULING_VIEW_SCOPE = ACTIVE_CURRENT_TASK_METADATA_ONLY
TASK_LABEL = NOT_INCLUDED
CONTEXT_TITLE_AS_METADATA = FORBIDDEN
```

Each logical task record has `principalId == currentAssigneePrincipalId`; `activeTaskCount` is that Principal's count in the same projection snapshot. A child wire contract may group or repeat these values but cannot change authorized data. All fields below are required. Node type/lifecycle/status use closed non-sensitive code sets. `updatedAt` is the latest committed Workflow Instance state-change time represented by the current authoritative projection—not read, cache, scheduler, audit, Assistance, or external-system time.

Complete field allowlist:

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

Only active current-task records are eligible. Archived, cancelled, terminal-without-current-task, historical Visit, and non-current records are excluded under every filter/group/page mode. Child Specs may define stable filters/pagination only within this population.

Forbidden content includes Context and Context title; task label; Submission/history; timeline `EventData`; Assistance request/escalation/resolution/body/supporting payload or derived blocking status; credential/token; Receipt/command attempt/SecurityAudit/audit payload; historical/terminal/archived inventory; Transition options/write capability; and any derived field that reconstructs or summarizes forbidden content. A Context title cannot be renamed metadata. `blockedFlag`, `blockedReasonCode`, `waitingAssistance`, and `assistanceStatus` remain unauthorized without a later whole Product Direction.

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

V4 preserves V3's record of, and does not excuse, these gaps:

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

V4 preserves and fully restates V3’s already-accepted CTO bounded exception without changing its pair, scope, counts, child authority, or production gate and without creating ordinary reassignment:

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

## 17A. Additive Build in Public exact-pair bounded successor exception

### 17A.1 Exact identity and exclusion boundary

```text
BIP_EXCEPTION = ADDITIVE_EXACT_PAIR_ONLY
BIP_OLD_AGENT_ID = build-in-public-agent
BIP_OLD_PRINCIPAL = bb9d8f48-7962-4321-8fb1-554bb428c159
BIP_NEW_AGENT_ID = agt_build-in-public-agent
BIP_NEW_PRINCIPAL = d5b3aeb2-e754-49a9-9914-b963521c0985

EXCLUDED_AGENT_ID = blog-agent
EXCLUDED_PRINCIPAL = 81c7fc7e-c696-4b47-bfd6-f12a9ecb68a6
OTHER_PRINCIPAL_PAIRS = FORBIDDEN
MIGRATION_KIND = ONE_TIME_SUCCESSOR
```

This exception authorizes a later accepted bounded Child Spec to govern only the exact `BIP_OLD_PRINCIPAL -> BIP_NEW_PRINCIPAL` pair. Agent labels are explanatory provenance and MUST NOT be used as runtime identity selectors. `blog-agent`, its exact Principal, and every other Principal pair are outside authority. The exception creates no general Principal migration capability.

### 17A.2 Plan-selected live scope only

Only two object categories may be selected, and only through an independently reviewed canonical production read-only plan:

A. Domain authority bindings whose live tuple still identifies BIP OLD and remains enabled in the reviewed snapshot;

B. workflow responsibility whose live current Visit is assigned to BIP OLD, is current, active, non-terminal, not cancelled, and remains exactly present in the reviewed snapshot at apply time.

The Owner expects one `DOMAIN_OWNER` and eight `DOMAIN_MEMBER` bindings, but `1 + 8` is expected authoring scope, not live database truth and not permission to force counts. The future plan MUST re-read exact tuples, including Domain, Role, binding identity, enabled state, Principal, and review-relevant timestamps. A missing, additional, disabled, role-changed, Principal-changed, state-version-changed, or otherwise mismatched tuple is conflict with zero committed writes. A reviewed plan may lawfully contain zero eligible current responsibilities:

```text
ACTIVE_RESPONSIBILITY_TRANSFER_COUNT = 0
RESULT = VALID_PLAN
HISTORICAL_REACTIVATION = FORBIDDEN
```

### 17A.3 Domain successor semantics

For each exact reviewed enabled Domain tuple:

- `DOMAIN_OWNER` uses atomic formal Owner replacement from BIP OLD to BIP NEW and never commits dual Owner authority;
- `DOMAIN_MEMBER` preserves Domain and Role, enables/re-enables BIP NEW, disables BIP OLD, and never commits long-lived dual authority;
- every tuple participates in one atomic migration outcome together with every selected responsibility successor Visit/Event/Receipt/Audit and projection CAS;
- exact successful rerun is `NOOP` with zero writes and zero new audit;
- any live-plan drift or post-state mismatch fails closed with zero committed writes.

No number in this Product Direction substitutes for live tuple review.

### 17A.4 Current responsibility and immutable history

For every exact reviewed eligible responsibility, future apply MUST preserve the same Workflow Instance and node, append one successor Visit assigned to BIP NEW, append a dedicated successor Event, complete a dedicated Receipt, append durable Audit, and CAS the reviewed current Visit and expected workflow state version. It MUST update or delete zero historical Visits, assignments, Events, Submissions, Context revisions, Receipts, or Audits. Completed, terminal, cancelled, archived, non-current, and unplanned tasks remain untouched. The legacy 5,583 archive is excluded and MUST NOT be queried or migrated by this operator.

### 17A.5 Plan-first and separate production authorization

```text
SPEC_ACCEPTED != PRODUCTION_APPLY_AUTHORIZED
```

The complete mandatory sequence is:

1. accepted bounded Child Spec;
2. independently reviewed exact operator implementation;
3. production read-only `--plan` with zero writes;
4. canonical plan bytes and separately computed `PLAN_SHA256`;
5. independent review of the exact live plan and digest;
6. separate explicit production apply authorization over exact implementation/plan/database/operator coordinates;
7. apply;
8. verify;
9. exact rerun returning `NOOP` with zero writes and zero new audits.

No write is authorized without a reviewed live plan. Product Direction acceptance, Child acceptance, implementation review, or plan generation alone does not authorize production apply. Apply must be atomic, fail closed on drift, and use no OLD credential impersonation, manual SQL mutation, online management API, or database-switch/fallback path.

### 17A.6 No durable general capability

```text
CTO_BOUNDED_SUCCESSOR_AUTHORITY = PRESERVED_UNCHANGED
BIP_BOUNDED_SUCCESSOR_AUTHORITY = ADDITIVE_EXACT_PAIR_ONLY
ORDINARY_REASSIGNMENT = STILL_FORBIDDEN
HANDOFF = STILL_FORBIDDEN
DELEGATION = STILL_FORBIDDEN
GENERAL_SUCCESSOR_API = FORBIDDEN
OTHER_PRINCIPAL_PAIRS = FORBIDDEN
ONLINE_MANAGEMENT_API = FORBIDDEN
HISTORICAL_REWRITE = FORBIDDEN
LEGACY_5583_ARCHIVE_MIGRATION = FORBIDDEN
PRODUCTION_APPLY_AUTHORIZED = NO
```

### 17A.7 Blocked Child relationship

Draft PR #9 at exact Head `3056263c3fc964a2b225720dd2b859b47e296c2e` remains blocked and inert while V4 is proposed/unmerged. After a separately reviewed Owner-accepted V4 is merged, PR #9 may receive only an authority-alignment amendment that updates `governed_by`/parent and base/revision coordinates, refreshes authority observations, and closes the lawful-parent blocker. That amendment MUST NOT change the child's exact BIP pair, `blog-agent` exclusion, plan-first model, Domain successor model, active-current-responsibility model, historical immutability, or separate production apply authorization. This V4 PR MUST NOT modify PR #9.

## 18. Capability-scoped child authorities and ordering

No common all-Slices global gate is created. Each child authorizes only its own capability.

### Slice A — Dedicated Admin Agent identity

Create/select a new dedicated Agent; establish exact Agent Principal UUID and exact Client; verify enabled status, credential ownership, rotation, and revoke. This Product Direction performs none of these actions.

### Slice B — Trusted Agent designation root

Independently review, Owner-accept, and merge `SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1` with exact Agent/Client and split permissions.

### Slice C — auth-service permission supply

An independent accepted auth-service child authority supplies only the designated Agent's required audience/scope/grant. It grants no Human or other Agent, is versioned/auditable/idempotent/revocable, and does not put business role authority in self-reported JWT claims.

### Slice D — svc-workflow global scheduler read

An implementation-authorizing child Spec freezes the exact field allowlist, excludes terminal/history, fails disabled Principal closed, commits durable read audit before publication, evaluates the separated permission, and removes legacy composite role authorization/aliasing.

### Slice E — svc-workflow Domain admin

An implementation-authorizing child Spec freezes Domain create, initial Owner, Owner replacement, no-self-grant, no-self-owner, atomic audit, minimum directory, and reconciliation of conflicting `IDENTITY_PROVISIONING_API_V0` semantics.

### Slice F — dsh-agent-core / Feishu command route

An independently owned authority freezes exact Feishu ingress gates, fleet scheduler capability, Domain-admin capability, Agent credential broker, and request/receipt correlation. It uses neither Human OBO nor body actor.

Slices may have dependency edges necessary for their own execution, but no Slice silently activates another. Assistance and Admin Recovery remain independent unless a child actually changes their data/semantics. Current HTTP/OpenAPI/SDK surfaces change only with their own accepted implementation authority.

## 19. Decisions

### DEC-V4-001 — Single-user dedicated Agent operation

- Decision owner: `mayf3`.
- Decision: daily global administration uses one new dedicated Agent Principal with direct token; Human runtime Principal/OBO/administration and two-person approval are not V1 prerequisites.
- Rejected: reuse an ordinary business/canary Agent; retain Human-root/two-approver V1 prerequisite.
- Owner input remaining: none.

### DEC-V4-002 — Repository designation replaces runtime grant governance

- Decision owner: `mayf3`.
- Decision: exact Agent/Client/split permissions are activated only by merged docs-only `SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1`; replacement uses whole-authority successor, with emergency disablement only for containment.
- Rejected: runtime self-grant, runtime grant API requirement, auto-expiring designation, break-glass grant.
- Owner input remaining: none.

### DEC-V4-003 — Split global permissions remain independent

- Decision owner: `mayf3`.
- Decision: preserve exactly the two split permissions and V2's bounded scheduler/Domain-admin semantics; the same Agent may hold both without union or third role.
- Rejected: composite Coordinator runtime role or alias.
- Owner input remaining: none.

### DEC-V4-004 — Feishu is gated provenance, never authority

- Decision owner: `mayf3`.
- Decision: exact single-user Feishu ingress gates command admission in Agent core; svc-workflow actor remains the designated Agent Principal.
- Rejected: Feishu sender as Human Principal or body actor; Human OBO prerequisite.
- Owner input remaining: none.

### DEC-V4-005 — Preserve V2 workflow and one-time successor boundaries

- Decision owner: `mayf3`.
- Decision: retain all V2 workflow, Domain, scheduler, Domain-admin, audit/idempotency, ownership, technology, trade-off, and exact one-time successor limits unless explicitly replaced by DEC-V4-001/002/004.
- Rejected: partial supersession or reader-side composition with V2.
- Owner input remaining: none.

### DEC-V4-006 — Add only the exact BIP bounded exception

- Decision owner: `mayf3`.
- Decision: preserve the CTO exception unchanged and add only §17A's exact Build in Public pair with reviewed-live-plan selection, append-only successor facts, atomic apply, fail-closed drift, exact NOOP, and separate production authorization.
- Rejected: modifying the CTO pair, arbitrary Principal migration, ordinary reassignment/handoff/delegation, online management API, historical rewrite/reactivation, count-forcing, and legacy archive migration.
- Owner input remaining: none.

## 20. Normative Contracts

### CTR-V4-001 — Whole-authority lifecycle
V4 MUST remain non-active while proposed/unmerged, MUST replace all V3 meaning only through an atomic accepted transition with V3 backlink/authority-map updates, and MUST NOT authorize implementation or production apply directly.

### CTR-V4-002 — Serial workflow product shape
svc-workflow MUST preserve §5's single-current-node/current-assignee deterministic serial workflow shape and MUST NOT add the excluded orchestration capabilities without later authority.

### CTR-V4-003 — Definition lifecycle and immutability
Definition ownership, lifecycle, publication immutability, Domain locality, and non-destructive archive MUST satisfy §5.1.

### CTR-V4-004 — Instance and immutable history
Instance ownership, non-physical deletion, immutable Context/Visit/Submission/Event facts, projection semantics, and one-version/one-Event atomic state command MUST satisfy §§5.2-5.4.

### CTR-V4-005 — Transition actor and atomicity
Only the authorized current assignee MAY perform normal Transition; global permission/designation MUST NOT imply it, and all resulting facts/outcome/audit MUST commit atomically.

### CTR-V4-006 — Strict normal Domain isolation
Ordinary Agent/member/Owner access, lookup/list/count/cursor/denial/serialization MUST preserve §6 isolation; only enumerated global permissions MAY cross Domains.

### CTR-V4-007 — Domain-local views and administration
Ordinary worklists, history, Owner views, membership, Definition governance, and Visit snapshots MUST remain bounded as in §6 and MUST NOT inherit global authority.

### CTR-V4-008 — Exactly two split global permissions
Authorization MUST evaluate `GLOBAL_SCHEDULER_READ` and `GLOBAL_DOMAIN_ADMIN` independently; the same Agent MAY hold both, but neither implies the other and no composite/alias role MAY authorize either.

### CTR-V4-009 — Scheduler complete field allowlist
Every global scheduler record MUST use only and all fields in §8's complete allowlist with the stated identity/count/code-set/`updatedAt` semantics.

### CTR-V4-010 — Scheduler active-current population only
The scheduler MUST return active current-task records only and MUST exclude historical, non-current, cancelled, archived, and terminal-without-current-task records under every mode.

### CTR-V4-011 — Scheduler sensitive-content exclusion
The scheduler MUST NOT expose or derive Context/title, task label, Submission, EventData, Assistance/supporting payload/status, credentials/tokens/audit payload, Transition options, or any sensitive derived field.

### CTR-V4-012 — Domain-admin allowed surface
`GLOBAL_DOMAIN_ADMIN` MUST authorize only idempotent Domain create, atomic initial Owner/disabled fallback, atomic Owner replacement, and §9's minimum selection directory.

### CTR-V4-013 — Domain-admin excluded surface
The permission MUST NOT grant workflow content/write/Transition/cancel/archive/Definition/membership/audit-content/other Domain writes or infer actor from body/Feishu/display/service/scope/allowlist.

### CTR-V4-014 — No self-grant and no self-Owner
The Agent MUST NOT grant itself permission or set its own canonical Principal as Domain Owner, directly or through aliases/chains/retries/migrations; distinct UUIDs remain distinct absent exact accepted linkage authority.

### CTR-V4-015 — Dedicated actual Agent actor
The runtime actor MUST be the exact designated Admin Agent Principal in verified direct-token `sub`; ordinary Agents, Humans, Clients/intermediaries, Feishu senders, bodies, display names, claims, and self-report MUST NOT substitute.

### CTR-V4-016 — Server-side independent authorization and fail closed
Each protected request MUST evaluate active designation and server-side split binding; disabled/revoked Principal or Client, invalid/expired token, missing permission, inactive designation/binding, or unavailable authorization MUST fail closed.

### CTR-V4-017 — Trusted designation root contents and activation
`SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1` MUST contain every field and only closed permissions in §11 and MUST activate only after independent review, Owner acceptance, and merge to main; runtime inputs cannot create it.

### CTR-V4-018 — Designation rotation and emergency lifecycle
Designation has no required expiry; replacement/revocation MUST follow whole-authority successor or emergency disable containment, retain short token/rotatable secret/revocable Client/disableable Principal and binding controls, and provide no break-glass grant.

### CTR-V4-019 — Compromise and replacement order
Credential compromise and Agent replacement MUST follow §11's exact containment and successor order; no replacement Agent MAY receive authority before its merged root successor.

### CTR-V4-020 — Exact Feishu ingress gate
Agent core MUST admit administrative commands only for the exact app, tenant, prebound conversation, allowed sender, verified event/message ID, timestamp/nonce, and replay checks in §12.

### CTR-V4-021 — Feishu provenance is not authorization
Feishu/message facts MUST remain provenance only; svc-workflow MUST authorize only actual Agent Principal/server binding and MUST NOT treat sender ID as actor or Human permission.

### CTR-V4-022 — End-to-end correlation
Durable records MUST correlate the Agent Principal and every Feishu/request/receipt identifier enumerated in §12 without storing sensitive bodies or confusing provenance with authorization.

### CTR-V4-023 — Durable audit coverage
Successful and authenticated-denied global reads, Domain create/Owner replace, designation/grant/revoke/disable actions MUST produce durable non-sensitive audit identifying the actual authenticated Agent Principal.

### CTR-V4-024 — Audit-before-read and atomic-write failure policy
Protected read audit MUST be durable before data publication; protected writes and audit MUST be atomic; audit/authorization failure MUST release/commit nothing.

### CTR-V4-025 — Revocation/disablement publication barrier
Revocation or disablement before publication/commit MUST prevent older in-flight reads from releasing data and writes from committing; the old Agent MUST cease operating.

### CTR-V4-026 — Idempotency and outcome reconciliation
Same-key/same-request MUST replay; same-key/different-request MUST conflict; `outcome_unknown` MUST reconcile only by exact same-key/same-request retry, never a blind new key.

### CTR-V4-027 — Retention and sensitive audit exclusion
Required audit MUST be retained exactly 365 days and MUST exclude §13 sensitive content; no product audit-read API or external export is authorized.

### CTR-V4-028 — External ownership and direct-token supply
Auth-service/Agent-core/upper-layer/UI ownership MUST remain as §14 states. Agent-first permission supply requires independent child authority and MUST NOT depend on Human PR #15 or legacy PR #2.

### CTR-V4-029 — Layer and storage trust boundary
The technology/layer/storage boundary in §14 MUST be preserved; global security MUST NOT rely on UI filtering, post-read redaction, adapter bypass, or shared-database access.

### CTR-V4-030 — Conformance debt remains unimplemented
No debt listed in §16 MAY be represented as compliant or implementation-authorized until exact accepted child Contracts and Contract-by-Contract evidence establish it.

### CTR-V4-031 — Capability-scoped child authority
Each Slice in §18 MUST have its own accepted authority before implementation and MUST NOT activate, broaden, or waive another Slice; no Human-governance common gate exists.

### CTR-V4-032 — Exact one-time successor scope
The retained migration MUST be offline and fixed to §17's exact pair, nine reviewed enabled Domain authorities, and exact live eligible current responsibility; drift MUST commit zero writes.

### CTR-V4-033 — Successor historical immutability and append-only transfer
The migration MUST rewrite zero historical assignments/Visits, preserve the known 58/111 exclusions, and represent successor responsibility only through new Visit/Event/Receipt/Audit facts.

### CTR-V4-034 — Successor atomic NOOP and no durable product surface
The retained migration MUST commit atomically, exact-rerun NOOP with zero writes/audits, fail closed on mismatched metadata/post-state, create no general API/capability, and retain separate implementation/production gates.

### CTR-V4-035 — BIP exact pair and explicit exclusions
The additive exception MUST be fixed only to §17A's exact BIP OLD/NEW UUIDs, MUST exclude `blog-agent`/`81c7fc7e-c696-4b47-bfd6-f12a9ecb68a6` and every other pair, and MUST NOT use labels, credentials, request fields, or dynamic input to widen identity.

### CTR-V4-036 — BIP reviewed live-plan scope
Only exact enabled OLD Domain tuples and exact current active non-terminal non-cancelled OLD-assigned responsibilities present in the independently reviewed live plan MAY be selected; expected `1 + 8` Domain scope is not live truth, zero active responsibility is valid, and any drift MUST commit zero writes.

### CTR-V4-037 — BIP Domain successor atomicity
BIP Owner replacement and Member enable-NEW/disable-OLD MUST preserve exact Domain/Role and commit atomically with every selected responsibility successor fact/projection CAS, without persistent dual authority; conflict MUST fail closed and exact rerun MUST return NOOP with zero writes/audits.

### CTR-V4-038 — BIP append-only responsibility and history
Each reviewed eligible responsibility MUST preserve Instance/node, append successor Visit/dedicated Event/Receipt/Audit, CAS exact Visit/state version, and rewrite/reactivate zero historical, terminal, completed, cancelled, archived, or legacy-archive facts.

### CTR-V4-039 — BIP plan-first production gate
The complete §17A.5 sequence MUST be enforced; canonical `PLAN_SHA256` and independent live-plan review are mandatory, and no Product Direction/Child/implementation/plan milestone authorizes production apply without the separate exact execution authorization.

### CTR-V4-040 — BIP no general product capability
The BIP exception MUST create no general migration, reassignment, handoff, delegation, online management, arbitrary-pair, manual-SQL, database-switch, historical-rewrite, or legacy-archive capability and MUST leave every V3 prohibition unchanged.

### CTR-V4-041 — Blocked Child alignment only
After V4 acceptance and merge, PR #9 MAY change only authority-alignment coordinates/observations and blocker state; its exact pair, exclusion, plan-first, Domain, active-responsibility, history, and separate apply-authorization product semantics MUST remain unchanged.

## 21. Acceptance

Every item requires executed evidence at the implementation/authority revision named by its child; a test definition or prose assertion alone is not evidence.

### ACC-V4-001 — Lifecycle and supersession check
- Contracts: `CTR-V4-001`.
- Method/environment: repository frontmatter/backlink/map review on proposed and any later acceptance commits.
- Expected: proposed V4 is inactive; any later acceptance changes V3 backlink/map atomically while implementation and production apply authority remain none.
- Required evidence: exact Git diffs, reviewed commits, Owner receipt, final-head recheck, and main merge coordinate.
- Failure condition: proposed V4 is called active/accepted, V3 is partially superseded, or implementation/production apply is authorized.

### ACC-V4-002 — Serial-shape negative matrix
- Contracts: `CTR-V4-002`.
- Method/environment: child-spec contract and executable capability matrix.
- Expected: only §5 serial shape is available.
- Required evidence: exact endpoint/domain tests and implementation mapping.
- Failure condition: parallel/dynamic/claim/timer/retry/SLA/script/LLM/general reassignment capability becomes available.

### ACC-V4-003 — Definition lifecycle matrix
- Contracts: `CTR-V4-003`.
- Method/environment: integration tests over all Definition states and cross-Domain attempts.
- Expected: §5.1 lifecycle/immutability/Domain ownership holds.
- Required evidence: executed state matrix and storage diff.
- Failure condition: published bytes mutate, invalid state creates Instances, archive rewrites facts, or Definition is shared cross-Domain.

### ACC-V4-004 — Instance/history integrity
- Contracts: `CTR-V4-004`.
- Method/environment: transactional and history-replay integration tests.
- Expected: no physical delete; immutable facts and projection agree; one version/Event per success.
- Required evidence: executed queries and commit coordinates.
- Failure condition: fact rewrite/delete, partial commit, duplicate/missing Event, or projection/history divergence.

### ACC-V4-005 — Transition authority matrix
- Contracts: `CTR-V4-005`.
- Method/environment: test current assignee, Owner, globally authorized Agent, and unrelated Principal.
- Expected: only current assignee succeeds and transaction is atomic.
- Required evidence: executed auth matrix and database audit.
- Failure condition: Admin Agent/global permission transitions solely by that status or any partial fact commits.

### ACC-V4-006 — Cross-Domain noninterference
- Contracts: `CTR-V4-006`.
- Method/environment: ordinary Agent/Owner lookup/list/count/cursor/error/serialization matrix across two Domains.
- Expected: no cross-Domain fact or existence leak.
- Required evidence: executed responses and query/audit traces.
- Failure condition: an ordinary Agent or role combination obtains global access or any observable cross-Domain leak.

### ACC-V4-007 — Domain-local view boundary
- Contracts: `CTR-V4-007`.
- Method/environment: worklist/history/Owner/admin test matrix including Owner replacement.
- Expected: views remain participation/Domain scoped and old Visit snapshots remain unchanged.
- Required evidence: executed responses and immutable rows.
- Failure condition: old Owner retains access, another Domain appears, or Owner change rewrites Visit.

### ACC-V4-008 — Split-permission anti-alias matrix
- Contracts: `CTR-V4-008`.
- Method/environment: test each permission alone, both, neither, and legacy `GLOBAL_WORKFLOW_COORDINATOR` only.
- Expected: each operation requires its exact permission; both add no third power.
- Required evidence: executed allow/deny matrix and server-side binding rows.
- Failure condition: one permission implies the other or legacy/composite Coordinator bypasses split checks.

### ACC-V4-009 — Scheduler schema allowlist
- Contracts: `CTR-V4-009`.
- Method/environment: schema/property tests plus live projection comparison.
- Expected: exactly all allowed fields with correct identity/count/time semantics.
- Required evidence: executed payloads and projection-source query.
- Failure condition: allowed field missing, extra field present, assignee mismatch, count mismatch, or `updatedAt` uses another clock.

### ACC-V4-010 — Scheduler population filter
- Contracts: `CTR-V4-010`.
- Method/environment: active/current, historical, terminal, cancelled, archived fixtures under every filter/page/group mode.
- Expected: only active current tasks appear.
- Required evidence: executed result sets and source fixture IDs.
- Failure condition: archived/terminal/history/non-current inventory is returned.

### ACC-V4-011 — Scheduler sensitive-field rejection
- Contracts: `CTR-V4-011`.
- Method/environment: seeded unique markers in every forbidden source and derived-field scan.
- Expected: no marker or forbidden key/derivation is returned.
- Required evidence: executed response corpus and field/marker scanner.
- Failure condition: Context title, Context, task label, Submission, EventData, Assistance/supporting payload, token/credential/audit content, or derived sensitive data appears.

### ACC-V4-012 — Domain-admin allowed operation matrix
- Contracts: `CTR-V4-012`.
- Method/environment: create with Owner, disabled fallback, replace Owner, and directory integration tests.
- Expected: only enumerated operations/fields work; Owner invariant is atomic.
- Required evidence: executed responses, rows, and audits.
- Failure condition: enabled ownerless Domain, partial replacement, extra directory data, or non-idempotent replay.

### ACC-V4-013 — Domain-admin excluded operation matrix
- Contracts: `CTR-V4-013`.
- Method/environment: attempt every excluded read/write using only global Domain-admin permission.
- Expected: all are denied without side effect or sensitive audit content.
- Required evidence: executed denial matrix and unchanged-state proof.
- Failure condition: workflow content/transition/reassignment/cancel/archive/Definition/membership/audit-content/other write succeeds.

### ACC-V4-014 — Self-grant/self-Owner attacks
- Contracts: `CTR-V4-014`.
- Method/environment: direct, alias, chained, retry, migration, same-UUID, distinct-UUID, and unproven-linkage cases.
- Expected: self-grant/same-Principal Owner denied; distinct Principals allowed unless exact accepted linkage applies.
- Required evidence: executed cases and canonical identity traces.
- Failure condition: Agent self-grants, sets itself Owner, evades via alias, or implementation invents common control.

### ACC-V4-015 — Actual actor anti-forgery matrix
- Contracts: `CTR-V4-015`.
- Method/environment: valid dedicated token plus ordinary Agent/Human/Client/Feishu/body/display/JWT-role/tool-argument forgery attempts.
- Expected: only exact token `sub` is actor; only designated Agent can proceed.
- Required evidence: verified claims, authorization trace, and durable audit actor.
- Failure condition: ordinary Agent gains global permission or any request body/self-report/Feishu field substitutes as admin actor.

### ACC-V4-016 — Disabled/revoked fail-closed matrix
- Contracts: `CTR-V4-016`.
- Method/environment: disable/revoke each Principal, Client, token, designation, binding, and permission; inject authorization-store failure.
- Expected: protected operation denies and releases/commits nothing.
- Required evidence: executed responses, publication checks, and state/audit traces.
- Failure condition: disabled Agent still reads/writes or unavailable authorization fails open.

### ACC-V4-017 — Root authority activation gate
- Contracts: `CTR-V4-017`.
- Method/environment: repository lifecycle and malformed-authority negative tests.
- Expected: all exact fields/closed permissions present; proposed/unmerged/runtime-created roots are inert.
- Required evidence: exact review/acceptance/merge and activation trace.
- Failure condition: runtime API/message/self-claim activates designation, missing field passes, or an unmerged authority grants access.

### ACC-V4-018 — Designation lifecycle controls
- Contracts: `CTR-V4-018`.
- Method/environment: token expiry, secret rotation, Client revoke, Principal/binding disable, attempted break-glass and runtime replacement.
- Expected: controls fail closed; only successor activates replacement; no break-glass exists.
- Required evidence: executed lifecycle matrix and audits.
- Failure condition: indefinite token, unrotatable/unrevocable credential, runtime grant/replacement, or break-glass authority.

### ACC-V4-019 — Compromise/replacement sequencing
- Contracts: `CTR-V4-019`.
- Method/environment: staged incident and Agent replacement rehearsal.
- Expected: old Agent stops before replacement; replacement remains denied until merged successor.
- Required evidence: ordered timestamps, revocation/binding/audit/root/main coordinates.
- Failure condition: old Agent continues after replacement/revoke or new Agent acts before successor merge.

### ACC-V4-020 — Feishu exact-ingress matrix
- Contracts: `CTR-V4-020`.
- Method/environment: vary app, tenant, conversation, sender, event signature/ID, timestamp, nonce, and replay.
- Expected: only exact fresh verified single-user ingress is admitted once.
- Required evidence: executed Agent-core gate matrix and ingress audit.
- Failure condition: non-owner sender, wrong app/tenant/conversation, stale/duplicate/unsigned event reaches command execution.

### ACC-V4-021 — Feishu provenance/actor separation
- Contracts: `CTR-V4-021`.
- Method/environment: send valid command while forging sender/body actor and compare token/audit identity.
- Expected: Agent Principal remains actor; Feishu values remain provenance.
- Required evidence: token verification, svc-workflow auth trace, audit record.
- Failure condition: Feishu sender ID becomes svc-workflow actor/permission or Human OBO is required.

### ACC-V4-022 — Correlation completeness
- Contracts: `CTR-V4-022`.
- Method/environment: end-to-end accepted and denied Feishu commands.
- Expected: all enumerated IDs correlate without sensitive bodies.
- Required evidence: joined durable records and redaction scan.
- Failure condition: missing correlation edge, actor/provenance conflation, or sensitive content copied.

### ACC-V4-023 — Protected audit coverage
- Contracts: `CTR-V4-023`.
- Method/environment: success/denial matrix for read/create/replace/designate/revoke/disable.
- Expected: one durable accountability record per required attempt with actual Agent actor.
- Required evidence: executed operation/audit joins.
- Failure condition: required success/denial lacks durable audit or records a self-reported actor.

### ACC-V4-024 — Audit failure and atomicity
- Contracts: `CTR-V4-024`.
- Method/environment: inject audit failure before read publication and during write transaction.
- Expected: read returns no protected data; write and audit both roll back.
- Required evidence: network publication capture and database transaction proof.
- Failure condition: audit fails but data is returned or write commits without audit.

### ACC-V4-025 — Publication/commit revocation race
- Contracts: `CTR-V4-025`.
- Method/environment: pause requests after initial check, revoke/disable, then release.
- Expected: no response data/no protected commit; old Agent cannot operate.
- Required evidence: synchronized race trace and state/audit results.
- Failure condition: prechecked in-flight request publishes/commits after revoke/disable.

### ACC-V4-026 — Idempotency/unknown-outcome matrix
- Contracts: `CTR-V4-026`.
- Method/environment: same/different request-key concurrency and induced lost response after commit.
- Expected: replay/conflict/exact same-key reconciliation semantics.
- Required evidence: executed receipts, hashes, outcomes, and row counts.
- Failure condition: same-key/different-request mutates, same request double-commits, or client retries unknown outcome with a new key.

### ACC-V4-027 — Audit retention/redaction
- Contracts: `CTR-V4-027`.
- Method/environment: retention boundary and seeded-sensitive-marker scan.
- Expected: exact 365-day policy; no forbidden body/credential; no runtime read/export API.
- Required evidence: lifecycle execution and API/schema inventory.
- Failure condition: early deletion, configurable longer retention under V4, sensitive content, or audit read/export surface.

### ACC-V4-028 — External authority and PR disposition
- Contracts: `CTR-V4-028`.
- Method/environment: child-authority dependency graph review at exact revisions.
- Expected: Agent permission child is independent; PR #15 remains deferred/non-prerequisite/non-active; PR #2 independent.
- Required evidence: exact external authority/PR/main coordinates.
- Failure condition: unmerged PR #15 is treated as active/prerequisite, PR #2 blocks route, Human/other Agent is granted, or local Spec governs external behavior.

### ACC-V4-029 — Layer/trust bypass scan
- Contracts: `CTR-V4-029`.
- Method/environment: architecture/source query path and direct-storage/adaptor attack review.
- Expected: authorization/redaction enforced before broad read and through application/store boundaries.
- Required evidence: call graph, query projection, access tests.
- Failure condition: UI/handler-only redaction, adapter bypass, or direct shared-DB mutation authorizes behavior.

### ACC-V4-030 — Drift truth check
- Contracts: `CTR-V4-030`.
- Method/environment: exact-base conformance report against each debt item.
- Expected: unresolved items remain DRIFTED/UNKNOWN, never VERIFIED without qualified evidence.
- Required evidence: Contract-level conformance table.
- Failure condition: existing partial implementation is declared V4-compliant by existence, tests, or runtime alone.

### ACC-V4-031 — Slice non-escalation graph
- Contracts: `CTR-V4-031`.
- Method/environment: authority/dependency/activation review plus partial-deployment tests.
- Expected: each Slice enables only itself; missing unrelated Human/Assistance/Recovery authority does not block; missing own child does.
- Required evidence: accepted authority graph and per-Slice activation results.
- Failure condition: one Slice silently activates another or a common Human gate is imposed.

### ACC-V4-032 — Exact successor scope drift
- Contracts: `CTR-V4-032`.
- Method/environment: reviewed plan, exact pair/nine rows, changed pair/row/live-current fixtures.
- Expected: only exact eligible plan succeeds; every drift commits zero.
- Required evidence: executed plan digest, pre/post rows, Receipt/audit.
- Failure condition: arbitrary pair, non-nine Domain scope, historical/ineligible responsibility, or drift commits.

### ACC-V4-033 — Successor history preservation
- Contracts: `CTR-V4-033`.
- Method/environment: pre/post digest and row-level history comparison.
- Expected: 58 historical assignments and 111 Visits unchanged; only new successor facts appended.
- Required evidence: executed digests, row counts, new fact lineage.
- Failure condition: any historical attribution is updated/deleted/relabeled or successor lacks append-only facts.

### ACC-V4-034 — Successor atomic NOOP/surface/gates
- Contracts: `CTR-V4-034`.
- Method/environment: failure injection, exact rerun, mismatched rerun, API/SDK inventory, production-gate review.
- Expected: all-or-nothing; exact rerun zero writes/audits; no general surface; production remains separately gated.
- Required evidence: executed transactions, counts, surface diff, gate record.
- Failure condition: partial commit, rerun side effect, reusable reassignment surface, or Spec/merge alone authorizes production.

### ACC-V4-035 — BIP exact-pair negative matrix
- Contracts: `CTR-V4-035`.
- Method/environment: child/tool review using exact BIP pair, `blog-agent`, CTO pair, and arbitrary UUID fixtures.
- Expected: only exact BIP OLD to NEW is selectable under the BIP exception; all excluded/other pairs fail before writes.
- Required evidence: exact source constants, CLI/surface inventory, executed negative matrix.
- Failure condition: label/dynamic input widens pair, excluded Principal is selected, or CTO authority is modified.

### ACC-V4-036 — BIP live-plan scope and zero-responsibility case
- Contracts: `CTR-V4-036`.
- Method/environment: production read-only reviewed plan plus disposable changed/missing/extra/zero fixtures.
- Expected: exact live enabled tuples and eligible current responsibilities are frozen; zero responsibility is valid; drift writes zero.
- Required evidence: canonical plan/digest, tuple identities, eligibility fields, before snapshot, and zero-write proof.
- Failure condition: expected count substitutes for live rows, an ineligible task enters scope, zero reactivates history, or drift commits.

### ACC-V4-037 — BIP Domain atomic successor
- Contracts: `CTR-V4-037`.
- Method/environment: disposable Owner/Member/failure/concurrent/rerun matrix.
- Expected: exact Role/Domain preserved, one effective Owner, no committed dual authority, atomic rollback, exact NOOP.
- Required evidence: pre/post tuples, fault-injection transaction proof, Receipt/Audit counts.
- Failure condition: partial Domain transfer, role change, dual authority, mismatched rerun repair, or extra audit.

### ACC-V4-038 — BIP append-only responsibility and history
- Contracts: `CTR-V4-038`.
- Method/environment: disposable current/terminal/completed/cancelled/archived/history/CAS/replay matrix.
- Expected: same Instance/node successor chain is appended only for reviewed eligible rows; every old fact remains immutable.
- Required evidence: before/after row digests, CAS outcomes, new Visit/Event/Receipt/Audit lineage.
- Failure condition: history changes, wrong Instance/node, ineligible reactivation, missing dedicated fact, or legacy archive access.

### ACC-V4-039 — BIP plan/apply gate sequence
- Contracts: `CTR-V4-039`.
- Method/environment: authority/implementation/plan/execution-record lifecycle review.
- Expected: all nine ordered gates occur; canonical plan digest is reviewed; apply lacks authority until separate exact approval.
- Required evidence: exact commits, plan bytes/SHA, review receipt, execution authorization, apply/verify/NOOP transcript.
- Failure condition: any earlier milestone implies apply, write precedes reviewed plan, or exact NOOP is absent.

### ACC-V4-040 — No general successor surface
- Contracts: `CTR-V4-040`.
- Method/environment: Product Direction diff plus HTTP/OpenAPI/SDK/CLI/database-path inventory.
- Expected: V3 prohibitions remain and only the offline exact-pair exception exists.
- Required evidence: semantic diff, surface inventory, excluded-operation tests.
- Failure condition: general migration/reassign/handoff/delegate/API/manual-SQL/database-switch/history/archive capability exists.

### ACC-V4-041 — PR #9 alignment-only diff
- Contracts: `CTR-V4-041`.
- Method/environment: after V4 acceptance/merge only, compare exact blocked Head with proposed child amendment.
- Expected: only parent/base/revision coordinates, authority observations, and lawful-parent blocker state change.
- Required evidence: exact PR #9 semantic diff and independent child re-review.
- Failure condition: any frozen BIP product semantic changes or child implementation/apply authority is inferred.

```text
CONTRACT_COUNT = 41
CONTRACTS_WITH_ACCEPTANCE = 41
ACCEPTANCE_COUNT = 41
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
- Directly amend accepted V3 for the BIP pair: rejected; exact-pair meaning changes, so V4 is a whole-authority successor.
- Generalize CTO/BIP into arbitrary Principal migration: rejected; each exception remains independently exact and bounded.
- Use PR #15 or mutable PR #2 as common prerequisites: rejected; both are independent from Agent-first V1.

## 23. Migration, compatibility, containment, and rollback

V4 acceptance, if later performed, is docs-only and does not mutate runtime. Existing `GLOBAL_WORKFLOW_COORDINATOR` bindings do not auto-map, alias, or grant either split permission. Activation requires all relevant child authorities and exact designation; absent them, global operations remain fail closed.

Capability rollout is Slice-scoped. Rollback of code/config is owned by each child. Security containment may revoke/rotate Client, disable binding, disable Principal, and record durable audit without creating a replacement authority. Product-direction rollback before runtime implementation is a Git revert of the complete accepted transition. After dependent authorities/implementation exist, a lawful whole-authority successor and child rollback plan are required; neither V3 nor V2 is silently revived.

The retained CTO migration and additive BIP exception each keep separate implementation, reviewed-plan, and production execution gates. PR #7 remains independent. No current HTTP Contract, OpenAPI, SDK, Rust, SQL, migration, test, deployment, auth-service, or dsh-agent-core change is made or authorized here.

## 24. Open Questions and authoring readiness

```text
OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE (V4 lawfully proposes whole-authority resolution)
PARTIAL_SUPERSESSION = NONE
DUPLICATE_AUTHORITY_RISK = NONE
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
CHILD_IMPLEMENTATION_SPEC_REQUIRED = YES
AUTHORING_READY_FOR_REVIEW = YES

TRUSTED_AGENT_ROOT_REQUIRED = YES
ROOT_AUTHORITY_ID = SVC_WORKFLOW_TRUSTED_ADMIN_AGENT_ROOT_V1
ADMIN_AGENT_STRATEGY = NEW_DEDICATED_AGENT
HUMAN_PRINCIPAL_ADMINISTRATION_REQUIRED_FOR_V1 = NO
HUMAN_OBO_REQUIRED_FOR_V1 = NO
TWO_PERSON_APPROVAL_REQUIRED_FOR_V1 = NO
```

The exact Admin Agent Principal UUID and Client ID are intentionally not open Product Direction decisions: they are required fields owned by the later designation authority. Implementation Agents have no discretion to choose or activate them outside that authority.

## 25. Proposed lifecycle record

```text
ACCEPTANCE_STATUS = NOT_PERFORMED
STATUS = proposed
AUTHORING_BASE = 327b74f138151a7f4d9d88e3881e54d203f1e8f6
BLOCKED_CHILD_PR = 9
BLOCKED_CHILD_HEAD = 3056263c3fc964a2b225720dd2b859b47e296c2e
V3_STATUS_ON_MAIN = accepted
V3_TRANSITION = NOT_PERFORMED
IMPLEMENTATION_AUTHORITY = none
PRODUCTION_APPLY_AUTHORITY = none
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
PRODUCTION_APPLY_AUTHORIZED = NO
INDEPENDENT_REVIEW_REQUIRED = YES
OWNER_ACCEPTANCE_REQUIRED = YES
FINAL_HEAD_RECHECK_REQUIRED = YES
MERGE_PERFORMED = NO
PRODUCTION_CHANGE = NONE
```

This authoring change only proposes V4. It does not edit accepted V3, modify blocked Child PR #9, finalize acceptance, update the repository authority map, authorize implementation, query production, mutate any Domain/workflow/history fact, merge, or authorize production apply.
