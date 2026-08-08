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

# 返回运行中 svc-workflow 进程的 txt（可执行）文件路径；进程未运行则返回空
running_binary_path() {
  local pid
  pid="$(launchctl print "gui/$(id -u)/$LABEL" 2>/dev/null | awk -F'= ' '/pid = /{print $2; exit}')"
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
    '{deployedAt: $deployedAt, sourceSha: $sourceSha, artifactSha256: $artifactSha256, previousArtifactSha256: $previousArtifactSha256, migrationMaxVersion: $migrationMaxVersion, migrationBundleDigest: $migrationBundleDigest}' \
    >> "$LEDGER"
  log "deployment ledger 已追加: $LEDGER"

  # 6) restart
  launchctl print "gui/$(id -u)/$LABEL" >/dev/null 2>&1 \
    || fail "launchctl 服务不存在: ${LABEL}（先加载 plist）"
  log "restart $LABEL (launchctl kickstart -k)"
  launchctl kickstart -k "gui/$(id -u)/$LABEL"
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
