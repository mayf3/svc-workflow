---
spec_id: SVC_WORKFLOW_HR_DISPATCHER_IDENTITY_V1
title: Dedicated Workflow Dispatcher Identity for HR Global Read V1
status: proposed
repo: mayf3/svc-workflow
base_head: 2ff81ae47ab068216bd0012fa0e76a45dd2fb572
date: 2026-08-27
revision_of_drafts:
  - SVC_WORKFLOW_HR_GLOBAL_COORDINATOR_ASSIGNMENT_V1@cf45d7c6 (same-PR draft, never merged, withdrawn by INTRODUCE_READ_ONLY_GLOBAL_ROLE)
  - SVC_WORKFLOW_HR_GLOBAL_WORKFLOW_READER_ASSIGNMENT_V1@e2e3464 (same-PR draft, never merged, withdrawn by P0_USE_DEDICATED_WORKFLOW_DISPATCHER_IDENTITY)
owner_ruling: P0_USE_DEDICATED_WORKFLOW_DISPATCHER_IDENTITY
product_code_changed_by_this_spec_pr: false
svc_workflow_code_change_upon_acceptance: none
identity_created_by_this_pr: false
production_apply_authorized: false
companion_broker_spec: mayf3/dsh-agent-core AGENT_CORE_WORKFLOW_GLOBAL_INSTANCES_CAPABILITY_V1 (proposed, PR #83)
---

# SVC_WORKFLOW_HR_DISPATCHER_IDENTITY_V1

## 1. Decision summary

OWNER_RULING = `P0_USE_DEDICATED_WORKFLOW_DISPATCHER_IDENTITY`. Neither the
existing `GLOBAL_WORKFLOW_COORDINATOR` role nor any other role may be granted
to the HR main identity (`agt_hr-agent`), because that identity already holds
`workflow.execute`-capable credentials while the coordinator role also gates
Domain creation / Domain-owner replacement. Read-only global visibility is
achieved instead through **credential scope separation on a NEW dedicated
dispatcher identity**:

```text
HR_MAIN_COORDINATOR_ROLE = NO          (frozen; no role, no scope change to the HR main identity)
DEDICATED_DISPATCHER_MODEL = YES
DISPLAY_PURPOSE          = workflow-dispatcher-hr-agent
ROLE_KEY (future grant)  = GLOBAL_WORKFLOW_COORDINATOR   (existing role, unchanged semantics)
GRANT SCOPES (future)    = {workflow.read} EXACTLY       (no workflow.execute, no workflow.admin)
SVC_WORKFLOW_CODE_CHANGE = NONE                          (no new role, no gate change, no migration)
IDENTITY_CREATED_THIS_ROUND = NO                        (plan only; production creation separately authorized)
```

The read-only guarantee is structural: the coordinator role's write-gated
endpoints additionally require a `workflow.execute`-scoped token, and
auth-service refuses to mint scopes outside the credential's grant
(`scope ⊄ grant.scopes → 400 invalid_scope`). A credential whose grant is
exactly `{workflow.read}` can therefore NEVER reach
`POST /internal/v1/domains` or `PUT /internal/v1/domains/{domainId}/owner`,
while the global instance list (`workflow.read` + coordinator role) works.

## 2. Security boundary finding (carried evidence)

Production read-only evidence (auth-service DB, 2026-08-27):

| Identity | Principal | Credential reach |
|---|---|---|
| Fleet HR agent (`agentcore:v1:principal:agt_hr-agent`) | `dc702687-6515-4a2a-91ae-e572a9bbd766` | active client `mc_IuBMfCYe9-b522IhSWKBGjyz`; machine_access_grants v2 = **{workflow.read, workflow.execute}** (not revoked) |
| Legacy HR agent (`openclaw:agent:hr-agent`) | `bc970ced-710f-4479-9ff0-e295a1c59424` | active client `mc_4Ud_9wGR1mwQM9W7s7foX8qp`; allowed_scopes + grant = **{workflow.admin, workflow.read, workflow.execute}** |

`GLOBAL_WORKFLOW_COORDINATOR` gates, server-side: the global instance read
(wanted) AND `POST /internal/v1/domains` + `PUT
/internal/v1/domains/{domainId}/owner` (each additionally needs
`workflow.execute` + canary guard — src/http/mod.rs:152-162,
handlers/coordinator_domains.rs). Granting the role to any HR lineage
principal would make those write endpoints reachable in principle. Both prior
same-PR drafts are withdrawn: direct coordinator grant (rejected — §2), and
the `GLOBAL_WORKFLOW_READER` role introduction (rejected by this P0 ruling —
a new role + server gate/error-code change is a larger blast radius than a
dedicated identity with a read-only grant; the server stays untouched).

## 3. Read-only investigation results (2026-08-27)

| Question | Result |
|---|---|
| Does a dispatcher identity already exist? | **NO** — auth-service active-principal lookup and `machine_principals.external_ref ILIKE '%dispatch%'` both empty |
| Does anything like `workflow-dispatcher-hr-agent` exist? | NO |
| Current production holders of GLOBAL_WORKFLOW_COORDINATOR (`global_role_bindings`) | **NOT VERIFIED** — the read-only account (`auth_ro`) is denied on that table in the live DB; recorded as mandatory pre-execution check E-5 (§6) instead of guessed |
| Fleet roster (dsh-agent-core) | 86 identities, EXACT_ROSTER_SHA256 f046d18f… (docs/investigations/build-in-public-mapping-canary-plan-v1); dispatcher is NOT a fleet identity and the roster stays byte-unchanged |

## 4. Frozen future identity/client plan (the ONLY thing this Spec freezes)

One new, dedicated, non-fleet service identity — to be created only under a
separate owner-authorized execution record, exactly as follows and nothing
more:

```text
PRINCIPAL
  principal_type = machine (agent family)
  agent_id       = agt_workflow-dispatcher-hr-agent
  external_ref   = agentcore:v1:principal:agt_workflow-dispatcher-hr-agent
  display name / purpose = workflow-dispatcher-hr-agent
  status         = active
  UUID           = assigned at creation; the created UUID MUST be recorded
                   back into this Spec (amendment) before the §6 role apply
CLIENT
  exactly ONE machine client for that principal (client_id minted by
  auth-service; recorded with the UUID amendment)
GRANT
  exactly ONE machine access grant for audience svc-workflow:
  scopes = {workflow.read} EXACTLY
  explicit prohibitions: NO workflow.execute, NO workflow.admin, NO other
  audiences/scopes piggybacked in the same execution
ROLE (after E-checks, separate step)
  PUT /internal/v1/admin/global-role-bindings/<dispatcher-uuid>
  Body: {"roleKey": "GLOBAL_WORKFLOW_COORDINATOR", "enabled": true}
  — existing admin idempotent endpoint (handlers/provisioning/
  global_role_bindings.rs already accepts exactly this roleKey)
```

Structural prohibitions on the identity itself:

- does NOT reuse, clone, or reference any HR main-identity credential;
- is NOT a fleet identity: not added to the dsh-agent-core agents.json roster
  (stays 86), no workspace, no Agent lifecycle, no Feishu binding, no
  scheduler surface, no session;
- grant mutation adding `workflow.execute`/`workflow.admin` to this client is
  OUT OF this Spec's authority and requires explicit owner authorization
  (recorded; audits verify — see §8 AC-6).

## 5. What changes where (authority map)

| Repo / system | Change authorized by this Spec |
|---|---|
| svc-workflow code / routes / migrations | **NONE** (global gate, coordinator role, write endpoints, error codes all unchanged) |
| auth-service | identity + client + grant creation per §4 — future, separately authorized execution (this PR: plan only) |
| svc-workflow data (role binding) | one exact-identity binding per §4 — future, after E-checks |
| HR main identity (`dc702687-…`) | **NOTHING** — no role, no scope, no credential change |
| Other 85 fleet identities | NOTHING |
| dsh-agent-core broker | governed separately by PR #83 (generic coordinator capability; no HR-session binding) |

## 6. Execution path (future, separately authorized)

Execution = (a) create identity/client/grant exactly per §4 → (b) amend this
Spec with the created UUIDs → (c) role apply → (d) negative/positive
verification. Pre-execution and pre-apply checklist (recorded):

- E-1: §4 shape re-validated against then-current auth-service schema;
- E-2: dispatcher identity still absent (no accidental pre-creation);
- E-3: created principal maps to `agt_workflow-dispatcher-hr-agent`, client
  active, grant scopes exactly {workflow.read};
- E-4: HR main identity unchanged (no new role/scope/credential) and legacy
  HR principal `bc970ced-…` unchanged;
- E-5: production `global_role_bindings` read by an authorized reader; all
  existing enabled GLOBAL_WORKFLOW_COORDINATOR holders enumerated and
  recorded (baseline before the new binding);
- E-6: fleet roster still 86/86 byte-unchanged.

Rollback: revoke role (`DELETE /internal/v1/admin/global-role-bindings/
<dispatcher-uuid>`, roleKey GLOBAL_WORKFLOW_COORDINATOR), revoke client, no
code rollback needed (server untouched).

## 7. Acceptance criteria

- AC-1: with the dispatcher credential (grant = {workflow.read}) + enabled
  coordinator binding, a `workflow.read` token calling
  `GET /internal/v1/workflow-instances/global` returns 200
  `Page<DomainInstanceSummary>` (summaries only).
- AC-2: the same credential requesting a `workflow.execute` token is refused
  by auth-service (`invalid_scope`) — proving the write endpoints are
  unreachable.
- AC-3: with the coordinator binding but a NON-coordinator caller (any fleet
  agent), the global list returns 403 `global_coordinator_required` (no
  privilege leak).
- AC-4: HR main identity and legacy HR principal: no role binding, no scope
  or credential change (before/after diff = empty).
- AC-5: fleet roster and the other 85 fleet identities byte-unchanged.
- AC-6: audit verifies the dispatcher grant is still exactly {workflow.read}
  (no drift to workflow.execute/admin) — drift = FAIL.
- AC-7: svc-workflow code unchanged (no diff in this repo attributable to
  this Spec).

## 8. Alternatives and disposition

- Grant existing coordinator role to HR main identity (draft 1) — **rejected**:
  HR holds `workflow.execute`-capable credentials on both lineage principals
  (§2); write endpoints would become reachable.
- Introduce `GLOBAL_WORKFLOW_READER` role + server gate/error-code change
  (draft 2) — **rejected by P0 ruling**: unnecessary server change and error
  churn when credential scope separation achieves read-only structurally;
  server stays untouched.
- Revoke HR's `workflow.execute` first, then grant coordinator — rejected:
  out of this repo's authority and couples read-only intent to fleet
  credential surgery.
- Per-domain DOMAIN_MEMBER grants to HR — rejected: does not cover future
  domains; not a global view.

## 9. What this PR changes

```text
DOCS ONLY — removes the withdrawn same-PR draft
(SVC_WORKFLOW_HR_GLOBAL_WORKFLOW_READER_ASSIGNMENT_V1, e2e3464, never merged)
and adds exactly this file.
SVC_WORKFLOW_CODE_CHANGE = NONE
IDENTITY_CREATED        = NO
ROLE_CHANGE              = NONE (future plan only)
PRODUCTION_CHANGE        = NONE
```

Independent review required before acceptance; acceptance authorizes only
the §6 execution-review path — production identity creation and role apply
remain separately owner-authorized.
