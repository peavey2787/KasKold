from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
WEB_JS = ROOT / "apps/kaskold-companion-web/web/js"
BALANCE = WEB_JS / "features/wallet/core/balance.js"
BROADCAST = WEB_JS / "features/transactions/send/broadcast.js"
SIGNED_IMAGE = WEB_JS / "features/transactions/send/signed_qr_image_import.js"
TRANSACTION_EVENTS = WEB_JS / "app/events/transactions/transactions.js"
BROADCAST_HTML = ROOT / "apps/kaskold-companion-web/web/html/screens/transactions/broadcast.html"
BROWSER_WEBSOCKET = ROOT / "crates/online-watcher/src/infrastructure/browser_websocket.rs"
BIP32 = ROOT / "crates/online-watcher/src/account/bip32.rs"
DOM = WEB_JS / "core/dom.js"
SETTINGS_EVENTS = WEB_JS / "app/events/wallet/settings_and_wallet.js"
NAVIGATION = WEB_JS / "app/navigation.js"
KPUB_IMPORT = WEB_JS / "features/wallet/core/kpub_import.js"
KPUB_MANAGER = WEB_JS / "features/wallet/kpub_manager/controller.js"
LOADING_HTML = ROOT / "apps/kaskold-companion-web/web/html/document/close.html"


class CompanionConnectionAndQrImportPolicyTests(unittest.TestCase):
    def test_balance_reconnect_is_background_and_bounded(self) -> None:
        source = BALANCE.read_text()
        self.assertNotIn("showLoading('Connecting...')", source)
        self.assertNotIn("console.error('Balance fetch failed:'", source)
        self.assertIn("const BALANCE_RECONNECT_ATTEMPTS = 3", source)
        self.assertIn("isRetryableNodeError", source)
        self.assertIn("'timeout'", source)
        self.assertIn("noteReconnectAttempt", source)
        self.assertIn("showNodeFailureAfterRetries", source)
        self.assertIn("Last known balance shown", source)

    def test_supplemental_utxo_timeout_does_not_replace_balance(self) -> None:
        source = BALANCE.read_text()
        self.assertIn("History tracking is supplemental", source)
        self.assertIn("setStatus('online', 'Connected');", source)
        self.assertNotIn("UTXO history track:", source)

    def test_websocket_error_handler_does_not_read_optional_message(self) -> None:
        source = BROWSER_WEBSOCKET.read_text()
        self.assertIn("type ErrorHandler = Closure<dyn FnMut(Event)>;", source)
        self.assertNotIn("ErrorEvent", source)
        self.assertNotIn("event.message()", source)
        self.assertIn('ConnectionFailed("WebSocket error".into())', source)

    def test_wallet_derivation_diagnostics_are_removed(self) -> None:
        source = BIP32.read_text()
        self.assertNotIn("Parsed kpub at depth", source)
        self.assertNotIn("Derived {} receive + {} change addresses", source)


    def test_retired_donation_surface_stays_removed_and_logo_is_home(self) -> None:
        dom = DOM.read_text()
        events = SETTINGS_EVENTS.read_text()
        shell = (WEB_JS / "app/shell_controls.js").read_text()
        header = (ROOT / "apps/kaskold-companion-web/web/html/document/open.html").read_text()
        self.assertIn("export function bindClick(id, handler)", dom)
        self.assertIn("if (target) target.onclick = handler", dom)
        self.assertNotIn("btn-donate", events)
        self.assertNotIn("features/donations", shell)
        self.assertIn("logo.onclick = () => navigateHome();", shell)
        self.assertIn('title="Home"', header)
        self.assertFalse((WEB_JS / "features/donations").exists())
        self.assertFalse((WEB_JS / "core/config/donations.js").exists())


    def test_companion_stealth_and_network_status_consumer_layout(self) -> None:
        stealth_html = (ROOT / "apps/kaskold-companion-web/web/html/screens/privacy/stealth.html").read_text()
        stealth_events = (WEB_JS / "app/events/transactions/stealth.js").read_text()
        catch_up = (WEB_JS / "features/stealth/index/scanning/live/catch_up_session.js").read_text()
        navigation = (WEB_JS / "app/navigation.js").read_text()
        connectivity = (WEB_JS / "core/ui/connectivity_status.js").read_text()
        header = (ROOT / "apps/kaskold-companion-web/web/html/document/open.html").read_text()
        header_css = (ROOT / "apps/kaskold-companion-web/web/css/app/layout/header_and_menu.css").read_text()
        multisig = (ROOT / "apps/kaskold-companion-web/web/html/screens/wallet/multisig.html").read_text()

        self.assertNotIn("<br>", stealth_html[stealth_html.index('stealth-menu-help'):stealth_html.index('</p>', stealth_html.index('stealth-menu-help'))])
        self.assertIn("Stealth addresses let anyone pay you without linking the payment to your public address.", stealth_html)
        self.assertIn("Fetches announcements from the stealth announcement address, sends R values to your device via QR for ECDH scanning, then checks which one-time pubkeys have UTXOs.", stealth_html)
        self.assertIn('id="btn-stealth-back"', stealth_html)
        self.assertIn("bindClick('btn-stealth-back', () => navigateBack('dashboard'))", stealth_events)
        self.assertIn("bindClick('btn-stealth-fetch-announcements', async () =>", stealth_events)
        self.assertIn("button.textContent = 'Fetching Announcements…'", stealth_events)
        self.assertIn("Recent announcement scan is unavailable right now.", catch_up)

        self.assertIn('class="network-tag hidden"', header)
        self.assertIn("document.querySelector('#status-dot .network-tag')", navigation)
        self.assertIn("networkTag.classList.toggle('hidden', !isTestnet)", navigation)
        self.assertIn("clearNetworkTag(networkTag)", connectivity)
        self.assertIn(".header-status .network-tag {", header_css)
        self.assertIn("font-size: 14px", header_css)
        self.assertIn("Branch 0 is the default; use the cosigner index shown by the KasKold signer.", multisig)

    def test_loading_copy_tracks_the_actual_operation(self) -> None:
        navigation = NAVIGATION.read_text()
        kpub_import = KPUB_IMPORT.read_text()
        kpub_manager = KPUB_MANAGER.read_text()
        loading_html = LOADING_HTML.read_text()

        self.assertIn("const note = byId('loading-note')", navigation)
        self.assertIn("Companion will open automatically when the connection is ready.", navigation)
        self.assertIn("title: 'Loading wallet'", navigation)
        self.assertIn("title: 'Broadcasting transaction'", navigation)
        self.assertIn("title: 'Preparing transaction'", navigation)
        self.assertIn("Reading the latest balance using the current node connection.", navigation)
        self.assertIn('id="loading-note"', loading_html)
        self.assertIn('Loading wallet “${entry.name}”…', kpub_manager)
        self.assertIn('Loading wallet “${friendlyName}”…', kpub_manager)
        self.assertNotIn("showLoading('Connecting to a Kaspa node…')", kpub_import)
        self.assertIn('do not block the UI on', kpub_import)
        self.assertIn('hideLoading();', kpub_import)
        self.assertNotIn('await refreshBalance();', kpub_import)

    def test_view_kpub_renders_generated_qr_as_image_not_literal_svg(self) -> None:
        source = KPUB_MANAGER.read_text()
        css = (ROOT / "apps/kaskold-companion-web/web/css/app/components/qr_and_address.css").read_text()
        self.assertIn("image.src = `data:image/svg+xml;charset=utf-8,${encodeURIComponent(generate_qr_svg_text(kpub))}`", source)
        self.assertIn("qr.replaceChildren(image)", source)
        self.assertNotIn("setSafeMarkup(byId('view-kpub-qr')", source)
        self.assertIn(".qr-container .qr-svg-image", css)

    def test_broadcast_actions_have_explicit_vertical_spacing(self) -> None:
        html = BROADCAST_HTML.read_text()
        css = (ROOT / "apps/kaskold-companion-web/web/css/app/screens/system.css").read_text()
        self.assertIn('btn btn-secondary u-mt-8px" id="btn-load-signed-qr-image"', html)
        self.assertIn('btn btn-secondary u-hidden u-mt-8px" id="btn-broadcast-hex"', html)
        self.assertIn('btn btn-back broadcast-back" id="btn-broadcast-back"', html)
        self.assertNotIn("#screen-broadcast .broadcast-back {\n    margin-top: 16px;", css)
        self.assertIn("#screen-broadcast .back-home-row", css)
        self.assertIn("margin-top: 16px", css)


    def test_advanced_broadcast_returns_to_advanced_menu(self) -> None:
        events = (WEB_JS / "app/events/system/core.js").read_text()
        self.assertIn("navigationState._broadcastReturnScreen = 'advanced'", events)
        self.assertIn("navigationState._broadcastReturnScreen = 'welcome'", events)
        return_routing = (WEB_JS / "app/events/transactions/return_routing.js").read_text()
        self.assertIn("showScreen(screen, { recordHistory: false })", return_routing)

    def test_qr_frame_hex_paste_auto_accepts_and_enter_submits_without_buttons(self) -> None:
        broadcast_html = BROADCAST_HTML.read_text()
        scanner_html = (ROOT / "apps/kaskold-companion-web/web/html/screens/system/scanner.html").read_text()
        events = TRANSACTION_EVENTS.read_text()
        camera = (WEB_JS / "features/stealth/index/camera.js").read_text()
        self.assertNotIn('btn-accept-signed-frame-hex', broadcast_html)
        self.assertNotIn('btn-scanner-accept-frame-hex', scanner_html)
        self.assertIn("signedFrameHexInput.onpaste", events)
        self.assertIn("event.key !== 'Enter'", events)
        self.assertIn("input.onpaste = event =>", camera)
        self.assertIn("acceptFrameHex();", camera)
        self.assertIn('accepted automatically, or press Enter', broadcast_html)
        self.assertIn('accepted automatically, or press Enter', scanner_html)

    def test_broadcast_screen_supports_signed_qr_image_files(self) -> None:
        html = BROADCAST_HTML.read_text()
        events = TRANSACTION_EVENTS.read_text()
        importer = SIGNED_IMAGE.read_text()
        broadcast = BROADCAST.read_text()

        self.assertIn('id="btn-load-signed-qr-image"', html)
        self.assertIn('id="input-signed-qr-image"', html)
        self.assertIn('id="broadcast-image-status"', html)
        self.assertIn("importSignedQrImage(file)", events)
        self.assertIn("decodeQrImageFile(file)", importer)
        self.assertIn("stopCamera: false", importer)
        self.assertIn("progressTargetId: 'broadcast-image-status'", importer)
        self.assertIn("options.stopCamera !== false", broadcast)


if __name__ == "__main__":
    unittest.main()
