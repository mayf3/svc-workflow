# Human Executor Normalization V0 acceptance

Repository: `mayf3/svc-workflow`.

On 2026-09-14, repository Owner `mayf3` explicitly authorized
`ACCEPT_G2_AUTHORITY=YES` and `MERGE_G2_AUTHORITY=AUTHORIZED` for reviewed
candidate commit `1a777a94fc0df22190c01492b50563233206d4b0`.

The frozen reviewed bytes were:

```text
SPEC_SHA256 = 85e4f42f84ce7c3124f74774a5084065d02881a4ef674b6a2efeeef8d1a12401
PLAN_SHA256 = b349e203c00ac82e286666a89dbedd6a17f77e0221090a1a9f2db51d8a253199
```

The independent exact-head review first returned two bounded blockers. One
bounded repair added the required governance primitives and acceptance
coverage and replaced the impossible one-command/twenty-Event shape with 20
unique row Receipt/Event pairs in one group transaction. The single final
re-audit returned `ACCEPT`, `REMAINING_BLOCKERS=NONE`, and `SCOPE_DRIFT=NO` at
the exact hashes above.

Fresh acceptance readback found the candidate head, Spec bytes, plan bytes,
three-file docs-only scope, and `github/main` base unchanged. This acceptance
transaction changes lifecycle metadata only and records the Owner action. The
reviewed candidate remains unchanged as its parent.

Acceptance activates only the contracts in
`SVC_WORKFLOW_HUMAN_EXECUTOR_NORMALIZATION_V0` after merge to `main`. It
authorizes no production Workflow mutation. The Workflow HUMAN Principal
projection remains a separate Owner-authorized production mutation through the
existing provisioning API; exact-20 production apply requires another later
authorization.
