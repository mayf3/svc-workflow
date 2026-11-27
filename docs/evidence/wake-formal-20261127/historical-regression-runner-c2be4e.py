#!/usr/bin/env python3
"""Run bounded Rust regressions in a new, owned, loopback-only PG cluster.

Requires PostgreSQL 16 binaries and the repository's Rust toolchain/dependencies.
Never reads TEST_DATABASE_URL or connects to an existing database. Each run
creates a disposable cluster and keeps postgres and cargo in one process tree.
An unsuccessful run remains unsuccessful; the runner does not update proof
claims or reinterpret expected assertion failures as passing tests.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import signal
import socket
import subprocess
import sys
import tempfile
import time


def source_manifest(repo):
    paths = set()
    for directory in ("src", "tests", "migrations"):
        paths.update(path for path in (repo / directory).rglob("*") if path.is_file())
    # Include exact external-to-src compile inputs, including existing cfg(test)
    # include_str! specs. Missing conditional inputs stay explicit in the receipt.
    for path in list(paths):
        if path.suffix == ".rs":
            for relative in re.findall(r'include_(?:str|bytes)!\s*\(\s*"([^"]+)"', path.read_text()):
                included = (path.parent / relative).resolve()
                included.relative_to(repo)
                paths.add(included)
    paths.update(repo / name for name in ("Cargo.toml", "Cargo.lock", "build.rs")
                 if (repo / name).is_file())
    return {str(path.relative_to(repo)): (hashlib.sha256(path.read_bytes()).hexdigest() if path.is_file() else "MISSING")
            for path in sorted(paths)}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--postgres-bin", type=Path, required=True)
    parser.add_argument("--cargo", type=Path, required=True)
    parser.add_argument("--repository", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--suite", choices=("wake", "wake-database", "outbox"), default="wake")
    parser.add_argument("--out-dir", type=Path, required=True)
    args = parser.parse_args()
    repo, pg, out = (p.resolve() for p in
                     (args.repository, args.postgres_bin, args.out_dir))
    # Preserve the executable name of rustup's cargo proxy symlink.
    cargo = args.cargo.absolute()
    if out.exists() and any(out.iterdir()):
        parser.error("--out-dir must be new or empty; existing evidence is never overwritten")
    out.mkdir(parents=True, exist_ok=True)
    env = {key: value for key, value in os.environ.items()
           if not key.startswith("PG") and key not in ("DATABASE_URL", "TEST_DATABASE_URL")}
    env.setdefault("CARGO_BUILD_JOBS", "4")
    env.setdefault("CARGO_INCREMENTAL", "0")
    env["PGCONNECT_TIMEOUT"] = "2"
    before = source_manifest(repo)
    (out / "source-manifest.json").write_text(json.dumps(before, indent=2) + "\n")
    receipt = {
        "status": "RUNNING", "suite": args.suite,
        "repository": str(repo),
        "source_head": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=repo, text=True).strip(),
        "source_manifest_sha256": hashlib.sha256((out / "source-manifest.json").read_bytes()).hexdigest(),
        "runner_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
        "commands": [],
        "boundaries": ["disposable local PostgreSQL, no production access",
                       "Rust/SQL regression evidence, not a proof of PostgreSQL or live-task acceptance",
                       "deferred constraint rejection is a known rollback, not lost commit acknowledgement"],
    }
    proc = None
    pg_log = None
    exit_code = 1
    try:
        receipt["postgres_version"] = subprocess.check_output([str(pg / "postgres"), "--version"], text=True).strip()
        receipt["cargo_version"] = subprocess.check_output([str(cargo), "--version"], env=env, text=True).strip()
        receipt["rustc_version"] = subprocess.check_output([str(cargo.parent / "rustc"), "--version"], env=env, text=True).strip()
        # Shared target directories across worktrees can retain this package's
        # old library. Preserve dependency builds but force the selected package
        # and tests to be rebuilt from the frozen input tree.
        with (out / "cargo-clean.log").open("w") as log:
            subprocess.run([str(cargo), "clean", "--package", "svc-workflow"],
                           cwd=repo, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
        receipt["package_artifacts_cleaned"] = True
        with tempfile.TemporaryDirectory(prefix="svc-formal-pg-") as temp:
            data = Path(temp) / "data"
            with socket.socket() as reserve:
                reserve.bind(("127.0.0.1", 0))
                port = reserve.getsockname()[1]
            with (out / "initdb.log").open("w") as log:
                subprocess.run([str(pg / "initdb"), "-D", str(data), "-U", "postgres",
                                "--auth=trust", "--encoding=UTF8", "--locale=C"],
                               env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
            pg_log = (out / "postgres.log").open("w")
            proc = subprocess.Popen([str(pg / "postgres"), "-D", str(data), "-h", "127.0.0.1",
                                     "-p", str(port), "-c", "unix_socket_directories=",
                                     "-c", "timezone=UTC", "-c", "log_timezone=UTC"],
                                    env=env, stdout=pg_log, stderr=subprocess.STDOUT)
            try:
                for _ in range(100):
                    if proc.poll() is not None:
                        raise RuntimeError("owned PostgreSQL exited before becoming ready")
                    ready = subprocess.run([str(pg / "pg_isready"), "-h", "127.0.0.1", "-p", str(port), "-U", "postgres"],
                                           env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
                    if ready.returncode == 0:
                        break
                    time.sleep(0.1)
                else:
                    raise RuntimeError("owned PostgreSQL readiness timed out")
                maintenance_url = f"postgres://postgres@127.0.0.1:{port}/postgres"
                ownership = subprocess.check_output(
                    [str(pg / "psql"), maintenance_url, "-XAt", "-v", "ON_ERROR_STOP=1",
                     "-c", "SELECT current_setting('data_directory'), current_setting('port');"],
                    env=env, text=True).strip()
                own_pid = int((data / "postmaster.pid").read_text().splitlines()[0])
                if ownership != f"{data}|{port}" or own_pid != proc.pid or proc.poll() is not None:
                    raise RuntimeError("read-only cluster ownership verification failed before database creation")
                receipt["cluster_ownership_verified_before_writes"] = True
                subprocess.run([str(pg / "createdb"), "-h", "127.0.0.1", "-p", str(port),
                                "-U", "postgres", "formal_tests"], env=env, check=True)
                env["TEST_DATABASE_URL"] = f"postgres://postgres@127.0.0.1:{port}/formal_tests"
                identity = subprocess.check_output(
                    [str(pg / "psql"), env["TEST_DATABASE_URL"], "-XAt", "-v", "ON_ERROR_STOP=1",
                     "-c", "SELECT current_database(), current_user, inet_server_addr(), current_setting('port');"],
                    env=env, text=True).strip()
                receipt["database_identity"] = identity
                commands = (
                    [("wake-codec", ["test", "--locked", "--lib", "wake_transaction::tests", "--", "--test-threads=1"]),
                     ("visit-activation", ["test", "--locked", "--test", "28_visit_activation_v1", "--", "--test-threads=1"])]
                    if args.suite in ("wake", "wake-database") else
                    [("outbox", ["test", "--locked", "--test", "48_outbox_three_channels", "--", "--test-threads=2"])])
                if args.suite == "wake-database":
                    commands = commands[1:]
                exit_code = 0
                for name, command in commands:
                    print(f"Running {name} against owned PostgreSQL {port}", flush=True)
                    started = time.monotonic()
                    with (out / f"{name}.log").open("w") as log:
                        result = subprocess.run([str(cargo), *command], cwd=repo, env=env,
                                                stdout=log, stderr=subprocess.STDOUT)
                    matches = re.findall(r"test result: (?:ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored;", (out / f"{name}.log").read_text())
                    counts = [{"passed": int(p), "failed": int(f), "ignored": int(i)} for p, f, i in matches]
                    receipt["commands"].append({"name": name, "arguments": command,
                                                "exit_code": result.returncode,
                                                "test_counts": counts,
                                                "elapsed_seconds": round(time.monotonic() - started, 3)})
                    print(f"{name}: exit {result.returncode}", flush=True)
                    if result.returncode or not counts or sum(row["passed"] for row in counts) == 0:
                        exit_code = 1
                        break
            finally:
                if proc.poll() is None:
                    proc.send_signal(signal.SIGINT)
                    try:
                        proc.wait(timeout=20)
                    except subprocess.TimeoutExpired:
                        proc.kill()
                        proc.wait()
                receipt["owned_postgres_stopped"] = proc.poll() is not None
                pg_log.close()
    except Exception as error:
        receipt["error"] = f"{type(error).__name__}: {error}"
        exit_code = 1
    finally:
        receipt["sources_unchanged"] = before == source_manifest(repo)
        if not receipt["sources_unchanged"]:
            exit_code = 1
        receipt["status"] = "PASS_LOCAL_PG_REGRESSIONS" if exit_code == 0 else "FAIL_LOCAL_PG_REGRESSIONS"
        (out / "regression-receipt.json").write_text(json.dumps(receipt, indent=2) + "\n")
        print(json.dumps({key: receipt[key] for key in ("status", "suite", "sources_unchanged", "commands")}), flush=True)
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
