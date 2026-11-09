# H06 Tier-2 R2 preflight and one-attempt runbook

2026-11-06. Attributable Owner/user mandate: `BRIEF-H06-TIER2-R2.md`.
This renews the stopped R1 attempt only for the explicitly verified locale
repair and the requested local port. R1 records remain historical evidence.

```text
SPEC_GOVERNANCE_MODE = PREFLIGHT
TARGET_REPOSITORY = mayf3/svc-workflow
REVIEW_TARGET_HEAD = bcd8950466091f4cb4e0133c431186d9267ac2c2
BASE_HEAD = 59daec042786f0416a6a9061c0b638ba56c0d83e
CURRENT_BASE_HEAD = 59daec042786f0416a6a9061c0b638ba56c0d83e
ROUTE_STAGE = OPERATION (isolated candidate validation)
AUTHORITY_ACCEPTED_IN_BASE = YES (unchanged governance and invariants)
AUTHORITY_ACTION = REUSE
PRIMARY_AUTHORITY = SVC_WORKFLOW_DEVELOPMENT_GOVERNANCE_ADOPTION_V1@BASE_HEAD
RELATED_AUTHORITIES = Product V13 CTR-V6-035..042; Architecture V0.4.7 CTR-ARCH-026
IMPLEMENTATION_AUTHORITY = not_applicable (no semantic implementation)
ATOMIC_SPEC_IMPLEMENTATION_PERMITTED = NO
PLAN_LEVEL = BRIEF
ASSURANCE_LEVEL = CONTROLLED
EXECUTION_MANDATE = VALID (Owner/user, 2026-11-06, one local R2 attempt)
MUTATION_AUTHORIZATION = VALID
ISOLATED_WRITE_SURFACE = YES
CONTROLLED_RUNBOOK_REQUIRED = YES (below)
SPEC_GAP_DEPENDENCY = NONE for unchanged mechanical-candidate validation
EVIDENCE_REVIEWABILITY = PASS (raw logs, exact hashes, independent reviewer)
LIVE_AUTHORITY_GAP = NONE asserted (production state not inspected)
OWNER_DECISION_REQUIRED = NO
EMERGENCY_STATE = NONE
BASE_IMPACT = NONE (fresh GitHub PR base/head match these coordinates)
IMPLEMENTATION_ALLOWED = NO (product semantics)
MERGE_READY = NO (pending actual result and independent review)
OPERATION_ALLOWED = YES (only this isolated R2 run)
NEXT_ACTION = CONTINUE
```

Goal: execute the existing complete three-test Tier-2 target once, stop the
temporary server, and deliver reviewable results. Current gap: R1 stopped at
startup with no tests run. Observation: the regulator's retained `start.log`
records PG16.15 ready after the locale repair, followed by clean shutdown;
its failed implicit `yanfenma` connection does not require creating a user.
Original initdb command used `-U postgres`. Both fixture paths were freshly
read: 540472 bytes, SHA256
`0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606`.
Data directory is retained, mode 0700, PG_VERSION=16, no PID/socket.

Adoption is accepted in exact main/base snapshot above; adoption commit
`f4a591af81c2f40f330d8254fc2aaa9700270eb7` is its ancestor and governance
integrity check returned rc0. The local branch named `main` is historical;
it is not used as the integration base. Relevant authority bytes match base.
Cutover child remains proposed / implementation_authority:none. Existing
independently reviewed mechanical exemption is reused; no fleet acceptance,
product implementation, deployment, production apply or merge is authorized.

Sole writer/operator: root agent, `/private/tmp/svc-h06-manifest`, isolated
branch `ac-h06-migration-manifest`; independent reviewer is read-only.

Runbook:

1. Preserve R1 evidence and regulator start.log unchanged. Create new 0700
   `/private/tmp/h06-tier2-r2-evidence`; archive source hashes, original runner,
   exact patch and tool versions. Require no retained-cluster PID/socket.
2. Every pg_ctl, psql and script invocation inherits a clean explicit environment
   with `LC_ALL=C LANG=C`, fixed tool PATH and PSQLRC=/dev/null. No inherited
   database URLs, PG services, credentials or dotenv are passed.
3. Reuse existing cluster, no initdb or role creation. Start exactly once:
   `/opt/homebrew/opt/postgresql@16/bin/pg_ctl -D /private/tmp/h06-tier2-pg
   -l /private/tmp/h06-tier2-r2-evidence/postgres.log -w -t 30 -o
   "-p 55432 -h '' -k /private/tmp/h06-tier2-pg -c unix_socket_permissions=0700
   -c log_connections=on -c log_disconnections=on" start`.
4. Connect only through this Unix socket as postgres. Read back exact directory,
   port, socket, listen_addresses='', PG16, null server IP and actual superuser;
   verify initial database inventory contains only postgres/template0/template1.
   Do not connect to other instances. Any failure/mismatch stops execution.
5. Reuse R1 opt-in runner patch. Only its two fixed local port literals change
   55487 -> 55432. Run the exact script with TRUSTED_FLEET_LOCAL_PG=1 once.
   Keep Docker default branch, SQL/order, Cargo locked command, cfg, threads,
   source, frozen fixture and every assertion unchanged. Record complete merged
   stdout/stderr and rc. A nonzero rc is terminal; no retry or semantic repair.
6. Finally call pg_ctl on this directory only, `-m fast -w -t 30 stop`, with
   LC_ALL=C LANG=C. Read status; verify PID/socket/process absence. Retain data
   and logs. Archive unchanged-source comparison, result.json and SHA256SUMS.
7. Independent static and post-state/evidence review; persist receipt and, on
   failure, STOPPED-H06-TIER2-R2.md. Commit and push GitHub origin branch, then
   update PR #129 and #1360 status. A timed-out submission ends further writes
   and submissions; preserve commit and UNKNOWN outcome without retry.

Secrets: none. Scope: zero production DB connections/service writes, zero system
service registration. Existing local servers remain untouched. Convergence:
RETAIN frozen identities and both existing consumers; no new executable model.
Evidence needed: start/identity/suite/stop logs, actual rc, both fixture hashes,
source equality, runner diff, independent review and final-head readback.
Done when: one run and truthful archived result, stopped retained cluster,
committed/pushed receipt and requested status updates, or explicit timeout stop.
Expansion trigger: source/assertion/fixture drift, identity mismatch, failed gate,
non-isolated access or unknown external write. Only cleanup and evidence delivery
continue after a failed test. The three tests cover fixture, CLI and one empty-DB/
canary COMMITTED/NOOP scenario; full fleet/recovery/production acceptance is not
established by this suite.
