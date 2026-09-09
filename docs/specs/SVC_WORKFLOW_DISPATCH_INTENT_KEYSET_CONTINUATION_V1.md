---
spec_id: SVC_WORKFLOW_DISPATCH_INTENT_KEYSET_CONTINUATION_V1
status: accepted
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
production_apply_authority: none
date: 2026-09-09
accepted_date: 2026-09-09
acceptance_authority_basis: >-
  Owner exact-head acceptance (PR mayf3/svc-workflow#33, ACCEPT=YES at
  67a092b98984b740eeb03d5534b73ed2d206bb44 on current main authority V7 /
  Architecture v0.4.1, second gate after the first REVISE round closed
  B-O2); prior chain: independent semantic review REVISE -> fix -> delta
  re-review ACCEPT_READY. This commit is the lifecycle transaction only: the
  accepted contract text is byte-identical to the accepted head except this
  frontmatter.
accepted_reviewed_head: 67a092b98984b740eeb03d5534b73ed2d206bb44
scope:
  - svc-workflow due Dispatch Intent read (GET /internal/v1/dispatch-intents)
  - keyset continuation only; no scheduler semantics
governed_by:
  - SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1 (accepted on main; amends
    CTR-VAI-009; all other CTR-VAI contracts unchanged)
  - SVC_WORKFLOW_PRODUCT_BOUNDARY_V7 (current main authority)
  - SVC_WORKFLOW_ARCHITECTURE_V0_4_1 (current main authority)
external_authorities:
  - dsh-agent-core AGENT_CORE_WORKFLOW_AGENT_EXECUTION_V1 (candidate, same
    closure round) — the sole motivating consumer; its CTR-WAE-001b consumes
    this contract
supersedes: []
superseded_by: null
owners:
  - mayf3/svc-workflow maintainers
---

# SVC_WORKFLOW_DISPATCH_INTENT_KEYSET_CONTINUATION_V1

## 1. Goal

```text
GOAL = make the due Dispatch Intent feed exhaustible past the first LIMIT
       window WITHOUT adding any scheduler/pagination-platform semantics
SUCCESS_OUTCOME = a consumer holding GLOBAL_SCHEDULER_READ can, by passing the
       keyset cursor of the last item it saw, continue reading the SAME
       ordered feed until exhaustion; callers that send no cursor observe
       byte-identical CTR-VAI-009 behavior
BLOCKER_BEING_CLOSED = confirmed starvation: with >100 due intents where the
       first 100 already have execution attempts (still due, not yet
       transitioned), the server re-returns the same first window every poll
       and intents 101+ are unreachable (required by Owner ruling B2,
       2026-09-09; scoped by the goal's minimal continuation exception)
```

This is a read-path amendment to exactly ONE endpoint. It adds NO workflow
state, NO retry, NO lease, NO dispatcher, NO priority/fairness, NO event
bus, NO schema/migration, NO role or grant change.

## 2. Contracts

### CTR-DKC-001 — keyset continuation query parameters

`GET /internal/v1/dispatch-intents` gains exactly TWO OPTIONAL query
parameters:

```text
afterNextEligibleAt   RFC3339 timestamp string (exact round-trip of a
                      nextEligibleAt value previously returned by this
                      endpoint; consumers MUST pass the byte-exact string,
                      never a reformatted one)
afterDispatchIntentId UUID (a dispatchIntentId previously returned by this
                      endpoint)
```

- BOTH-OR-NEITHER: exactly one of the two present ⇒ 422 `invalid_pagination`
  (same code and family as the existing limit violation; no new error code).
- Malformed values (unparseable RFC3339 timestamp, non-UUID id) ⇒ 422
  `invalid_pagination`.
- Absent both ⇒ behavior is BYTE-IDENTICAL to CTR-VAI-009 (existing
  consumers unaffected; verified by an unchanged-response snapshot test).
- Error precedence: cursor validation (422) is handler-side and precedes the
  in-snapshot role check (403) — "with cursor parameters" in ACC-DKC-006
  means VALID cursors. The extractor takes the cursor params as strings and
  parses them in-handler (typed `Option<DateTime>` fields would surface axum
  extraction rejections as 400, not the mandated 422).

### CTR-DKC-002 — selection and ordering (cursor key == order key)

The result set is the CTR-VAI-009 due set (unchanged predicate: active
DISPATCH_INTENT activation, no closure row, instance not cancelled/archived,
current nextEligibleAt ≤ authoritative now) INTERSECTED with the exclusive
keyset filter:

```text
(current nextEligibleAt, activation_id) > (afterNextEligibleAt, afterDispatchIntentId)
```

as a SQL row-value comparison, where `current nextEligibleAt` is the SAME
COALESCE(latest eligibility event new_next_eligible_at,
initial_next_eligible_at) expression that the existing ORDER BY uses (order
key and cursor key are one expression identity, evaluated in the same
REPEATABLE READ snapshot as the existing role check + query). ORDER BY
remains `(current nextEligibleAt, activation_id)`; `LIMIT` remains 1..100
(default 50); the projection remains EXACTLY the CTR-VAI-009 seven fields.
Timestamp comparison happens on parsed instants (server parses the RFC3339
cursor string; no string comparison).

### CTR-DKC-003 — no platform pagination

The request accepts NOTHING else new and the response gains NO field: no
page, no offset, no count/totalCount, no totalPages, no opaque cursor token,
no hasMore. Exhaustion is signaled ONLY by `items.length < limit` (consumer
rule, restated from the external consumer contract).

### CTR-DKC-004 — feed safety under concurrent writes (the actual invariant)

The load-bearing guarantees, stated exactly (verified against the current
writers):

1. An already-RETURNED due row's key NEVER moves backward. The only
   value-changing writer today, WAKE, is a durable no-op (`ALREADY_DUE`)
   on intents whose current nextEligibleAt ≤ server now — i.e. on every
   row the due feed can return. Already-returned rows are therefore never
   re-exposed with a different key, and never mutated by wake.
2. Entries and moves land at/after the cursor EXCEPT three bounded cases,
   each of which leaves the intent DUE and caught by the NEXT (cursorless,
   hence exhaustive) sweep:
   - a not-yet-due deferred intent woken mid-sweep: applied wake writes
     new = wake-transaction now() < previous (this is the only
     earlier-than-before move, and it can only ENTER the due set); if the
     wake transaction's now() predates the cursor row's stored value but
     commits between pages, the intent sorts before the cursor for this
     sweep;
   - a brand-new activation created mid-sweep: `initial_next_eligible_at`
     = the CREATION transaction's now() (in-tx start), so a
     long-running creation transaction committing mid-sweep can sort
     before the cursor;
   - an exact-timestamp tie with a smaller `activation_id`.
3. Closures, cancel and archive only shrink the due set.

Conclusion: a sweep with continuation is exhaustion-complete up to those
bounded mid-sweep cases; every sweep restarts cursorless and is therefore
exhaustive; nothing is lost. (The external consumer additionally dedupes by
its one-attempt fence, so repeated observations are harmless.) No locking
beyond the existing per-request REPEATABLE READ snapshot is added.
(SCHEDULER_DEFER is a reserved cause class with no writer today; any future
defer authority must preserve guarantee 1 and is out of scope here.)

### CTR-DKC-005 — boundaries kept

- Authorization unchanged: `workflow.read` scope + enabled
  GLOBAL_SCHEDULER_READ binding verified server-side inside the same
  snapshot; missing gate = 403 `scheduler_read_role_required` (CTR-VAI-009
  semantics, byte-identical).
- Zero writes: the endpoint remains read-only; no wake, no eligibility
  mutation, no audit-row semantics change.
- No new workflow/activation/assistance state; no scheduler policy; no
  Grant/role creation.
- This Spec amends ONLY CTR-VAI-009's parameter surface; every other
  CTR-VAI-001..014 contract of SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1 stands
  unchanged.

## 3. Acceptance criteria (mechanical)

- ACC-DKC-001 — no-cursor behavior byte-identical to CTR-VAI-009 (response
  snapshot diff on an identical fixture).
- ACC-DKC-002 — keyset paging reaches exhaustion across ≥ 3 windows with
  stable order and zero duplicates/zero skips against a static due set.
- ACC-DKC-003 — equal-`nextEligibleAt` ties break by `activation_id` in both
  ORDER BY and the keyset filter (tie fixture).
- ACC-DKC-004 — half-cursor and malformed-cursor requests return 422
  `invalid_pagination` with zero rows read beyond validation.
- ACC-DKC-005 — STARVATION PROOF: fixture with > 100 due intents; reading
  window 1 (no cursor, limit 100) then window 2 (cursor = exact last item of
  window 1) returns intents 101+ — i.e. with the first 100 already consumed
  by a client, intent 101+ is reachable ONLY via the continuation (this is
  the server-side half of the external consumer's starvation invariant).
- ACC-DKC-006 — role gate unchanged: role-less caller (including
  GLOBAL_WORKFLOW_READER holders) gets 403 `scheduler_read_role_required`
  with and without cursor parameters.

## 4. Test gate

The implementation candidate ships with the existing suite green plus
focused tests covering ACC-DKC-001..006 (integration tests against the
run-scoped test database, following the repo's existing test-28 pattern).

## 5. Implementation note (non-normative)

Natural seam: `parse_due_limit` grows into a cursor-aware query parser in
`query_dispatch_intents.rs`; the SQL gains the row-value filter reusing the
COALESCE expression; handler auth path untouched. No migration.

## 6. Sequencing

Authored as a docs-only candidate on CURRENT MAIN (base 5e37d9a, the
authority branch — main already contains the accepted Visit Activation
lineage and has advanced to Product Boundary V7 / Architecture v0.4.1; an
earlier candidate revision sat on the pre-merge impl/visit-activation-v1
branch @ 6191ce4 and was rebased per Owner exact-head review B-O2). Narrow
impact recheck at rebase time: the due-feed surface this Spec amends is
byte-identical between 6191ce4 and current main
(`query_dispatch_intents.rs`, `handlers/dispatch_intents.rs`,
`wake_transaction.rs` — empty diff), so every CTR fact verified against the
impl-branch code holds verbatim on main. Per repo governance, implementation
begins only after this Spec is accepted and present in the implementation PR
base. The external dsh consumer deploys only after this contract is
accepted, implemented, and deployed. At acceptance/merge time, the accepted
SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1.md gains a reciprocal amended-by
backlink on CTR-VAI-009 (this candidate does not modify the accepted spec
file itself).
