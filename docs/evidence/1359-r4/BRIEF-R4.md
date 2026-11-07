# #1359 R4 fixed fixture database route and delivery mandate

Issuer: current user R4 continuation, 2026-11-08 Asia/Shanghai. Reopens the
R3 full-suite stop only for the fixed database-name runner correction,
full cargo rc0, commit, push and unmerged PR against main, then completed
RECEIPT-1359-OUTBOX.md. Unexpected failure stops dependent work. No governance
edits, rebase, self-merge, production access, deployment, config or backlog
operations, or unrelated repair. Existing R3 attribution/focused evidence
is retained and reused; no repeat RED or backoff repair.

PREFLIGHT: REUSE / BRIEF / DURABLE; ROUTE_STAGE=IMPLEMENTATION;
AUTHORITY_ACCEPTED_IN_BASE=YES; IMPLEMENTATION_AUTHORITY=contracts;
EXECUTION_MANDATE=VALID; MUTATION_AUTHORIZATION=VALID;
ISOLATED_WRITE_SURFACE=YES; SPEC_GAP_DEPENDENCY=NONE;
EVIDENCE_REVIEWABILITY=PASS; LIVE_AUTHORITY_GAP=NONE;
IMPLEMENTATION_ALLOWED=YES; MERGE_READY=NO (pending review; self-merge forbidden);
OPERATION_ALLOWED=NO (production); NEXT_ACTION=CONTINUE.

Repository/surface: mayf3/svc-workflow, /private/tmp/h1359-outbox.
Branch: fix/outbox-channel-isolation. Start HEAD:
8aa864ab9ff70107faf8e016a04d32f4e78ad141. Integration base and freshly read
remote main: 8da8dbcb70e700d9e35970e8b662b1be2c29afdc.
Accepted authority and preserved contracts are bound in BRIEF-1359-OUTBOX.md:
Execution Control V3, Product V13, Architecture V0.4.7 and Owner Obligation V3.
Adoption lock accepted by mayf3 is present in base; authority paths unchanged.
Independent read-only reviewer /root/r4_review confirmed preflight and route.

OBSERVATION: R3 45_workflow_prerequisites failed 18 fixture guards because
TEST_DATABASE_URL lacked the required /svc_prereq_20261029_impl suffix.
R4 supplies that fixed input route. The fixture still creates its own fresh
UUID database per test; tests/prerequisites/support.rs is unchanged.
Preserve R3 runner/log bytes; R4 runner is a separate copy. Other routing
uses separate task-owned databases and the existing pinned fixture names/ports.
Check task cluster data-directory/port/loopback identity before runner writes.
No duplicated product execution model is introduced; distinct fixture
identities are RETAINed. Product source and accepted semantics are unchanged.

DONE_WHEN: full suite rc0 with raw output, all binary URLs/rc, unchanged
source/test hashes, independent affected-contract/exact-head review, pushed
main-base PR open/unmerged and completed receipt with attribution and routing
coordinates. EXPANSION_TRIGGER: any unexpected validation failure or contract
delta; stop and record. No additional approval is required by this route.

## End checkpoint

STOPPED:45=28/28rc0,48=22/22rc0; full=1324pass/10fail/1ignored,rc101,
newly reached67 admission failures. No R4 commit/push/PR/merge or rerun.
All3 owned PG stops actualrc0. No behavior change beyond the authorized
evidence runner; R3 source/tests and records remain resumable.
