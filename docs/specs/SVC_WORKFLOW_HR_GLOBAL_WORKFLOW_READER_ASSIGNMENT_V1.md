---
spec_id: SVC_WORKFLOW_HR_GLOBAL_WORKFLOW_READER_ASSIGNMENT_V1
title: Read-Only Global Workflow Reader Role and Exact HR Agent Assignment V1
status: proposed
repo: mayf3/svc-workflow
base_head: 2ff81ae47ab068216bd0012fa0e76a45dd2fb572
date: 2026-08-27
revision_of_draft: SVC_WORKFLOW_HR_GLOBAL_COORDINATOR_ASSIGNMENT_V1@cf45d7c6 (same-PR draft, never merged, withdrawn by OWNER_RULING)
owner_ruling: INTRODUCE_READ_ONLY_GLOBAL_ROLE
product_code_changed_by_this_spec_pr: false
server_change_authorized_upon_acceptance: true (exact closure in section 5)
role_change_authorized_now: false
production_apply_authorized: false
companion_broker_spec: mayf3/dsh-agent-core AGENT_CORE_WORKFLOW_GLOBAL_INSTANCES_CAPABILITY_V1 (proposed, PR #83)
---

# SVC_WORKFLOW_HR_GLOBAL_WORKFLOW_READER_ASSIGNMENT_V1

## 1. Decision summary

OWNER_RULING = `INTRODUCE_READ_ONLY_GLOBAL_ROLE`. The existing
`GLOBAL_WORKFLOW_COORDINATOR` role must NOT be granted to the HR agent's main
identity, because that role simultaneously gates the global instance read AND
two domain-management write endpoints while the HR agent already holds
`workflow.execute`-capable credentials. This Spec therefore proposes:

1. a NEW read-only role `GLOBAL_WORKFLOW_READER`, whose only permission is
   `GET /internal/v1/workflow-instances/global`;
2. a minimal server change (upon acceptance, exact closure in §5) so the
   global instance list accepts `GLOBAL_WORKFLOW_READER` OR
   `GLOBAL_WORKFLOW_COORDINATOR`, while every other gate keeps accepting
   `GLOBAL_WORKFLOW_COORDINATOR` only;
3. an exact-identity assignment of `GLOBAL_WORKFLOW_READER` to the HR agent
   (execution separately authorized, §7).

```text
NEW_ROLE_KEY            = GLOBAL_WORKFLOW_READER
AGENT_ID                = agt_hr-agent
PRINCIPAL_ID            = dc702687-6515-4a2a-91ae-e572a9bbd766   (frozen, full UUID)
ROLE_APPLIED_BY_THIS_PR = NO
PRODUCTION_APPLY_AUTHORIZED = NO
```

This PR is docs-only (one spec file). Acceptance authorizes the §5 server
change for implementation review; it still does not apply any role.

## 2. Security boundary finding (why the coordinator draft was withdrawn)

Production read-only evidence (auth-service DB, 2026-08-27):

| Identity | Principal | Credential reach |
|---|---|---|
| Fleet HR agent (`agentcore:v1:principal:agt_hr-agent`) | `dc702687-6515-4a2a-91ae-e572a9bbd766` | active client `mc_IuBMfCYe9-b522IhSWKBGjyz`; machine_access_grants v2 = **{workflow.read, workflow.execute}** (not revoked) |
| Legacy HR agent (`openclaw:agent:hr-agent`) | `bc970ced-710f-4479-9ff0-e295a1c59424` | active client `mc_4Ud_9wGR1mwQM9W7s7foX8qp`; allowed_scopes + grant = **{workflow.admin, workflow.read, workflow.execute}** |

`GLOBAL_WORKFLOW_COORDINATOR` gates, server-side, ALL of:

- `GET /internal/v1/workflow-instances/global` (read — the wanted effect);
- `POST /internal/v1/domains` (create Domain; also needs `workflow.execute`
  + canary guard — src/http/mod.rs:152-162, handlers/coordinator_domains.rs);
- `PUT /internal/v1/domains/{domainId}/owner` (replace Domain Owner; same
  additional gates).

Granting that role to a principal whose credential can mint
`workflow.execute` tokens would make the two write endpoints reachable in
principle. Both HR lineage principals have such credential paths today.
Conclusion: the coordinator role is not assignable to the HR main identity;
a role whose ONLY permission is the read is required. (The earlier same-PR
draft's claim "no workflow.execute reach-through exists at proposal time"
was wrong — it read `machine_clients.allowed_scopes` instead of the
`machine_access_grants` surface — and is hereby corrected and withdrawn.)

## 3. Frozen production facts (read-only, 2026-08-27)

| Fact | Value |
|---|---|
| Fleet HR principal | `dc702687-6515-4a2a-91ae-e572a9bbd766`, active, display_name `HR助手` |
| Existing enabled Domain bindings of that principal | DOMAIN_OWNER `hr-onboarding` (1) + DOMAIN_MEMBER × 8 (adc-v2-dogfood, build-in-public-dogfood, commercial-exploration-dogfood, game-dev, journal-submission, knowledge-curation, okr-dogfood, workflow-todo-dogfood) |
| Existing global role bindings of that principal | none |
| Fleet size (dsh-agent-core roster) | 86 agents; the other 85 are untouched by this exact-identity grant |
| `global_role_bindings.role_key` | free text at schema level (migrations/0020: TEXT 1..128, unique (principal_id, role_key)); supported values validated at the API layer — **no new migration needed** |

All existing Domain bindings remain unchanged. The sole increment is one
global binding row: principal `dc702687-…`, role_key
`GLOBAL_WORKFLOW_READER`, enabled.

## 4. New role definition — GLOBAL_WORKFLOW_READER

Permission matrix (server-side enforcement; frozen):

| Surface | GLOBAL_WORKFLOW_READER | GLOBAL_WORKFLOW_COORDINATOR |
|---|---|---|
| `GET /internal/v1/workflow-instances/global` (scope `workflow.read`) | ALLOW | ALLOW (unchanged) |
| `POST /internal/v1/domains` | DENY | ALLOW (unchanged, + `workflow.execute` + canary guard) |
| `PUT /internal/v1/domains/{domainId}/owner` | DENY | ALLOW (unchanged, + `workflow.execute` + canary guard) |
| workflow transitions / assignment mutation | DENY (assignee-gated, unchanged) | DENY (unchanged) |
| cancel / archive | DENY (DOMAIN_OWNER-gated, unchanged) | DENY (unchanged) |
| assistance-case listing (`require_global_coordinator`, assistance_transaction/query.rs:272-287) | DENY (unchanged) | ALLOW (unchanged) |
| provisioning / admin operations | DENY (admin-gated, unchanged) | DENY (unchanged) |
| Scheduler job surfaces | DENY (out of svc-workflow authority entirely) | DENY |

Explicit prohibitions on the role itself and on any future apply: READER must
never gate any write endpoint, any assistance surface, or anything other than
the global instance list; READER grants no scope (token scope authority stays
with auth-service grants); the assignment grants nothing to any other
principal.

## 5. Server-side change closure (authorized only upon acceptance)

No migration. Exact file closure (base 2ff81ae):

1. `src/domain/provisioning/mod.rs` — add
   `pub const GLOBAL_WORKFLOW_READER_ROLE: &str = "GLOBAL_WORKFLOW_READER";`
   with doc comment: read-only, gates ONLY the global instance list.
2. `src/store/postgres/workflow_instance_repository/query_visibility.rs`
   (the global-list gate, :54-65) — accept either role:
   `role_key IN ('GLOBAL_WORKFLOW_READER','GLOBAL_WORKFLOW_COORDINATOR') AND
   enabled = TRUE`. This gate is used ONLY by
   `WorkflowInstanceQueryService::list_global_instances`.
3. `src/application/workflow_instance/query_types.rs` +
   `query_service.rs:103-131` — the read-gate error variant's message names
   both accepted roles.
4. `src/http/error.rs` (`from_query`, :516-519) — global-list gate failure
   becomes HTTP 403 code **`global_read_role_required`** ("caller must hold
   GLOBAL_WORKFLOW_READER or GLOBAL_WORKFLOW_COORDINATOR").
   `from_provisioning` (:302-303) and the coordinator write endpoints keep
   `global_coordinator_required` unchanged.
5. `src/http/handlers/provisioning/global_role_bindings.rs` (:34-39 PUT,
   :76-79 DELETE) — accept `roleKey` ∈ {`GLOBAL_WORKFLOW_COORDINATOR`,
   `GLOBAL_WORKFLOW_READER`}; anything else stays 422 `role_key_invalid`.
   (Store layer `upsert_global_role_binding` / revoke need no change — they
   do not whitelist role keys.)
6. Tests (unit + conformance per repo convention) — obligations in §8.

Deliberately UNCHANGED (red lines): `handlers/coordinator_domains.rs`
(`verify_coordinator` stays exact COORDINATOR-only), the assistance
`require_global_coordinator` predicate, all transition/cancel/archive/domain
write gates, all admin provisioning gates, route table, migrations.

## 6. Wire contract change (freeze)

`GET /internal/v1/workflow-instances/global` failure contract after §5:
non-holder (neither role) → `403 {"error":{"code":"global_read_role_required",
"message":"caller must hold GLOBAL_WORKFLOW_READER or GLOBAL_WORKFLOW_COORDINATOR"}}`.
Before §5 deploys, the same failure is `403 global_coordinator_required`
(existing behavior). Both codes are transitional realities for downstream
declarers (see companion broker Spec PR #83). No other endpoint's error
contract changes.

## 7. Production apply path (future, separately authorized)

```text
PUT /internal/v1/admin/global-role-bindings/dc702687-6515-4a2a-91ae-e572a9bbd766
Headers: Idempotency-Key: <fresh uuid>, x-request-id: <uuid>
Body:    {"roleKey": "GLOBAL_WORKFLOW_READER", "enabled": true}
Auth:    admin provisioning credential
```

Pre-apply checklist (recorded in the apply record):

1. §5 change is deployed (READER gate + new error code live);
2. principal still exists, active, still maps to `agt_hr-agent`;
3. no enabled GLOBAL_WORKFLOW_READER binding already exists (idempotent
   exact-rerun NOOP acceptable per receipt);
4. exact-identity apply only — the legacy HR principal `bc970ced-…` gets
   NOTHING; the other 85 fleet agents' bindings untouched.

Rollback: `DELETE /internal/v1/admin/global-role-bindings/dc702687-6515-4a2a-91ae-e572a9bbd766`
with `{"roleKey": "GLOBAL_WORKFLOW_READER"}` (sets enabled=false, idempotent).
Code rollback = revert the §5 change (role rows are inert free text without
the gate).

## 8. Acceptance criteria

- AC-1: with an enabled READER binding, a `workflow.read`-scoped token for
  the frozen principal calling the global list returns 200
  `Page<DomainInstanceSummary>` (summaries only, no detail/submissions).
- AC-2: with an enabled COORDINATOR binding (any test principal), the same
  call still returns 200 (no regression).
- AC-3: with neither role, the call returns 403 `global_read_role_required`.
- AC-4: with ONLY a READER binding, `POST /internal/v1/domains` and
  `PUT /internal/v1/domains/{domainId}/owner` still return 403
  `global_coordinator_required` (write gates unchanged).
- AC-5: assistance listing with only READER still fails
  `global_coordinator_required` (unchanged).
- AC-6: PUT/DELETE admin global-role-bindings accept `roleKey:
  GLOBAL_WORKFLOW_READER`; other role keys still 422 `role_key_invalid`.
- AC-7: the frozen principal's 9 Domain bindings unchanged after apply; no
  binding created for any other principal.
- AC-8: repo test suite passes (conventions: disposable-DB conformance run).

## 9. Alternatives and disposition

- Grant existing `GLOBAL_WORKFLOW_COORDINATOR` to the HR main identity
  (original draft) — **REJECTED by OWNER_RULING**: role conflates read with
  domain-management write gates; HR holds `workflow.execute`-capable
  credentials on both lineage principals (§2).
- Strip the write endpoints from the COORDINATOR role instead — rejected:
  changes behavior of existing coordinators; broader blast radius than a new
  role.
- Revoke HR's `workflow.execute` grant first, then assign COORDINATOR —
  rejected: scope authority lives in auth-service (out of this repo's
  authority), and read-only intent should not depend on credential state.
- Per-domain DOMAIN_MEMBER grants to HR everywhere — rejected: does not
  scale to future domains and violates exact-increment minimalism.

## 10. What this PR changes

```text
DOCS ONLY — removes the withdrawn same-PR draft
(SVC_WORKFLOW_HR_GLOBAL_COORDINATOR_ASSIGNMENT_V1, cf45d7c6, never merged)
and adds exactly this file.
SVC_WORKFLOW_CODE_CHANGE (this PR) = NONE
ROLE_CHANGE                          = NONE (proposal only)
PRODUCTION_CHANGE                    = NONE
```

Independent review required before acceptance; acceptance authorizes only
the §5 implementation review path and the §7 apply path remains separately
owner-authorized.
