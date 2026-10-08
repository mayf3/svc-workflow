#!/usr/bin/env bash
# Isolated fixtures for scripts/release.sh target resolution + admission gates (#683).
#
# Baseline (6c7a86b, cases A-F): target resolution happens BEFORE any live write;
# the real deployment target (system/com.svc-workflow) is bound once and shared by
# deploy/restart/verify. Cases G+ close the five proven P1 gaps (#683 review):
#   G1 control identity / service UID: root admission before system write actions;
#      expected SERVICE UID (EXPECTED_SERVICE_UID) is separate from the calling
#      EUID; running PID is checked live via ps; a stopped unit's identity must
#      come from the existing unit config (uid line) — unverifiable = refuse;
#      root must never substitute the service identity.
#   G2 target: the launchd unit's program must equal the install target this
#      script manages ($SERVICE_DIR/$BINARY) after path normalization; a
#      different path with the same content hash is NOT equivalent.
#   G3 preimage: updating an existing target requires EXPECTED_PREIMAGE_SHA256;
#      the target must exist, be readable, and match the declaration.
#   G4 dual-realm state: print results are classified present running /
#      present stopped / absent / denied_unknown; any denied_unknown rejects
#      FIRST (a valid unit on the other realm does not mask it); a running unit
#      plus a stopped unit of the same label is ambiguity.
#   G5 ledger: deploy appends pretty multi-line JSON objects (a concatenated
#      JSON stream, NOT line-delimited); verify/rollback must parse the whole
#      stream, select the latest record for the run's sourceSha, and fail on
#      corrupt/missing records or missing fields; verify and oldSha redeploy
#      both re-use the ledger's recorded binding (no substitute realms).
#
# Case map (coverage requested by the #683 closure scope):
#   A deploy via system target (root caller + explicit service UID 502, running
#     PID ps=502) — the "root + correct UID" positive; kickstart lands on the
#     SYSTEM target, ledger appended, never a gui binding.
#   B both realms running → ambiguous → FAIL zero writes.
#   C no realm present → FAIL zero writes.
#   D root caller + WRONG running UID (ps=501 vs expected 502) → FAIL zero writes.
#   E declared preimage hash mismatch → FAIL zero writes.
#   F verify re-uses the recorded SYSTEM binding and passes.
#   H non-root caller + system target → root-admission gate, zero writes.
#   J unit program = different path, same content hash → foreign-install gate.
#   K existing target binary, no preimage declared → required-preimage gate.
#   L preimage declared but target binary absent → target-existence gate.
#   M system unit valid but gui print denied/unknown → denied-first gate.
#   N gui stopped + system running → running+stopped ambiguity gate.
#   O deploy / verify against a corrupt ledger stream → parse-failure gate.
#   P multi-record multi-line ledger: verify selects the latest record for the
#     run's sourceSha and re-uses its recorded binding (positive).
#   Q ledger records system, fresh resolution lands gui → binding-drift gate;
#     Q2: verify against the same state refuses the absent recorded binding.
#   R stopped unit, same binding, unit config has no uid → identity-unverifiable.
#   S stopped unit, same binding, full evidence (config uid + program + current
#     preimage) → same-binding restore proceeds, kickstart on the same target.
#
# All launchd/process/HTTP/file-descriptor commands are FAKE (launchctl/ps/curl/
# lsof/id under $FAKEBIN; no real launchd contact, no live paths). launchctl
# print output shape (tab indentation, program/uid/pid keys, nested arguments
# block, "Could not find service" text) follows the non-secret sample recorded
# in the 2026-10-08 read-only REALM-MISMATCH evidence. Zero-write assertion:
# full-tree fingerprint of the fake service dir must be identical before/after
# a failed run. Each case stages its own service root; state files live under
# $STATE and are rewritten per case.

set -u
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_SH="$HERE/release.sh"
MERGE_SHA="18fb2f8bc80983a36b15e65bf51d8f5c4b64fb78"
WORK="$(mktemp -d "${TMPDIR:-/tmp}/release-fix-test.XXXXXX")"
FAKEBIN="$WORK/fakebin"
SVC="$WORK/services/svc-workflow"
STATE="$WORK/state"
FOREIGN_INSTALL="$WORK/foreign-install"
mkdir -p "$FAKEBIN" "$STATE" "$FOREIGN_INSTALL"
cp /bin/echo "$FOREIGN_INSTALL/svc-workflow"   # same content hash as the staged binary, different path
PASS=0; FAIL=0
cleanup() { if [ "${KEEP_WORK:-0}" = "1" ]; then echo "KEEP_WORK: $WORK"; else rm -rf "$WORK"; fi; }
trap cleanup EXIT

# ── fake commands (all state files under $STATE) ──────────────────────────
cat > "$FAKEBIN/launchctl" <<EOF
#!/usr/bin/env bash
STATE=\$(cat "$STATE/launchctl-state")
PROG="$SVC/svc-workflow"
FOREIGN="$FOREIGN_INSTALL/svc-workflow"
emit_running() {
  printf '%s = {\n' "\$1"
  printf '\tpath = /Library/LaunchDaemons/com.svc-workflow.plist\n'
  printf '\ttype = LaunchDaemon\n'
  printf '\tstate = running\n'
  printf '\tprogram = %s\n' "\$2"
  printf '\targuments = {\n\t\t0 = %s\n\t}\n' "\$2"
  printf '\tdomain = %s\n' "\${1%%/*}"
  printf '\truns = 3\n'
  printf '\tpid = 99999\n'
  printf '\tlast exit code = 0\n'
  printf '\tuid = 502\n'
  printf '\tstate = active\n'
  printf '}\n'
}
emit_stopped() {
  printf '%s = {\n' "\$1"
  printf '\tpath = /Library/LaunchDaemons/com.svc-workflow.plist\n'
  printf '\ttype = LaunchDaemon\n'
  printf '\tstate = not running\n'
  printf '\tprogram = %s\n' "\$2"
  printf '\targuments = {\n\t\t0 = %s\n\t}\n' "\$2"
  printf '\tdomain = %s\n' "\${1%%/*}"
  printf '\truns = 3\n'
  printf '\tlast exit code = 0\n'
  if [ "\$3" != "nouid" ]; then printf '\tuid = 502\n'; fi
  printf '}\n'
}
emit_absent() {
  printf 'Bad request.\nCould not find service \\"com.svc-workflow\\" in domain%s\n' "\$1" >&2
  exit 113
}
case "\$1 \$2" in
  "print gui/"*)
    case "\$STATE" in
      gui|both|gui-drift)            emit_running "gui/502/com.svc-workflow" "\$PROG" ;;
      runstop)                       emit_stopped "gui/502/com.svc-workflow" "\$PROG" ;;
      denied-gui)                    echo "launchctl: bootstrap failed: 5: Input/output error" >&2; exit 1 ;;
      *)                             emit_absent " for user gui: 502" ;;
    esac ;;
  "print system/"*)
    case "\$STATE" in
      system|both|runstop|denied-gui) emit_running "system/com.svc-workflow" "\$PROG" ;;
      sys-stopped)                    emit_stopped "system/com.svc-workflow" "\$PROG" ;;
      sys-stopped-nouid)              emit_stopped "system/com.svc-workflow" "\$PROG" nouid ;;
      foreign)                        emit_running "system/com.svc-workflow" "\$FOREIGN" ;;
      *)                              emit_absent "" ;;
    esac ;;
  "kickstart"*)
    echo "\$(date -u +%Y-%m-%dT%H:%M:%SZ) \$*" >> "$STATE/kickstart.log"
    exit 0 ;;
  *) exit 1 ;;
esac
EOF
cat > "$FAKEBIN/ps" <<EOF
#!/usr/bin/env bash
if [ "\$1" = "-o" ] && [ "\$2" = "uid=" ]; then cat "$STATE/fake-running-uid"; exit 0; fi
exit 1
EOF
cat > "$FAKEBIN/id" <<EOF
#!/usr/bin/env bash
if [ "\$1" = "-u" ]; then cat "$STATE/fake-euid"; exit 0; fi
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

set_state() {  # $1 = launchctl-state, $2 = caller euid (default 0 = root), $3 = running uid (default 502)
  echo "${1:-system}" > "$STATE/launchctl-state"
  printf "%s" "${2:-0}" > "$STATE/fake-euid"
  printf "%s" "${3:-502}" > "$STATE/fake-running-uid"
}

run_deploy() {  # $1 = state, $2 = tag, [$3 = euid], [$4 = running uid]
  local tag="${2:-x}"
  rm -rf "$SVC"
  stage_release
  set_state "$1" "${3:-0}" "${4:-502}"
  rm -f "$STATE/kickstart.log"
  local before after rc
  before=$(fingerprint)
  SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502     bash ${TRACE_DEPLOY:+-x} "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-$tag.txt" 2>&1
  rc=$?
  after=$(fingerprint)
  echo "$rc" > "$WORK/rc-$tag.txt"
  echo "$before" > "$WORK/fp-before-$tag"
  echo "$after" > "$WORK/fp-after-$tag"
}

run_verify() {  # $1 = tag
  local tag="$1" rc
  SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502     bash "$RELEASE_SH" verify "$MERGE_SHA" > "$WORK/out-$tag.txt" 2>&1
  rc=$?
  echo "$rc" > "$WORK/rc-$tag.txt"
}

assert() { if [ "$2" = "1" ]; then PASS=$((PASS+1)); echo "PASS $1"; else FAIL=$((FAIL+1)); echo "FAIL $1: $3"; fi; }

rc_of()  { cat "$WORK/rc-$1.txt" 2>/dev/null; }
zero_write_ok() { [ "$(cat "$WORK/fp-before-$1.txt")" = "$(cat "$WORK/fp-after-$1.txt")" ] && echo 1 || echo 0; }
out_has() { grep -q "$2" "$WORK/out-$1.txt" 2>/dev/null && echo 1 || echo 0; }

# Negative case: run must fail, reach EXACTLY the target gate message, write nothing.
assert_gate() {  # $1 = case name, $2 = tag, $3 = gate grep pattern
  local rc; rc=$(rc_of "$2")
  assert "$1: reaches gate [$3]" $([ "$(out_has "$2" "$3")" = "1" ] && [ "$rc" != "0" ] && echo 1 || echo 0) "rc=$rc pattern-hit=$(out_has "$2" "$3") (see out-$2)"
  assert "$1: ZERO live writes" "$(zero_write_ok "$2")" "state-diff (see fp-$2)"
}

# ── A. gui absent + system present: deploy OK via SYSTEM target ──────────
#     ("root + correct UID" positive: caller euid mocked 0, service UID 502
#     declared separately, running PID ps-verified 502)
run_deploy system A
RC=$(rc_of A)
KICK=$(grep -c "kickstart -k system/com.svc-workflow" "$STATE/kickstart.log" 2>/dev/null); KICK=${KICK:-0}
NOGUI=$(grep -c "kickstart -k gui/" "$STATE/kickstart.log" 2>/dev/null); NOGUI=${NOGUI:-0}
LED=$([ -f "$SVC/ledger.json" ] && echo 1 || echo 0)
assert "A: deploy succeeds via SYSTEM target" $([ "$RC" = "0" ] && [ "$KICK" -ge 1 ] && [ "$LED" = "1" ] && echo 1 || echo 0) "rc=$RC kick=$KICK ledger=$LED"
assert "A: never kickstarts a gui binding" $([ "$NOGUI" = "0" ] && echo 1 || echo 0) "nogui=$NOGUI"

# ── B. both realms present: ambiguous → FAIL, ZERO writes ────────────────
run_deploy both B
assert "B: ambiguous realms fail" $([ "$(rc_of B)" != "0" ] && echo 1 || echo 0) "rc=$(rc_of B)"
assert "B: ZERO live writes on ambiguity" "$(zero_write_ok B)" "state-diff"

# ── C. no realm present: FAIL, ZERO writes ────────────────────────────────
run_deploy none C
assert "C: missing unit fails" $([ "$(rc_of C)" != "0" ] && echo 1 || echo 0) "rc=$(rc_of C)"
assert "C: ZERO live writes on missing unit" "$(zero_write_ok C)" "state-diff"

# ── D. root caller + wrong running-uid: FAIL, ZERO writes ─────────────────
run_deploy system D 0 501
assert_gate "D: running-uid mismatch" D "service process uid=501 != expected 502"

# ── E. declared preimage mismatch: FAIL, ZERO writes ──────────────────────
stage_release
cp /bin/echo "$SVC/svc-workflow"   # an already-installed binary at the service root
set_state system 0 502
before=$(fingerprint)
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502 EXPECTED_PREIMAGE_SHA256="deadbeef"     bash "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-E.txt" 2>&1
echo "$?" > "$WORK/rc-E.txt"
after=$(fingerprint)
echo "$before" > "$WORK/fp-before-E.txt"; echo "$after" > "$WORK/fp-after-E.txt"
assert_gate "E: preimage mismatch" E "PREIMAGE MISMATCH"

# ── F. verify re-uses the recorded SYSTEM binding and passes ─────────────
run_deploy system F   # deploy installs binary+migrations at the service root (real sequence)
printf '{"service":"svc-workflow","gitSha":"%s","gitTreeState":"clean"}\n' "$MERGE_SHA" > "$STATE/fake-version.json"
run_verify F
assert "F: verify resolves SYSTEM binding and passes" $([ "$(rc_of F)" = "0" ] && echo 1 || echo 0) "rc=$(rc_of F) (see out-F)"

# ── H. non-root caller + system target: root-admission gate ───────────────
run_deploy system H 502 502
assert_gate "H: system control requires root" H "system-domain control requires root"

# ── J. unit program = different path, same content hash: foreign install ──
stage_release
set_state foreign 0 502
before=$(fingerprint)
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502     bash "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-J.txt" 2>&1
echo "$?" > "$WORK/rc-J.txt"
after=$(fingerprint)
echo "$before" > "$WORK/fp-before-J.txt"; echo "$after" > "$WORK/fp-after-J.txt"
assert_gate "J: same-hash different-path is a foreign install" J "refusing to manage a foreign install"

# ── K. existing target binary, preimage not declared ──────────────────────
stage_release
cp /bin/echo "$SVC/svc-workflow"
set_state system 0 502
before=$(fingerprint)
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502     bash "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-K.txt" 2>&1
echo "$?" > "$WORK/rc-K.txt"
after=$(fingerprint)
echo "$before" > "$WORK/fp-before-K.txt"; echo "$after" > "$WORK/fp-after-K.txt"
assert_gate "K: declared preimage missing" K "declared preimage missing"

# ── L. preimage declared but target binary absent ─────────────────────────
stage_release
set_state system 0 502
ECHO_SHA=$(shasum -a 256 /bin/echo | awk '{print $1}')
before=$(fingerprint)
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502 EXPECTED_PREIMAGE_SHA256="$ECHO_SHA"     bash "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-L.txt" 2>&1
echo "$?" > "$WORK/rc-L.txt"
after=$(fingerprint)
echo "$before" > "$WORK/fp-before-L.txt"; echo "$after" > "$WORK/fp-after-L.txt"
assert_gate "L: declared preimage but no target binary" L "target preimage missing"

# ── M. system unit valid but gui print denied/unknown → denied-first ─────
run_deploy denied-gui M
assert_gate "M: denied/unknown print rejects before any success masks it" M "denied/unknown launchctl print result"

# ── N. gui stopped + system running: running+stopped ambiguity ────────────
run_deploy runstop N
assert_gate "N: running+stopped two units are ambiguous" N "ambiguous, 2 present units"

# ── O. corrupt ledger stream: deploy and verify must refuse ───────────────
run_deploy system Opre   # rc0, ledger created
printf '{ "deployedAt": "corrupt-truncated-record\n' >> "$SVC/ledger.json"
set_state system 0 502
INSTALLED_SHA=$(shasum -a 256 "$SVC/svc-workflow" | awk '{print $1}')
before=$(fingerprint)
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502 EXPECTED_PREIMAGE_SHA256="$INSTALLED_SHA"     bash "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-O1.txt" 2>&1
echo "$?" > "$WORK/rc-O1.txt"
after=$(fingerprint)
echo "$before" > "$WORK/fp-before-O1.txt"; echo "$after" > "$WORK/fp-after-O1.txt"
assert_gate "O1: deploy refuses corrupt ledger" O1 "deployment ledger corrupt"

run_verify O2   # verify against the same corrupt ledger
assert_gate "O2: verify refuses corrupt ledger" O2 "deployment ledger corrupt"

# ── P. multi-record multi-line ledger: verify selects the sha's record ────
run_deploy system P   # rc0, one pretty multi-line record
jq -n --arg t "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg s "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa01"     --arg a "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb01"     '{deployedAt: $t, sourceSha: $s, artifactSha256: $a, previousArtifactSha256: "", migrationMaxVersion: "0001", migrationBundleDigest: "ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc01", launchctlTarget: "system/com.svc-workflow", targetPreimageSha256: "", otherRealmState: "none"}'     >> "$SVC/ledger.json"
set_state system 0 502
printf '{"service":"svc-workflow","gitSha":"%s","gitTreeState":"clean"}\n' "$MERGE_SHA" > "$STATE/fake-version.json"
run_verify P
assert "P: verify passes on multi-record multi-line ledger" $([ "$(rc_of P)" = "0" ] && echo 1 || echo 0) "rc=$(rc_of P) (see out-P)"
assert "P: verify used the ledger-recorded binding" $([ "$(out_has P 'ledger-recorded binding system/com.svc-workflow')" = "1" ] && echo 1 || echo 0) "no recorded-binding use (see out-P)"

# ── Q. ledger records system, fresh resolution lands gui: drift gate ──────
#     (continues from P: installed binary + ledger present)
set_state gui-drift 0 502
INSTALLED_SHA=$(shasum -a 256 "$SVC/svc-workflow" | awk '{print $1}')
KICK_BEFORE=$(wc -l < "$STATE/kickstart.log" 2>/dev/null); KICK_BEFORE=${KICK_BEFORE:-0}
before=$(fingerprint)
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502 EXPECTED_PREIMAGE_SHA256="$INSTALLED_SHA"     bash "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-Q.txt" 2>&1
echo "$?" > "$WORK/rc-Q.txt"
after=$(fingerprint)
echo "$before" > "$WORK/fp-before-Q.txt"; echo "$after" > "$WORK/fp-after-Q.txt"
assert_gate "Q: system→gui drift refused (rollback is same-binding only)" Q "binding drift"
KICK_AFTER=$(wc -l < "$STATE/kickstart.log" 2>/dev/null); KICK_AFTER=${KICK_AFTER:-0}
assert "Q: no kickstart on drifted realm" $([ "$KICK_BEFORE" = "$KICK_AFTER" ] && echo 1 || echo 0) "kick=$KICK_BEFORE->$KICK_AFTER"

# ── Q2. verify against the same state: recorded binding absent → refuse ──
run_verify Q2
assert_gate "Q2: verify refuses absent recorded binding (no substitute realm)" Q2 "ledger-recorded binding system/com.svc-workflow not present"

# ── R. stopped unit, same binding, unit config without uid ────────────────
run_deploy system Rpre   # rc0; installs binary + ledger(system)
set_state sys-stopped-nouid 0 502
INSTALLED_SHA=$(shasum -a 256 "$SVC/svc-workflow" | awk '{print $1}')
before=$(fingerprint)
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502 EXPECTED_PREIMAGE_SHA256="$INSTALLED_SHA"     bash "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-R.txt" 2>&1
echo "$?" > "$WORK/rc-R.txt"
after=$(fingerprint)
echo "$before" > "$WORK/fp-before-R.txt"; echo "$after" > "$WORK/fp-after-R.txt"
assert_gate "R: stopped unit without config uid is unverifiable" R "stopped unit identity unverifiable"

# ── S. stopped unit, same binding, full evidence → same-binding restore ──
run_deploy system Spre   # rc0; installs binary + ledger(system)
set_state sys-stopped 0 502
INSTALLED_SHA=$(shasum -a 256 "$SVC/svc-workflow" | awk '{print $1}')
SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502 EXPECTED_PREIMAGE_SHA256="$INSTALLED_SHA"     bash "$RELEASE_SH" deploy "$MERGE_SHA" > "$WORK/out-S.txt" 2>&1
echo "$?" > "$WORK/rc-S.txt"
KICKS=$(grep -c "kickstart -k system/com.svc-workflow" "$STATE/kickstart.log" 2>/dev/null); KICKS=${KICKS:-0}
NOGUI=$(grep -c "kickstart -k gui/" "$STATE/kickstart.log" 2>/dev/null); NOGUI=${NOGUI:-0}
assert "S: stopped same-binding restore proceeds" $([ "$(rc_of S)" = "0" ] && [ "$KICKS" -ge 2 ] && echo 1 || echo 0) "rc=$(rc_of S) kicks=$KICKS (see out-S)"
assert "S: restore kickstarts the SAME system binding only" $([ "$NOGUI" = "0" ] && echo 1 || echo 0) "nogui=$NOGUI"

echo "RESULT: $PASS passed, $FAIL failed"
[ "$FAIL" = "0" ] || exit 1
