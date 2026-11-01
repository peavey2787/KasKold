#!/usr/bin/env python3
"""Regression contracts for the developer-only firmware workflow E2E harness."""
from __future__ import annotations

import json
import sys
from pathlib import Path
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))
sys.path.insert(0, str(ROOT / "tools/build/firmware"))
sys.path.insert(0, str(ROOT / "qa/checks/firmware"))
import run_workflow_tests as workflow_runner  # noqa: E402
FW = ROOT / "apps/kaskold-hardware"


class WorkflowRunnerHelperTests(unittest.TestCase):
    def test_receive_resume_alias_selects_last_connected_tranche(self) -> None:
        self.assertEqual(workflow_runner.RESUME_ALIASES["receive"], 11)
        self.assertEqual(workflow_runner.CONNECTED_TRANCHES[-1], "RECEIVE")
        self.assertEqual(workflow_runner.parse_resume_from("receive"), 11)
        self.assertEqual(workflow_runner.parse_resume_from("11"), 11)

    def test_parse_resume_from_accepts_digits_and_every_alias(self) -> None:
        self.assertEqual(workflow_runner.parse_resume_from(" Root "), 1)
        self.assertEqual(workflow_runner.parse_resume_from("1"), 1)
        for alias in ("qr", "qr-protocol", "qr_protocol"):
            self.assertEqual(workflow_runner.parse_resume_from(alias), 5, alias)
        for alias in ("sd", "sd-workflows", "sd_workflows"):
            self.assertEqual(workflow_runner.parse_resume_from(alias), 6, alias)
        for alias in ("security", "security-policies", "security_policies"):
            self.assertEqual(workflow_runner.parse_resume_from(alias), 9, alias)

    def test_parse_resume_from_rejects_out_of_range_and_unknown_values(self) -> None:
        for value in ("0", "12", "bogus", ""):
            with self.subTest(value=value), self.assertRaises(ValueError):
                workflow_runner.parse_resume_from(value)

    def test_required_connected_markers_are_sorted_and_exclude_hil_only_by_default(self) -> None:
        document = json.loads((ROOT / "qa/config/workflow/production_e2e_scenarios.json").read_text(encoding="utf-8"))
        connected = [
            scenario for scenario in document["scenarios"]
            if scenario.get("status") == "implemented" and scenario.get("level") == "connected"
        ]
        hil_only = {m for s in connected for m in s.get("hil_only_serial_markers", [])}
        all_markers = {m for s in connected for m in s.get("serial_markers", [])}
        self.assertTrue(hil_only, "fixture expects HIL-only markers in the connected scenarios")

        default = workflow_runner.required_connected_markers()
        self.assertEqual(list(default), sorted(default))
        self.assertEqual(set(default), all_markers - hil_only)
        self.assertEqual(set(workflow_runner.required_connected_markers(include_hil_only=True)), all_markers)

    def test_firmware_package_version_matches_cargo_manifest(self) -> None:
        manifest = tomllib.loads((FW / "Cargo.toml").read_text(encoding="utf-8"))
        self.assertEqual(workflow_runner.firmware_package_version(), manifest["package"]["version"])
        self.assertRegex(workflow_runner.firmware_package_version(), r"^\d+\.\d+\.\d+")

    def test_required_runtime_markers_resume_semantics(self) -> None:
        total = len(workflow_runner.CONNECTED_TRANCHES)
        self.assertEqual(
            workflow_runner.required_runtime_markers(1),
            workflow_runner.required_connected_markers(include_hil_only=False),
        )
        self.assertEqual(
            workflow_runner.required_runtime_markers(11),
            (f"KASKOLD_WORKFLOW_TESTS: CONNECTED TRANCHE 11/{total} RECEIVE PASS",),
        )
        resumed = workflow_runner.required_runtime_markers(3)
        self.assertEqual(len(resumed), total - 2)
        self.assertEqual(resumed[0], f"KASKOLD_WORKFLOW_TESTS: CONNECTED TRANCHE 3/{total} ONBOARDING PASS")
        self.assertFalse(any("CONNECTED TRANCHE 2/" in marker for marker in resumed))

    def test_required_physical_render_states_only_for_m5stack_from_root(self) -> None:
        document = json.loads((ROOT / "qa/config/workflow/production_runtime_qualification.json").read_text(encoding="utf-8"))
        expected = tuple(document["workflow_e2e_physical_render_states"])
        self.assertTrue(expected)
        self.assertEqual(workflow_runner.required_physical_render_states(1, "m5stack"), expected)
        self.assertEqual(workflow_runner.required_physical_render_states(1, "m5stack-lite"), expected)
        self.assertEqual(workflow_runner.required_physical_render_states(2, "m5stack"), ())
        self.assertEqual(workflow_runner.required_physical_render_states(1, "other-board"), ())


if __name__ == "__main__":
    unittest.main()
