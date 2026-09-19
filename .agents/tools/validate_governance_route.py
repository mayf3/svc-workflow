#!/usr/bin/env python3
"""Validate deterministic consistency of a declared Governance V1 route.

This tool does not decide semantic ownership, Contract completeness, or real-world
Evidence sufficiency. It checks whether declared structured facts are internally
consistent with the accepted routing rules.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

AUTHORITY_ACTIONS = {
    "REUSE",
    "AMEND",
    "SUPERSEDE",
    "NEW",
    "AMEND_OR_NEW_PENDING_OWNERSHIP",
}
PLAN_LEVELS = {"NONE", "BRIEF", "EXEC_PLAN"}
ASSURANCE_LEVELS = {"ROUTINE", "DURABLE", "CONTROLLED"}
ROUTE_STAGES = {"AUTHORITY_AUTHORING", "IMPLEMENTATION", "OPERATION"}
AUTHORITY_ACCEPTED_IN_BASE = {"YES", "NO", "NOT_APPLICABLE"}
READINESS = {"YES", "NO", "NOT_APPLICABLE"}
NEXT_ACTIONS = {"CONTINUE", "STOP", "RE_PREFLIGHT", "OWNER_DECISION"}
BLOCKER_CLASSES = {
    "CONTRACT_VIOLATION",
    "REPOSITORY_INVARIANT_VIOLATION",
    "CONCRETE_REGRESSION",
    "SECURITY_OR_DATA_LOSS",
    "FALSE_EVIDENCE",
    "SCOPE_ESCALATION",
    "REQUIRED_GATE_FAILURE",
}
LEGAL_SOURCE_TYPES = {
    "ACCEPTED_PRODUCT_AUTHORITY",
    "ACCEPTED_LOCAL_GOVERNANCE",
    "MACHINE_GATE",
    "EXECUTION_MANDATE",
}
NON_BLOCKER_KINDS = {"SPEC_GAP", "FOLLOW_UP", "TOOLING_DEBT"}
EMERGENCY_STATES = {"NONE", "ACTIVE"}
EMERGENCY_ACTIONS = {
    "NONE",
    "ROLLBACK",
    "DISABLEMENT",
    "SHUTDOWN",
    "REVOCATION",
    "ISOLATION",
    "CONTAINMENT",
}


def _mapping(value: Any, label: str, errors: list[str]) -> dict[str, Any]:
    if not isinstance(value, dict):
        errors.append(f"{label} must be an object")
        return {}
    return value


def _required_text(record: dict[str, Any], field: str, errors: list[str]) -> None:
    value = record.get(field)
    if not isinstance(value, str) or not value.strip():
        errors.append(f"{field} must be non-empty text")


def expected_authority_action(authority: dict[str, Any]) -> str:
    if authority.get("ownership_known") is False:
        return "AMEND_OR_NEW_PENDING_OWNERSHIP"
    if authority.get("accepted_meaning_changed") is True:
        return "SUPERSEDE"
    if authority.get("proposed_target_named") is True:
        if any(
            authority.get(field) is True
            for field in (
                "proposal_scope_changed",
                "proposal_ownership_changed",
                "proposal_decision_identity_changed",
            )
        ):
            return "NEW"
        return "AMEND"
    if authority.get("accepted_owner_exists") is True:
        return "AMEND" if authority.get("accepted_strict_addition") is True else "REUSE"
    return "NEW"


def expected_plan_level(complexity: str) -> str | None:
    return {
        "TRIVIAL": "NONE",
        "BOUNDED": "BRIEF",
        "COMPLEX": "EXEC_PLAN",
    }.get(complexity)


def expected_assurance_level(consequence: str) -> str | None:
    return {
        "LOW": "ROUTINE",
        "DURABLE_STATE": "DURABLE",
        "HIGH_RISK": "CONTROLLED",
    }.get(consequence)


def mandate_is_general_authorization(mandate: dict[str, Any]) -> bool:
    return (
        mandate.get("status") == "VALID"
        and mandate.get("attributable") is True
        and mandate.get("target_bound") is True
        and mandate.get("scope_bound") is True
        and mandate.get("allowed_effects_bound") is True
        and mandate.get("forbidden_effects_bound") is True
        and mandate.get("done_when_bound") is True
        and mandate.get("self_issued") is False
    )


def mandate_is_controlled(mandate: dict[str, Any]) -> bool:
    return (
        mandate_is_general_authorization(mandate)
        and mandate.get("actor_bound") is True
        and mandate.get("environment_bound") is True
        and mandate.get("exact_operation_bound") is True
        and mandate.get("abort_conditions_bound") is True
        and mandate.get("secret_handling_bound") is True
        and mandate.get("receipt_bound") is True
        and mandate.get("attempt_bounds_bound") is True
    )


def validate_route(record: Any) -> list[str]:
    errors: list[str] = []
    if not isinstance(record, dict):
        return ["route record must be an object"]
    if record.get("schema_version") != 1:
        errors.append("schema_version must be 1")
    for field in ("task_id", "goal", "current_gap", "done_when"):
        _required_text(record, field, errors)

    authority = _mapping(record.get("authority"), "authority", errors)
    plan = _mapping(record.get("plan"), "plan", errors)
    assurance = _mapping(record.get("assurance"), "assurance", errors)
    mandate = _mapping(record.get("execution_mandate"), "execution_mandate", errors)
    write_surface = _mapping(record.get("write_surface"), "write_surface", errors)
    evidence = _mapping(record.get("evidence"), "evidence", errors)
    live_gap = _mapping(record.get("live_authority_gap"), "live_authority_gap", errors)
    emergency = _mapping(record.get("emergency"), "emergency", errors)
    review = _mapping(record.get("review"), "review", errors)
    readiness = _mapping(record.get("readiness"), "readiness", errors)
    stop = _mapping(record.get("stop"), "stop", errors)

    route_stage = record.get("route_stage")
    if route_stage not in ROUTE_STAGES:
        errors.append("route_stage is invalid")

    accepted_in_base = record.get("authority_accepted_in_base")
    if accepted_in_base not in AUTHORITY_ACCEPTED_IN_BASE:
        errors.append("authority_accepted_in_base is invalid")

    owner_decision_required = record.get("owner_decision_required")
    if not isinstance(owner_decision_required, bool):
        errors.append("owner_decision_required must be boolean")

    atomic_spec_implementation_permitted = authority.get(
        "atomic_spec_implementation_permitted"
    )
    if not isinstance(atomic_spec_implementation_permitted, bool):
        errors.append(
            "authority.atomic_spec_implementation_permitted must be boolean"
        )

    declared_action = authority.get("declared_action")
    if declared_action not in AUTHORITY_ACTIONS:
        errors.append("authority.declared_action is invalid")
    expected_action = expected_authority_action(authority)
    if declared_action in AUTHORITY_ACTIONS and declared_action != expected_action:
        errors.append(
            f"authority.declared_action must be {expected_action} for the declared facts"
        )

    if declared_action == "REUSE":
        if accepted_in_base != "YES":
            errors.append("REUSE requires authority_accepted_in_base=YES")
        if route_stage == "AUTHORITY_AUTHORING":
            errors.append("REUSE is not an authority-authoring route")
    elif declared_action in {
        "AMEND",
        "NEW",
        "SUPERSEDE",
        "AMEND_OR_NEW_PENDING_OWNERSHIP",
    }:
        if accepted_in_base != "NO":
            errors.append(
                f"{declared_action} requires authority_accepted_in_base=NO for the requested change"
            )

    if authority.get("ownership_known") is False:
        for field in (
            "implementation_allowed",
            "merge_ready",
            "operation_allowed",
        ):
            if readiness.get(field) == "YES":
                errors.append(
                    f"readiness.{field} cannot be YES while authority ownership is pending"
                )

    implementation_authority = authority.get("implementation_authority")
    if implementation_authority not in {
        "contracts",
        "none",
        "unknown",
        "not_applicable",
    }:
        errors.append("authority.implementation_authority is invalid")
    if (
        declared_action == "REUSE"
        and readiness.get("implementation_allowed") == "YES"
        and implementation_authority != "contracts"
    ):
        errors.append("REUSE implementation requires implementation_authority=contracts")
    if (
        implementation_authority == "none"
        and readiness.get("implementation_allowed") == "YES"
    ):
        errors.append("implementation_authority=none cannot permit implementation")

    complexity = plan.get("complexity")
    expected_plan = expected_plan_level(complexity)
    if expected_plan is None:
        errors.append("plan.complexity is invalid")
    if plan.get("level") not in PLAN_LEVELS:
        errors.append("plan.level is invalid")
    elif expected_plan is not None and plan.get("level") != expected_plan:
        errors.append(f"plan.level must be {expected_plan} for complexity={complexity}")

    consequence = assurance.get("failure_consequence")
    expected_assurance = expected_assurance_level(consequence)
    if expected_assurance is None:
        errors.append("assurance.failure_consequence is invalid")
    if assurance.get("level") not in ASSURANCE_LEVELS:
        errors.append("assurance.level is invalid")
    elif (
        expected_assurance is not None
        and assurance.get("level") != expected_assurance
    ):
        errors.append("assurance.level must match the declared failure consequence")

    for field in ("implementation_allowed", "merge_ready", "operation_allowed"):
        if readiness.get(field) not in READINESS:
            errors.append(f"readiness.{field} is invalid")

    implementation_allowed = readiness.get("implementation_allowed")
    operation_allowed = readiness.get("operation_allowed")
    mutation_allowed = implementation_allowed == "YES" or operation_allowed == "YES"

    if route_stage == "AUTHORITY_AUTHORING" and mutation_allowed:
        errors.append(
            "authority-authoring stage cannot permit implementation or operation"
        )

    if declared_action == "SUPERSEDE":
        if route_stage != "AUTHORITY_AUTHORING":
            errors.append(
                "SUPERSEDE is docs-first and must remain in authority authoring"
            )
        if mutation_allowed:
            errors.append(
                "SUPERSEDE cannot permit same-stage implementation or operation"
            )
        if atomic_spec_implementation_permitted is True:
            errors.append(
                "SUPERSEDE cannot use atomic Spec-and-implementation permission"
            )

    if declared_action in {"AMEND", "NEW"}:
        if assurance.get("level") == "CONTROLLED":
            if route_stage != "AUTHORITY_AUTHORING":
                errors.append(
                    "AMEND/NEW + CONTROLLED is docs-first; post-acceptance execution must re-route as REUSE"
                )
            if mutation_allowed:
                errors.append(
                    "AMEND/NEW + CONTROLLED cannot permit implementation or operation before authority acceptance"
                )
            if atomic_spec_implementation_permitted is True:
                errors.append(
                    "AMEND/NEW + CONTROLLED cannot use atomic Spec-and-implementation permission"
                )
        elif route_stage == "IMPLEMENTATION":
            if atomic_spec_implementation_permitted is not True:
                errors.append(
                    "AMEND/NEW + ROUTINE/DURABLE implementation requires explicit local atomic Spec-and-implementation permission"
                )
            if operation_allowed == "YES":
                errors.append(
                    "atomic Spec-and-implementation route cannot authorize a separate operation"
                )
        elif route_stage == "OPERATION":
            errors.append(
                "AMEND/NEW operation cannot precede authority acceptance; post-acceptance work must re-route as REUSE"
            )

    if declared_action in {"REUSE", "AMEND_OR_NEW_PENDING_OWNERSHIP"}:
        if atomic_spec_implementation_permitted is True:
            errors.append(
                f"{declared_action} cannot use atomic Spec-and-implementation permission"
            )

    mutation_planned = write_surface.get("mutation_planned")
    isolated = write_surface.get("isolated")
    if not isinstance(mutation_planned, bool):
        errors.append("write_surface.mutation_planned must be boolean")
    if not isinstance(isolated, bool):
        errors.append("write_surface.isolated must be boolean")
    if mutation_allowed and mutation_planned is not True:
        errors.append(
            "allowed implementation or operation requires write_surface.mutation_planned=true"
        )
    if mutation_planned is True:
        if isolated is not True:
            errors.append(
                "write work requires an isolated worktree or equivalent isolated write surface"
            )
        if not mandate_is_general_authorization(mandate):
            errors.append(
                "mutation requires a valid attributable Execution Mandate bound to target, scope, effects, and DONE_WHEN"
            )

    if assurance.get("level") == "CONTROLLED" and mutation_allowed:
        if assurance.get("controlled_runbook_present") is not True:
            errors.append("controlled mutation requires a Controlled Runbook")
        if not mandate_is_controlled(mandate):
            errors.append(
                "controlled mutation requires actor, environment, exact operation, abort, Secret, receipt, and attempt bounds"
            )

    if mandate.get("status") == "INVALID" and mutation_allowed:
        errors.append("invalid Execution Mandate cannot permit mutation")
    if mandate.get("self_issued") is True:
        errors.append("an acting Agent cannot self-issue its Execution Mandate")

    spec_gap = record.get("spec_gap_dependency")
    if spec_gap not in {"NONE", "NON_LOAD_BEARING", "LOAD_BEARING"}:
        errors.append("spec_gap_dependency is invalid")
    if spec_gap == "LOAD_BEARING":
        readiness_values = [
            readiness.get("implementation_allowed"),
            readiness.get("merge_ready"),
            readiness.get("operation_allowed"),
        ]
        for field in ("implementation_allowed", "merge_ready", "operation_allowed"):
            if readiness.get(field) == "YES":
                errors.append(
                    f"load-bearing SPEC_GAP requires readiness.{field}=NO or NOT_APPLICABLE"
                )
        if all(value == "NOT_APPLICABLE" for value in readiness_values):
            errors.append(
                "load-bearing SPEC_GAP must make at least one applicable readiness boundary explicitly NO"
            )
        if declared_action not in {"AMEND", "SUPERSEDE", "NEW"}:
            errors.append(
                "load-bearing SPEC_GAP requires AUTHORITY_ACTION=AMEND, SUPERSEDE, or NEW"
            )
        if stop.get("next_action") != "RE_PREFLIGHT":
            errors.append(
                "load-bearing SPEC_GAP requires next_action=RE_PREFLIGHT"
            )

    reviewability = evidence.get("reviewability")
    failure_class = evidence.get("failure_class")
    fabrication = evidence.get("fabrication_observed")
    if reviewability not in {"PASS", "FAIL", "NOT_APPLICABLE"}:
        errors.append("evidence.reviewability is invalid")
    if failure_class not in {
        "NONE",
        "REQUIRED_GATE_FAILURE",
        "FALSE_EVIDENCE",
    }:
        errors.append("evidence.failure_class is invalid")
    if evidence.get("load_bearing_required") is True and reviewability == "FAIL":
        if fabrication is True:
            if failure_class != "FALSE_EVIDENCE":
                errors.append(
                    "fabricated load-bearing Evidence requires FALSE_EVIDENCE"
                )
        elif failure_class != "REQUIRED_GATE_FAILURE":
            errors.append(
                "inaccessible load-bearing Evidence requires REQUIRED_GATE_FAILURE"
            )
        for field in ("implementation_allowed", "merge_ready", "operation_allowed"):
            if readiness.get(field) == "YES":
                errors.append(
                    f"failed required Evidence reviewability requires readiness.{field} != YES"
                )
    if failure_class == "FALSE_EVIDENCE" and fabrication is not True:
        errors.append(
            "FALSE_EVIDENCE requires observed fabrication/distortion/false execution claim"
        )

    live_state = live_gap.get("state")
    if live_state not in {"NONE", "DETECTED"}:
        errors.append("live_authority_gap.state is invalid")
    if live_state == "DETECTED":
        if live_gap.get("expansion_frozen") is not True:
            errors.append("live authority gap must freeze expansion")
        if live_gap.get("auto_delete") is not False:
            errors.append("live authority gap must not auto-delete")
        if live_gap.get("permanent_grandfather") is not False:
            errors.append("live authority gap must not permanently grandfather")
        if declared_action == "REUSE":
            errors.append("live authority gap cannot be REUSE")
        if owner_decision_required is not True:
            errors.append("live authority gap requires owner_decision_required=true")
        if (
            live_gap.get("owner_disposition_present") is not True
            and operation_allowed == "YES"
        ):
            errors.append(
                "live authority gap requires Owner disposition before operation"
            )

    emergency_state = emergency.get("state")
    emergency_action = emergency.get("action_kind")
    if emergency_state not in EMERGENCY_STATES:
        errors.append("emergency.state is invalid")
    if emergency_action not in EMERGENCY_ACTIONS:
        errors.append("emergency.action_kind is invalid")
    if emergency_state == "NONE":
        if emergency_action != "NONE":
            errors.append("non-active emergency must use action_kind=NONE")
    elif emergency_state == "ACTIVE":
        if route_stage != "OPERATION":
            errors.append("active emergency containment must use route_stage=OPERATION")
        if emergency_action == "NONE":
            errors.append("active emergency containment requires an allowed action kind")
        if emergency.get("owner_authorized") is not True:
            errors.append("emergency containment requires Owner authorization")
        if emergency.get("incident_reference_present") is not True:
            errors.append("emergency containment requires an incident reference")
        if emergency.get("durable_new_behavior") is not False:
            errors.append("emergency containment must not introduce durable new behavior")
        if emergency.get("normal_authority_reconciliation_required") is not True:
            errors.append(
                "emergency containment requires later normal authority reconciliation"
            )
        if implementation_allowed == "YES" or readiness.get("merge_ready") == "YES":
            errors.append(
                "emergency containment cannot authorize durable implementation or merge"
            )

    target_changed = review.get("target_head_changed")
    relevant_impact = review.get("relevant_base_impact")
    full_rereview = review.get("full_rereview_required")
    if not all(
        isinstance(value, bool)
        for value in (target_changed, relevant_impact, full_rereview)
    ):
        errors.append("review movement flags must be booleans")
    elif full_rereview != (target_changed or relevant_impact):
        errors.append(
            "review.full_rereview_required must reflect target or relevant Base impact"
        )

    if stop.get("next_action") not in NEXT_ACTIONS:
        errors.append("stop.next_action is invalid")
    if not isinstance(stop.get("done_when_met"), bool):
        errors.append("stop.done_when_met must be boolean")
    if not isinstance(stop.get("expansion_triggered"), bool):
        errors.append("stop.expansion_triggered must be boolean")
    if (
        stop.get("done_when_met") is True
        and stop.get("expansion_triggered") is False
        and stop.get("next_action") != "STOP"
    ):
        errors.append(
            "DONE_WHEN met without EXPANSION_TRIGGER requires next_action=STOP"
        )
    if (
        stop.get("expansion_triggered") is True
        and stop.get("next_action") not in {"RE_PREFLIGHT", "OWNER_DECISION"}
    ):
        errors.append(
            "triggered expansion requires RE_PREFLIGHT or OWNER_DECISION"
        )

    findings = record.get("findings", [])
    if not isinstance(findings, list):
        errors.append("findings must be an array")
    else:
        for index, value in enumerate(findings):
            finding = _mapping(value, f"findings[{index}]", errors)
            kind = finding.get("kind")
            if kind == "BLOCKER":
                if finding.get("blocker_class") not in BLOCKER_CLASSES:
                    errors.append(f"findings[{index}].blocker_class is invalid")
                if finding.get("source_type") not in LEGAL_SOURCE_TYPES:
                    errors.append(
                        f"findings[{index}].source_type is not a legal blocker source"
                    )
                for field in (
                    "source",
                    "counterexample",
                    "impact",
                    "minimal_closure",
                ):
                    item = finding.get(field)
                    if not isinstance(item, str) or not item.strip():
                        errors.append(
                            f"findings[{index}].{field} must be non-empty"
                        )
            elif kind in NON_BLOCKER_KINDS:
                if finding.get("blocker_class") not in {None, ""}:
                    errors.append(
                        f"findings[{index}] non-Blocker must not set blocker_class"
                    )
            else:
                errors.append(f"findings[{index}].kind is invalid")
    return errors


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("record", type=Path)
    args = parser.parse_args(argv)
    try:
        record = json.loads(args.record.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print(f"cannot read route record: {exc}", file=sys.stderr)
        return 2
    errors = validate_route(record)
    if errors:
        print("Governance V1 route validation failed:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1
    print("Governance V1 route is internally consistent")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
