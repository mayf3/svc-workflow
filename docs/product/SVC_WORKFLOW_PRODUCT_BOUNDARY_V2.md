---
authority_id: SVC_WORKFLOW_PRODUCT_BOUNDARY_V2
status: proposed
authority_kind: product_direction
owning_repository: mayf3/svc-workflow
supersedes:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V1
superseded_by: null
owners:
  - mayf3
---

# SVC_WORKFLOW_PRODUCT_BOUNDARY_V2

## 1. Authority status and purpose

This document proposes the complete Product Direction boundary for `svc-workflow`. It restates the still-valid product boundary in full and adds one explicit, authorized, and bounded global control-plane exception to otherwise strict Domain isolation.

```text
AUTHORITY_ID = SVC_WORKFLOW_PRODUCT_BOUNDARY_V2
AUTHORITY_KIND = product_direction
STATUS = proposed
SUPERSEDES = SVC_WORKFLOW_PRODUCT_BOUNDARY_V1
WHOLE_AUTHORITY_SUPERSESSION = YES
PARTIAL_SUPERSESSION = NONE
PRODUCT_DIRECTION_AUTHORIZES_IMPLEMENTATION_DIRECTLY = NO
```

This proposal is not active authority. `SVC_WORKFLOW_PRODUCT_BOUNDARY_V1` at `PRODUCT-BOUNDARY.md` remains the current Product Direction until an acceptance-only transition atomically accepts this V2, supersedes V1 with the required backlink and authority-map update, passes an independent final-head recheck, and the accepted transition is merged into `main`.

This authoring change does not modify V1, accept either authority, authorize implementation, declare current code compliant, or change product/runtime state.

```text
AUTHORING_BASE_COMMIT = c7830e58578d7c7360710f2449c48cb801da773e
V1_HISTORICAL_PATH = PRODUCT-BOUNDARY.md
V1_DATE_AND_SOURCE_PROVENANCE = preserved unchanged at the historical path
```

## 2. Product positioning

`svc-workflow` is a platform-level, serial, governed workflow engine for fixed Agent, Human, and Service Principals. It owns workflow definitions and workflow execution facts needed to provide:

- versioned Workflow Definition governance;
- Workflow Instance lifecycle management;
- legal Transition execution;
- immutable, event-sourced workflow history;
- strict normal-data-plane Domain isolation;
- assignee- and creator-oriented worklists;
- Domain-local administration and provisioning;
- an explicitly bounded global control plane for scheduling metadata and Domain bootstrap/Owner replacement;
- idempotent, concurrency-safe state commands.

The engine guarantees that a known Principal acts on an authorized workflow at its current node, against an explicit Definition Version and workflow state version, and that every committed state change has an immutable history. It validates workflow structure and JSON protocol shape; it does not understand the business meaning or truth of payload content and does not run an LLM.

The qualifying V2 workflow shape remains:

```text
one current node per Workflow Instance
one concrete current assignee
one deterministic normal forward path
JSON stage delivery
configured backward RETURN paths
configured or governed termination paths
```

Parallel nodes, dynamic forward branching, claim/pull assignment, ordinary reassignment, handoff, delegation, timers, external signals, automatic retry, SLA orchestration, arbitrary script guards, built-in LLM execution, cross-Domain shared templates, in-flight template replacement, and in-flight Domain transfer are not product capabilities authorized by this Product Direction.

## 3. Product-owned concepts

### 3.1 Workflow Definition

A Workflow Definition is a Domain-owned, versioned workflow template. It owns the node graph, Transition graph, assignee references, Context and Submission schemas, and the deterministic normal path.

Definition Versions follow the lifecycle:

```text
DRAFT -> PUBLISHED -> DEPRECATED -> REVOKED
```

- `DRAFT` may be edited and validated but cannot create normal production instances.
- `PUBLISHED` may create instances and is immutable after publication.
- `DEPRECATED` cannot create new instances; existing instances may continue as allowed by governing Architecture and child Specs.
- `REVOKED` prevents normal use according to governing Architecture and implementation-authorizing Specs.

Definition governance includes version creation, validation, publication, deprecation, revocation, and non-destructive archival/discovery. A published Definition Version's graph, schemas, assignee references, ordering, validator semantics, and digest inputs are immutable; archival MUST NOT rewrite it. A Definition belongs to exactly one Domain; similar workflows in another Domain use a separate Definition unless a future accepted Product Direction supersedes this rule. Exact archive state and wire behavior remain child-Spec concerns.

### 3.2 Workflow Instance

A Workflow Instance is an independent execution of one immutable Definition Version inside one Domain. It owns its current Context Revision, current Node Visit, state version, lifecycle/governance metadata, and references to immutable history.

An Instance does not become an upper-layer business object. Optional external references or lightweight metadata may correlate it to another product, but the upper layer owns that business object's identity and complete data.

The lifecycle includes creation, Context revision where authorized, Transition, graph-external Domain Owner cancellation, and non-destructive archive. Instances are not physically deleted through a normal product API. Cancel and archive retain immutable workflow facts and remain subject to their accepted Architecture authority.

### 3.3 Transition

A Transition is a legal edge from the locked current node under the Instance's immutable Definition Version:

- `ADVANCE` follows the configured normal direction, including normal completion into a terminal node;
- `RETURN` moves to an allowed earlier non-terminal node and creates a new Node Visit;
- `TERMINATE` follows a configured graph edge into an exceptional terminal node;
- Domain Owner `CANCEL` is a separate, graph-external governance command and is not a Transition effect.

A normal Transition is performed only by the authorized current assignee and commits its Submission, target Node Visit, projection update, state-version increment, Workflow Event, and command outcome atomically. Domain ownership, a broad scope, or a global control-plane permission does not implicitly grant Transition authority.

### 3.4 Context, Node Visit, and Submission

Workflow Context is versioned workflow input needed to execute one Instance. It is not the complete upper-layer business record. Context revisions are immutable and form a single chain. Context mutation is restricted by the governing workflow rules, including the Draft/creator boundary.

A Node Visit is an immutable record of one entry into one node, including the assignee snapshot. Owner or Definition changes do not rewrite an existing Visit.

A Submission is the immutable JSON stage-delivery primitive. `svc-workflow` does not introduce separate business entities for report, comment, artifact, or evidence; large resources may be referenced by URI and digest. Schema validation establishes shape, not business truth.

## 4. Event sourcing and authoritative history

Every successful workflow state command must produce an immutable, auditable state history according to the governing Architecture and accepted child Specs. The core authoritative workflow history consists of immutable facts including:

```text
WorkflowContextRevision
NodeVisit
Submission
WorkflowEvent
```

The Workflow Instance's current Context, current Node Visit, and workflow state version are query projections over those facts. A successful state command changes the state version once and records the corresponding Workflow Event once; partial workflow-fact commits are forbidden.

Timeline views are projections of immutable events and related immutable facts. They do not create a second history authority. Global scheduling access defined in this V2 does not grant timeline access or EventData access.

## 5. Domain and normal data-plane isolation

A Domain is the workflow business-ownership, Definition-management, permission, and audit boundary. Each Definition and Instance belongs to one Domain. Domain ownership is represented by the canonical Domain role-binding authority, not duplicated as an unrelated second owner field.

```text
NORMAL_DATA_PLANE_DOMAIN_ISOLATION = STRICT
GLOBAL_CONTROL_PLANE_EXCEPTION = AUTHORIZED_AND_BOUNDED
```

For ordinary data-plane behavior:

- an ordinary Agent, ordinary member, or Domain Owner MUST NOT see another Domain merely because that Domain exists;
- Domain Owner authority applies only within the owned Domain;
- a current assignee receives only the Instance-local access required by the governing workflow rules;
- a historical participant receives only the explicitly governed history related to that participation;
- scopes, allowlists, role combinations, service identity, Feishu identity, or possession of multiple Domain-local roles MUST NOT implicitly create cross-Domain authority;
- object lookup, lists, counts, cursors, denial behavior, and serialization MUST preserve the isolation boundary and MUST NOT leak another Domain's existence or facts;
- cross-Domain authority exists only through the two explicit global control-plane permissions in §8 and only for their enumerated operations and fields.

No lower-level Architecture, implementation Spec, API, SDK, migration, code path, test, deployment state, legacy role, or UI label may broaden this exception.

## 6. Worklists and Domain-local operational views

`svc-workflow` owns workflow worklists and workflow projections, including:

- current tasks assigned to the authenticated Principal;
- creator-owned drafts where the workflow rules permit creator action;
- Domain-local Instance and audit views for the effective Domain Owner;
- feedback projections showing how a Principal's own Submissions were advanced, returned, or concluded, where authorized.

A worklist item is a view of a current Node Visit or other explicitly governed workflow fact; it is not a Todo or other upper-layer business object. Ordinary worklists remain Domain- and participation-scoped. The global scheduling view in §9 is a separate, bounded control-plane projection and is not an ordinary worklist or a full Instance view.

## 7. Admin and provisioning ownership

`svc-workflow` owns the local workflow projections and bindings needed to administer Principals, Domains, Domain membership, Domain ownership, and workflow roles. Authentication and global identity authority remain external to `svc-workflow`.

Domain-local administration remains bounded:

- a Domain Owner may manage only the Domain-local capabilities granted by accepted authority;
- a Domain Owner does not become a global administrator;
- one Domain's role cannot authorize another Domain's reads or writes;
- Principal identity for an action comes from verified authentication, not request-body identity;
- enabled Domains must have a valid effective Owner as required by governing Architecture;
- Owner replacement does not rewrite existing Node Visit assignee snapshots.

The global Domain administration exception is exactly §10; it does not grant workflow data-plane authority.

## 8. Split global permission model

V2 defines exactly two independent global control-plane permissions:

```text
GLOBAL_PERMISSIONS =
  GLOBAL_SCHEDULER_READ
  GLOBAL_DOMAIN_ADMIN
PERMISSION_MODEL = SPLIT
SAME_PRINCIPAL_MAY_HOLD_BOTH = YES
GLOBAL_WORKFLOW_COORDINATOR = UI_LABEL_ONLY
```

`GLOBAL_WORKFLOW_COORDINATOR` is not a permission, role, migration target, authorization fact, or compatibility alias. A UI may display that label for a Principal who holds an appropriate permission combination, but authorization MUST evaluate `GLOBAL_SCHEDULER_READ` and `GLOBAL_DOMAIN_ADMIN` separately.

Holding one global permission does not imply the other. Holding both does not merge their authority or create any third capability. Neither permission grants full workflow content access, Transition authority, cancel/archive authority, Definition management, Domain membership management, Assistance content access, credential access, or audit-payload access.

## 9. `GLOBAL_SCHEDULER_READ`

### 9.1 Purpose and scope

`GLOBAL_SCHEDULER_READ` supports deployment-wide human or Agent scheduling using only metadata for current active tasks. It is a workload-visibility capability, not a timer, dispatch, automatic retry, SLA orchestration, external-signal, or workflow-transition engine.

```text
FULL_CONTENT_ACCESS_REQUIRED = NO
SCHEDULING_VIEW_SCOPE = ACTIVE_CURRENT_TASK_METADATA_ONLY
TASK_LABEL = NOT_INCLUDED_IN_V2
CONTEXT_TITLE_AS_METADATA = FORBIDDEN
```

The logical scheduling projection contains one record per active current task. In every record, `principalId` is the scheduling subject and MUST equal `currentAssigneePrincipalId`; `activeTaskCount` is the total number of active current-task records for that Principal at the projection snapshot. A child wire contract MAY group records under a Principal summary or repeat Principal/count fields per task, but it MUST preserve those semantics and MUST NOT change the authorized data.

All listed identity, Domain, Definition, current-node, lifecycle/status, assignee, count, and timestamp fields are required. Node type, lifecycle, and status values MUST come from closed, non-sensitive code sets frozen by the child implementation Contract; free-form text is forbidden. `updatedAt` means the timestamp of the latest committed Workflow Instance state change represented by the current authoritative Instance projection. It is not a read time, cache refresh time, scheduler observation time, audit time, Assistance time, or external-system update time.

The scheduling view MUST include only active, current-task records. Archived, cancelled, terminal-without-current-task, historical Node Visit, and non-current task records MUST NOT be returned under this permission under any filter, mode, grouping, or pagination option. A child implementation Spec may define pagination and stable filtering only within this active-current-task population; it may not add fields or lifecycle scope beyond this Product Direction. Assistance status or blocking-status fields may be added only by a later accepted Product Direction or a whole-authority successor to V2; a child implementation Spec cannot add them.

### 9.2 Complete allowed field set

The complete V2 scheduling projection is limited to:

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

These fields are the complete V2 scheduling metadata allowlist. Their authorization does not make any similarly named field in another payload globally readable. In particular, `blockedFlag`, `blockedReasonCode`, `waitingAssistance`, and `assistanceStatus` are not V2 scheduler metadata.

### 9.3 Forbidden scheduling content

`GLOBAL_SCHEDULER_READ` MUST NOT expose:

- Context title;
- any Context payload or Context Revision body;
- Submission payload or history;
- Assistance request, escalation, resolution, or supporting payload;
- timeline `EventData` or event payload bodies;
- credential, token, Receipt, command-attempt, SecurityAudit, or other audit payload;
- archived, cancelled, terminal-without-current-task, historical, or non-current records under any scheduling filter or mode;
- task label (not included in V2);
- Transition options or write capability;
- any derived field that reconstructs or summarizes forbidden content.

A title located inside Context remains Context content and MUST NOT be relabeled as metadata. Full-content access is not necessary to perform the authorized scheduling purpose.

## 10. `GLOBAL_DOMAIN_ADMIN`

### 10.1 Allowed operations

`GLOBAL_DOMAIN_ADMIN` authorizes only:

1. idempotent Domain creation;
2. atomic assignment of an initial Domain Owner during creation, or creation/retention of the Domain in a disabled state until a valid Owner is ready;
3. atomic replacement of a Domain Owner;
4. reading the minimum directory data needed to select a Domain and a Principal for those operations.

Domain creation and Owner replacement must preserve the one-effective-Owner invariant, canonical Principal identity, idempotency, transactional integrity, and durable audit requirements. An enabled Domain MUST NOT be left ownerless by partial success.

Minimum directory data is a selection surface, not a workflow-data surface. Its complete V2 field set is:

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

Directory results MUST be one logical record per Domain or Principal, MUST support selection only for Domain create/Owner replace, and MUST NOT include membership, workflow counts, workflow facts, email, Feishu identifiers, credentials, scopes, permission bindings, audit data, or content-derived fields. A child implementation Spec may freeze query/search, pagination, required/nullable display-name behavior, and exact wire grouping, but it MUST NOT add globally readable fields or infer new authority.

### 10.2 Explicitly excluded authority

`GLOBAL_DOMAIN_ADMIN` does not grant:

- Workflow Instance, Context, Submission, Node Visit, timeline, Assistance body, or worklist data access;
- Transition, Context revision, cancel, or archive authority;
- Workflow Definition creation, modification, publication, deprecation, revocation, or assignment;
- Domain membership management;
- Domain-local audit-content access merely because the caller is a global administrator;
- permission grant/revoke authority to the caller itself;
- authority to set the caller as Domain Owner;
- authority to use a body `principalId`, Feishu identity, service identity, scope, or allowlist as the authenticated actor.

```text
GLOBAL_ADMIN_SELF_GRANT = FORBIDDEN
GLOBAL_ADMIN_SELF_DOMAIN_OWNER = FORBIDDEN
```

These prohibitions apply directly and through aliases, chained calls, role combinations, retries, migrations, or compatibility routes. The immediate self-Owner rule is canonical Principal equality: if `authenticatedActorPrincipalId == newOwnerPrincipalId`, Owner replacement MUST be rejected. When the two values are different canonical Principal UUIDs, `svc-workflow` treats them as distinct Principals, MUST NOT infer common control, MUST NOT require proof that no linkage exists, and MUST NOT reject merely because linkage evidence is absent.

```text
SELF_DOMAIN_OWNER_RULE = CANONICAL_PRINCIPAL_EQUALITY
NO_ACCEPTED_LINKAGE_AUTHORITY = TREAT_DISTINCT_CANONICAL_PRINCIPALS_AS_DISTINCT
IMPLICIT_COMMON_CONTROL_INFERENCE = FORBIDDEN
FAIL_CLOSED_WHEN_LINKAGE_IS_ABSENT = NO
```

Only an exact accepted external identity authority that explicitly establishes a canonical controller/identity linkage between Principal A and Principal B may extend the self-Owner prohibition to that accepted linkage set. An accepted child Spec must pin that authority before using the linkage. No implementation Agent, runtime heuristic, request assertion, Feishu identity, body field, or service identity may invent or infer it.

## 11. Global permission lifecycle

The two global permissions MUST be:

- granted separately;
- revoked separately;
- audited separately;
- independently evaluated on every protected request;
- allowed to coexist on the same Principal without authority union beyond §§9-10.

Normal grants use:

```text
NORMAL_GRANT_DEFAULT_TTL = 30_DAYS
NORMAL_GRANT_MAX_TTL = 90_DAYS
GRANT_REQUIRES_TWO_PERSON_APPROVAL = YES
REVOKE_MAY_BE_PERFORMED_BY_ONE_AUTHORIZED_SECURITY_ACTOR = YES
BREAK_GLASS = NOT_SUPPORTED_IN_V2
```

Every grant MUST have an expiry. Omitted requested expiry means exactly 30 days; an explicit requested expiry may be longer than 30 days but MUST NOT exceed 90 days. No grant is indefinite. Renewal is a new grant decision and MUST repeat the same approval process. Expiry and revocation take effect immediately and fail closed.

GitHub/repository identity `mayf3` is the repository governance owner and acceptance actor; it is not a runtime actor and MUST NOT be treated as a canonical Principal. Before any global-permission grant, renewal, approver-roster mutation, or product activation, a separate repository-owned security invariant must be accepted and present on `main`:

```text
ROOT_AUTHORITY_ID = SVC_WORKFLOW_GLOBAL_PERMISSION_GOVERNANCE_ROOT_V1
ROOT_AUTHORITY_KIND = repository-owned security invariant
ROOT_AUTHORITY_OWNER = mayf3
```

That authority must record the repository governance owner `mayf3`, one exact `canonicalRootPrincipalId` from auth-service, at least two distinct exact canonical `initialSecurityApproverPrincipalIds`, activation time, `supersedes`, `superseded_by`, and owners. V2 freezes its necessity and semantics but does not create or accept it.

```text
NO_ACTIVE_ACCEPTED_ROOT_AUTHORITY =
  GRANT_DISABLED
  RENEWAL_DISABLED
  APPROVER_ROSTER_CHANGE_DISABLED
```

Root bootstrap, rotation, recovery, and every security-approver roster change require a new docs-only revision/whole-authority transition for `SVC_WORKFLOW_GLOBAL_PERMISSION_GOVERNANCE_ROOT_V1`, acceptance by repository owner `mayf3`, independent audit, and presence on `main`. Runtime APIs, Feishu Bots, global-permission holders, request bodies, and services MUST NOT create or modify the root mapping or roster. When a new accepted root authority becomes active, the old root and old roster cease authorizing new governance actions immediately. If the canonical root identity is unavailable, the system MUST NOT fall back to GitHub username, body actor, Feishu identity, Adapter identity, or service identity.

The initial and current security-approver roster comes only from the active accepted root authority. The canonical root Principal is not automatically an approver unless explicitly listed. A grant or renewal requires approvals from exactly two distinct active security approvers; the requester never counts as an approval; the subject MUST NOT approve itself; and a security approver MUST NOT self-grant. After two valid approvals exist, finalization is a deterministic system action with no discretionary runtime finalizer. Holding `GLOBAL_SCHEDULER_READ`, `GLOBAL_DOMAIN_ADMIN`, or both grants no request, approval, roster, finalization, renewal, or revoke authority.

Emergency revoke remains available to the active canonical root Principal or any one current active security approver, without a second approver. A child implementation Spec must freeze storage, API, race, and evidence mechanics while preserving this complete eligibility model.

Legacy `GLOBAL_WORKFLOW_COORDINATOR` bindings MUST NOT auto-migrate. Every legacy holder must be explicitly reviewed and mapped to:

```text
GLOBAL_SCHEDULER_READ only
GLOBAL_DOMAIN_ADMIN only
both permissions
zero permissions
```

No default, role-name mapping, inferred usage, or compatibility alias may choose that mapping. V2 provides no break-glass bypass; urgent containment uses revocation or disablement, not a temporary unaudited grant.

## 12. Feishu entry and on-behalf-of identity

Feishu is an allowed human-operator entry channel, not an identity or permission authority.

```text
FEISHU_HUMAN_OPERATOR = ALLOWED_VIA_OBO
FEISHU_IS_PERMISSION_SOURCE = NO
FEISHU_HUMAN_OBO = TARGET_PRODUCT_DIRECTION
AUTH_SERVICE_HUMAN_OBO_CURRENTLY_AVAILABLE = NO
FEISHU_HUMAN_IMPLEMENTATION_BLOCKED_UNTIL_AUTH_SERVICE_AUTHORITY = YES
DIRECT_TOKEN_SLICES_BLOCKED_BY_HUMAN_OBO = NO
```

This is a target Product Direction, not a claim that an implementable Human OBO contract already exists. At exact auth-service revision `450a0ecb286cbe5da6e790d3c572fa71218ca9c0`:

- repository `mayf3/auth-service`, authority `MINIMAL_AUTH_FOUNDATION_V1`, normative path `docs/contracts/minimal-auth-v1/`, status `FROZEN_TARGET_CONTRACT`, relationship `constrained_by`, explicitly freezes `HUMAN_OBO=false`;
- repository `mayf3/auth-service`, authority `AUTH_SERVICE_WORKFLOW_AGENT_OBO_V0_FROZEN`, path `docs/contracts/WORKFLOW_AGENT_OBO_TOKEN_EXCHANGE_V0.md`, status frozen and still potentially governing until the V1 effectiveness/supersession gates complete, relationship `constrained_by`, supports Agent OBO only and defers User/Human OBO.

Therefore every Feishu Human implementation is blocked until `mayf3/auth-service` establishes and accepts a whole-authority Human OBO successor and a later accepted svc-workflow implementation Spec pins it. Agent-only OBO, a request-body actor, message or mention identity, Feishu user identity, and Adapter/service identity MUST NOT substitute for that missing Human OBO authority.

The target identity boundary is:

- the canonical Human Principal is the actor whose global permission is evaluated;
- the Feishu Adapter acts as the service/client identity and, only after the blocking authority transition above, uses the accepted auth-service Human on-behalf-of successor;
- the verified `token.sub` is the actual human operator;
- the adapter/service identity, `act`, or equivalent delegation claim identifies the intermediary only as defined by the auth contract and does not replace `token.sub` as actor;
- message text, mention identity, request-body `principalId`, Feishu user identifier, or chat-room membership MUST NOT impersonate or substitute for the authenticated actor;
- the Feishu event ID is used for idempotency and durable audit correlation;
- the bot or Adapter MUST NOT self-grant, approve its own grant, or derive permission from Feishu configuration.

Direct authenticated channels and Feishu-via-OBO must converge on the same canonical permission and audit decisions. Feishu unavailability does not weaken authorization or create a fallback identity path.

## 13. Audit, retention, and failure policy

Every successful or authenticated-denied attempt for a protected global-control-plane operation requires a durable audit record, including:

- permission request, approval, deterministic finalization, renewal, revoke, and expiry processing;
- Domain creation;
- Domain Owner replacement;
- global scheduling reads;
- global Domain and Principal directory reads.

Unauthenticated denials reuse the existing authentication authority's denial and security-audit semantics. They do not require a nonexistent canonical actor and MUST NOT promote unverified token, message, mention, body, or Feishu fields into actor facts.

Audit records must identify the canonical actor when one is authenticated, subject/target, permission or operation, decision/result, time, idempotency/correlation identity where applicable, and non-sensitive reason codes needed for accountability. They MUST NOT copy sensitive workflow content, Context payloads, Submissions, Assistance content, credentials, tokens, Receipt bodies, unrestricted request bodies, or unrestricted response payloads.

```text
AUDIT_RETENTION = 365_DAYS
FAILURE_POLICY = FAIL_CLOSED
AUDIT_PRODUCT_READ_API = NOT_SUPPORTED_IN_V2
EXTERNAL_AUDIT_EXPORT = NOT_SUPPORTED_IN_V2
PERMISSION_EXPIRY = FAIL_CLOSED_AT_EXPIRES_AT
EXPIRY_AUDIT = RECONCILE_LATER_WITHOUT_EXTENDING_AUTHORITY
```

The V2 retention period is exactly 365 days. Earlier deletion and a product-configurable longer retention period are not supported by V2; any different retention direction requires a later accepted Product Direction. V2 authorizes durable audit writes only; it does not authorize a product runtime audit query API, make the repository/product Owner a runtime audit reader, or authorize external audit export. Any future read or export product capability requires later accepted Product Direction.

Permission authority ends fail-closed at its authoritative `expiresAt` even when the expiry-time audit write or reconciler is unavailable. The expiry lifecycle audit may be appended during the next successful reconciliation, with the original `expiresAt` and reconciliation time distinguished; delayed audit MUST NOT extend, revive, or imply continued permission authority.

If authorization state or a required request-time durable audit is unavailable, the protected operation MUST fail closed. A successful read must durably record its audit before any protected data is released. A successful write and its required audit must commit atomically. Authorization is not complete merely because a permission check once succeeded: if revocation or expiry takes effect before the authorization publication/commit barrier, an older in-flight request MUST neither release protected response data nor commit a protected write.

For writes, if the client cannot know whether the authoritative transaction committed, the result must be `outcome_unknown` (or the exact child-Spec wire equivalent). The client must retry only the exact request with the same idempotency key. It MUST NOT create a new idempotency key and blindly repeat the write.

Exact HTTP status codes, database schema, transaction/lock order, Event structure, SDK types, and operational rollout mechanics are not selected by this Product Direction; they are owned by accepted implementation-authorizing child Specs.

## 14. Idempotency and concurrency product boundary

State-changing workflow and control-plane commands require client-supplied idempotency identities and canonical request comparison. The service must:

- scope and bind an idempotency key to the authenticated actor and complete command meaning;
- replay the original completed outcome for the same key and same request;
- reject the same key with a different request without altering the original outcome;
- serialize conflicting state changes and enforce optimistic workflow state versioning where applicable;
- commit workflow facts, projections, events, receipts, and required audits atomically according to accepted child Contracts;
- return an unknown-outcome class when commit certainty is unavailable;
- prevent retries, role revocation races, Owner replacement races, or compatibility paths from bypassing current authorization.

The exact request-hash envelope, receipt states, lock order, database constraints, response codes, and retry intervals remain child-Spec concerns.

## 15. External platform and ownership boundaries

### 15.1 `auth-service`

`auth-service` owns global identity, authentication, token issuance, token exchange/OBO, and signing-key publication. `svc-workflow` does not issue identity or access tokens and does not treat a Feishu identifier or body field as an identity authority.

The preserved V1 machine/service boundary is RS256 verification using auth-service JWKS, with audience `aud=svc-workflow` and a canonical Machine Principal UUID in `token.sub`; verification is offline at request authorization time subject to the bounded JWKS cache trade-off in §19. Current cross-service delegation is Agent-only under the exact authorities and revision pinned in §12. Those authorities explicitly do not support Human OBO.

V2 requires a future Human-via-OBO path in which verified `token.sub` is the canonical Human Principal and the Adapter/service identity remains only the intermediary identity. This requirement is blocked, not currently implementable. Before any Feishu Human implementation, auth-service must accept a whole-authority successor that authorizes Human OBO, and an accepted svc-workflow child Spec must pin that successor's exact revision, token-use, actor/delegation claims, key validation, cache/revocation behavior, and service-to-service wire contract. This Product Direction does not redefine or partially supersede auth-service behavior.

### 15.2 `adc-v2` and upper-layer business services

`adc-v2` and other business layers own business logic and business objects such as Requirement, Todo, project, priority, task label, Article, Campaign, and their long-lived business state. `adc-v2` uses auth-service `client_credentials` to obtain its workflow machine token, may correlate a business object to a `workflowInstanceId`, and calls `svc-workflow` only through accepted contracts; it does not directly mutate workflow-owned storage or persist a competing workflow state authority.

`svc-workflow` is the authority for workflow Definitions, Instances, current workflow projection, immutable workflow facts, and legal workflow transitions. It does not decide business approval rules beyond the configured workflow protocol.

### 15.3 `architecture-portal` and UI

`architecture-portal` or another UI layer owns presentation, navigation, interaction, and the display label `GLOBAL_WORKFLOW_COORDINATOR`. `svc-workflow` owns neither UI nor frontend behavior. A UI label does not grant permission and must be resolved to the two explicit permissions before calling protected operations.

### 15.4 External integrations

Message delivery, Feishu transport, email, notification, third-party integration, Agent dispatch, and external-system orchestration belong to integration adapters or business services. `svc-workflow` may expose bounded APIs and consume an authenticated request, but it does not own transport delivery or derive permission from external-message content.

## 16. Requirement ownership guide

The following terms identify product ownership and routing; they do not grant authorization by themselves:

| Requirement language | Owning product boundary |
|---|---|
| workflow, instance, current node, node advance, approval flow | `svc-workflow` |
| state machine, Transition, `ADVANCE`, `RETURN`, `TERMINATE` | `svc-workflow` |
| event sourcing, timeline, immutable workflow event | `svc-workflow` |
| Definition, workflow template, version publication, graph validation | `svc-workflow` |
| Domain, workflow Domain, normal cross-Domain isolation | `svc-workflow` |
| workflow worklist, assigned-to-me, creator-owned draft | `svc-workflow` |
| Instance cancel or archive | `svc-workflow` |
| upper-layer Requirement, Todo, task board, article, campaign, or business approval rule | `adc-v2` or the relevant business product |
| UI presentation and interaction | `architecture-portal` or another UI product |
| identity proofing and token issuance | `auth-service` |
| Feishu transport, notification delivery, email, or third-party integration | integration Adapter/layer |

## 17. Explicit non-ownership

`svc-workflow` does not own or provide:

- UI rendering or frontend interaction;
- upper-layer business objects or their complete content;
- business-specific decision logic;
- identity proofing, credential issuance, or token signing;
- Feishu identity or permission administration;
- outbound message, email, webhook, or arbitrary third-party integration delivery;
- built-in LLM execution or Agent dispatch;
- a generic task label or Context-title scheduling field in V2;
- unrestricted cross-Domain workflow content access.

Shared PostgreSQL infrastructure does not change ownership. Workflow-owned tables or schemas may be written only through accepted `svc-workflow` command boundaries; another product's ability to connect to the same cluster does not authorize direct mutation.

## 18. Technology and architecture layers

The product technology direction remains:

```text
Rust
Axum
PostgreSQL
sqlx
tokio
tower-http
```

The architectural separation remains:

| Layer | Product responsibility |
|---|---|
| `domain` | Strongly typed identifiers, workflow entities, enums, commands, events, permission concepts, and domain errors without HTTP ownership. |
| `application` | Use-case orchestration and authorization decisions over domain/storage ports without depending on Axum request mechanics. |
| `http` | Axum routes, authentication adaptation, strict DTOs, response/error mapping, and wire-boundary validation. |
| `store` | PostgreSQL repositories, atomic transactions, concurrency control, durable facts/projections/audit, and migration-owned persistence. |
| `auth` | Auth V1 RS256/JWKS offline verification, accepted credential adaptation, and canary write gating; never token issuance. |

The global control-plane exception must preserve these layers and must not be implemented as UI-only filtering, handler-only redaction after broad reads, or an external adapter bypassing application/storage authorization.

## 19. Known V2 product trade-offs

The following product-level trade-offs remain explicit:

- there is no normal physical DELETE API for Workflow Instances; cancel/archive preserve history;
- the V2 baseline permits a single PostgreSQL instance and does not require read/write separation; scale-topology changes require later accepted authority;
- offline JWKS verification can accept a revoked token during a bounded cache-invalidation window governed by the accepted auth-service contract;
- V2 intentionally withholds full content from global schedulers;
- V2 intentionally has no break-glass path;
- V2 intentionally excludes task label and forbids reclassifying Context title as metadata.

These trade-offs are not permission to weaken fail-closed authorization, audit, or Domain isolation.

## 20. Downstream authority boundary

```text
PRODUCT_DIRECTION_AUTHORIZES_IMPLEMENTATION_DIRECTLY = NO
PRODUCT_IMPLEMENTATION_AUTHORIZED = NO
CHILD_IMPLEMENTATION_SPEC_REQUIRED = YES
```

Before implementation of any capability Slice begins, at least one relevant accepted governing child Spec with:

```text
spec_kind: implementation
implementation_authority: contracts
```

must be present in that Slice's implementation base. Across the Slices, accepted child authorities must collectively freeze the applicable items below. A Slice child freezes only the items it consumes or changes and is not blocked by an unrelated item or independent authority:

- the exact relationship to frozen/effective Architecture and each legacy Contract relevant to that Slice, with explicit reconciliation wherever narrower or conflicting meaning cannot remain active;
- permission storage and expiry model;
- two-person grant/renewal and one-actor revoke APIs;
- auth-service/OBO contract and exact canonical actor rules;
- Feishu command Adapter and event-id idempotency;
- scheduler query, complete response contract, filtering, pagination, revocation barrier, and denial semantics;
- Domain creation and Owner replacement transactions;
- durable audit writes, unauthenticated-denial reuse, expiry reconciliation, retention, sensitive-content exclusion, and fail-closed behavior;
- legacy holder mapping and migration;
- rollout, containment, rollback, and compatibility removal;
- exact HTTP and SDK behavior;
- executable Acceptance covering positive, negative, race, expiry, revoke, unknown-outcome, and sensitive-content paths.

### 20.1 Legacy and external authority classification

The following relationships are capability-scoped. Legacy documents without frontmatter retain only their established status; this table cites them and does not silently promote descriptive current state into normative predecessor authority.

| PATH / REPOSITORY | AUTHORITY_ID | REVISION / STATUS | CLASS | BLOCKS | DOES_NOT_BLOCK | REQUIRED_RECONCILIATION |
|---|---|---|---|---|---|---|
| `docs/contracts/IDENTITY_PROVISIONING_API_V0.md` / `mayf3/svc-workflow` | `IDENTITY_PROVISIONING_API_V0` | `c7830e58578d7c7360710f2449c48cb801da773e`; `FROZEN_FOR_PROVISIONING_READY` | `NORMATIVE_PREDECESSOR` | Slice A global permission lifecycle and Slice C global Domain admin | direct scheduler query design that does not activate permissions; Assistance; Admin Recovery | owning-repository whole-authority successor reconciles conflicting provisioning, role, Owner, and governance semantics |
| `contracts/workflow-http/v1/contract.md` / `mayf3/svc-workflow` | `WORKFLOW_RUNTIME_HTTP_CONTRACT_V1` | `c7830e58578d7c7360710f2449c48cb801da773e`; current-state freeze | `DESCRIPTIVE_CURRENT_STATE` | no global implementation gate by itself; the relevant Slice must establish accepted normative HTTP/wire authority before activation | unrelated Slices | accepted capability child Spec establishes the new normative HTTP Contract; the descriptive bundle is updated with that Slice and passes compliance |
| `docs/contracts/minimal-auth-v1/` / `mayf3/auth-service` | `MINIMAL_AUTH_FOUNDATION_V1` | `450a0ecb286cbe5da6e790d3c572fa71218ca9c0`; `FROZEN_TARGET_CONTRACT`; Human OBO forbidden | `CAPABILITY_SPECIFIC_EXTERNAL_DEPENDENCY` | Slice D and Slice E | Slice A, Slice B, Slice C | accepted auth-service Human OBO whole-authority successor plus accepted Feishu Adapter/OBO authority |
| `docs/contracts/WORKFLOW_AGENT_OBO_TOKEN_EXCHANGE_V0.md` / `mayf3/auth-service` | `AUTH_SERVICE_WORKFLOW_AGENT_OBO_V0_FROZEN` | `450a0ecb286cbe5da6e790d3c572fa71218ca9c0`; Agent-only | `CAPABILITY_SPECIFIC_EXTERNAL_DEPENDENCY` | Slice D and Slice E because it cannot authorize Human OBO | Slice A, Slice B, Slice C | same Human OBO successor; Agent-only authority MUST NOT be reinterpreted |
| Assistance current-state sections in `contracts/workflow-http/v1/contract.md`; uncommitted proposed draft `docs/specs/SVC_WORKFLOW_ASSISTANCE_V1.md` on `agent/workflow-assistance-v1-spec` | current Assistance surface; proposed `SVC_WORKFLOW_ASSISTANCE_V1` draft | current surface at `c7830e58578d7c7360710f2449c48cb801da773e`; draft not in authority branch | `INDEPENDENT_AUTHORITY` | only future Assistance-content or Assistance-derived scheduling metadata work | Slice A, Slice B, Slice C, and Slice D/E when no Assistance data is returned | accepted Assistance authority only when that independent capability changes |
| `docs/contracts/ADMIN_RECOVERY_CONTRACT_V0_1.md` / `mayf3/svc-workflow` | `ADMIN_RECOVERY_CONTRACT_V0_1` | `c7830e58578d7c7360710f2449c48cb801da773e`; `CURRENT` | `INDEPENDENT_AUTHORITY` | only implementation that changes Recovery event/rebuild semantics | Slice A, Slice B, Slice C, Slice D, Slice E | planned `SVC_WORKFLOW_ADMIN_RECOVERY_V1` only for Recovery-changing work |

V2 does not partially supersede any entry through prose. If V2 is later accepted, parent-conflicting legacy semantics cannot authorize a new capability; however, reconciliation is required only for the capability Slice that consumes or changes that authority. Assistance and Recovery remain independent and are not absorbed into V2 or its global-permission implementation.

### 20.2 Capability-scoped predecessor gates

No all-successors global gate exists. Each Slice may proceed only after V2 itself is accepted and the Slice-specific authorities below are accepted in its implementation base.

#### Slice A — Global permission governance foundation

Scope: split permission storage, TTL, grant, approval, renewal, revoke, expiry, root/approver evaluation, and durable audit.

Required:

- accepted `SVC_WORKFLOW_PRODUCT_BOUNDARY_V2`;
- accepted `SVC_WORKFLOW_GLOBAL_PERMISSION_GOVERNANCE_ROOT_V1`;
- `IDENTITY_PROVISIONING_API_V0` relationship reconciled by its owning whole-authority successor;
- relevant implementation-authorizing child Spec accepted.

Not required: auth-service Human OBO successor, Workflow Assistance successor, or Admin Recovery successor.

#### Slice B — Direct-token `GLOBAL_SCHEDULER_READ`

Required: Slice A foundation complete, scheduler-read child implementation Spec accepted, and relevant runtime HTTP/wire authority established and reconciled. This Slice may return only V2 active-current-task metadata. Human OBO, Assistance, and Admin Recovery successors are not prerequisites.

#### Slice C — Direct-token `GLOBAL_DOMAIN_ADMIN`

Required: Slice A foundation complete, accepted whole-authority successor to `IDENTITY_PROVISIONING_API_V0`, Domain-admin child implementation Spec accepted, and relevant HTTP/control-plane authority reconciled. Human OBO, Assistance, and Admin Recovery successors are not prerequisites.

#### Slice D — Feishu Human `GLOBAL_SCHEDULER_READ`

Required: Slice B direct scheduler available, accepted auth-service Human OBO whole-authority successor, and accepted Feishu Adapter/OBO authority. Missing Human OBO blocks only this Feishu Slice; it does not block direct-token scheduler implementation.

#### Slice E — Feishu Human `GLOBAL_DOMAIN_ADMIN`

Required: Slice C direct Domain-admin available, accepted auth-service Human OBO whole-authority successor, and accepted Feishu Adapter/OBO authority.

#### Slice F — Assistance-derived scheduling metadata

```text
NOT_AUTHORIZED_BY_V2
```

V2 excludes Assistance and blocking fields. Adding them requires a new accepted Product Direction or lawful whole-authority successor. Current Assistance authority is not a prerequisite for Slice A, B, or C.

#### Slice G — Admin Recovery

Admin Recovery is independently governed. `ADMIN_RECOVERY_CONTRACT_V0_1` and planned `SVC_WORKFLOW_ADMIN_RECOVERY_V1` are not common prerequisites for any global scheduler or Domain-admin Slice. Only an implementation that changes Recovery event/rebuild semantics is blocked on its Recovery successor.

Current Rust, SQL, migrations, HTTP Contract bundle, OpenAPI, SDK, tests, and deployments are descriptive current state only. Acceptance of V2 would not automatically make them compliant or authorize retrospective implementation. Conformance requires Contract-by-Contract evaluation against an exact accepted child-Spec revision and exact implementation coordinates.

## 21. Whole-authority supersession and future acceptance transition

This proposal is classified `SUPERSEDE`, not `AMEND`, because V2 changes Product Direction by introducing two explicit bounded global control-plane permissions while preserving strict normal-data-plane Domain isolation. V2 fully replaces the scope and meaning of V1; it is not a prose-only exception attached to V1.

The closed Draft PR #3 Option A proposal to remove all global Coordinator capabilities was rejected and MUST NOT be continued or treated as authority. Its lower-level implementation approach cannot override the frozen V2 Owner direction.

While V2 is proposed:

- only this new authority is created;
- V1 remains unchanged and active;
- `.agents/local/README.md` continues to point to V1;
- no implementation may cite V2 as authority;
- no partial supersession is inferred.

A future acceptance-only, docs-only transition must prepare one atomic candidate that completes all content changes below before final-head review:

1. change V2 frontmatter `status: proposed` to `status: accepted`;
2. update every V2 body status field and every prose statement that says V2 is proposed, inactive, or pending so the complete document consistently describes the accepted candidate;
3. set `INDEPENDENT_REVIEW_PENDING = NO` and `AUTHORIZED_ACCEPTANCE_PENDING = NO` only after their corresponding acts are complete;
4. prepend complete metadata to the existing V1 path `PRODUCT-BOUNDARY.md`: `authority_id: SVC_WORKFLOW_PRODUCT_BOUNDARY_V1`, `status: superseded`, `authority_kind: product_direction`, `owning_repository: mayf3/svc-workflow`, `supersedes: []`, `superseded_by: SVC_WORKFLOW_PRODUCT_BOUNDARY_V2`, and `owners: [mayf3]`;
5. preserve the V1 historical body byte-for-byte from the fixed separator: after the prepended metadata's closing `---` and one blank line, the historical body begins with `# svc-workflow 产品边界定义` and continues unchanged to EOF;
6. verify the preserved V1 historical-body SHA-256 is exactly `ab4fb261f5fe1f7eef0dd710b60ec088a3cb24747c8070c32ab5e30e8f1b70c2`;
7. update `.agents/local/README.md` so active Product Direction points to V2 and V1 is listed only as superseded history, while retaining V2 `supersedes: [SVC_WORKFLOW_PRODUCT_BOUNDARY_V1]` as the forward link;
8. if `docs/product/README.md` exists or is introduced as an authority index, update it in the same candidate to point to V2 as active and V1 as superseded history;
9. perform an independent final-head recheck of the exact complete acceptance candidate, including metadata/prose consistency, backlinks, authority map, optional index, fixed separator, and historical-body digest;
10. merge that exact accepted candidate to `main`, after which—and only after which—V2 becomes active Product Direction and V1 becomes superseded history.

The `supersedes` value in this proposed V2 is a proposed future relationship only; it has no effect while `status: proposed` or while the acceptance candidate is unmerged. No one-sided metadata update, acceptance in an unmerged branch, PR approval alone, lower-level Spec, code merge, or runtime deployment performs this supersession. This amendment does not execute any acceptance step.

## 22. Decision and readiness summary

```text
NORMAL_DATA_PLANE_DOMAIN_ISOLATION = STRICT
GLOBAL_CONTROL_PLANE_EXCEPTION = AUTHORIZED_AND_BOUNDED
GLOBAL_PERMISSIONS = GLOBAL_SCHEDULER_READ, GLOBAL_DOMAIN_ADMIN
PERMISSION_MODEL = SPLIT
SAME_PRINCIPAL_MAY_HOLD_BOTH = YES
GLOBAL_WORKFLOW_COORDINATOR = UI_LABEL_ONLY
FULL_CONTENT_ACCESS_REQUIRED = NO
SCHEDULING_VIEW_SCOPE = ACTIVE_CURRENT_TASK_METADATA_ONLY
SCHEDULER_UPDATED_AT = LATEST_COMMITTED_WORKFLOW_INSTANCE_STATE_CHANGE_TIMESTAMP
REMOVE_FROM_V2_SCHEDULER_METADATA = blockedFlag, blockedReasonCode, waitingAssistance, assistanceStatus
TASK_LABEL = NOT_INCLUDED_IN_V2
CONTEXT_TITLE_AS_METADATA = FORBIDDEN
ALTERNATE_IDENTITY_RULE = NO_IMPLICIT_LINKAGE
SELF_DOMAIN_OWNER_RULE = CANONICAL_PRINCIPAL_EQUALITY
NO_ACCEPTED_LINKAGE_AUTHORITY = TREAT_DISTINCT_CANONICAL_PRINCIPALS_AS_DISTINCT
IMPLICIT_COMMON_CONTROL_INFERENCE = FORBIDDEN
FAIL_CLOSED_WHEN_LINKAGE_IS_ABSENT = NO
GLOBAL_ADMIN_SELF_GRANT = FORBIDDEN
GLOBAL_ADMIN_SELF_DOMAIN_OWNER = FORBIDDEN
ROOT_AUTHORITY_ID = SVC_WORKFLOW_GLOBAL_PERMISSION_GOVERNANCE_ROOT_V1
ROOT_AUTHORITY_KIND = repository-owned security invariant
ROOT_AUTHORITY_OWNER = mayf3
REPOSITORY_OWNER_IS_RUNTIME_ACTOR = NO
NO_ACTIVE_ACCEPTED_ROOT_AUTHORITY = GRANT_RENEWAL_AND_ROSTER_CHANGE_DISABLED
SECURITY_APPROVER_ROSTER = canonical Principals explicitly recorded by active accepted root authority
GRANT_REQUESTER_COUNTS_AS_APPROVER = NO
GRANT_APPROVALS_REQUIRED = 2_DISTINCT_SECURITY_APPROVERS
GRANT_FINALIZATION = DETERMINISTIC_SYSTEM_FINALIZATION_AFTER_2_VALID_APPROVALS
REVOKE_AUTHORITY = active canonical root Principal or 1 active security approver
SUBJECT_SELF_APPROVAL = FORBIDDEN
SECURITY_APPROVER_SELF_GRANT = FORBIDDEN
FEISHU_HUMAN_OPERATOR = ALLOWED_VIA_OBO
FEISHU_IS_PERMISSION_SOURCE = NO
FEISHU_HUMAN_OBO = TARGET_PRODUCT_DIRECTION
AUTH_SERVICE_HUMAN_OBO_CURRENTLY_AVAILABLE = NO
FEISHU_HUMAN_IMPLEMENTATION_BLOCKED_UNTIL_AUTH_SERVICE_AUTHORITY = YES
DIRECT_TOKEN_SLICES_BLOCKED_BY_HUMAN_OBO = NO
NORMAL_GRANT_DEFAULT_TTL = 30_DAYS
NORMAL_GRANT_MAX_TTL = 90_DAYS
GRANT_REQUIRES_TWO_PERSON_APPROVAL = YES
REVOKE_MAY_BE_PERFORMED_BY_ONE_AUTHORIZED_SECURITY_ACTOR = YES
BREAK_GLASS = NOT_SUPPORTED_IN_V2
AUDIT_RETENTION = 365_DAYS
AUDIT_PRODUCT_READ_API = NOT_SUPPORTED_IN_V2
EXTERNAL_AUDIT_EXPORT = NOT_SUPPORTED_IN_V2
UNAUTHENTICATED_DENIAL_AUDIT = REUSE_EXISTING_AUTHENTICATION_AUTHORITY
PERMISSION_EXPIRY = FAIL_CLOSED_AT_EXPIRES_AT
EXPIRY_AUDIT = RECONCILE_LATER_WITHOUT_EXTENDING_AUTHORITY
LEGACY_AUTHORITY_STRATEGY = CAPABILITY_SCOPED_PREDECESSOR_GATES
ALL_SUCCESSORS_GLOBAL_GATE = FORBIDDEN
FAILURE_POLICY = FAIL_CLOSED

OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
INDEPENDENT_REVIEW_PENDING = YES
AUTHORIZED_ACCEPTANCE_PENDING = YES
AUTHORING_READY_FOR_REVIEW = YES
```
