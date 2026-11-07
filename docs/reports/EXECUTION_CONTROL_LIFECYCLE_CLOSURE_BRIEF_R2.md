# Execution Control lifecycle registration closure — R2 Brief

This Brief records the user's R2 execution mandate and its bounded registration route. It is not new Product Authority, a new acceptance, or implementation/runtime evidence.

## Mandate and exact surface

- Issuer: the user in the R2 continuation instruction for `mayf3/svc-workflow`.
- Isolated worktree: `/private/tmp/h-svc-lifecycle`.
- Branch: `docs/v2-lifecycle-closure`; parent and fresh `origin/main`: `d4c510d94f8929f5377989bd69229d611ab49304`.
- R2 ruling: “V3对**两位前任**的继任声明已accepted——补全该声明已包含的两步登记(V1与V2均翻superseded+各自backlink)仍属§3A登记补全, 非新语义”; “V1→V2的历史回链保留原样”; requested result: `V1 superseded / superseded_by=V2`, then `V2 superseded / superseded_by=V3`, with the existing V3 declaration checked without duplication.
- Authorized effects: complete the already declared lifecycle registration and the two discovery indexes, run documentation checks, commit, push, open a PR against `main`, and write `RECEIPT-SVC-LIFECYCLE.md` after PR readback. The PR remains unmerged.
- Abort: a failed required check, an inconsistent historical backlink, a new semantic conflict, or any need to change accepted contract meaning. Stop and persist the failure; no automatic replay of a failed mutation.
- The original untracked `BRIEF-SVC-LIFECYCLE.md` and `STOPPED-SVC-LIFECYCLE.md` remain historical handoff records. This R2 mandate resolves their registration-routing stop; their bytes are preserved.

## PREFLIGHT

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
TARGET_REPOSITORY = mayf3/svc-workflow
BASE_HEAD = d4c510d94f8929f5377989bd69229d611ab49304
CURRENT_BASE_HEAD = d4c510d94f8929f5377989bd69229d611ab49304
REVIEW_TARGET_HEAD = final candidate SHA recorded in PR and receipt after commit
GOAL_OR_TARGET = close the accepted WEC V1 -> V2 -> V3 registration chain
CURRENT_GAP = V2 remains accepted/null; discovery indexes are stale
AUTHORITY_ACTION = REUSE
PRIMARY_AUTHORITY = SVC_WORKFLOW_EXECUTION_CONTROL_V3 @ BASE_HEAD
RELATED_AUTHORITIES = WEC V1/V2; accepted local governance; Product V13; Architecture V0_4_7
ROUTE_STAGE = OPERATION (documentation registration only)
AUTHORITY_ACCEPTED_IN_BASE = YES
IMPLEMENTATION_AUTHORITY = not_applicable (no product implementation)
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = DURABLE
MUTATION_AUTHORIZATION = VALID (explicit user R2 continuation)
ISOLATED_WRITE_SURFACE = YES
CONTROLLED_RUNBOOK_REQUIRED = NO
SPEC_GAP_DEPENDENCY = NONE
WORKING_GUESS = NOT_APPLICABLE
OWNER_DECISION_REQUIRED = NO (R2 ruling supplied)
EVIDENCE_REVIEWABILITY = PASS (exact Git objects and public acceptance record)
LIVE_AUTHORITY_GAP = NONE (runtime untouched)
BASE_IMPACT = BOUNDED
IMPLEMENTATION_ALLOWED = NO
MERGE_READY = NO (unmerged PR is the requested stop boundary)
OPERATION_ALLOWED = YES (bounded documentation registration and publication)
EVIDENCE_NEEDED = exact before/after metadata; historical-edge consistency; byte preservation; index checks; independent final-Head review; remote PR readback
DONE_WHEN = documentation checks rc0 + commit/push + open PR(base main, unmerged) + completed receipt
EXPANSION_TRIGGER = accepted semantics, historical backlinks, governance tools, product source, runtime, or production would need changing
NEXT_REAL_ACTION = NOT_APPLICABLE
NEXT_ACTION = CONTINUE
```

Governance adoption is accepted in `.agents/governance.lock.json`; its latest adoption commit `f4a591af81c2f40f330d8254fc2aaa9700270eb7` is in the base ancestry. The WEC V3 acceptance commit `f1211d0ee37f795d108f8b1820037f732769b2d1` is also in the base ancestry. The task does not accept or amend a Product Contract.

## Observations and consistency decision

All coordinates below bind `BASE_HEAD`; paths are under `docs/specs/` unless otherwise stated.

| Record | Before | Required after | Disposition |
|---|---|---|---|
| `SVC_WORKFLOW_EXECUTION_CONTROL_V1.md:4,64-65` | `superseded`; `supersedes: []`; `superseded_by: V2` | identical | Step 1 already satisfied; zero file delta |
| `SVC_WORKFLOW_EXECUTION_CONTROL_V2.md:4,19-21` | `accepted`; `supersedes: [V1]`; `superseded_by: null` | `superseded`; same `supersedes`; `superseded_by: V3` | Step 2 changes exactly two frontmatter fields |
| `SVC_WORKFLOW_EXECUTION_CONTROL_V3.md:4,19-22` | `accepted`; `supersedes: [V2,V1]`; `superseded_by: null` | identical | Existing accepted declaration retained; zero file delta |

Here V1/V2/V3 abbreviate the complete `SVC_WORKFLOW_EXECUTION_CONTROL_V1/V2/V3` IDs. The R2 prompt's description of V1 as still accepted differs from the exact base: V1 is already superseded. The requested V1 post-state is satisfied without rewriting history or claiming a nonexistent status change.

The existing V1→V2 backlink means **V1 was replaced by V2**. Evidence is V1 lines 64-65, V2 title line 3 and `supersedes` lines 19-20, plus V2 lines 28-30 and 41 explicitly describing the whole-authority successor and reciprocal predecessor backlink. Historical commit `75a09fee1322c01f3f2c4a49a6064a62f95d37d4` made the V1 `accepted→superseded`, `null→V2` change together with V2 acceptance. Retaining that exact edge and state is fully consistent with R2. V3's already accepted list `[V2,V1]` records its full predecessor coverage; the immediate historical retirement path remains V1→V2→V3, as explicitly ruled in R2.

V3 acceptance provenance remains its lines 25-37, commit `f1211d0ee37f795d108f8b1820037f732769b2d1`, and [Owner decision on issue #125](https://github.com/mayf3/svc-workflow/issues/125#issuecomment-6600869500) (author `mayf3`, `2026-11-03T10:12:47Z`; fresh read during R2). That prior acceptance remains distinct from this registration operation.

Affected historical model disposition: RETAIN V1/V2 bodies, stable IDs, acceptance provenance, and V1→V2; RETAIN V3's accepted double-predecessor declaration; retire only V2's stale active lifecycle projection by the authorized metadata registration. No executable path or consumer migration is part of this docs-only task.

## Exact change and assurance boundary

Change only V2's `status` and `superseded_by`, the WEC entries in `docs/specs/README.md`, and the WEC entries in `.agents/local/README.md`, plus this mandate Brief. Preserve every Spec body and all other frontmatter bytes. The index lists the accepted V3 and the V1/V2 historical path; it does not itself create authority.

Required documentation checks are `git diff --check`, `python3 .agents/tools/verify_governance.py --target .`, and a focused readback of the real base/candidate YAML, exact two-field diff, chain termination at accepted V3, affected index rows/links, and unchanged V1/V3/body bytes. An independent reviewer checks the exact committed final Head before push/PR. Results and actual candidate coordinates belong in the PR record and final receipt.

The vendored `validate_spec_transition.py` is **NOT_RUN**, and no transition-validator PASS is claimed. Static inspection at base lines 227-240 and 312-314 shows it rejects late registration to an already normative successor and the retained V3/V1 coverage alongside historical V1→V2. These assumptions do not represent the user's explicit R2 adjudication. Local enforcement truth (`.agents/local/README.md`, enforcement section) identifies base-branch/full syntax gates as not implemented. This task neither changes that tool nor its governance lock, fabricates a proposed baseline, filters V1 out of a claimed transition run, or claims this retained graph passes that validator. The focused documentation readback checks this exact authorized registration; it is not a replacement governance gate.

No product test, source implementation, merge, deployment, production write, or business/runtime acceptance is a deliverable here.
