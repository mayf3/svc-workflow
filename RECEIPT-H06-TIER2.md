# H06 Tier-2 isolated conformance receipt — STOPPED

Date: 2026-11-06. Refs: [svc-workflow #129](https://github.com/mayf3/svc-workflow/pull/129),
[agent-control #1360](https://github.com/mayf3/agent-control/issues/1360).

```text
RESULT = STOPPED_AT_POSTGRES_START
INITDB_RC = 0
PG_START_RC = 1
CONFORMANCE_EXECUTION = NOT_RUN
CONFORMANCE_RC = NOT_APPLICABLE
ASSERTION_OUTPUT = NONE (test executable never invoked)
MERGE_READY = NO
PRODUCTION_DB_CONNECTIONS = 0
PRODUCTION_DB_OR_SERVICE_WRITES = 0
SYSTEM_SERVICE_REGISTRATION = NONE
RETRY = NONE
DATA_DIRECTORY = /private/tmp/h06-tier2-pg (retained, mode 0700)
EVIDENCE_DIRECTORY = /private/tmp/h06-tier2-evidence (retained, mode 0700)
```

## Coordinates, authorization and unchanged semantics

Repository: `mayf3/svc-workflow`; worktree `/private/tmp/svc-h06-manifest`;
branch `ac-h06-migration-manifest`; evaluated source HEAD
`5a7dc9ee2dde1dd730dae94afb39f116160aec2c`; base/main
`59daec042786f0416a6a9061c0b638ba56c0d83e`, freshly matched by GitHub read
and `git ls-remote origin` before execution. The durable Owner mandate is
`BRIEF-H06-TIER2.md`; route and one-attempt runbook: `PREFLIGHT-H06-TIER2.md`.
The existing cutover child Spec remains proposed/none; no Product Contract,
acceptance, merge or production authority is created by this validation.
Prior R2 Tier-1 evidence remains unchanged; its pending Tier-2 is not closed.

Both the checkout fixture and the test's frozen absolute fixture path
`/Users/yanfenma/workspace/project/svc-workflow/workflow_trusted_fleet_successor_plan_v2.json`
were read and matched at 540472 bytes, SHA256
`0a05ed2d6099601a567d0ebf652e9adc737e8dd7c4c9dfc1260a6037c49f3606`.
All tracked Rust source, tests, migrations, Cargo files and checkout fixture
hashes matched before/after; full manifests are archived. Only the runner was
minimally adapted for an opt-in socket path. Its SHA256 is
`85fcf8e54f3474828d572c7567f8ed793c18cbf167be52f6ff0fc8b51e9b6b64`.
The `runner-diff` section below is the complete exact patch. Original Docker
mode remains default, and original migration SQL/order, Cargo locked command,
RUSTFLAGS, thread count, frozen fixture, tests and assertions are unchanged.
`bash -n` and `git diff --check` passed. Independent static isolation review
PASS, blocker=0, at that exact script hash; this is not runtime conformance.

## Execution and failure boundary

Tools: Homebrew PostgreSQL 16.15 at
`/opt/homebrew/opt/postgresql@16/bin`. No brew services or launchctl command
was used. Existing instances on TCP 55432/55439 were observed via process/lsof
only; no DB connection or mutation was made to them. New data path was absent
before initdb. The requested new cluster port was 55487, with `-h ''` (no TCP),
`-k /private/tmp/h06-tier2-pg`, socket permission 0700, local trust and host
reject. Complete initdb/start parameters are in the command transcript below.

The supervisor passed only this explicit environment, removing inherited DB
URLs, PG services, credentials, dotenv and flags:

```json
{
  "HOME": "/Users/yanfenma",
  "USER": "yanfenma",
  "LOGNAME": "yanfenma",
  "PATH": "/opt/homebrew/opt/postgresql@16/bin/:/Users/yanfenma/.cargo/bin:/opt/homebrew/bin:/usr/bin:/bin:/usr/sbin:/sbin",
  "TMPDIR": "/private/tmp",
  "PSQLRC": "/dev/null",
  "TRUSTED_FLEET_LOCAL_PG": "1"
}
```

This environment did not explicitly set `LC_ALL` or `LANG`. The PostgreSQL log
records startup, Unix socket creation, then `FATAL: postmaster became
multithreaded during startup`, with the hint `Set the LC_ALL environment
variable to a valid locale`, and `database system is shut down`. These are
observed diagnostics; no repaired environment or second startup was attempted,
so successful remediation is UNKNOWN.

`pg_ctl start` returned 1. The one-attempt supervisor immediately stopped;
no psql identity connection, DB creation, migrations, Cargo/test execution or
assertion ran. The adapted runner itself was never invoked. A successful
initdb bootstrap is not a successful database-server or Tier-2 test result.

Startup exited on its own. `pg_ctl status` returned 3 (no server). Explicit
`pg_ctl -D /private/tmp/h06-tier2-pg -m fast -w -t 30 stop` was then issued;
it returned 1 because postmaster.pid was already absent. It was a cleanup
request, not a new startup or test attempt. PID/socket absence and retained
0700 data directory were checked; independent post-state verification follows
in the review record. No assertion is claimed PASS. The existing suite, if
later run under a new mandate, contains only three tests: frozen fixture/counts,
closed CLI, and one empty-DB/canary COMMITTED/NOOP scenario. It does not prove
full fleet apply or all recovery replay integrations.

## Complete raw lifecycle output and exact runner diff

The following is all stdout/stderr from every attempted command, including
initial status, exact patch, initdb, failed start, status, cleanup stop and
complete server log. There is no test output because the test command was
not reached. Raw files are also retained in the evidence directory.

### git-before

Command: `git status --short`

Start: 2026-11-06T00:31:17.576258+00:00; end: 2026-11-06T00:31:17.592351+00:00; rc: `0`.

```text
 M scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
?? BRIEF-H06-TIER2.md
?? PREFLIGHT-H06-TIER2.md
```

### runner-diff

Command: `git diff HEAD -- scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh`

Start: 2026-11-06T00:31:17.592565+00:00; end: 2026-11-06T00:31:17.603939+00:00; rc: `0`.

```text
diff --git a/scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh b/scripts/run_trusted_fleet_principal_cutover_v1_conformance.sh
index ce8652e..d7cb15c 100755
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
+  BASE="postgres://postgres@localhost:55487"
+  SUFFIX="?host=/private/tmp/h06-tier2-pg"
+  IDENTITY="$(psql "${BASE}/postgres${SUFFIX}" -XAt -v ON_ERROR_STOP=1 -c \
+    "SELECT current_setting('data_directory') = '/private/tmp/h06-tier2-pg'
+       AND current_setting('port') = '55487'
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

Command: `/opt/homebrew/opt/postgresql@16/bin/postgres --version`

Start: 2026-11-06T00:31:17.604189+00:00; end: 2026-11-06T00:31:17.695921+00:00; rc: `0`.

```text
postgres (PostgreSQL) 16.15 (Homebrew)
```

### initdb

Command: `/opt/homebrew/opt/postgresql@16/bin/initdb -D /private/tmp/h06-tier2-pg -U postgres --auth-local=trust --auth-host=reject --encoding=UTF8 --locale=en_US.UTF-8`

Start: 2026-11-06T00:31:17.755789+00:00; end: 2026-11-06T00:31:18.140895+00:00; rc: `0`.

```text
属于此数据库系统的文件宿主为用户 "yanfenma".
此用户也必须为服务器进程的宿主.
数据库簇将使用本地化语言 "en_US.UTF-8"进行初始化.
缺省的文本搜索配置将会被设置到"english"

禁止为数据页生成校验和.

创建目录 /private/tmp/h06-tier2-pg ... 成功
正在创建子目录 ... 成功
选择动态共享内存实现 ......posix
选择默认最大联接数 (max_connections) ... 100
选择默认共享缓冲区大小 (shared_buffers) ... 128MB
选择默认时区 ... Asia/Shanghai
创建配置文件 ... 成功
正在运行自举脚本 ...成功
正在执行自举后初始化 ...成功
同步数据到磁盘...成功

成功。您现在可以用下面的命令开启数据库服务器：

    '/opt/homebrew/opt/postgresql@16/bin/pg_ctl' -D /private/tmp/h06-tier2-pg -l 日志文件 start

```

### pg-start

Command: `/opt/homebrew/opt/postgresql@16/bin/pg_ctl -D /private/tmp/h06-tier2-pg -l /private/tmp/h06-tier2-evidence/postgres.log -w -t 30 -o '-p 55487 -h '"'"''"'"' -k /private/tmp/h06-tier2-pg -c unix_socket_permissions=0700 -c log_connections=on -c log_disconnections=on' start`

Start: 2026-11-06T00:31:18.141117+00:00; end: 2026-11-06T00:31:18.270238+00:00; rc: `1`.

```text
等待服务器进程启动 .... 已停止等待
pg_ctl: 无法启动服务器进程
检查日志输出.
```

### pg-status-after

Command: `/opt/homebrew/opt/postgresql@16/bin/pg_ctl -D /private/tmp/h06-tier2-pg status`

Start: 2026-11-06T00:31:18.270580+00:00; end: 2026-11-06T00:31:18.279853+00:00; rc: `3`.

```text
pg_ctl:没有服务器进程正在运行
```

### pg-stop

Command: `/opt/homebrew/opt/postgresql@16/bin/pg_ctl -D /private/tmp/h06-tier2-pg -m fast -w -t 30 stop`

Start: 2026-11-06T00:32:24.758050+00:00; end: 2026-11-06T00:32:24.767496+00:00; rc: `1`.

```text
pg_ctl: PID 文件 "/private/tmp/h06-tier2-pg/postmaster.pid" 不存在
服务器进程是否正在运行?
```

### PostgreSQL server log (complete)

```text
2026-11-06 08:31:18.170 CST [10192] LOG:  starting PostgreSQL 16.15 (Homebrew) on aarch64-apple-darwin25.6.0, compiled by Apple clang version 21.0.0 (clang-2100.1.1.101), 64-bit
2026-11-06 08:31:18.170 CST [10192] LOG:  listening on Unix socket "/private/tmp/h06-tier2-pg/.s.PGSQL.55487"
2026-11-06 08:31:18.170 CST [10192] FATAL:  postmaster became multithreaded during startup
2026-11-06 08:31:18.170 CST [10192] HINT:  Set the LC_ALL environment variable to a valid locale.
2026-11-06 08:31:18.170 CST [10192] LOG:  database system is shut down
```

## Durable raw artifacts

Complete raw logs, exact sanitized environment/commands, result JSON and all
tracked source hashes before/after are copied byte-for-byte into
`docs/reports/h06-tier2-20261106/`. `SHA256SUMS` binds every archived file.
Data files stay at the retained local data directory and are not committed.

```text
172f3c7dc8fcb0b6623937800f028cb2ff10ada3a28692a969463a3a0eae921b  commands.json
ec536ee4ea5016b4ef278f57b1a0102d105c1907df26ffdd314e507a9bbbe850  git-before.log
ecb7bb67cae1141a93b2510327f5cd50d8bb442ae7e3cc1f86ac9cc7310c46a5  initdb.log
400fde295577ae52d07f51cadc885f883be82d0828ffa4c057a3a8c944bef49e  pg-start.log
725ad518760eb293c0b9628dce975adfea830e566d2f318c56503cffa609875b  pg-status-after.log
1de1e4f991d3ed27fa600b8078e052d494444e4dd3fb02c0f06a8c2b5a470500  pg-stop.log
ccf56037739c623f4a93c13504fe287ca39d124deece08982a631329e252f2b5  postgres.log
94b693becf5d64d90491e9269a0bf8bf93cab819b8a46acc66bcbd98b3a52bb6  result.json
5f4dbed4f239f9de0417cf20708a882c722c20a94b83d0770451c5ef19112b28  runner-diff.log
45ead84b1d1dcdfe2028dce6214eb61835adbd34eafbe16892d87f4b8aa7f88e  source-hashes-after.json
45ead84b1d1dcdfe2028dce6214eb61835adbd34eafbe16892d87f4b8aa7f88e  source-hashes-before.json
35d68bbf12bf8662960742f3fafe274c03f333d678a3672c656f9f6bd59eac85  tool-version.log
```
