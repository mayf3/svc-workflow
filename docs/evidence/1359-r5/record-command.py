#!/usr/bin/env python3
"""Persist actual delivery command/output/rc at exact local coordinates."""
import datetime
import hashlib
import json
import pathlib
import subprocess
import sys
import time

out = pathlib.Path(__file__).resolve().parent
root = out.parents[2]
tag, *command = sys.argv[1:]
assert command and not (out / f'{tag}.json').exists()
record = {
    'command': command, 'cwd': str(root),
    'started_at': datetime.datetime.now(datetime.timezone.utc).isoformat(),
    'head': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=root, text=True).strip(),
}
start = time.monotonic()
result = subprocess.run(command, cwd=root, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
(out / f'{tag}.stdout').write_bytes(result.stdout)
(out / f'{tag}.stderr').write_bytes(result.stderr)
record.update(
    rc=result.returncode,
    ended_at=datetime.datetime.now(datetime.timezone.utc).isoformat(),
    elapsed_seconds=round(time.monotonic()-start, 3),
    stdout_sha256=hashlib.sha256(result.stdout).hexdigest(),
    stderr_sha256=hashlib.sha256(result.stderr).hexdigest(),
)
(out / f'{tag}.json').write_text(json.dumps(record, indent=2) + '\n')
print(json.dumps(record), flush=True)
sys.exit(result.returncode)
