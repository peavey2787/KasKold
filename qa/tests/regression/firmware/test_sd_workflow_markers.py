#!/usr/bin/env python3
"""CMD42 compatibility, write-protect, and workflow/HIL isolation contracts."""
from pathlib import Path
from typing import Any
import importlib.util
import json
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]
CHECKS = ROOT / "qa/checks/firmware"
PHYSICAL = {
    "SD IMPORT MENU REAL-CARD SCAN/BACK OWNER OK",
    "SD MEDIA READ-WRITE-DELETE OK",
    "SD PHYSICAL SPI/FAT32 READ-WRITE VERIFIED",
}


def workflow_runner() -> Any:
    sys.path.insert(0, str(CHECKS))
    try:
        spec = importlib.util.spec_from_file_location("workflow_runner_contract", CHECKS / "run_workflow_tests.py")
        assert spec is not None and spec.loader is not None
        module: Any = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module
    finally:
        sys.path.pop(0)


class SdWorkflowMarkerTests(unittest.TestCase):

    def test_hil_without_a_card_waives_exactly_the_physical_sd_tranche(self) -> None:
        module = workflow_runner()
        waiver = module.hil_sd_waiver()
        self.assertEqual(set(waiver.waived), PHYSICAL)
        firmware = ROOT / "apps/kaskold-hardware/src/runtime/workflow_tests/connected/sd_media.rs"
        self.assertIn(waiver.marker, firmware.read_text(encoding="utf-8"))
        with tempfile.TemporaryDirectory() as temporary:
            log = Path(temporary) / "uart.log"
            self.assertEqual(module.physical_sd_tranche(log), "run")
            log.write_text(f"boot\n{waiver.marker}; INSERT A DISPOSABLE QA CARD TO RUN IT\n", encoding="utf-8")
            self.assertEqual(module.physical_sd_tranche(log), "skipped-no-card")
            log.write_text("KASKOLD_WORKFLOW_HIL: PHYSICAL SD TRANCHE ENABLED ON THE REAL CARD\n", encoding="utf-8")
            self.assertEqual(module.physical_sd_tranche(log), "run")

    def test_every_tranche_refreshes_the_deadline_and_hil_also_allows_a_forced_erase(self) -> None:
        module = workflow_runner()
        tranche = {module.CONNECTED_TRANCHE_DEADLINE_MARKER: module.CONNECTED_TRANCHE_WINDOW_SECONDS}
        self.assertEqual(module.deadline_extensions(hil=False), tranche)
        self.assertEqual(
            module.deadline_extensions(hil=True),
            {**tranche, module.LOCKED_SD_ERASE_MARKER: module.LOCKED_SD_RECOVERY_WINDOW_SECONDS},
        )

    def test_workflow_e2e_excludes_physical_sd_hil_markers_but_hil_requires_them(self) -> None:
        scenarios_path = ROOT / "qa/config/workflow/production_e2e_scenarios.json"
        scenarios = {entry["id"]: entry for entry in json.loads(scenarios_path.read_text())["scenarios"]}
        physical = PHYSICAL
        declared = set(scenarios["connected-sd-browser-navigation"]["hil_only_serial_markers"])
        declared.update(scenarios["connected-sd-backup-crypto-overwrite"]["hil_only_serial_markers"])
        self.assertEqual(declared, physical)

        module = workflow_runner()
        normal = set(module.required_connected_markers())
        hil = set(module.required_connected_markers(include_hil_only=True))
        self.assertTrue(physical.isdisjoint(normal))
        self.assertTrue(physical.issubset(hil))
        self.assertEqual(hil - normal, physical)
        runner = (CHECKS / "run_workflow_tests.py").read_text()
        self.assertIn("required_runtime_evidence_markers(", runner)
        self.assertIn("args.resume_from, args.board, hil=args.hil", runner)

        coverage = (CHECKS / "production_e2e_coverage.py").read_text()
        self.assertIn("hil_only_serial_markers must be a subset of serial_markers", coverage)


if __name__ == "__main__":
    unittest.main()
