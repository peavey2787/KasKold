from pathlib import Path
import tomllib
import unittest

from qa.checks.portal_source import portal_root

ROOT = Path(__file__).resolve().parents[3]


class VerifiedModelFinalizationBoundaryTests(unittest.TestCase):
    def test_parser_differential_is_symmetric_and_checks_verified_materialization(self):
        fuzz = (ROOT / "qa/fuzz/pskt_parser_differential.rs").read_text(encoding="utf-8")
        self.assertIn("assert_eq!(", fuzz)
        self.assertIn("host_result.is_ok()", fuzz)
        self.assertIn("vault_ok", fuzz)
        self.assertIn("assert_same_semantic_model", fuzz)
        self.assertIn("assert_verified_materialization_matches", fuzz)
        self.assertIn("materialize_signature_script", fuzz)

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
