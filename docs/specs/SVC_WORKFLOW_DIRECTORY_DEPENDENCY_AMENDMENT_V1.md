---
spec_id: SVC_WORKFLOW_DIRECTORY_DEPENDENCY_AMENDMENT_V1
status: proposed
spec_kind: focused_amendment
amends: SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V1
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: conditional_controlled_operation
scope:
  - mayf3/svc-workflow
  - admission dependency re-pinning from dedicated Workflow SERVICE identity to the generic internal identity-directory reads
governed_by:
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V7
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_1
external_authorities:
  - repository: mayf3/auth-service
    authority_id: AUTH_SERVICE_INTERNAL_IDENTITY_DIRECTORY_V1
    relation: depends_on
  - repository: mayf3/dsh-agent-core
    authority_id: AGENT_CORE_INTERNAL_AGENT_DIRECTORY_V1
    relation: depends_on
supersedes: []
superseded_by: null
owners: [mayf3]
---

# SVC_WORKFLOW_DIRECTORY_DEPENDENCY_AMENDMENT_V1

## Goal

Amend exactly the admission-dependency sentences of the accepted parent
SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V1 (CTR-CIR-003, the paragraph beginning
"Apply this admission rule to corrected-source publish/defaults/enums …" through "…
No display-name fallback is introduced.") so that Workflow admission consumes the two
generic internal identity-directory successor Specs instead of the retired dedicated
Workflow SERVICE Principal/Client/Grant model. Owner direction 2026-09-06
(WORKFLOW_ASSIGNEE_CANONICAL_IDENTITY_RECONCILIATION_V1, CONTINUE_SAME_GOAL): the
dedicated-service model is SUPERSEDED_DIRECTION_PENDING; canonical identity lookup is
foundational internal directory information available to any authenticated canonical
internal Agent/SERVICE caller. Every other parent Contract, decision and acceptance row
is unchanged and remains in force.

## Amended text (replacement sentences)

Within CTR-CIR-003, replace the dependency/identity sentences with:

1. "The owning dependencies are Auth AUTH_SERVICE_INTERNAL_IDENTITY_DIRECTORY_V1 and dsh
   AGENT_CORE_INTERNAL_AGENT_DIRECTORY_V1 at their individually accepted exact heads.
   Workflow calls the pinned Auth directory route
   (GET /api/v1/directory/principals/{principalId}/agent) and, for the returned exact
   agentId, the pinned agent-core directory route (GET /v1/directory/agents/{agentId}),
   each with a fresh token for its exact read audience per command."
2. "Workflow authenticates as a normal authenticated canonical internal SERVICE caller:
   one general backend SERVICE principal + client for svc-workflow (its own service
   identity, not a per-purpose lookup identity), provisioned through the existing
   accepted provisioning surface and holding exactly the generic grants
   (identity-directory / auth.directory.read) and (agent-directory /
   agent.directory.read). The previously pinned dedicated admission identity
   (cedb954a-3d99-4e5a-b568-d312441bcc56 / svc-workflow-canonical-admission-v1 and the
   two workflow-*-admission audiences) is retired without ever having been provisioned."
3. "Admission acceptance now requires, for every distinct Agent Principal in the
   command: Auth observation with principalStatus == active and exact agentId, then
   agent-core observation with exists == true and enabled == true. Any other status,
   missing/disabled/duplicate identity, timeout or unavailable validator rejects the
   entire business write. Service URLs remain fixed reviewed backend configuration,
   never supplied by request fields; plain HTTP only on exact loopback endpoints;
   redirects forbidden; non-loopback requires authenticated TLS; responses must bind the
   requested exact UUID/ID; total admission through commit remains bounded to 5 seconds
   on a monotonic clock; at most 8 in-flight reads; no automatic retry; per-command
   identical IDs may share one still-current observation, never across commands."

All remaining sentences of that paragraph (business-actor separation, transaction
locking/precondition discipline, outcome_unknown receipt inspection, no cross-service
atomicity, no display-name fallback, before-persist scope including corrected-source
publish/defaults/enums, direct creation, revision, revise-and-transition, ordinary
future transitions, admin moves and the bounded repair operator) are unchanged.

## Motivation and evidence

Mechanical reconciliation (docs/investigations/INTERNAL_IDENTITY_DIRECTORY_RECONCILIATION_V1.md,
disposition CASE C): no suitable generic authenticated internal read existed at the
accepted bases; the smallest correction is the two successor directory Specs plus this
dependency re-pin. Workflow admission's fail-closed obligation is NOT reduced: the
removal of a dedicated SERVICE account removes only the per-consumer lookup identity,
never the before-persist validation.

## Boundary

This amendment authorizes no anonymous lookup, no Auth DB browsing, no fuzzy or
display-name resolution, no identity mutation, no new identity store, and no permission
broadening beyond the two generic directory read grants held by svc-workflow's own
backend identity. The parent's census, repair, successor-lineage, deprecation and
real-work acceptance contracts are untouched.

STATUS=proposed; IMPLEMENTATION_ALLOWED_NOW=NO; PRODUCTION_READY=NO. Requires
independent semantic review and exact-head Owner acceptance together with the two
successor Specs before any implementation continues under the amended dependency model.
