import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "qa/checks"))
from portal_source import portal_text  # noqa: E402


class OnlineWatcherTransactionFixturePolicyTests(unittest.TestCase):
    def test_standard_send_fixture_preserves_non_dust_change(self) -> None:
        source = portal_text("transaction/builder/unit-tests/mod.rs")
        test = source.split(
            "fn standard_send_preparation_and_utxo_paths_are_host_testable()", 1
        )[1].split("\n#[test]", 1)[0]
        self.assertIn("utxo(0xb2, 1, 40_000_000)", test)
        self.assertNotIn("utxo(0xb2, 1, 30_000_000)", test)


if __name__ == "__main__":
    unittest.main()
