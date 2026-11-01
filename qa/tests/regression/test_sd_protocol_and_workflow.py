#!/usr/bin/env python3
"""CMD42 compatibility, write-protect, and workflow/HIL isolation contracts."""
from pathlib import Path
import importlib.util
import json
import sys
import unittest

ROOT = Path(__file__).resolve().parents[3]


class SdProtocolAndWorkflowTests(unittest.TestCase):


    def test_workflow_e2e_excludes_physical_sd_hil_markers_but_hil_requires_them(self) -> None:
        scenarios_path = ROOT / "qa/config/workflow/production_e2e_scenarios.json"
        scenarios = {entry["id"]: entry for entry in json.loads(scenarios_path.read_text())["scenarios"]}
        physical = {
            "SD IMPORT MENU REAL-CARD SCAN/BACK OWNER OK",
            "SD MEDIA READ-WRITE-DELETE OK",
            "SD PHYSICAL SPI/FAT32 READ-WRITE VERIFIED",
        }
        declared = set(scenarios["connected-sd-browser-navigation"]["hil_only_serial_markers"])
        declared.update(scenarios["connected-sd-backup-crypto-overwrite"]["hil_only_serial_markers"])
        self.assertEqual(declared, physical)

        checks = ROOT / "qa/checks/firmware"
        sys.path.insert(0, str(checks))
        try:
            spec = importlib.util.spec_from_file_location("workflow_runner_contract", checks / "run_workflow_tests.py")
            self.assertIsNotNone(spec)
            self.assertIsNotNone(spec.loader)
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
        finally:
            sys.path.pop(0)

        normal = set(module.required_connected_markers())
        hil = set(module.required_connected_markers(include_hil_only=True))
        self.assertTrue(physical.isdisjoint(normal))
        self.assertTrue(physical.issubset(hil))
        self.assertEqual(hil - normal, physical)
        runner = (checks / "run_workflow_tests.py").read_text()
        self.assertIn("required_runtime_evidence_markers(", runner)
        self.assertIn("args.resume_from, args.board, hil=args.hil", runner)

        coverage = (checks / "production_e2e_coverage.py").read_text()
        self.assertIn("hil_only_serial_markers must be a subset of serial_markers", coverage)


if __name__ == "__main__":
    unittest.main()
