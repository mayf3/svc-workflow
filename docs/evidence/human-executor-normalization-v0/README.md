# HUMAN_EXECUTOR_NORMALIZATION_V0 exact-20 plan

This directory freezes the bounded input plan for Lane G2. It is evidence and
does not authorize implementation or production apply.

```text
SOURCE_DATABASE_LOGICAL_IDENTITY = svc_workflow_dogfood_clean
OBSERVED_AT = 2026-09-14T07:43:29+08:00
READ_MODE = PostgreSQL session default_transaction_read_only=on
TARGET_ROW_COUNT = 20
EXCLUDED_CANARY_WORKFLOW_ID = 6ea453e2-6a14-4c1f-a87d-76d225ead0f5
TARGET_AUTH_USER_ID = 8902db0d-429a-4e37-985c-f8b92d4b78fb
TARGET_PRINCIPAL_TYPE = HUMAN
PLAN_FILE = exact-20-plan.tsv
PLAN_SHA256 = b349e203c00ac82e286666a89dbedd6a17f77e0221090a1a9f2db51d8a253199
```

## Selection and identity basis

The row set is the previously Owner-confirmed exact 20 real human-work
Workflow IDs. It is not selected by title, description, metadata, or runtime
inference. The separately identified credential-separation canary above is
excluded by exact Workflow UUID.

The target UUID is the Owner-authorized canonical Auth User registration for
the single real human executor. Read-only Auth verification found that exact
row enabled and active. It is a User identity, not an Agent impersonation and
does not require machine credentials. The bounded implementation must ensure
the matching svc-workflow Principal projection has type `HUMAN` before it may
create any successor.

## Determinism and conflict checks

`exact-20-plan.tsv` is UTF-8, LF-only, has one header plus 20 rows sorted by
full `workflow_id`, and ends with one LF. Its SHA-256 is over those exact raw
bytes. Each row binds the production source Workflow to its current Visit,
state version, assignee, DefinitionVersion, node, visit number, Context
revision, and Context payload digest. Every successor target Visit UUID is
preassigned in the TSV; the 20 target UUIDs are unique and a read-only query at
the observation coordinate found zero conflicts in production
`workflow_node_visits`.

Before any apply, all frozen source coordinates, the target User status and
type projection, the plan hash, and all preassigned UUID non-conflicts must be
revalidated. Any mismatch is a zero-write conflict; the apply process must not
regenerate target identities or modify this plan in place.

No production mutation, Workflow transition, dispatch, task completion,
historical-row rewrite, or code implementation was performed while generating
these artifacts.
