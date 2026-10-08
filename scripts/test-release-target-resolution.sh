#!/usr/bin/env bash
# Isolated fixtures for scripts/release.sh target resolution (#683).
#
# Regression: release.sh hardcoded the gui/$(id -u) launchd realm and checked
# the unit only AFTER binary/migrations/ledger writes — the real deployment
# target lives in the system domain, so an as-is run would WRITE FIRST and
# FAIL LAST. Fixtures use FAKE launchctl/ps/curl/lsof commands (no real
# launchd contact) and assert:
#   A. gui absent + system present + uid match → deploy OK, kickstart on the
#      SYSTEM target, ledger appended.
#   B. both realms present → ambiguous → FAIL with ZERO live writes.
#   C. no realm present → FAIL with ZERO live writes.
#   D. running-uid mismatch → FAIL with ZERO live writes.
#   E. declared preimage sha mismatch → FAIL with ZERO live writes.
#   F. verify resolves the same SYSTEM binding and passes.
#
# Zero-write assertion: full-tree fingerprint of the fake service dir must be
# identical before/after a failed run.

set -u
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_SH="$HERE/release.sh"
MERGE_SHA="18fb2f8bc80983a36b15e65bf51d8f5c4b64fb78"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/release-fix-test.XXXXXX")"
FAKEBIN="$WORK/fakebin"
SVC="$WORK/services/svc-workflow"
STATE="$WORK/state"
mkdir -p "$FAKEBIN" "$STATE"
PASS=0; FAIL=0
cleanup() { if [ "${KEEP_WORK:-0}" = "1" ]; then echo "KEEP_WORK: $WORK"; else rm -rf "$WORK"; fi; }
trap cleanup EXIT

# ── fake commands (all state files under $STATE) ──────────────────────────
cat > "$FAKEBIN/launchctl" <<EOF
#!/usr/bin/env bash
STATE=\$(cat "$STATE/launchctl-state")
case "\$1 \$2" in
  "print gui/"*)
    if [ "\$STATE" = "gui" ] || [ "\$STATE" = "both" ]; then echo "pid = 99999"; exit 0; fi
    exit 1 ;;
  "print system/"*)
    if [ "\$STATE" = "system" ] || [ "\$STATE" = "both" ]; then echo "pid = 99999"; exit 0; fi
    exit 1 ;;
  "kickstart"*)
    echo "\$(date -u +%Y-%m-%dT%H:%M:%SZ) \$*" >> "$STATE/kickstart.log"
    exit 0 ;;
esac
exit 1
EOF
cat > "$FAKEBIN/ps" <<EOF
#!/usr/bin/env bash
if [ "\$1" = "-o" ] && [ "\$2" = "uid=" ]; then cat "$STATE/fake-running-uid"; exit 0; fi
exit 1
EOF
cat > "$FAKEBIN/curl" <<EOF
#!/usr/bin/env bash
URL="\${@: -1}"
if [ "\$1" = "-sf" ]; then
  if echo "\$URL" | grep -q "/version"; then
    [ -f "$STATE/fake-version.json" ] && cat "$STATE/fake-version.json" && exit 0
  fi
  exit 1
fi
if [ "\$1" = "-s" ]; then
  if echo "\$URL" | grep -q "healthz"; then printf "200"; exit 0; fi
  if echo "\$URL" | grep -q "readyz"; then printf "200"; exit 0; fi
  if echo "\$URL" | grep -q "assigned-to-me"; then printf "401"; exit 0; fi
  printf "200"; exit 0
fi
exit 0
EOF
cat > "$FAKEBIN/lsof" <<EOF
#!/usr/bin/env bash
STAGED=\$(cat "$STATE/staged-binary-path")
echo "n\$STAGED"
EOF
chmod +x "$FAKEBIN/"*

# ── service dir staging (fresh per case) ─────────────────────────────────
stage_release() {
  rm -rf "$SVC"
  mkdir -p "$SVC/releases/$MERGE_SHA/migrations"
  cp /bin/echo "$SVC/releases/$MERGE_SHA/svc-workflow"
  echo "# fixture migration" > "$SVC/releases/$MERGE_SHA/migrations/0001_fixture.sql"
  local sha; sha=$(shasum -a 256 "$SVC/releases/$MERGE_SHA/svc-workflow" | awk '{print $1}')
  local dig; dig=$(cd "$SVC/releases/$MERGE_SHA" && find migrations -name '*.sql' -type f | sort | xargs shasum -a 256 | shasum -a 256 | awk '{print $1}')
  jq -n --arg s "$MERGE_SHA" --arg a "$sha" --arg d "$dig"     '{sourceSha: $s, treeState: "clean", artifactSha256: $a, builtAt: "2026-10-09T00:00:00Z", buildCommand: "fixture", migrationMaxVersion: "0001", migrationBundleDigest: $d}'     > "$SVC/releases/$MERGE_SHA/provenance.json"
  echo "$SVC/releases/$MERGE_SHA/svc-workflow" > "$STATE/staged-binary-path"
}

fingerprint() { (cd "$SVC" && find . -type f | sort | xargs shasum -a 256 2>/dev/null); }

run_deploy() {  # $1 = state, $2 = tag
  local tag="${2:-x}"
  rm -rf "$SVC"
  stage_release
  echo "$1" > "$STATE/launchctl-state"
  printf "502" > "$STATE/fake-running-uid"
  rm -f "$STATE/kickstart.log"
  local before after rc
  before=$(fingerprint)
  SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH"     bash ${TRACE_DEPLOY:+-x} "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-$tag.txt" 2>&1
  rc=$?
  after=$(fingerprint)
  echo "$rc|$before|$after"
}

assert() { if [ "$2" = "1" ]; then PASS=$((PASS+1)); echo "PASS $1"; else FAIL=$((FAIL+1)); echo "FAIL $1: $3"; fi; }

# ── A. gui absent + system present: deploy OK via SYSTEM target ──────────
RES=$(run_deploy system A)
RC=${RES%%|*}
KICK=$(grep -c "kickstart -k system/com.svc-workflow" "$STATE/kickstart.log" 2>/dev/null); KICK=${KICK:-0}
NOGUI=$(grep -c "kickstart -k gui/" "$STATE/kickstart.log" 2>/dev/null); NOGUI=${NOGUI:-0}
LED=$([ -f "$SVC/ledger.json" ] && echo 1 || echo 0)
assert "A: deploy succeeds via SYSTEM target" $([ "$RC" = "0" ] && [ "$KICK" -ge 1 ] && [ "$LED" = "1" ] && echo 1 || echo 0) "rc=$RC kick=$KICK ledger=$LED"
assert "A: never kickstarts a gui binding" $([ "$NOGUI" = "0" ] && echo 1 || echo 0) "nogui=$NOGUI"

# ── B. both realms present: ambiguous → FAIL, ZERO writes ────────────────
RES=$(run_deploy both B)
RC=${RES%%|*}
BEFORE=${RES#*|}; BEFORE=${BEFORE%%|*}
AFTER=${RES##*|}
assert "B: ambiguous realms fail" $([ "$RC" != "0" ] && echo 1 || echo 0) "rc=$RC"
assert "B: ZERO live writes on ambiguity" $([ "$BEFORE" = "$AFTER" ] && echo 1 || echo 0) "state-diff"

# ── C. no realm present: FAIL, ZERO writes ────────────────────────────────
RES=$(run_deploy none C)
RC=${RES%%|*}
BEFORE=${RES#*|}; BEFORE=${BEFORE%%|*}
AFTER=${RES##*|}
assert "C: missing unit fails" $([ "$RC" != "0" ] && echo 1 || echo 0) "rc=$RC"
assert "C: ZERO live writes on missing unit" $([ "$BEFORE" = "$AFTER" ] && echo 1 || echo 0) "state-diff"

# ── D. running-uid mismatch: FAIL, ZERO writes ────────────────────────────
RES=$(run_deploy system D)
printf "501" > "$STATE/fake-running-uid"
RC=${RES%%|*}
BEFORE=${RES#*|}; BEFORE=${BEFORE%%|*}
AFTER=${RES##*|}
# D re-runs deploy with mismatched uid via a fresh staged dir
rm -rf "$SVC"; stage_release
echo "system" > "$STATE/launchctl-state"
printf "501" > "$STATE/fake-running-uid"
BEFORE=$(fingerprint)
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH"   bash "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-D.txt" 2>&1
RC=$?
AFTER=$(fingerprint)
assert "D: running-uid mismatch fails" $([ "$RC" != "0" ] && echo 1 || echo 0) "rc=$RC"
assert "D: ZERO live writes on uid mismatch" $([ "$BEFORE" = "$AFTER" ] && echo 1 || echo 0) "state-diff"

# ── E. declared preimage mismatch: FAIL, ZERO writes ──────────────────────
rm -rf "$SVC"; stage_release
cp /bin/echo "$SVC/svc-workflow"   # an already-installed binary at the service root
echo "system" > "$STATE/launchctl-state"
printf "502" > "$STATE/fake-running-uid"
BEFORE=$(fingerprint)
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_PREIMAGE_SHA256="deadbeef"   bash "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-E.txt" 2>&1
RC=$?
AFTER=$(fingerprint)
assert "E: preimage mismatch fails" $([ "$RC" != "0" ] && echo 1 || echo 0) "rc=$RC"
assert "E: ZERO live writes on preimage mismatch" $([ "$BEFORE" = "$AFTER" ] && echo 1 || echo 0) "state-diff"

# ── F. verify resolves the same SYSTEM binding and passes ────────────────
RES=$(run_deploy system F)
RC=${RES%%|*}
rm -rf "$SVC"; stage_release
echo "system" > "$STATE/launchctl-state"
printf "502" > "$STATE/fake-running-uid"
run_deploy system F   # deploy installs binary+migrations at the service root (real sequence)
printf '{"service":"svc-workflow","gitSha":"%s","gitTreeState":"clean"}\n' "$MERGE_SHA" > "$STATE/fake-version.json"
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH"   bash "$RELEASE_SH" verify "$MERGE_SHA" > "$WORK/out-F.txt" 2>&1
RC=$?
assert "F: verify resolves SYSTEM binding and passes" $([ "$RC" = "0" ] && echo 1 || echo 0) "rc=$RC (see out-F)"

echo "RESULT: $PASS passed, $FAIL failed"
[ "$FAIL" = "0" ] || exit 1
