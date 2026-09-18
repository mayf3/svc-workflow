# M1B dangling-instance cancel closure — source review record

Repository: `mayf3/svc-workflow`. Recorded 2026-09-18.

Directive: `WORKFLOW_DANGLING_INSTANCE_CANCEL_M1B_CLOSURE_V1` (Owner,
2026-09-18, SOURCE CLOSURE ONLY — no production deploy, no production data
mutation). Base: `github/main` `07d888205001708080aa50720e428c6f1c775c7b`.

Candidate: `SVC_WORKFLOW_DANGLING_INSTANCE_CANCEL_V1` (proposed spec) +
bounded cancel-branch implementation + tests T1-T8, commits `a1e48bf`,
`13336e8`, `2706af1`. PR #45 (`161bc9ee`, stale base `bd47668`,
AUTHORITY_GATE=PENDING) is predecessor evidence only; superseded by this
landing.

Independent review (reviewer session independent of the author), round 1 at
`a1e48bf`:

```text
M1B_REVIEW_VERDICT=PASS
SHIP_BLOCKERS=NONE
MAJOR=NONE
MINOR=4 (spec citation V7 §5.3->§5.4; cancel deterministic-failure receipt
        wording; dangling cancelled_from_node_key="" undocumented; T8 error
        variant unpinned) + 1 no-action observation (TERMINAL guard
        structurally unreachable for dangling rows)
DANGLING_BRANCH_BOUNDED=YES
NO_AUTHZ_WIDENING=YES
NO_SYNTHETIC_VISIT=YES
OPEN_RUNTIME_FACT_FAIL_CLOSED=YES
NORMAL_CANCEL_UNREGRESSED=YES
IDEMPOTENCY_STABLE=YES
ATOMICITY_PROVEN=YES
```

All actionable MINORs fixed at `13336e8` + `2706af1` (spec + tests only;
`git diff a1e48bf..2706af1 -- src/` empty — implementation byte-untouched
since review). Fresh final-head recheck by the same independent reviewer:

```text
M1B_FINAL_HEAD_RECHECK_VERDICT=PASS
TESTS=21_instance_cancel_archive 27/27 green at 2706af1 (20 pre-existing + 7 new)
LANDING_AUTHORIZATION=directive §8 pre-authorization on review PASS
```

F4 production preflight (read-only, 2026-09-18): 4/4 exact shape match
(`current_node_visit_id IS NULL`, active, unarchived),
`F4_WITH_OPEN_RUNTIME_FACT = 0`.

This record covers source closure only. Production deploy, F4 data
disposition, and parent-pool mutations remain separate gates.
