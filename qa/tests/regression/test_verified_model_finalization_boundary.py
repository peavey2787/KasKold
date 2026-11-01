from pathlib import Path
import tomllib
import unittest

from qa.checks.portal_source import portal_root

ROOT = Path(__file__).resolve().parents[3]


class VerifiedModelFinalizationBoundaryTests(unittest.TestCase):

    def test_new_security_boundary_is_in_mutation_scope(self):
        # The verify-once pipeline and KSPT grammar live in Kaspa Portal, whose
        # mutation gate requires every viable mutant to be caught.
        portal = tomllib.loads((portal_root().parent / ".cargo/mutants.toml").read_text(encoding="utf-8"))
        self.assertIn("src/transaction/interchange/pskt/**/*.rs", portal["examine_globs"])
        self.assertIn("src/transaction/interchange/kspt/**/*.rs", portal["examine_globs"])
        local = tomllib.loads((ROOT / ".cargo/mutants.toml").read_text(encoding="utf-8"))
        self.assertIn("crates/online-watcher/src/protocol/**/*.rs", local["examine_globs"])


if __name__ == "__main__":
    unittest.main()
