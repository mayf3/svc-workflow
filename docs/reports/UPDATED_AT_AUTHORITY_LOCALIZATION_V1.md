# UPDATED_AT Authority Localization & SPEC_GAP Record V1

```text
DATE = 2026-10-27
ROUND = DOCS_ONLY_AUTHORITY_AND_INTEGRATION_PREPARATION (no product source/test/runtime change; no merge; no deploy)
TASK_LINE = business 7cd02846-67e1-4458-a096-297a7c8fd32f under formal parent 8121ef1d-0138-4431-b4f8-95dd86b24fe5
ORIGINAL_BUSINESS_DEADLINE = 2026-10-26T01:15:17.944209Z (overdue; retained, not reset)
BASE_REVISION_OF_THIS_RECORD = main bfc28c15320acab8cf7d7a36128cd5c9db9a1b1a (fresh ls-remote verified 2026-10-27)
WRITER = #1301 sole source writer (same session; no second writer; no new parent workflow)
```

## 1. DEVELOPMENT_PREFLIGHT (recorded before authoring the new docs)

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
TARGET_REPOSITORY = mayf3/svc-workflow
REVIEW_TARGET_HEAD = docs-only branch docs/updated-at-authority-prep-v1 (base bfc28c1)
BASE_HEAD = bfc28c15320acab8cf7d7a36128cd5c9db9a1b1a (fresh main; PR #85 head e068dd0 is ahead 9 / behind 14 — whole-branch merge and cherry-pick both ruled out by reviewer instruction)
CURRENT_BASE_HEAD = bfc28c15320acab8cf7d7a36128cd5c9db9a1b1a
ROUTE_STAGE = AUTHORITY_AUTHORING
AUTHORITY_ACCEPTED_IN_BASE = NO (the authority this round needs does not exist yet; this PR only proposes it)
GOAL_OR_TARGET = locate active authority for workflow_instances.updated_at write maintenance; if absent, author a bounded proposed Spec and a main-bound minimal integration plan
CURRENT_GAP = no active accepted authority owns the updated_at maintenance obligation for ordinary ADVANCE/RETURN/TERMINATE/revise/combined paths
OBSERVATIONS = see §2–§4 below (all quoted at exact path+line on bfc28c1)
WORKING_GUESS = NOT_APPLICABLE (routing determined by direct text)
AUTHORITY_ACTION = NEW
PRIMARY_AUTHORITY (proposal governed by) = SVC_WORKFLOW_PRODUCT_BOUNDARY_V8@bfc28c1 (accepted, implementation_authority: none), SVC_WORKFLOW_ARCHITECTURE_V0_4_3@bfc28c1 (accepted, implementation_authority: none)
RELATED_AUTHORITIES = SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2 (accepted, contracts — identity-only; basis of PR #85, unchanged); SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1 (accepted, contracts, one-time-migration scope); ADMIN_RECOVERY_CONTRACT_V0_1 (docs/contracts legacy, self-declared CURRENT); superseded V2–V5 (historical provenance only)
IMPLEMENTATION_AUTHORITY = none until the proposed Spec is accepted (its frontmatter targets implementation_authority: contracts)
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = NO (implementation deliberately excluded from this round; docs-first)
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = DURABLE
EXECUTION_MANDATE = VALID (handoff instruction 2026-10-27, same task line, docs-only scope, controller single-writer preserved, no renumbering)
MUTATION_AUTHORIZATION = VALID (docs-only files on a dedicated docs branch)
ISOLATED_WRITE_SURFACE = YES (worktree ac-1301-docs-main @ bfc28c1; user dirty root untouched; prior #1301 worktrees untouched)
CONTROLLED_RUNBOOK_REQUIRED = NO (no operation performed)
SPEC_GAP_DEPENDENCY = LOAD_BEARING (implementation on main depends on the missing authority)
EVIDENCE_REVIEWABILITY = PASS (all citations in-repo at fixed revision; hashes recorded)
LIVE_AUTHORITY_GAP = DETECTED
OWNER_DECISION_REQUIRED = YES (Spec acceptance belongs to mayf3 per SPEC_ACCEPTANCE_ACTORS; writer does not self-accept)
EMERGENCY_STATE = NONE
BASE_IMPACT = NONE (docs-only)
IMPLEMENTATION_ALLOWED = NO (this round)
MERGE_READY = NO (draft PR; review + owner acceptance required)
OPERATION_ALLOWED = NO
EVIDENCE_NEEDED = in-repo citations (§2–§4); future RED/GREEN on the main-bound fix branch after acceptance (NOT executed this round; no new-main-PASS claimed)
DONE_WHEN = authority localization + proposed unaccepted Spec + minimal integration plan delivered as reviewable docs-only candidate
EXPANSION_TRIGGER = any move to implement, self-accept, or claim production success — NOT FIRED
NEXT_REAL_ACTION = independent review of this docs PR; owner acceptance decision on the proposed Spec
NEXT_ACTION = OWNER_DECISION
```

## 2. Active-authority narrow verification (bfc28c1)

Checked every `updated_at`/`updatedAt` mention in `docs/` at bfc28c1. Results:

| Location | What it is | Active authority for instance updated_at write maintenance? |
|---|---|---|
| `docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V8.md` ~:992 (and V7:548, V6:490) | `updatedAt` as a field of the **Scheduler-facing Dispatch Intent record** | **NO — different object** (DispatchIntent.updatedAt, fed by activation semantics) |
| `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_4_3.md` :1118 (§5.7) | Dispatch Intent minimum projection field list | NO (same Dispatch Intent object; architecture is `implementation_authority: none`) |
| `docs/product/SVC_WORKFLOW_PRODUCT_BOUNDARY_V5.md` :451; V4 :346,:1133; V3 :281,:831; V2 :208 | **Instance-projection** `updatedAt` semantics: "the latest committed Workflow Instance state change represented by the current authoritative projection — not read, cache, scheduler, audit, Assistance, or external-system time"; V3/V4 failure condition: "`updatedAt` uses another clock" | **NO — superseded authorities** (V2→V3→V4→V5→V6→V7→V8; the sentence does not survive into V6/V7/V8, whose scheduler surface is Dispatch Intent). Recorded as historical provenance of the convention only. |
| `docs/specs/SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1.md` :333 (accepted, `implementation_authority: contracts`, scope: one-time-principal-successor-migration) | step 5: "CAS-update only `workflow_instances.current_node_visit_id`, `workflow_state_version = old + 1`, and `updated_at`" | Scope-bounded precedent (one-time migration path only); supports the convention; does not own ordinary paths |
| `docs/contracts/ADMIN_RECOVERY_CONTRACT_V0_1.md` :125 (legacy, self-declared CURRENT, v0.3.1-era) | rebuild "只允许更新 Instance 三个投影字段和 `updated_at`" | Scope-bounded precedent (admin recovery rebuild path only) |
| `docs/specs/SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1.md` :233–236; `docs/specs/SVC_WORKFLOW_EXECUTION_CONTROL_V1.md` :81; `docs/specs/SVC_WORKFLOW_COORDINATOR_CONTROL_PLANE_V1.md` :188,:323; `docs/contracts/DEFINITION_SERVICE_CONTRACT_V0_1.md` :165; `docs/contracts/LEGACY_IMPORT_CONTRACT_V0_1.md` :55 | updated_at of **other objects** (activation records, execution-control tables, domains, definition versions, legacy import payload) | NO — different objects |
| `SVC_WORKFLOW_CANONICAL_IDENTITY_RECONCILIATION_V2` CTR-CIR-004/005 | identity-only revise predicates (creator revise; RETURN→creator revise→ADVANCE) | Supports operability of revise (basis of PR #85) but grants **no** updated_at semantics |

## 3. Implementation-state facts on main (bfc28c1, observation not authority)

Instance-row UPDATE sites in `src/store/postgres/workflow_instance_repository/` at bfc28c1:

| Path | Maintains `updated_at = now()` | Evidence (file:line at bfc28c1) |
|---|---|---|
| cancel | YES | cancel_transaction.rs:409–415 |
| archive | YES | archive_transaction.rs:343–348 |
| assistance activation | YES | assistance_transaction.rs:339–340 |
| execution escalation | YES | execution_escalation.rs:238–239 |
| wake/push-kick | YES | wake_transaction.rs:434–435 |
| admin recovery rebuild/override | YES | admin_recovery_repository/rebuild_transaction.rs:233, override_transaction.rs:504 |
| transition (ADVANCE/RETURN/TERMINATE) | **NO** | transition_helpers.rs:252 (update_instance) |
| combined revise+transition | **NO** | combined_helpers.rs:118 |
| revise-only | **NO** | revise_transaction.rs:481–483 |
| context repair | **NO** | repair_transaction.rs:531–533 (4th gap; present identically on candidate head e068dd0 — the 2c135ef fix did not touch it) |

The current convention in executable code is already "maintain on real state change" (6 of 10 sites), but no active accepted authority states it; the four gaps are the 7cd02846 defect surface plus the repair path.

## 4. Determination

```text
SPEC_GAP_DEPENDENCY = LOAD_BEARING
AUTHORITY_ACTION = NEW
PROPOSAL = docs/specs/SVC_WORKFLOW_INSTANCE_UPDATED_AT_MAINTENANCE_V1.md (status: proposed, NOT accepted, in this docs-only PR)
READINESS = implementation on main is FORBIDDEN until the proposal is independently reviewed and accepted by an authorized acceptance actor
NOT_AUTHORITY = user goals, investigations, PR descriptions, the historical 2c135ef implementation, or superseded V2–V5 text — none may stand in for the missing active authority
```

Per the local authority precedence and the forward-only existing-authority bridge, the superseded V2–V5 sentences and the two scope-bounded precedents are cited as the convention's provenance and consistency anchors, not as implementing authority.
