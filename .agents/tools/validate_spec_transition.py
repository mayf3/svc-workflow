#!/usr/bin/env python3
"""Validate whole-authority lifecycle transitions, including legacy-ID retirement.

Inputs are JSON arrays of governing-Spec frontmatter objects at the transition base
and candidate states. This tool complements JSON Schema: it checks cross-record
existence, lifecycle closure, and atomic backlinks that a single-record schema cannot.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path
from typing import Any

STRICT_SPEC_ID_RE = re.compile(r"^[A-Z][A-Z0-9_]*_V[0-9]+$")
LEGACY_SPEC_ID_RE = re.compile(
    r"^[A-Z][A-Z0-9_]*_V[0-9]+_[A-Z][A-Z0-9_]*$"
)
STATUSES = {"proposed", "accepted", "superseded"}
IMMUTABLE_AUTHORITY_FIELDS = (
    "spec_kind",
    "authority_level",
    "implementation_authority",
    "scope",
    "governed_by",
    "external_authorities",
    "owners",
)


def is_strict_spec_id(value: object) -> bool:
    return isinstance(value, str) and STRICT_SPEC_ID_RE.fullmatch(value) is not None


def is_legacy_spec_id(value: object) -> bool:
    return (
        isinstance(value, str)
        and LEGACY_SPEC_ID_RE.fullmatch(value) is not None
        and not is_strict_spec_id(value)
    )


def is_reference_id(value: object) -> bool:
    return is_strict_spec_id(value) or is_legacy_spec_id(value)


def index_records(records: object, label: str, errors: list[str]) -> dict[str, dict[str, Any]]:
    if not isinstance(records, list):
        errors.append(f"{label} is not an array")
        return {}
    indexed: dict[str, dict[str, Any]] = {}
    for position, record in enumerate(records):
        if not isinstance(record, dict):
            errors.append(f"{label}[{position}] is not an object")
            continue
        spec_id = record.get("spec_id")
        if not isinstance(spec_id, str):
            errors.append(f"{label}[{position}].spec_id is not a string")
            continue
        if spec_id in indexed:
            errors.append(f"{label} contains duplicate spec_id: {spec_id}")
            continue
        indexed[spec_id] = record
    return indexed


def validate_metadata(record: dict[str, Any], label: str = "record") -> list[str]:
    errors: list[str] = []
    spec_id = record.get("spec_id")
    status = record.get("status")

    if status not in STATUSES:
        errors.append(f"{label}.status is invalid")
    if status == "superseded":
        if not is_reference_id(spec_id):
            errors.append(f"{label}.spec_id is not a strict or legacy historical ID")
    elif not is_strict_spec_id(spec_id):
        errors.append(f"{label}.spec_id must be a strict _V<number> ID while active")

    governed_by = record.get("governed_by")
    if not isinstance(governed_by, list):
        errors.append(f"{label}.governed_by is not an array")
    else:
        for authority_id in governed_by:
            if not is_strict_spec_id(authority_id):
                errors.append(f"{label}.governed_by contains a non-strict ID: {authority_id}")

    supersedes = record.get("supersedes")
    if not isinstance(supersedes, list):
        errors.append(f"{label}.supersedes is not an array")
    else:
        if len(supersedes) != len(set(map(str, supersedes))):
            errors.append(f"{label}.supersedes contains duplicates")
        for authority_id in supersedes:
            if not is_reference_id(authority_id):
                errors.append(f"{label}.supersedes contains an invalid whole-authority ID: {authority_id}")

    superseded_by = record.get("superseded_by")
    if status == "superseded":
        if not is_strict_spec_id(superseded_by):
            errors.append(f"{label}.superseded_by must name a strict successor")
    elif superseded_by is not None:
        errors.append(f"{label}.superseded_by must be null while active")

    return errors


def validate_transition(base_records: object, candidate_records: object) -> list[str]:
    errors: list[str] = []
    base = index_records(base_records, "base", errors)
    candidate = index_records(candidate_records, "candidate", errors)

    for spec_id, record in candidate.items():
        errors.extend(validate_metadata(record, f"candidate[{spec_id}]"))

    for spec_id in sorted(set(base) - set(candidate)):
        errors.append(f"candidate omits base authority: {spec_id}")

    for spec_id, base_record in base.items():
        candidate_record = candidate.get(spec_id)
        if candidate_record is None:
            continue
        old_status = base_record.get("status")
        new_status = candidate_record.get("status")
        if old_status in {"accepted", "superseded"}:
            for field in IMMUTABLE_AUTHORITY_FIELDS:
                if candidate_record.get(field) != base_record.get(field):
                    errors.append(f"{spec_id} mutates accepted authority field: {field}")
        allowed_statuses = {
            "proposed": {"proposed", "accepted"},
            "accepted": {"accepted", "superseded"},
            "superseded": {"superseded"},
        }.get(old_status, set())
        if new_status not in allowed_statuses:
            errors.append(f"{spec_id} has forbidden lifecycle transition: {old_status} -> {new_status}")

        if old_status in {"accepted", "superseded"}:
            if candidate_record.get("supersedes") != base_record.get("supersedes"):
                errors.append(f"{spec_id} mutates accepted supersedes metadata")
        if old_status == "accepted" and new_status == "accepted":
            if candidate_record.get("superseded_by") != base_record.get("superseded_by"):
                errors.append(f"{spec_id} mutates active superseded_by metadata")
        if old_status == "superseded":
            if candidate_record.get("superseded_by") != base_record.get("superseded_by"):
                errors.append(f"{spec_id} mutates historical successor backlink")

    base_active_legacy = {
        spec_id
        for spec_id, record in base.items()
        if is_legacy_spec_id(spec_id) and record.get("status") == "accepted"
    }
    candidate_active_legacy = {
        spec_id
        for spec_id, record in candidate.items()
        if is_legacy_spec_id(spec_id) and record.get("status") == "accepted"
    }

    for spec_id in sorted(candidate_active_legacy - base_active_legacy):
        errors.append(f"candidate creates active legacy authority: {spec_id}")

    for spec_id, record in sorted(candidate.items()):
        if is_legacy_spec_id(spec_id) and spec_id not in base:
            errors.append(f"candidate invents legacy authority: {spec_id}")
        if record.get("status") == "superseded" and spec_id not in base:
            errors.append(f"candidate creates already-superseded authority: {spec_id}")

    referenced_old: dict[str, list[str]] = {}
    for successor_id, successor in candidate.items():
        supersedes = successor.get("supersedes")
        if not isinstance(supersedes, list):
            continue
        for old_id in supersedes:
            if not isinstance(old_id, str) or not is_reference_id(old_id):
                continue
            referenced_old.setdefault(old_id, []).append(successor_id)
            old_base = base.get(old_id)
            old_candidate = candidate.get(old_id)
            if old_base is None:
                errors.append(f"{successor_id} supersedes nonexistent base authority: {old_id}")
                continue
            if successor.get("status") != "accepted" or not is_strict_spec_id(successor_id):
                errors.append(f"successor must be accepted with a strict ID: {successor_id}")
            if old_candidate is None:
                errors.append(f"candidate omits superseded authority: {old_id}")
                continue

            if old_base.get("status") == "accepted":
                if old_base.get("authority_level") != "governing_spec":
                    errors.append(f"{old_id} was not a governing authority in base")
                base_successor = base.get(successor_id)
                if base_successor is not None and base_successor.get("status") != "proposed":
                    errors.append(f"successor was already normative in base: {successor_id}")
                if old_candidate.get("status") != "superseded":
                    errors.append(f"{old_id} is not superseded atomically")
                if old_candidate.get("superseded_by") != successor_id:
                    errors.append(f"{old_id} backlink does not name {successor_id}")
            elif old_base.get("status") == "superseded":
                if old_base.get("superseded_by") != successor_id:
                    errors.append(f"historical edge changed for {old_id}")
            else:
                errors.append(f"{old_id} was not accepted or superseded in base")

    for old_id, old_candidate in candidate.items():
        if old_candidate.get("status") != "superseded":
            continue
        successor_id = old_candidate.get("superseded_by")
        if not is_strict_spec_id(successor_id):
            errors.append(f"{old_id} is superseded without a strict successor backlink")
            continue
        successor = candidate.get(successor_id)
        if successor is None:
            errors.append(f"{old_id} backlinks nonexistent successor: {successor_id}")
            continue
        if successor.get("status") != "accepted":
            errors.append(f"{old_id} backlinks non-accepted successor: {successor_id}")
        supersedes = successor.get("supersedes")
        if not isinstance(supersedes, list) or old_id not in supersedes:
            errors.append(f"{successor_id} does not backlink superseded authority: {old_id}")

    for old_id, successors in referenced_old.items():
        if len(successors) > 1:
            errors.append(f"{old_id} is superseded by multiple successors: {', '.join(sorted(successors))}")

    if not candidate_active_legacy.issubset(base_active_legacy):
        errors.append("active legacy authority set increased")

    return errors


def load_records(path: Path) -> object:
    return json.loads(path.read_text(encoding="utf-8"))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True, type=Path)
    parser.add_argument("--candidate", required=True, type=Path)
    args = parser.parse_args(argv)

    try:
        errors = validate_transition(load_records(args.base), load_records(args.candidate))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"cannot read transition input: {exc}", file=sys.stderr)
        return 2

    if errors:
        print("Spec transition validation failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print("Spec transition is a closed whole-authority lifecycle change")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
