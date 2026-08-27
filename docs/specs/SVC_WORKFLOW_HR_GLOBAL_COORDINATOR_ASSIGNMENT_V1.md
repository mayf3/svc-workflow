---
spec_id: SVC_WORKFLOW_HR_GLOBAL_COORDINATOR_ASSIGNMENT_V1
title: Exact HR Agent GLOBAL_WORKFLOW_COORDINATOR Role Assignment V1
status: proposed
repo: mayf3/svc-workflow
base_head: 2ff81ae47ab068216bd0012fa0e76a45dd2fb572
date: 2026-08-27
product_code_changed_by_this_spec_pr: false
role_change_authorized_now: false
production_apply_authorized: false
companion_broker_spec: mayf3/dsh-agent-core AGENT_CORE_WORKFLOW_GLOBAL_INSTANCES_CAPABILITY_V1 (proposed)
---

# SVC_WORKFLOW_HR_GLOBAL_COORDINATOR_ASSIGNMENT_V1

## 1. Decision summary

This Spec proposes exactly one role increment, for exactly one frozen identity,
to authorize read-only visibility of workflow instance summaries across ALL
domains:

```text
AGENT_ID      = agt_hr-agent
PRINCIPAL_ID  = dc702687-6515-4a2a-91ae-e572a9bbd766   (frozen, full UUID)
ROLE_KEY      = GLOBAL_WORKFLOW_COORDINATOR             (existing role, no new role key)
BINDING_KIND  = global (domain-independent)
ENABLED       = true
```

The PRINCIPAL_ID was obtained on 2026-08-27 via a production read-only query
(auth-service principal lookup by agent_id; display_name `HR助手`, status
`active`) and is frozen into this Spec. Implementation and production apply are
NOT authorized by this document at proposal time:

```text
PRODUCTION_APPLY_AUTHORIZED = NO
ROLE_APPLIED_BY_THIS_PR     = NO
SVC_WORKFLOW_CODE_CHANGE    = NONE
MIGRATION_CHANGE            = NONE
OTHER_PRINCIPALS_AFFECTED   = 0
```

## 2. Frozen production facts (read-only, 2026-08-27)

| Fact | Value |
|---|---|
| Principal UUID | `dc702687-6515-4a2a-91ae-e572a9bbd766` |
| Principal status | active |
| Existing enabled bindings | DOMAIN_OWNER `hr-onboarding` (1) + DOMAIN_MEMBER × 8 (adc-v2-dogfood, build-in-public-dogfood, commercial-exploration-dogfood, game-dev, journal-submission, knowledge-curation, okr-dogfood, workflow-todo-dogfood) |
| Existing global role bindings | none |
| Associated machine client | `mc_IuBMfCYe9-b522IhSWKBGjyz` (active; observed scope set `{}` at query time) |
| Fleet size (dsh-agent-core roster) | 86 agents; the other 85 are untouched by this exact-identity grant |

All existing Domain bindings above remain byte-for-byte unchanged. The sole
increment is the single global binding named in §1.

## 3. Normative repository findings (base 2ff81ae)

1. The role already exists as a formal constant with read-only intent:
   `GLOBAL_WORKFLOW_COORDINATOR_ROLE` (src/domain/provisioning/mod.rs:39) —
   "the formal cross-domain read-only workflow role. Holders may read workflow
   instance summaries across all domains via
   `GET /internal/v1/workflow-instances/global`. It grants no write powers:
   transitions stay assignee-gated, cancel/archive stay DOMAIN_OWNER-gated,
   provisioning stays admin-gated."
2. The authorized read surface already exists and is deployed:
   `GET /internal/v1/workflow-instances/global` (src/http/mod.rs:119 →
   handlers/instances.rs `global_list`; scope `workflow.read` at
   instances.rs:177).
3. Authorization is enforced server-side before any query:
   `WorkflowInstanceQueryService::list_global_instances`
   (src/application/workflow_instance/query_service.rs:103+) calls
   `check_global_workflow_coordinator`; a caller without an enabled binding
   receives `WorkflowQueryError::GlobalCoordinatorRequired`, mapped to
   **HTTP 403 `global_coordinator_required`** (src/http/error.rs:516-519).
4. The grant path already exists and is admin-gated:
   `PUT /internal/v1/admin/global-role-bindings/{principalId}`
   (src/http/mod.rs:242 → handlers/provisioning/global_role_bindings.rs:19+;
   `ProvisioningAuth`, mandatory `Idempotency-Key`, `roleKey` must equal
   `GLOBAL_WORKFLOW_COORDINATOR`, emits the idempotent
   `PROVISION_GLOBAL_ROLE_BINDING` command). Revocation:
   `DELETE` on the same path (`REVOKE_GLOBAL_ROLE_BINDING`).
5. The binding storage check is `role_key = 'GLOBAL_WORKFLOW_COORDINATOR' AND
   enabled = TRUE` with no domain scope
   (src/store/postgres/workflow_instance_repository/query_visibility.rs:54-65).

This Spec requires no svc-workflow code, route, or migration change. It is a
governing Spec for an operational role assignment executed through existing
seams.

## 4. Authorized reach (the ONLY intended effect)

After a separately authorized production apply, the frozen principal may:

- call `GET /internal/v1/workflow-instances/global` (read-only, paginated
  instance summaries across all domains, `DomainInstanceSummary` projection —
  no instance detail, no submission payloads);
- use it through the dsh-agent-core broker capability
  `workflow_global_instances` (companion broker Spec, separate repository and
  review).

## 5. Explicit non-goals and prohibitions

The assignment and any future apply MUST NOT be read as authorization for:

- workflow transitions (remain assignee-gated);
- assignment modifications (current-node assignee changes);
- Domain creation, Domain modification, or Domain-owner changes;
- Scheduler job creation/modification/enablement (any scheduler surface);
- provisioning/admin operations (remain admin-gated);
- granting the role to any other principal, or to a role key other than
  `GLOBAL_WORKFLOW_COORDINATOR`;
- changing any of the other 85 fleet agents' identities, credentials, scopes,
  or bindings (the grant is exact-identity; no wildcard, no group semantics).

## 6. Honest role-power disclosure (write-gated coordinator endpoints)

`GLOBAL_WORKFLOW_COORDINATOR` also server-side gates two agent-facing
domain-management write endpoints (src/http/mod.rs:152-162 →
handlers/coordinator_domains.rs):

```text
POST /internal/v1/domains                      (create domain)
PUT  /internal/v1/domains/{domainId}/owner     (set domain owner)
```

Both additionally require scope `workflow.execute` on the presented token and
pass the `canary_write_guard`. This Spec's intent is read-only usage;
nevertheless the role itself is the server-side gate for those endpoints, and
this Spec must not hide that fact. The read-only outcome is conditioned as
follows:

- OBSERVED: the principal's current associated client
  (`mc_IuBMfCYe9-b522IhSWKBGjyz`) shows an empty scope set at 2026-08-27, so no
  `workflow.execute` reach-through exists at proposal time;
- REQUIRED pre-apply check (§7): the applier must re-verify that no enabled
  credential path for this principal can mint `workflow.execute` (or
  `workflow.admin`) tokens; if such a path exists it is PRE_EXISTING and must
  be recorded in the apply record, not silently accepted;
- this Spec authorizes no scope grant; the companion broker capability
  requests `workflow.read` only.

## 7. Production apply path (future, separately authorized)

Acceptance of this Spec authorizes only the exact assignment below; it does
not execute it. Execution requires a separate explicit owner authorization
over an exact apply record:

```text
PUT /internal/v1/admin/global-role-bindings/dc702687-6515-4a2a-91ae-e572a9bbd766
Headers: Idempotency-Key: <fresh uuid>, x-request-id: <uuid>
Body:    {"roleKey": "GLOBAL_WORKFLOW_COORDINATOR", "enabled": true}
Auth:    admin provisioning credential
```

Pre-apply verification checklist (all must PASS, recorded in the apply record):

1. principal `dc702687-6515-4a2a-91ae-e572a9bbd766` still exists and is
   active, and still maps to agent_id `agt_hr-agent`;
2. no enabled `GLOBAL_WORKFLOW_COORDINATOR` binding already exists for it
   (idempotent re-run of the exact PUT is acceptable NOOP if the receipt says
   so);
3. the §6 scope reach-through check passes (no `workflow.execute`/`workflow.admin`
   minting path for this principal, or recorded as PRE_EXISTING);
4. the other 85 fleet agents' bindings are untouched (exact-identity apply,
   one binding row).

Rollback: `DELETE /internal/v1/admin/global-role-bindings/dc702687-6515-4a2a-91ae-e572a9bbd766`
(revokes = sets enabled=false; idempotent).

## 8. Acceptance criteria

- AC-1: after apply, a `workflow.read` token minted from the frozen principal's
  credential calling `GET /internal/v1/workflow-instances/global` returns
  `200` with a `Page<DomainInstanceSummary>` (summaries only).
- AC-2: before apply, the same call returns `403` with code
  `global_coordinator_required` (negative control, proves the server-side gate).
- AC-3: a non-coordinator agent's identical call returns
  `403 global_coordinator_required` (no privilege leak to the other 85).
- AC-4: the frozen principal's existing 9 Domain bindings are unchanged
  (1 OWNER + 8 MEMBER) after apply.
- AC-5: the apply record shows Idempotency-Key receipt, exact-identity PUT,
  and the §7 checklist results.
- AC-6: no write path (transition/assignment/domain/scheduler/provisioning)
  becomes executable with a `workflow.read`-scoped token held by this
  principal.

## 9. What this PR changes

```text
DOCS ONLY — exactly one new file: docs/specs/SVC_WORKFLOW_HR_GLOBAL_COORDINATOR_ASSIGNMENT_V1.md
SVC_WORKFLOW_CODE_CHANGE = NONE
ROLE_CHANGE              = NONE (proposal only)
PRODUCTION_CHANGE        = NONE
```

Independent review is required before acceptance; acceptance still does not
apply the role (§7 governs execution).
