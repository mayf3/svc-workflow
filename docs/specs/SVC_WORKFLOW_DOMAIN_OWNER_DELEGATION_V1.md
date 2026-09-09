---
spec_id: SVC_WORKFLOW_DOMAIN_OWNER_DELEGATION_V1
status: proposed
spec_kind: contract-delta
authority_level: governing_spec_candidate
implementation_authority: none
production_apply_authority: none
date: 2026-09-09
scope:
  - the DOMAIN_OWNER delegation question raised by the
    WORKFLOW_DOMAIN_MEMBERS_CONTROL_PLANE goal directive §7
    (DOMAIN_OWNER_DELEGATION_SEMANTICS_REQUIRED): may a DOMAIN_OWNER grant
    or revoke DOMAIN_OWNER through the member-management surface?
  - THIS SPEC PROPOSES ONLY. It authorizes no implementation, no
    migration, and no production change until accepted.
governed_by:
  - docs/architecture/AUTH_PRINCIPAL_SELF_PROJECTION_AND_DOMAIN_MEMBERSHIP_V1.md
    (FROZEN) — the authority whose `DOMAIN_OWNER_CAN_MANAGE_DOMAIN_OWNER=false`
    this candidate would amend
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3/svc-workflow maintainers
---

# SVC_WORKFLOW_DOMAIN_OWNER_DELEGATION_V1

## 1. Ownership semantic audit (required by the goal directive §7)

Answers against main 4bbbbe9 (code + migration 0001 + production data):

### Q1 — Does a domain allow multiple enabled DOMAIN_OWNERs?

**No.** `migrations/0001_identity_domain.sql` defines the partial unique
index `idx_drb_single_owner ON domain_role_bindings (domain_id, role_key)
WHERE enabled = TRUE AND role_key = 'DOMAIN_OWNER'` — at most one ENABLED
owner binding per domain. Multiple DISABLED owner rows are legal (audit
trail). Production `workflow-todo-dogfood` shows exactly this shape: 4
historical owner rows, 1 enabled.

### Q2 — Is DOMAIN_OWNER an ordinary role binding?

It is stored as a `domain_role_bindings` row like any role, BUT it carries
the single-owner invariant above plus a separate replacement contract —
so it is NOT an ordinary multi-tenant role; it is a uniquely-constrained
binding with coordinator-mediated transfer.

### Q3 — Does the `/owner` contract mean only GLOBAL_WORKFLOW_COORDINATOR
(or admin provisioning) replaces owners?

**Yes.** Two surfaces write owner bindings today:
`PUT /internal/v1/domains/{domainId}/owner` (agent-facing; requires
`workflow.execute` + direct token + enabled GLOBAL_WORKFLOW_COORDINATOR
global binding; `replace_domain_owner` disables every other enabled owner
row and enables/upserts the target in ONE transaction) and the admin
`PUT /internal/v1/admin/domains/{domainId}/owner` (provisioning
allow-list). A DOMAIN_OWNER cannot replace anyone, including themselves.

### Q4 — Would DOMAIN_OWNER granting DOMAIN_OWNER change the privilege
delegation model?

**Yes.** It moves owner-grant authority from a globally-scoped coordinator
role to per-domain self-service delegation — a real trust-boundary change
(same class as the frozen `DOMAIN_OWNER_CAN_MANAGE_DOMAIN_OWNER=false`
ruling), and the reason this is a Spec + Owner gate instead of code.

### Q5 — Are there last-owner / self-removal invariants today?

There is no DOMAIN_OWNER removal path at all (member-remove never touches
owner rows; replacement is the only owner lifecycle writer), so
"last owner" is currently impossible by construction. Any delegation
contract that enables owner add/remove MUST add an explicit last-owner
guard (below).

## 2. The proposed contract delta (IF accepted)

Minimal multi-owner semantics on the member-management surface:

1. **Multi-owner:** drop the single-owner partial unique index in favor of
   the ordinary `(domain_id, principal_id, role_key)` uniqueness; owner
   replacement via coordinator `/owner` remains available and still works
   (its "disable all other enabled owners" step becomes "disable only the
   previous owner when the intent is replacement" — see migration note).
2. **Grant:** `DOMAIN_OWNER` may grant `DOMAIN_OWNER` in their own domain
   only through `PUT members/{principalId}` with `role:"DOMAIN_OWNER"`
   (replacing CTR-DMC-002's forbidden outcome). Everything else (scopes,
   direct-token, receipts, audit) identical to member add.
3. **Revoke:** `DELETE members/{principalId}?role=DOMAIN_OWNER` disables
   exactly that owner binding; `role` becomes an explicit parameter on
   remove at that point (no guessing, no broad delete).
4. **Last-owner invariant:** a revoke that would leave the domain with
   ZERO enabled DOMAIN_OWNER bindings is rejected 409
   `last_domain_owner` — the domain must always have ≥1 enabled owner;
   transferring away is coordinator `/owner`'s atomic job.
5. **Self-removal:** permitted ONLY when another enabled owner remains
   (subsumed by the last-owner guard); no special-casing.
6. **Migration/backward compatibility:** one migration (drop partial
   index — no data change; every domain already has exactly one enabled
   owner, which remains valid); coordinator `/owner` semantics preserved
   by keeping "disable previous enabled owner(s)" as its documented
   replacement behavior (it remains the atomic transfer path); admin
   provisioning unchanged; `domain_owner_conflict` (409) on the admin
   second-owner provisioning route is retired ONLY if redundant — audit
   first, then remove.
7. **Audit/security:** owner grants/revokes audit as
   `DOMAIN_MEMBERSHIP` mutations with the role in details (same table,
   no new format); risk accepted explicitly: a compromised owner can
   escalate peers within their own domain only — global blast radius
   unchanged (cross-domain still needs each domain's owner or the
   coordinator).

## 3. Relationship to the frozen architecture doc

This candidate, once accepted, amends exactly two decisions of
AUTH_PRINCIPAL_SELF_PROJECTION_AND_DOMAIN_MEMBERSHIP_V1:
`DOMAIN_OWNER_CAN_MANAGE_DOMAIN_OWNER` false→true (bounded by §2 above)
and the single-owner invariant; everything else in that document stands.
Until acceptance, SVC_WORKFLOW_DOMAIN_MEMBERSHIP_CONTROL_PLANE_V1
CTR-DMC-002 (403 `domain_owner_delegation_forbidden`) is the live
contract and MUST NOT be relaxed by implementation.

## 4. Acceptance criteria (post-acceptance, future PR)

- Owner grants DOMAIN_OWNER in own domain ⇒ success + audit; non-owner ⇒
  403; cross-domain ⇒ 403.
- Revoke with remaining owner ⇒ success; revoke-to-zero ⇒ 409
  `last_domain_owner`; self-revoke guarded identically.
- Coordinator `/owner` still atomically transfers (regression).
- Migration applies with zero data change; rollback restores the partial
  index.
