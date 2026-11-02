"""Source-level complexity contracts for first-party production Rust."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
import re
import sys
from typing import Any

ROOT = Path(__file__).resolve().parents[4]
CHECK_DIR = Path(__file__).resolve().parent
sys.path.insert(0, str(CHECK_DIR))
sys.path.insert(0, str(ROOT / "qa/checks"))

from architecture.core.common import (  # noqa: E402
    _rust_body_closing,
    _rust_body_opening,
    rust_code_only,
)
from report import classify_path  # noqa: E402

FUNCTION_PATTERN = re.compile(
    r"(?m)^[ \t]*(?:pub(?:\([^)]*\))?\s+)?(?:const\s+)?(?:async\s+)?"
    r"(?:unsafe\s+)?(?:extern\s+\"[^\"]+\"\s+)?fn\s+"
    r"([A-Za-z_][A-Za-z0-9_]*)"
)
DECISION_PATTERNS = tuple(
    re.compile(rf"\b{keyword}\b") for keyword in ("if", "for", "while", "loop")
)
FORBIDDEN_EXCEPTIONS = (
    "allow(clippy::cognitive_complexity)",
    "allow(clippy::too_many_lines)",
)
PRODUCTION_ROOTS = (
    "apps/kaskold-companion-web/src",
    "apps/kaskold-vault-web/src",
    "apps/kaskold-hardware/src",
    "crates/kaskold-protocol/src",
    "crates/kaskold-sdk/src",
    "crates/offline-signer/src",
    "crates/online-watcher/src",
    "crates/shared-signer/src",
    "crates/kaskold-hardware-core/src",
)


@dataclass(frozen=True)
class FunctionDecisionCount:
    path: str
    name: str
    line: int
    decisions: int


def source_decisions(body: str) -> int:
    """Return a conservative source-level decision count for one Rust function."""
    code = rust_code_only(body)
    return (
        1
        + sum(len(pattern.findall(code)) for pattern in DECISION_PATTERNS)
        + code.count("&&")
        + code.count("||")
        + code.count("?")
        + len(re.findall(r"=>", code))
    )


def function_decisions(source: str, path: str = "") -> list[FunctionDecisionCount]:
    records: list[FunctionDecisionCount] = []
    for match in FUNCTION_PATTERN.finditer(source):
        opening = _rust_body_opening(source, match.end())
        if opening is None:
            continue
        closing = _rust_body_closing(source, opening)
        if closing is None:
            continue
        records.append(
            FunctionDecisionCount(
                path=path,
                name=match.group(1),
                line=source.count("\n", 0, match.start()) + 1,
                decisions=source_decisions(source[match.start() : closing + 1]),
            )
        )
    return records


def production_records(root: Path) -> list[FunctionDecisionCount]:
    records: list[FunctionDecisionCount] = []
    for relative_root in PRODUCTION_ROOTS:
        for path in (root / relative_root).rglob("*.rs"):
            relative = path.relative_to(root).as_posix()
            if classify_path(relative) != "production":
                continue
            source = path.read_text(errors="replace")
            records.extend(function_decisions(source, relative))
    return records


def _monitored_records(
    root: Path,
    relative: str,
    errors: list[str],
) -> tuple[str, list[FunctionDecisionCount]]:
    path = root / relative
    if not path.is_file():
        errors.append(f"monitored source is missing: {relative}")
        return "", []
    source = path.read_text(errors="replace")
    for exception in FORBIDDEN_EXCEPTIONS:
        if exception in source:
            errors.append(f"complexity exception is forbidden: {relative}: {exception}")
    return source, function_decisions(source, relative)


def _records_over(
    records: list[FunctionDecisionCount],
    level: int,
) -> list[FunctionDecisionCount]:
    return [record for record in records if record.decisions > level]


def _production_errors(
    records: list[FunctionDecisionCount],
    maximum: int,
    maximum_warnings: int,
    warnings: list[FunctionDecisionCount],
) -> list[str]:
    errors: list[str] = []
    for record in _records_over(records, maximum):
        errors.append(
            "production function exceeds the source-decision limit: "
            f"{record.path}:{record.line} {record.name} "
            f"({record.decisions} > {maximum})"
        )
    if len(warnings) > maximum_warnings:
        errors.append(
            "production source-decision warning count regressed: "
            f"{len(warnings)} > {maximum_warnings}"
        )
    return errors


def _helper_limit_errors(
    relative: str,
    records: list[FunctionDecisionCount],
    monitored_maximum: int,
) -> list[str]:
    return [
        "monitored helper exceeds the source-decision limit: "
        f"{relative}:{record.line} {record.name} "
        f"({record.decisions} > {monitored_maximum})"
        for record in _records_over(records, monitored_maximum)
    ]


def _monitored_errors(
    root: Path,
    monitored_paths: list,
    monitored_maximum: int,
    errors: list[str],
) -> tuple[dict[str, str], dict[str, list[FunctionDecisionCount]]]:
    records_by_path: dict[str, list[FunctionDecisionCount]] = {}
    source_by_path: dict[str, str] = {}
    for relative in monitored_paths:
        if not isinstance(relative, str):
            errors.append("monitored source path is invalid")
            continue
        source, path_records = _monitored_records(root, relative, errors)
        source_by_path[relative] = source
        records_by_path[relative] = path_records
        errors.extend(_helper_limit_errors(relative, path_records, monitored_maximum))
    return source_by_path, records_by_path


def _unique_record(
    records: list[FunctionDecisionCount],
    name: str,
) -> FunctionDecisionCount | None:
    matches = [record for record in records if record.name == name]
    return matches[0] if len(matches) == 1 else None


def _target_record_errors(
    target: dict,
    record: FunctionDecisionCount,
    relative: str,
    name: str,
    source: str,
) -> list[str]:
    errors: list[str] = []
    previous = target.get("previous_complexity")
    fragments = target.get("required_fragments")
    if not isinstance(previous, int) or record.decisions >= previous:
        errors.append(
            "decomposition target did not remain below its original complexity: "
            f"{relative}::{name} ({record.decisions} vs original CC {previous})"
        )
    if not isinstance(fragments, list) or any(
        not isinstance(fragment, str) or fragment not in source
        for fragment in fragments
    ):
        errors.append(f"decomposition contract is incomplete: {relative}::{name}")
    return errors


def _target_errors(
    targets: list,
    records_by_path: dict[str, list[FunctionDecisionCount]],
    source_by_path: dict[str, str],
    errors: list[str],
) -> list[int]:
    target_counts: list[int] = []
    for target in targets:
        if not isinstance(target, dict):
            errors.append("decomposition target is invalid")
            continue
        relative = target.get("path")
        name = target.get("function")
        if not isinstance(relative, str) or not isinstance(name, str):
            errors.append("decomposition target identity is invalid")
            continue
        record = _unique_record(records_by_path.get(relative, []), name)
        if record is None:
            errors.append(
                f"decomposition target must have one implementation: {relative}::{name}"
            )
            continue
        target_counts.append(record.decisions)
        errors.extend(
            _target_record_errors(
                target,
                record,
                relative,
                name,
                source_by_path.get(relative, ""),
            )
        )
    return target_counts


def check_source_policy(
    root: Path,
    policy: dict[str, Any],
) -> tuple[list[str], dict[str, int]]:
    maximum = policy.get("maximum_production_source_decisions")
    warning_level = policy.get("warning_source_decisions")
    maximum_warnings = policy.get("maximum_warning_functions")
    monitored_maximum = policy.get("maximum_monitored_source_decisions")
    targets = policy.get("targets")
    monitored_paths = policy.get("monitored_paths")
    if (
        not isinstance(maximum, int)
        or not isinstance(warning_level, int)
        or not isinstance(maximum_warnings, int)
        or not isinstance(monitored_maximum, int)
    ):
        return ["source-complexity limits are incomplete"], {}
    if not isinstance(targets, list) or not isinstance(monitored_paths, list):
        return ["source-complexity contracts are incomplete"], {}

    records = production_records(root)
    warnings = _records_over(records, warning_level)
    errors = _production_errors(records, maximum, maximum_warnings, warnings)
    source_by_path, records_by_path = _monitored_errors(
        root,
        monitored_paths,
        monitored_maximum,
        errors,
    )
    target_counts = _target_errors(targets, records_by_path, source_by_path, errors)
    return errors, {
        "production_functions": len(records),
        "warning_functions": len(warnings),
        "maximum_decisions": max((record.decisions for record in records), default=0),
        "maximum_target_decisions": max(target_counts, default=0),
    }
