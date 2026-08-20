#!/usr/bin/env python3
"""Verify vendored governance bytes and adoption metadata against the local lock."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

DISTRIBUTION = "development-governance-v0"
SOURCE_REPOSITORY = "mayf3/agent-development-governance"
COMMIT_RE = re.compile(r"^[0-9a-f]{40}$")
SHA_RE = re.compile(r"^[0-9a-f]{64}$")


def sha256_bytes(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def parse_timestamp(value: object, field: str, errors: list[str]) -> dt.datetime | None:
    if not isinstance(value, str):
        errors.append(f"{field} is not a timestamp string")
        return None
    try:
        parsed = dt.datetime.fromisoformat(value.replace("Z", "+00:00"))
    except ValueError:
        errors.append(f"{field} is not valid ISO-8601")
        return None
    if parsed.tzinfo is None:
        errors.append(f"{field} has no timezone")
        return None
    return parsed.astimezone(dt.timezone.utc)


def safe_path(target: Path, relative: str) -> Path:
    rel = Path(relative)
    if not relative or rel.is_absolute() or ".." in rel.parts:
        raise ValueError(f"unsafe lock path: {relative}")
    path = (target / rel).resolve()
    root = target.resolve()
    if root != path and root not in path.parents:
        raise ValueError(f"lock path escapes target: {relative}")
    return path


def verify_adoption(
    adoption: object, errors: list[str], require_accepted: bool
) -> None:
    if not isinstance(adoption, dict):
        errors.append("adoption is not an object")
        return

    allowed = {
        "mode",
        "status",
        "prepared_by",
        "prepared_at",
        "accepted_by",
        "accepted_at",
    }
    unknown = set(adoption) - allowed
    if unknown:
        errors.append(f"adoption has unknown fields: {', '.join(sorted(unknown))}")

    if adoption.get("mode") != "vendored":
        errors.append("adoption.mode is not vendored")
    status = adoption.get("status")
    if status not in {"proposed", "accepted"}:
        errors.append("adoption.status is not proposed or accepted")
        return
    if require_accepted and status != "accepted":
        errors.append("adoption is not accepted")

    prepared_by = adoption.get("prepared_by")
    if not isinstance(prepared_by, str) or not prepared_by.strip():
        errors.append("adoption.prepared_by is empty")
    prepared_at = parse_timestamp(adoption.get("prepared_at"), "adoption.prepared_at", errors)

    accepted_by = adoption.get("accepted_by")
    accepted_at_value = adoption.get("accepted_at")
    if status == "proposed":
        if accepted_by is not None or accepted_at_value is not None:
            errors.append("proposed adoption claims acceptance metadata")
        return

    if not isinstance(accepted_by, str) or not accepted_by.strip():
        errors.append("accepted adoption has no accepted_by")
    accepted_at = parse_timestamp(
        accepted_at_value, "adoption.accepted_at", errors
    )
    if prepared_at is not None and accepted_at is not None and accepted_at < prepared_at:
        errors.append("adoption.accepted_at precedes prepared_at")


def verify(target: Path, require_accepted: bool = False) -> list[str]:
    errors: list[str] = []
    lock_path = target / ".agents/governance.lock.json"
    if not lock_path.is_file():
        return ["missing .agents/governance.lock.json"]

    try:
        lock: Any = json.loads(lock_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        return [f"cannot read lock: {exc}"]
    if not isinstance(lock, dict):
        return ["governance lock is not an object"]

    allowed = {
        "schema_version",
        "distribution",
        "version",
        "source_repository",
        "source_commit",
        "distribution_manifest_sha256",
        "adoption",
        "files",
    }
    unknown = set(lock) - allowed
    if unknown:
        errors.append(f"lock has unknown fields: {', '.join(sorted(unknown))}")

    if lock.get("schema_version") != 1:
        errors.append("unsupported lock schema_version")
    if lock.get("distribution") != DISTRIBUTION:
        errors.append("unexpected distribution")
    version = lock.get("version")
    if not isinstance(version, str) or not version:
        errors.append("version is empty")
    if lock.get("source_repository") != SOURCE_REPOSITORY:
        errors.append("unexpected source_repository")
    source_commit = lock.get("source_commit")
    if not isinstance(source_commit, str) or not COMMIT_RE.fullmatch(source_commit):
        errors.append("source_commit is not lowercase 40-hex")
    manifest_digest = lock.get("distribution_manifest_sha256")
    if not isinstance(manifest_digest, str) or not SHA_RE.fullmatch(manifest_digest):
        errors.append("distribution_manifest_sha256 is not lowercase 64-hex")

    verify_adoption(lock.get("adoption"), errors, require_accepted)

    files = lock.get("files")
    if not isinstance(files, list) or not files:
        errors.append("lock contains no distributed files")
        return errors

    seen: set[str] = set()
    for entry in files:
        if not isinstance(entry, dict):
            errors.append("invalid file entry")
            continue
        unknown_entry = set(entry) - {"path", "sha256", "size"}
        if unknown_entry:
            errors.append(
                "file entry has unknown fields: " + ", ".join(sorted(unknown_entry))
            )
        relative = entry.get("path")
        expected_sha = entry.get("sha256")
        expected_size = entry.get("size")
        if not isinstance(relative, str):
            errors.append("file entry missing path")
            continue
        if relative in seen:
            errors.append(f"duplicate locked path: {relative}")
            continue
        seen.add(relative)
        if not isinstance(expected_sha, str) or not SHA_RE.fullmatch(expected_sha):
            errors.append(f"invalid sha256 for {relative}")
            continue
        if not isinstance(expected_size, int) or expected_size < 0:
            errors.append(f"invalid size for {relative}")
            continue
        try:
            path = safe_path(target, relative)
        except ValueError as exc:
            errors.append(str(exc))
            continue
        if not path.is_file():
            errors.append(f"missing vendored file: {relative}")
            continue
        data = path.read_bytes()
        if len(data) != expected_size:
            errors.append(
                f"size mismatch for {relative}: expected {expected_size}, got {len(data)}"
            )
        actual_sha = sha256_bytes(data)
        if actual_sha != expected_sha:
            errors.append(
                f"sha256 mismatch for {relative}: expected {expected_sha}, got {actual_sha}"
            )

    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--target", required=True, type=Path)
    parser.add_argument(
        "--require-accepted",
        action="store_true",
        help="also fail when the local adoption lock is still proposed",
    )
    args = parser.parse_args(argv)

    try:
        errors = verify(args.target, require_accepted=args.require_accepted)
    except OSError as exc:
        print(f"verification failed: {exc}", file=sys.stderr)
        return 2

    if errors:
        print("vendored governance verification failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    suffix = " and adoption is accepted" if args.require_accepted else ""
    print(f"vendored governance bytes match governance.lock.json{suffix}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
