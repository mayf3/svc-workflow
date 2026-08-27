---
spec_id: SVC_WORKFLOW_HR_DISPATCHER_COORDINATOR_GRANT_V1
title: Exact Coordinator Role Grant for the Dedicated HR Dispatcher System Agent V1
status: proposed
repo: mayf3/svc-workflow
base_head: 2ff81ae47ab068216bd0012fa0e76a45dd2fb572
date: 2026-08-27
revision_of_drafts:
  - SVC_WORKFLOW_HR_GLOBAL_COORDINATOR_ASSIGNMENT_V1@cf45d7c6 (never merged; withdrawn — grants coordinator to HR main identity)
  - SVC_WORKFLOW_HR_GLOBAL_WORKFLOW_READER_ASSIGNMENT_V1@e2e3464 (never merged; withdrawn — read-only via new role)
  - SVC_WORKFLOW_HR_DISPATCHER_IDENTITY_V1@d1298b9 (never merged; withdrawn — identity/client governance moved to auth-service per authority split)
owner_ruling: DEDICATED_SYSTEM_AGENT_MODEL
product_code_changed_by_this_spec_pr: false
svc_workflow_code_change_upon_acceptance: none
identity_governed_here: false (Principal/Client/scopes/grants/secret handoff/rerun/rollback governed by mayf3/auth-service AUTH_SERVICE_AGENTCORE_HR_DISPATCHER_IDENTITY_V1)
role_applied_by_this_pr: false
production_apply_authorized: false
companion_specs:
  - mayf3/auth-service AUTH_SERVICE_AGENTCORE_HR_DISPATCHER_IDENTITY_V1 (identity authority)
  - mayf3/dsh-agent-core AGENT_CORE_HR_DISPATCHER_V1 (system Agent / scheduler authority)
  - mayf3/dsh-agent-core AGENT_CORE_WORKFLOW_GLOBAL_INSTANCES_CAPABILITY_V1 (generic broker capability)
---

# SVC_WORKFLOW_HR_DISPATCHER_COORDINATOR_GRANT_V1

## 1. Decision summary

OWNER_RULING = `DEDICATED_SYSTEM_AGENT_MODEL`. This Spec governs exactly ONE
thing: the svc-workflow data-level grant of the existing
`GLOBAL_WORKFLOW_COORDINATOR` role to the principal of the **dedicated
system Agent** `agt_workflow-dispatcher-hr-agent` — and the explicit
non-grant to everyone else. Identity, Client, scopes, grants, secret
handoff, rerun and rollback of that identity are governed by the
auth-service Spec (authority split, §5); the Agent definition, minimal
runtime directory, scheduler execution and wake path are governed by
dsh-agent-core `AGENT_CORE_HR_DISPATCHER_V1`.

```text
GRANT_TARGET      = principal of agt_workflow-dispatcher-hr-agent
                    (UUID = <PENDING_AUTH_IDENTITY>; backfilled by amendment
                    after the auth-service identity exists — §6)
ROLE_KEY          = GLOBAL_WORKFLOW_COORDINATOR (existing role, unchanged)
HR_MAIN_COORDINATOR_ROLE = NO   (frozen; agt_hr-agent / dc702687-… never)
SVC_WORKFLOW_CODE_CHANGE  = NONE (no new role, no gate change, no migration,
                    no error-code change)
ROLE_APPLIED_THIS_ROUND   = NO
```

The dispatcher's read-only reach is structural and lives outside this repo:
its auth-service grant set is exactly `{workflow.read (+ the agent-wake
capability scope)}` with `workflow.execute` / `workflow.admin` /
`scheduler.manage` FORBIDDEN, so the coordinator role's write-gated
endpoints (`POST /internal/v1/domains`, `PUT
/internal/v1/domains/{domainId}/owner` — each additionally requiring a
`workflow.execute`-scoped token + canary guard, src/http/mod.rs:152-162)
stay unreachable for it.

## 2. Identity model (cross-repo alignment, non-normative here)

`agt_workflow-dispatcher-hr-agent` is a **dedicated system Agent** per the
ruling: it HAS an Agent definition, an independent Auth Principal/Client,
and a minimal runtime directory; it has NO Feishu binding and NO OpenClaw
runtime; it CAN be executed by the Agent Core Scheduler. It is NOT one of
the 86 business trusted-fleet identities (the fleet roster stays
byte-unchanged), and must NOT be described as a pure service identity
without an Agent lifecycle, nor as an alias of the HR main session, nor as
the 87th business fleet Agent. Normative ownership of those properties is
split per §5; this section only prevents model drift.

## 3. Security boundary finding (carried evidence, why HR main gets nothing)

Production read-only evidence (auth-service DB, 2026-08-27):

| Identity | Principal | Credential reach |
|---|---|---|
| Fleet HR agent (`agentcore:v1:principal:agt_hr-agent`) | `dc702687-6515-4a2a-91ae-e572a9bbd766` | active client `mc_IuBMfCYe9-b522IhSWKBGjyz`; machine_access_grants v2 = **{workflow.read, workflow.execute}** |
| Legacy HR agent (`openclaw:agent:hr-agent`) | `bc970ced-710f-4479-9ff0-e295a1c59424` | active client `mc_4Ud_9wGR1mwQM9W7s7foX8qp`; {workflow.admin, workflow.read, workflow.execute} |

`GLOBAL_WORKFLOW_COORDINATOR` gates the global instance read AND the two
domain-management write endpoints above. Any HR-lineage principal can mint
`workflow.execute` tokens today, so granting the role there would make the
write endpoints reachable in principle. Frozen rulings:

```text
HR_MAIN (agt_hr-agent / dc702687-…) gets: NO GLOBAL_WORKFLOW_COORDINATOR,
  no coordinator-equivalent role, no svc-workflow data change of any kind.
LEGACY HR (bc970ced-…) gets: NOTHING.
```

## 4. The grant (the only thing this Spec authorizes, upon separate apply)

Execution after the auth-service identity exists and its UUID is backfilled
(§6), via the existing admin idempotent seam (handlers/provisioning/
global_role_bindings.rs, already accepts exactly this roleKey):

```text
PUT /internal/v1/admin/global-role-bindings/<dispatcher-principal-uuid>
Headers: Idempotency-Key: <fresh uuid>, x-request-id: <uuid>
Body:    {"roleKey": "GLOBAL_WORKFLOW_COORDINATOR", "enabled": true}
Auth:    admin provisioning credential
```

Authorized effect: the dispatcher credential (scope `workflow.read`) may
call the existing `GET /internal/v1/workflow-instances/global`
(DomainInstanceSummary projection only). Nothing else changes for anyone.

Rollback: `DELETE /internal/v1/admin/global-role-bindings/
<dispatcher-principal-uuid>` with the same roleKey (sets enabled=false,
idempotent). No code rollback exists or is needed.

## 5. Cross-repo authority split (frozen)

| Concern | Governing authority |
|---|---|
| Principal, Client, exact scopes/grants (workflow.read + agent-wake scope; workflow.execute / workflow.admin / scheduler.manage FORBIDDEN), secret handoff, rerun NOOP, rollback/revoke | mayf3/auth-service `AUTH_SERVICE_AGENTCORE_HR_DISPATCHER_IDENTITY_V1` |
| THIS grant (coordinator role on the exact dispatcher principal), HR-main non-grant, svc-workflow code = NONE | THIS Spec |
| Agent definition, minimal runtime directory, no-Feishu/no-OpenClaw, scheduler execution, wake path, HR managed-set scheduler tools | mayf3/dsh-agent-core `AGENT_CORE_HR_DISPATCHER_V1` |
| Generic broker capability `workflow_global_instances` | mayf3/dsh-agent-core `AGENT_CORE_WORKFLOW_GLOBAL_INSTANCES_CAPABILITY_V1` |

This Spec must not, and does not, govern Principal/Client/Credential/Grant
creation or mutation — those are out of scope here by ruling.

## 6. Pre-apply checklist (recorded in the apply record)

- E-1: auth-service identity exists per its Spec; the created
  principal UUID + client id are ALREADY backfilled into this Spec by
  amendment (apply is invalid against `<PENDING_AUTH_IDENTITY>`);
- E-2: backfilled principal maps to `agt_workflow-dispatcher-hr-agent`
  (external_ref `agentcore:v1:principal:agt_workflow-dispatcher-hr-agent`),
  enabled; its grant set verified to be exactly {workflow.read (+ agent-wake
  scope)} — any workflow.execute/admin reach-through = BLOCKER;
- E-3: no enabled GLOBAL_WORKFLOW_COORDINATOR binding already exists for it
  (idempotent exact-rerun NOOP acceptable per receipt);
- E-4: production `global_role_bindings` read by an authorized reader;
  existing enabled coordinator holders enumerated and recorded (baseline);
  (honest limit: not readable with auth_ro at proposal time);
- E-5: HR main and legacy HR principals unchanged; 86 fleet roster and all
  fleet identities byte-unchanged;
- E-6: svc-workflow deployed revision still matches base expectations (gate
  semantics unchanged — no code change attributable to this family).

## 7. Acceptance criteria

- AC-1: after apply, the dispatcher credential (workflow.read token) calls
  the global list → 200 `Page<DomainInstanceSummary>`.
- AC-2: before apply / after rollback, the same call → 403
  `global_coordinator_required` (negative control).
- AC-3: any non-coordinator fleet agent → 403 (no privilege leak).
- AC-4: dispatcher credential requesting a workflow.execute token is
  refused by auth-service (structural read-only; verified on the auth side,
  referenced here).
- AC-5: HR main + legacy HR: no role binding before/after (diff empty).
- AC-6: apply record shows Idempotency-Key receipt + E-1..E-6 results.
- AC-7: svc-workflow code unchanged (repo diff attributable to this Spec =
  none).

## 8. Alternatives and disposition

- Coordinator role to HR main identity (draft 1) — **rejected**: HR lineage
  holds workflow.execute-capable credentials; write endpoints reachable.
- New `GLOBAL_WORKFLOW_READER` role + server change (draft 2) — **rejected
  by P0/P0′ rulings**: credential-scope separation achieves read-only
  structurally with zero server change.
- Identity/client/grant plan governed here (draft 3) — **rejected by
  authority split**: identity governance belongs to auth-service; this Spec
  was trimmed to the role grant only.
- Revoke HR workflow.execute first, then grant — rejected: couples read-only
  intent to fleet credential surgery, out of authority.

## 9. What this PR changes

```text
DOCS ONLY — removes the withdrawn same-PR draft
(SVC_WORKFLOW_HR_DISPATCHER_IDENTITY_V1, d1298b9, never merged) and adds
exactly this file.
SVC_WORKFLOW_CODE_CHANGE = NONE
ROLE_CHANGE              = NONE (proposal only)
PRODUCTION_CHANGE        = NONE
```

Independent review required before acceptance; acceptance authorizes only
the §4 grant-review path against a backfilled UUID — production apply
remains separately owner-authorized.
