#!/usr/bin/env python3
"""Execute one command, preserving exact output, coordinates, and real rc."""
import datetime
import hashlib
import json
import os
import pathlib
import subprocess
import sys
import time

tag, cwd, *command = sys.argv[1:]
out = pathlib.Path(__file__).resolve().parent
root = pathlib.Path(cwd)
record = {
    "command": command,
    "cwd": str(root),
    "head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=root, text=True).strip(),
    "started_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
    "scope": "task-owned disposable PostgreSQL; loopback fakes; no production access",
    "environment": {k: os.environ[k] for k in (
        "TEST_DATABASE_URL", "DATABASE_URL", "CARGO_TARGET_DIR", "RUSTFLAGS",
        "H1359_BACKOFF_ATTRIBUTION", "CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER"
    ) if k in os.environ},
    "hashes": {p: hashlib.sha256((root / p).read_bytes()).hexdigest() for p in (
        "src/forum_sync.rs", "src/store/postgres/outbox.rs", "tests/48_outbox_three_channels.rs", "docs/evidence/1359-r4/cargo-test-runner.py"
    )},
}
start = time.monotonic()
with (out / (tag + ".log")).open("wb") as log:
    result = subprocess.run(command, cwd=root, stdout=log, stderr=subprocess.STDOUT)
record.update(rc=result.returncode, elapsed_seconds=round(time.monotonic()-start, 3),
              ended_at=datetime.datetime.now(datetime.timezone.utc).isoformat())
(out / (tag + ".json")).write_text(json.dumps(record, indent=2) + "\n")
print(json.dumps(record), flush=True)
sys.exit(result.returncode)
