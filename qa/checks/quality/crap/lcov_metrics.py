"""LCOV parsing and aggregation for CRAP health checks."""

from __future__ import annotations

from fnmatch import fnmatch
from pathlib import Path

METRICS = ("lines", "functions", "branches")

_SUMMARY_KEYS = frozenset({"LF", "LH", "FNF", "FNH", "BRF", "BRH"})
_SUMMARY_MAPPING = {
    "LF": ("lines", "found"),
    "LH": ("lines", "hit"),
    "FNF": ("functions", "found"),
    "FNH": ("functions", "hit"),
    "BRF": ("branches", "found"),
    "BRH": ("branches", "hit"),
}


def _normalize_path(raw: str) -> str:
    normalized = raw.replace("\\", "/").removeprefix("./")
    # LCOV records can be emitted from arbitrary checkout directory names
    # (for example KasKold-build-... rather than literally /kaskold/).
    # Normalize by stable repository-owned roots, not by the checkout basename.
    for marker in ("/crates/", "/apps/", "/qa/", "/tools/", "/external/"):
        if marker in normalized:
            return marker[1:] + normalized.rsplit(marker, 1)[1]
    legacy_marker = "/kaskold/"
    if legacy_marker in normalized:
        return normalized.split(legacy_marker, 1)[1]
    return normalized


def _empty_metric() -> dict[str, int | float | bool]:
    return {"found": 0, "hit": 0, "percent": 0.0, "available": False}


def _finish_metric(metric: dict[str, int | float | bool]) -> None:
    found = int(metric["found"])
    hit = int(metric["hit"])
    metric["percent"] = round(hit * 100.0 / found, 4) if found else 0.0
    metric["available"] = found > 0


class _LcovRecord:
    """Accumulates the raw LCOV lines of one source file."""

    def __init__(self, path: str) -> None:
        self.path = path
        self.summary: dict[str, int] = {}
        self.line_hits: dict[int, int] = {}
        self.functions: set[str] = set()
        self.function_hits: dict[str, int] = {}
        self.branches: set[tuple[str, ...]] = set()
        self.branch_hits: set[tuple[str, ...]] = set()

    def apply(self, prefix: str, value: str) -> None:
        if prefix in _SUMMARY_KEYS:
            self._add_summary(prefix, value)
        elif prefix == "DA":
            self._add_line_hit(value)
        elif prefix == "FN":
            self._add_function(value)
        elif prefix == "FNDA":
            self._add_function_hit(value)
        elif prefix == "BRDA":
            self._add_branch(value)

    def _add_summary(self, prefix: str, value: str) -> None:
        try:
            self.summary[prefix] = int(value)
        except ValueError:
            pass

    def _add_line_hit(self, value: str) -> None:
        fields = value.split(",")
        if len(fields) >= 2:
            try:
                self.line_hits[int(fields[0])] = int(fields[1])
            except ValueError:
                pass

    def _add_function(self, value: str) -> None:
        _, separator, name = value.partition(",")
        if separator:
            self.functions.add(name)

    def _add_function_hit(self, value: str) -> None:
        count, separator, name = value.partition(",")
        if separator:
            self.functions.add(name)
            try:
                self.function_hits[name] = self.function_hits.get(name, 0) + int(float(count))
            except ValueError:
                pass

    def _add_branch(self, value: str) -> None:
        fields = tuple(value.split(","))
        if len(fields) >= 4:
            identity = fields[:3]
            self.branches.add(identity)
            if fields[3] not in {"-", "0"}:
                self.branch_hits.add(identity)

    def _derived_totals(self) -> dict[str, int]:
        return {
            "LF": len(self.line_hits),
            "LH": sum(value > 0 for value in self.line_hits.values()),
            "FNF": len(self.functions),
            "FNH": sum(self.function_hits.get(name, 0) > 0 for name in self.functions),
            "BRF": len(self.branches),
            "BRH": len(self.branch_hits),
        }

    def metrics(self) -> dict[str, dict[str, int | float | bool]]:
        metrics = {name: _empty_metric() for name in METRICS}
        derived = self._derived_totals()
        for key, (metric_name, field) in _SUMMARY_MAPPING.items():
            # LLVM/LCOV producers can emit stale BRH summary counters after
            # merging instrumented Rust monomorphizations even when the concrete
            # BRDA records show every branch was executed. When BRDA exists, the
            # per-branch records are the authoritative evidence. Keep summary
            # fallback only for producers that provide branch totals without
            # concrete branch records.
            if metric_name == "branches" and self.branches:
                metrics[metric_name][field] = derived[key]
            else:
                metrics[metric_name][field] = self.summary.get(key, derived[key])
        for metric in metrics.values():
            _finish_metric(metric)
        return metrics


def _store_record(
    records: dict[str, dict[str, dict[str, int | float | bool]]],
    record: _LcovRecord | None,
) -> None:
    if record is not None:
        records[record.path] = record.metrics()


def parse_lcov(path: Path) -> dict[str, dict[str, dict[str, int | float | bool]]]:
    """Parse per-file LCOV totals, deriving totals when summaries are absent."""
    records: dict[str, dict[str, dict[str, int | float | bool]]] = {}
    record: _LcovRecord | None = None
    for raw in path.read_text(errors="replace").splitlines():
        if raw.startswith("SF:"):
            _store_record(records, record)
            record = _LcovRecord(_normalize_path(raw[3:]))
            continue
        if raw == "end_of_record":
            _store_record(records, record)
            record = None
            continue
        if record is None:
            continue
        prefix, separator, value = raw.partition(":")
        if separator:
            record.apply(prefix, value)
    _store_record(records, record)
    return records


def aggregate(
    records: dict[str, dict[str, dict[str, int | float | bool]]],
    paths: list[str],
) -> dict[str, dict[str, int | float | bool]]:
    totals = {name: _empty_metric() for name in METRICS}
    for path in paths:
        record = records.get(path)
        if record is None:
            continue
        for metric_name in METRICS:
            totals[metric_name]["found"] = int(totals[metric_name]["found"]) + int(
                record[metric_name]["found"]
            )
            totals[metric_name]["hit"] = int(totals[metric_name]["hit"]) + int(
                record[metric_name]["hit"]
            )
    for metric in totals.values():
        _finish_metric(metric)
    return totals


def matches(path: str, patterns: list[str]) -> bool:
    return any(fnmatch(path, pattern) for pattern in patterns)
