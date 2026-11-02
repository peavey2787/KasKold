#!/usr/bin/env python3
"""Run the consolidated CRAP report, complexity, and testability checks."""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[4]
CHECK_DIR = ROOT / "qa/checks/quality/crap"
POLICY_PATH = CHECK_DIR / "policy.json"
RATCHET_PATH = ROOT / "qa/contracts/quality/crap_ratchets.json"
GENERATED_REPORT_PATH = ROOT / "target/qa/crap/current.json"
sys.path.insert(0, str(CHECK_DIR))

from check_stages import (  # noqa: E402
    health_stage,
    load_json,
    manifest_stage,
    ratchet_shape_errors,
    report_stage,
    schema_errors,
    static_policy_stage,
)


def fail(errors: list[str]) -> int:
    for error in errors:
        print(f"ERROR: {error}")
    return 1


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="enforce CRAP scope, source complexity, and firmware testability"
    )
    parser.add_argument(
        "--report",
        type=Path,
        help="fresh full CRAP report (.txt) or classified JSON",
    )
    parser.add_argument(
        "--strict-report",
        action="store_true",
        help="enforce the configured production failure and warning limits",
    )
    parser.add_argument(
        "--previous-report",
        type=Path,
        help="previous classified snapshot used to reject new production failures",
    )
    parser.add_argument(
        "--run-manifest",
        type=Path,
        help="fresh coverage run manifest containing aggregate coverage totals",
    )
    parser.add_argument(
        "--lcov",
        type=Path,
        help="fresh LCOV trace used for complete health-criteria measurement",
    )
    parser.add_argument(
        "--health-output",
        type=Path,
        help="write a machine-readable complete health audit",
    )
    parser.add_argument(
        "--browser-recovery-coverage",
        type=Path,
        help="persisted Companion browser recovery coverage summary",
    )
    parser.add_argument(
        "--web-runtime-coverage",
        type=Path,
        help="persisted all-module Companion runtime V8 coverage summary",
    )
    parser.add_argument(
        "--previous-run-manifest",
        type=Path,
        help="previous coverage run manifest used to verify comparable instrumentation",
    )
    parser.add_argument(
        "--previous-health-summary",
        type=Path,
        help="previous classified health summary used to ratchet host-production coverage",
    )
    parser.add_argument(
        "--ratchet-contract",
        type=Path,
        default=RATCHET_PATH,
        help="compact committed host-coverage ratchet contract",
    )
    parser.add_argument(
        "--ignore-generated-report",
        action="store_true",
        help="run source/testability policy without loading target/qa/crap/current.json",
    )
    return parser.parse_args()


def _report_path(arguments: argparse.Namespace) -> Path | None:
    if arguments.report is not None or arguments.ignore_generated_report:
        return arguments.report
    if GENERATED_REPORT_PATH.is_file():
        return GENERATED_REPORT_PATH
    return None


def _report_suffix(
    exact_stats: dict[str, int] | None,
    strict_report: bool,
) -> str:
    if exact_stats is None:
        return ""
    mode = "strict" if strict_report else "debt-aware"
    return (
        f"; fresh report ({mode}): "
        f"{exact_stats['production_failures']} failures, "
        f"{exact_stats['production_warnings']} warnings"
    )


def _regression_suffix(regression_stats) -> str:
    if regression_stats is None:
        return ""
    return (
        "; regression gate: "
        f"{regression_stats.new_failures} new failures, "
        f"{regression_stats.new_warnings} new warnings, "
        f"{regression_stats.measured_to_unavailable} coverage losses"
    )


def _health_suffix(health_document: dict | None) -> str:
    if health_document is None:
        return ""
    criteria = health_document.get("criteria", {})
    host = health_document.get("host_metrics", {})
    return (
        "; complete health audit: "
        f"host lines {host.get('lines', {}).get('percent', 0.0):.2f}%, "
        f"host functions {host.get('functions', {}).get('percent', 0.0):.2f}%, "
        f"{criteria.get('pure_firmware_models', {}).get('actual', 0)}/"
        f"{criteria.get('pure_firmware_models', {}).get('target', 0)} "
        "firmware models measured, "
        f"{criteria.get('board_adapters_above_cc_limit', {}).get('actual', 0)} "
        "board adapters above CC 4"
    )


def _success_message(
    arguments: argparse.Namespace,
    source_stats: dict,
    firmware_stats: dict,
    exact_stats: dict[str, int] | None,
    regression_stats,
    health_document: dict | None,
) -> str:
    return (
        "PASS: CRAP quality check "
        f"({source_stats.get('production_functions', 0)} production functions; "
        f"max source decisions {source_stats.get('maximum_decisions', 0)}; "
        f"{source_stats.get('warning_functions', 0)} source warnings; "
        f"{firmware_stats.get('host_tests', 0)} firmware host tests)"
        + _report_suffix(exact_stats, arguments.strict_report)
        + _regression_suffix(regression_stats)
        + _health_suffix(health_document)
    )


def main() -> int:
    arguments = parse_args()
    try:
        policy = load_json(POLICY_PATH, "CRAP policy")
        ratchet = load_json(arguments.ratchet_contract, "CRAP ratchet contract")
    except ValueError as error:
        return fail([str(error)])

    schema = schema_errors(policy, ratchet)
    if schema:
        return fail(schema)

    profile = ratchet.get("coverage_profile")
    floors = ratchet.get("host_production_minimum_percent")
    errors = ratchet_shape_errors(ratchet)

    static_errors, source_stats, firmware_stats = static_policy_stage(ROOT, policy)
    errors.extend(static_errors)

    report_path = _report_path(arguments)
    report_errors, exact_stats, regression_stats = report_stage(
        report_path,
        policy,
        arguments.strict_report,
        arguments.previous_report,
    )
    errors.extend(report_errors)

    manifest_errors, current_run, previous_run = manifest_stage(
        arguments.run_manifest,
        arguments.previous_run_manifest,
        policy,
    )
    errors.extend(manifest_errors)

    health_errors, health_document = health_stage(
        report_path,
        arguments,
        policy,
        profile,
        floors,
        current_run,
        previous_run,
    )
    errors.extend(health_errors)

    if errors:
        return fail(errors)
    print(
        _success_message(
            arguments,
            source_stats,
            firmware_stats,
            exact_stats,
            regression_stats,
            health_document,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
