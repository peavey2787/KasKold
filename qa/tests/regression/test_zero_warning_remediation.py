
import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[3] / "qa/checks"))
from portal_source import kaskold_source, module_text  # noqa: E402
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
        review_input = kaskold_source("crates/online-watcher/src/protocol/pskt/review/input.rs").read_text()
        self.assertIn("u32::try_from", review_input)

    def test_offline_parser_result_contracts_compile_cleanly(self):
        input_details = kaskold_source("crates/offline-signer/src/transaction/std_pskt/parser/inputs/details.rs").read_text()
        outputs = kaskold_source("crates/offline-signer/src/transaction/std_pskt/parser/outputs.rs").read_text()
        script = kaskold_source("crates/offline-signer/src/transaction/model/script.rs").read_text()
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





    def test_native_online_watcher_has_no_browser_transport_on_host(self):
        # The wRPC codec and browser transport are Kaspa Portal's; native
        # Companion builds must fail closed instead of opening sockets.
        network = (ROOT / "crates/online-watcher/src/network/mod.rs").read_text()
        self.assertIn('#[cfg(target_arch = "wasm32")]', network)
        self.assertIn('#[cfg(not(target_arch = "wasm32"))]', network)
        self.assertIn("unavailable on native hosts", network)
        self.assertNotIn("#[allow(dead_code)]", network)

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
        signed = (ROOT / "crates/online-watcher/src/facade.rs").read_text()
        signed_tests = kaskold_source("crates/online-watcher/src/protocol/pskt/unit_tests/consensus_finalizer.rs").read_text()

        canonical_decode = module_text("crates/kaskold-protocol/src/wire/kspt/decode.rs")
        canonical_tests = kaskold_source("crates/kaskold-protocol/src/unit_tests/kspt_wire/mod.rs").read_text()
        self.assertIn("verify_complete_kspt(&bytes)", signed)
        self.assertIn("placeholder_signatures_and_unsupported_versions_never_reach_consensus_bytes", signed_tests)
        self.assertIn("fn read_global", canonical_decode)
        self.assertIn("canonical_codec_round_trips_every_v4_trailer", canonical_tests)

        records = production_records(ROOT)
        targets = {
            ("crates/online-watcher/src/facade.rs", "broadcast"),
        }
        found = {(record.path, record.name): record.decisions for record in records if (record.path, record.name) in targets}
        self.assertEqual(set(found), targets)
        self.assertTrue(all(decisions <= 10 for decisions in found.values()), found)

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
