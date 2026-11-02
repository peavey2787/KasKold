"""Policy validation and typed limits for the strict health audit."""
from __future__ import annotations

from typing import Any

NUMERIC_LIMIT_KEYS = (
    "maximum_production_failures",
    "maximum_production_warnings",
    "maximum_production_warning_percent",
    "minimum_host_line_coverage_percent",
    "minimum_host_function_coverage_percent",
    "minimum_host_branch_coverage_percent",
    "minimum_critical_domain_coverage_percent",
    "minimum_web_runtime_mapping_percent",
    "maximum_board_adapter_cc",
)


class HealthLimits:
    """Typed view over a validated health policy; each value converts at its point of use."""

    def __init__(self, health: dict[str, Any]) -> None:
        self._health = health

    @property
    def max_failures(self) -> int:
        return int(self._health["maximum_production_failures"])

    @property
    def max_warnings(self) -> int:
        return int(self._health["maximum_production_warnings"])

    @property
    def max_warning_percent(self) -> float:
        return float(self._health["maximum_production_warning_percent"])

    @property
    def min_host_line(self) -> float:
        return float(self._health["minimum_host_line_coverage_percent"])

    @property
    def min_host_function(self) -> float:
        return float(self._health["minimum_host_function_coverage_percent"])

    @property
    def min_host_branch(self) -> float:
        return float(self._health["minimum_host_branch_coverage_percent"])

    @property
    def min_critical(self) -> float:
        return float(self._health["minimum_critical_domain_coverage_percent"])

    @property
    def min_web_mapping(self) -> float:
        return float(self._health["minimum_web_runtime_mapping_percent"])

    @property
    def max_adapter_cc(self) -> int:
        return int(self._health["maximum_board_adapter_cc"])

    @property
    def host_roots(self) -> list[str]:
        return self._health["host_production_roots"]

    @property
    def adapter_roots(self) -> list[str]:
        return self._health["board_adapter_roots"]

    @property
    def critical_domains(self) -> dict[str, Any]:
        return self._health["critical_domains"]

    @property
    def require_branches(self) -> bool:
        return bool(self._health.get("require_branch_coverage"))


def health_policy_error(health: Any, firmware: Any) -> str | None:
    """Return the first policy-completeness error, or None when the policy is usable."""
    if not isinstance(health, dict) or not isinstance(firmware, dict):
        return "health policy is incomplete"
    if not all(isinstance(health.get(key), (int, float)) for key in NUMERIC_LIMIT_KEYS):
        return "health numeric limits are incomplete"
    if not _is_string_list(health.get("host_production_roots")):
        return "host-production roots are incomplete"
    if not _is_string_list(health.get("board_adapter_roots")):
        return "board-adapter roots are incomplete"
    if not isinstance(health.get("critical_domains"), dict):
        return "critical-domain policy is incomplete"
    return None


def build_limits(health: dict[str, Any]) -> HealthLimits:
    """Wrap a validated health policy as typed limits."""
    return HealthLimits(health)


def _is_string_list(value: Any) -> bool:
    return isinstance(value, list) and all(isinstance(item, str) for item in value)
