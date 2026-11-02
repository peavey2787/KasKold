
import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[3] / "qa/checks"))
import json
import sys
import unittest
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[3]
CHECK_DIR = ROOT / "qa/checks/quality/crap"
sys.path.insert(0, str(CHECK_DIR))
from regression import compare_coverage_manifests, compare_health_summaries  # noqa: E402

class BranchCoverage90ContractTests(unittest.TestCase):
    def test_rust_host_and_critical_domains_require_ninety_percent_branches(self):
        policy = json.loads((ROOT / "qa/checks/quality/crap/policy.json").read_text())["health"]
        self.assertEqual(policy["minimum_host_line_coverage_percent"], 90.0)
        self.assertEqual(policy["minimum_host_function_coverage_percent"], 90.0)
        self.assertEqual(policy["minimum_host_branch_coverage_percent"], 90.0)
        for name, domain in policy["critical_domains"].items():
            self.assertGreaterEqual(domain["minimum_branch_coverage_percent"], 90.0, name)
        crypto = policy["critical_domains"]["critical_crypto"]
        self.assertEqual(crypto["target_branch_coverage_percent"], 100.0)


    def test_existing_thresholds_are_not_lowered(self) -> None:
        policy = json.loads((ROOT / "qa/checks/quality/crap/policy.json").read_text())
        ratchet = json.loads((ROOT / "qa/contracts/quality/crap_ratchets.json").read_text())
        self.assertEqual(policy["regression"]["coverage_drop_tolerance_percent"], 0.05)
        self.assertEqual(policy["health"]["minimum_host_branch_coverage_percent"], 90.0)
        self.assertEqual(ratchet["host_production_minimum_percent"]["lines"], 95.9701)
        self.assertEqual(ratchet["host_production_minimum_percent"]["functions"], 90.6473)
        self.assertEqual(ratchet["host_production_minimum_percent"]["branches"], 91.4141)
        self.assertIs(ratchet["coverage_profile"]["branch_instrumentation"], True)

    def test_raw_manifest_keeps_branch_instrumentation_contract_only(self) -> None:
        profile = {"dev_opt_level": "0", "test_opt_level": "0", "branch_instrumentation": True}
        previous: dict[str, Any] = {
            "coverage_profile": profile,
            "branch_coverage_requested": True,
            "coverage": {"branches": {"available": True, "percent": 95.0}},
        }
        current: dict[str, Any] = {
            "coverage_profile": profile,
            "branch_coverage_requested": True,
            "coverage": {"branches": {"available": True, "percent": 50.0}},
        }
        self.assertEqual(compare_coverage_manifests(previous, current, {}), [])
        current["coverage"]["branches"]["available"] = False
        self.assertIn("branch coverage disappeared", compare_coverage_manifests(previous, current, {})[0])

    def test_classified_host_production_metric_is_the_regression_ratchet(self) -> None:
        profile = {"dev_opt_level": "0", "test_opt_level": "0", "branch_instrumentation": True}
        run = {"coverage_profile": profile}
        previous = {
            "host_metrics": {
                "lines": {"percent": 96.0},
                "functions": {"percent": 91.0},
                "branches": {"percent": 91.4141},
            }
        }
        current = {
            "host_metrics": {
                "lines": {"percent": 96.0},
                "functions": {"percent": 91.0},
                "branches": {"percent": 91.30},
            }
        }
        errors = compare_health_summaries(
            previous,
            current,
            run,
            run,
            {"coverage_drop_tolerance_percent": 0.05},
        )
        self.assertEqual(len(errors), 1)
        self.assertIn("host production branches coverage regressed", errors[0])
        current["host_metrics"]["branches"]["percent"] = 91.37
        self.assertEqual(
            compare_health_summaries(
                previous,
                current,
                run,
                run,
                {"coverage_drop_tolerance_percent": 0.05},
            ),
            [],
        )


if __name__ == "__main__":
    unittest.main()
