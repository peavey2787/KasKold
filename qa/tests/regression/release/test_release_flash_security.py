#!/usr/bin/env python3
"""release-qualification, normal-flash, and security-evidence regressions."""
from __future__ import annotations

import importlib.util
import json
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
COVERAGE_PATH = ROOT / "qa/checks/firmware/production_e2e_coverage.py"


def load_coverage():
    spec = importlib.util.spec_from_file_location("production_e2e_coverage_v612", COVERAGE_PATH)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


class ReleaseFlashSecurityTests(unittest.TestCase):
    def test_release_inventory_is_closed_without_lowering_connected_threshold(self) -> None:
        coverage = load_coverage()
        manifest, errors = coverage.build_manifest(ROOT)
        self.assertEqual(errors, [])
        self.assertEqual(coverage.LEVELS["connected"], 2)
        self.assertEqual(manifest["item_coverage_summary"]["total"], 501)
        self.assertEqual(manifest["item_coverage_summary"]["backlog"], 0)
        self.assertGreaterEqual(manifest["coverage_summary"]["states"]["total"], 172)
        self.assertGreaterEqual(manifest["coverage_summary"]["menu_items"]["total"], 58)
        self.assertGreaterEqual(manifest["coverage_summary"]["actions"]["total"], 197)
        self.assertEqual(manifest["release_qualification"]["incomplete_item_count"], 0)
        self.assertTrue(manifest["release_qualification"]["ready"])
        self.assertEqual(coverage.release_qualification_errors(manifest), [])

    def test_workflow_test_controller_is_not_production_surface(self) -> None:
        coverage = load_coverage()
        actions = set(coverage.production_actions(ROOT))
        self.assertFalse(any("controllers/workflow_tests.rs::" in action for action in actions))
        self.assertEqual(len(coverage.EXCLUDED_BASELINE_ACTIONS), 5)
        self.assertTrue(coverage.EXCLUDED_BASELINE_ACTIONS.isdisjoint(actions))

    def test_firmware_update_action_remains_explicitly_owned(self) -> None:
        coverage = load_coverage()
        manifest, errors = coverage.build_manifest(ROOT)
        self.assertEqual(errors, [])
        retired = "apps/kaskold-hardware/src/runtime/interactions/stego/mode.rs::handle_firmware_update"
        self.assertNotIn(retired, manifest["surface"]["actions"])
        action = "apps/kaskold-hardware/src/runtime/interactions/settings/mod.rs::handle_advanced_navigation"
        entry = manifest["surface"]["actions"][action]
        self.assertIn("connected-firmware-update-usb-guidance", entry["implemented_scenarios"])
        self.assertGreaterEqual(coverage.LEVELS[entry["highest_level"]], coverage.LEVELS["connected"])


    def test_qa_exhaustiveness_scenario_owns_every_current_production_surface(self) -> None:
        coverage = load_coverage()
        surface = coverage.discover_surface(ROOT)
        scenarios = json.loads((ROOT / coverage.SCENARIOS).read_text())["scenarios"]
        scenario = next(item for item in scenarios if item["id"] == "qa-production-exhaustiveness-gate")
        self.assertEqual(set(scenario["states"]), set(surface["states"]))
        self.assertEqual(set(scenario["menu_items"]), set(surface["menu_items"]))
        self.assertEqual(set(scenario["actions"]), set(surface["actions"]))
        self.assertEqual(set(scenario["items"]), {f"E2E-099-{index:02d}" for index in range(1, 6)})


if __name__ == "__main__":
    unittest.main()
