#!/usr/bin/env python3
"""Clean, pinned Lean build plus source and compiled-environment axiom audits.

The audit harness is adapted from mayf3/primitives tools/verify_lean.py at
9500d8e24730fdbce334b8fdcde71830838c21f4. Proof modules cannot execute the
audit metaprogramming. This verifier is reviewed tooling, not a sandbox.
"""
from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent
REPOSITORY = ROOT.parents[1]
VERSION = "4.19.0"
ALLOWED_AXIOMS = {"propext", "Quot.sound", "Classical.choice"}
PREFIX = "WAKE_COMPILED_AUDIT="


def hashes(paths: list[Path], root: Path) -> dict[str, str]:
    return {str(path.relative_to(root)): hashlib.sha256(path.read_bytes()).hexdigest()
            for path in paths}


def without_comments(source: str) -> str:
    """Strip nested Lean comments before the limited source-policy checks."""
    depth = 0
    string = False
    escaped = False
    result: list[str] = []
    position = 0
    while position < len(source):
        pair = source[position:position + 2]
        char = source[position]
        if depth:
            if pair == "/-":
                depth += 1
                result.extend("  ")
                position += 2
            elif pair == "-/":
                depth -= 1
                result.extend("  ")
                position += 2
            else:
                result.append("\n" if char == "\n" else " ")
                position += 1
        elif string:
            result.append(char)
            if escaped:
                escaped = False
            elif char == "\\":
                escaped = True
            elif char == '"':
                string = False
            position += 1
        elif pair == "/-":
            depth = 1
            result.extend("  ")
            position += 2
        elif pair == "--":
            end = source.find("\n", position)
            if end == -1:
                end = len(source)
            result.extend(" " * (end - position))
            position = end
        else:
            result.append(char)
            string = char == '"'
            position += 1
    if depth or string:
        raise ValueError("Unterminated comment or string in model source")
    return "".join(result)


def read_layout() -> tuple[dict, dict[str, list[str]], list[Path]]:
    manifest = json.loads((ROOT / "model-manifest.json").read_text())
    if manifest.get("schema") != "svc-workflow-wake-models/v1":
        raise ValueError("Unexpected model manifest schema")
    if manifest.get("toolchain") != "leanprover/lean4:v4.19.0":
        raise ValueError("Manifest toolchain differs from pinned verifier")
    if (ROOT / "lean-toolchain").read_text().strip() != manifest["toolchain"]:
        raise ValueError("Toolchain file differs from manifest")
    models = manifest["models"]
    files = [ROOT / f"{model['module']}.lean" for model in models]
    if len(set(files)) != len(files) or set(ROOT.glob("*.lean")) != set(files):
        raise ValueError("Missing, duplicate or unregistered Lean source")
    inventory: dict[str, list[str]] = {}
    seen_modules: set[str] = set()
    for model, path in zip(models, files):
        module = model["module"]
        namespace = model["namespace"]
        source = without_comments(path.read_text())
        if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_]*", module):
            raise ValueError("Unsupported module identifier")
        if namespace != "SvcWorkflow." + module:
            raise ValueError("Unexpected namespace")
        imports = re.findall(r"^import\s+(.+)$", source, re.M)
        if imports != model["imports"] or any(i != "Std" and i not in seen_modules for i in imports):
            raise ValueError(f"Unexpected import or module build order in {module}")
        if re.findall(r"^namespace\s+(\S+)\s*$", source, re.M) != [namespace]:
            raise ValueError(f"Each module must have its one declared namespace: {module}")
        if re.search(r"\b(sorry|admit|axiom|native_decide|unsafe|partial|run_tac|run_elab|implemented_by)\b", source):
            raise ValueError(f"Disallowed proof-source token in {module}")
        if re.search(r"^\s*(macro|elab|syntax|initialize|opaque|private|protected|attribute)\b", source, re.M):
            raise ValueError(f"Unsupported declaration form in {module}")
        names = re.findall(r"^theorem\s+([A-Za-z][A-Za-z0-9_]*)\b", source, re.M)
        if not names or len(names) != len(set(names)):
            raise ValueError(f"Missing or duplicate named theorem in {module}")
        inventory[module] = [namespace + "." + name for name in names]
        seen_modules.add(module)
    files += [ROOT / "model-manifest.json", ROOT / "lean-toolchain", ROOT / "verify.py", ROOT / "README.md"]
    return manifest, inventory, files


def audit_source(modules: list[str], names: list[str]) -> str:
    header = ["import Lean", *(f"import {module}" for module in modules),
              "set_option warningAsError true", *(f"#print axioms {name}" for name in names)]
    compiled = '''
open Lean in
run_elab do
  let env ← getEnv
  let modules : List String := MODULE_LIST
  let present := modules.filter fun module =>
    env.header.moduleNames.any fun name => name.toString == module
  let mut declarations : Array Json := #[]
  let mut unsafeNames : Array Json := #[]
  let mut partialNames : Array Json := #[]
  for (name, info) in env.constants do
    if let some index := env.getModuleIdxFor? name then
      let owner := env.header.moduleNames[index]!
      if modules.contains owner.toString then
        if info.isUnsafe || info.isPartial then
          if info.isTheorem then
            throwError "Cannot exclude a theorem from the audit: {name}"
          if info.isUnsafe then
            unsafeNames := unsafeNames.push (Json.str name.toString)
          else
            partialNames := partialNames.push (Json.str name.toString)
          continue
        let kind := match info with
          | .axiomInfo _ => "axiom"
          | .thmInfo _ => "theorem"
          | .defnInfo _ => "definition"
          | .opaqueInfo _ => "opaque"
          | .quotInfo _ => "quotient"
          | .inductInfo _ => "inductive"
          | .ctorInfo _ => "constructor"
          | .recInfo _ => "recursor"
        let axioms ← collectAxioms name
        declarations := declarations.push (Json.mkObj [
          ("name", Json.str name.toString),
          ("module", Json.str owner.toString),
          ("kind", Json.str kind),
          ("axioms", Json.arr (axioms.map fun axiomName => Json.str axiomName.toString))])
  IO.println ("AUDIT_PREFIX" ++ (Json.mkObj [
    ("schema", Json.str "svc-workflow-wake-compiled-audit/v1"),
    ("modules", Json.arr (present.toArray.map Json.str)),
    ("excluded_unsafe", Json.arr unsafeNames),
    ("excluded_partial", Json.arr partialNames),
    ("declarations", Json.arr declarations)]).compress)
'''
    return "\n".join(header) + "\n" + compiled.replace(
        "MODULE_LIST", json.dumps(modules)).replace("AUDIT_PREFIX", PREFIX)


def parse_audit(output: str, modules: list[str], names: list[str]) -> tuple[dict, dict]:
    pattern = r"'([^'\n]+)' (?:does not depend on any axioms|depends on axioms:\s*\[([^\]]*)\])"
    matches = re.findall(pattern, output)
    named = {name: sorted(a.strip() for a in listed.split(",") if a.strip()) for name, listed in matches}
    if len(matches) != len(names) or set(named) != set(names):
        raise ValueError("Missing, duplicate or unexpected named theorem audit")
    lines = [line[len(PREFIX):] for line in output.splitlines() if line.startswith(PREFIX)]
    if len(lines) != 1:
        raise ValueError("Missing or duplicate compiled audit")
    compiled = json.loads(lines[0])
    if (compiled.get("schema") != "svc-workflow-wake-compiled-audit/v1"
            or set(compiled.get("modules", [])) != set(modules)
            or len(compiled.get("modules", [])) != len(modules)):
        raise ValueError("Incomplete compiled module inventory")
    declarations = {}
    for record in compiled["declarations"]:
        if (set(record) != {"name", "module", "kind", "axioms"}
                or record["name"] in declarations or record["module"] not in modules):
            raise ValueError("Malformed or duplicate compiled declaration")
        if record["kind"] == "axiom" or set(record["axioms"]) - ALLOWED_AXIOMS:
            raise ValueError(f"Unapproved transitive axiom: {record}")
        declarations[record["name"]] = record
    for name, dependencies in named.items():
        record = declarations.get(name)
        if (record is None or record["kind"] != "theorem"
                or sorted(record["axioms"]) != dependencies):
            raise ValueError(f"Source and compiled theorem audits disagree: {name}")
    seen = set(declarations)
    for category in ("excluded_unsafe", "excluded_partial"):
        excluded = compiled.get(category)
        if (not isinstance(excluded, list) or any(not isinstance(name, str) for name in excluded)
                or len(set(excluded)) != len(excluded) or seen.intersection(excluded)):
            raise ValueError(f"Invalid compiler helper exclusions: {category}")
        seen.update(excluded)
    compiled["declarations"] = sorted(declarations.values(), key=lambda entry: entry["name"])
    return dict(sorted(named.items())), compiled


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--lean", default="lean", help="Pinned Lean 4.19.0 executable")
    parser.add_argument("--out", type=Path, default=ROOT / "evidence")
    args = parser.parse_args()
    manifest, inventory, source_paths = read_layout()
    context_paths = [REPOSITORY / path for path in manifest["implementation_context"]]
    source_before = hashes(source_paths, ROOT)
    context_before = hashes(context_paths, REPOSITORY)
    lean = shutil.which(args.lean)
    if lean is None:
        raise ValueError(f"Lean executable not found: {args.lean}")
    lean = str(Path(lean).resolve())
    version = subprocess.run([lean, "--version"], check=True, capture_output=True, text=True).stdout.strip()
    short_version = subprocess.run([lean, "--short-version"], check=True, capture_output=True, text=True).stdout.strip()
    if short_version != VERSION:
        raise ValueError(f"Expected Lean {VERSION}; observed {version}")
    names = [name for values in inventory.values() for name in values]
    modules = list(inventory)
    audit = audit_source(modules, names)
    commands = []
    with tempfile.TemporaryDirectory(prefix="svc-workflow-wake-") as temporary:
        work = Path(temporary)
        for module in modules:
            shutil.copy2(ROOT / f"{module}.lean", work / f"{module}.lean")
        (work / "AxiomAudit.lean").write_text(audit)
        env = dict(os.environ)
        for key in ("LEAN_PATH", "LEAN_SRC_PATH", "LEAN_SYSROOT"):
            env.pop(key, None)
        env["LEAN_PATH"] = str(work)
        arguments = [["-DwarningAsError=true", "-o", f"{module}.olean", f"{module}.lean"]
                     for module in modules] + [["-DwarningAsError=true", "AxiomAudit.lean"]]
        for tail in arguments:
            command = [lean, *tail]
            result = subprocess.run(command, cwd=work, env=env, capture_output=True, text=True, timeout=180)
            commands.append({"command": command, "exit_code": result.returncode,
                             "stdout": result.stdout, "stderr": result.stderr})
            if result.returncode:
                raise RuntimeError(f"Command failed: {command}\n{result.stdout}\n{result.stderr}")
    named, compiled = parse_audit(commands[-1]["stdout"], modules, names)
    if source_before != hashes(source_paths, ROOT) or context_before != hashes(context_paths, REPOSITORY):
        raise ValueError("Proof or implementation-context inputs changed during verification")
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=REPOSITORY,
                          check=True, capture_output=True, text=True).stdout.strip()
    status = subprocess.run(["git", "status", "--short"], cwd=REPOSITORY,
                            check=True, capture_output=True, text=True).stdout
    receipt = {
        "status": "PASS_CONDITIONAL_MODEL_PROOFS",
        "created_at": dt.datetime.now(dt.timezone.utc).isoformat(),
        "source_repository": "https://github.com/mayf3/svc-workflow",
        "repository_head_at_run": head,
        "working_tree_dirty_at_run": bool(status),
        "historical_counterexample_base": "c50ab2ac98decceaa26138a282c34dd92ede0ddc",
        "authority": ["SVC_WORKFLOW_VISIT_ACTIVATION_IMPL_V3 / CTR-VAI-008 / CTR-VAI-014",
                      "POSTGRES_STORAGE_CONTRACT_V0_3"],
        "lean_version": version,
        "clean_build": True,
        "source_hashes_frozen_before_and_after": True,
        "theorem_count": len(names),
        "modules": {module: len(values) for module, values in inventory.items()},
        "compiled_declarations_audited": len(compiled["declarations"]),
        "compiled_theorems_audited": sum(r["kind"] == "theorem" for r in compiled["declarations"]),
        "axioms": named,
        "compiled_audit": compiled,
        "source_sha256": source_before,
        "implementation_context_sha256": context_before,
        "generated_audit_sha256": hashlib.sha256(audit.encode()).hexdigest(),
        "commands": commands,
        "limits": [
            "Conditional model proofs, not a Lean refinement of Rust, SQLx or PostgreSQL.",
            "Implementation hashes bind the reviewed context; hashing does not prove implementation conformance.",
            "WakeReceipt.run assumes one atomic serialized transaction after valid instance/activation resolution.",
            "WakeDurability.settle describes possible durable outcomes, not a proof of commit, rollback or crash behavior.",
            "Matching request hash must bind original cause; hash collision resistance and canonicalization are unproved.",
            "Authorization, fresh rejection classification and full JSON parsing remain implementation obligations.",
            "No production task completion, deployment or liveness result is asserted.",
        ],
    }
    args.out.mkdir(parents=True, exist_ok=True)
    receipt_path = args.out / "verification.json"
    receipt_path.write_text(json.dumps(receipt, ensure_ascii=False, indent=2) + "\n")
    (args.out / "axioms.txt").write_text(commands[-1]["stdout"])
    print(json.dumps({"status": receipt["status"], "theorem_count": len(names),
                      "modules": receipt["modules"],
                      "compiled_declarations_audited": receipt["compiled_declarations_audited"],
                      "compiled_theorems_audited": receipt["compiled_theorems_audited"],
                      "axioms": sorted({a for values in named.values() for a in values}),
                      "receipt": str(receipt_path)}))


if __name__ == "__main__":
    main()
