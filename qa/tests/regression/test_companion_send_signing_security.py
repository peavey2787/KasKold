
import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[3] / "qa/checks"))
from portal_source import kaskold_source  # noqa: E402
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
FW = ROOT / "apps/kaskold-hardware"


def read(relative: str) -> str:
    return kaskold_source(str(relative)).read_text(errors="strict")


class CompanionSendSigningSecurityTests(unittest.TestCase):
    def test_companion_review_defaults_to_simple_summary_with_optional_inspection(self):
        html = read("apps/kaskold-companion-web/web/html/screens/transactions/pskt_review.html")
        js = read("apps/kaskold-companion-web/web/js/features/transactions/pskt_multisig/review.js")
        self.assertIn('id="pskt-to-address"', html)
        self.assertIn('id="pskt-send-total"', html)
        self.assertIn('id="pskt-fee"', html)
        self.assertIn('id="pskt-change-total"', html)
        self.assertIn('id="btn-pskt-inspect"', html)
        self.assertIn('id="pskt-inspect-details" class="pskt-inspect-details hidden"', html)
        self.assertIn('>UTXOs</h3>', html)
        self.assertIn("toAddressEl.textContent = externalOutputs[0].address", js)
        self.assertIn("inspectDetails.classList.add('hidden')", js)
        self.assertIn("inspectButton.setAttribute('aria-expanded', 'false')", js)
        self.assertIn('class="card pskt-review-card"', html)

    def test_review_always_offers_a_signing_action_until_ready(self):
        js = read("apps/kaskold-companion-web/web/js/features/transactions/pskt_multisig/review.js")
        relay = read("apps/kaskold-companion-web/web/js/features/transactions/pskt_multisig/review_relay.js")
        self.assertIn("relayButton.textContent = needsMultisigRelay ? 'Relay to next signer' : 'Send to signer'", js)
        self.assertIn("relayButton.classList.toggle('hidden', summary.finalize_ready)", js)
        self.assertIn("btnFinalize.classList.toggle('hidden', !summary.finalize_ready)", js)
        self.assertIn("const missingInputs = inputs.filter", js)
        self.assertIn("actionLabel === 'Send to signer'", relay)

    def test_review_secondary_hover_matches_navigation_theme(self):
        css = read("apps/kaskold-companion-web/web/css/app/components/buttons.css")
        pskt_css = read("apps/kaskold-companion-web/web/css/app/screens/pskt.css")
        self.assertIn(".btn-secondary:hover", css)
        self.assertIn("border-color: var(--teal);", css)
        self.assertIn("background: var(--teal-glow);", css)
        self.assertIn(".pskt-review-card::before", pskt_css)

    def test_multisig_review_binds_local_destination_and_keeps_change_out_of_send_total(self):
        review = read("apps/kaskold-companion-web/web/js/features/transactions/pskt_multisig/review.js")
        create = read("apps/kaskold-companion-web/web/js/features/transactions/pskt_multisig/multisig.js")
        state = read("apps/kaskold-companion-web/web/js/app/state/core/transaction_state.js")
        self.assertIn("function psktBodyKey(summary)", review)
        self.assertIn("context?.bodyKey === bodyKey", review)
        self.assertIn("reviewContext?.kind === 'multisig-send'", review)
        self.assertIn("return 'MULTISIG CHANGE'", review)
        self.assertIn("ownership === 'CHANGE' || ownership === 'MULTISIG CHANGE'", review)
        self.assertIn("{ kind: 'multisig-send', destinationAddress: resolvedDest }", create)
        self.assertIn("'_psktReviewContext': null", state)

    def test_change_hint_mismatch_is_fail_closed_not_external_spend(self):
        review = read("apps/kaskold-hardware/src/runtime/signing/review.rs")
        anti = read("apps/kaskold-hardware/src/runtime/interactions/camera_loop/dispatch/anti_klepto.rs")
        errors = read("apps/kaskold-hardware/src/runtime/presentation/errors.rs")
        self.assertIn("Result<(), &'static str>", review)
        self.assertIn('return Err("Transaction output does not belong to active wallet")', review)
        self.assertIn("verify_transaction_output_ownership_with_checkpoint", review)
        self.assertIn("TX ownership hint output={}", review)
        self.assertIn("trusted_multisig_output_chain", review)
        self.assertIn("TX trusted multisig ownership output={} chain={}", review)
        self.assertIn("reject_ownership(ad, message)", anti)
        self.assertIn('code: "TX-OWN-01"', errors)

    def test_transaction_signing_does_not_rebuild_ui_address_cache(self):
        workflow = read("apps/kaskold-hardware/src/runtime/signing/workflow.rs")
        self.assertNotIn("ensure_pubkeys(ad, boot_display)", workflow)
        self.assertNotIn('draw_saving_screen("Deriving addresses...")', workflow)
        self.assertNotIn("populate_active_pubkeys", workflow)

    def test_signing_address_match_has_watchdog_checkpoints(self):
        lookup = read("crates/offline-signer/src/derivation/bip32/address_lookup.rs")
        signing = read("crates/offline-signer/src/transaction/kspt/signing/multi_address.rs")
        firmware = read("apps/kaskold-hardware/src/runtime/signing/kspt.rs")
        derivation = read("apps/kaskold-hardware/src/runtime/signing/derivation.rs")
        self.assertIn("find_address_index_for_pubkey_with_checkpoint", lookup)
        self.assertIn("checkpoint();", lookup)
        self.assertIn("sign_account_input_with_entropy_checkpointed", signing)
        self.assertIn("sign_account_input_with_entropy_checkpointed", firmware)
        self.assertIn("AccountKeyDerivation::new", derivation)
        self.assertIn("while !derivation.is_complete()", derivation)
        self.assertIn("derivation.advance_one()", derivation)

    def test_anti_klepto_reveal_keeps_watchdog_alive_during_key_matching(self):
        camera = read("apps/kaskold-hardware/src/runtime/event_loop/camera.rs")
        dispatch = read("apps/kaskold-hardware/src/runtime/interactions/camera_loop/dispatch/anti_klepto.rs")
        finalization = read("crates/offline-signer/src/transaction/kspt/signing/anti_klepto/finalization.rs")
        context = read("crates/offline-signer/src/transaction/kspt/signing/context.rs")
        table = read("crates/offline-signer/src/derivation/bip32/address_lookup.rs")
        self.assertIn("$cam_status:ident, $watchdog_feed:ident", camera)
        self.assertIn("finalize_reveal_signature_with_checkpoint", dispatch)
        self.assertIn("finalize_account_signatures_with_checkpoint", finalization)
        self.assertIn("matching_material_with_checkpoint", context)
        self.assertIn("AddrPubkeyTable::build_with_checkpoint", context)
        self.assertIn("pub fn build_with_checkpoint", table)

    def test_generated_web_bundle_contains_new_review_contract(self):
        index = read("apps/kaskold-companion-web/web/index.html")
        css = read("apps/kaskold-companion-web/web/css/app.css")
        self.assertIn('id="btn-pskt-inspect"', index)
        self.assertIn('id="pskt-to-address"', index)
        self.assertIn(".pskt-simple-summary", css)
        self.assertIn(".pskt-inspect-details", css)


if __name__ == "__main__":
    unittest.main()
