#!/usr/bin/env bash
# Isolated fixtures for scripts/release.sh target resolution + admission gates (#683).
#
# All launchd/process/HTTP/file-descriptor commands are FAKE (launchctl/ps/curl/
# lsof/id under $FAKEBIN; no real launchd contact, no live paths). launchctl
# print output shape (tab indentation, program/uid/pid keys, nested arguments
# block, "Could not find service" text) follows the non-secret sample recorded
# in the 2026-10-08 read-only REALM-MISMATCH evidence. Each case stages its own
# service root; state files live under $STATE and are rewritten per case.
# Zero-write assertion: INDEPENDENT full-tree fingerprints of the fake service
# dir are captured before/after every run (fp-before-TAG.txt / fp-after-TAG.txt);
# the assertion fails on missing, empty, or differing fingerprints — self-tested
# in Z0-Z3.
#
# Contract under test (closure of mayf3/agent-control#683):
#   G1 identity (static PASS, kept): EXPECTED_SERVICE_UID separate from calling
#      EUID; running PID live-checked via ps + unit config cross-check; stopped
#      units take identity only from the existing unit config; root admission
#      before system write actions; root never substitutes the service identity.
#   G2 target (kept): launchd unit program == managed install target after path
#      normalization; same-hash different-path refused.
#   G3/P3 preimage: this entry only updates/restores an EXISTING service — the
#      target binary must EXIST (readable) unconditionally and
#      EXPECTED_PREIMAGE_SHA256 must be declared and match. The former
#      first-install pass-through is removed; file and declaration missing
#      together is refused at the same gate (K2).
#   G4 dual-realm state (static PASS, kept): present running / present stopped /
#      absent / denied_unknown classification; denied_unknown rejects first;
#      running+stopped is ambiguity.
#   G5/P5 ledger binding: deploy appends pretty multi-line JSON objects (one
#      concatenated JSON stream, not JSONL). When the ledger exists, the latest
#      record's launchctlTarget is the fixed binding (corrupt/empty/missing-
#      field refuses pre-write). When the ledger does NOT exist (first update
#      through this entry), the binding must be pinned explicitly via
#      EXPECTED_LAUNCHCTL_TARGET (value sourced from approved evidence, e.g.
#      the REALM-MISMATCH readonly actualLoaded=system/com.svc-workflow);
#      missing pin refuses pre-write — a missing ledger can never cause
#      re-discovery of another realm. Verify parses the whole stream, selects
#      the latest record for the run's sourceSha, and re-uses the recorded
#      binding without re-resolving realms.
#   P2 running-executable binding: the running PID's actual executable must be
#      normalized-equal to the fixed install target BEFORE the first write
#      (deploy) and during verify — another directory's same-hash copy is
#      refused. Stopped units have no PID: identity/config/preimage only, no
#      fabricated running path.
#
# Case map:
#   A  existing service updated via system target (root caller, service UID 502
#      declared, ps-verified) — "root + correct UID" positive; kickstart lands
#      on the SYSTEM target, ledger appended, never a gui binding.
#   B  both realms running → ambiguous → FAIL zero writes.
#   C  no realm present → FAIL zero writes.
#   D  root caller + wrong running UID (ps=501 vs expected 502) → FAIL.
#   E  declared preimage hash mismatch → FAIL.
#   F  verify re-uses the recorded SYSTEM binding and passes (lsof reports the
#      real installed path).
#   H  non-root caller + system target → root-admission gate.
#   J  unit program = different path, same content hash → foreign-install gate.
#   K  existing target binary, preimage not declared → required-preimage gate.
#   K2 target binary AND declaration both missing → target-existence gate
#      (first-install pass-through removed).
#   L  preimage declared but target binary absent → same existence gate.
#   M  system unit valid but gui print denied/unknown → denied-first gate.
#   N  gui stopped + system running → running+stopped ambiguity gate.
#   O  corrupt ledger stream: deploy (O1) and verify (O2) refuse.
#   P  multi-record multi-line ledger: verify selects the latest record for the
#      run's sourceSha and re-uses its recorded binding (positive).
#   Q  ledger records system, fresh resolution lands gui → binding-drift gate;
#      Q2: verify against the same state refuses the absent recorded binding.
#   R  stopped unit, same binding, unit config without uid → identity-unverifiable.
#   S  stopped unit, same binding, full evidence → same-binding restore proceeds.
#   T  running unit whose executable is a same-hash copy at another path →
#      running-executable gate, pre-write (deploy).
#   T2 same condition at verify time → running-binary path gate (hash alone
#      would pass — the path check is what fails it).
#   V  cross-version rollback (P5): real different old/new SHAs and binaries.
#      V1 first pinned update (no ledger yet) deploys OLD via system;
#      V2 update to NEW; V3 verify NEW uses the latest NEW record (system
#      binding); V4 restore OLD on the same binding; V5 corrupt ledger refuses
#      the rollback; V6 realm drift refuses the rollback; W missing ledger
#      without a pin refuses pre-write (no realm re-discovery).
#   L  legacy-ledger compatibility (real-site shape: 26 original-schema records,
#      none carrying launchctlTarget):
#      L1 pinned update over a legacy ledger succeeds; the NEW record written
#         by this deploy carries the real binding; the 26 historical records
#         stay byte-identical (no backfill, no migration);
#      L3 one historical MODERN record with a conflicting target (gui) among
#         legacy records → binding-drift refusal even though the latest is
#         legacy;
#      L4 a record with modern-only fields but no launchctlTarget → refused as
#         neither-modern-nor-legacy (distinguishable from genuine legacy);
#      L5 wrong pin over a legacy ledger → binding-drift refusal;
#      L6 wrong declared preimage over a legacy ledger → preimage gate
#         (proves the legacy path reaches later gates in order);
#      L7 after L1: same-target rollback on the MIXED ledger (26 legacy +
#         modern records) succeeds — the modern record now drives binding.
#   Z  assertion self-test: the zero-write/rc checks themselves must be able to
#      fail (deleted/altered fingerprint, missing rc file).

set -u
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
RELEASE_SH="$HERE/release.sh"
MERGE_SHA="18fb2f8bc80983a36b15e65bf51d8f5c4b64fb78"
NEW_SHA="de9d43b8bd7d37d16cb2579f7664b9442f2e39c2"   # real commit in this repo (this PR's fix)
WORK="$(mktemp -d "${TMPDIR:-/tmp}/release-fix-test.XXXXXX")"
FAKEBIN="$WORK/fakebin"
SVC="$WORK/services/svc-workflow"
STATE="$WORK/state"
FOREIGN_INSTALL="$WORK/foreign-install"
SYS_TARGET="system/com.svc-workflow"
mkdir -p "$FAKEBIN" "$STATE" "$FOREIGN_INSTALL"
cp /bin/echo "$FOREIGN_INSTALL/svc-workflow"   # same content hash as the staged binaries, different path
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
# -p PID -a -d txt -Fn  → 运行 executable 路径 = staged-binary-path 状态文件
STAGED=\$(cat "$STATE/staged-binary-path")
echo "n\$STAGED"
EOF
chmod +x "$FAKEBIN/"*

# ── service dir staging (fresh per case; independent temp root) ──────────
stage_release_for() {  # $1 = sourceSha, $2 = binary source (default /bin/echo)
  local sha="$1" bin_src="${2:-/bin/echo}"
  mkdir -p "$SVC/releases/$sha/migrations"
  cp "$bin_src" "$SVC/releases/$sha/svc-workflow"
  echo "# fixture migration" > "$SVC/releases/$sha/migrations/0001_fixture.sql"
  local bsha bdig
  bsha=$(shasum -a 256 "$SVC/releases/$sha/svc-workflow" | awk '{print $1}')
  bdig=$(cd "$SVC/releases/$sha" && find migrations -name '*.sql' -type f | sort | xargs shasum -a 256 | shasum -a 256 | awk '{print $1}')
  jq -n --arg s "$sha" --arg a "$bsha" --arg d "$bdig"     '{sourceSha: $s, treeState: "clean", artifactSha256: $a, builtAt: "2026-10-09T00:00:00Z", buildCommand: "fixture", migrationMaxVersion: "0001", migrationBundleDigest: $d}'     > "$SVC/releases/$sha/provenance.json"
}
stage_release() {
  rm -rf "$SVC"
  stage_release_for "$MERGE_SHA" /bin/echo
}

fingerprint() { (cd "$SVC" && find . -type f | sort | xargs shasum -a 256 2>/dev/null); }

set_state() {  # $1 = launchctl-state, $2 = caller euid (default 0 = root), $3 = running uid (default 502)
  echo "${1:-system}" > "$STATE/launchctl-state"
  printf "%s" "${2:-0}" > "$STATE/fake-euid"
  printf "%s" "${3:-502}" > "$STATE/fake-running-uid"
  printf "%s" "$SVC/svc-workflow" > "$STATE/staged-binary-path"   # P2 正例：lsof 报真实安装路径
}
set_staged_path() { printf '%s' "$1" > "$STATE/staged-binary-path"; }

installed_sha() { shasum -a 256 "$SVC/svc-workflow" | awk '{print $1}'; }
kick_count() {  # $1 = full launchd target; prints count (0 when absent)
  local n; n=$(grep -c "kickstart -k $1" "$STATE/kickstart.log" 2>/dev/null)
  echo "${n:-0}"
}

run_deploy_env() {  # $1 = tag, $2 = sourceSha, remaining: extra env K=V
  local tag="$1" sha="$2"; shift 2
  local before after rc
  before=$(fingerprint)
  env SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502 "$@"     bash "$RELEASE_SH" deploy "$sha" > "$WORK/out-$tag.txt" 2>&1
  rc=$?
  after=$(fingerprint)
  printf '%s' "$rc" > "$WORK/rc-$tag.txt"
  printf '%s' "$before" > "$WORK/fp-before-$tag.txt"
  printf '%s' "$after" > "$WORK/fp-after-$tag.txt"
}

run_deploy() {  # $1 = launchctl-state, $2 = tag, [$3 = euid], [$4 = running uid]
  rm -rf "$SVC"
  stage_release
  cp /bin/echo "$SVC/svc-workflow"   # P3 前提：既有服务的目标 binary 已存在
  set_state "$1" "${3:-0}" "${4:-502}"
  rm -f "$STATE/kickstart.log"
  run_deploy_env "$2" "$MERGE_SHA"     EXPECTED_PREIMAGE_SHA256="$(installed_sha)"     EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
}

run_verify() {  # $1 = tag, $2 = sourceSha (default MERGE_SHA)
  local tag="$1" sha="${2:-$MERGE_SHA}" before after rc
  before=$(fingerprint)
  SVC_WORKFLOW_SERVICE_DIR="$SVC" PATH="$FAKEBIN:$PATH" EXPECTED_SERVICE_UID=502     bash "$RELEASE_SH" verify "$sha" > "$WORK/out-$tag.txt" 2>&1
  rc=$?
  after=$(fingerprint)
  printf '%s' "$rc" > "$WORK/rc-$tag.txt"
  printf '%s' "$before" > "$WORK/fp-before-$tag.txt"
  printf '%s' "$after" > "$WORK/fp-after-$tag.txt"
}

# ── hardened assertion helpers ────────────────────────────────────────────
rc_of() { cat "$WORK/rc-$1.txt" 2>/dev/null; }
have_rc() { if [[ -s "$WORK/rc-$1.txt" ]]; then echo 1; else echo 0; fi; }
fp_intact() {  # 1 仅当前后指纹文件都存在、非空且逐字节相等；缺/空/异 → 0
  local b="$WORK/fp-before-$1.txt" a="$WORK/fp-after-$1.txt"
  if [[ -s "$b" && -s "$a" ]] && cmp -s "$b" "$a"; then echo 1; else echo 0; fi
}
assert() { if [ "$2" = "1" ]; then PASS=$((PASS+1)); echo "PASS $1"; else FAIL=$((FAIL+1)); echo "FAIL $1: $3"; fi; }

# Negative case: rc file present+non-empty+numeric+nonzero, target gate message
# in output, and the INDEPENDENT before/after fingerprints byte-identical.
assert_gate() {  # $1 = case name, $2 = tag, $3 = gate grep pattern
  local rc rcok=0 hit=0
  rc=$(rc_of "$2")
  [ "$(have_rc "$2")" = "1" ] && [[ "$rc" =~ ^[0-9]+$ ]] && [ "$rc" != "0" ] && rcok=1
  grep -q "$3" "$WORK/out-$2.txt" 2>/dev/null && hit=1
  assert "$1: reaches gate [$3]" $([ "$rcok" = "1" ] && [ "$hit" = "1" ] && echo 1 || echo 0) "rc=$rc hit=$hit (see out-$2)"
  assert "$1: ZERO live writes (independent fingerprints)" "$(fp_intact "$2")" "fp missing/empty/differing (fp-*-$2.txt)"
}

# ── A. existing service updated via SYSTEM target (root + correct UID) ───
run_deploy system A
RC=$(rc_of A)
KICK=$(kick_count "$SYS_TARGET"); NOGUI=$(kick_count "gui/502/com.svc-workflow")
LED=$([ -f "$SVC/ledger.json" ] && echo 1 || echo 0)
assert "A: deploy succeeds via SYSTEM target" $([ "$(have_rc A)" = "1" ] && [ "$RC" = "0" ] && [ "$KICK" -ge 1 ] && [ "$LED" = "1" ] && echo 1 || echo 0) "rc=$RC kick=$KICK ledger=$LED"
assert "A: never kickstarts a gui binding" $([ "$NOGUI" = "0" ] && echo 1 || echo 0) "nogui=$NOGUI"

# ── B. both realms present: ambiguous → FAIL, ZERO writes ────────────────
run_deploy both B
assert "B: ambiguous realms fail" $([ "$(have_rc B)" = "1" ] && [ "$(rc_of B)" != "0" ] && echo 1 || echo 0) "rc=$(rc_of B)"
assert "B: ZERO live writes on ambiguity" "$(fp_intact B)" "fp diff"

# ── C. no realm present: FAIL, ZERO writes ────────────────────────────────
run_deploy none C
assert "C: missing unit fails" $([ "$(have_rc C)" = "1" ] && [ "$(rc_of C)" != "0" ] && echo 1 || echo 0) "rc=$(rc_of C)"
assert "C: ZERO live writes on missing unit" "$(fp_intact C)" "fp diff"

# ── D. root caller + wrong running-uid: FAIL, ZERO writes ─────────────────
run_deploy system D 0 501
assert_gate "D: running-uid mismatch" D "service process uid=501 != expected 502"

# ── E. declared preimage mismatch: FAIL, ZERO writes ──────────────────────
stage_release
cp /bin/echo "$SVC/svc-workflow"
set_state system 0 502
rm -f "$STATE/kickstart.log"
run_deploy_env E "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="deadbeef" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
assert_gate "E: preimage mismatch" E "PREIMAGE MISMATCH"

# ── F. verify re-uses the recorded SYSTEM binding and passes ─────────────
run_deploy system F
printf '{"service":"svc-workflow","gitSha":"%s","gitTreeState":"clean"}\n' "$MERGE_SHA" > "$STATE/fake-version.json"
run_verify F
assert "F: verify resolves SYSTEM binding and passes" $([ "$(have_rc F)" = "1" ] && [ "$(rc_of F)" = "0" ] && echo 1 || echo 0) "rc=$(rc_of F) (see out-F)"

# ── H. non-root caller + system target: root-admission gate ───────────────
run_deploy system H 502 502
assert_gate "H: system control requires root" H "system-domain control requires root"

# ── J. unit program = different path, same content hash: foreign install ──
stage_release
set_state foreign 0 502
rm -f "$STATE/kickstart.log"
run_deploy_env J "$MERGE_SHA"
assert_gate "J: same-hash different-path is a foreign install" J "refusing to manage a foreign install"

# ── K. existing target binary, preimage not declared ──────────────────────
stage_release
cp /bin/echo "$SVC/svc-workflow"
set_state system 0 502
rm -f "$STATE/kickstart.log"
run_deploy_env K "$MERGE_SHA" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
assert_gate "K: declared preimage missing" K "declared preimage missing"

# ── K2. target binary AND declaration both missing (no first-install) ─────
stage_release
set_state system 0 502
rm -f "$STATE/kickstart.log"
run_deploy_env K2 "$MERGE_SHA" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
assert_gate "K2: file and declaration missing together" K2 "target preimage missing"

# ── L. preimage declared but target binary absent ─────────────────────────
stage_release
set_state system 0 502
rm -f "$STATE/kickstart.log"
ECHO_SHA=$(shasum -a 256 /bin/echo | awk '{print $1}')
run_deploy_env L "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$ECHO_SHA" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
assert_gate "L: declared preimage but no target binary" L "target preimage missing"

# ── M. system unit valid but gui print denied/unknown → denied-first ─────
run_deploy denied-gui M
assert_gate "M: denied/unknown print rejects before any success masks it" M "denied/unknown launchctl print result"

# ── N. gui stopped + system running: running+stopped ambiguity ────────────
run_deploy runstop N
assert_gate "N: running+stopped two units are ambiguous" N "ambiguous, 2 present units"

# ── O. corrupt ledger stream: deploy and verify must refuse ───────────────
run_deploy system Opre
printf '{ "deployedAt": "corrupt-truncated-record\n' >> "$SVC/ledger.json"
set_state system 0 502
run_deploy_env O1 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$(installed_sha)"
assert_gate "O1: deploy refuses corrupt ledger" O1 "deployment ledger corrupt"
run_verify O2
assert_gate "O2: verify refuses corrupt ledger" O2 "deployment ledger corrupt"

# ── P. multi-record multi-line ledger: verify selects the sha's record ────
run_deploy system P
jq -n --arg t "$(date -u +%Y-%m-%dT%H:%M:%SZ)" --arg s "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa01"     --arg a "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb01"     '{deployedAt: $t, sourceSha: $s, artifactSha256: $a, previousArtifactSha256: "", migrationMaxVersion: "0001", migrationBundleDigest: "ccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc01", launchctlTarget: "system/com.svc-workflow", targetPreimageSha256: "", otherRealmState: "none"}'     >> "$SVC/ledger.json"
set_state system 0 502
printf '{"service":"svc-workflow","gitSha":"%s","gitTreeState":"clean"}\n' "$MERGE_SHA" > "$STATE/fake-version.json"
run_verify P
assert "P: verify passes on multi-record multi-line ledger" $([ "$(have_rc P)" = "1" ] && [ "$(rc_of P)" = "0" ] && echo 1 || echo 0) "rc=$(rc_of P) (see out-P)"
assert "P: verify used the ledger-recorded binding" $([ "$(grep -c 'ledger-recorded binding system/com.svc-workflow' "$WORK/out-P.txt" 2>/dev/null)" -ge 1 ] && echo 1 || echo 0) "no recorded-binding use (see out-P)"

# ── Q. ledger records system, fresh resolution lands gui: drift gate ──────
set_state gui-drift 0 502
KICK_BEFORE=$(kick_count "$SYS_TARGET")
run_deploy_env Q "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$(installed_sha)"
assert_gate "Q: system→gui drift refused (rollback is same-binding only)" Q "binding drift"
assert "Q: no kickstart on drifted realm" $([ "$KICK_BEFORE" = "$(kick_count "$SYS_TARGET")" ] && echo 1 || echo 0) "kick changed"

# ── Q2. verify against the same state: recorded binding absent → refuse ──
run_verify Q2
assert_gate "Q2: verify refuses absent recorded binding (no substitute realm)" Q2 "ledger-recorded binding system/com.svc-workflow not present"

# ── R. stopped unit, same binding, unit config without uid ────────────────
run_deploy system Rpre
set_state sys-stopped-nouid 0 502
run_deploy_env R "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$(installed_sha)"
assert_gate "R: stopped unit without config uid is unverifiable" R "stopped unit identity unverifiable"

# ── S. stopped unit, same binding, full evidence → same-binding restore ──
run_deploy system Spre
set_state sys-stopped 0 502
run_deploy_env S "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$(installed_sha)"
RC=$(rc_of S); KICKS=$(kick_count "$SYS_TARGET"); NOGUI=$(kick_count "gui/502/com.svc-workflow")
assert "S: stopped same-binding restore proceeds" $([ "$(have_rc S)" = "1" ] && [ "$RC" = "0" ] && [ "$KICKS" -ge 2 ] && echo 1 || echo 0) "rc=$RC kicks=$KICKS (see out-S)"
assert "S: restore kickstarts the SAME system binding only" $([ "$NOGUI" = "0" ] && echo 1 || echo 0) "nogui=$NOGUI"

# ── T. running executable is a same-hash copy at another path (deploy) ────
stage_release
cp /bin/echo "$SVC/svc-workflow"
set_state system 0 502
set_staged_path "$FOREIGN_INSTALL/svc-workflow"   # 运行 PID 的 executable 在另一目录（同 hash）
rm -f "$STATE/kickstart.log"
ECHO_SHA=$(shasum -a 256 /bin/echo | awk '{print $1}')
run_deploy_env T "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$ECHO_SHA" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
assert_gate "T: running executable at same-hash different path refuses pre-write" T "running executable (.*) != install target"

# ── T2. same condition at verify time (hash alone would pass) ─────────────
run_deploy system T2pre
printf '{"service":"svc-workflow","gitSha":"%s","gitTreeState":"clean"}\n' "$MERGE_SHA" > "$STATE/fake-version.json"
set_staged_path "$FOREIGN_INSTALL/svc-workflow"
run_verify T2
assert_gate "T2: verify refuses running binary at same-hash different path" T2 "running binary (.*) != install target"

# ── V. cross-version rollback (P5): real different old/new SHA + binary ───
stage_release                          # OLD = MERGE_SHA (/bin/echo)
stage_release_for "$NEW_SHA" /bin/test # NEW：不同 sourceSha、不同 binary 内容
ECHO_SHA=$(shasum -a 256 /bin/echo | awk '{print $1}')
TEST_SHA=$(shasum -a 256 /bin/test | awk '{print $1}')
cp /bin/echo "$SVC/svc-workflow"       # 既有服务安装的是 OLD
set_state system 0 502
rm -f "$STATE/kickstart.log"
run_deploy_env V1 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$ECHO_SHA" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
RC=$(rc_of V1)
assert "V1: first pinned update (no ledger) deploys OLD via system" $([ "$(have_rc V1)" = "1" ] && [ "$RC" = "0" ] && [ "$(kick_count "$SYS_TARGET")" -ge 1 ] && [ -f "$SVC/ledger.json" ] && echo 1 || echo 0) "rc=$RC (see out-V1)"
run_deploy_env V2 "$NEW_SHA" EXPECTED_PREIMAGE_SHA256="$ECHO_SHA"
RC=$(rc_of V2)
assert "V2: update to NEW sha/binary succeeds" $([ "$(have_rc V2)" = "1" ] && [ "$RC" = "0" ] && [ "$(kick_count "$SYS_TARGET")" -ge 2 ] && echo 1 || echo 0) "rc=$RC (see out-V2)"
printf '{"service":"svc-workflow","gitSha":"%s","gitTreeState":"clean"}\n' "$NEW_SHA" > "$STATE/fake-version.json"
run_verify V3 "$NEW_SHA"
assert "V3: verify NEW uses latest NEW record" $([ "$(have_rc V3)" = "1" ] && [ "$(rc_of V3)" = "0" ] && echo 1 || echo 0) "rc=$(rc_of V3) (see out-V3)"
assert "V3: recorded system binding re-used" $([ "$(grep -c 'ledger-recorded binding system/com.svc-workflow' "$WORK/out-V3.txt" 2>/dev/null)" -ge 1 ] && echo 1 || echo 0) "no recorded-binding use (see out-V3)"
run_deploy_env V4 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$TEST_SHA"
RC=$(rc_of V4)
assert "V4: restore OLD on the same system binding" $([ "$(have_rc V4)" = "1" ] && [ "$RC" = "0" ] && [ "$(kick_count "$SYS_TARGET")" -ge 3 ] && [ "$(kick_count "gui/502/com.svc-workflow")" = "0" ] && echo 1 || echo 0) "rc=$RC kicks=$(kick_count "$SYS_TARGET") (see out-V4)"
cp "$SVC/ledger.json" "$STATE/ledger-good.json"
printf '{ "deployedAt": "corrupt-truncated-record\n' >> "$SVC/ledger.json"
run_deploy_env V5 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$ECHO_SHA"
assert_gate "V5: corrupt ledger refuses the rollback" V5 "deployment ledger corrupt"
cp "$STATE/ledger-good.json" "$SVC/ledger.json"
set_state gui-drift 0 502
KICK_BEFORE=$(kick_count "$SYS_TARGET")
run_deploy_env V6 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$ECHO_SHA"
assert_gate "V6: realm drift refuses the rollback" V6 "binding drift"
assert "V6: no kickstart on drifted rollback" $([ "$KICK_BEFORE" = "$(kick_count "$SYS_TARGET")" ] && echo 1 || echo 0) "kick changed"

# ── W. missing ledger + no pin: no realm re-discovery, pre-write refusal ──
rm -f "$SVC/ledger.json"
set_state gui-drift 0 502
KICK_BEFORE=$(kick_count "$SYS_TARGET")
KICK_GUI_BEFORE=$(kick_count "gui/502/com.svc-workflow")
run_deploy_env W "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$ECHO_SHA"
assert_gate "W: missing binding record refuses pre-write" W "deployment binding record missing"
assert "W: no kickstart on any realm" $([ "$KICK_BEFORE" = "$(kick_count "$SYS_TARGET")" ] && [ "$KICK_GUI_BEFORE" = "$(kick_count "gui/502/com.svc-workflow")" ] && echo 1 || echo 0) "kick changed"

# ── L. legacy-ledger compatibility: original six-field records, no binding ─
append_legacy_record() {  # 精确复刻 main 18fb2f8b 原入口写出的记录（六字段、无 binding 字段）
  jq -n --arg deployedAt "$(date -u +%Y-%m-%dT%H:%M:%SZ)"     --arg sourceSha "${1:-$MERGE_SHA}"     --arg artifactSha256 "$LEGACY_ART_SHA"     --arg previousArtifactSha256 ""     --arg migrationMaxVersion "0001"     --arg migrationBundleDigest "$LEGACY_MIG_DIGEST"     '{deployedAt: $deployedAt, sourceSha: $sourceSha, artifactSha256: $artifactSha256, previousArtifactSha256: $previousArtifactSha256, migrationMaxVersion: $migrationMaxVersion, migrationBundleDigest: $migrationBundleDigest}'     >> "$SVC/ledger.json"
}
append_modern_record() {  # $1 = target：本入口 d4baf7e 起写出的记录形状
  jq -n --arg deployedAt "$(date -u +%Y-%m-%dT%H:%M:%SZ)"     --arg sourceSha "$MERGE_SHA"     --arg artifactSha256 "$LEGACY_ART_SHA"     --arg launchctlTarget "$1"     '{deployedAt: $deployedAt, sourceSha: $sourceSha, artifactSha256: $artifactSha256, previousArtifactSha256: "", migrationMaxVersion: "0001", migrationBundleDigest: "$LEGACY_MIG_DIGEST", launchctlTarget: $launchctlTarget, targetPreimageSha256: "", otherRealmState: "none"}'     >> "$SVC/ledger.json"
}
append_modern_missing_field_record() {  # 有现代专有字段但缺 launchctlTarget：既非现代也非 legacy
  jq -n --arg deployedAt "$(date -u +%Y-%m-%dT%H:%M:%SZ)"     --arg sourceSha "$MERGE_SHA"     --arg artifactSha256 "$LEGACY_ART_SHA"     '{deployedAt: $deployedAt, sourceSha: $sourceSha, artifactSha256: $artifactSha256, previousArtifactSha256: "", migrationMaxVersion: "0001", migrationBundleDigest: "$LEGACY_MIG_DIGEST", targetPreimageSha256: "", otherRealmState: "none"}'     >> "$SVC/ledger.json"
}
build_legacy_site() {  # 真实现场形状：既有 binary + 26 条旧 schema 记录（无 binding 字段）
  stage_release
  cp /bin/echo "$SVC/svc-workflow"
  set_state system 0 502
  LEGACY_ART_SHA=$(shasum -a 256 /bin/echo | awk '{print $1}')
  LEGACY_MIG_DIGEST=$(cd "$SVC/releases/$MERGE_SHA" && find migrations -name '*.sql' -type f | sort | xargs shasum -a 256 | shasum -a 256 | awk '{print $1}')
  rm -f "$SVC/ledger.json" "$STATE/kickstart.log"
  local i; for i in $(seq 1 26); do append_legacy_record; done
}

# L1: pinned update over the legacy ledger → success, new record carries the
#     real binding, the 26 historical records stay byte-identical.
build_legacy_site
cp "$SVC/ledger.json" "$STATE/ledger-before-L1.json"
run_deploy_env L1 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$LEGACY_ART_SHA" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
RC=$(rc_of L1)
assert "L1: pinned update succeeds over 26 legacy records" $([ "$(have_rc L1)" = "1" ] && [ "$RC" = "0" ] && [ "$(kick_count "$SYS_TARGET")" -ge 1 ] && echo 1 || echo 0) "rc=$RC (see out-L1)"
assert "L1: new record written with real binding, history untouched" $([ "$(jq -s 'length' "$SVC/ledger.json")" = "27" ]     && [ "$(jq -s '.[-1].launchctlTarget' "$SVC/ledger.json" | tr -d '"')" = "$SYS_TARGET" ]     && [ "$(jq -s --slurpfile before "$STATE/ledger-before-L1.json" '.[0:26] == $before' "$SVC/ledger.json")" = "true" ] && echo 1 || echo 0) "records=$(jq -s 'length' "$SVC/ledger.json") last=$(jq -s '.[-1].launchctlTarget' "$SVC/ledger.json")"

# L7: after L1 — same-target rollback on the MIXED ledger (26 legacy + modern).
run_deploy_env L7 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$(installed_sha)" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
RC=$(rc_of L7)
assert "L7: same-target rollback on mixed ledger succeeds" $([ "$(have_rc L7)" = "1" ] && [ "$RC" = "0" ] && [ "$(kick_count "$SYS_TARGET")" -ge 2 ] && [ "$(kick_count "gui/502/com.svc-workflow")" = "0" ] && echo 1 || echo 0) "rc=$RC kicks=$(kick_count "$SYS_TARGET") (see out-L7)"
assert "L7: latest record still carries the real binding" $([ "$(jq -s '.[-1].launchctlTarget' "$SVC/ledger.json" | tr -d '"')" = "$SYS_TARGET" ] && echo 1 || echo 0) "last=$(jq -s '.[-1].launchctlTarget' "$SVC/ledger.json")"

# L3: one historical MODERN record with a conflicting target → refuse.
build_legacy_site
append_legacy_record; append_legacy_record
append_modern_record "gui/502/com.svc-workflow"   # 冲突 target，位于中段；最新仍是 legacy
append_legacy_record
run_deploy_env L3 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$LEGACY_ART_SHA" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
assert_gate "L3: conflicting historical binding refuses" L3 "binding drift: ledger records gui/502/com.svc-workflow"

# L4: modern-only fields but no launchctlTarget → neither modern nor legacy.
build_legacy_site
append_modern_missing_field_record   # 最新记录：现代字段、无 binding 字段
run_deploy_env L4 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$LEGACY_ART_SHA" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
assert_gate "L4: modern-missing-field is distinguishable from legacy" L4 "nor valid legacy schema"

# L5: wrong pin over a legacy ledger → drift refusal.
build_legacy_site
run_deploy_env L5 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="$LEGACY_ART_SHA" EXPECTED_LAUNCHCTL_TARGET="gui/502/com.svc-workflow"
assert_gate "L5: wrong pin refuses" L5 "binding drift: ledger records gui/502/com.svc-workflow but resolved system/com.svc-workflow"

# L6: wrong declared preimage over a legacy ledger → preimage gate.
build_legacy_site
run_deploy_env L6 "$MERGE_SHA" EXPECTED_PREIMAGE_SHA256="deadbeef" EXPECTED_LAUNCHCTL_TARGET="$SYS_TARGET"
assert_gate "L6: legacy path reaches the preimage gate" L6 "PREIMAGE MISMATCH"

# ── Z. assertion self-test: the checks themselves must be able to fail ────
cp "$WORK/fp-before-O2.txt" "$WORK/fp-before-Z.txt"   # O2 refuses corrupt ledger → intact pair
cp "$WORK/fp-after-O2.txt" "$WORK/fp-after-Z.txt"
assert "Z0: intact fingerprint pair asserts clean" $([ "$(fp_intact Z)" = "1" ] && echo 1 || echo 0) "fp_intact Z=$(fp_intact Z)"
rm -f "$WORK/fp-after-Z.txt"
assert "Z1: deleted after-fingerprint fails the assertion" $([ "$(fp_intact Z)" = "0" ] && echo 1 || echo 0) "fp_intact Z=$(fp_intact Z)"
printf 'tampered\n' > "$WORK/fp-after-Z.txt"
assert "Z2: altered after-fingerprint fails the assertion" $([ "$(fp_intact Z)" = "0" ] && echo 1 || echo 0) "fp_intact Z=$(fp_intact Z)"
assert "Z3: missing rc file fails the gate rc check" $([ "$(have_rc Z-NO-RUN)" = "0" ] && echo 1 || echo 0) "have_rc=$(have_rc Z-NO-RUN)"

echo "RESULT: $PASS passed, $FAIL failed"
[ "$FAIL" = "0" ] || exit 1
