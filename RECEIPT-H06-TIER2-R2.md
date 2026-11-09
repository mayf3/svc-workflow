# H06 Tier-2 R2 isolated conformance receipt — STOPPED

Date: 2026-11-06. Refs: [svc-workflow #129](https://github.com/mayf3/svc-workflow/pull/129),
[agent-control #1360](https://github.com/mayf3/agent-control/issues/1360).

```text
RESULT = STOPPED_AT_MIGRATION_0030
PG_START_RC = 0
PG_IDENTITY_RC = 0
CONFORMANCE_SCRIPT_EXECUTION = EXECUTED_ONCE
CONFORMANCE_SCRIPT_RC = 3
CARGO_TEST_EXECUTION = NOT_RUN
ASSERTION_EXECUTION = NOT_RUN
ASSERTION_OUTPUT = NONE (Cargo/test executable was never reached)
PG_STOP_RC = 0
PG_STATUS_AFTER_RC = 3 (no server running)
MERGE_READY = NO
PRODUCTION_DB_CONNECTIONS = 0
PRODUCTION_DB_OR_SERVICE_WRITES = 0
SYSTEM_SERVICE_REGISTRATION = NONE
RETRY = NONE
DATA_DIRECTORY = /private/tmp/h06-tier2-pg (retained, mode 0700)
EVIDENCE_DIRECTORY = /private/tmp/h06-tier2-r2-evidence (retained, mode 0700)
```

## Coordinates and authorization

Repository `mayf3/svc-workflow`; sole writer/operator root agent; isolated worktree
`/private/tmp/svc-h06-manifest`, branch `ac-h06-migration-manifest`. Source parent
`bcd8950466091f4cb4e0133c431186d9267ac2c2`; exact integration main/base
`59daec042786f0416a6a9061c0b638ba56c0d83e`. Fresh GitHub PR read matched these
coordinates. The historical local branch named main was not used as base.
Owner/user mandate is persisted verbatim in `BRIEF-H06-TIER2-R2.md`; route and
one-attempt runbook are `PREFLIGHT-H06-TIER2-R2.md`. Accepted adoption and unchanged
invariants were checked at base, governance integrity rc0. This is isolated
validation of the existing mechanical candidate. The child cutover Spec stays
proposed / implementation_authority:none; no semantic implementation, acceptance,
merge, production apply or deployment authority is inferred.

## Actual result and stop boundary

With explicit `LC_ALL=C LANG=C`, PG16.15 started successfully on the retained
R1 directory. No initdb, user creation or system service registration occurred.
Identity readback proved data directory, socket and port 55432, postgres
superuser, PG16, empty listen_addresses, null server IP and initial inventory
postgres/template0/template1. TCP was disabled by `-h ''`, so existing TCP
55432 instances were not accessed.

The exact conformance script then ran once. It created its two isolated test
databases and reached migration 0030. Its complete output was:

```text
CREATE DATABASE
CREATE DATABASE
psql:/private/tmp/svc-h06-manifest/migrations/0030_composite_command_event_integrity.sql:5: ERROR:  LOCK TABLE can only be used in transaction blocks
```

The script returned **3** before Cargo. No test or assertion ran, and no assertion
is reported PASS. This is distinct from R1, whose script never ran. The existing
runner executes each migration as `psql ... -v ON_ERROR_STOP=1 -f "$migration"`
without an explicit transaction; unchanged migration 0030 line 5 begins a
`LOCK TABLE`. PostgreSQL's own log records that exact failed statement. This
is a directly observed runner/migration transaction incompatibility, not a
remaining locale/startup failure. No migration, transaction wrapper, test,
assertion, script semantics or other repair was attempted after failure.

The finally path immediately stopped only the designated data directory:
stop rc0, status rc3. PID/socket and target PostgreSQL process are absent.
The cluster and both test databases, including partial pre-0030 schema state,
remain as evidence; no cleanup deletion or subsequent DB connection was made.
R1 files and regulator start.log remain intact. Only archival, independent
review, commit/push and requested PR/issue status reporting continue.

## Exact input and adaptation bindings

Both the checkout fixture and frozen absolute test fixture
`/Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json`
were read: 540472 bytes each, SHA256
`0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606`.
All tracked Rust source, tests, migrations, Cargo files and checkout fixture
have identical before/after hash manifests. No product or assertion file changed.

The R1 opt-in adaptation is reused. Relative to source parent, exactly two
fixed local port literals changed 55487 -> 55432. Original Docker default,
migration loop/SQL/order, locked Cargo command, Rust cfg and test thread count
are unchanged. Runner SHA256:
`2264bebc411e191a16433ed4cba66f8a97635c23f7be023689af2664bd438947`.
Test file SHA256:
`cde80ff0b177da3d4daab34de4c8f68ae47a45d112234352fb7545d5d7577ffc`.
The complete R2-only patch and inherited full patch are reproduced below.
Independent static review PASS, blocker=0, at those bytes; this does not
establish runtime conformance.

## Environment, start parameters and evidence

Every pg_ctl, psql and script call inherited only the environment below.
No inherited DB URL, PG service, credentials or dotenv was passed. All
connections created by the run target only its restricted Unix socket.

```json
{
  "HOME": "/Users/yanfenma",
  "USER": "yanfenma",
  "LOGNAME": "yanfenma",
  "PATH": "/opt/homebrew/opt/postgresql@16/bin/:/Users/yanfenma/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin",
  "TMPDIR": "/private/tmp",
  "PSQLRC": "/dev/null",
  "LC_ALL": "C",
  "LANG": "C",
  "TRUSTED_FLEET_LOCAL_PG": "1"
}
```

Raw archive: [`docs/reports/h06-tier2-r2-20261106/`](docs/reports/h06-tier2-r2-20261106/).
It includes exact argv/timestamps/rc in commands.json, the executed supervisor
source, all merged command stdout/stderr, complete PostgreSQL log, regulator's
prior start.log copy, source manifests, result.json and SHA256SUMS. Process
inspection is a filtered observation retaining only exact target-cluster
PostgreSQL matches; no claim of archiving unrelated process output is made.
All conformance output is preserved byte for byte. Its three defined tests
would cover fixture, CLI and one canary/NOOP scenario only; none ran here.
Prior Tier-1 rc0 evidence remains in RECEIPT-H06-MANIFEST-R2.md and is not
repeated or promoted to Tier-2 PASS. Independent evidence/closure review is
recorded separately in REVIEW-H06-TIER2-R2.md.

## Complete command output

### git-before

Command: `git status --short`

Start: 2026-11-06T00:46:17.809287+00:00; end: 2026-11-06T00:46:17.825938+00:00; rc: `0`.

```text
 M scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
?? BRIEF-H06-TIER2-R2.md
?? PREFLIGHT-H06-TIER2-R2.md
```

### runner-r2-diff

Command: `git diff HEAD -- scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh`

Start: 2026-11-06T00:46:17.826257+00:00; end: 2026-11-06T00:46:17.838968+00:00; rc: `0`.

```text
diff --git a/scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh b/scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
index d7cb15c..4edf9bf 100755
--- a/scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
+++ b/scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
@@ -17,11 +17,11 @@ trap cleanup EXIT
 SUFFIX=""
 if [[ "${TRUSTED_FLEET_LOCAL_PG:-0}" == 1 ]]; then
   # Owner-authorized H06 Tier-2 alternative; lifecycle is owned by the runbook.
-  BASE="postgres://postgres@localhost:55487"
+  BASE="postgres://postgres@localhost:55432"
   SUFFIX="?host=/private/tmp/h06-tier2-pg"
   IDENTITY="$(psql "${BASE}/postgres${SUFFIX}" -XAt -v ON_ERROR_STOP=1 -c \
     "SELECT current_setting('data_directory') = '/private/tmp/h06-tier2-pg'
-       AND current_setting('port') = '55487'
+       AND current_setting('port') = '55432'
        AND current_setting('unix_socket_directories') = '/private/tmp/h06-tier2-pg'
        AND current_setting('listen_addresses') = ''
        AND current_setting('server_version_num')::int BETWEEN 160000 AND 169999
```

### runner-full-diff

Command: `git diff 5a7dc9ee2dde1dd730dae94afb39f116160aec2c -- scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh`

Start: 2026-11-06T00:46:17.839266+00:00; end: 2026-11-06T00:46:17.851507+00:00; rc: `0`.

```text
diff --git a/scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh b/scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
index ce8652e..4edf9bf 100755
--- a/scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
+++ b/scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
@@ -8,10 +8,28 @@ TARGET="${TRUSTED_FLEET_CARGO_TARGET_DIR:-$ROOT/target/trusted-fleet-conformance
 
 cleanup() {
   set +e
-  docker rm -f "$NAME" >/dev/null 2>&1
+  if [[ "${TRUSTED_FLEET_LOCAL_PG:-0}" != 1 ]]; then
+    docker rm -f "$NAME" >/dev/null 2>&1
+  fi
 }
 trap cleanup EXIT
 
+SUFFIX=""
+if [[ "${TRUSTED_FLEET_LOCAL_PG:-0}" == 1 ]]; then
+  # Owner-authorized H06 Tier-2 alternative; lifecycle is owned by the runbook.
+  BASE="postgres://postgres@localhost:55432"
+  SUFFIX="?host=/private/tmp/h06-tier2-pg"
+  IDENTITY="$(psql "${BASE}/postgres${SUFFIX}" -XAt -v ON_ERROR_STOP=1 -c \
+    "SELECT current_setting('data_directory') = '/private/tmp/h06-tier2-pg'
+       AND current_setting('port') = '55432'
+       AND current_setting('unix_socket_directories') = '/private/tmp/h06-tier2-pg'
+       AND current_setting('listen_addresses') = ''
+       AND current_setting('server_version_num')::int BETWEEN 160000 AND 169999
+       AND current_user = 'postgres' AND inet_server_addr() IS NULL")"
+  [[ "$IDENTITY" == t ]] || { printf 'H06 isolated PostgreSQL identity mismatch\n' >&2; exit 1; }
+  psql "${BASE}/postgres${SUFFIX}" -X -v ON_ERROR_STOP=1 \
+    -c 'CREATE DATABASE svc_workflow_cutover_test' -c 'CREATE DATABASE auth_cutover_test'
+else
 docker run -d --rm --name "$NAME" \
   -e POSTGRES_PASSWORD=postgres -e POSTGRES_USER=postgres \
   -P postgres:16-alpine >/dev/null
@@ -25,12 +43,13 @@ docker exec "$NAME" pg_isready -U postgres >/dev/null
 
 docker exec "$NAME" createdb -U postgres svc_workflow_cutover_test
 docker exec "$NAME" createdb -U postgres auth_cutover_test
+fi
 for migration in "$ROOT"/migrations/*.sql; do
-  psql "${BASE}/svc_workflow_cutover_test" -v ON_ERROR_STOP=1 -f "$migration" >/dev/null
+  psql "${BASE}/svc_workflow_cutover_test${SUFFIX}" -v ON_ERROR_STOP=1 -f "$migration" >/dev/null
 done
 
-export TEST_WORKFLOW_DATABASE_URL="${BASE}/svc_workflow_cutover_test"
-export TEST_AUTH_DATABASE_URL="${BASE}/auth_cutover_test"
+export TEST_WORKFLOW_DATABASE_URL="${BASE}/svc_workflow_cutover_test${SUFFIX}"
+export TEST_AUTH_DATABASE_URL="${BASE}/auth_cutover_test${SUFFIX}"
 export CARGO_TARGET_DIR="$TARGET"
 export RUSTFLAGS="${RUSTFLAGS:-} --cfg trusted_fleet_cutover_conformance --check-cfg=cfg(trusted_fleet_cutover_conformance)"
 
```

### tool-version

Command: `/opt/homebrew/opt/postgresql@16/bin/pg_ctl --version`

Start: 2026-11-06T00:46:17.851845+00:00; end: 2026-11-06T00:46:17.856689+00:00; rc: `0`.

```text
pg_ctl (PostgreSQL) 16.15 (Homebrew)
```

### runner-syntax

Command: `/bin/bash -n scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh`

Start: 2026-11-06T00:46:17.856947+00:00; end: 2026-11-06T00:46:17.860786+00:00; rc: `0`.

```text
```

### pg-status-before

Command: `/opt/homebrew/opt/postgresql@16/bin/pg_ctl -D /private/tmp/h06-tier2-pg status`

Start: 2026-11-06T00:46:17.861102+00:00; end: 2026-11-06T00:46:17.865469+00:00; rc: `3`.

```text
pg_ctl: no server running
```

### pg-start

Command: `/opt/homebrew/opt/postgresql@16/bin/pg_ctl -D /private/tmp/h06-tier2-pg -l /private/tmp/h06-tier2-r2-evidence/postgres.log -w -t 30 -o '-p 55432 -h '"'"''"'"' -k /private/tmp/h06-tier2-pg -c unix_socket_permissions=0700 -c log_connections=on -c log_disconnections=on' start`

Start: 2026-11-06T00:46:17.865704+00:00; end: 2026-11-06T00:46:17.983898+00:00; rc: `0`.

```text
waiting for server to start.... done
server started
```

### pg-identity

Command: `/opt/homebrew/opt/postgresql@16/bin/psql 'postgres://postgres@localhost:55432/postgres?host=/private/tmp/h06-tier2-pg' -XAt -v ON_ERROR_STOP=1 -c 'SELECT json_build_object('"'"'data_directory'"'"', current_setting('"'"'data_directory'"'"'), '"'"'port'"'"', current_setting('"'"'port'"'"'), '"'"'socket'"'"', current_setting('"'"'unix_socket_directories'"'"'), '"'"'listen_addresses'"'"', current_setting('"'"'listen_addresses'"'"'), '"'"'server_version_num'"'"', current_setting('"'"'server_version_num'"'"'), '"'"'user'"'"', current_user, '"'"'is_superuser'"'"', current_setting('"'"'is_superuser'"'"'), '"'"'server_ip'"'"', inet_server_addr(), '"'"'database'"'"', current_database(), '"'"'databases'"'"', (SELECT json_agg(datname ORDER BY datname) FROM pg_database))'`

Start: 2026-11-06T00:46:17.984411+00:00; end: 2026-11-06T00:46:18.005882+00:00; rc: `0`.

```text
{"data_directory" : "/private/tmp/h06-tier2-pg", "port" : "55432", "socket" : "/private/tmp/h06-tier2-pg", "listen_addresses" : "", "server_version_num" : "160015", "user" : "postgres", "is_superuser" : "on", "server_ip" : null, "database" : "postgres", "databases" : ["postgres", "template0", "template1"]}
```

### conformance

Command: `/bin/bash scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh`

Start: 2026-11-06T00:46:18.006322+00:00; end: 2026-11-06T00:46:18.434478+00:00; rc: `3`.

```text
CREATE DATABASE
CREATE DATABASE
psql:/private/tmp/svc-h06-manifest/migrations/0030_composite_command_event_integrity.sql:5: ERROR:  LOCK TABLE can only be used in transaction blocks
```

### pg-stop

Command: `/opt/homebrew/opt/postgresql@16/bin/pg_ctl -D /private/tmp/h06-tier2-pg -m fast -w -t 30 stop`

Start: 2026-11-06T00:46:18.435245+00:00; end: 2026-11-06T00:46:18.544511+00:00; rc: `0`.

```text
waiting for server to shut down.... done
server stopped
```

### pg-status-after

Command: `/opt/homebrew/opt/postgresql@16/bin/pg_ctl -D /private/tmp/h06-tier2-pg status`

Start: 2026-11-06T00:46:18.544796+00:00; end: 2026-11-06T00:46:18.549849+00:00; rc: `3`.

```text
pg_ctl: no server running
```

### Complete R2 PostgreSQL log

```text
2026-11-06 08:46:17.891 CST [18501] LOG:  starting PostgreSQL 16.15 (Homebrew) on aarch64-apple-darwin25.6.0, compiled by Apple clang version 21.0.0 (clang-2100.1.1.101), 64-bit
2026-11-06 08:46:17.891 CST [18501] LOG:  listening on Unix socket "/private/tmp/h06-tier2-pg/.s.PGSQL.55432"
2026-11-06 08:46:17.894 CST [18504] LOG:  database system was shut down at 2026-11-06 08:42:31 CST
2026-11-06 08:46:17.895 CST [18501] LOG:  database system is ready to accept connections
2026-11-06 08:46:17.994 CST [18511] LOG:  connection received: host=[local]
2026-11-06 08:46:17.994 CST [18511] LOG:  connection authorized: user=postgres database=postgres application_name=psql
2026-11-06 08:46:18.005 CST [18511] LOG:  disconnection: session time: 0:00:00.010 user=postgres database=postgres host=[local]
2026-11-06 08:46:18.019 CST [18517] LOG:  connection received: host=[local]
2026-11-06 08:46:18.020 CST [18517] LOG:  connection authorized: user=postgres database=postgres application_name=psql
2026-11-06 08:46:18.021 CST [18517] LOG:  disconnection: session time: 0:00:00.002 user=postgres database=postgres host=[local]
2026-11-06 08:46:18.028 CST [18519] LOG:  connection received: host=[local]
2026-11-06 08:46:18.029 CST [18519] LOG:  connection authorized: user=postgres database=postgres application_name=psql
2026-11-06 08:46:18.129 CST [18519] LOG:  disconnection: session time: 0:00:00.100 user=postgres database=postgres host=[local]
2026-11-06 08:46:18.137 CST [18521] LOG:  connection received: host=[local]
2026-11-06 08:46:18.137 CST [18521] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.144 CST [18521] LOG:  disconnection: session time: 0:00:00.007 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.151 CST [18524] LOG:  connection received: host=[local]
2026-11-06 08:46:18.151 CST [18524] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.161 CST [18524] LOG:  disconnection: session time: 0:00:00.009 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.169 CST [18526] LOG:  connection received: host=[local]
2026-11-06 08:46:18.169 CST [18526] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.183 CST [18526] LOG:  disconnection: session time: 0:00:00.014 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.191 CST [18529] LOG:  connection received: host=[local]
2026-11-06 08:46:18.191 CST [18529] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.196 CST [18529] LOG:  disconnection: session time: 0:00:00.005 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.204 CST [18531] LOG:  connection received: host=[local]
2026-11-06 08:46:18.204 CST [18531] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.209 CST [18531] LOG:  disconnection: session time: 0:00:00.005 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.217 CST [18533] LOG:  connection received: host=[local]
2026-11-06 08:46:18.217 CST [18533] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.225 CST [18533] LOG:  disconnection: session time: 0:00:00.008 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.232 CST [18535] LOG:  connection received: host=[local]
2026-11-06 08:46:18.233 CST [18535] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.235 CST [18535] LOG:  disconnection: session time: 0:00:00.002 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.241 CST [18537] LOG:  connection received: host=[local]
2026-11-06 08:46:18.241 CST [18537] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.243 CST [18537] LOG:  disconnection: session time: 0:00:00.002 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.250 CST [18539] LOG:  connection received: host=[local]
2026-11-06 08:46:18.250 CST [18539] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.251 CST [18539] LOG:  disconnection: session time: 0:00:00.001 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.257 CST [18541] LOG:  connection received: host=[local]
2026-11-06 08:46:18.257 CST [18541] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.259 CST [18541] LOG:  disconnection: session time: 0:00:00.002 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.266 CST [18543] LOG:  connection received: host=[local]
2026-11-06 08:46:18.266 CST [18543] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.267 CST [18543] LOG:  disconnection: session time: 0:00:00.001 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.273 CST [18545] LOG:  connection received: host=[local]
2026-11-06 08:46:18.273 CST [18545] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.275 CST [18545] LOG:  disconnection: session time: 0:00:00.001 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.281 CST [18547] LOG:  connection received: host=[local]
2026-11-06 08:46:18.281 CST [18547] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.282 CST [18547] LOG:  disconnection: session time: 0:00:00.000 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.288 CST [18550] LOG:  connection received: host=[local]
2026-11-06 08:46:18.288 CST [18550] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.290 CST [18550] LOG:  disconnection: session time: 0:00:00.001 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.296 CST [18552] LOG:  connection received: host=[local]
2026-11-06 08:46:18.296 CST [18552] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.298 CST [18552] LOG:  disconnection: session time: 0:00:00.002 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.304 CST [18554] LOG:  connection received: host=[local]
2026-11-06 08:46:18.304 CST [18554] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.307 CST [18554] LOG:  disconnection: session time: 0:00:00.003 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.313 CST [18556] LOG:  connection received: host=[local]
2026-11-06 08:46:18.314 CST [18556] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.315 CST [18556] LOG:  disconnection: session time: 0:00:00.001 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.321 CST [18558] LOG:  connection received: host=[local]
2026-11-06 08:46:18.321 CST [18558] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.322 CST [18558] LOG:  disconnection: session time: 0:00:00.001 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.328 CST [18560] LOG:  connection received: host=[local]
2026-11-06 08:46:18.329 CST [18560] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.330 CST [18560] LOG:  disconnection: session time: 0:00:00.001 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.336 CST [18562] LOG:  connection received: host=[local]
2026-11-06 08:46:18.336 CST [18562] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.338 CST [18562] LOG:  disconnection: session time: 0:00:00.002 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.344 CST [18564] LOG:  connection received: host=[local]
2026-11-06 08:46:18.344 CST [18564] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.350 CST [18564] LOG:  disconnection: session time: 0:00:00.005 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.356 CST [18566] LOG:  connection received: host=[local]
2026-11-06 08:46:18.356 CST [18566] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.358 CST [18566] LOG:  disconnection: session time: 0:00:00.002 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.364 CST [18568] LOG:  connection received: host=[local]
2026-11-06 08:46:18.364 CST [18568] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.371 CST [18568] LOG:  disconnection: session time: 0:00:00.007 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.378 CST [18570] LOG:  connection received: host=[local]
2026-11-06 08:46:18.378 CST [18570] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.379 CST [18570] LOG:  disconnection: session time: 0:00:00.001 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.385 CST [18572] LOG:  connection received: host=[local]
2026-11-06 08:46:18.385 CST [18572] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.388 CST [18572] LOG:  disconnection: session time: 0:00:00.003 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.394 CST [18574] LOG:  connection received: host=[local]
2026-11-06 08:46:18.395 CST [18574] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.396 CST [18574] LOG:  disconnection: session time: 0:00:00.001 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.402 CST [18576] LOG:  connection received: host=[local]
2026-11-06 08:46:18.402 CST [18576] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.405 CST [18576] LOG:  disconnection: session time: 0:00:00.003 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.412 CST [18578] LOG:  connection received: host=[local]
2026-11-06 08:46:18.412 CST [18578] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.413 CST [18578] LOG:  disconnection: session time: 0:00:00.001 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.420 CST [18580] LOG:  connection received: host=[local]
2026-11-06 08:46:18.420 CST [18580] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.426 CST [18580] LOG:  disconnection: session time: 0:00:00.006 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.433 CST [18582] LOG:  connection received: host=[local]
2026-11-06 08:46:18.433 CST [18582] LOG:  connection authorized: user=postgres database=svc_workflow_cutover_test application_name=psql
2026-11-06 08:46:18.433 CST [18582] ERROR:  LOCK TABLE can only be used in transaction blocks
2026-11-06 08:46:18.433 CST [18582] STATEMENT:  LOCK TABLE workflow_command_receipts IN ACCESS EXCLUSIVE MODE;
2026-11-06 08:46:18.433 CST [18582] LOG:  disconnection: session time: 0:00:00.000 user=postgres database=svc_workflow_cutover_test host=[local]
2026-11-06 08:46:18.438 CST [18501] LOG:  received fast shutdown request
2026-11-06 08:46:18.439 CST [18501] LOG:  aborting any active transactions
2026-11-06 08:46:18.439 CST [18501] LOG:  background worker "logical replication launcher" (PID 18507) exited with exit code 1
2026-11-06 08:46:18.439 CST [18502] LOG:  shutting down
2026-11-06 08:46:18.439 CST [18502] LOG:  checkpoint starting: shutdown immediate
2026-11-06 08:46:18.519 CST [18502] LOG:  checkpoint complete: wrote 1974 buffers (12.0%); 0 WAL file(s) added, 0 removed, 0 recycled; write=0.066 s, sync=0.015 s, total=0.080 s; sync files=773, longest=0.001 s, average=0.001 s; distance=10104 kB, estimate=10104 kB; lsn=0/1EFBEC8, redo lsn=0/1EFBEC8
2026-11-06 08:46:18.522 CST [18501] LOG:  database system is shut down
```

### Filtered target-process post-state

```json
{
  "method": "ps -axo pid=,command=; retain only postgres processes for the exact data directory",
  "matches": [],
  "rc": 0,
  "observed_at": "2026-11-06T00:46:18.573314+00:00"
}
```

### Recorded execution result

```json
{
  "source_head": "bcd8950466091f4cb4e0133c431186d9267ac2c2",
  "base_head": "59daec042786f0416a6a9061c0b638ba56c0d83e",
  "environment": {
    "HOME": "/Users/yanfenma",
    "USER": "yanfenma",
    "LOGNAME": "yanfenma",
    "PATH": "/opt/homebrew/opt/postgresql@16/bin/:/Users/yanfenma/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin",
    "TMPDIR": "/private/tmp",
    "PSQLRC": "/dev/null",
    "LC_ALL": "C",
    "LANG": "C",
    "TRUSTED_FLEET_LOCAL_PG": "1"
  },
  "suite_rc": 3,
  "conformance_execution": "EXECUTED",
  "start_rc": 0,
  "stop_rc": 0,
  "attempts": 1,
  "production_db_connections": 0,
  "production_db_or_service_writes": 0,
  "system_service_registration": "NONE",
  "error": "conformance failed rc=3",
  "runner_sha256": "2264bebc411e191a16433ed4cba66f8a97635c23f7be023689af2664bd438947",
  "fixture_bindings": [
    {
      "path": "/private/tmp/svc-h06-manifest/workflow_trusted_fleet_successor_plan_v2.json",
      "size": 540472,
      "sha256": "0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606"
    },
    {
      "path": "/Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json",
      "size": 540472,
      "sha256": "0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606"
    }
  ],
  "regulator_start_log_sha256": "f650ce809c87264e3868d0bd7c31d75d402a22a4fc7a50c0ca4e5f4e51cd3eb8",
  "identity": {
    "data_directory": "/private/tmp/h06-tier2-pg",
    "port": "55432",
    "socket": "/private/tmp/h06-tier2-pg",
    "listen_addresses": "",
    "server_version_num": "160015",
    "user": "postgres",
    "is_superuser": "on",
    "server_ip": null,
    "database": "postgres",
    "databases": [
      "postgres",
      "template0",
      "template1"
    ]
  },
  "status_after_rc": 3,
  "postmaster_pid_exists": false,
  "socket_paths_after": [],
  "data_mode_after": "0o700",
  "data_retained": true,
  "postgres_processes_after": [],
  "source_hashes_unchanged": true,
  "regulator_start_log_unchanged": true,
  "result": "STOPPED",
  "completed_at": "2026-11-06T00:46:18.594869+00:00"
}
```

