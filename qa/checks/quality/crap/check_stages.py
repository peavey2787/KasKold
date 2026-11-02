"""Contract, report, and manifest stages for the consolidated CRAP gate."""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

from firmware_testability import check_firmware_policy
from health import audit_health, parse_lcov
from regression import (
    compare_coverage_manifests,
    compare_health_summaries,
    compare_reports,
)
from report import (
    apply_coverage_unavailable_source_policy,
    baseline_document,
    classify_path,
    parse_report,
)
from source_complexity import check_source_policy


def load_json(path: Path, label: str) -> dict[str, Any]:
    try:
        document = json.loads(path.read_text())
    except (OSError, json.JSONDecodeError) as error:
        raise ValueError(f"{label} cannot be read: {path}: {error}") from error
    if not isinstance(document, dict):
        raise ValueError(f"{label} must be a JSON object: {path}")
    return document


REFERENCE_STATUSES = ("fail", "warning", "pass")


def _reference_entry_error(
    entry: Any,
    identities: set[tuple[str, int | None, str]],
    counts: dict[str, dict[str, int]],
) -> str | None:
    if not isinstance(entry, dict):
        return "reference report contains an invalid function record"
    path = entry.get("path")
    scope = entry.get("scope")
    status = entry.get("status")
    function = entry.get("function")
    line = entry.get("line")
    if not isinstance(path, str) or not isinstance(function, str):
        return "reference report contains an invalid function identity"
    try:
        expected_scope = classify_path(path)
    except ValueError as error:
        return str(error)
    if scope != expected_scope:
        return f"reference report scope drift: {path} is classified as {scope}"
    if status not in REFERENCE_STATUSES:
        return f"reference report has an invalid status: {path}::{function}"
    identity = (path, line if isinstance(line, int) else None, function)
    if identity in identities:
        return f"reference report contains a duplicate identity: {identity}"
    identities.add(identity)
    counts[scope]["functions"] += 1
    counts[scope][status] += 1
    return None


def _reference_count_errors(
    summary: dict[str, Any],
    functions: list[Any],
    counts: dict[str, dict[str, int]],
) -> list[str]:
    errors: list[str] = []
    summary_all = summary.get("all", {})
    summary_scopes = summary.get("scopes", {})
    if summary_all.get("functions") != len(functions):
        errors.append("reference report total does not match its function records")
    for scope, actual in counts.items():
        expected = summary_scopes.get(scope, {})
        expected_status = expected.get("status", {})
        if expected.get("functions") != actual["functions"]:
            errors.append(f"reference report {scope} function count is inconsistent")
        for status in REFERENCE_STATUSES:
            if expected_status.get(status) != actual[status]:
                errors.append(
                    f"reference report {scope} {status} count is inconsistent"
                )
    return errors


def validate_reference(document: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    if document.get("schema_version") != 1:
        return ["reference report uses an unsupported schema"]

    source = document.get("source")
    summary = document.get("summary")
    functions = document.get("functions")
    if not isinstance(source, dict) or not isinstance(source.get("report_sha256"), str):
        errors.append("reference report source identity is incomplete")
    if not isinstance(summary, dict) or not isinstance(functions, list) or not functions:
        errors.append("reference report is incomplete")
        return errors

    identities: set[tuple[str, int | None, str]] = set()
    counts = {
        scope: {"functions": 0, "fail": 0, "warning": 0, "pass": 0}
        for scope in ("production", "tests", "external", "tools")
    }
    for entry in functions:
        error = _reference_entry_error(entry, identities, counts)
        if error is not None:
            errors.append(error)
    errors.extend(_reference_count_errors(summary, functions, counts))
    return errors


def load_exact_report(
    path: Path,
    policy: dict[str, Any] | None = None,
) -> dict[str, Any]:
    if path.suffix.lower() == ".json":
        document = load_json(path, "CRAP report")
        if document.get("schema_version") == 1:
            return document
    report = parse_report(path)
    if policy is not None:
        report = apply_coverage_unavailable_source_policy(report, policy)
    return baseline_document(report, path.name)


def _function_detail(entry: dict[str, Any]) -> str:
    return (
        f"{entry.get('path')}:{entry.get('line')}::{entry.get('function')} "
        f"CRAP={entry.get('crap')} coverage={entry.get('coverage_percent')}%"
    )


def _limit_error(
    label: str,
    count: int,
    maximum: int,
    entries: list[dict[str, Any]],
) -> str:
    detail = "; ".join(_function_detail(entry) for entry in entries)
    return f"{label}: {count} > {maximum}" + (f" [{detail}]" if detail else "")


def validate_exact_report(
    document: dict[str, Any],
    report_policy: dict[str, Any],
    strict: bool,
) -> tuple[list[str], dict[str, int]]:
    errors = validate_reference(document)
    functions = document.get("functions")
    if not isinstance(functions, list):
        return [*errors, "CRAP report has no function records"], {}

    threshold = document.get("source", {}).get("threshold")
    expected_threshold = report_policy.get("threshold")
    if threshold != expected_threshold:
        errors.append(
            f"CRAP threshold changed: {threshold!r} != {expected_threshold!r}"
        )

    production = [
        entry for entry in functions
        if isinstance(entry, dict) and entry.get("scope") == "production"
    ]
    failure_entries = [entry for entry in production if entry.get("status") == "fail"]
    warning_entries = [entry for entry in production if entry.get("status") == "warning"]
    failures = len(failure_entries)
    warnings = len(warning_entries)
    maximum_failures = report_policy.get("maximum_production_failures")
    maximum_warnings = report_policy.get("maximum_production_warnings")
    if not isinstance(maximum_failures, int) or not isinstance(maximum_warnings, int):
        errors.append("CRAP report limits are incomplete")
    elif strict:
        if failures > maximum_failures:
            errors.append(_limit_error(
                "production CRAP failures remain",
                failures,
                maximum_failures,
                failure_entries,
            ))
        if warnings > maximum_warnings:
            errors.append(_limit_error(
                "production CRAP warnings regressed",
                warnings,
                maximum_warnings,
                warning_entries,
            ))
    return errors, {
        "production_functions": len(production),
        "production_failures": failures,
        "production_warnings": warnings,
    }


def _is_percent(value: Any) -> bool:
    return isinstance(value, (int, float)) and 0 <= value <= 100


def schema_errors(policy: dict[str, Any], ratchet: dict[str, Any]) -> list[str]:
    if policy.get("schema_version") != 1:
        return ["CRAP policy uses an unsupported schema"]
    if ratchet.get("schema_version") != 1:
        return ["CRAP ratchet contract uses an unsupported schema"]
    return []


def ratchet_shape_errors(ratchet: dict[str, Any]) -> list[str]:
    profile = ratchet.get("coverage_profile")
    floors = ratchet.get("host_production_minimum_percent")
    if not isinstance(profile, dict) or not isinstance(floors, dict):
        return ["CRAP ratchet contract is incomplete"]
    return [
        f"CRAP ratchet contract has invalid {metric} floor"
        for metric in ("lines", "functions", "branches")
        if not _is_percent(floors.get(metric))
    ]


def static_policy_stage(
    root: Path,
    policy: dict[str, Any],
) -> tuple[list[str], dict[str, Any], dict[str, Any]]:
    source_errors, source_stats = check_source_policy(
        root,
        policy.get("source_complexity", {}),
    )
    firmware_errors, firmware_stats = check_firmware_policy(
        root,
        policy.get("firmware_testability", {}),
    )
    return [*source_errors, *firmware_errors], source_stats, firmware_stats


def _compare_previous_report(
    previous_path: Path,
    exact: dict[str, Any],
    policy: dict[str, Any],
) -> tuple[list[str], Any]:
    try:
        previous = load_exact_report(previous_path, policy)
    except (ValueError, OSError) as error:
        return [str(error)], None
    return compare_reports(previous, exact, policy.get("regression", {}))


def report_stage(
    report_path: Path | None,
    policy: dict[str, Any],
    strict: bool,
    previous_report: Path | None,
) -> tuple[list[str], dict[str, int] | None, Any]:
    if report_path is None:
        return [], None, None
    if not report_path.is_file():
        return [f"CRAP report does not exist: {report_path}"], None, None
    try:
        exact = load_exact_report(report_path, policy)
    except (ValueError, OSError) as error:
        return [str(error)], None, None
    errors, exact_stats = validate_exact_report(exact, policy.get("report", {}), strict)
    if previous_report is None:
        return errors, exact_stats, None
    regression_errors, regression_stats = _compare_previous_report(
        previous_report,
        exact,
        policy,
    )
    return [*errors, *regression_errors], exact_stats, regression_stats


def manifest_stage(
    run_manifest: Path | None,
    previous_run_manifest: Path | None,
    policy: dict[str, Any],
) -> tuple[list[str], dict[str, Any] | None, dict[str, Any] | None]:
    if run_manifest is None or previous_run_manifest is None:
        return [], None, None
    try:
        current_run = load_json(run_manifest, "coverage run manifest")
        previous_run = load_json(
            previous_run_manifest,
            "previous coverage run manifest",
        )
    except ValueError as error:
        return [str(error)], None, None
    return (
        compare_coverage_manifests(previous_run, current_run, policy.get("regression", {})),
        current_run,
        previous_run,
    )


def _supplemental_coverage(
    browser_path: Path | None,
    web_path: Path | None,
) -> tuple[dict[str, dict[str, Any]], list[str]]:
    coverage: dict[str, dict[str, Any]] = {}
    errors: list[str] = []
    sources = (
        ("browser_recovery", browser_path, "browser recovery coverage"),
        ("web_runtime", web_path, "web runtime coverage"),
    )
    for key, path, label in sources:
        if path is None:
            continue
        try:
            coverage[key] = load_json(path, label)
        except ValueError as error:
            errors.append(str(error))
    return coverage, errors


def _profile_errors(profile: Any, health_run: dict[str, Any]) -> list[str]:
    if not isinstance(profile, dict):
        return []
    current_profile = health_run.get("coverage_profile")
    if current_profile == profile:
        return []
    return [
        "fresh coverage profile does not match committed CRAP ratchet: "
        f"{current_profile!r} != {profile!r}"
    ]


def _coverage_tolerance(policy: dict[str, Any]) -> float:
    tolerance = policy.get("regression", {}).get("coverage_drop_tolerance_percent", 0.05)
    if not isinstance(tolerance, (int, float)) or tolerance < 0:
        return 0.05
    return tolerance


def _floor_errors(
    floors: Any,
    policy: dict[str, Any],
    health_document: dict[str, Any],
) -> list[str]:
    if not isinstance(floors, dict):
        return []
    tolerance = _coverage_tolerance(policy)
    host_metrics = health_document.get("host_metrics", {})
    errors: list[str] = []
    for metric in ("lines", "functions", "branches"):
        observed = host_metrics.get(metric, {})
        actual = observed.get("percent") if isinstance(observed, dict) else None
        floor = floors.get(metric)
        if (
            isinstance(actual, (int, float))
            and isinstance(floor, (int, float))
            and actual + tolerance < floor
        ):
            errors.append(
                f"host production {metric} coverage regressed: "
                f"{actual:.2f}% < {floor:.2f}% "
                f"(tolerance {tolerance:.2f}%)"
            )
    return errors


def _previous_health_errors(
    previous_health_path: Path | None,
    health_document: dict[str, Any],
    previous_run: dict[str, Any] | None,
    current_run: dict[str, Any] | None,
    policy: dict[str, Any],
) -> list[str]:
    if previous_health_path is None or current_run is None or previous_run is None:
        return []
    try:
        previous_health = load_json(previous_health_path, "previous health summary")
    except ValueError as error:
        return [str(error)]
    return compare_health_summaries(
        previous_health,
        health_document,
        previous_run,
        current_run,
        policy.get("regression", {}),
    )


def _strict_health_errors(health_errors: list[str]) -> list[str]:
    return [
        error
        for error in health_errors
        if not error.startswith("production CRAP failures remain:")
        and not error.startswith("production warning target is not met:")
    ]


def _write_health_output(path: Path, document: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(document, indent=2, sort_keys=True) + "\n")


def health_stage(
    report_path: Path | None,
    arguments: Any,
    policy: dict[str, Any],
    profile: Any,
    floors: Any,
    current_run: dict[str, Any] | None,
    previous_run: dict[str, Any] | None,
) -> tuple[list[str], dict[str, Any] | None]:
    if report_path is None or arguments.lcov is None or arguments.run_manifest is None:
        return [], None
    try:
        health_report = load_exact_report(report_path, policy)
        health_run = load_json(arguments.run_manifest, "coverage run manifest")
        health_records = parse_lcov(arguments.lcov)
    except (ValueError, OSError) as error:
        return [str(error)], None
    supplemental, errors = _supplemental_coverage(
        arguments.browser_recovery_coverage,
        arguments.web_runtime_coverage,
    )
    health_errors, health_document = audit_health(
        health_report,
        health_records,
        health_run,
        policy,
        supplemental,
    )
    errors.extend(_profile_errors(profile, health_run))
    errors.extend(_floor_errors(floors, policy, health_document))
    errors.extend(_previous_health_errors(
        arguments.previous_health_summary,
        health_document,
        previous_run,
        current_run,
        policy,
    ))
    if arguments.strict_report:
        errors.extend(_strict_health_errors(health_errors))
    if arguments.health_output is not None:
        _write_health_output(arguments.health_output, health_document)
    return errors, health_document
