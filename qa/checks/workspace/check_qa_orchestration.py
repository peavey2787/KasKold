#!/usr/bin/env python3
"""Validate the public QA catalog and explicit test/check entrypoint ownership."""
from __future__ import annotations
from collections.abc import Sequence

import json
from pathlib import Path
import re
import tomllib

ROOT = Path(__file__).resolve().parents[3]
CATALOG = ROOT / "qa/config/run_all_steps.tsv"
OWNERSHIP = ROOT / "qa/config/test_entrypoints.json"
TEST_PROFILE = ROOT / "qa/config/run_all_test_steps.txt"
ALLOWED_COMMANDS = {
    "make test",
    "make qa",
    "make test-hardware",
    "make workflow-e2e",
    "make workflow-hil",
    "make release",
    "make release-readiness",
    "make android-qa",
    "make ios-qa",
}


def catalog_rows() -> list[tuple[str, str, str, str, str]]:
    rows: list[tuple[str, str, str, str, str]] = []
    for raw in CATALOG.read_text(encoding="utf-8").splitlines():
        if not raw or raw.startswith("#"):
            continue
        fields = raw.split("\t", 4)
        if len(fields) != 5:
            raise ValueError(f"invalid QA catalog row: {raw!r}")
        rows.append(tuple(fields))  # type: ignore[arg-type]
    return rows


def _has_main_guard(path: Path) -> bool:
    text = path.read_text(errors="replace")
    return bool(re.search(r"if\s+__name__\s*==\s*['\"]__main__['\"]", text))


REQUIRED_STEPS = {
    "integration.real-node",
    "integration.funded-testnet-e2e",
    "emulation.signer-firmware-qemu",
    "bench.shared-signer-protocol-throughput",
    "mutation.repository-security-fresh",
    "mutation.repository-crypto-certification",
    "fuzz.repository-security-targets",
}
MOBILE_WORKSPACES = {"companion-ios", "companion-android"}
HARDWARE_COMMANDS = {"make test-hardware", "make workflow-e2e", "make workflow-hil"}


def _relative(path: Path) -> str:
    return path.relative_to(ROOT).as_posix()


def _shell_and_runner_entrypoints() -> set[str]:
    found: set[str] = set()
    for pattern in (
        "qa/linux/run-*.sh",
        "qa/windows/run-*.ps1",
        "qa/linux/release/*.sh",
        "qa/windows/release/*.ps1",
    ):
        found.update(_relative(path) for path in ROOT.glob(pattern) if path.is_file())
    found.add("qa/windows/runner/run_all.py")
    return found


def _check_script_entrypoints() -> set[str]:
    found: set[str] = set()
    for path in (ROOT / "qa/checks").rglob("*.py"):
        if _has_main_guard(path):
            found.add(_relative(path))
    for path in (ROOT / "qa/checks").rglob("*.mjs"):
        name = path.name
        if name.endswith(".test.mjs") or name.startswith("check_"):
            found.add(_relative(path))
    return found


def _python_test_entrypoints() -> set[str]:
    found: set[str] = set()
    found.update(_relative(path) for path in (ROOT / "qa/tests/tooling").rglob("test_*.py"))
    found.update(_relative(path) for path in (ROOT / "qa/tests/regression").rglob("test_*.py"))
    return found


def _cargo_entrypoints() -> set[str]:
    found: set[str] = set()
    qa_manifest = tomllib.loads((ROOT / "qa/Cargo.toml").read_text())
    for section in ("test", "bench"):
        for item in qa_manifest.get(section, []):
            found.add((Path("qa") / str(item["path"])).as_posix())
    fuzz_manifest = tomllib.loads((ROOT / "qa/fuzz/Cargo.toml").read_text())
    for item in fuzz_manifest.get("bin", []):
        found.add((Path("qa/fuzz") / str(item["path"])).as_posix())
    return found


def _mobile_test_entrypoints() -> set[str]:
    found: set[str] = set()
    for path in (ROOT / "apps/kaskold-companion-ios/Tests").rglob("*.swift"):
        found.add(_relative(path))
    android_app = ROOT / "apps/kaskold-companion-android/app/src"
    for branch in ("test", "androidTest"):
        found.update(_relative(path) for path in (android_app / branch).rglob("*.kt"))
    found.update(
        _relative(path) for path in (ROOT / "apps/kaskold-companion-android/portable-tests").rglob("*.kt")
    )
    return found


def discover_entrypoints() -> set[str]:
    return (
        _shell_and_runner_entrypoints()
        | _check_script_entrypoints()
        | _python_test_entrypoints()
        | _cargo_entrypoints()
        | _mobile_test_entrypoints()
    )


def _unique_id_errors(ids: list[str]) -> list[str]:
    if len(ids) != len(set(ids)):
        return ["QA catalog step IDs must be unique"]
    return []


def _block_order_errors(rows: Sequence[tuple[str, ...]]) -> list[str]:
    errors: list[str] = []
    scopes = [row[0] for row in rows]
    if not rows or rows[0][3] != "preflight.crap-check" or rows[0][0] != "qa":
        errors.append("preflight.crap-check must be the first full make-qa gate")
    if len(rows) < 2 or rows[1][3] != "preflight.core-ci" or rows[1][0] != "qa":
        errors.append("preflight.core-ci must run immediately after preflight.crap-check")
    test_positions = [i for i, scope in enumerate(scopes) if scope == "test"]
    if test_positions:
        first_test, last_test = min(test_positions), max(test_positions)
        if test_positions != list(range(first_test, last_test + 1)):
            errors.append("make-test catalog rows must form one contiguous block in make-qa")
        if first_test != 2:
            errors.append("make-test catalog block must immediately follow preflight.core-ci")
    return errors


def _unknown_scope_errors(rows: Sequence[tuple[str, ...]]) -> list[str]:
    if any(row[0] not in {"test", "qa", "hardware"} for row in rows):
        return ["QA catalog contains an unknown scope"]
    return []


def _row_errors(row: tuple[str, ...]) -> list[str]:
    scope, category, workspace, step_id, _description = row
    errors: list[str] = []
    if scope in {"test", "qa"} and category == "hardware":
        errors.append(f"physical hardware step leaked into make qa: {step_id}")
    if scope == "hardware" and category != "hardware":
        errors.append(f"hardware-only step must use hardware category: {step_id}")
    if scope in {"test", "qa"} and workspace in MOBILE_WORKSPACES:
        errors.append(f"mobile test leaked into public make {scope}: {step_id}")
    return errors


def _test_profile_errors(test_ids: list[str]) -> list[str]:
    configured_test = [
        line.strip() for line in TEST_PROFILE.read_text().splitlines()
        if line.strip() and not line.lstrip().startswith("#")
    ]
    if configured_test != test_ids:
        return ["run_all_test_steps.txt does not exactly match the catalog test block"]
    return []


def _required_step_errors(ids: list[str]) -> list[str]:
    missing_required = sorted(REQUIRED_STEPS - set(ids))
    if missing_required:
        return ["authoritative make-qa catalog is missing required steps: " + ", ".join(missing_required)]
    return []


def _ordering_errors(ids: list[str], test_ids: list[str]) -> list[str]:
    if not all(step in ids for step in REQUIRED_STEPS):
        return []
    errors: list[str] = []
    position = {step: ids.index(step) for step in ids}
    test_end = max(ids.index(step) for step in test_ids)
    if min(position["integration.real-node"], position["integration.funded-testnet-e2e"]) <= test_end:
        errors.append("real-node/funded E2E must run after the complete make-test prefix")
    if position["integration.real-node"] > position["mutation.repository-security-fresh"] or position["integration.funded-testnet-e2e"] > position["mutation.repository-security-fresh"]:
        errors.append("interactive real-node/funded E2E must run before fresh mutation")
    if not (position["mutation.repository-security-fresh"] < position["mutation.repository-crypto-certification"] < position["fuzz.repository-security-targets"]):
        errors.append("fresh mutation, crypto certification, and final fuzz ordering is incorrect")
    return errors


def _mobile_kind(path: str) -> str | None:
    name = Path(path).name
    if path.startswith(("qa/checks/ios/", "apps/kaskold-companion-ios/")) or name.startswith("test_ios_"):
        return "ios"
    if path.startswith(("qa/checks/android/", "apps/kaskold-companion-android/")) or name.startswith("test_android_"):
        return "android"
    if name.startswith("test_mobile_"):
        return "both"
    return None


def _mobile_entry_errors(path: str, commands: list[str]) -> list[str]:
    errors: list[str] = []
    mobile_kind = _mobile_kind(path)
    if not mobile_kind:
        return errors
    if {"make test", "make qa"} & set(commands):
        errors.append(f"mobile entrypoint leaked into shared make test/make qa: {path}")
    expected_mobile_commands = (
        ["make android-qa", "make ios-qa"]
        if mobile_kind == "both"
        else [f"make {mobile_kind}-qa"]
    )
    if commands != expected_mobile_commands:
        errors.append(
            f"mobile entrypoint {path} must be owned only by {expected_mobile_commands}: {commands!r}"
        )
    return errors


def _step_assignment_errors(
    path: str,
    entry: dict[str, object],
    commands: list[str],
    ids: list[str],
    rows: Sequence[tuple[str, ...]],
) -> list[str]:
    if "make test" not in commands and "make qa" not in commands:
        return []
    step = entry.get("step")
    role = str(entry.get("role", ""))
    if step == "catalog" and role in {"runner", "alias"}:
        return []
    if not isinstance(step, str) or step not in ids:
        return [f"entrypoint {path} is assigned to an unknown run-all step: {step!r}"]
    if "make test" in commands and dict((row[3], row[0]) for row in rows).get(step) != "test":
        return [f"entrypoint {path} claims make test but step {step} is not in the test prefix"]
    return []


def _entry_errors(
    path: str,
    entry: dict[str, object],
    ids: list[str],
    rows: Sequence[tuple[str, ...]],
) -> list[str]:
    errors: list[str] = []
    commands = entry.get("commands")
    if not isinstance(commands, list) or not commands or any(command not in ALLOWED_COMMANDS for command in commands):
        errors.append(f"entrypoint {path} has invalid public command classification: {commands!r}")
        commands = []
    if "make test" in commands and "make qa" not in commands:
        errors.append(f"entrypoint {path} is in make test but missing from authoritative make qa")
    errors.extend(_mobile_entry_errors(path, commands))
    if "make qa" in commands and HARDWARE_COMMANDS.intersection(commands):
        errors.append(f"entrypoint {path} mixes non-hardware make qa with physical/HIL ownership")
    if str(entry.get("role", "")) == "hardware-runner" and ({"make test", "make qa"} & set(commands)):
        errors.append(f"physical/HIL entrypoint leaked into non-hardware public QA: {path}")
    errors.extend(_step_assignment_errors(path, entry, commands, ids, rows))
    return errors


def _load_owned_entries(
    ids: list[str],
    rows: Sequence[tuple[str, ...]],
) -> tuple[dict[str, dict[str, object]], list[str]]:
    document = json.loads(OWNERSHIP.read_text(encoding="utf-8"))
    owned: dict[str, dict[str, object]] = {}
    errors: list[str] = []
    for entry in document.get("entrypoints", []):
        path = str(entry.get("path", ""))
        if not path or path in owned:
            errors.append(f"duplicate/empty entrypoint ownership record: {path!r}")
            continue
        owned[path] = entry
        errors.extend(_entry_errors(path, entry, ids, rows))
    return owned, errors


def _discovery_errors(owned_paths: set[str]) -> list[str]:
    discovered = discover_entrypoints()
    errors: list[str] = []
    missing = sorted(discovered - owned_paths)
    stale = sorted(owned_paths - discovered)
    if missing:
        errors.append("orphaned registered test/check entrypoints: " + ", ".join(missing))
    if stale:
        errors.append("stale test/check ownership records: " + ", ".join(stale))
    return errors


def check(root: Path = ROOT) -> list[str]:
    del root  # paths are repository-global by design
    try:
        rows = catalog_rows()
    except (OSError, ValueError) as error:
        return [str(error)]
    ids = [row[3] for row in rows]
    test_ids = [row[3] for row in rows if row[0] == "test"]
    errors = _unique_id_errors(ids)
    errors.extend(_block_order_errors(rows))
    errors.extend(_unknown_scope_errors(rows))
    for row in rows:
        errors.extend(_row_errors(row))
    errors.extend(_test_profile_errors(test_ids))
    errors.extend(_required_step_errors(ids))
    errors.extend(_ordering_errors(ids, test_ids))
    owned, ownership_errors = _load_owned_entries(ids, rows)
    errors.extend(ownership_errors)
    errors.extend(_discovery_errors(set(owned)))
    return errors


def main() -> int:
    errors = check()
    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1
    print(f"PASS: QA orchestration catalog owns {len(discover_entrypoints())} registered test/check entrypoints")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
