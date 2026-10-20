#!/usr/bin/env python3
"""Ratchet on Python tests that assert over production source text.

A test that greps Rust, JavaScript, Kotlin or Swift source proves a token is
present, not that the code behaves. Existing ones are being replaced by
behavioural tests; this ratchet stops new ones appearing and stops any
remaining file from growing. Each file's `assertIn`/`assertNotIn` count may
only fall, and the baseline must be lowered with `--tighten` when it does.
"""

from __future__ import annotations

import ast
import json
import re
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
TESTS_ROOT = ROOT / "qa/tests"
BASELINE = ROOT / "qa/baselines/source_grep_assertions.json"
PRODUCTION_SOURCE = re.compile(r"(?:^|/)(?:apps|crates|src)/[\w./-]+\.(?:rs|js|mjs|kt|swift)$")
GREP_ASSERTIONS = {"assertIn", "assertNotIn"}


def _reads_production_source(tree: ast.AST) -> bool:
    return any(
        isinstance(node, ast.Constant)
        and isinstance(node.value, str)
        and PRODUCTION_SOURCE.search(node.value) is not None
        for node in ast.walk(tree)
    )


def _grep_assertions(tree: ast.AST) -> int:
    return sum(
        1
        for node in ast.walk(tree)
        if isinstance(node, ast.Call)
        and isinstance(node.func, ast.Attribute)
        and node.func.attr in GREP_ASSERTIONS
    )


def file_count(source: str) -> int:
    """Grep assertions in a test module that reads production source, else 0."""
    tree = ast.parse(source)
    return _grep_assertions(tree) if _reads_production_source(tree) else 0


def scan(root: Path = TESTS_ROOT) -> dict[str, int]:
    counts: dict[str, int] = {}
    for path in sorted(root.rglob("test_*.py")):
        count = file_count(path.read_text(encoding="utf-8"))
        if count:
            counts[path.relative_to(ROOT).as_posix()] = count
    return counts


def compare(current: dict[str, int], baseline: dict[str, int]) -> list[str]:
    errors: list[str] = []
    for path, count in sorted(current.items()):
        allowed = baseline.get(path, 0)
        if count > allowed:
            errors.append(
                f"{path}: {count} source-text assertions exceed the ratchet ({allowed}); "
                "test behaviour instead of grepping production source"
            )
        elif count < allowed:
            errors.append(
                f"{path}: source-text assertions fell to {count} (baseline {allowed}); "
                "run qa/checks/security/source_grep_ratchet.py --tighten"
            )
    for path in sorted(set(baseline) - set(current)):
        errors.append(f"{path}: no source-text assertions remain; run --tighten to drop it")
    return errors


def tighten(current: dict[str, int], baseline: dict[str, int]) -> dict[str, int]:
    """Lower the baseline to the current counts; never raise an entry."""
    return {
        path: min(count, baseline[path])
        for path, count in sorted(current.items())
        if path in baseline
    }


def load_baseline(path: Path = BASELINE) -> dict[str, int]:
    return json.loads(path.read_text(encoding="utf-8"))


def check() -> tuple[list[str], int]:
    current = scan()
    return compare(current, load_baseline()), sum(current.values())


def main(argv: list[str]) -> int:
    if argv == ["--tighten"]:
        tightened = tighten(scan(), load_baseline())
        BASELINE.write_text(json.dumps(tightened, indent=2) + "\n", encoding="utf-8")
        print(f"PASS: ratchet lowered to {sum(tightened.values())} assertions in {len(tightened)} files")
        return 0
    errors, total = check()
    for error in errors:
        print(f"ERROR: {error}")
    if errors:
        return 1
    print(f"PASS: source-text assertion ratchet holds at {total}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
