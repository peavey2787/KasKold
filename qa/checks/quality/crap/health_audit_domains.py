"""Host-production and critical-domain coverage evaluation for the strict health audit."""
from __future__ import annotations

from typing import Any

from health_audit_policy import HealthLimits
from lcov_metrics import aggregate, matches
from report import classify_path


def host_production_paths(lcov_records: dict[str, Any], host_roots: list[str]) -> list[str]:
    return sorted(
        path
        for path in lcov_records
        if any(path.startswith(root) for root in host_roots)
        and classify_path(path) == "production"
    )


def evaluate_critical_domains(
    lcov_records: dict[str, Any],
    limits: HealthLimits,
    supplemental_coverage: dict[str, dict[str, Any]],
) -> tuple[str | None, dict[str, Any]]:
    """Return (branch-policy error or None, per-domain results keyed by domain name)."""
    domain_results: dict[str, Any] = {}
    for name, domain in limits.critical_domains.items():
        if not _is_evaluable_domain(name, domain):
            continue
        branch_policy = _branch_policy(domain)
        if branch_policy is None:
            return f"critical-domain branch policy is incomplete for {name}", {}
        domain_results[name] = _domain_result(
            lcov_records,
            name,
            domain,
            branch_policy,
            limits.min_critical,
            supplemental_coverage,
        )
    return None, domain_results


def _is_evaluable_domain(name: Any, domain: Any) -> bool:
    if not isinstance(name, str) or not isinstance(domain, dict):
        return False
    patterns = domain.get("paths")
    return isinstance(patterns, list) and all(isinstance(pattern, str) for pattern in patterns)


def _branch_policy(domain: dict[str, Any]) -> tuple[float, float] | None:
    floor = domain.get("minimum_branch_coverage_percent")
    target = domain.get("target_branch_coverage_percent", 90.0)
    if not isinstance(floor, (int, float)) or not isinstance(target, (int, float)):
        return None
    return float(floor), float(target)


def _domain_result(
    lcov_records: dict[str, Any],
    name: str,
    domain: dict[str, Any],
    branch_policy: tuple[float, float],
    min_critical: float,
    supplemental_coverage: dict[str, dict[str, Any]],
) -> dict[str, Any]:
    patterns = domain["paths"]
    paths = sorted(
        path
        for path in lcov_records
        if classify_path(path) == "production" and matches(path, patterns)
    )
    metrics = aggregate(lcov_records, paths)
    branch_floor, branch_target = branch_policy
    branch_metrics = metrics["branches"]
    branch_percent = float(branch_metrics["percent"])
    branch_met = (
        bool(branch_metrics["available"])
        and int(branch_metrics["found"]) > 0
        and branch_percent >= branch_floor
    )
    line_percent = float(metrics["lines"]["percent"])
    function_percent = float(metrics["functions"]["percent"])
    rust_met = (
        bool(paths)
        and line_percent >= min_critical
        and function_percent >= min_critical
        and branch_met
    )
    supplemental = _supplemental_result(domain, supplemental_coverage, min_critical)
    supplemental_met = supplemental is None or bool(supplemental["met"])
    return {
        "label": domain.get("label", name),
        "files": len(paths),
        "metrics": metrics,
        "branch_gate": {
            "actual": branch_percent,
            "minimum": branch_floor,
            "target": branch_target,
            "met": branch_met,
        },
        "rust_met": rust_met,
        "supplemental": supplemental,
        "met": rust_met and supplemental_met,
    }


def _supplemental_result(
    domain: dict[str, Any],
    supplemental_coverage: dict[str, dict[str, Any]],
    min_critical: float,
) -> dict[str, Any] | None:
    key = domain.get("supplemental_coverage")
    if not isinstance(key, str):
        return None
    return _supplemental_domain_result(key, supplemental_coverage.get(key), min_critical)


def _supplemental_domain_result(
    name: str,
    document: dict[str, Any] | None,
    minimum_percent: float,
) -> dict[str, Any]:
    coverage = document.get("coverage") if isinstance(document, dict) else None
    lines = coverage.get("lines") if isinstance(coverage, dict) else None
    functions = coverage.get("functions") if isinstance(coverage, dict) else None
    branches = coverage.get("branches") if isinstance(coverage, dict) else None
    line_percent = float(lines.get("percent", 0.0)) if isinstance(lines, dict) else 0.0
    function_percent = (
        float(functions.get("percent", 0.0)) if isinstance(functions, dict) else 0.0
    )
    branch_percent = (
        float(branches.get("percent", 0.0)) if isinstance(branches, dict) else 0.0
    )
    available = bool(
        isinstance(lines, dict)
        and lines.get("available")
        and isinstance(functions, dict)
        and functions.get("available")
        and isinstance(branches, dict)
        and branches.get("available")
    )
    files = document.get("files") if isinstance(document, dict) else None
    missing = files.get("missing", []) if isinstance(files, dict) else []
    complete = isinstance(missing, list) and not missing
    return {
        "name": name,
        "available": available,
        "complete": complete,
        "line_percent": line_percent,
        "function_percent": function_percent,
        "branch_percent": branch_percent,
        "files": files if isinstance(files, dict) else {},
        "met": available
        and complete
        and line_percent >= minimum_percent
        and function_percent >= minimum_percent
        and branch_percent >= minimum_percent,
    }
