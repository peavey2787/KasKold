import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
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

    def test_value_narrowing_in_restored_pskt_paths_is_checked(self):
        consensus_input = (ROOT / "crates/online-watcher/src/protocol/pskt/consensus/input.rs").read_text()
        consensus_output = (ROOT / "crates/online-watcher/src/protocol/pskt/consensus/output.rs").read_text()
        review_input = (ROOT / "crates/online-watcher/src/protocol/pskt/review/input.rs").read_text()
        self.assertIn("u32::try_from", consensus_input)
        self.assertIn("u8::try_from", consensus_input)
        self.assertIn("u16::try_from", consensus_output)
        self.assertIn("u32::try_from", review_input)

    def test_offline_parser_result_contracts_compile_cleanly(self):
        input_details = (ROOT / "crates/offline-signer/src/transaction/std_pskt/parser/inputs/details.rs").read_text()
        outputs = (ROOT / "crates/offline-signer/src/transaction/std_pskt/parser/outputs.rs").read_text()
        script = (ROOT / "crates/offline-signer/src/transaction/model/script.rs").read_text()
        self.assertIn(
            "hex_decode_strict(hex_str, &mut self.input.previous_outpoint.transaction_id).map(|_| ())",
            input_details,
        )
        self.assertIn(
            "hex_decode_strict(covenant_id, &mut self.output.covenant_id)\n            .map(|_| ())",
            outputs,
        )
        self.assertIn("let Some((_, n)) = multisig_thresholds(script, len)", script)
        self.assertNotIn("let Some((m, n)) = multisig_thresholds(script, len)", script)




    def test_redeem_boundary_constant_is_test_only_import(self):
        production = (ROOT / "crates/offline-signer/src/transaction/kspt/wire_adapter.rs").read_text()
        tests = (ROOT / "crates/offline-signer/src/transaction/kspt/wire_adapter/unit_tests/mod.rs").read_text()
        production_imports = production.split("use kaskold_protocol", 1)[0]
        self.assertNotIn("MAX_REDEEM_SIZE", production_imports)
        self.assertIn("use crate::transaction::model::MAX_REDEEM_SIZE;", tests)
        self.assertIn("MAX_REDEEM_SIZE + 1", tests)

    def test_native_online_watcher_excludes_wasm_response_only_helpers(self):
        websocket = (ROOT / "crates/online-watcher/src/infrastructure/browser_websocket.rs").read_text()
        wrpc_mod = (ROOT / "crates/online-watcher/src/network/wrpc/mod.rs").read_text()
        operation = (ROOT / "crates/online-watcher/src/network/wrpc/operation.rs").read_text()

        self.assertIn(
            '#[cfg(any(target_arch = "wasm32", test))]\npub(super) fn validate_response(',
            websocket,
        )
        self.assertIn(
            '#[cfg(any(target_arch = "wasm32", test))]\npub(crate) mod error_payload;',
            wrpc_mod,
        )
        self.assertIn(
            '#[cfg(any(target_arch = "wasm32", test))]\npub(crate) mod response;',
            wrpc_mod,
        )
        self.assertIn(
            '#[cfg(any(target_arch = "wasm32", test))]\n    GetSink,',
            operation,
        )
        self.assertIn(
            '#[cfg(any(target_arch = "wasm32", test))]\n    pub const fn from_code',
            operation,
        )
        self.assertNotIn('#[allow(dead_code)]', websocket + wrpc_mod + operation)

    def test_board_specific_display_and_gpio_primitives_live_with_their_board(self):
        shared_display = (ROOT / "apps/kaskold-hardware/src/hw/shared/display.rs").read_text()
        cores3_display = (ROOT / "apps/kaskold-hardware/src/hw/m5stack/display.rs").read_text()
        cores3_power_lines = (ROOT / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/sd_power_lines.rs").read_text()

        self.assertNotIn("SpiInterface", shared_display)
        self.assertNotIn("SPI_BUFFER", shared_display)
        self.assertIn("static SPI_BUFFER", cores3_display)
        self.assertIn("SpiInterface::new", cores3_display)
        self.assertIn("lcd_device(cs_pin)?", cores3_display)
        self.assertIn("GPIO_OUT_W1TS", cores3_power_lines)
        self.assertNotIn("allow(dead_code)", shared_display + cores3_display + cores3_power_lines)
        self.assertEqual({path.name for path in (ROOT / "apps/kaskold-hardware/src/hw").iterdir() if path.is_dir()}, {"m5stack", "shared"})

    def test_final_measured_warning_targets_remain_decomposed_and_covered(self):
        utxo = (ROOT / "crates/online-watcher/src/network/codec/responses/utxo.rs").read_text()
        utxo_tests = (ROOT / "crates/online-watcher/src/network/unit_tests/utxo_response.rs").read_text()
        signed = (ROOT / "crates/online-watcher/src/protocol/transaction/signed_kspt.rs").read_text()
        signed_tests = (ROOT / "crates/online-watcher/src/protocol/transaction/unit_tests/signed_kspt.rs").read_text()

        self.assertIn("fn skip_present_entry_metadata", utxo)
        self.assertIn("utxo_response_skips_present_optional_metadata_and_rejects_truncation", utxo_tests)
        canonical_decode = (ROOT / "crates/kaskold-protocol/src/wire/kspt/decode.rs").read_text()
        canonical_tests = (ROOT / "crates/kaskold-protocol/src/unit_tests/kspt_wire/mod.rs").read_text()
        self.assertIn("kspt::decode(bytes, &mut sink)", signed)
        self.assertIn("signed_kspt_global_fields_cover_payload_and_truncation_boundaries", signed_tests)
        self.assertIn("fn read_global", canonical_decode)
        self.assertIn("canonical_codec_round_trips_every_v4_trailer", canonical_tests)

        records = production_records(ROOT)
        targets = {
            ("crates/online-watcher/src/network/codec/responses/utxo.rs", "skip_entry_metadata"),
            ("crates/online-watcher/src/protocol/transaction/signed_kspt.rs", "decode_signed_kspt"),
        }
        found = {(record.path, record.name): record.decisions for record in records if (record.path, record.name) in targets}
        self.assertEqual(set(found), targets)
        self.assertTrue(all(decisions <= 10 for decisions in found.values()), found)

    def test_security_sensitive_crap_targets_remain_decomposed(self):
        records = production_records(ROOT)
        targets = {
            ("crates/offline-signer/src/transaction/std_pskt/parser/outputs.rs", "parse_covenant_binding"),
            ("crates/offline-signer/src/transaction/std_pskt/parser/inputs/details.rs", "parse_outpoint"),
            ("crates/online-watcher/src/protocol/transaction/signed_kspt.rs", "decode_signed_kspt"),
            ("crates/online-watcher/src/protocol/pskt/consensus/input.rs", "build_consensus_input"),
            ("crates/online-watcher/src/protocol/pskt/consensus/output.rs", "build_consensus_output"),
            ("crates/offline-signer/src/transaction/kspt/wire_adapter.rs", "serialize_compact_kspt"),
        }
        found = {(record.path, record.name): record.decisions for record in records if (record.path, record.name) in targets}
        self.assertEqual(set(found), targets)
        self.assertTrue(all(decisions <= 15 for decisions in found.values()), found)




if __name__ == "__main__":
    unittest.main()
