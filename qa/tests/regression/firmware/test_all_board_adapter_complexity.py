#!/usr/bin/env python3
"""Regression contracts for every supported CoreS3-family hardware adapter."""

from __future__ import annotations

from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "qa/checks/quality/crap"))

from source_complexity import function_decisions  # noqa: E402

BOARD_ROOTS = (
    "apps/kaskold-hardware/src/hw/m5stack",
)


class AllBoardAdapterComplexityTests(unittest.TestCase):
    def test_every_production_board_function_stays_at_cc_four_or_less(self) -> None:
        offenders: list[str] = []
        for relative_root in BOARD_ROOTS:
            for path in (ROOT / relative_root).rglob("*.rs"):
                if "unit_tests" in path.parts:
                    continue
                relative = path.relative_to(ROOT).as_posix()
                for record in function_decisions(path.read_text(), relative):
                    if record.decisions > 4:
                        offenders.append(
                            f"{relative}:{record.line} {record.name}={record.decisions}"
                        )
        self.assertEqual(offenders, [])


if __name__ == "__main__":
    unittest.main()
