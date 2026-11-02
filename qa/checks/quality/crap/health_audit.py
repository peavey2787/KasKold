"""Strict-health audit facade: composes policy, domain, and section helpers."""
from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

CHECK_DIR = Path(__file__).resolve().parent
if str(CHECK_DIR) not in sys.path:
    sys.path.insert(0, str(CHECK_DIR))

from health_audit_domains import evaluate_critical_domains, host_production_paths  # noqa: E402
from health_audit_policy import build_limits, health_policy_error  # noqa: E402
from health_audit_sections import (  # noqa: E402
    AuditFindings,
    adapter_entries,
    adapter_offenders,
    branch_status,
    build_criteria,
    build_document,
    build_errors,
    firmware_model_status,
    production_entries,
    production_tally,
    web_runtime_status,
)
from lcov_metrics import aggregate as _aggregate  # noqa: E402


def audit_health(
    report: dict[str, Any],
    lcov_records: dict[str, dict[str, dict[str, int | float | bool]]],
    run_manifest: dict[str, Any],
    policy: dict[str, Any],
    supplemental_coverage: dict[str, dict[str, Any]] | None = None,
) -> tuple[list[str], dict[str, Any]]:
    """Return strict-health errors and a machine-readable audit document."""
    health = policy.get("health")
    firmware = policy.get("firmware_testability")
    policy_error = health_policy_error(health, firmware)
    if policy_error is not None:
        return [policy_error], {}
    limits = build_limits(health)
    supplemental = supplemental_coverage or {}
    production = production_entries(report)
    host_paths = host_production_paths(lcov_records, limits.host_roots)
    domain_error, domain_results = evaluate_critical_domains(lcov_records, limits, supplemental)
    if domain_error is not None:
        return [domain_error], {}
    required_models, missing_models = firmware_model_status(firmware, lcov_records)
    adapters = adapter_entries(production, limits.adapter_roots)
    findings = AuditFindings(
        tally=production_tally(production),
        host_metrics=_aggregate(lcov_records, host_paths),
        host_file_count=len(host_paths),
        required_models=required_models,
        missing_models=missing_models,
        adapters=adapters,
        offenders=adapter_offenders(adapters, limits.max_adapter_cc),
        branch=branch_status(run_manifest, limits.require_branches),
        web=web_runtime_status(supplemental, limits.min_web_mapping),
    )
    criteria = build_criteria(findings, limits)
    errors = build_errors(criteria, findings, limits, domain_results)
    return errors, build_document(errors, findings, criteria, domain_results, supplemental)
