# WORK_ELIGIBILITY_PROJECTION_REVIEW_RECORD_V1

- reviewer: independent read-only reviewer (mechanical verification from repository)
- date: 2026-09-05
- review target: worktree `/Users/yanfenma/workspace/worktrees/svc-work-eligibility-projection`, branch `codex/work-eligibility-projection-v1`
- base verified: `c4f1fa8d9bae7c91d9cc09751cfa8e2195c3911a` (github/main), impl commit `cfb0134`, spec commit `d1ec1dc`, HEAD = d1ec1dc, working tree clean
- spec reviewed: `docs/specs/SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1.md` (status: proposed)

## VERDICT: FAIL

Exactly one blocker (serde wire shape, checkpoint 3). Everything else verified clean:
classifier semantics, SQL derivation, test suites, boundary discipline, and spec
metadata all conform. The blocker is locally remediable (one enum-variant shape
change in `eligibility.rs` plus the matching spec §3 sentence, or the inverse:
drop the `content` attribute and amend spec §3 + variant docs). Re-review after
remediation should be sufficient; no other finding invalidates the derivation.

## BLOCKER_UNION

### B-1 — `WorkEligibility::WaitingForTime` is a unit variant: the declared `nextEligibleAt` content is never serialized AND is rejected on deserialization

- Location: `src/application/workflow_instance/eligibility.rs:25-33` (derive line 25; both variants unit, lines 30 and 33) vs. spec §3 `docs/specs/SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1.md:83`.
- Spec contract (§3, line 83): the enum has "serde SCREAMING_SNAKE_CASE with `nextEligibleAt` content for the waiting case".
- Empirical wire check (throwaway integration test, run then deleted; tree left clean):
  - `serde_json::to_string(WorkEligibility::ActionableNow)` → `{"classification":"ACTIONABLE_NOW"}`
  - `serde_json::to_string(WorkEligibility::WaitingForTime)` → `{"classification":"WAITING_FOR_TIME"}` — **no `nextEligibleAt` key ever appears**. The `content = "nextEligibleAt"` attribute on line 25 is dead: an adjacently-tagged unit variant emits tag only.
  - Deserialization of a payload that DOES carry the spec-promised content — `{"classification":"WAITING_FOR_TIME","nextEligibleAt":"2026-09-05T12:00:00Z"}` — **fails**: `Error("invalid type: string \"2026-09-05T12:00:00Z\", expected unit variant WorkEligibility::WaitingForTime", line 1, column 76)`.
- Why it blocks: the spec's own §3 wire contract is neither produced nor accepted by the type. The dispatcher's declared consumer benefit (knowing WHEN waiting work becomes actionable — the effective instant the classifier computed) silently disappears on the wire, and any producer/consumer implementing spec §3 literally will be rejected by the deserializer. This is exactly the "content on a unit variant is unserializable or unexpected" case defined as a blocker in the review mandate; automatic FAIL per mandate.
- Remediation options (owner's choice, both small):
  1. Make the waiting case carry the instant: `WaitingForTime(chrono::DateTime<Utc>)` (with a serde-compatible chrono serialization), have `classify` return it, and thread the value through the three mapping sites (`DomainInstanceRow::eligibility`, `GlobalInstanceRow::eligibility`, `QueryBaseRow::current_visit_eligibility`); or
  2. Drop the `content = "nextEligibleAt"` attribute (plain `#[serde(rename_all = "SCREAMING_SNAKE_CASE")]`, tag-only or internally-tagged) and amend spec §3 + the variant doc comment to match.
- Note honestly recorded: the classification-only wire form the type ACTUALLY emits does round-trip cleanly (`{"classification":"WAITING_FOR_TIME"}` deserializes back to `WaitingForTime`). The failure is the broken declared contract, not corruption of the emitted form.

## NON_BLOCKING findings

1. **Timeline vs current-visit inconsistency inside one detail surface.** `list_node_visits` (the historical timeline, `query_detail.rs:330`) stamps every visit — including the CURRENT visit, which is the last element of the timeline — unconditionally `ACTIONABLE_NOW` (`query_rows.rs:283-289`, `VisitRow::into_item`), while `FullWorkflowInstanceDetail.current_visit` (same instance-detail payload, via `QueryBaseRow::current_visit`, `query_rows.rs:137-148`) carries the true classifier result. A consumer reading the visits array could see `ACTIONABLE_NOW` for a visit that `current_visit` reports `WAITING_FOR_TIME`. This is disclosed in spec §5 ("never on the historical timeline") and is semantically defensible (timeline = history; only current-work surfaces carry dispatch eligibility), but it is a real intra-surface inconsistency a broker consumer can trip on. Suggest a doc note in the variant comment of `NodeVisitItem.eligibility` (query_types.rs) stating that timeline items are always `ACTIONABLE_NOW` by definition — the comment says "Terminal visits classify as ACTIONABLE_NOW", which does not cover the non-terminal current visit appearing in the timeline.
2. **Clock-at-mapping-time.** Classification uses `chrono::Utc::now()` at row-mapping time (`query_rows.rs:141`, `query_domain_instances.rs`/`query_global_instances.rs` `eligibility()`), not the transaction snapshot clock. Spec §5 discloses the microsecond due-straddle; acceptable for a read projection.
3. **Alias coupling of the shared SQL fragment.** `ELIGIBILITY_FACT_JOINS` hard-codes the caller's visit alias (`v.` in both list queries, `nv.` in `load_base` — the fragment is duplicated inline at each site rather than string-interpolated, so each site spells the join itself; all three verified correct today). The doc comment in `eligibility.rs:63-66` documents the contract. Fragile under future refactors; no action required now.
4. **Closed-activation unit test models the post-JOIN view.** `closed_activation_is_actionable_now` (eligibility.rs:109-113) feeds `open=false` — correct, since the SQL excludes closed activations before classification; worth noting the test documents the JOIN-classifier contract, not standalone classifier behavior.

## Per-checkpoint evidence

### Checkpoint 1 — Migration 0023 schema (PASS)

`migrations/0023_visit_activation_v1.sql`:
- `uq_activation_node_visit UNIQUE (node_visit_id)` — line 99. Exactly one activation per visit.
- CHECK constraint lines 102-106: `initial_next_eligible_at IS NOT NULL` iff `activation_kind = 'DISPATCH_INTENT'`; NULL iff `HUMAN_WORK_ITEM`. Matches spec §2 "timerless by schema CHECK" and the fail-open justification (open DISPATCH_INTENT with NULL effective instant is impossible under schema, only data anomaly).
- `workflow_activation_closures` lines 116-125: rows mark closed activations (`activation_id` PK, `closure_reason` 1..128). The `NOT EXISTS` closure subquery in the impl matches this table's semantics.
- `workflow_dispatch_eligibility_events` lines 136-146: `eligibility_event_id UUID NOT NULL PRIMARY KEY`, `new_next_eligible_at TIMESTAMPTZ NOT NULL`, `created_at TIMESTAMPTZ NOT NULL DEFAULT now()`. Index line 148-149 on `(activation_id, created_at, eligibility_event_id)` exactly serves the LATERAL's `ORDER BY created_at DESC, eligibility_event_id DESC` — index-aligned.
- "effective instant = latest event else initial" cross-checked against `src/store/postgres/workflow_instance_repository/activation_facts.rs` header doc (lines 10-11): eligibility-event rows are "the only writers of later `nextEligibleAt` values"; `current_next_eligible_at` derivation in the same file (lines 186-223) uses the same latest-event-else-initial rule. Spec's effective-instant definition is faithful to the facts, no reinterpretation.

### Checkpoint 2 — Accepted external authority (PASS)

`docs/specs/SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V1.md`: `status: accepted`, `accepted_date: 2026-09-02`, spec_kind implementation, authority_level governing_spec. The projection reads its facts (activation/closure/eligibility-event tables) and redefines nothing: `EligibilityFactRow`/`classify` consume only columns those contracts author; `activation_facts.rs` cites the same authority ("accepted v0.4.0 §5.7-5.9"). §5.7-5.9 verified present in `docs/architecture/SVC_WORKFLOW_ARCHITECTURE_V0_4_0.md` (lines 720 "Canonical activation facts and projections", 778 "Canonical activation timestamp and wait semantics", 814 "Command serialization and atomic closure"). Spec also cites accepted SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1 as the global-list authorization (file present in docs/specs).

### Checkpoint 3 — Serde wire check (FAIL → BLOCKER B-1)

Empirical throwaway test (created, run, deleted; `git status` clean after). Results:
- `ActionableNow` → `{"classification":"ACTIONABLE_NOW"}`; round-trips.
- `WaitingForTime` → `{"classification":"WAITING_FOR_TIME"}`; round-trips its own form, but the spec-promised `nextEligibleAt` content is absent, and a payload carrying the content fails deserialization ("invalid type: string, expected unit variant").
- Verdict per mandate: automatic FAIL. See BLOCKER B-1.

### Checkpoint 4 — Test suites (PASS)

- `cargo test --lib eligibility`: **7/7 ok** (`legacy_no_activation_row_is_actionable_now`, `closed_activation_is_actionable_now`, `human_work_item_has_no_timer_and_is_actionable`, `due_dispatch_intent_is_actionable_now`, `waiting_for_time_carries_future_instant`, `due_at_exact_instant_is_actionable`, `no_effective_instant_with_open_dispatch_intent_is_actionable`) — covers all five §2 table rows + exact-due boundary + fail-open, satisfying ACC-001.
- Full `cargo test` on impl HEAD: **170 passed, exactly ONE failure: `upgrade_0012_to_0014_succeeds`** (00_upgrade_verification).
- A/B: temp worktree at pristine `c4f1fa8` (`/private/tmp/svc-eligibility-base-verify`, removed after run): `cargo test --test 00_upgrade_verification` → same single test FAILED with the identical panic: `connect to PostgreSQL administration database: Database(PgDatabaseError { severity: Fatal, code: "28P01", message: "password authentication failed for user \"postgres\"" ... })` at `tests/00_upgrade_verification.rs:46`. Env-dependent (local PostgreSQL credentials), pre-existing, not introduced by this diff. ACC-002 verified.

### Checkpoint 5 — SQL correctness (PASS)

All three sites (`query_domain_instances.rs` list query, `query_global_instances.rs` list query, `query_visibility.rs` `load_base`):
- (a) Join alias: both list queries `JOIN workflow_node_visits v ON v.node_visit_id = wi.current_node_visit_id` and the eligibility join uses `a_open.node_visit_id = v.node_visit_id` — the CURRENT visit. `load_base` names its visit `nv` (`LEFT JOIN workflow_node_visits nv ON nv.node_visit_id = wi.current_node_visit_id`) and joins `a_open.node_visit_id = nv.node_visit_id`. Correct per-query aliasing.
- (b) Closure exclusion: all three use `AND NOT EXISTS (SELECT 1 FROM workflow_activation_closures c WHERE c.activation_id = a_open.activation_id)` inside the LEFT JOIN condition — a closed activation makes the join produce no row → all facts NULL → classifier sees "no open activation". Correctly excludes closed activations while preserving the LEFT JOIN's no-row = legacy/terminal semantics.
- (c) LATERAL: `ORDER BY e.created_at DESC, e.eligibility_event_id DESC LIMIT 1` — `eligibility_event_id` is the table PK (migration line 137), giving a total deterministic order; matches the index `(activation_id, created_at, eligibility_event_id)`. Deterministic latest-event pick. Note: `created_at` defaults to `now()` (transaction timestamp, constant within a transaction), so event_id tiebreak matters only across transactions in the same microsecond — sound.
- (d) Column aliases: `activation_kind`, `open_activation_id`, `effective_next_eligible_at` in all three SELECTs exactly match the sqlx `#[derive(sqlx::FromRow)]` field names on `DomainInstanceRow` / `GlobalInstanceRow` / `QueryBaseRow` (derives verified). Compile + runtime mapping confirmed by the passing suite.
- (e) NULL semantics: no activation row → all three columns NULL → `classify` arm `(None, _) => ActionableNow` (eligibility.rs:52). Terminal current visits: transition into TERMINAL creates no TASK activation (activation write is gated on TASK visits; source activation closed with `CLOSURE_REASON_TRANSITIONED` in `transition_transaction.rs:519`), so terminal = NULL facts = ACTIONABLE_NOW. Legacy work is never hidden.

### Checkpoint 6 — Semantics vs spec §2 (PASS with non-blocking note)

`classify` (eligibility.rs:49-60) implements the §2 table exactly:
| §2 case | code path | result |
|---|---|---|
| no activation row | `(None, _)` | ACTIONABLE_NOW |
| closed activation (excluded by JOIN → NULL facts) | `(None, _)` | ACTIONABLE_NOW |
| open HUMAN_WORK_ITEM | `(Some(_), _)` fallthrough (unit-tested with kind) | ACTIONABLE_NOW |
| open DISPATCH_INTENT, effective ≤ now | `Some(effective) if effective > now` else-branch; exact-due (`== now`) → actionable (unit test) | ACTIONABLE_NOW |
| open DISPATCH_INTENT, effective > now | `Some(effective) if effective > now` | WAITING_FOR_TIME |

No BLOCKED state exists anywhere in the diff (grep of the 7 impl files: only the two variants). Transition semantics untouched (no file in the diff touches the transition engine; `TRANSITION_ELIGIBILITY_ENFORCEMENT = FOLLOW_UP_DEBT` recorded at spec §2 line 76, consistent with VISIT_ACTIVATION_IMPL_V1 §4's own out-of-scope/FOLLOW_UP_DEBT section). Timeline justification honest: `list_node_visits` selects ALL visits of the instance ordered ASC (query_detail.rs:330-345) — the historical timeline; `VisitRow::into_item` stamps unconditional ACTIONABLE_NOW; every prior visit was necessarily left via a transition whose source activation was closed in the same transaction (`transition_transaction.rs:507-525`, `close_activation_by_visit_required` with `CLOSURE_REASON_TRANSITIONED`, and a missing/already-closed activation aborts the command as drift). Fail-open case honest (see checkpoint 1 CHECK constraint). Non-blocking note 1 above records the intra-surface timeline/current-visit duality.

### Checkpoint 7 — Boundary discipline (PASS)

`git diff c4f1fa8..HEAD --stat` = exactly 8 files, +409/−2:
1. docs/specs/SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1.md (spec, +142)
2. src/application/workflow_instance/eligibility.rs (NEW, +146)
3. src/application/workflow_instance/mod.rs (+3, module registration only)
4. src/application/workflow_instance/query_types.rs (+10, two fields + docs)
5. src/store/postgres/workflow_instance_repository/query_domain_instances.rs (+33/−1)
6. src/store/postgres/workflow_instance_repository/query_global_instances.rs (+33/−1)
7. src/store/postgres/workflow_instance_repository/query_rows.rs (+30)
8. src/store/postgres/workflow_instance_repository/query_visibility.rs (+14, load_base SELECT + joins)
No migration file, no broker file, no auth/credential file, no transition-engine file, no Cargo.toml / lockfile change. Read-only projection confirmed: the diff contains no INSERT/UPDATE/DELETE SQL and no new endpoint (eligibility flows out through existing surfaces: `DomainInstanceSummary` shared by domain+global lists; `NodeVisitItem` via `FullWorkflowInstanceDetail.current_visit` — which `workflow_my_tasks` (`AssignedWorkItem.detail`, query_worklists.rs `list_assigned_to_me` → `build_full` → `base.current_visit(true)`, query_detail.rs:170-171) and instance detail both consume — and via the visits timeline).

### Checkpoint 8 — Spec metadata (PASS)

Frontmatter of `docs/specs/SVC_WORKFLOW_WORK_ELIGIBILITY_PROJECTION_V1.md`: `status: proposed` (line 4); `base_head: c4f1fa8d9bae7c91d9cc09751cfa8e2195c3911a (github/main == production live)` (line 8, matches verified base); `implementation_head: cfb0134` (line 9, matches); `implementation_authority: none` (line 10); `production_apply_authority: none` (line 11); `supersedes: []` (line 19). Body restates "authorizes no merge and no production apply" (lines 24-28).

## Test-run log summary

- `cargo test --lib eligibility` (impl HEAD): 7 passed, 0 failed.
- `cargo test` (impl HEAD): 170 passed; 1 failed = `upgrade_0012_to_0014_succeeds` (Pg 28P01 password auth at tests/00_upgrade_verification.rs:46).
- `cargo test --test 00_upgrade_verification` (pristine c4f1fa8 in temp worktree): 1 failed = same test, same 28P01 panic — A/B identical.
- Throwaway serde wire test: 3/3 ran; outputs recorded under Checkpoint 3; file deleted, working tree clean.

## Reviewer hygiene statement

No pushes, no merges, no production access. Only writes: this review record file, plus a throwaway test file (created, executed, deleted) and a temp git worktree (created, tested, removed) strictly for verification. Working tree of the review target is clean apart from this record.

---

# RE-AUDIT (fix-once phase) — 2026-09-05

- scope: BLOCKER B-1 fix + no-regression spot-check of prior PASS checkpoints, per re-audit mandate.
- fix commit: `7c270c7` (`WORK_ELIGIBILITY_PROJECTION_B1_FIX`), on top of reviewed head `d1ec1dc` (spec) / `cfb0134` (impl). At re-audit start the fix was an uncommitted working-tree delta to `eligibility.rs` only; it was committed mid-audit (contents byte-identical to what was tested).
- delta verified: `git diff d1ec1dc..7c270c7` = exactly `src/application/workflow_instance/eligibility.rs` (+34/−3 by content) plus this audit record file. NO other file changed — no SQL, no migration, no broker file, no spec text, no Cargo.toml. All prior checkpoint conclusions (1-2, 4-8) are structurally unaffected; spot-checked and holding.

## B-1 resolution — VERIFIED FIXED

1. **Type change** (eligibility.rs): `WaitingForTime` is now a newtype variant `WaitingForTime(chrono::DateTime<Utc>)`; `classify` constructs `WorkEligibility::WaitingForTime(effective)` from the same `effective > now` guard (no semantic change — the instant it previously computed and dropped is now carried); doc comment freezes the exact wire shape.
2. **Independent wire-string check** (throwaway integration test, run then deleted; tree clean after):
   - `serde_json::to_string(WorkEligibility::WaitingForTime(2026-09-05T12:00:00Z))` → `{"classification":"WAITING_FOR_TIME","nextEligibleAt":"2026-09-05T12:00:00Z"}` — exactly the spec §3 promise (adjacently tagged, RFC3339 content).
   - `serde_json::to_string(WorkEligibility::ActionableNow)` → `{"classification":"ACTIONABLE_NOW"}` (bare tag).
   - Both deserialize back with equality — round-trip clean, including the previously-rejected content-bearing payload case.
3. **Permanent regression coverage**: new unit test `wire_shape_round_trips_with_next_eligible_at_content` asserts classification tag + `nextEligibleAt` presence via `serde_json::Value`, round-trips both variants, and pins the ACTIONABLE_NOW bare-tag shape. B-1 cannot silently regress.
4. **Tests re-run by reviewer**:
   - `cargo test --lib eligibility` → **8/8 ok** (7 prior + 1 new regression test).
   - `cargo test --lib` (full lib binary) → **171 passed, 0 failed**.
   - Full `cargo test` in THIS reviewer environment: lib binary 171/0, then the suite aborts at `00_upgrade_verification` (`28P01` password authentication to local PostgreSQL) under cargo's fail-fast default; with `--no-fail-fast` all DB-dependent integration binaries fail on the same 28P01 — this reviewer shell has no working local PostgreSQL credentials. This is the identical environment failure A/B-verified on pristine `c4f1fa8` during the original audit; the fix delta touches no DB path, so it cannot affect those tests. The implementer's run reports full suite **171 passed / 0 failed** (upgrade test included) with working credentials — consistent with a unit-only delta. No regression signal.

## Non-blocking notes carried forward / added

- N1 (carried, unchanged): timeline vs current-visit duality in instance detail (disclosed spec §5).
- N2 (carried, unchanged): clock-at-mapping-time microsecond straddle (disclosed spec §5).
- N3 (carried, unchanged): per-site join-alias coupling of the eligibility SQL (documented, correct at all three sites).
- N4 (new, process): fix commit message says "eligibility.rs delta ONLY ... no other file touched" but `7c270c7` also committed this audit record file (`docs/audits/`). Cosmetic commit-message inaccuracy; no code impact.
- N5 (new, acceptance action item): spec frontmatter still pins `implementation_head: cfb0134`; the implementation is now `7c270c7`. At spec acceptance the implementation_head must be amended to the fixed head (or a successor commit) so the accepted authority pins what actually ships.

## RE-AUDIT VERDICT: PASS

- BLOCKER_UNION: **empty** (B-1 resolved and permanently regression-tested).
- All non-blocking findings are documentation/process notes; none blocks shipping.
- Authorization state unchanged by this re-audit: spec remains `proposed` with `implementation_authority: none`; acceptance must pin the fixed implementation head (N5).
