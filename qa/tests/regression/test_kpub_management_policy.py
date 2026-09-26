from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
WEB = ROOT / "apps/kaskold-companion-web/web"
OPEN_HTML = WEB / "html/document/open.html"
MANAGER_HTML = WEB / "html/screens/wallet/kpub_manager.html"
WELCOME_HTML = WEB / "html/screens/system/welcome.html"
SCANNER_HTML = WEB / "html/screens/system/scanner.html"
MANAGER = WEB / "js/features/wallet/kpub_manager/controller.js"
REPOSITORY = WEB / "js/features/wallet/kpub_manager/repository.js"
NAVIGATION = WEB / "js/app/navigation.js"
WALLET_SESSION = WEB / "js/app/state/core/wallet_session.js"
SETTINGS_EVENTS = WEB / "js/app/events/wallet/settings_and_wallet.js"
SYSTEM_EVENTS = WEB / "js/app/events/system/core.js"
SYSTEM_CSS = WEB / "css/app/screens/system.css"
HEADER_CSS = WEB / "css/app/layout/header_and_menu.css"
KASPA_STREAM_ICON = WEB / "img/kaspa.stream.svg"
OLD_KASPA_ICON = WEB / "img/kaspa-icon.png"
STATE_RESET = WEB / "js/features/wallet/state_reset.js"
RESET = WEB / "js/features/wallet/reset.js"
SETTINGS_SCREEN = WEB / "js/features/settings/screen.js"
DASHBOARD_HTML = WEB / "html/screens/system/dashboard.html"
MULTISIG_HTML = WEB / "html/screens/wallet/multisig.html"
MULTISIG_DESCRIPTORS = WEB / "js/features/transactions/pskt_multisig/descriptors.js"


class KpubManagementPolicyTests(unittest.TestCase):
    def test_wallet_management_is_centralized_and_settings_cog_opens_node_directly(self) -> None:
        shell = OPEN_HTML.read_text()
        screen = MANAGER_HTML.read_text()
        welcome = WELCOME_HTML.read_text()
        scanner = SCANNER_HTML.read_text()
        settings_events = SETTINGS_EVENTS.read_text()
        system_events = SYSTEM_EVENTS.read_text()

        self.assertNotIn('id="gear-menu"', shell)
        self.assertNotIn('class="gear-tab', shell)
        self.assertIn('title="Node Settings"', shell)
        self.assertIn('src="img/kaspa.stream.svg"', shell)
        self.assertTrue(KASPA_STREAM_ICON.is_file())
        self.assertFalse(OLD_KASPA_ICON.exists())
        self.assertIn('id="screen-kpub-manager"', screen)
        self.assertIn('id="kpub-saved-list"', screen)
        self.assertIn('class="card kpub-manager-list-card hidden" id="kpub-saved-section"', screen)
        self.assertNotIn('Choose a saved watch-only wallet or add a new kpub.', screen)
        self.assertNotIn('or enter manually', screen)
        self.assertIn('id="btn-open-kpub-import"', screen)
        self.assertIn('id="btn-scan-managed-kpub"', screen)
        self.assertIn('id="btn-upload-managed-kpub"', screen)
        self.assertIn('id="input-managed-kpub-image"', screen)
        self.assertIn('id="input-managed-kpub"', screen)
        self.assertIn('id="input-kpub-friendly-name"', screen)
        self.assertIn('id="chk-new-kpub-auto-load"', screen)
        self.assertIn('id="btn-save-managed-kpub"', screen)
        self.assertIn('>Use wallet once</button>', screen)
        self.assertNotIn('It is not saved and will not load on startup.', screen)
        self.assertIn('id="welcome-saved-kpubs"', welcome)
        self.assertIn('id="welcome-kpub-list"', welcome)
        self.assertIn('Friendly name (optional)', screen)
        self.assertIn('placeholder="Example: Main wallet"', screen)
        self.assertNotIn('Scan with a camera, upload a QR image, or paste the account public key.', screen)
        self.assertNotIn('input-managed-kpub', scanner)
        self.assertIn("bindClick('btn-header-settings', () => showSettings(", settings_events)
        settings_screen = SETTINGS_SCREEN.read_text()
        self.assertIn("if (source === 'settings') {", settings_screen)
        self.assertIn('exitSettings();', settings_screen)
        self.assertIn("showKpubManager('welcome', { openImport: true })", system_events)
        navigation = NAVIGATION.read_text()
        self.assertNotIn('Choose a saved kpub to continue.', navigation)
        self.assertIn("status.classList.toggle('hidden', text.length === 0)", navigation)
        self.assertIn('.header-logo:hover,', HEADER_CSS.read_text())
        self.assertIn('#btn-explorer:hover .explorer-network-icon,', HEADER_CSS.read_text())


    def test_kaspa_stream_header_icon_is_slightly_smaller_than_other_header_icons(self) -> None:
        source = HEADER_CSS.read_text()
        start = source.index('.explorer-network-icon {')
        rule = source[start:source.index('}', start)]
        self.assertIn('width: 50px', rule)
        self.assertIn('height: 50px', rule)
        self.assertIn('flex: 0 0 50px', rule)
        self.assertIn('transform: scale(1.2)', rule)
        self.assertIn('#btn-explorer {\n    overflow: hidden;', source)

    def test_settings_cog_has_no_secondary_menu(self) -> None:
        source = HEADER_CSS.read_text()
        shell = OPEN_HTML.read_text()
        self.assertNotIn('.gear-menu', source)
        self.assertNotIn('.gear-tab', source)
        self.assertNotIn('id="gear-menu"', shell)

    def test_receive_address_is_three_balanced_lines_and_copy_keeps_raw_value(self) -> None:
        source = (WEB / "js/features/transactions/send/receive.js").read_text()
        css = (WEB / "css/app/components/qr_and_address.css").read_text()
        self.assertIn("export function splitReceiveAddressLines(address)", source)
        self.assertIn("const base = Math.floor(chars.length / 3)", source)
        self.assertIn("const remainder = chars.length % 3", source)
        self.assertIn("display.dataset.address = address", source)
        self.assertIn("row.className = 'address-line'", source)
        self.assertIn("const addr = display.dataset.address", source)
        self.assertIn(".address-line {", css)
        self.assertIn("white-space: nowrap", css)
        self.assertIn("text-align: center", css)

    def test_verify_address_renders_qr_as_image_wraps_and_copy_stays_on_screen(self) -> None:
        address_views = (WEB / "js/features/wallet/tools/address_views.js").read_text()
        settings_events = SETTINGS_EVENTS.read_text()
        system_css = SYSTEM_CSS.read_text()

        self.assertIn("image.className = 'qr-svg-image'", address_views)
        self.assertIn("image.alt = 'Address QR code'", address_views)
        self.assertIn("data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}", address_views)
        self.assertIn("qr.replaceChildren(image)", address_views)
        self.assertNotIn("setSafeMarkup(byId('verify-qr'), frames[0].svg)", address_views)
        self.assertIn("overflow-wrap: anywhere", system_css)
        self.assertIn("word-break: break-word", system_css)

        copy_start = settings_events.index("bindClick('btn-verify-copy'")
        copy_end = settings_events.index("bindClick('btn-verify-back'", copy_start)
        copy_handler = settings_events[copy_start:copy_end]
        self.assertIn("await navigator.clipboard.writeText(address)", copy_handler)
        self.assertIn("toast('Address copied'", copy_handler)
        self.assertNotIn("showScreen('addresses')", copy_handler)

    def test_short_history_hides_redundant_bottom_back_home_pair(self) -> None:
        source = (WEB / "js/features/wallet/tools/history.js").read_text()
        css = (WEB / "css/app/screens/history_and_assets.css").read_text()
        self.assertIn("walletState.historyEntries.length <= 4", source)
        self.assertIn("classList.toggle('history-compact'", source)
        self.assertIn("#screen-history.history-compact .back-home-row:not(.back-home-row-top)", css)
        self.assertIn("display: none", css)

    def test_short_asset_inventory_hides_redundant_bottom_back_home_pair(self) -> None:
        source = (WEB / "js/features/assets/render.js").read_text()
        css = (WEB / "css/app/screens/history_and_assets.css").read_text()
        self.assertIn("total <= 4", source)
        self.assertIn("classList.toggle('tokens-compact'", source)
        self.assertIn("#screen-tokens.tokens-compact .back-home-row:not(.back-home-row-top)", css)

    def test_saved_kpub_list_is_bounded_and_scrollable(self) -> None:
        source = SYSTEM_CSS.read_text()
        rule = source[source.index('.kpub-saved-list {'):source.index('}', source.index('.kpub-saved-list {'))]
        self.assertIn('max-height:', rule)
        self.assertIn('overflow-y: auto', rule)
        welcome_rule = source[source.index('.welcome-kpub-list {'):source.index('}', source.index('.welcome-kpub-list {'))]
        self.assertIn('max-height:', welcome_rule)
        self.assertIn('overflow-y: auto', welcome_rule)

    def test_import_form_heading_keeps_normal_readable_width(self) -> None:
        source = SYSTEM_CSS.read_text()
        rule = source[source.index('.kpub-manager-form-heading {'):source.index('}', source.index('.kpub-manager-form-heading {'))]
        self.assertIn('grid-template-columns: minmax(0, 1fr) auto', rule)
        cancel_rule = source[
            source.index('.kpub-manager-form-heading .btn-link {'):
            source.index('}', source.index('.kpub-manager-form-heading .btn-link {'))
        ]
        self.assertIn('width: auto', cancel_rule)

    def test_repository_persists_named_entries_and_one_startup_selection(self) -> None:
        source = REPOSITORY.read_text()
        self.assertIn("const STORAGE_KEY = 'companion-kpub-manager-v1'", source)
        self.assertIn("function save({ name, kpub, network })", source)
        self.assertIn("function rename(id, name)", source)
        self.assertIn("function remove(id)", source)
        self.assertIn("function setAutoLoad(id)", source)
        self.assertIn("function autoLoadEntry()", source)
        self.assertIn("function nextDefaultName(entries)", source)
        self.assertIn("return `Wallet ${index}`", source)
        self.assertIn("store.autoLoadId = id", source)
        self.assertIn("if (store.autoLoadId === id) store.autoLoadId = null", source)

    def test_manager_supports_all_import_methods_and_safe_switching(self) -> None:
        source = MANAGER.read_text()
        self.assertIn("deriveKpubQrWallet(data, networkState.network)", source)
        self.assertIn("decodeKpubQrImage(file)", source)
        self.assertIn("deriveKpubWallet(kpubInput.value, networkState.network)", source)
        self.assertIn("loadSavedKpub(entry.id)", source)
        self.assertIn("savedSection?.classList.toggle('hidden', entries.length === 0)", source)
        self.assertIn("Any in-progress unsigned transaction will be discarded", source)
        self.assertIn("export function useKpubOnce()", source)
        self.assertIn("profile: null", source)
        self.assertIn("hardenedWalletCleanup()", source)
        self.assertIn("clearForWalletSwitch()", source)
        self.assertLess(
            source.index("deriveKpubWallet(entry.kpub, entry.network)"),
            source.index("clearForWalletSwitch()", source.index("export function loadSavedKpub")),
            "a saved kpub must validate before hardened cleanup clears the current wallet",
        )
        self.assertIn("kpubRepository.remove(entry.id)", source)
        self.assertIn("This only removes the public watch-only key", source)
        self.assertNotIn("xprv", source.lower())
        self.assertNotIn("mnemonic", source.lower())

    def test_reset_and_one_time_unload_share_hardened_cleanup(self) -> None:
        reset = RESET.read_text()
        cleanup = STATE_RESET.read_text()
        manager = MANAGER.read_text()

        self.assertIn("button.textContent = 'Unload wallet'", reset)
        self.assertNotIn('Reset Wallet', DASHBOARD_HTML.read_text())
        self.assertNotIn('current kpub', reset.lower())
        self.assertIn("byId('wallet-unload-modal')", reset)
        self.assertIn("byId('btn-wallet-unload-confirm')", reset)
        self.assertIn("byId('btn-wallet-unload-cancel')", reset)
        self.assertNotIn('confirm(', reset)
        self.assertIn('Your saved wallet will remain available in Manage Wallets.', reset)
        self.assertIn("hardenedWalletCleanup();", reset)
        self.assertIn("requestWalletRuntimeReset();", reset)
        self.assertIn("export function hardenedWalletCleanup()", cleanup)
        self.assertIn("clearAntiKleptoSession();", cleanup)
        self.assertIn("resetSignedQrImageImportSession();", cleanup)
        self.assertIn("stopQrCycle();", cleanup)
        self.assertIn("stopAutoRefresh();", cleanup)
        self.assertIn("sessionStorage.clear()", cleanup)
        self.assertIn("companion:request-runtime-reset", cleanup)
        self.assertIn("consumeSkipAutoLoadOnce()", manager)

    def test_network_change_rerenders_welcome_kpubs_for_selected_network(self) -> None:
        source = SETTINGS_SCREEN.read_text()
        self.assertIn("networkState.network = newNetwork", source)
        self.assertIn("renderWelcomeKpubs();", source)
        self.assertLess(source.index("networkState.network = newNetwork"), source.index("renderWelcomeKpubs();"))
        self.assertLess(source.index("renderWelcomeKpubs();"), source.index("showScreen('welcome')"))

    def test_dashboard_has_four_icon_actions_and_one_line_secondary_views(self) -> None:
        html = DASHBOARD_HTML.read_text()
        css = SYSTEM_CSS.read_text()
        self.assertIn('class="dashboard-action-grid"', html)
        self.assertGreaterEqual(html.count('class="dashboard-action-icon"'), 4)
        for control in ('btn-send', 'btn-receive', 'btn-multisig-spend', 'btn-advanced'):
            self.assertIn(f'id="{control}"', html)
        for control in ('btn-dashboard-tokens', 'btn-dashboard-history', 'btn-dashboard-portfolio'):
            self.assertIn(f'id="{control}"', html)
        self.assertIn('.dashboard-action-grid {', css)
        self.assertIn('grid-template-columns: repeat(2, minmax(0, 1fr))', css)
        self.assertIn('.dashboard-secondary-links {', css)
        self.assertIn('white-space: nowrap', css)
        balance_start = html.index('class="balance-card"')
        secondary_start = html.index('class="dashboard-secondary-links"')
        grid_start = html.index('class="dashboard-action-grid"')
        self.assertLess(balance_start, secondary_start)
        self.assertLess(secondary_start, grid_start)
        self.assertIn('class="balance-refresh-button" id="btn-refresh"', html)
        self.assertIn('aria-label="Refresh balance"', html)
        self.assertNotIn('Refresh Balance', html)
        self.assertIn('.balance-refresh-button {', (WEB / 'css/app/components/cards.css').read_text())

    def test_multisig_descriptor_management_supports_camera_and_image_import(self) -> None:
        html = MULTISIG_HTML.read_text()
        source = MULTISIG_DESCRIPTORS.read_text()
        events = SYSTEM_EVENTS.read_text()
        self.assertIn('id="btn-ms-descriptor-scan-saved"', html)
        self.assertIn('id="btn-ms-descriptor-upload-saved"', html)
        self.assertIn('id="input-ms-descriptor-image-saved"', html)
        self.assertIn('handleSavedDescriptorScan', source)
        self.assertIn('decodeQrImageFile(file)', source)
        self.assertIn('uploadSavedDescriptorQrImage', events)

    def test_startup_routes_after_wasm_to_wallet_or_saved_selection(self) -> None:
        source = NAVIGATION.read_text()
        ready_offset = source.index("markWasmReady();")
        route_offset = source.index("routeStartupKpub")
        self.assertGreater(route_offset, ready_offset)
        self.assertIn("if (wasmStarted)", source)
        self.assertIn("startupRoute.state === 'loaded'", source)
        self.assertIn("startupRoute.state === 'failed'", source)
        self.assertIn("startupRoute.state === 'selection'", source)

        manager = MANAGER.read_text()
        self.assertIn("export function routeStartupKpub()", manager)
        self.assertIn("renderWelcomeKpubs()", manager)
        self.assertIn("showScreen('welcome')", manager)
        self.assertIn("state: entries.length > 0 ? 'selection' : 'empty'", manager)
        self.assertIn("loadSavedKpub(startupEntry.id, { startup: true })", manager)

    def test_wallet_session_tracks_active_saved_profile(self) -> None:
        source = WALLET_SESSION.read_text()
        self.assertIn("let walletProfile = null", source)
        self.assertIn("profile()", source)
        self.assertIn("setProfile(profile)", source)
        self.assertGreaterEqual(source.count("walletProfile = null"), 3)


if __name__ == "__main__":
    unittest.main()
