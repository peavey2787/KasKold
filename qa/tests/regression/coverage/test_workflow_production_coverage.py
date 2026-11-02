#!/usr/bin/env python3
"""Exhaustive production E2E requirements and coverage-ratchet contracts."""
from __future__ import annotations

import sys
import importlib.util
import json
from pathlib import Path
import shutil
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "qa/checks"))
CHECKER = ROOT / "qa/checks/firmware/production_e2e_coverage.py"
SPEC = importlib.util.spec_from_file_location("production_e2e_coverage", CHECKER)
assert SPEC and SPEC.loader
coverage = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(coverage)


class ProductionE2ECoverageRatchetTests(unittest.TestCase):


    def test_connected_surface_evolved_to_release_qualified_inventory(self) -> None:
        manifest = json.loads((ROOT / coverage.MANIFEST).read_text())
        scenarios = {scenario["id"]: scenario for scenario in manifest["scenarios"]}
        for scenario in (
            "connected-home-root",
            "connected-receive-controls",
            "connected-settings-root",
            "connected-display-settings",
            "connected-audio-settings-controller",
        ):
            self.assertEqual(scenarios[scenario]["status"], "implemented")
        self.assertEqual(scenarios["connected-display-settings"]["level"], "hil")
        self.assertEqual(manifest["item_coverage_summary"]["backlog"], 0)
        self.assertEqual(manifest["coverage_summary"]["actions"]["backlog"], 0)
        self.assertEqual(manifest["coverage_summary"]["menu_items"]["backlog"], 0)
        self.assertEqual(manifest["coverage_summary"]["states"]["backlog"], 0)
        self.assertTrue(manifest["release_qualification"]["ready"])

    def test_new_state_menu_or_action_requires_an_implemented_scenario(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            temp = Path(tmp)
            for relative in (
                coverage.REQUIREMENTS_SPEC,
                coverage.BASELINE,
                coverage.SCENARIOS,
                coverage.STATE_FILE,
                coverage.NAV_FILE,
                coverage.NAV_DATA_FILE,
                *coverage.ui_graph.GRAPH_PARTS,
            ):
                target = temp / relative
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(ROOT / relative, target)
            shutil.copytree(ROOT / coverage.CATALOG_DIR, temp / coverage.CATALOG_DIR)
            shutil.copytree(ROOT / coverage.CONTROLLERS, temp / coverage.CONTROLLERS)

            state_path = temp / coverage.STATE_FILE
            state_text = state_path.read_text()
            state_path.write_text(state_text.replace("    StorageModeChoice,", "    FutureProductionScreen,\n    StorageModeChoice,", 1))

            graph_states = temp / coverage.ui_graph.GRAPH_PARTS[1]
            graph_state_text = graph_states.read_text()
            graph_states.write_text(
                graph_state_text.replace(
                    "];",
                    '    ui_state!(FutureProductionScreen, Settings, Screen, "AdvancedMenu", "AdvancedMenu"),\n];',
                    1,
                )
            )

            graph_menus = temp / coverage.ui_graph.GRAPH_PARTS[2]
            graph_menu_text = graph_menus.read_text()
            graph_menu_text = graph_menu_text.replace(
                "pub(crate) const ADVANCED_MENU_ITEMS: [UiMenuItemSpec; 4]",
                "pub(crate) const ADVANCED_MENU_ITEMS: [UiMenuItemSpec; 5]",
                1,
            ).replace(
                '    ui_menu!(AdvancedMenu, 3, "Pop It!", "settings.advanced.pop_it", PopItPrompt, "secure_boot_disabled"),',
                '    ui_menu!(AdvancedMenu, 3, "Pop It!", "settings.advanced.pop_it", PopItPrompt, "secure_boot_disabled"),\n'
                '    ui_menu!(AdvancedMenu, 4, "Future Menu Action", "settings.advanced.future", FutureProductionScreen, "always"),',
                1,
            )
            graph_menus.write_text(graph_menu_text)

            action_path = temp / "apps/kaskold-hardware/src/runtime/interactions/settings/menu.rs"
            action_path.write_text(action_path.read_text() + "\nfn handle_future_action() {}\n")

            _, errors = coverage.build_manifest(temp)
            joined = "\n".join(errors)
            self.assertIn("new production state lacks implemented E2E scenario: FutureProductionScreen", joined)
            self.assertIn("new production menu_item lacks implemented E2E scenario: Future Menu Action", joined)
            self.assertIn("handle_future_action", joined)


if __name__ == "__main__":
    unittest.main()
