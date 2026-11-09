#!/usr/bin/env bash
# Exercises release verification with real bash/jq, local artifacts, and only
# external Git/macOS/HTTP boundaries replaced by controlled command fixtures.
# The upstream target/identity/attribution fixtures run through this same entry.
set -euo pipefail

RELEASE_SCRIPT="${1:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/release.sh}"
TEST_ROOT="$(mktemp -d "${TMPDIR:-/tmp}/svc-workflow-release-test.XXXXXX")"
trap 'rm -rf "$TEST_ROOT"' EXIT
SOURCE_SHA="5d479d834c098c301cd11993c5682c3b7da94480"
OTHER_SHA="1111111111111111111111111111111111111111"

setup_case() {
  CASE_DIR="$(mktemp -d "$TEST_ROOT/case.XXXXXX")"
  SERVICE_DIR="$CASE_DIR/service"
  local release_dir="$SERVICE_DIR/releases/$SOURCE_SHA"
  mkdir -p "$CASE_DIR/bin" "$release_dir/migrations" "$SERVICE_DIR/migrations"
  printf 'local test executable\n' > "$release_dir/svc-workflow"
  printf 'SELECT 1;\n' > "$release_dir/migrations/0001_initial.sql"
  cp "$release_dir/svc-workflow" "$SERVICE_DIR/svc-workflow"
  cp "$release_dir/migrations/0001_initial.sql" "$SERVICE_DIR/migrations/"
  ARTIFACT_SHA="$(shasum -a 256 "$release_dir/svc-workflow" | awk '{print $1}')"
  MIGRATION_DIGEST="$(cd "$release_dir" && shasum -a 256 migrations/0001_initial.sql | shasum -a 256 | awk '{print $1}')"
  jq -n --arg sourceSha "$SOURCE_SHA" --arg artifactSha256 "$ARTIFACT_SHA" \
    --arg migrationBundleDigest "$MIGRATION_DIGEST" \
    '{sourceSha: $sourceSha, treeState: "clean", artifactSha256: $artifactSha256,
      builtAt: "2026-09-30T12:00:00Z", migrationMaxVersion: "0001",
      migrationBundleDigest: $migrationBundleDigest}' > "$release_dir/provenance.json"

  cat > "$CASE_DIR/bin/git" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "$1" == "-C" ]]
case "$3" in
  rev-parse)
    [[ "$4" == "--verify" && "$5" == "$TEST_SOURCE_SHA^{commit}" ]]
    printf '%s\n' "$TEST_SOURCE_SHA" ;;
  worktree)
    case "$4" in
      add)
        [[ "$5" == "--detach" && "$7" == "$TEST_SOURCE_SHA" ]]
        mkdir -p "$6/migrations"
        cp "$SVC_WORKFLOW_SERVICE_DIR/migrations/0001_initial.sql" "$6/migrations/"
        printf '%s\n' "$6" > "$TEST_CASE_DIR/worktree.created" ;;
      remove)
        [[ "$5" == "--force" && "$6" == "${TMPDIR:-/tmp}/svc-workflow-release."* ]]
        rm -rf "$6" ;;
      *) exit 2 ;;
    esac ;;
  ls-tree) printf 'migrations/0001_initial.sql\n' ;;
  *) exit 2 ;;
esac
EOF
  cat > "$CASE_DIR/bin/launchctl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$1" in
  print)
    case "$2" in
      gui/502/com.svc-workflow)
        printf '%s = {\n\tprogram = %s/svc-workflow\n\tpid = 123\n\tuid = 502\n}\n' \
          "$2" "$SVC_WORKFLOW_SERVICE_DIR" ;;
      system/com.svc-workflow)
        printf 'Could not find service "com.svc-workflow"\n' >&2
        exit 113 ;;
      *) exit 2 ;;
    esac ;;
  kickstart) [[ "$2" == "-k" && "$3" == "gui/502/com.svc-workflow" ]] ;;
  *) exit 2 ;;
esac
EOF
  cat > "$CASE_DIR/bin/id" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
case "$1" in
  -u) printf '502\n' ;;
  -un) printf 'fixture-release-user\n' ;;
  *) exit 2 ;;
esac
EOF
  cat > "$CASE_DIR/bin/ps" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "$*" == "-o uid= -p 123" ]]
printf '502\n'
EOF
  cat > "$CASE_DIR/bin/cargo" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "$*" == "build --release --locked" ]]
[[ "$TEST_BUILD_FAIL" != "1" ]] || exit 31
mkdir -p target/release
cp "$SVC_WORKFLOW_SERVICE_DIR/svc-workflow" target/release/svc-workflow
EOF
  cat > "$CASE_DIR/bin/lsof" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
[[ "$*" == "-p 123 -a -d txt -Fn" ]]
printf 'p123\nn%s/svc-workflow\n' "$SVC_WORKFLOW_SERVICE_DIR"
EOF
  cat > "$CASE_DIR/bin/curl" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
url="${!#}"
case "$url" in
  */version)
    printf '{"gitSha":"%s","gitTreeState":"clean","packageVersion":"0.1.0","builtAt":"2026-09-30T12:00:00Z"}\n' "$TEST_SOURCE_SHA" ;;
  */healthz) printf '%s' "$TEST_HEALTH_STATUS" ;;
  */readyz)
    printf '%s' "$TEST_READY_STATUS"
    [[ "$TEST_READY_STATUS" != "000" ]] ;;
  */internal/v1/worklists/assigned-to-me)
    if [[ "$*" == *"Authorization: Bearer "* ]]; then
      printf '%s' "$TEST_AUTH_STATUS"
    else
      printf '%s' "$TEST_UNAUTH_STATUS"
    fi ;;
  *) printf 'unexpected URL: %s\n' "$url" >&2; exit 2 ;;
esac
EOF
  chmod +x "$CASE_DIR/bin/"*
  export TEST_SOURCE_SHA="$SOURCE_SHA"
  export TEST_CASE_DIR="$CASE_DIR" TEST_BUILD_FAIL=0
  export SVC_WORKFLOW_SERVICE_DIR="$SERVICE_DIR"
  export EXPECTED_SERVICE_UID=502 EXPECTED_LAUNCHCTL_TARGET="gui/502/com.svc-workflow"
  export EXPECTED_PREIMAGE_SHA256="$ARTIFACT_SHA"
  export SVC_WORKFLOW_RELEASE_ACTOR="release-regression-fixture"
  export SVC_WORKFLOW_RELEASE_AUTH_REF="isolated-test-only"
  export SVC_WORKFLOW_RELEASE_ATTEMPT_ID="01234567-89ab-4cde-8fab-0123456789ab"
  export TEST_HEALTH_STATUS=200 TEST_READY_STATUS=200
  export TEST_UNAUTH_STATUS=401 TEST_AUTH_STATUS=200 AUTH_TOKEN=""
}

write_ledger() {
  local old_sha="${1:-$OTHER_SHA}" current_sha="${2:-$SOURCE_SHA}"
  jq -n --arg oldSha "$old_sha" --arg currentSha "$current_sha" \
    --arg artifactSha256 "$ARTIFACT_SHA" --arg migrationBundleDigest "$MIGRATION_DIGEST" \
    '{deployedAt: "2026-09-29T12:00:00Z", sourceSha: $oldSha,
      artifactSha256: $artifactSha256, previousArtifactSha256: "", migrationMaxVersion: "0001",
      migrationBundleDigest: $migrationBundleDigest, receipt: "historical",
      launchctlTarget: "gui/502/com.svc-workflow", targetPreimageSha256: $artifactSha256,
      verification: {healthz: "200", readyz: "503", authHttpStatus: "401", runningBinaryPath: "/historical/binary"}},
     {deployedAt: "2026-09-30T12:00:00Z", sourceSha: $currentSha,
      artifactSha256: $artifactSha256, previousArtifactSha256: $artifactSha256,
      migrationMaxVersion: "0001", migrationBundleDigest: $migrationBundleDigest, receipt: "current",
      launchctlTarget: "gui/502/com.svc-workflow", targetPreimageSha256: $artifactSha256}' \
    > "$SERVICE_DIR/ledger.json"
  cp "$SERVICE_DIR/ledger.json" "$CASE_DIR/ledger.before"
}

run_verify() {
  VERIFY_STATUS=0
  PATH="$CASE_DIR/bin:$PATH" bash "$RELEASE_SCRIPT" verify "$SOURCE_SHA" \
    > "$CASE_DIR/output" 2>&1 || VERIFY_STATUS=$?
}

expect_success() {
  [[ "$VERIFY_STATUS" == 0 ]] || { cat "$CASE_DIR/output"; return 1; }
  rg -q 'VERIFY PASSED:' "$CASE_DIR/output" || { cat "$CASE_DIR/output"; return 1; }
  [[ ! -d "$SERVICE_DIR/.release.lock" ]] || { printf 'successful release left its lock\n'; return 1; }
}

expect_failure_without_receipt_change() {
  if [[ "$VERIFY_STATUS" == 0 ]] || rg -q 'VERIFY PASSED:' "$CASE_DIR/output"; then
    printf 'verification incorrectly succeeded:\n'
    cat "$CASE_DIR/output"
    return 1
  fi
  cmp -s "$CASE_DIR/ledger.before" "$SERVICE_DIR/ledger.json" \
    || { printf 'failed verification changed the deployment receipt\n'; return 1; }
  [[ ! -d "$SERVICE_DIR/.release.lock" ]] || { printf 'failed release left its lock\n'; return 1; }
}

expect_current_receipt_verified() {
  local old_before old_after
  old_before="$(jq -cs '.[0]' "$CASE_DIR/ledger.before")"
  old_after="$(jq -cs '.[0]' "$SERVICE_DIR/ledger.json")"
  [[ "$old_before" == "$old_after" ]] \
    || { printf 'historical deployment receipt was rewritten\n'; return 1; }
  jq -es --arg binary "$SERVICE_DIR/svc-workflow" \
    'length == 2 and .[1].receipt == "current" and
      .[1].verification == {healthz: "200", readyz: "200", authHttpStatus: "401", runningBinaryPath: $binary}' \
    "$SERVICE_DIR/ledger.json" >/dev/null
}

test_preserves_historical_receipt() {
  setup_case; write_ledger; run_verify; expect_success; expect_current_receipt_verified
}

test_repeat_deploy_same_sha_preserves_previous_receipt() {
  setup_case; write_ledger "$SOURCE_SHA"; run_verify; expect_success; expect_current_receipt_verified
}

test_verify_preserves_attribution_and_history() {
  setup_case; write_ledger
  # Preserve an explicitly constructed prefix, independently of verify's parser.
  jq -s --arg actor $'historical "owner"\n{发布}' \
    --arg auth $'approval/{history}\n"原始授权"' \
    '.[0] + {osUser: "historical-os-user", actor: $actor, authRef: $auth,
      attemptId: "11111111-1111-4111-8111-111111111111"}' \
    "$SERVICE_DIR/ledger.json" > "$CASE_DIR/history.prefix"
  jq -cs '.[1] + {osUser: "deployment-os-user", actor: "deployment-actor",
    authRef: "deployment-approval", attemptId: "22222222-2222-4222-8222-222222222222"}' \
    "$SERVICE_DIR/ledger.json" > "$CASE_DIR/current.receipt"
  cat "$CASE_DIR/history.prefix" "$CASE_DIR/current.receipt" > "$SERVICE_DIR/ledger.json"
  cp "$SERVICE_DIR/ledger.json" "$CASE_DIR/ledger.before"
  export SVC_WORKFLOW_RELEASE_ACTOR="different-verify-caller"
  export SVC_WORKFLOW_RELEASE_AUTH_REF="different-verify-approval"
  export SVC_WORKFLOW_RELEASE_ATTEMPT_ID="33333333-3333-4333-8333-333333333333"
  run_verify; expect_success; expect_current_receipt_verified
  local prefix_bytes
  prefix_bytes="$(wc -c < "$CASE_DIR/history.prefix" | tr -d ' ')"
  head -c "$prefix_bytes" "$SERVICE_DIR/ledger.json" > "$CASE_DIR/history.after"
  cmp -s "$CASE_DIR/history.prefix" "$CASE_DIR/history.after" \
    || { printf 'verification changed historical receipt bytes\n'; return 1; }
  jq -es --slurpfile before "$CASE_DIR/ledger.before" \
    'def attribution: {osUser, actor, authRef, attemptId};
     map(attribution) == ($before | map(attribution))' \
    "$SERVICE_DIR/ledger.json" >/dev/null
  # A later failed verification must preserve both attributed receipts in full.
  cp "$SERVICE_DIR/ledger.json" "$CASE_DIR/ledger.before"
  export TEST_READY_STATUS=503
  run_verify; expect_failure_without_receipt_change
}

test_latest_receipt_mismatch_fails() {
  setup_case; write_ledger "$SOURCE_SHA" "$OTHER_SHA"; run_verify
  expect_failure_without_receipt_change
}

test_latest_receipt_artifact_mismatch_fails() {
  setup_case; write_ledger
  jq -cs '.[1].artifactSha256 = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff" | .[]' \
    "$SERVICE_DIR/ledger.json" > "$CASE_DIR/modified"
  mv "$CASE_DIR/modified" "$SERVICE_DIR/ledger.json"
  cp "$SERVICE_DIR/ledger.json" "$CASE_DIR/ledger.before"
  run_verify; expect_failure_without_receipt_change
}

test_latest_receipt_migration_mismatch_fails() {
  setup_case; write_ledger
  jq -cs '.[1].migrationBundleDigest = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff" | .[]' \
    "$SERVICE_DIR/ledger.json" > "$CASE_DIR/modified"
  mv "$CASE_DIR/modified" "$SERVICE_DIR/ledger.json"
  cp "$SERVICE_DIR/ledger.json" "$CASE_DIR/ledger.before"
  run_verify; expect_failure_without_receipt_change
}

test_latest_receipt_migration_max_mismatch_fails() {
  setup_case; write_ledger
  jq -cs '.[1].migrationMaxVersion = "0002" | .[]' \
    "$SERVICE_DIR/ledger.json" > "$CASE_DIR/modified"
  mv "$CASE_DIR/modified" "$SERVICE_DIR/ledger.json"
  cp "$SERVICE_DIR/ledger.json" "$CASE_DIR/ledger.before"
  run_verify; expect_failure_without_receipt_change
}

test_readyz_failure_fails() {
  setup_case; write_ledger; export TEST_READY_STATUS=503; run_verify
  expect_failure_without_receipt_change
}

test_readyz_transport_failure_fails() {
  setup_case; write_ledger; export TEST_READY_STATUS=000; run_verify
  expect_failure_without_receipt_change
}

test_unauthenticated_request_must_be_rejected() {
  setup_case; write_ledger; export TEST_UNAUTH_STATUS=200; run_verify
  expect_failure_without_receipt_change
}

test_authenticated_request_must_succeed() {
  setup_case; write_ledger; export AUTH_TOKEN=synthetic-test-token TEST_AUTH_STATUS=401; run_verify
  expect_failure_without_receipt_change
}

test_token_success_does_not_hide_unprotected_endpoint() {
  setup_case; write_ledger
  export AUTH_TOKEN=synthetic-test-token TEST_UNAUTH_STATUS=200 TEST_AUTH_STATUS=200
  run_verify; expect_failure_without_receipt_change
}

test_authenticated_success_records_current_receipt() {
  setup_case; write_ledger; export AUTH_TOKEN=synthetic-test-token; run_verify; expect_success
  jq -es '.[1].verification.authHttpStatus == "200"' "$SERVICE_DIR/ledger.json" >/dev/null
}

test_healthz_failure_fails() {
  setup_case; write_ledger; export TEST_HEALTH_STATUS=503; run_verify
  expect_failure_without_receipt_change
}

test_missing_receipt_fails() {
  setup_case; run_verify
  [[ "$VERIFY_STATUS" != 0 ]] && ! rg -q 'VERIFY PASSED:' "$CASE_DIR/output" \
    || { printf 'verification succeeded without a deployment receipt\n'; cat "$CASE_DIR/output"; return 1; }
  [[ ! -e "$SERVICE_DIR/ledger.json" ]]
}

test_empty_ledger_fails() {
  setup_case
  : > "$SERVICE_DIR/ledger.json"
  cp "$SERVICE_DIR/ledger.json" "$CASE_DIR/ledger.before"
  run_verify; expect_failure_without_receipt_change
}

test_malformed_ledger_fails() {
  setup_case
  printf '{"sourceSha":\n' > "$SERVICE_DIR/ledger.json"
  cp "$SERVICE_DIR/ledger.json" "$CASE_DIR/ledger.before"
  run_verify; expect_failure_without_receipt_change
}

wait_for_marker() {
  local marker="$1" attempt
  for ((attempt = 0; attempt < 250; attempt++)); do
    [[ -e "$marker" ]] && return 0
    sleep 0.02
  done
  printf 'timed out waiting for %s\n' "$marker"
  return 1
}

install_interleaving_barriers() {
  export REAL_JQ="$(command -v jq)" REAL_MKDIR="$(command -v mkdir)"
  export TEST_SYNC_DIR="$CASE_DIR/sync"
  mkdir -p "$TEST_SYNC_DIR"
  # Delegate every jq invocation to the real binary; pause only after its
  # ledger read has completed and its real result has been written to stdout.
  cat > "$CASE_DIR/bin/jq" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
"$REAL_JQ" "$@"
if [[ "${TEST_RELEASE_ROLE:-}" == "verify" && "${!#}" == "$SVC_WORKFLOW_SERVICE_DIR/ledger.json" ]]; then
  touch "$TEST_SYNC_DIR/verify-staged"
  for ((attempt = 0; attempt < 500; attempt++)); do
    [[ ! -e "$TEST_SYNC_DIR/allow-verify" ]] || exit 0
    sleep 0.02
  done
  exit 2
fi
EOF
  cat > "$CASE_DIR/bin/mkdir" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "${TEST_RELEASE_ROLE:-}" == "deploy" && "${!#}" == "$SVC_WORKFLOW_SERVICE_DIR/.release.lock" ]]; then
  touch "$TEST_SYNC_DIR/deploy-lock-attempt"
fi
exec "$REAL_MKDIR" "$@"
EOF
  chmod +x "$CASE_DIR/bin/jq" "$CASE_DIR/bin/mkdir"
}

finish_case_processes() {
  touch "$TEST_SYNC_DIR/allow-verify"
  local pid
  for pid in "${VERIFY_PID:-}" "${DEPLOY_PID:-}"; do
    if [[ -n "$pid" ]]; then
      kill "$pid" 2>/dev/null || true
      wait "$pid" 2>/dev/null || true
    fi
  done
}

test_concurrent_deploy_receipt_survives_verify() {
  setup_case; write_ledger; install_interleaving_barriers
  VERIFY_PID="" DEPLOY_PID=""
  trap finish_case_processes EXIT
  PATH="$CASE_DIR/bin:$PATH" TEST_RELEASE_ROLE=verify bash "$RELEASE_SCRIPT" verify "$SOURCE_SHA" \
    > "$CASE_DIR/output" 2>&1 &
  VERIFY_PID=$!
  wait_for_marker "$TEST_SYNC_DIR/verify-staged"
  (
    PATH="$CASE_DIR/bin:$PATH" TEST_RELEASE_ROLE=deploy bash "$RELEASE_SCRIPT" deploy "$SOURCE_SHA" \
      > "$CASE_DIR/deploy.output" 2>&1
    touch "$TEST_SYNC_DIR/deploy-done"
  ) &
  DEPLOY_PID=$!
  # Old code completes the append while verification is paused. Correct code
  # reaches the shared lock and can append only after verification releases it.
  local attempt synchronized=0
  for ((attempt = 0; attempt < 250; attempt++)); do
    if [[ -e "$TEST_SYNC_DIR/deploy-done" || -e "$TEST_SYNC_DIR/deploy-lock-attempt" ]]; then
      synchronized=1
      break
    fi
    sleep 0.02
  done
  touch "$TEST_SYNC_DIR/allow-verify"
  [[ "$synchronized" == 1 ]] || { cat "$CASE_DIR/deploy.output"; return 1; }
  VERIFY_STATUS=0; wait "$VERIFY_PID" || VERIFY_STATUS=$?
  VERIFY_PID=""
  local deploy_status=0
  wait "$DEPLOY_PID" || deploy_status=$?
  DEPLOY_PID=""
  trap - EXIT
  [[ "$deploy_status" == 0 ]] || { cat "$CASE_DIR/deploy.output"; return 1; }
  expect_success
  jq -es --arg sourceSha "$SOURCE_SHA" \
    'length == 3 and .[0].receipt == "historical" and
     .[0].verification.runningBinaryPath == "/historical/binary" and
     .[1].receipt == "current" and .[1].verification.readyz == "200" and
     .[2].sourceSha == $sourceSha and (.[2] | has("verification") | not)' \
    "$SERVICE_DIR/ledger.json" >/dev/null \
    || { printf 'concurrent deployment receipt was lost or attributed to the wrong deployment\n'; return 1; }
}

expect_busy_entry_fails() {
  local command="$1"
  setup_case; write_ledger
  mkdir "$SERVICE_DIR/.release.lock"
  printf 'foreign-owner\n' > "$SERVICE_DIR/.release.lock/owner"
  export TEST_BUILD_FAIL=1
  # Speed up bounded lock polling while retaining each attempted wait.
  cat > "$CASE_DIR/bin/sleep" <<'EOF'
#!/usr/bin/env bash
[[ "$1" == 1 ]]
printf 'wait\n' >> "$TEST_CASE_DIR/lock.waits"
EOF
  chmod +x "$CASE_DIR/bin/sleep"
  local status=0
  PATH="$CASE_DIR/bin:$PATH" bash "$RELEASE_SCRIPT" "$command" "$SOURCE_SHA" \
    > "$CASE_DIR/output" 2>&1 || status=$?
  # The entry must reach the lock-timeout failure branch itself; a bash 3.2
  # expansion abort on the message must never masquerade as success.
  rg -q '发布锁在 30s 内未释放' "$CASE_DIR/output" \
    || { printf '%s never reported the lock timeout\n' "$command"; return 1; }
  ! rg -q 'unbound variable' "$CASE_DIR/output" \
    || { printf '%s aborted on shell variable expansion\n' "$command"; return 1; }
  [[ "$status" != 0 ]] || { printf '%s bypassed the busy release lock\n' "$command"; return 1; }
  cmp -s "$CASE_DIR/ledger.before" "$SERVICE_DIR/ledger.json"
  [[ "$(cat "$SERVICE_DIR/.release.lock/owner")" == "foreign-owner" ]]
  [[ ! -e "$CASE_DIR/worktree.created" ]] || { printf '%s performed a build while another release held the lock\n' "$command"; return 1; }
  [[ -f "$CASE_DIR/lock.waits" && "$(wc -l < "$CASE_DIR/lock.waits")" -le 30 ]]
}

test_build_respects_busy_release_lock() { expect_busy_entry_fails build; }
test_deploy_respects_busy_release_lock() { expect_busy_entry_fails deploy; }
test_verify_respects_busy_release_lock() { expect_busy_entry_fails verify; }
test_all_respects_busy_release_lock() { expect_busy_entry_fails all; }
test_archive_respects_busy_release_lock() { expect_busy_entry_fails archive; }

test_build_failure_cleans_worktree_and_lock() {
  setup_case; write_ledger
  export TEST_BUILD_FAIL=1
  local status=0
  PATH="$CASE_DIR/bin:$PATH" bash "$RELEASE_SCRIPT" build "$SOURCE_SHA" \
    > "$CASE_DIR/output" 2>&1 || status=$?
  [[ "$status" == 31 ]] || { cat "$CASE_DIR/output"; return 1; }
  [[ -f "$CASE_DIR/worktree.created" && ! -d "$(cat "$CASE_DIR/worktree.created")" ]]
  [[ ! -d "$SERVICE_DIR/.release.lock" ]]
  cmp -s "$CASE_DIR/ledger.before" "$SERVICE_DIR/ledger.json"
}

test_build_cleanup_failure_preserves_failure_and_releases_lock() {
  setup_case; write_ledger
  export TEST_BUILD_FAIL=1 REAL_RM="$(command -v rm)"
  cat > "$CASE_DIR/bin/rm" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [[ "$*" == "-rf ${TMPDIR:-/tmp}/svc-workflow-release."* ]]; then
  exit 19
fi
exec "$REAL_RM" "$@"
EOF
  chmod +x "$CASE_DIR/bin/rm"
  local status=0
  PATH="$CASE_DIR/bin:$PATH" bash "$RELEASE_SCRIPT" build "$SOURCE_SHA" \
    > "$CASE_DIR/output" 2>&1 || status=$?
  "$REAL_RM" -rf "$(cat "$CASE_DIR/worktree.created")"
  [[ "$status" == 31 ]] || { printf 'cleanup replaced the original build failure with exit %s\n' "$status"; return 1; }
  [[ ! -d "$SERVICE_DIR/.release.lock" ]]
  cmp -s "$CASE_DIR/ledger.before" "$SERVICE_DIR/ledger.json"
}

test_term_releases_owned_lock_without_verifying() {
  setup_case; write_ledger; install_interleaving_barriers
  VERIFY_PID="" DEPLOY_PID=""
  trap finish_case_processes EXIT
  PATH="$CASE_DIR/bin:$PATH" TEST_RELEASE_ROLE=verify bash "$RELEASE_SCRIPT" verify "$SOURCE_SHA" \
    > "$CASE_DIR/output" 2>&1 &
  VERIFY_PID=$!
  wait_for_marker "$TEST_SYNC_DIR/verify-staged"
  [[ "$(cat "$SERVICE_DIR/.release.lock/owner")" == "$VERIFY_PID" ]]
  kill -TERM "$VERIFY_PID"
  touch "$TEST_SYNC_DIR/allow-verify"
  VERIFY_STATUS=0; wait "$VERIFY_PID" || VERIFY_STATUS=$?
  VERIFY_PID=""
  trap - EXIT
  [[ "$VERIFY_STATUS" == 143 ]] || { cat "$CASE_DIR/output"; return 1; }
  expect_failure_without_receipt_change
}

failures=0
for test in \
  test_preserves_historical_receipt \
  test_repeat_deploy_same_sha_preserves_previous_receipt \
  test_verify_preserves_attribution_and_history \
  test_latest_receipt_mismatch_fails \
  test_latest_receipt_artifact_mismatch_fails \
  test_latest_receipt_migration_mismatch_fails \
  test_latest_receipt_migration_max_mismatch_fails \
  test_readyz_failure_fails \
  test_readyz_transport_failure_fails \
  test_unauthenticated_request_must_be_rejected \
  test_authenticated_request_must_succeed \
  test_token_success_does_not_hide_unprotected_endpoint \
  test_authenticated_success_records_current_receipt \
  test_healthz_failure_fails \
  test_missing_receipt_fails \
  test_empty_ledger_fails \
  test_malformed_ledger_fails \
  test_concurrent_deploy_receipt_survives_verify \
  test_build_respects_busy_release_lock \
  test_deploy_respects_busy_release_lock \
  test_verify_respects_busy_release_lock \
  test_all_respects_busy_release_lock \
  test_archive_respects_busy_release_lock \
  test_build_failure_cleans_worktree_and_lock \
  test_build_cleanup_failure_preserves_failure_and_releases_lock \
  test_term_releases_owned_lock_without_verifying; do
  # Keep errexit active inside each test, while collecting independent failures.
  set +e
  (set -e; "$test")
  status=$?
  set -e
  if [[ "$status" == 0 ]]; then
    printf 'PASS %s\n' "$test"
  else
    printf 'FAIL %s\n' "$test"
    failures=$((failures + 1))
  fi
done

# Keep the upstream #683/#128 suite unchanged in the repository, while directing
# its release path to the optional candidate argument supported by this entry.
# Its launchctl/ps/id/lsof/curl/cargo are fixtures, and its service tree and build
# worktrees are temporary. No production service, network endpoint, or DB is used.
TARGET_SUITE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/test-release-target-resolution.sh"
TARGET_HARNESS="$TEST_ROOT/test-release-target-resolution.sh"
awk '/^RELEASE_SH="\$HERE\/release.sh"$/ { $0 = "RELEASE_SH=\"${SVC_WORKFLOW_TEST_RELEASE_SCRIPT:?}\"" } { print }' \
  "$TARGET_SUITE" > "$TARGET_HARNESS"
target_status=0
(
  unset AUTH_TOKEN SVC_WORKFLOW_RELEASE_ACTOR SVC_WORKFLOW_RELEASE_AUTH_REF SVC_WORKFLOW_RELEASE_ATTEMPT_ID
  export SVC_WORKFLOW_TEST_RELEASE_SCRIPT="$RELEASE_SCRIPT"
  bash "$TARGET_HARNESS"
) || target_status=$?
if [[ "$target_status" != 0 ]]; then
  printf 'FAIL upstream target/identity/attribution suite (rc=%s)\n' "$target_status"
  failures=$((failures + 1))
fi
[[ "$failures" == 0 ]]
