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

All listed identity, Domain, Definition, current-node, lifecycle/status, assignee, count, and timestamp fields are required. `blockedReasonCode` is nullable only when `blockedFlag=false`; `assistanceStatus` is nullable only when `waitingAssistance=false`. Node type, lifecycle, status, blocked-reason, and Assistance status values MUST come from closed, non-sensitive code sets frozen by the child implementation Contract; free-form text is forbidden.

The scheduling view MUST include only active, current-task records. Archived, cancelled, terminal-without-current-task, historical Node Visit, and non-current task records MUST NOT be returned under this permission under any filter, mode, grouping, or pagination option. A child implementation Spec may define pagination and stable filtering only within this active-current-task population; it may not add fields or lifecycle scope beyond this Product Direction.

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
blockedFlag
blockedReasonCode
waitingAssistance
assistanceStatus
```

These fields are scheduling metadata. Their authorization does not make any similarly named field in another payload globally readable. `blockedReasonCode` is a bounded code, not free-form reason text.

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

These prohibitions apply directly and through aliases, chained calls, role combinations, retries, migrations, compatibility routes, or another identity controlled by the same actor.

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

Two-person approval means two distinct canonical authorized approval actors. Neither approver may be the grant subject, and the subject MUST NOT approve, finalize, or otherwise cause its own grant; requesting consideration alone grants no authority. An authorized sponsor MAY request a grant and MAY count as one of the two approvers only if independently eligible and not the subject; a separate finalization step MUST NOT fabricate another approval. The child implementation Spec must freeze the external/local authority source and exact eligibility for sponsors, approvers, finalizers, and security revokers without weakening these constraints.

One eligible authorized security actor may revoke without a second approver so containment is not delayed. Grant, renewal, denial, expiry, and revocation authority must be evaluated from current canonical authorization state, not Feishu configuration, request content, legacy role name, or the permission being granted.

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
```

The identity boundary is:

- the canonical Human Principal is the actor whose global permission is evaluated;
- the Feishu Adapter acts as the service/client identity and uses the accepted auth-service on-behalf-of contract;
- the verified `token.sub` is the actual human operator;
- the adapter/service identity, `act`, or equivalent delegation claim identifies the intermediary only as defined by the auth contract and does not replace `token.sub` as actor;
- message text, mention identity, request-body `principalId`, Feishu user identifier, or chat-room membership MUST NOT impersonate or substitute for the authenticated actor;
- the Feishu event ID is used for idempotency and durable audit correlation;
- the bot or Adapter MUST NOT self-grant, approve its own grant, or derive permission from Feishu configuration.

Direct authenticated channels and Feishu-via-OBO must converge on the same canonical permission and audit decisions. Feishu unavailability does not weaken authorization or create a fallback identity path.

## 13. Audit, retention, and failure policy

Every successful and denied attempt for a protected global-control-plane operation requires a durable audit record, including:

- permission request, approval, finalization, renewal, revoke, and expiry processing;
- Domain creation;
- Domain Owner replacement;
- global scheduling reads;
- global Domain and Principal directory reads;
- global-control-plane audit reads and external audit exports.

Expiry itself MUST produce a durable lifecycle audit record even when no request occurs at the expiry instant.

Audit records must identify the canonical actor, subject/target, permission or operation, decision/result, time, idempotency/correlation identity where applicable, and non-sensitive reason codes needed for accountability. They MUST NOT copy sensitive workflow content, Context payloads, Submissions, Assistance content, credentials, tokens, Receipt bodies, unrestricted request bodies, or unrestricted response payloads.

```text
AUDIT_RETENTION = 365_DAYS
FAILURE_POLICY = FAIL_CLOSED
```

The normal retention period is exactly 365 days. Earlier deletion is forbidden; longer retention requires separate accepted legal/security authority and must remain access-controlled and minimized.

Only the repository/product Owner and explicitly authorized security auditors may read these global-control-plane audit records. External audit export must be redacted and minimized before release, and the read/export operation must itself be audited.

If authorization state or required durable audit is unavailable, the protected operation MUST fail closed. A successful read must durably record its audit before any protected data is released. A successful write and its required audit must commit atomically. Authorization is not complete merely because a permission check once succeeded: if revocation or expiry takes effect before the authorization publication/commit barrier, an older in-flight request MUST neither release protected response data nor commit a protected write.

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

The preserved V1 machine/service boundary is RS256 verification using auth-service JWKS, with audience `aud=svc-workflow` and a canonical Machine Principal UUID in `token.sub`; verification is offline at request authorization time subject to the bounded JWKS cache trade-off in §19. Cross-service delegation uses auth-service OBO token exchange. V2 adds the explicit Human-via-OBO path in §12: for that path the verified `token.sub` is the canonical Human Principal, while the Adapter/service identity remains the intermediary identity.

A child implementation Spec must pin these expectations to exact accepted auth-service authority and freeze token-use, actor/delegation claims, key validation, cache/revocation behavior, and service-to-service wire contracts without weakening the preserved V1 boundary or the V2 Human-via-OBO extension. This Product Direction states what `svc-workflow` requires but does not redefine or supersede auth-service behavior.

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

Before implementation begins, at least one accepted governing child Spec with:

```text
spec_kind: implementation
implementation_authority: contracts
```

must be present in the implementation base and must freeze, at minimum:

- the exact relationship to frozen/effective Architecture and each relevant legacy Contract, with explicit reconciliation wherever their narrower or conflicting meaning cannot remain active;
- permission storage and expiry model;
- two-person grant/renewal and one-actor revoke APIs;
- auth-service/OBO contract and exact canonical actor rules;
- Feishu command Adapter and event-id idempotency;
- scheduler query, complete response contract, filtering, pagination, revocation barrier, and denial semantics;
- Domain creation and Owner replacement transactions;
- audit records, readers, retention, redacted export, and failure behavior;
- legacy holder mapping and migration;
- rollout, containment, rollback, and compatibility removal;
- exact HTTP and SDK behavior;
- executable Acceptance covering positive, negative, race, expiry, revoke, unknown-outcome, and sensitive-content paths.

Assistance lifecycle/content and Admin Recovery event/rebuild details remain in their own governing authorities. The planned `SVC_WORKFLOW_ADMIN_RECOVERY_V1` must separately resolve its relationship to `ADMIN_RECOVERY_CONTRACT_V0_1`; neither this Product Direction nor a global-permission child Spec may partially supersede Recovery semantics in prose. A child implementation Spec may coordinate with Assistance and Recovery but MUST NOT copy their lower-level details into this Product Direction or silently supersede them.

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

1. change V2 `status: proposed` to `status: accepted`;
2. prepend authority metadata to the existing V1 path `PRODUCT-BOUNDARY.md` with `authority_id: SVC_WORKFLOW_PRODUCT_BOUNDARY_V1`, `status: superseded`, and `superseded_by: SVC_WORKFLOW_PRODUCT_BOUNDARY_V2`, while preserving every byte of V1's historical body below that metadata and preserving the path;
3. update `.agents/local/README.md` so active Product Direction points to V2 and V1 is listed only as superseded history;
4. retain V2 `supersedes: [SVC_WORKFLOW_PRODUCT_BOUNDARY_V1]` as the forward link;
5. perform an independent final-head recheck of the exact complete acceptance candidate;
6. merge that accepted candidate to `main`, after which—and only after which—V2 becomes active Product Direction and V1 becomes superseded history.

The `supersedes` value in this proposed V2 is a proposed future relationship only; it has no effect while `status: proposed` or while the acceptance candidate is unmerged. No one-sided metadata update, acceptance in an unmerged branch, PR approval alone, lower-level Spec, code merge, or runtime deployment performs this supersession.

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
TASK_LABEL = NOT_INCLUDED_IN_V2
CONTEXT_TITLE_AS_METADATA = FORBIDDEN
GLOBAL_ADMIN_SELF_GRANT = FORBIDDEN
GLOBAL_ADMIN_SELF_DOMAIN_OWNER = FORBIDDEN
FEISHU_HUMAN_OPERATOR = ALLOWED_VIA_OBO
FEISHU_IS_PERMISSION_SOURCE = NO
NORMAL_GRANT_DEFAULT_TTL = 30_DAYS
NORMAL_GRANT_MAX_TTL = 90_DAYS
GRANT_REQUIRES_TWO_PERSON_APPROVAL = YES
REVOKE_MAY_BE_PERFORMED_BY_ONE_AUTHORIZED_SECURITY_ACTOR = YES
BREAK_GLASS = NOT_SUPPORTED_IN_V2
AUDIT_RETENTION = 365_DAYS
FAILURE_POLICY = FAIL_CLOSED

OPEN_OWNER_DECISIONS = NONE
NORMATIVE_TBD = NONE
UNRESOLVED_AUTHORITY_CONFLICT = NONE
PARTIAL_SUPERSESSION = NONE
INDEPENDENT_REVIEW_PENDING = YES
AUTHORIZED_ACCEPTANCE_PENDING = YES
AUTHORING_READY_FOR_REVIEW = YES
```
