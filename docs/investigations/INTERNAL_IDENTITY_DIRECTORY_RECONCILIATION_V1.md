---
investigation_id: INTERNAL_IDENTITY_DIRECTORY_RECONCILIATION_V1
status: final
goal: WORKFLOW_ASSIGNEE_CANONICAL_IDENTITY_RECONCILIATION_V1
recorded: 2026-09-06
author: Primary Goal Agent (same-goal continuation)
kind: read-only mechanical reconciliation
---

# INTERNAL_IDENTITY_DIRECTORY_RECONCILIATION_V1

## 1. Trigger

Owner product direction recorded 2026-09-06 (CONTINUE_SAME_GOAL, OWNER_PRODUCT_DIRECTION):
canonical identity lookup ("this Principal → which canonical Agent", "this Agent exists /
enabled") is foundational internal directory information. Any authenticated canonical
internal Agent/SERVICE caller must be able to perform the MINIMAL read. Consumers must not
each mint a dedicated lookup SERVICE Principal + per-purpose read Grants.

Consequence recorded by the Owner dispatch:
`CURRENT_IMPLEMENTATION_AUTHORITY_FOR_DEDICATED_SERVICE_MODEL = SUPERSEDED_DIRECTION_PENDING`.
No deletion of code, no production provisioning of the dedicated objects, no silent
re-implementation under the old accepted Authority. Docs-first successor path.

## 2. Mechanical reconciliation (FIRST RECONCILIATION)

Question: do existing accepted platform contracts already provide a generic authenticated
internal canonical-identity directory read (Principal UUID → {agentId, principalStatus};
Agent ID → {exists, enabled}) usable by ANY authenticated canonical internal Agent or
SERVICE principal?

Method: read-only inventory of every HTTP surface in mayf3/auth-service and
mayf3/dsh-agent-core, their caller-authorization models, response fields, and governing
accepted Specs, at the accepted pre-implementation bases
(auth `2af21f87769af50b1c38abcd19655bb28c023e9a`, dsh `bc88cc81477a38da5c52f9a8503413cf67f30ee2`)
and at the current implementation heads (auth `fa209b4`, dsh `dda69b5`).

### 2.1 Auth-service surfaces relevant to Principal→agentId/status

| Surface | Exists at base | Caller model | Fields | Sufficient? |
|---|---|---|---|---|
| GET `/api/v1/agent-principals/:principal_id/agent` | YES | V1 direct-machine, audience `agent-principal-resolution` (accepted_principal_types=[agent] — SERVICE excluded); MachineAccessGrant required; CTR-EAPR-005 freezes sole initial grant recipient = HR Agent Principal `dc702687-6515-4a2a-91ae-e572a9bbd766` | exactly `{principalId, agentId}`; status absent by contract (DEC-EAPR-003: disabled = 409 error, never a field) | NO |
| GET `/api/v1/principals/by-external-ref` | YES | SERVICE-only provisioning audience `svc-auth` / `auth.identity.provision` | `{id, principal_type, agent_id, external_ref}` — no status; keyed by external_ref, not UUID | NO |
| GET `/api/users`, `/api/users/:id` | YES | any authenticated legacy HS256 User | legacy User store incl. email/name/role | NO — wrong store; exposes profile fields |
| POST `/api/services/verify-token` | YES | any authenticated User; introspects a token the caller already holds | token claims + implicit status check | NO — introspection, not UUID directory |
| CCR audience registry at base | — | 9 audiences; every read audience is resource-specific (agent-only resource access or service-only provisioning/ingress) | — | NO generic internal read audience exists |
| GET `/api/v1/workflow-admission/principals/:principal_id/agent` | NO (new, `fa209b4`) | exactly ONE dedicated SERVICE principal `cedb954a-…` + client `svc-workflow-canonical-admission-v1` | `{principalId, agentId}` | NO — single-caller by design (the superseded model) |

### 2.2 Agent-core surfaces relevant to Agent exists/enabled

| Surface | Exists at base | Caller model | Fields | Sufficient? |
|---|---|---|---|---|
| GET `/v1/agents` (mobile projection) | YES | unauthenticated; loopback-only transport posture (mobile Gate 1: "No auth, no TLS, no LAN exposure") | `{id, name, avatar:null, description}` — `disabled` deliberately stripped | NO — not authenticated; enabled absent |
| GET `/scheduler/runs*`, `/scheduler/occurrences/:id` | YES | V1 JWKS verifier, audience `scheduler`, scope scheduler.read(self)/scheduler.audit(global) per AGENT_CORE_SCHEDULER_RUN_HISTORY_V1 R7/R8 | run history | NO — wrong resource |
| Broker capability `agent_resolve_principal` (LOCAL, not HTTP) | YES | per-agent credentials + requiredScopes `auth.agent.resolve` | `{principalId, agentId}` + local disabled check | NO — inherits the auth-side agent-only/HR-grant chain; not an HTTP surface |
| GET `/v1/workflow-admission/agents/:agentId` | NO (new, `dda69b5`) | the same ONE dedicated SERVICE caller | `{agentId, enabled:true, observationDigest}` | NO — single-caller by design |

### 2.3 Machine-client facts

No `svc-workflow` SERVICE machine client exists (the only svc-workflow-audience grant
holders are AGENT principals). Auth issuance enforces `machine_grant_missing` for any
audience without a MachineAccessGrant, so ANY token-bearing read necessarily involves a
grant tuple; the dedicated model added two per-purpose tuples for one consumer.

## 3. Disposition (candidate artifact updated in audit repair r1)

CASE A (existing generic read sufficient): NO — no surface at either base permits an
arbitrary authenticated internal principal to perform the minimal read.

CASE B (existing generic read missing only safe minimal fields): NO — the nearest surface
(`agent-principals`) is restricted in caller TYPE (agent-only), in GRANT (sole-recipient
HR tuple), AND in fields (status contractually excluded); more than fields is missing.

**CASE C (no suitable generic read exists): SELECTED.**
Prepare the smallest canonical identity-directory read Authority:

- auth successor Spec `AUTH_SERVICE_INTERNAL_IDENTITY_DIRECTORY_V1`
  (supersedes `AUTH_SERVICE_WORKFLOW_CANONICAL_ADMISSION_V1` @ `2af21f8`): exact-UUID
  Principal→Agent directory read for ANY authenticated canonical internal Agent/SERVICE
  caller; generic audience `identity-directory` / scope `auth.directory.read`
  (accepted principal types [agent, service]); success is exactly
  `{principalId, agentId, principalStatus}` with principalStatus ∈ {active, disabled} as
  data (disabled becomes observable state, not a 409); all existing exact-lookup,
  bidirectional-cardinality, deadline, fail-closed and no-leak semantics retained.
- dsh successor Spec `AGENT_CORE_INTERNAL_AGENT_DIRECTORY_V1`
  (supersedes `AGENT_CORE_WORKFLOW_CANONICAL_ADMISSION_V1` @ `bc88cc81`): exact Agent-ID
  directory read for ANY authenticated internal Agent/SERVICE caller; generic audience
  `agent-directory` / scope `agent.directory.read`; success is exactly
  `{agentId, exists, enabled}` (existence as data; observationDigest dropped); malformed
  input, ambiguity, storage-error and deadline semantics retained.
- svc whole-authority successor `SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2`
  (supersedes V1): CTR-CIR-003's admission orchestration re-pins its two external
  dependencies to the successor Specs and replaces the dedicated-Service-identity
  sentence cluster with the generic internal-directory contract; every other Goal
  contract is carried forward verbatim. Workflow still validates
  exists/active/enabled fail-closed before any assignment persistence. (An initial
  focused-amendment vehicle was rejected by independent semantic audit as unlawful
  partial supersession under this repository's SPEC_GOVERNANCE_V0 §9.1/9.2 and replaced
  by this whole-authority successor in the single bounded repair.)

## 4. Reuse map (what is NOT rewritten)

- auth: exact UUID validation, exact forward/reverse relation query, two-row cardinality,
  whole-operation deadline, sanitized error taxonomy, no-store, minimal selects, V1
  verifier integration (src/routes/workflow-admission.ts,
  src/middleware/v1-workflow-admission-auth.ts, agent-principal-resolution lib) — the
  caller-authorization predicate and response fields change; query logic is preserved.
- dsh: exact-ID grammar check, authoritative synchronous snapshot read, uniqueness/
  ambiguity discrimination, 1s operation deadline, no-store, scheduler-auth JWKS verifier
  reuse (packages/product-api/src/workflow-admission.js) — the caller predicate and
  response fields change.
- svc: the admission orchestration design (fail-closed, 5s window, exact-binding, no
  retry/cache) is unchanged; only the token audience/scope targets and the
  principalStatus/exists field checks follow the successor contracts.

## 5. Retirement list (spec-level only; none of these were ever provisioned)

- SERVICE Principal `cedb954a-3d99-4e5a-b568-d312441bcc56` — never created in any
  environment; retired from the accepted contract set.
- MachineClient `svc-workflow-canonical-admission-v1` (row `c37e5e03-…`) — never created.
- CCR audiences `workflow-principal-admission` / `workflow-agent-admission` and scopes
  `auth.agent.admission.read` / `agent.definition.admission.read` — present only in
  unmerged branch-local bundle WIP; the successor replaces them with the two generic
  entries before any merge.
- The dedicated two-Grant supply plan — superseded by generic grant supply through the
  existing governed grant machinery.

## 6. Consumer identity model after the successor

Consumers authenticate with their existing valid internal identity (per-agent credentials
for Agents; a general backend SERVICE principal + client for svc-workflow, provisioned
through the EXISTING accepted provisioning surface, holding exactly the generic
directory grant(s) it needs). One shared audience/scope pair per directory surface for
all consumers; no per-consumer-per-purpose lookup identities or grant pairs unless fresh
evidence proves a required security boundary.

## 7. Boundary (unchanged from Owner direction)

The generic read exposes ONLY: agentId + principalStatus (Principal UUID lookup) and
exists + enabled (Agent ID lookup). No credentials, client secrets, token material,
grant lists, profile fields, email/phone, arbitrary external_refs, authorization
internals, or any mutation. Anonymous/Internet access, arbitrary-data search, fuzzy or
display-name resolution remain forbidden.
