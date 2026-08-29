---
spec_id: SVC_WORKFLOW_INVALID_RETURN_REFERENCES_HTTP_422_V1
status: proposed
spec_kind: implementation
authority_level: governing_spec
implementation_authority: contracts
scope:
  - mayf3/svc-workflow
  - workflow-transition-execute-api-invalid-return-references-error-surface
governed_by:
  - SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1
external_authorities: []
supersedes: []
superseded_by: null
owners:
  - mayf3
title: invalid_return_references 422 Detail Exposure and RETURN Contract Error Aggregation V1
repo: mayf3/svc-workflow
base_head: bf875c265843b3e07570a96b734051e9cfe27a43
date: 2026-08-29
product_code_changed_by_this_spec_pr: false
server_change_authorized_upon_acceptance: true (exact three-commit reland only; §5)
implementation_authority_activation: accepted_on_main
merge_required_for_activation: true
production_deploy_authorized_by_this_spec: false
owner_ruling_input: PRESERVE_PRODUCTION_RETURN_422_FIX_AND_RELAND_TO_MAIN
---

# SVC_WORKFLOW_INVALID_RETURN_REFERENCES_HTTP_422_V1

## 0. Problem and current split state

1. A RETURN transition whose submission payload misses or malforms the
   RETURN-specific contract fields (`rootCauseNodeVisitId`, `reasonCode`,
   `reason`) is rejected with HTTP 422 `invalid_return_references`
   (incident instance `121e76b4`). Before the production fix, the HTTP
   layer swallowed the domain `InvalidReturnReferences(detail)` string:
   callers received only the fixed message `"return references are
   invalid"` and could not locate the offending field. The domain layer
   also reported only the first failure instead of the full missing-field
   set.
2. The fix for this lives ONLY on the production side branch
   `feat/fix-return-422-invalid-return-references` (three commits,
   §4). Production runs the side-branch head
   `91fc4e40f400ee9cc17351f857a1ab2860682681`. Repository authority
   branch `main` is at
   `bf875c265843b3e07570a96b734051e9cfe27a43` (PR #15, Global Workflow
   Reader) and does NOT contain the fix.
3. Consequence frozen by Owner ruling
   `PRESERVE_PRODUCTION_RETURN_422_FIX_AND_RELAND_TO_MAIN`: deploying
   current main would silently revert live production behavior
   (`details.detail` exposure + aggregation). Therefore
   `GLOBAL_READER_DEPLOYMENT_BLOCKED = YES` until the reland authorized
   by this Spec is merged to main. Silent rollback of the production fix
   is FORBIDDEN.

## 1. Authority audit (why this Spec exists)

Acceptance gate result at base `bf875c2`:

- `SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1` (accepted,
  invariant, implementation_authority: none) — process authority; does
  not authorize this behavior.
- `SVC_WORKFLOW_PRINCIPAL_SUCCESSOR_MIGRATION_V1` (accepted) — one-time
  successor migration; out of scope.
- `SVC_WORKFLOW_GLOBAL_WORKFLOW_READER_V1` (accepted) — global reader
  role; out of scope.
- `SVC_WORKFLOW_TRUSTED_FLEET_PRINCIPAL_CUTOVER_V1` (proposed) — not
  authority.
- Product Direction (`SVC_WORKFLOW_PRODUCT_BOUNDARY_V4`) and frozen
  Architecture: no text names `invalid_return_references`, RETURN
  contract detail exposure, or 422 error-body shape for this error
  (searched at `bf875c2`; zero hits in `docs/product/`,
  `docs/architecture/`, `docs/archive/`).

Legacy authority bridge (per
`.agents/local/README.md` existing-authority rules), the ONLY pre-existing
authority touching this error:

- `docs/contracts/WORKFLOW_TRANSITION_CONTRACT_V0_1.md`
  (Status: `IMPLEMENTATION_CONTRACT`, PR 3C/3D era, present unchanged at
  adoption base `8cda3d05e1c22814b7aeaace97d317380df83836` and at
  `bf875c2`) declares in its error table:
  `Invalid RETURN references | 422 | invalid_return_references`.
  This covers the status code and stable error code ONLY — both of which
  main at `bf875c2` ALREADY implements (`unprocessable()` maps to 422
  with code `invalid_return_references`; the pre-fix mapping never
  returned a wrong status). The contract pins NO error response-body
  shape for this error and NOTHING about detail exposure or
  multi-field aggregation.

Therefore: no accepted authority covers the production-fix semantics
(error-body `details.detail` exposure + RETURN contract error
aggregation). The change is non-mechanical (observable wire-response and
validation-error-construction behavior). Per the governance minimum loop,
implementation is forbidden until this Spec is accepted; this Spec is the
required docs-only artifact.

## 2. Normative contracts (authorized upon acceptance)

### CTR-1 HTTP error mapping (L0a)

For `ExecuteWorkflowTransitionError::InvalidReturnReferences(detail)`,
the HTTP layer MUST respond:

- status `422 Unprocessable Content` (unchanged);
- `code = "invalid_return_references"` (unchanged, backward compatible);
- response body carries `details.detail` = the domain-provided detail
  string verbatim.

This additive field follows the repository's existing error-envelope
convention (`ApiError::with_details(json!({"detail": ...}))` already used
by multiple mappings on main: `src/http/error.rs` at `bf875c2`,
lines 255/321/365/417/443/482). No new error variant, no status change,
no code rename.

### CTR-2 RETURN contract error aggregation (L0b)

`src/store/postgres/workflow_instance_repository/transition_validation.rs`
MUST expose a pure `collect_return_contract_errors` aggregation that
collects ALL missing/malformed RETURN contract fields
(`rootCauseNodeVisitId`, `reasonCode`, `reason` — required, plus
UUID-validity for `rootCauseNodeVisitId` and cross-instance checks for
root-cause / related-submission references) into ONE
`InvalidReturnReferences` message that (a) names every offending field
and (b) states the full required RETURN contract. Single-error surface
is preserved; no additional error variants are introduced.

### CTR-3 Error catalog (contract file)

`contracts/workflow-http/v1/errors.json` message for
`invalid_return_references` MUST state that `details.detail` carries the
specific missing/invalid RETURN contract field(s).

### CTR-4 Diagnosis document

`docs/DIAGNOSIS_RETURN_422_INVALID_RETURN_REFERENCES.md` MUST document
all conditions under which a RETURN reference is judged invalid
(upstream root-cause path, incident-shaped summary-only schema, missing
single field, cross-instance references, malformed UUID).

### CTR-5 Test closure

The reland MUST carry, and CI-equivalent local runs MUST pass, the exact
tests from the three source commits (§4): four integration tests, two
inline proptests, one real-TCP e2e with positive control, one HTTP unit
test (names frozen in §6).

### CTR-6 Provenance transparency (no hidden origins)

The reland implementation PR MUST name all three source commits (full
SHAs), the source branch, and production commit `91fc4e4` in its
description. The source branch is preserved verbatim at
`github.com/mayf3/svc-workflow` ref
`refs/heads/feat/fix-return-422-invalid-return-references`
(head `91fc4e40f400ee9cc17351f857a1ab2860682681`, pushed 2026-08-29 for
provenance). Rewriting, rebasing, or deleting that branch is FORBIDDEN
while this Spec is active.

### CTR-7 Scope exclusivity

The reland changes EXACTLY the ten files in §4 (plus lockfile). Nothing
else may ride along — in particular Global Workflow Reader code is
untouched: its `src/http/error.rs` hunk
(`global_read_role_required` message) MUST remain intact alongside the
detail-exposure hunk.

## 3. Explicitly forbidden

- Accepting rollback of the production fix (deploying any main head
  that lacks CTR-1/CTR-2 behavior as a replacement for production).
- Treating the production side branch as deployment authority; only
  merged main after the reland PR is deployable.
- Manually assembling a composite binary mixing branch trees.
- Modifying historical production databases (this Spec requires zero
  migrations; the fix is code + tests + docs only).
- Any additional transition/RETURN semantic change beyond §2.

## 4. Frozen provenance: the three production-unique commits

Source branch `feat/fix-return-422-invalid-return-references`, based on
main merge-base `8cda3d05e1c22814b7aeaace97d317380df83836` (PR #1),
tip `91fc4e40f400ee9cc17351f857a1ab2860682681` = PRODUCTION_COMMIT:

| # | Commit | Subject |
|---|--------|---------|
| 1 | `f283a63261815f0c21276d59abc51f7e7b23edcb` | fix(return): surface invalid_return_references detail + aggregate RETURN contract errors |
| 2 | `9c96a2fab220d1a185a0585d894772accf9d81b7` | test(return-422): real-TCP e2e asserting detail exposure + positive control |
| 3 | `91fc4e40f400ee9cc17351f857a1ab2860682681` | test(return-422): drop residue assertion from parallel E2E to avoid race |

Exact file closure (combined `8cda3d0..91fc4e4`, 10 files,
+913/−35):

| File | Origin commits | Change |
|------|----------------|--------|
| `Cargo.lock` | f283a63 | +100 (proptest dev-dependency tree) |
| `Cargo.toml` | f283a63 | +1 (`proptest = "1"` under `[dev-dependencies]`) |
| `contracts/workflow-http/v1/errors.json` | f283a63 | message documents `details.detail` |
| `docs/DIAGNOSIS_RETURN_422_INVALID_RETURN_REFERENCES.md` | f283a63 | new, +109 |
| `src/http/error.rs` | f283a63 | detail mapping + unit test (+28/−3) |
| `src/store/postgres/workflow_instance_repository/transition_validation.rs` | f283a63 | aggregation + 2 proptests (+280/−33) |
| `tests/17_workflow_runtime/transition/submission_validation.rs` | f283a63 | +169 (4 new tests) |
| `tests/17_workflow_runtime/transition_helpers.rs` | f283a63 | new, +90 |
| `tests/17_workflow_runtime/http/e2e/mod.rs` | 9c96a2f | +1 (register e2e module) |
| `tests/17_workflow_runtime/http/e2e/return_422_detail.rs` | 9c96a2f, 91fc4e4 | new e2e + residue-assertion fix (+168) |

## 5. Reland recipe (implementation round, after acceptance)

1. Branch from then-current main (must contain this accepted Spec).
2. `git cherry-pick f283a63 9c96a2f 91fc4e4` (exact commits, in order).
3. Dry-run evidence at base `bf875c2` (2026-08-29, scratch worktree,
   discarded after verification): all three commits apply
   **conflict-free**; `src/http/error.rs` auto-merges and both hunks
   coexist (detail exposure at line ~216; `global_read_role_required`
   at line ~519). Expect zero conflicts at `bf875c2`; if main advanced
   beyond, resolve only mechanically and disclose any hunk movement.
4. Run: `cargo check --all-targets`; `cargo test --lib`; full
   `cargo test --test 17_workflow_runtime`. Record counts in the PR.
5. Draft implementation PR (separate from any spec PR), description per
   CTR-6. Do NOT deploy from the PR branch; production deploy of main
   after merge is a separate owner-gated act that ALSO unblocks Global
   Reader (`GLOBAL_READER_DEPLOYMENT_BLOCKED` clears only when the
   deployed commit is a main descendant containing this reland).

## 6. Frozen test inventory

- Integration (`tests/17_workflow_runtime/transition/submission_validation.rs`,
  added by f283a63):
  `test_transition_return_missing_contract_fields_reports_all`,
  `test_transition_return_missing_root_cause_only`,
  `test_transition_return_root_cause_not_uuid`,
  `test_transition_return_valid_references_succeeds`.
  (Pre-existing at base and unchanged:
  `test_transition_return_root_cause_wrong_instance`,
  `test_transition_return_related_submission_wrong_instance`.)
- Inline PBT (`transition_validation.rs` `#[cfg(test)]`): proptest block 1
  — root-cause parse never silently ignored (arbitrary string payloads);
  block 2 — boundedness over arbitrary byte payloads.
- E2E (`tests/17_workflow_runtime/http/e2e/return_422_detail.rs`):
  `return_422_exposes_aggregated_contract_detail_over_real_tcp`
  (real-TCP listener; asserts 422 + aggregated `details.detail`; then a
  positive control with a complete payload succeeds; per 91fc4e4 the
  test cleans up its own temporary database and leaves the global
  residue assertion to the scenario e2e).
- Unit (`src/http/error.rs`): `invalid_return_references_exposes_detail`
  (422 + stable code + `details.detail` verbatim).
- Source-round verification recorded in commit f283a63: `cargo test
  --lib` 148 passed; `cargo test --test 17_workflow_runtime` 450 passed;
  `cargo check --all-targets` exit 0 (at `91fc4e4`-lineage base
  `8cda3d0`). The reland round must re-run and re-report at the new
  main tip.

## 7. Acceptance checklist

- [ ] Independent semantic review of this Spec commit.
- [ ] Owner (mayf3) accepts the exact final head; spec merged to main
      with `status: accepted` (+ accepted pin fields).
- [ ] Implementation PR per §5; tests green; provenance disclosed.
- [ ] Only then: production deploy of the merged main (unblocks Global
      Reader). Spec stays `accepted` (no `implemented` lifecycle state).

## 8. Out of scope

Global Workflow Reader deployment mechanics, any other error-code
semantics, transition definition management, DB migrations, idempotency
changes, and everything already governed by other Specs.
