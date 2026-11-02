"""Criteria, error, and audit-document assembly for the strict health audit."""
from __future__ import annotations

from dataclasses import dataclass
from typing import Any

from health_audit_policy import HealthLimits


@dataclass(frozen=True)
class ProductionTally:
    function_count: int
    failures: int
    warnings: int
    raw_failures: int
    raw_warnings: int
    source_assessed: int
    warning_percent: float


@dataclass(frozen=True)
class AuditFindings:
    tally: ProductionTally
    host_metrics: dict[str, Any]
    host_file_count: int
    required_models: list[str]
    missing_models: list[str]
    adapters: list[dict[str, Any]]
    offenders: list[dict[str, Any]]
    branch: dict[str, Any]
    web: dict[str, Any]


def production_entries(report: dict[str, Any]) -> list[dict[str, Any]]:
    functions = report.get("functions")
    if not isinstance(functions, list):
        return []
    return [
        entry
        for entry in functions
        if isinstance(entry, dict) and entry.get("scope") == "production"
    ]


def criterion(actual: float | int, target: float | int, met: bool) -> dict[str, Any]:
    return {"actual": actual, "target": target, "met": met}


def production_tally(production: list[dict[str, Any]]) -> ProductionTally:
    function_count = len(production)
    warnings = sum(entry.get("status") == "warning" for entry in production)
    return ProductionTally(
        function_count=function_count,
        failures=sum(entry.get("status") == "fail" for entry in production),
        warnings=warnings,
        raw_failures=sum(_raw_status(entry) == "fail" for entry in production),
        raw_warnings=sum(_raw_status(entry) == "warning" for entry in production),
        source_assessed=sum(
            entry.get("assessment_basis") == "source_complexity" for entry in production
        ),
        warning_percent=round(warnings * 100.0 / function_count, 4) if function_count else 0.0,
    )


def _raw_status(entry: dict[str, Any]) -> Any:
    return entry.get("raw_status", entry.get("status"))


def firmware_model_status(
    firmware: dict[str, Any],
    lcov_records: dict[str, Any],
) -> tuple[list[str], list[str]]:
    model_paths = firmware.get("model_paths")
    if not isinstance(model_paths, list):
        model_paths = []
    required = [path for path in model_paths if isinstance(path, str)]
    missing = [path for path in required if not _has_model_coverage(lcov_records, path)]
    return required, missing


def _has_model_coverage(lcov_records: dict[str, Any], path: str) -> bool:
    if path not in lcov_records:
        return False
    record = lcov_records[path]
    return bool(record["lines"]["available"]) or bool(record["functions"]["available"])


def adapter_entries(
    production: list[dict[str, Any]],
    adapter_roots: list[str],
) -> list[dict[str, Any]]:
    return [
        entry
        for entry in production
        if isinstance(entry.get("path"), str)
        and any(entry["path"].startswith(root) for root in adapter_roots)
    ]


def adapter_offenders(adapters: list[dict[str, Any]], max_adapter_cc: int) -> list[dict[str, Any]]:
    offenders = [
        _offender(entry)
        for entry in adapters
        if isinstance(entry.get("complexity"), int) and entry["complexity"] > max_adapter_cc
    ]
    offenders.sort(key=lambda entry: (-int(entry["cc"]), str(entry["path"])))
    return offenders


def _offender(entry: dict[str, Any]) -> dict[str, Any]:
    return {
        "path": entry.get("path"),
        "line": entry.get("line"),
        "function": entry.get("function"),
        "cc": entry.get("complexity"),
    }


def branch_status(run_manifest: dict[str, Any], require_branches: bool) -> dict[str, Any]:
    coverage = run_manifest.get("coverage")
    branch = coverage.get("branches") if isinstance(coverage, dict) else None
    requested = bool(run_manifest.get("branch_coverage_requested"))
    available = bool(branch.get("available")) if isinstance(branch, dict) else False
    found = int(branch.get("found", 0)) if isinstance(branch, dict) else 0
    return {
        "requested": requested,
        "available": available,
        "found": found,
        "hit": int(branch.get("hit", 0)) if isinstance(branch, dict) else 0,
        "percent": float(branch.get("percent", 0.0)) if isinstance(branch, dict) else 0.0,
        "met": (not require_branches) or (requested and available and found > 0),
    }


def web_runtime_status(supplemental: dict[str, Any], minimum_web_mapping: float) -> dict[str, Any]:
    web_runtime = supplemental.get("web_runtime", {})
    web_files = web_runtime.get("files") if isinstance(web_runtime, dict) else None
    mapping = float(web_files.get("mapping_percent", 0.0)) if isinstance(web_files, dict) else 0.0
    tests_passed = bool(web_runtime.get("tests_passed")) if isinstance(web_runtime, dict) else False
    return {
        "actual": mapping,
        "target": minimum_web_mapping,
        "tests_passed": tests_passed,
        "met": tests_passed and mapping >= minimum_web_mapping,
    }


def build_criteria(findings: AuditFindings, limits: HealthLimits) -> dict[str, Any]:
    return {
        "web_runtime_trace_mapping": findings.web,
        "production_crap_failures": criterion(
            findings.tally.failures,
            limits.max_failures,
            findings.tally.failures <= limits.max_failures,
        ),
        "production_warnings": _warnings_criterion(findings.tally, limits),
        "host_line_coverage": _host_metric_criterion(
            findings.host_metrics["lines"], limits.min_host_line
        ),
        "host_function_coverage": _host_metric_criterion(
            findings.host_metrics["functions"], limits.min_host_function
        ),
        "host_branch_coverage": _host_branch_criterion(
            findings.host_metrics["branches"],
            limits.min_host_branch,
            bool(findings.branch["requested"]),
        ),
        "pure_firmware_models": _firmware_criterion(findings),
        "board_adapters_above_cc_limit": _adapter_criterion(findings, limits),
        "branch_coverage": _branch_criterion(findings.branch, limits),
    }


def _warnings_criterion(tally: ProductionTally, limits: HealthLimits) -> dict[str, Any]:
    return {
        "actual": tally.warnings,
        "target": limits.max_warnings,
        "percent": tally.warning_percent,
        "maximum_percent": limits.max_warning_percent,
        "met": tally.warnings <= limits.max_warnings
        and tally.warning_percent <= limits.max_warning_percent,
    }


def _host_metric_criterion(metric: dict[str, Any], target: float) -> dict[str, Any]:
    percent = float(metric["percent"])
    return criterion(percent, target, bool(metric["available"]) and percent >= target)


def _host_branch_criterion(metric: dict[str, Any], target: float, requested: bool) -> dict[str, Any]:
    percent = float(metric["percent"])
    met = (
        requested
        and bool(metric["available"])
        and int(metric["found"]) > 0
        and percent >= target
    )
    return criterion(percent, target, met)


def _firmware_criterion(findings: AuditFindings) -> dict[str, Any]:
    return {
        "actual": len(findings.required_models) - len(findings.missing_models),
        "target": len(findings.required_models),
        "missing": findings.missing_models,
        "met": not findings.missing_models,
    }


def _adapter_criterion(findings: AuditFindings, limits: HealthLimits) -> dict[str, Any]:
    return {
        "actual": len(findings.offenders),
        "target": 0,
        "maximum_cc": max(
            (int(entry.get("complexity", 0)) for entry in findings.adapters), default=0
        ),
        "cc_limit": limits.max_adapter_cc,
        "offenders": findings.offenders,
        "met": not findings.offenders,
    }


def _branch_criterion(branch: dict[str, Any], limits: HealthLimits) -> dict[str, Any]:
    return {
        "requested": branch["requested"],
        "available": branch["available"],
        "found": branch["found"],
        "hit": branch["hit"],
        "percent": branch["percent"],
        "required": limits.require_branches,
        "met": branch["met"],
    }


def build_errors(
    criteria: dict[str, Any],
    findings: AuditFindings,
    limits: HealthLimits,
    domain_results: dict[str, Any],
) -> list[str]:
    errors = _production_errors(criteria, findings.tally, limits)
    errors.extend(_host_errors(criteria, findings.host_metrics, limits))
    errors.extend(_domain_errors(domain_results, limits.min_critical))
    errors.extend(_remaining_errors(criteria, findings, limits))
    return errors


def _production_errors(
    criteria: dict[str, Any],
    tally: ProductionTally,
    limits: HealthLimits,
) -> list[str]:
    errors: list[str] = []
    if not criteria["production_crap_failures"]["met"]:
        errors.append(
            f"production CRAP failures remain: {tally.failures} > {limits.max_failures}"
        )
    if not criteria["production_warnings"]["met"]:
        errors.append(
            "production warning target is not met: "
            f"{tally.warnings} ({tally.warning_percent:.2f}%) > "
            f"{limits.max_warnings} or {limits.max_warning_percent:.2f}%"
        )
    return errors


def _host_errors(
    criteria: dict[str, Any],
    host_metrics: dict[str, Any],
    limits: HealthLimits,
) -> list[str]:
    checks = (
        (
            "host_line_coverage",
            "host production line coverage",
            host_metrics["lines"],
            limits.min_host_line,
        ),
        (
            "host_function_coverage",
            "host production function coverage",
            host_metrics["functions"],
            limits.min_host_function,
        ),
        (
            "host_branch_coverage",
            "host production branch coverage",
            host_metrics["branches"],
            limits.min_host_branch,
        ),
    )
    return [
        _below_target(label, float(metric["percent"]), target)
        for key, label, metric, target in checks
        if not criteria[key]["met"]
    ]


def _below_target(label: str, actual: float, target: float) -> str:
    return f"{label} is below target: {actual:.2f}% < {target:.2f}%"


def _domain_errors(domain_results: dict[str, Any], min_critical: float) -> list[str]:
    return [
        f"critical-domain coverage is below target for {name}: "
        f"{_domain_detail(domain)} < {min_critical:.2f}%"
        for name, domain in domain_results.items()
        if not domain["met"]
    ]


def _domain_detail(domain: dict[str, Any]) -> str:
    metrics = domain["metrics"]
    branch_gate = domain.get("branch_gate", {})
    detail = (
        f"Rust lines {float(metrics['lines']['percent']):.2f}%, "
        f"Rust functions {float(metrics['functions']['percent']):.2f}%, "
        f"Rust branches {float(metrics['branches']['percent']):.2f}% "
        f"(floor {float(branch_gate.get('minimum', 0.0)):.2f}%, "
        f"target {float(branch_gate.get('target', 90.0)):.2f}%)"
    )
    supplemental = domain.get("supplemental")
    if isinstance(supplemental, dict):
        detail += _supplemental_detail(supplemental)
    return detail


def _supplemental_detail(supplemental: dict[str, Any]) -> str:
    detail = (
        f"; {supplemental.get('name')} lines "
        f"{float(supplemental.get('line_percent', 0.0)):.2f}%, "
        f"functions {float(supplemental.get('function_percent', 0.0)):.2f}%"
    )
    if not supplemental.get("complete"):
        detail += "; source inventory incomplete"
    return detail


def _remaining_errors(
    criteria: dict[str, Any],
    findings: AuditFindings,
    limits: HealthLimits,
) -> list[str]:
    errors: list[str] = []
    web = criteria["web_runtime_trace_mapping"]
    if not web["met"]:
        errors.append(
            "web runtime trace mapping is incomplete: "
            f"{web['actual']:.2f}% < {web['target']:.2f}% or integration tests failed"
        )
    if findings.missing_models:
        errors.append(
            "pure firmware coverage is unavailable for: " + ", ".join(findings.missing_models)
        )
    if findings.offenders:
        errors.append(
            "board adapter complexity exceeds the limit: "
            f"{len(findings.offenders)} function(s) above CC {limits.max_adapter_cc}"
        )
    if not criteria["branch_coverage"]["met"]:
        errors.append("branch coverage was not requested, recorded, and persisted")
    return errors


def build_document(
    errors: list[str],
    findings: AuditFindings,
    criteria: dict[str, Any],
    domain_results: dict[str, Any],
    supplemental: dict[str, Any],
) -> dict[str, Any]:
    tally = findings.tally
    return {
        "schema_version": 1,
        "healthy": not errors,
        "production_functions": tally.function_count,
        "production_assessment": {
            "crap_functions": tally.function_count - tally.source_assessed,
            "source_complexity_functions": tally.source_assessed,
            "raw_pessimistic_failures": tally.raw_failures,
            "raw_pessimistic_warnings": tally.raw_warnings,
            "effective_failures": tally.failures,
            "effective_warnings": tally.warnings,
        },
        "host_production_files": findings.host_file_count,
        "host_metrics": findings.host_metrics,
        "criteria": criteria,
        "critical_domains": domain_results,
        "supplemental_coverage": supplemental,
    }
