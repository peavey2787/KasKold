"""Firmware model, adapter, ownership, and host-test contracts."""

from __future__ import annotations

from pathlib import Path
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[4]
CHECK_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(CHECK_DIR))

from source_complexity import (  # noqa: E402
    FORBIDDEN_EXCEPTIONS,
    function_decisions,
)


def _source(
    root: Path,
    relative: str,
    errors: list[str],
    label: str,
) -> str:
    path = root / relative
    if not path.is_file():
        errors.append(f"{label} is missing: {relative}")
        return ""
    return path.read_text(errors="replace")


def _check_decision_limit(
    source: str,
    relative: str,
    maximum: int,
    errors: list[str],
) -> list[int]:
    counts: list[int] = []
    for exception in FORBIDDEN_EXCEPTIONS:
        if exception in source:
            errors.append(f"complexity exception is forbidden: {relative}: {exception}")
    for record in function_decisions(source, relative):
        counts.append(record.decisions)
        if record.decisions > maximum:
            errors.append(
                f"firmware source exceeds its decision limit: "
                f"{relative}:{record.line} {record.name} "
                f"({record.decisions} > {maximum})"
            )
    return counts


def _limits_are_int(policy: dict[str, Any]) -> bool:
    return all(isinstance(policy.get(key), int) for key in (
        "maximum_model_source_decisions",
        "maximum_adapter_source_decisions",
        "maximum_board_adapter_source_decisions",
        "minimum_host_tests",
    ))


def _model_counts(
    root: Path,
    policy: dict[str, Any],
    errors: list[str],
) -> list[int]:
    counts: list[int] = []
    for relative in policy.get("model_paths", []):
        if not isinstance(relative, str):
            errors.append("firmware model path is invalid")
            continue
        source = _source(root, relative, errors, "firmware model")
        counts.extend(_check_decision_limit(
            source, relative, policy["maximum_model_source_decisions"], errors,
        ))
    return counts


def _board_adapter_counts(
    root: Path,
    relative_root: str,
    maximum: int,
    errors: list[str],
) -> list[int]:
    counts: list[int] = []
    adapter_root = root / relative_root
    if not adapter_root.is_dir():
        errors.append(f"firmware adapter root is missing: {relative_root}")
        return counts
    for path in adapter_root.rglob("*.rs"):
        if "unit_tests" in path.parts:
            continue
        relative = path.relative_to(root).as_posix()
        source = path.read_text(errors="replace")
        counts.extend(_check_decision_limit(source, relative, maximum, errors))
    return counts


def _adapter_root_counts(
    root: Path,
    policy: dict[str, Any],
    errors: list[str],
) -> list[int]:
    counts: list[int] = []
    for relative_root in policy.get("adapter_roots", []):
        if not isinstance(relative_root, str):
            errors.append("firmware adapter root is invalid")
            continue
        counts.extend(_board_adapter_counts(
            root, relative_root, policy["maximum_board_adapter_source_decisions"], errors,
        ))
    return counts


def _target_maximum(
    target: dict[str, Any],
    adapter_maximum: int,
    relative: str,
    name: str,
    errors: list[str],
) -> int:
    target_maximum = target.get("maximum_source_decisions", adapter_maximum)
    if not isinstance(target_maximum, int):
        errors.append(f"firmware adapter decision limit is invalid: {relative}::{name}")
        return adapter_maximum
    return target_maximum


def _target_record_counts(
    source: str,
    relative: str,
    name: str,
    target_maximum: int,
    errors: list[str],
) -> list[int]:
    records = [
        record for record in function_decisions(source, relative)
        if record.name == name
    ]
    if len(records) != 1:
        errors.append(
            f"firmware adapter must have one implementation: {relative}::{name}"
        )
        return []
    record = records[0]
    if record.decisions > target_maximum:
        errors.append(
            "firmware adapter exceeds its decision limit: "
            f"{relative}::{name} "
            f"({record.decisions} > {target_maximum})"
        )
    return [record.decisions]


def _adapter_target_counts(
    root: Path,
    target: dict[str, Any],
    adapter_maximum: int,
    errors: list[str],
) -> list[int]:
    relative = target.get("path")
    name = target.get("function")
    fragments = target.get("required_fragments")
    if not isinstance(relative, str) or not isinstance(name, str):
        errors.append("firmware adapter identity is invalid")
        return []
    source = _source(root, relative, errors, "firmware adapter")
    target_maximum = _target_maximum(target, adapter_maximum, relative, name, errors)
    counts = _target_record_counts(source, relative, name, target_maximum, errors)
    if not isinstance(fragments, list) or any(
        not isinstance(fragment, str) or fragment not in source
        for fragment in fragments
    ):
        errors.append(f"firmware adapter contract is incomplete: {relative}::{name}")
    return counts


def _adapter_targets_counts(
    root: Path,
    policy: dict[str, Any],
    errors: list[str],
) -> list[int]:
    counts: list[int] = []
    for target in policy.get("adapter_targets", []):
        if not isinstance(target, dict):
            errors.append("firmware adapter target is invalid")
            continue
        counts.extend(_adapter_target_counts(
            root, target, policy["maximum_adapter_source_decisions"], errors,
        ))
    return counts


def _host_test_count(
    root: Path,
    policy: dict[str, Any],
    errors: list[str],
) -> int:
    test_count = 0
    for relative in policy.get("test_paths", []):
        if not isinstance(relative, str):
            errors.append("host-test path is invalid")
            continue
        source = _source(root, relative, errors, "host-test source")
        test_count += source.count("#[test]")
    return test_count


def _ownership_errors(
    root: Path,
    policy: dict[str, Any],
    errors: list[str],
) -> None:
    for contract in policy.get("ownership_contracts", []):
        if not isinstance(contract, dict):
            errors.append("firmware ownership contract is invalid")
            continue
        relative = contract.get("path")
        fragments = contract.get("required_fragments")
        if not isinstance(relative, str) or not isinstance(fragments, list):
            errors.append("firmware ownership contract is invalid")
            continue
        source = _source(root, relative, errors, "firmware ownership source")
        missing = [
            fragment for fragment in fragments
            if not isinstance(fragment, str) or fragment not in source
        ]
        if missing:
            errors.append(
                f"firmware ownership contract is incomplete: {relative}: {missing}"
            )


def _forbidden_owner_errors(
    root: Path,
    policy: dict[str, Any],
    errors: list[str],
) -> None:
    for contract in policy.get("forbidden_owners", []):
        if not isinstance(contract, dict):
            errors.append("firmware forbidden-owner contract is invalid")
            continue
        relative = contract.get("path")
        fragments = contract.get("fragments")
        if not isinstance(relative, str) or not isinstance(fragments, list):
            errors.append("firmware forbidden-owner contract is invalid")
            continue
        source = _source(root, relative, errors, "firmware adapter source")
        present = [
            fragment for fragment in fragments
            if isinstance(fragment, str) and fragment in source
        ]
        if present:
            errors.append(
                f"shared firmware state owner was duplicated: {relative}: {present}"
            )


def check_firmware_policy(
    root: Path,
    policy: dict[str, Any],
) -> tuple[list[str], dict[str, int]]:
    if not _limits_are_int(policy):
        return ["firmware-testability limits are incomplete"], {}
    errors: list[str] = []
    model_counts = _model_counts(root, policy, errors)
    adapter_counts = _adapter_root_counts(root, policy, errors)
    adapter_counts.extend(_adapter_targets_counts(root, policy, errors))
    test_count = _host_test_count(root, policy, errors)
    minimum_tests = policy["minimum_host_tests"]
    if test_count < minimum_tests:
        errors.append(
            f"firmware host coverage regressed: {test_count} tests < {minimum_tests}"
        )
    _ownership_errors(root, policy, errors)
    _forbidden_owner_errors(root, policy, errors)
    return errors, {
        "host_tests": test_count,
        "maximum_model_decisions": max(model_counts, default=0),
        "maximum_adapter_decisions": max(adapter_counts, default=0),
    }
