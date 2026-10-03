import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[3] / "qa/checks"))
from portal_source import kaskold_source  # noqa: E402

from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[3]
NUMBER = kaskold_source("crates/online-watcher/src/protocol/script/number.rs")
TESTS = kaskold_source("crates/online-watcher/src/protocol/script/unit_tests/mod.rs")


class OnlineWatcherScriptLocktimePolicyTests(unittest.TestCase):
    def test_scanner_distinguishes_data_pushes_from_locktime_integers(self) -> None:
        source = NUMBER.read_text()
        self.assertIn("enum ScriptItem", source)
        self.assertIn("OversizedInteger", source)
        self.assertIn("previous = item", source)
        self.assertNotIn("if value.is_some()", source)

    def test_realistic_pubkey_and_salt_pushes_are_regression_tested(self) -> None:
        source = TESTS.read_text()
        self.assertIn("locktime_extractors_skip_non_integer_data_pushes", source)
        self.assertIn("[0x11; 32]", source)
        self.assertIn("[0x22; 8]", source)
        self.assertIn("immediately_precede_the_opcode", source)
        self.assertIn("locktime_extractors_handle_real_covenant_scripts", source)
        self.assertIn("build_global_allowance_script", source)
        self.assertIn("build_global_spending_limit_script", source)


if __name__ == "__main__":
    unittest.main()
