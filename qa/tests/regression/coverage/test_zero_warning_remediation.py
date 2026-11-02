
import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[4] / "qa/checks"))
import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "qa/checks/quality/crap"))
from source_complexity import production_records  # noqa: E402


class ZeroWarningRemediationTests(unittest.TestCase):
    def test_source_complexity_warning_ceiling_is_zero(self):
        policy = json.loads((ROOT / "qa/checks/quality/crap/policy.json").read_text())
        source_policy = policy["source_complexity"]
        self.assertEqual(source_policy["warning_source_decisions"], 15)
        self.assertEqual(source_policy["maximum_warning_functions"], 0)
        warnings = [
            record for record in production_records(ROOT)
            if record.decisions > source_policy["warning_source_decisions"]
        ]
        self.assertEqual(warnings, [], "production source-complexity warnings must stay at zero")


    def test_security_sensitive_crap_targets_remain_decomposed(self):
        records = production_records(ROOT)
        targets = {
            ("crates/online-watcher/src/facade.rs", "broadcast"),
        }
        found = {(record.path, record.name): record.decisions for record in records if (record.path, record.name) in targets}
        self.assertEqual(set(found), targets)
        self.assertTrue(all(decisions <= 15 for decisions in found.values()), found)


if __name__ == "__main__":
    unittest.main()
