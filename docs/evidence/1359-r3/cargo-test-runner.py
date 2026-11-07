#!/usr/bin/env python3
"""Cargo test executable runner: route existing fixture constraints safely."""
import json
import os
import pathlib
import subprocess
import sys

pg = pathlib.Path("/opt/homebrew/opt/postgresql@16/bin")
name = pathlib.Path(sys.argv[1]).name.split("-")[0]
env = os.environ.copy()
for key in ("AA_SHARED_BASELINE", "AA_REUSE_CANCELLED_RED", "ACB_OLD_BINARY",
            "CANONICAL_CREATE_TEST_PRINCIPAL_ID"):
    env.pop(key, None)
port, user = 56589, "postgres"
database = "h1359_r3_" + name.lower()
if name == "41_current_task_input":
    port, database = 55432, "svc_workflow_test_current_input_20261027_h1359r3"
elif name == "42_acceptance_binding_downgrade_gate":
    port, database = 55432, "svc_workflow_acb_impl_27oct"
elif name in ("46_owner_obligations", "47_owner_node_reference_authoring"):
    port, database = 55432, "svc_obl_20261029_impl"
elif name == "67_authorized_actions_r4":
    port, user, database = 55483, "swaar4", "h1359_r3_authorized_actions"
    env["AA_RESULT_DIR"] = "/private/tmp/h1359-r3-aa-results"
    pathlib.Path(env["AA_RESULT_DIR"]).mkdir(exist_ok=True)
args = [str(pg / "psql"), "-h", "127.0.0.1", "-p", str(port), "-U", user,
        "-d", "postgres", "-XAt", "-v", "ON_ERROR_STOP=1"]
exists = subprocess.check_output(args + ["-c", f"SELECT 1 FROM pg_database WHERE datname='{database}'"], text=True).strip()
if not exists:
    subprocess.run([str(pg / "createdb"), "-h", "127.0.0.1", "-p", str(port), "-U", user, database], check=True)
env["TEST_DATABASE_URL"] = f"postgres://{user}@127.0.0.1:{port}/{database}"
env["DATABASE_URL"] = env["TEST_DATABASE_URL"]
env["TEST_WORKFLOW_DATABASE_URL"] = "postgres://postgres@127.0.0.1:56589/svc_workflow_cutover_test"
env["TEST_AUTH_DATABASE_URL"] = "postgres://postgres@127.0.0.1:56589/auth_cutover_test"
record = {"binary": name, "database": env["TEST_DATABASE_URL"],
          "command": sys.argv[1:], "started_at": __import__("datetime").datetime.now(__import__("datetime").timezone.utc).isoformat()}
result = subprocess.run(sys.argv[1:], env=env)
record["rc"] = result.returncode
with pathlib.Path(__file__).with_name("full-binary-results.jsonl").open("a") as log:
    log.write(json.dumps(record) + "\n")
sys.exit(result.returncode)
