#!/usr/bin/env bash
#
# svc-workflow 正式发布唯一入口（build → provenance → deploy → ledger → restart → verify）。
#
#   scripts/release.sh <sourceSha>            # 完整流水线（默认）
#   scripts/release.sh build  <sourceSha>     # 仅构建：clean worktree + release build + provenance.json
#   scripts/release.sh deploy <sourceSha>     # 仅部署：校验 provenance → 备份 → 安装 → ledger → restart
#   scripts/release.sh verify <sourceSha>     # 仅机械验收：/version vs provenance vs 运行中 binary sha256
#
# 信任链：clean Git tree → build artifact → artifact SHA256 → deployment record → running binary。
# 正式部署不允许绕过本脚本手抄 cp + launchctl restart。
#
# 环境变量覆盖（默认值即 dogfood 部署路径）：
#   SVC_WORKFLOW_SERVICE_DIR  部署目录（默认 ~/.local/services/svc-workflow）
#   SVC_WORKFLOW_PORT         /version 探测端口（默认 8989）
#   AUTH_TOKEN                可选：部署后基础认证请求使用的 Bearer token
#   EXPECTED_SERVICE_UID      预期服务 UID（#683 G1，与调用 EUID 分开；root 调用必填）
#   EXPECTED_PREIMAGE_SHA256  必填（#683 G3/P3）：目标 binary 的精确 sha256（无首装例外）
#   EXPECTED_LAUNCHCTL_TARGET 仅当 ledger 尚不存在时必填（#683 P5）：固定目标绑定，
#                             取值须来自已批准证据（如 REALM-MISMATCH 只读记录 actualLoaded）
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SERVICE_DIR="${SVC_WORKFLOW_SERVICE_DIR:-$HOME/.local/services/svc-workflow}"
PORT="${SVC_WORKFLOW_PORT:-8989}"
LABEL="com.svc-workflow"
BINARY="svc-workflow"
RELEASES_DIR="$SERVICE_DIR/releases"
LEDGER="$SERVICE_DIR/ledger.json"
BASE_URL="http://127.0.0.1:$PORT"
GIT=$(command -v git)
SHASUM=$(command -v shasum)
RELEASE_WT=""

log() { printf '[release] %s\n' "$*"; }
fail() { printf '[release] ERROR: %s\n' "$*" >&2; exit 1; }

now_iso() { date -u +%Y-%m-%dT%H:%M:%SZ; }

assert_source_sha() {
  local sha="$1"
  [[ "$sha" =~ ^[0-9a-f]{40}$ ]] || fail "sourceSha 必须是完整 40 位 hex SHA，得到: $sha"
  "$GIT" -C "$REPO_ROOT" rev-parse --verify "$sha^{commit}" >/dev/null 2>&1 \
    || fail "commit 不存在于本仓库: $sha"
}

# 计算 migration bundle digest：bundle 内全部 .sql 文件按名排序后逐文件
# SHA-256，再对汇总列表整体 SHA-256。同内容 -> 同 digest（可机械复核）。
migration_bundle_digest() {
  local dir="$1"
  (cd "$dir" && find migrations -name '*.sql' -type f | sort | xargs "$SHASUM" -a 256) \
    | "$SHASUM" -a 256 | awk '{print $1}'
}

# bundle 内最高 migration 版本号（文件名 <NNNN>_*.sql 前缀）
migration_max_version() {
  local dir="$1"
  ls "$dir"/migrations/[0-9]*_*.sql 2>/dev/null | sed -E 's#.*/##; s/_.*//' | sort -n | tail -1
}

# 校验 provenance.json 与 binary 一致；输出 provenance 字段
load_provenance() {
  local sha="$1"
  local dir="$RELEASES_DIR/$sha"
  [[ -f "$dir/provenance.json" ]] || fail "缺少 provenance: $dir/provenance.json（先运行 build）"
  [[ -f "$dir/$BINARY" ]] || fail "缺少 artifact: $dir/${BINARY}（先运行 build）"
  [[ -d "$dir/migrations" ]] || fail "缺少 migration bundle: $dir/migrations（先运行 build）"

  local tree_state artifact_sha256 built_at migration_max migration_digest
  tree_state="$(jq -r '.treeState' "$dir/provenance.json")"
  artifact_sha256="$(jq -r '.artifactSha256' "$dir/provenance.json")"
  built_at="$(jq -r '.builtAt' "$dir/provenance.json")"
  migration_max="$(jq -r '.migrationMaxVersion' "$dir/provenance.json")"
  migration_digest="$(jq -r '.migrationBundleDigest' "$dir/provenance.json")"

  [[ "$tree_state" == "clean" ]] || fail "provenance.treeState != clean: $tree_state"
  [[ "$artifact_sha256" =~ ^[0-9a-f]{64}$ ]] || fail "provenance.artifactSha256 非法: $artifact_sha256"
  [[ "$built_at" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}T ]] || fail "provenance.builtAt 非法: $built_at"
  [[ "$migration_max" =~ ^[0-9]+$ ]] || fail "provenance.migrationMaxVersion 非法: $migration_max"
  [[ "$migration_digest" =~ ^[0-9a-f]{64}$ ]] || fail "provenance.migrationBundleDigest 非法: $migration_digest"

  local actual
  actual="$("$SHASUM" -a 256 "$dir/$BINARY" | awk '{print $1}')"
  [[ "$actual" == "$artifact_sha256" ]] || fail "artifact 与 provenance 不一致: actual=$actual expected=$artifact_sha256"

  # 回归防线：bundle 内容必须与 provenance 声明一致（缺 N / 缺文件 -> fail）
  local actual_max actual_digest
  actual_max="$(migration_max_version "$dir")"
  actual_digest="$(migration_bundle_digest "$dir")"
  [[ "$actual_max" == "$migration_max" ]] || fail "bundle 最高版本与 provenance 不一致: actual=$actual_max expected=$migration_max"
  [[ "$actual_digest" == "$migration_digest" ]] || fail "bundle digest 与 provenance 不一致"

  jq -n \
    --arg sourceSha "$sha" \
    --arg treeState "$tree_state" \
    --arg artifactSha256 "$artifact_sha256" \
    --arg builtAt "$built_at" \
    --arg migrationMaxVersion "$migration_max" \
    --arg migrationBundleDigest "$migration_digest" \
    '{sourceSha: $sourceSha, treeState: $treeState, artifactSha256: $artifactSha256, builtAt: $builtAt, migrationMaxVersion: $migrationMaxVersion, migrationBundleDigest: $migrationBundleDigest}'
}

# 解析唯一的 svc-workflow launchd 目标（gui/UID 或 system，二选一）。
# 必须在 ANY live 写入前调用；同一绑定供 deploy/restart/verify/rollback 共用。
# print 结果按 present running / present stopped / absent / denied_unknown 分类
# （#683 G4）：任一 denied_unknown 先行拒绝（另一 realm 的成功不得掩盖探测异常）；
# present>1（含 running+stopped 组合）= 域歧义；present=0 = 无 daemon；全部零写入。
# unit 顶层键为真实输出的单 TAB 缩进（嵌套块两 TAB 起，不得误读）；
# 非秘密输出样本来源：2026-10-08 只读证据 REALM-MISMATCH.json。
# 身份（#683 G1）：预期服务 UID（EXPECTED_SERVICE_UID）与调用 EUID 分开；GUI
# 候选以服务 UID 探测；运行单位以 ps 实核运行 UID（root/声明不得替代）；停止
# 单位身份只能取自既有 unit 配置（uid 行），不能核则拒；不改实际权限或服务。
LAUNCHCTL_TARGET=""
SERVICE_PID=""
UNIT_PROGRAM=""
SERVICE_UID=""
GUI_STATE="absent"
SYS_STATE="absent"
OTHER_REALM_STATE="none"

# launchctl print 顶层字段（恰好单 TAB 缩进）解析；嵌套块（两 TAB 起）不匹配
unit_field() {
  awk -F'= ' -v pat="^\t$2 = " '$0 ~ pat {print $2; exit}' <<<"${1:-}"
}

# 路径规范化等值：解析已存在目录的符号链接后比较；不同路径即使内容同 hash 也不等价
norm_path() {
  local d b
  d="$(dirname "$1")"
  b="$(basename "$1")"
  if [[ -d "$d" ]]; then d="$(cd "$d" && pwd -P)"; fi
  printf '%s/%s' "$d" "$b"
}

# 服务身份预期：EXPECTED_SERVICE_UID 未声明时回退调用 EUID；root 回退被拒
# （系统域服务以专用 UID 运行时，root 回退 0 必然错绑且属替代身份验证）
require_expected_service_uid() {
  local euid
  euid="$(id -u)"
  if [[ -z "${EXPECTED_SERVICE_UID:-}" ]]; then
    [[ "$euid" != "0" ]] \
      || fail "service identity unverifiable: EXPECTED_SERVICE_UID must be set when running as root (root must not substitute the service identity); zero live writes performed"
    EXPECTED_SERVICE_UID="$euid"
  fi
}

resolve_service_target() {
  require_expected_service_uid
  local gui_target="gui/$EXPECTED_SERVICE_UID/$LABEL"
  local sys_target="system/$LABEL"
  local t rc out err pid program uid_val
  local present=0 denied_unknown=0
  LAUNCHCTL_TARGET=""; SERVICE_PID=""; UNIT_PROGRAM=""; SERVICE_UID=""
  GUI_STATE="absent"; SYS_STATE="absent"; OTHER_REALM_STATE="none"

  for t in "$gui_target" "$sys_target"; do
    rc=0
    err="$(mktemp)"
    out="$(launchctl print "$t" 2>"$err")" || rc=$?
    if [[ $rc -eq 0 ]]; then
      pid="$(unit_field "$out" "pid")"
      program="$(unit_field "$out" "program")"
      uid_val="$(unit_field "$out" "uid")"
      if [[ "$t" == "$gui_target" ]]; then
        GUI_STATE="present"; [[ -n "$pid" ]] || GUI_STATE="present_stopped"
      else
        SYS_STATE="present"; [[ -n "$pid" ]] || SYS_STATE="present_stopped"
      fi
      present=$((present+1))
      LAUNCHCTL_TARGET="$t"; SERVICE_PID="$pid"; UNIT_PROGRAM="$program"; SERVICE_UID="$uid_val"
    elif grep -qi "could not find service" "$err" 2>/dev/null; then
      if [[ "$t" == "$gui_target" ]]; then GUI_STATE="absent"; else SYS_STATE="absent"; fi
    else
      if [[ "$t" == "$gui_target" ]]; then GUI_STATE="denied_unknown"; else SYS_STATE="denied_unknown"; fi
      denied_unknown=$((denied_unknown+1))
    fi
    rm -f "$err"
  done

  if [[ $denied_unknown -gt 0 ]]; then
    fail "service target resolution: denied/unknown launchctl print result (gui=$GUI_STATE system=$SYS_STATE); zero live writes performed"
  fi
  if [[ $present -eq 0 ]]; then
    fail "service target resolution: no unit found (gui=$GUI_STATE system=$SYS_STATE); zero live writes performed"
  fi
  if [[ $present -gt 1 ]]; then
    fail "service target resolution: ambiguous, $present present units (gui=$GUI_STATE system=$SYS_STATE); zero live writes performed"
  fi

  # unit program 绑定（#683 G2）：launchd 配置的可执行文件必须就是本脚本管理的
  # 安装目标（路径规范化等值；同 hash 异路径不等价），否则任何写入都指向错误安装
  if [[ -z "$UNIT_PROGRAM" ]]; then
    fail "service target resolution: unit program UNKNOWN from launchctl print; zero live writes performed"
  fi
  if [[ "$(norm_path "$UNIT_PROGRAM")" != "$(norm_path "$SERVICE_DIR/$BINARY")" ]]; then
    fail "service target resolution: unit program ($UNIT_PROGRAM) != $SERVICE_DIR/$BINARY; refusing to manage a foreign install (same-hash different-path is not equivalent); zero live writes performed"
  fi

  # 身份实核（#683 G1）：运行单位 ps 实核运行 UID，unit 配置 uid 与之互证；
  # 停止单位身份只能取自既有 unit 配置，缺即拒（不放宽、不以 root 替代）
  if [[ -n "$SERVICE_PID" ]]; then
    local ps_uid
    ps_uid="$(ps -o uid= -p "$SERVICE_PID" 2>/dev/null | tr -d '[:space:]')"
    if [[ "$ps_uid" != "$EXPECTED_SERVICE_UID" ]]; then
      fail "service process uid=${ps_uid:-unknown} != expected $EXPECTED_SERVICE_UID; refusing to bind target=$LAUNCHCTL_TARGET; zero live writes performed"
    fi
    if [[ -n "$SERVICE_UID" && "$SERVICE_UID" != "$ps_uid" ]]; then
      fail "service identity drift: unit config uid=$SERVICE_UID != running pid uid=$ps_uid; refusing target=$LAUNCHCTL_TARGET; zero live writes performed"
    fi
    # 实际运行 executable 绑定（#683 P2）：运行 PID 的可执行文件必须与固定
    # install target 规范化同路径——另一目录的同 hash 副本不等价，拒绝。
    # （停止单位无 PID：沿既有 unit 身份/配置/preimage，不伪造运行路径。）
    local run_path
    run_path="$(lsof -p "$SERVICE_PID" -a -d txt -Fn 2>/dev/null | sed -n 's/^n//p' | head -1)"
    if [[ -z "$run_path" ]]; then
      fail "running executable path unknown for pid $SERVICE_PID; refusing target=$LAUNCHCTL_TARGET; zero live writes performed"
    fi
    if [[ "$(norm_path "$run_path")" != "$(norm_path "$SERVICE_DIR/$BINARY")" ]]; then
      fail "running executable ($run_path) != install target $SERVICE_DIR/$BINARY (same-hash different-path is not equivalent); refusing target=$LAUNCHCTL_TARGET; zero live writes performed"
    fi
  else
    if [[ -z "$SERVICE_UID" ]]; then
      fail "stopped unit identity unverifiable: no uid in unit config for $LAUNCHCTL_TARGET; restore refused; zero live writes performed"
    fi
    if [[ "$SERVICE_UID" != "$EXPECTED_SERVICE_UID" ]]; then
      fail "stopped unit config uid=$SERVICE_UID != expected $EXPECTED_SERVICE_UID; refusing target=$LAUNCHCTL_TARGET; zero live writes performed"
    fi
  fi

  # system 域写动作的 root 准入（pre-write；gui 域即属主用户会话）
  if [[ "$LAUNCHCTL_TARGET" == system/* && "$(id -u)" != "0" ]]; then
    fail "system-domain control requires root execution (current euid $(id -u)); hand to the deploy Owner; zero live writes performed"
  fi

  if [[ "$LAUNCHCTL_TARGET" == "$gui_target" ]]; then
    OTHER_REALM_STATE="$SYS_STATE"
  else
    OTHER_REALM_STATE="$GUI_STATE"
  fi
  log "service target resolved: $LAUNCHCTL_TARGET (pid ${SERVICE_PID:-none}, uid $SERVICE_UID, program $UNIT_PROGRAM; other realm: $OTHER_REALM_STATE)"
}

# ledger 解析（#683 G5）：deploy 以 jq -n 追加 pretty 多行 JSON 对象，ledger 是
# 多个 JSON 值的拼接流——不是行分隔 JSONL。对原始文件逐行 tail 只会读到 '}'
# 且 jq 静默空结果（2cace 缺陷：binding 检查被跳过）。此处以 jq 解析完整流：
# 任一损坏段都使 jq 失败 → 显式拒绝；无匹配记录 → 显式失败。
ledger_last_record_for() {  # $1=sourceSha  $2=错误前缀；输出该 sha 最近一条记录（compact 单行）
  local sha="$1" prefix="$2" out
  [[ -f "$LEDGER" ]] || fail "$prefix: deployment ledger missing: $LEDGER"
  out="$(jq -c --arg sha "$sha" 'select(.sourceSha == $sha)' "$LEDGER")" \
    || fail "$prefix: deployment ledger corrupt (JSON stream parse failed): $LEDGER"
  out="$(printf '%s\n' "$out" | tail -1)"
  [[ -n "$out" ]] || fail "$prefix: no deployment ledger record for sourceSha=$sha in $LEDGER"
  printf '%s' "$out"
}

ledger_field() {  # $1=record(compact JSON)  $2=字段名  $3=错误前缀；输出非空字段值
  local v
  v="$(jq -r --arg f "$2" '.[$f] // empty' <<<"$1")"
  [[ -n "$v" ]] || fail "$3: ledger record missing $2"
  printf '%s' "$v"
}

# 返回运行中 svc-workflow 进程的 txt（可执行）文件路径；进程未运行则返回空
running_binary_path() {
  local pid
  pid="$(launchctl print "$LAUNCHCTL_TARGET" 2>/dev/null | awk -F'= ' '/pid = /{print $2; exit}')"
  [[ -n "$pid" && "$pid" =~ ^[0-9]+$ ]] || return 0
  lsof -p "$pid" -a -d txt -Fn 2>/dev/null | sed -n 's/^n//p' | head -1
}

build() {
  local sha="$1"
  assert_source_sha "$sha"

  # 1) 在全新 detached worktree 构建 → worktree 必然 clean（build.rs 亦强制）
  RELEASE_WT="$(mktemp -d "${TMPDIR:-/tmp}/svc-workflow-release.XXXXXX")"
  log "创建 clean worktree: $RELEASE_WT (commit $sha)"
  "$GIT" -C "$REPO_ROOT" worktree add --detach "$RELEASE_WT" "$sha" >/dev/null
  # 全局变量 + EXIT trap：无论成功失败都清理 worktree（local 变量在函数返回后不可用）
  trap 'git -C "$REPO_ROOT" worktree remove --force "$RELEASE_WT" >/dev/null 2>&1 || rm -rf "$RELEASE_WT"' EXIT

  log "release build（独立 CARGO_TARGET_DIR，确保产物只来自该 commit 的干净源码）"
  (cd "$RELEASE_WT" && cargo build --release --locked)

  local binary="$RELEASE_WT/target/release/$BINARY"
  [[ -f "$binary" ]] || fail "release build 未产出 $binary"

  local dir="$RELEASES_DIR/$sha"
  mkdir -p "$dir"

  # 2) 打包 migration bundle（与 binary 同源：同一 clean worktree = 同一 commit）
  cp -r "$RELEASE_WT/migrations" "$dir/migrations"

  # 回归防线：bundle 必须与该 commit 的 migration 树完全一致（缺文件/缺 N -> fail）
  local repo_sql_count bundle_sql_count
  repo_sql_count="$("$GIT" -C "$REPO_ROOT" ls-tree -r --name-only "$sha" migrations | grep -c '\.sql$')"
  bundle_sql_count="$(find "$dir/migrations" -name '*.sql' -type f | wc -l | tr -d ' ')"
  [[ "$bundle_sql_count" == "$repo_sql_count" ]] \
    || fail "migration bundle 不完整: bundle=$bundle_sql_count repo=$repo_sql_count (必须来自同一 commit)"

  # 3) 生成 provenance.json（含 migration bundle 证据）
  local artifact_sha256 built_at migration_max migration_digest
  artifact_sha256="$("$SHASUM" -a 256 "$binary" | awk '{print $1}')"
  built_at="$(now_iso)"
  migration_max="$(migration_max_version "$dir")"
  migration_digest="$(migration_bundle_digest "$dir")"
  cp "$binary" "$dir/$BINARY"
  chmod +x "$dir/$BINARY"
  jq -n \
    --arg sourceSha "$sha" \
    --arg treeState "clean" \
    --arg artifactSha256 "$artifact_sha256" \
    --arg builtAt "$built_at" \
    --arg buildCommand "cargo build --release --locked (clean detached worktree @ $sha)" \
    --arg migrationMaxVersion "$migration_max" \
    --arg migrationBundleDigest "$migration_digest" \
    '{sourceSha: $sourceSha, treeState: $treeState, artifactSha256: $artifactSha256, builtAt: $builtAt, buildCommand: $buildCommand, migrationMaxVersion: $migrationMaxVersion, migrationBundleDigest: $migrationBundleDigest}' \
    > "$dir/provenance.json"
  log "artifact 已归档: $dir/$BINARY"
  log "migration bundle 已归档: $dir/migrations (max=$migration_max, digest=$migration_digest)"
  log "provenance 已写入: $dir/provenance.json"
}

deploy() {
  local sha="$1"
  assert_source_sha "$sha"
  local provenance dir
  provenance="$(load_provenance "$sha")"
  dir="$RELEASES_DIR/$sha"

  # 0) 解析唯一服务目标（分类：present running/stopped、absent、denied_unknown）
  #    ——必须先于任何写入；program 绑定安装目标；身份实核；system 域 root 准入
  resolve_service_target

  # 0a) ledger 绑定（#683 G5/P5）：ledger 存在 → 以完整流最新记录为固定绑定，
  #     缺/坏/字段缺一律首写拒绝（回退=同 binding 恢复，不接受替代 realm）；
  #     ledger 缺失（首次以本入口更新既有服务）→ 必须以固定目标输入
  #     EXPECTED_LAUNCHCTL_TARGET 显式绑定（取值须来自已批准证据，如
  #     REALM-MISMATCH 只读记录 actualLoaded），缺失或不符即拒绝——
  #     缺 ledger 时不得重新发现另一 realm。
  local prior_target
  if [[ -f "$LEDGER" ]]; then
    local prior_line
    prior_line="$(jq -c '.' "$LEDGER")" \
      || fail "deployment ledger corrupt (JSON stream parse failed): $LEDGER; zero live writes performed"
    prior_line="$(printf '%s\n' "$prior_line" | tail -1)"
    [[ -n "$prior_line" ]] || fail "deployment ledger empty: $LEDGER; zero live writes performed"
    prior_target="$(ledger_field "$prior_line" "launchctlTarget" "deployment ledger")"
  else
    if [[ -z "${EXPECTED_LAUNCHCTL_TARGET:-}" ]]; then
      fail "deployment binding record missing: $LEDGER does not exist and EXPECTED_LAUNCHCTL_TARGET is not set; first update of an existing service must pin the binding from approved evidence; refusing pre-write; zero live writes performed"
    fi
    prior_target="$EXPECTED_LAUNCHCTL_TARGET"
  fi
  if [[ "$prior_target" != "$LAUNCHCTL_TARGET" ]]; then
    fail "binding drift: ledger records $prior_target but resolved $LAUNCHCTL_TARGET; rollback is same-binding only; zero live writes performed"
  fi

  # 0b) 目标 preimage 门（#683 G3/P3）：本入口只服务既有 service 的更新/回退，
  #     首装例外已废除——目标必须无条件存在、可读，且声明 EXPECTED_PREIMAGE_SHA256
  #     精确匹配；文件与声明同时缺失同样拒绝。不符一律零写入失败。
  local target_preimage_sha256=""
  [[ -f "$SERVICE_DIR/$BINARY" && -r "$SERVICE_DIR/$BINARY" ]] \
    || fail "target preimage missing: $SERVICE_DIR/$BINARY does not exist or is not a readable regular file; zero live writes performed"
  if [[ -z "${EXPECTED_PREIMAGE_SHA256:-}" ]]; then
    fail "declared preimage missing: EXPECTED_PREIMAGE_SHA256 must be set for an existing-service update; zero live writes performed"
  fi
  target_preimage_sha256="$("$SHASUM" -a 256 "$SERVICE_DIR/$BINARY" | awk '{print $1}')"
  if [[ "$target_preimage_sha256" != "$EXPECTED_PREIMAGE_SHA256" ]]; then
    fail "PREIMAGE MISMATCH: target sha256=$target_preimage_sha256 != declared $EXPECTED_PREIMAGE_SHA256; zero live writes performed"
  fi

  local artifact_sha256 migration_max migration_digest previous_sha256 deployed_at
  artifact_sha256="$(jq -r '.artifactSha256' <<<"$provenance")"
  migration_max="$(jq -r '.migrationMaxVersion' <<<"$provenance")"
  migration_digest="$(jq -r '.migrationBundleDigest' <<<"$provenance")"
  deployed_at="$(now_iso)"

  # 3) 备份当前 binary + migrations（记录 previousArtifactSha256）
  previous_sha256=""
  if [[ -f "$SERVICE_DIR/$BINARY" ]]; then
    local backup_path="$SERVICE_DIR/$BINARY.backup-$(date +%Y%m%d-%H%M%S)"
    mkdir -p "$backup_path"
    previous_sha256="$("$SHASUM" -a 256 "$SERVICE_DIR/$BINARY" | awk '{print $1}')"
    cp "$SERVICE_DIR/$BINARY" "$backup_path/"
    if [[ -d "$SERVICE_DIR/migrations" ]]; then
      cp -r "$SERVICE_DIR/migrations" "$backup_path/"
    fi
    log "已备份当前 binary+migrations → $backup_path ($previous_sha256)"
  fi

  # 4) 安装新 binary + exact migrations bundle（不可分割）
  install -m 0755 "$dir/$BINARY" "$SERVICE_DIR/$BINARY"
  rm -rf "$SERVICE_DIR/migrations"
  cp -r "$dir/migrations" "$SERVICE_DIR/migrations"

  local installed_sha256 deployed_max deployed_digest
  installed_sha256="$("$SHASUM" -a 256 "$SERVICE_DIR/$BINARY" | awk '{print $1}')"
  deployed_max="$(migration_max_version "$SERVICE_DIR")"
  deployed_digest="$(migration_bundle_digest "$SERVICE_DIR")"
  [[ "$installed_sha256" == "$artifact_sha256" ]] \
    || fail "DEPLOYMENT_FAIL: 安装后 binary 校验失败: installed=$installed_sha256 expected=$artifact_sha256"
  [[ "$deployed_max" == "$migration_max" ]] \
    || fail "DEPLOYMENT_FAIL: 部署目录 migrations 最高版本=$deployed_max != provenance=$migration_max (bundle 缺 migration)"
  [[ "$deployed_digest" == "$migration_digest" ]] \
    || fail "DEPLOYMENT_FAIL: 部署目录 migration bundle digest 与 provenance 不一致"
  log "已安装 binary + migrations bundle (max=$deployed_max, digest=$deployed_digest)"

  # 5) deployment ledger（JSONL，追加）
  mkdir -p "$SERVICE_DIR"
  jq -n \
    --arg deployedAt "$deployed_at" \
    --arg sourceSha "$sha" \
    --arg artifactSha256 "$artifact_sha256" \
    --arg previousArtifactSha256 "${previous_sha256:-}" \
    --arg migrationMaxVersion "$migration_max" \
    --arg migrationBundleDigest "$migration_digest" \
    --arg launchctlTarget "$LAUNCHCTL_TARGET" \
    --arg targetPreimage "$target_preimage_sha256" \
    --arg otherRealmState "$OTHER_REALM_STATE" \
    '{deployedAt: $deployedAt, sourceSha: $sourceSha, artifactSha256: $artifactSha256, previousArtifactSha256: $previousArtifactSha256, migrationMaxVersion: $migrationMaxVersion, migrationBundleDigest: $migrationBundleDigest, launchctlTarget: $launchctlTarget, targetPreimageSha256: $targetPreimage, otherRealmState: $otherRealmState}' \
    >> "$LEDGER"
  log "deployment ledger 已追加: $LEDGER"

  # 6) restart（与解析/写入同一绑定）
  launchctl print "$LAUNCHCTL_TARGET" >/dev/null 2>&1 \
    || fail "launchctl 服务不存在: $LAUNCHCTL_TARGET（先加载 plist）"
  log "restart $LAUNCHCTL_TARGET (launchctl kickstart -k)"
  launchctl kickstart -k "$LAUNCHCTL_TARGET"
}

# 等待 /version 可访问；返回响应体
wait_for_version() {
  local body="" i
  for i in $(seq 1 30); do
    body="$(curl -sf -m 2 "$BASE_URL/version" 2>/dev/null || true)"
    [[ -n "$body" ]] && { echo "$body"; return 0; }
    sleep 1
  done
  return 1
}

verify() {
  local sha="$1"
  assert_source_sha "$sha"

  # 7.0) 固定绑定（#683 G5）：从 ledger 完整 JSON 流取本 sourceSha 最近一条
  # 记录；记录缺/坏/必要字段缺一律失败。不重新做域解析、不接受替代 realm。
  local rec fixed_target
  rec="$(ledger_last_record_for "$sha" "VERIFY FAIL")"
  fixed_target="$(ledger_field "$rec" "launchctlTarget" "VERIFY FAIL")"
  ledger_field "$rec" "artifactSha256" "VERIFY FAIL" >/dev/null
  jq -e 'has("sourceSha") and has("targetPreimageSha256")' <<<"$rec" >/dev/null \
    || fail "VERIFY FAIL: ledger record missing required fields for sourceSha=$sha"
  LAUNCHCTL_TARGET="$fixed_target"
  require_expected_service_uid

  # 7.0b) 固定绑定当前状态核验：unit 仍 present 且 running、program 仍绑定本
  # 安装、运行 UID 仍等于服务 UID；记录的 realm 缺席即失败，绝不换 realm。
  local out rc=0 pid program ps_uid
  out="$(launchctl print "$LAUNCHCTL_TARGET" 2>/dev/null)" || rc=$?
  [[ $rc -eq 0 ]] \
    || fail "VERIFY FAIL: ledger-recorded binding $LAUNCHCTL_TARGET not present per launchctl print (rc=$rc); substitute realms are not accepted"
  pid="$(unit_field "$out" "pid")"
  [[ -n "$pid" ]] || fail "VERIFY FAIL: ledger-recorded binding $LAUNCHCTL_TARGET is not running"
  program="$(unit_field "$out" "program")"
  [[ "$(norm_path "${program:-}")" == "$(norm_path "$SERVICE_DIR/$BINARY")" ]] \
    || fail "VERIFY FAIL: binding drift — unit program (${program:-UNKNOWN}) != $SERVICE_DIR/$BINARY"
  ps_uid="$(ps -o uid= -p "$pid" 2>/dev/null | tr -d '[:space:]')"
  [[ "$ps_uid" == "$EXPECTED_SERVICE_UID" ]] \
    || fail "VERIFY FAIL: service process uid=${ps_uid:-unknown} != expected $EXPECTED_SERVICE_UID"
  log "ledger-recorded binding $LAUNCHCTL_TARGET verified (pid $pid, uid $ps_uid)"

  local provenance
  provenance="$(load_provenance "$sha")"

  local version_body
  version_body="$(wait_for_version)" || fail "服务在 30s 内未恢复 /version"

  # 7) 机械验收
  local running_sha running_tree artifact_sha256 actual_sha256 bin_path
  running_sha="$(jq -r '.gitSha' <<<"$version_body")"
  running_tree="$(jq -r '.gitTreeState' <<<"$version_body")"
  artifact_sha256="$(jq -r '.artifactSha256' <<<"$provenance")"

  [[ "$running_sha" == "$sha" ]] || fail "VERIFY FAIL: /version.gitSha=$running_sha != provenance.sourceSha=$sha"
  [[ "$running_tree" == "clean" ]] || fail "VERIFY FAIL: /version.gitTreeState=$running_tree != clean"
  log "/version.gitSha == $running_sha ✓"
  log "/version.gitTreeState == clean ✓"

  bin_path="$(running_binary_path)"
  [[ -n "$bin_path" ]] || fail "无法定位运行中进程的 binary 路径"
  if [[ "$bin_path" == *"(deleted)"* ]]; then
    fail "VERIFY FAIL: 运行中的 binary 文件已被替换（${bin_path}）"
  fi
  # 运行 binary 路径绑定（#683 P2）：运行中的可执行文件必须就是固定 install
  # target 本身（规范化同路径）——另一目录的同 hash 副本不得通过仅 hash 比对
  [[ "$(norm_path "$bin_path")" == "$(norm_path "$SERVICE_DIR/$BINARY")" ]] \
    || fail "VERIFY FAIL: running binary ($bin_path) != install target $SERVICE_DIR/$BINARY (same-hash different-path is not equivalent)"
  actual_sha256="$("$SHASUM" -a 256 "$bin_path" | awk '{print $1}')"
  [[ "$actual_sha256" == "$artifact_sha256" ]] \
    || fail "VERIFY FAIL: 运行中 binary sha256=$actual_sha256 != provenance.artifactSha256=$artifact_sha256"
  log "运行中 binary sha256 == $artifact_sha256 ✓ (path: $bin_path)"

  # 7b) migration bundle 机械验证（部署目录 vs provenance）
  local migration_max migration_digest deployed_max deployed_digest
  migration_max="$(jq -r '.migrationMaxVersion' <<<"$provenance")"
  migration_digest="$(jq -r '.migrationBundleDigest' <<<"$provenance")"
  deployed_max="$(migration_max_version "$SERVICE_DIR")"
  deployed_digest="$(migration_bundle_digest "$SERVICE_DIR")"
  [[ "$deployed_max" == "$migration_max" ]] \
    || fail "VERIFY FAIL: 部署目录 migrations max=$deployed_max != provenance=$migration_max"
  [[ "$deployed_digest" == "$migration_digest" ]] \
    || fail "VERIFY FAIL: 部署目录 migration bundle digest 与 provenance 不一致"
  log "migration bundle max=$deployed_max, digest=$deployed_digest == provenance ✓"

  # 8) 基础只读 HTTP smoke + 记录
  local healthz readyz auth_code
  healthz="$(curl -s -m 5 -o /dev/null -w '%{http_code}' "$BASE_URL/healthz" || echo 000)"
  readyz="$(curl -s -m 5 -o /dev/null -w '%{http_code}' "$BASE_URL/readyz" || echo 000)"
  # 只读认证端点：无 token 应 401；有 AUTH_TOKEN 则期望 200
  auth_code="$(curl -s -m 5 -o /dev/null -w '%{http_code}' "$BASE_URL/internal/v1/worklists/assigned-to-me" || echo 000)"
  if [[ -n "${AUTH_TOKEN:-}" ]]; then
    auth_code="$(curl -s -m 5 -o /dev/null -w '%{http_code}' -H "Authorization: Bearer $AUTH_TOKEN" "$BASE_URL/internal/v1/worklists/assigned-to-me" || echo 000)"
  fi
  log "smoke: healthz=$healthz readyz=$readyz auth(domains)=$auth_code"
  [[ "$healthz" == "200" ]] || fail "VERIFY FAIL: healthz=$healthz"
  if [[ "$readyz" != "200" ]]; then
    log "注意: readyz=${readyz}（已知独立问题：JWKS/auth 缓存，本轮只记录不修）"
  fi

  # 验收结果并入 ledger 最近一条
  jq -c \
    --arg healthz "$healthz" \
    --arg readyz "$readyz" \
    --arg authHttpStatus "$auth_code" \
    --arg runningBinaryPath "$bin_path" \
    '.verification = {healthz: $healthz, readyz: $readyz, authHttpStatus: $authHttpStatus, runningBinaryPath: $runningBinaryPath}' \
    "$LEDGER" > "$LEDGER.tmp" && mv "$LEDGER.tmp" "$LEDGER"

  log "VERIFY PASSED: 运行中的 svc-workflow = clean commit ${sha} 的产物（sha256 ${artifact_sha256}）"
}

main() {
  local cmd="${1:-all}"
  local sha="${2:-}"
  [[ -n "$sha" ]] || fail "用法: release.sh [build|deploy|verify|all] <sourceSha>"
  case "$cmd" in
    build) build "$sha" ;;
    deploy) deploy "$sha" ;;
    verify) verify "$sha" ;;
    all)
      build "$sha"
      deploy "$sha"
      verify "$sha"
      ;;
    *) fail "未知子命令: $cmd" ;;
  esac
}

main "$@"
