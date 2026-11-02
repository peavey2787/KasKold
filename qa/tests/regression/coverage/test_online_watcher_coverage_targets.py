import json
from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[4]
MANIFEST = ROOT / "qa/contracts/coverage/online_watcher_coverage_targets.json"


class OnlineWatcherCoverageTargetsTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.document = json.loads(MANIFEST.read_text())

    def test_requested_target_counts_are_encoded(self):
        requirements = self.document["requirements"]
        summary = self.document["summary"]
        self.assertGreaterEqual(
            summary["zero_function_targets"],
            requirements["minimum_zero_function_targets"],
        )
        self.assertGreaterEqual(
            summary["warning_targets"],
            requirements["minimum_warning_targets"],
        )
        self.assertGreaterEqual(
            summary["parser_function_targets"],
            requirements["minimum_parser_function_targets"],
        )
        self.assertGreaterEqual(
            summary["parser_estimated_source_lines"],
            requirements["minimum_parser_line_gain"],
        )
        self.assertGreaterEqual(
            summary["transaction_construction_targets"],
            requirements["minimum_transaction_boundary_targets"],
        )


if __name__ == "__main__":
    unittest.main()
