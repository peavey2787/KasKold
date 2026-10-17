
import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[3] / "qa/checks"))
from portal_source import kaskold_source, portal_path  # noqa: E402
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]


def read(relative: str) -> str:
    return kaskold_source(str(relative)).read_text(errors="replace")


class RestoredFeatureCrapCoverageTests(unittest.TestCase):
    def test_backup_header_and_private_swap_claim_are_decomposed_without_policy_changes(self):
        backup = read("apps/kaskold-hardware/src/services/backup/container.rs")
        framing = read("crates/offline-signer/src/crypto/container_framing.rs")
        claim = read("crates/offline-signer/src/transaction/private_swap.rs")
        self.assertIn("container_framing::parse_backup_header", backup)
        self.assertIn("fn derive_reader_key", backup)
        for helper in (
            "parse_current_backup_header",
            "parse_backup_kind", "parse_backup_len", "copy_nonzero",
        ):
            self.assertIn(f"fn {helper}", framing)
        for helper in (
            "validate_claim_transaction_shape", "validate_claim_policy", "validate_claim_output",
        ):
            self.assertIn(f"fn {helper}", claim)

    def test_restored_crypto_paths_have_native_unit_coverage_outside_production_watcher_files(self):
        required = {
            "crates/shared-signer/src/covenant_sign/private_swap/unit_tests/mod.rs": (
                "every_private_swap_request_kind_roundtrips", "every_private_swap_response_kind_roundtrips",
            ),
            "crates/offline-signer/src/transaction/private_swap/unit_tests/mod.rs": (
                "canonical_private_swap_script_roundtrips", "claim_sighash_enforces_transaction",
            ),
            "crates/online-watcher/src/protocol/private_swap/adaptor/unit_tests/mod.rs": (
                "public_adaptor_math_verifies_completes_extracts", "adaptor_verifiers_fail_closed",
            ),
            "crates/online-watcher/src/wasm_api/contracts/covenant/families/oracle_v1/unit_tests/mod.rs": (
                "oracle_attestation_decode_and_redeem_binding", "oracle_builder_rejects",
            ),
            "crates/online-watcher/src/transaction_builder/zk/unit_tests/mod.rs": (
                "crowdfund_helpers_have_host_native_coverage", "fetch_contributions(&[], \"unused\")",
            ),
            "crates/online-watcher/src/transaction_builder/covenant/unit_tests/mod.rs": (
                "allowance_prepare_material_has_host_native_coverage", "prepare_material(",
            ),
            "crates/online-watcher/src/wasm_api/protocol/covenant_sign/unit_tests/mod.rs": (
                "covenant_anti_klepto_host_verifier", "verify_covenant_anti_klepto_string",
            ),
        }
        for relative, terms in required.items():
            source = read(relative)
            for term in terms:
                self.assertIn(term, source, f"{relative}: {term}")

    def test_private_swap_script_int_vectors_distinguish_op_zero_from_pushed_zero(self):
        number_tests = portal_path("contract/script/unit-tests/mod.rs").read_text()
        recognizer = portal_path("contract/covenant/script/private_swap.rs").read_text()
        self.assertIn("next_script_item(&[0x00], 0).unwrap(),\n        (1, ScriptItem::Integer(0)),", number_tests)
        self.assertIn("crate::contract::script::element_integer(elements.get(32)?)?", recognizer)
        self.assertIn("(rebuilt == script).then_some(fields)", recognizer)

    def test_residual_restoration_crap_warnings_are_decomposed_or_directly_covered(self):
        oracle = read("crates/online-watcher/src/contracts/covenant/oracle_v1.rs")
        oracle_wasm = read("crates/online-watcher/src/wasm_api/contracts/covenant/families/oracle_v1.rs")
        private_wire = read("crates/shared-signer/src/covenant_sign/private_swap.rs")
        private_validation = read("crates/shared-signer/src/covenant_sign/private_swap/validation.rs")
        covenant = read("crates/shared-signer/src/covenant_sign/script_int.rs")
        claim = read("crates/offline-signer/src/transaction/private_swap.rs")
        for term in ("fn checked_inputs", "fn bound_statement"):
            self.assertIn(term, oracle)
        self.assertIn("crate::contracts::covenant::oracle_v1::build_json", oracle_wasm)
        self.assertNotIn("fn checked_inputs", oracle_wasm)
        self.assertNotIn("fn bound_statement", oracle_wasm)
        for term in ("fn validate_request_prefix", "fn validate_request_payload_length", "fn parse_bool"):
            self.assertIn(term, private_validation)
        # Portal's one canonical script-integer reader and encoder.
        for term in ("fn push_int", "fn element_integer", "fn next_script_item"):
            self.assertIn(term, covenant)
        self.assertIn("crate::contract::covenant::private_swap_script_fields(script)", claim)
        for term in ("fn validate_claim_transaction_shape", "fn validate_claim_policy", "fn validate_claim_output"):
            self.assertIn(term, claim)


if __name__ == "__main__":
    unittest.main()
