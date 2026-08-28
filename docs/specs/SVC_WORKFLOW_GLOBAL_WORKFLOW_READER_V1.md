---
spec_id: SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1
title: Global Workflow Reader Role (Dual Grant: HR Main + Dedicated Dispatcher) V1
status: accepted
accepted_date: 2026-08-28
accepted_reviewed_head: f900586fe198b3a1e1a069fe8ccc3690a481612a
repo: mayf3/svc-workflow
base_head: 2ff81ae47ab068216bd0012fa0e76a45dd2fb572
date: 2026-08-27
revision_of_drafts:
  - SVC_WORKFLOW_HR_GLOBAL_COORDINATOR_ASSIGNMENT_V1@cf45d7c6 (never merged; withdrawn — coordinator to HR main)
  - SVC_WORKFLOW_HR_GLOBAL_WORKFLOW_READER_ASSIGNMENT_V1@e2e3464 (never merged; withdrawn — single-grantee reader model superseded)
  - SVC_WORKFLOW_HR_DISPATCHER_IDENTITY_V1@d1298b9 (never merged; withdrawn — identity governance split to auth-service)
  - SVC_WORKFLOW_HR_DISPATCHER_COORDINATOR_GRANT_V1@5efcd81 (never merged; withdrawn — coordinator-to-dispatcher conflicts with final product goal)
owner_ruling: DUAL_GLOBAL_READER_MODEL (FINAL — no further identity-model switches)
product_code_changed_by_this_spec_pr: false
server_change_authorized_upon_acceptance: true (controlled closure, section 5; NO database migration)
implementation_authority: contracts
implementation_authority_activation: accepted_on_main
merge_required_for_activation: true
role_applied_by_this_pr: false
production_apply_authorized: false
companion_specs:
  # Sole upstream authority (dependency DAG, frozen: auth-service PR #31 ->
  # THIS Spec -> dsh-agent-core PR #83 -> dsh-agent-core PR #87). This Spec
  # depends ONLY on the auth-service identity Spec below and pins ONLY its
  # final head; dsh-agent-core Specs are DOWNSTREAM (they may pin this
  # Spec's head; this Spec never pins theirs).
  - mayf3/auth-service AUTH_SERVICE_AGENTCORE_HR_DISPATCHER_IDENTITY_V1
    (accepted, PR #31 merged @ 51a11af57ce39eafac5883e0c32474ea06906b8e —
    dispatcher Principal/Client/grants; sole upstream authority)
---

# SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1

> **Revision note (2026-08-28, round 3 — upstream acceptance repin,
> metadata-only):** the sole normative upstream
> `AUTH_SERVICE_AGENTCORE_HR_DISPATCHER_IDENTITY_V1` completed its
> acceptance transaction and merged to auth-service main via PR #31
> (merge commit `51a11af57ce39eafac5883e0c32474ea06906b8e`; upstream
> frontmatter: status accepted, implementation_authority contracts,
> production_apply_authority none). Both normative pins in THIS Spec
> (frontmatter `companion_specs` and §9 `UPSTREAM_HEAD_PINS`) are repinned
> from the superseded proposed-branch head
> `5b4a3ed7e28c631280d3e6894437e7e8569958ac` to that accepted main merge
> commit. The old head survives ONLY in this historical note and is no
> longer a normative dependency. No ruling, role definition, grantee
> freeze, or §4/§5 contract changes in this round.

## 1. Decision summary

OWNER_RULING = `DUAL_GLOBAL_READER_MODEL` (final). The product needs BOTH:
(1) the HR main session may manually view workflow instances across ALL
domains read-only; (2) a dedicated Dispatcher Agent may background-scan
read-only and auto-dispatch. Neither needs — or may get — the
`GLOBAL_WORKFLOW_COORDINATOR` role, because that role also gates
domain-management write endpoints. This Spec therefore introduces ONE new
read-only server role and freezes TWO exact grantees:

```text
NEW_ROLE_KEY            = GLOBAL_WORKFLOW_READER
ROLE_ALLOWED_SURFACE    = GET /internal/v1/workflow-instances/global ONLY
SERVER_GATE (global list) = GLOBAL_WORKFLOW_READER OR GLOBAL_WORKFLOW_COORDINATOR
WRITE_ENDPOINTS (unchanged, COORDINATOR-only):
  POST /internal/v1/domains ; PUT /internal/v1/domains/{domainId}/owner

GRANTEE 1 (exact, frozen now):
  AGENT_ID     = agt_hr-agent
  PRINCIPAL_ID = dc702687-6515-4a2a-91ae-e572a9bbd766
  ROLE         = GLOBAL_WORKFLOW_READER
GRANTEE 2 (exact, UUID pending):
  AGENT_ID     = agt_workflow-dispatcher-hr-agent
  PRINCIPAL_ID = <PENDING_AUTH_IDENTITY> (amendment backfill REQUIRED;
                 production role apply FORBIDDEN before backfill)
  ROLE         = GLOBAL_WORKFLOW_READER

HR_GLOBAL_READER            = YES (planned; apply separately authorized)
DISPATCHER_GLOBAL_READER_PLAN = YES (planned; UUID backfill gate)
HR_GLOBAL_COORDINATOR       = NO  (frozen)
DISPATCHER_GLOBAL_COORDINATOR = NO (frozen)
LEGACY_HR (bc970ced-710f-4479-9ff0-e295a1c59424) = NO role of any kind (frozen)
ROLE_APPLIED_THIS_ROUND     = NO
```

This PR is docs-only (one spec file). Acceptance authorizes the §5 code
change for implementation review; role apply remains a separately
owner-authorized production step (§7).

Dependency direction (frozen, 2026-08-27 DAG sync): this Spec's ONLY
normative dependency is the auth-service identity Spec
(`AUTH_SERVICE_AGENTCORE_HR_DISPATCHER_IDENTITY_V1`, PR #31 — the
dispatcher Principal/Client/grants whose creation §6 grantee 2 waits for).
The dsh-agent-core broker-capability and dispatcher Specs are DOWNSTREAM of
this Spec: they depend on the final server-side GLOBAL_WORKFLOW_READER
contract frozen here; this Spec takes no authority from, and pins no exact
head of, any dsh-agent-core artifact (their responsibilities may be
described as out-of-scope context, never as dependencies).

## 2. Why coordinator is not grantable to either principal (carried evidence)

Production read-only evidence (auth-service DB, 2026-08-27):

| Identity | Principal | Credential reach |
|---|---|---|
| Fleet HR agent (`agentcore:v1:principal:agt_hr-agent`) | `dc702687-6515-4a2a-91ae-e572a9bbd766` | active client; machine_access_grants v2 = **{workflow.read, workflow.execute}** |
| Dedicated dispatcher (planned; identity does not exist yet) | `<PENDING>` | planned grant set = {workflow.read, agent-wake scope} (auth-service PR #31) |
| Legacy HR agent | `bc970ced-710f-4479-9ff0-e295a1c59424` | active client {workflow.admin, workflow.read, workflow.execute} |

`GLOBAL_WORKFLOW_COORDINATOR` gates the global read AND the two
domain-management write endpoints (each additionally needs
`workflow.execute` scope + canary guard — src/http/mod.rs:152-162). The HR
main identity can mint `workflow.execute` tokens today, so the coordinator
role is not grantable to it; the dedicated dispatcher is deliberately kept
minimal and needs read-only only. A dedicated read-only role is the single
mechanism that serves both grantees without any write reach.

## 3. Role definition — GLOBAL_WORKFLOW_READER

Permission matrix (server-side enforcement; frozen):

| Surface | GLOBAL_WORKFLOW_READER | GLOBAL_WORKFLOW_COORDINATOR |
|---|---|---|
| `GET /internal/v1/workflow-instances/global` (scope `workflow.read`) | ALLOW | ALLOW (unchanged) |
| `POST /internal/v1/domains` | DENY | ALLOW (unchanged, + `workflow.execute` + canary guard) |
| `PUT /internal/v1/domains/{domainId}/owner` | DENY | ALLOW (unchanged, + `workflow.execute` + canary guard) |
| workflow transitions / assignment mutation | DENY (assignee-gated, unchanged) | DENY (unchanged) |
| Domain mutation (create/config/owner/membership) | DENY | as above |
| assistance mutation / assistance-case surfaces (`require_global_coordinator`, assistance_transaction/query.rs:272-287) | DENY (unchanged) | ALLOW (unchanged) |
| provisioning / admin operations | DENY (admin-gated, unchanged) | DENY (unchanged) |
| Scheduler mutation (any scheduler surface) | DENY (outside svc-workflow authority entirely) | DENY |

GLOBAL_WORKFLOW_READER grants no scope, no token right, no write power of
any kind; it is inert outside the global instance list.

## 4. Server-side change closure (authorized only upon acceptance; NO migration)

`global_role_bindings.role_key` is free text at the schema level
(migrations/0020: TEXT 1..128, unique (principal_id, role_key)); supported
values are validated at the API layer — **no new migration**. Controlled
code change, exact file closure (base 2ff81ae):

1. `src/domain/provisioning/mod.rs` — add
   `pub const GLOBAL_WORKFLOW_READER_ROLE: &str = "GLOBAL_WORKFLOW_READER";`
   (doc: read-only, gates ONLY the global instance list).
2. `src/store/postgres/workflow_instance_repository/query_visibility.rs`
   (:54-65, the global-list gate used ONLY by
   `WorkflowInstanceQueryService::list_global_instances`) — predicate
   becomes `role_key IN ('GLOBAL_WORKFLOW_READER','GLOBAL_WORKFLOW_COORDINATOR')
   AND enabled = TRUE`.
3. `src/application/workflow_instance/query_types.rs` +
   `query_service.rs:103-131` — the read-gate error variant message names
   both accepted roles.
4. `src/http/error.rs` `from_query` (:516-519) — global-list gate failure
   becomes HTTP 403 code **`global_read_role_required`** ("caller must hold
   GLOBAL_WORKFLOW_READER or GLOBAL_WORKFLOW_COORDINATOR").
   `from_provisioning` (:302-303) and the coordinator write endpoints keep
   `global_coordinator_required` unchanged.
5. `src/http/handlers/provisioning/global_role_bindings.rs` (:34-39 PUT,
   :76-79 DELETE) — accept `roleKey` ∈ {`GLOBAL_WORKFLOW_COORDINATOR`,
   `GLOBAL_WORKFLOW_READER`}; anything else stays 422 `role_key_invalid`.
   (Store-layer upsert/revoke do not whitelist role keys — no change.)
6. Tests (unit + conformance per repo convention) — obligations in §8.

Deliberately UNCHANGED: `handlers/coordinator_domains.rs` write gates
(exact COORDINATOR-only), the assistance `require_global_coordinator`
predicate, all transition/cancel/archive/domain write gates, all admin
gates, route table, migrations.

## 5. Wire contract (freeze, with deployment transition)

`GET /internal/v1/workflow-instances/global` non-holder failure:

```text
after §4 deploys:  403 {"code":"global_read_role_required", ...}
before §4 deploys: 403 {"code":"global_coordinator_required", ...}   (existing)
```

Both codes are legitimate deployment-transition realities; downstream
declarers on the dsh-agent-core side (governed by their own Specs, out of
scope here) are expected to declare BOTH. No other endpoint's error
contract changes.

## 6. Grant plan (future applies, separately authorized)

Grantee 1 — exact, ready after §4 deploys:

```text
PUT /internal/v1/admin/global-role-bindings/dc702687-6515-4a2a-91ae-e572a9bbd766
Headers: Idempotency-Key: <fresh uuid>, x-request-id: <uuid>
Body:    {"roleKey": "GLOBAL_WORKFLOW_READER", "enabled": true}
Auth:    admin provisioning credential
```

Grantee 2 — gated on identity creation + amendment backfill:

```text
PUT /internal/v1/admin/global-role-bindings/<dispatcher-principal-uuid>
Body:    {"roleKey": "GLOBAL_WORKFLOW_READER", "enabled": true}
PRECONDITIONS: auth-service identity exists (PR #31 plan executed);
  the created UUID backfilled into THIS Spec by amendment — apply against
  <PENDING_AUTH_IDENTITY> is INVALID and must be refused by the operator.
```

Pre-apply checklist (recorded per grant): (a) §4 change deployed; (b)
principal exists/active and maps to the exact agent_id; (c) no enabled
READER binding already present (idempotent exact-rerun NOOP acceptable);
(d) production `global_role_bindings` enumerated by an authorized reader as
baseline (honest limit: `auth_ro` is denied on that table today); (e) legacy
HR `bc970ced-…` and every other principal unchanged; (f) for grantee 2,
dispatcher grant set verified = {workflow.read (+ agent-wake scope)} only.

Rollback per grant: `DELETE /internal/v1/admin/global-role-bindings/<uuid>`
with `{"roleKey":"GLOBAL_WORKFLOW_READER"}` (enabled=false, idempotent).
Code rollback = revert §4 (READER rows are inert without the gate).

## 7. Acceptance criteria

- AC-1: with an enabled READER binding, a `workflow.read`-scoped token
  calling the global list returns 200 `Page<DomainInstanceSummary>`
  (summaries only) — verified for both a READER-only HR-main-shaped caller
  and a READER-only dispatcher-shaped caller.
- AC-2: with an enabled COORDINATOR binding (any test principal), the same
  call still returns 200 (no regression).
- AC-3: with neither role, the call returns 403 `global_read_role_required`
  (post-§4) / `global_coordinator_required` (pre-§4).
- AC-4: with ONLY a READER binding, `POST /internal/v1/domains` and
  `PUT /internal/v1/domains/{domainId}/owner` still return 403
  `global_coordinator_required` (write gates unchanged).
- AC-5: assistance surfaces with only READER still fail
  `global_coordinator_required` (unchanged).
- AC-6: PUT/DELETE admin global-role-bindings accept
  `roleKey: GLOBAL_WORKFLOW_READER`; other keys still 422 `role_key_invalid`.
- AC-7: after grantee-1 apply, the HR main principal's ONLY new artifact is
  the single READER binding (no scope/credential/other-role change); after
  (future) grantee-2 apply, same for the dispatcher principal.
- AC-8: legacy `bc970ced-…` and all other principals: before/after diff
  empty. Repo test suite passes.

## 8. Alternatives and disposition

- Coordinator to HR main (draft 1) / coordinator to dispatcher only (draft
  4) — **rejected**: role conflates read with domain-management write gates;
  final model grants coordinator to NOBODY in this family
  (HR_GLOBAL_COORDINATOR = NO, DISPATCHER_GLOBAL_COORDINATOR = NO).
- Single-grantee reader (draft 2: reader for HR only) — **superseded by the
  final dual product goal**: manual HR viewing AND background dispatcher
  scanning both required; one read-only role, two exact grantees.
- Revoke HR `workflow.execute` then grant coordinator — rejected: couples
  read-only intent to fleet credential surgery, out of authority.
- No-code path (no new role; rely on scope separation alone with
  coordinator) — rejected: coordinator would still nominally gate write
  endpoints for a dispatcher-holding credential; a read-only role states
  the actual grant precisely and keeps both grantees' blast radius at zero.

## 9. What this PR changes

```text
DOCS ONLY — removes the withdrawn same-PR draft
(SVC_WORKFLOW_HR_DISPATCHER_COORDINATOR_GRANT_V1, 5efcd81, never merged)
and adds exactly this file.
SVC_WORKFLOW_CODE_CHANGE (this PR) = NONE
ROLE_CHANGE                            = NONE (plan only)
PRODUCTION_CHANGE                      = NONE
DEPENDENCY_POSITION = node 3 of WAKE -> 31 -> 14 -> 83 -> 87
UPSTREAM_HEAD_PINS   = auth-service PR #31 only
                      (51a11af57ce39eafac5883e0c32474ea06906b8e)
DOWNSTREAM_PINS      = NONE (no normative dependency on, and no exact-head
                      pin of, any dsh-agent-core Spec)
CIRCULAR_AUTHORITY_PIN_COUNT (this Spec) = 0
READY_FOR_INDEPENDENT_REVIEW = YES
STATUS = accepted
INDEPENDENT_REVIEW_RESULT = PASS
REQUIRED_FIXES = NONE
SEMANTIC_DELTA_AFTER_REVIEW = LIFECYCLE_ONLY
IMPLEMENTATION_PERFORMED = NO
MERGE_REQUIRED_FOR_ACTIVATION = YES
```

Independent review required before acceptance; acceptance authorizes the
§4 implementation-review path and nothing else — role applies remain
separately owner-authorized per grant (§6).

## 10. Acceptance Record

```text
ACCEPTANCE_TRANSACTION = 2026-08-28 (TASK_NAME = 全查 执行,
  TASK_TYPE = ACCEPTANCE_AND_IMPLEMENTATION, Part A)
ACCEPTED_SPEC = SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1
ACCEPTED_REVIEWED_HEAD = f900586fe198b3a1e1a069fe8ccc3690a481612a
INDEPENDENT_AUDIT (全查 审计) = PASS
  HEAD_DRIFT = NONE
  UPSTREAM_ACCEPTED_PIN = EXACT
  READ_ONLY_ROLE_BOUNDARY = HOLDS
  WRITE_GATE_PRESERVATION = HOLDS
  EXACT_GRANTEE_MODEL = HOLDS
  IMPLEMENTATION_CLOSURE = COMPLETE
  DATABASE_MIGRATION_REQUIRED = NO
  BLOCKERS = NONE
  REQUIRED_FIXES = NONE
  READY_FOR_ACCEPTANCE_FINALIZE = YES
UPSTREAM_CHAIN_ACCEPTED =
  AUTH_SERVICE_AGENT_WAKE_AUDIENCE_CCR_V1 = eb1a1c15488b75c4a1828902f5c65a38178a88ce
  AUTH_SERVICE_AGENTCORE_HR_DISPATCHER_IDENTITY_V1 = 51a11af57ce39eafac5883e0c32474ea06906b8e
TRANSACTION_SEMANTICS = LIFECYCLE_ONLY — this record plus the status /
  activation metadata fields above; every §1-§9 ruling, the §3 permission
  matrix, the §6 grantees, and the §4 exact file closure are
  byte-preserved. Pre-acceptance conditional sentences are preserved
  verbatim as historical records.
ACTIVATION_SEMANTICS = merge of this exact accepted head to main activates
  ONLY the §4 implementation contracts (implementation-review path);
  production role apply remains separately owner-authorized per grant (§6),
  and grantee-2 apply stays additionally gated on identity creation + the
  Principal-UUID backfill amendment + independent apply authority.
```
