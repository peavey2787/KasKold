#!/usr/bin/env python3
"""Regression contracts for host-owned firmware decisions and thin board adapters."""

from __future__ import annotations

from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks/quality/crap"))

from source_complexity import function_decisions  # noqa: E402


ADAPTER_PATHS = (
    "apps/kaskold-hardware/src/hw/m5stack/storage/transport/protocol/initialization.rs",
    "apps/kaskold-hardware/src/hw/m5stack/storage/transport/protocol/wire.rs",
    "apps/kaskold-hardware/src/hw/m5stack/storage/transport/block.rs",
    "apps/kaskold-hardware/src/hw/m5stack/storage/transport/multi_block.rs",
    "apps/kaskold-hardware/src/hw/m5stack/storage/transport/card.rs",
)


class FirmwareDecisionExtractionTests(unittest.TestCase):


    def test_extracted_storage_adapters_stay_at_four_decisions_or_less(self) -> None:
        offenders: list[str] = []
        for relative in ADAPTER_PATHS:
            source = (ROOT / relative).read_text()
            for record in function_decisions(source, relative):
                if record.decisions > 4:
                    offenders.append(
                        f"{relative}:{record.line} {record.name}={record.decisions}"
                    )
        self.assertEqual(offenders, [])


if __name__ == "__main__":
    unittest.main()
