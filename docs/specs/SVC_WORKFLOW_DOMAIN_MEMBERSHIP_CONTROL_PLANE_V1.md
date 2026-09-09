---
spec_id: SVC_WORKFLOW_DOMAIN_MEMBERSHIP_CONTROL_PLANE_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: none
production_apply_authority: none
date: 2026-09-09
scope:
  - svc-workflow domain member management surface
    (GET/PUT/DELETE /internal/v1/domains/{domainId}/members[/...]) — the
    DOMAIN_OWNER member-management contract only
  - add-member `role` parameter, `already_member` logical-duplicate outcome,
    `domain_owner_delegation_forbidden` explicit rejection, command-receipt
    hash extension
governed_by:
  - docs/architecture/AUTH_PRINCIPAL_SELF_PROJECTION_AND_DOMAIN_MEMBERSHIP_V1.md
    (FROZEN) — this Spec amends ONLY its add-member semantics
    (re-add outcome + role grammar); every other frozen decision of that
    document is restated unchanged below
external_authorities:
  - dsh-agent-core AGENT_CORE_WORKFLOW_DOMAIN_MEMBERS_BROKER_V1 (candidate,
    same closure round) — the sole motivating consumer; its tool surface
    projects this contract
supersedes: []
superseded_by: null
owners:
  - mayf3/svc-workflow maintainers
---

# SVC_WORKFLOW_DOMAIN_MEMBERSHIP_CONTROL_PLANE_V1

## 1. Goal

Make the existing DOMAIN_OWNER member-management endpoints serve a stable
Agent-facing control plane: an explicit role grammar on add, a
machine-judgeable `already_member` outcome for logical duplicates, an
explicit (never silent) rejection of DOMAIN_OWNER delegation under the
current single-owner contract, and a command-receipt hash that covers the
full request identity.

```text
GOAL = domain member add/remove/list behave as a stable, idempotent,
       auditable control plane that a thin broker can project 1:1
SUCCESS_OUTCOME = every membership outcome is machine-judgeable from the
       error code alone; no silent upsert, no implicit role change
```

## 2. Problem (observed on main 4bbbbe9)

### OBS-DMC-001 — Logical duplicate add is a silent success

`insert_member_binding`
(src/store/postgres/domain_role_repository.rs:317-330) is an
`ON CONFLICT (domain_id, principal_id, role_key) DO UPDATE SET enabled =
TRUE` upsert. With a NEW idempotency key, re-adding a principal that
already holds an enabled `DOMAIN_MEMBER` binding returns 200 success and
performs a no-op write path (receipt + audit as if a fresh grant). A caller
cannot distinguish "granted now" from "already a member" without a second
read; an Agent-facing tool needs a stable `already_member` outcome
(per WORKFLOW_DOMAIN_MEMBERS_CONTROL_PLANE_V1 goal directive §8).

### OBS-DMC-002 — No role grammar on add

`add_member` (application + handler) accepts no `role`. The surface can
only ever grant `DOMAIN_MEMBER`. A control-plane contract that names its
role grammar explicitly (`DOMAIN_MEMBER` actionable now; `DOMAIN_OWNER`
explicitly forbidden under the frozen single-owner invariant) is required
so the rejection is a stable contract outcome instead of an unexpressible
request.

### OBS-DMC-003 — Receipt hash omits the role dimension

`compute_receipt_hash` covers
`{commandType, actorId, domainId, targetPrincipalId}`. Once `role` becomes
part of the request identity it must be inside the hash, otherwise two
different logical commands sharing one key would replay each other's
response.

## 3. Frozen context (unchanged; restated for review)

From AUTH_PRINCIPAL_SELF_PROJECTION_AND_DOMAIN_MEMBERSHIP_V1 (FROZEN) and
the current implementation, ALL preserved:

- `DOMAIN_OWNER_CAN_MANAGE_DOMAIN_MEMBER=true`; member add/remove/list is
  DOMAIN_OWNER-only, enforced server-side inside the business transaction
  (`check_domain_owner`; non-owner → 403 `not_domain_owner`).
- `DOMAIN_OWNER_CAN_MANAGE_DOMAIN_OWNER=false`; single-enabled-owner
  invariant via partial unique index `idx_drb_single_owner`
  (migrations/0001). Owner replacement stays on the
  GLOBAL_WORKFLOW_COORDINATOR `/owner` contract and the admin provisioning
  surface. NOT touched here; the delegation question is a separate
  candidate (SVC_WORKFLOW_DOMAIN_OWNER_DELEGATION_V1).
- Direct-token-only for member operations (OBO → 403
  `direct_token_required`); scopes: `workflow.read` (list) /
  `workflow.execute` (add/remove).
- Idempotency machinery: per-principal `(principal_id, idempotency_key)`
  unique receipt; replay returns the stored response verbatim; hash
  mismatch → 409 `idempotency_conflict`; in-flight → 425
  `command_still_processing`. Unchanged.
- Audit: `workflow_security_audits` written in-transaction for successful
  mutations only (`member_added` / `member_removed` with
  `resource_type='DOMAIN_MEMBERSHIP'`, `resource_id="{domainId}/{target}"`,
  details carrying operation/actor/target/domain/requestId/result).
  Unchanged.
- Error envelope `{"error":{"code","message"}}`; no SQL/storage error ever
  becomes a public code. Unchanged.
- `list_members` shape, cursor discipline, limit bounds (1..100, default
  20), and the owner-check-then-read transaction split. Unchanged.

## 4. Contract changes (the delta this Spec authorizes)

### CTR-DMC-001 — add-member request grammar

`PUT /internal/v1/domains/{domainId}/members/{principalId}` gains an
OPTIONAL JSON body:

```json
{ "role": "DOMAIN_MEMBER" }
```

- `role` omitted (or body absent) ⇒ `DOMAIN_MEMBER` — byte-level backward
  compatibility for existing callers.
- Unknown fields rejected (serde `deny_unknown_fields`, camelCase) ⇒ 400
  `invalid_input`; unknown role string ⇒ 400 `invalid_input`.
- Accepted enum values: `DOMAIN_MEMBER`, `DOMAIN_OWNER` (see CTR-DMC-002
  for the DOMAIN_OWNER outcome).

### CTR-DMC-002 — role outcomes

- `role=DOMAIN_MEMBER` (default): current semantics with the
  CTR-DMC-003 duplicate check.
- `role=DOMAIN_OWNER`: **403 `domain_owner_delegation_forbidden`** —
  stable, machine-judgeable, completes the command receipt (replay-stable)
  and writes NO mutation and NO success audit. This is the frozen
  single-owner invariant expressed on this surface; it does not change
  who may become owner (coordinator `/owner` and admin provisioning
  remain the only paths). If/when
  SVC_WORKFLOW_DOMAIN_OWNER_DELEGATION_V1 is accepted it will amend this
  clause; until then DOMAIN_OWNER here is a forbidden request, never a
  silent redirect to DOMAIN_MEMBER.

### CTR-DMC-003 — `already_member` (logical duplicate, new command)

Order of checks in `add_member` after target-principal validation and the
existing `principal_is_owner` guard: if the target already holds an
ENABLED `DOMAIN_MEMBER` binding and this is an owned (non-replay)
command, return **409 `already_member`**, message naming domainId +
target principalId. No binding write, no success audit row; the receipt
IS completed with the error outcome so replays of the same key return the
same 409 stably.

- Idempotent transport replay (same key, same hash) keeps returning the
  ORIGINAL stored outcome (200 for a real grant, 409 for a duplicate) —
  never a second business mutation and never a synthesized audit row.
- Same key with a different request hash (incl. different `role`) ⇒
  409 `idempotency_conflict` (existing machinery; now role-sensitive).
- Removing and re-adding across separate commands remains fully legal;
  `already_member` only fires on a concurrent ENABLED binding.

### CTR-DMC-004 — receipt hash extension

`compute_receipt_hash` canonical JSON becomes
`{commandType, actorId, domainId, targetPrincipalId, role}` for
`DOMAIN_MEMBER_ADD` (remove hash unchanged — remove has no role
parameter; its semantics remain "remove the enabled DOMAIN_MEMBER
binding", explicit and unambiguous).

### CTR-DMC-005 — remove stays explicit

`DELETE` keeps its exact current semantics: disables the enabled
`DOMAIN_MEMBER` binding only (`member_not_found` 404 when absent), never
touches DOMAIN_OWNER bindings. No broad delete, no role guessing.

### CTR-DMC-006 — public error registry

`contracts/workflow-http/v1/errors.json` + `openapi.yaml` + `contract.md`
gain: `already_member` (409), `domain_owner_delegation_forbidden` (403);
`invalid_input` (400) documented for the body grammar. No existing code
changes meaning. `principal_is_owner` (409) remains as-is (adding the
domain owner as DOMAIN_MEMBER).

## 5. Non-goals

- No multi-owner enablement, no change to `/owner`, coordinator, or admin
  provisioning surfaces (→ SVC_WORKFLOW_DOMAIN_OWNER_DELEGATION_V1).
- No DOMAIN_MEMBER read-visibility change
  (`DOMAIN_MEMBER_VISIBILITY_UNCHANGED=true` stands).
- No new table, migration, shadow store, or second ACL; the only storage
  interaction is the existing `domain_role_bindings` upsert plus one new
  pre-check read.

## 6. Acceptance criteria

- ACC-DMC-001: add without body / with `role:"DOMAIN_MEMBER"` grants
  member; response carries the granted role.
- ACC-DMC-002: re-add with a NEW key while enabled ⇒ 409 `already_member`;
  exactly one enabled binding; zero new success audit rows.
- ACC-DMC-003: same-key replay returns the stored original outcome;
  same-key different-role ⇒ 409 `idempotency_conflict`.
- ACC-DMC-004: `role:"DOMAIN_OWNER"` ⇒ 403
  `domain_owner_delegation_forbidden`; no binding change; replay-stable.
- ACC-DMC-005: unknown role / unknown body field ⇒ 400 `invalid_input`.
- ACC-DMC-006: non-owner add/remove/list ⇒ 403 `not_domain_owner`;
  unknown domain ⇒ 404; unregistered target ⇒ 404 `principal_not_registered`;
  remove nonexistent binding ⇒ 404 `member_not_found` (regression guards).
- ACC-DMC-007: contract mirrors (errors.json/openapi/contract.md) updated;
  tests/18 suite extended with the above; full offline suite green.

## 7. Sequencing

1. This Spec accepted (docs-only PR) ⇒ implementation authority.
2. Implementation PR (code + tests + contract mirrors), citing this Spec.
3. dsh consumer (AGENT_CORE_WORKFLOW_DOMAIN_MEMBERS_BROKER_V1) may land in
   parallel; production deploy of either is slot-gated and NOT authorized
   by this Spec (`production_apply_authority: none`).
