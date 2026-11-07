# #1359 R5 attribution and bounded delivery

Issuer: current user R5 continuation, 2026-11-08 Asia/Shanghai. Exact request:
stash candidate changes, rerun affected group67 on clean 8aa864a, restore via
stash pop; clean red is recorded pre-existing and excluded from this delivery
gate, clean green requires a candidate repair; then candidate surface validation,
commit, push and PR against main, no merge, completed receipt. Unexpected failures
stop dependent work. The expressly requested diagnostic red is expected evidence.

PREFLIGHT / REUSE / BRIEF / DURABLE / IMPLEMENTATION.
TARGET_REPOSITORY=mayf3/svc-workflow.
SURFACE=/private/tmp/h1359-outbox; BRANCH=fix/outbox-channel-isolation.
REVIEW_TARGET_HEAD=8aa864ab9ff70107faf8e016a04d32f4e78ad141 plus preserved R3/R4 dirty delta.
BASE_HEAD=CURRENT_BASE_HEAD=8da8dbcb70e700d9e35970e8b662b1be2c29afdc.
AUTHORITY_ACCEPTED_IN_BASE=YES; IMPLEMENTATION_AUTHORITY=contracts.
PRIMARY_AUTHORITY=SVC_WORKFLOW_EXECUTION_CONTROL_V3 at integration base.
RELATED_AUTHORITIES=ProductV13, ArchitectureV0.4.7, OwnerObligationV3.
Authority paths match integration base; accepted governance byte check rc0.
EXECUTION_MANDATE=MUTATION_AUTHORIZATION=VALID; ISOLATED_WRITE_SURFACE=YES.
SPEC_GAP_DEPENDENCY=NONE; EVIDENCE_REVIEWABILITY=PASS; LIVE_AUTHORITY_GAP=NONE.
IMPLEMENTATION_ALLOWED=YES; MERGE_READY=NO; OPERATION_ALLOWED=NO (production).
NEXT_ACTION=CONTINUE. Independent reviewer=/root/r5_review, read-only.

OBSERVATION: R4 full1324/10/1 rc101; failures are nine Unavailable and one
HTTP422 admission_legacy_input vs200. Not all ten are Unavailable.
WORKING_GUESS: pre-existing fixture/admission configuration; pending clean control.
Clean8aa already contains the earlier product implementation062fe82, so this
control alone attributes relative to the uncommitted R3/R4 delta. Supplementary
static file/path evidence must be separately identified, not called an original
main runtime comparison.

All original evidence is preserved by hash before stash; only the exact new
task-owned stash may be popped. No rebase, merge, governance editing, unrelated
admission repair, production access/deploy/config/backlog operations. New PG16
clusters bind loopback only, with exact directory/port identity checks before
writes and shutdown; no existing service restart. Whole group67 (23 cases) is a
bounded superset of the affected ten and preserves the R4 test order/context.
Retain all original assertions and enabled/claim/order/idempotency/backoff paths.

EVIDENCE_NEEDED: clean status+source hashes; raw67 command/log/rc and exact failing
names; stash restoration hashes; candidate outbox22/22 and library suite rc0;
reuse R4 full non67/45 evidence where hashes match; independent affected/final
head review; ordinary push and fresh open/unmerged base-main PR readback.
DONE_WHEN: attributed failure set, preserved/restored candidate, candidate gates
pass, source/evidence committed, branch pushed, unmerged PR and complete receipt.
EXPANSION_TRIGGER: unexpected test/setup/restoration/publication failure or
load-bearing accepted Contract delta; stop and write precise checkpoint.
CONTROLLED_RUNBOOK_REQUIRED=NO (source/disposable fixtures only).
