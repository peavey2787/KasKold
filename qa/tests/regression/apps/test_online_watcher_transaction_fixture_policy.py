import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "qa/checks"))
from portal_source import portal_text  # noqa: E402


class OnlineWatcherTransactionFixturePolicyTests(unittest.TestCase):
    def test_standard_send_fixture_preserves_non_dust_change(self) -> None:
        source = portal_text("transaction/builder/unit_tests/priced.rs")
        test = source.split(
            "fn a_small_automatic_send_pays_the_feerate_on_its_storage_mass()", 1
        )[1].split("\n#[test]", 1)[0]
        # The 0.2 KAS payment is funded by the 91.7 KAS funded-wallet UTXO, so
        # its change stays far above dust whatever the mass fee.
        self.assertIn("const FUNDED_UTXO: u64 = 9_172_200_000;", source)
        self.assertIn("const SMALL_PAYMENT: u64 = 20_000_000;", source)
        self.assertIn("prepared(SMALL_PAYMENT, 300_000)", test)
        self.assertIn("vec![utxo(0xb1, 0, FUNDED_UTXO)]", test)


if __name__ == "__main__":
    unittest.main()
