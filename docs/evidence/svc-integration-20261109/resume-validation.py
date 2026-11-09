#!/usr/bin/env python3
"""Reproduce only task-isolated library and three-channel acceptance."""
import datetime, hashlib, json, os, re, subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
OUT = Path(__file__).resolve().parent
PG = "/opt/homebrew/opt/postgresql@16/bin/psql"
DSN = "postgres://svc_integration_test@127.0.0.1:55947/svc_integration_20261109_lib"
EXPECTED = "svc_integration_20261109_lib|svc_integration_test|127.0.0.1|55947|/private/tmp/svc-integration/postgres/data"

def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()

def save(name, value):
    (OUT / name).write_text(json.dumps(value, ensure_ascii=False, indent=2) + "\n")

def hashes():
    names = subprocess.check_output(["git", "ls-files"], cwd=ROOT, text=True).splitlines()
    names = [n for n in names if n.endswith((".rs", ".sql")) or n in
             ["Cargo.toml", "Cargo.lock", "scripts/release.sh", "scripts/test_release.sh"]]
    return {n: hashlib.sha256((ROOT / n).read_bytes()).hexdigest() for n in names}

env = dict(os.environ)
removed = []
for key in list(env):
    if key.startswith(("WORKFLOW_", "PG", "SVC_WORKFLOW_RELEASE_")) or key in (
        "DATABASE_URL", "AUTH_DATABASE_URL", "TEST_DATABASE_URL", "OWNER_WAKE_TEST_DATABASE_URL",
        "HTTP_PROXY", "HTTPS_PROXY", "ALL_PROXY", "http_proxy", "https_proxy", "all_proxy"):
        removed.append(key)
        env.pop(key)
env["TEST_DATABASE_URL"] = DSN
env["NO_PROXY"] = env["no_proxy"] = "127.0.0.1,localhost,::1"
cmd = [PG, "-X", "-w", DSN, "-At", "-c",
       "SELECT current_database(),current_user,inet_server_addr(),inet_server_port(),current_setting('data_directory');"]
r = subprocess.run(cmd, cwd=ROOT, env=env, capture_output=True, text=True)
identity = {"evaluated_at": now(), "command": cmd, "rc": r.returncode,
            "stdout": r.stdout, "stderr": r.stderr, "expected": EXPECTED,
            "removed_environment_keys": sorted(removed)}
save("postgres-resume-identity.json", identity)
assert r.returncode == 0 and r.stdout.strip() == EXPECTED, "Task PG identity mismatch: abort before tests"
before = hashes()
save("source-hashes-before.json", before)
head = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
results = []
for name, selector in [("library", ["--lib"]), ("channels", ["--test", "48_outbox_three_channels"])]:
    command = ["cargo", "test", "--locked", "--offline", "--jobs", "4"] + selector + ["--", "--test-threads=1", "--nocapture"]
    started = now()
    print("START", name, started, flush=True)
    with (OUT / (name + ".log")).open("w") as log:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
    raw = (OUT / (name + ".log")).read_text()
    counts = re.findall(r"test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out", raw)
    value = {"command": command, "started_at": started, "finished_at": now(), "rc": result.returncode,
             "candidate_head": head, "test_database_url": DSN,
             "summaries": [{"status": s, "passed": int(p), "failed": int(f), "ignored": int(i),
                            "measured": int(m), "filtered_out": int(x)} for s,p,f,i,m,x in counts]}
    save(name + ".json", value)
    results.append(value)
    print("FINISH", name, json.dumps(value), flush=True)
after = hashes()
save("source-hashes-after.json", after)
save("source-integrity.json", {"evaluated_at": now(), "files_checked": len(before),
     "unchanged": before == after, "changed": [n for n in set(before) | set(after) if before.get(n) != after.get(n)]})
assert before == after, "Source changed during validation"
assert all(r["rc"] == 0 and r["summaries"] and all(s["failed"] == 0 for s in r["summaries"]) for r in results), "Actual test failure; retain all logs"
