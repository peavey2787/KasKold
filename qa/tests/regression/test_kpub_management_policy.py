"""Static markup and stylesheet contracts for wallet management screens.

Behaviour (repository persistence, switching, unload, startup routing, the
receive/verify/history/asset views) is asserted by executing the modules in
qa/checks/web/check_web_runtime.mjs, web_kpub_manager_deep_paths.test.mjs,
web_branch_hardening*.test.mjs and web_wallet_views_behaviour.test.mjs. What
remains here is the shipped HTML and CSS itself.
"""
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
WEB = ROOT / "apps/kaskold-companion-web/web"
OPEN_HTML = WEB / "html/document/open.html"
MANAGER_HTML = WEB / "html/screens/wallet/kpub_manager.html"
WELCOME_HTML = WEB / "html/screens/system/welcome.html"
SCANNER_HTML = WEB / "html/screens/system/scanner.html"
DASHBOARD_HTML = WEB / "html/screens/system/dashboard.html"
MULTISIG_HTML = WEB / "html/screens/wallet/multisig.html"
SYSTEM_CSS = WEB / "css/app/screens/system.css"
HEADER_CSS = WEB / "css/app/layout/header_and_menu.css"


def css_rule(source: str, selector: str) -> str:
    start = source.index(selector + " {")
    return source[start:source.index("}", start)]


class KpubManagementMarkupTests(unittest.TestCase):
    def test_wallet_management_screens_offer_every_import_path(self) -> None:
        shell = OPEN_HTML.read_text()
        screen = MANAGER_HTML.read_text()
        welcome = WELCOME_HTML.read_text()

        self.assertNotIn('id="gear-menu"', shell)
        self.assertIn('title="Node Settings"', shell)
        self.assertIn('src="img/kaspa.stream.svg"', shell)
        self.assertTrue((WEB / "img/kaspa.stream.svg").is_file())
        self.assertFalse((WEB / "img/kaspa-icon.png").exists())
        self.assertIn('class="card kpub-manager-list-card hidden" id="kpub-saved-section"', screen)
        for control in (
            "btn-open-kpub-import", "btn-scan-managed-kpub", "btn-upload-managed-kpub",
            "input-managed-kpub-image", "input-managed-kpub", "input-kpub-friendly-name",
            "chk-new-kpub-auto-load", "btn-save-managed-kpub", "kpub-saved-list",
        ):
            self.assertIn(f'id="{control}"', screen)
        self.assertIn('>Use wallet once</button>', screen)
        self.assertIn('Friendly name (optional)', screen)
        self.assertIn('placeholder="Example: Main wallet"', screen)
        self.assertIn('id="welcome-saved-kpubs"', welcome)
        self.assertIn('id="welcome-kpub-list"', welcome)
        self.assertNotIn('input-managed-kpub', SCANNER_HTML.read_text())

    def test_header_icons_and_settings_cog_styles(self) -> None:
        source = HEADER_CSS.read_text()
        rule = css_rule(source, '.explorer-network-icon')
        self.assertIn('width: 50px', rule)
        self.assertIn('height: 50px', rule)
        self.assertIn('flex: 0 0 50px', rule)
        self.assertIn('transform: scale(1.2)', rule)
        self.assertIn('#btn-explorer {\n    overflow: hidden;', source)
        self.assertIn('.header-logo:hover,', source)
        self.assertIn('#btn-explorer:hover .explorer-network-icon,', source)
        self.assertNotIn('.gear-menu', source)
        self.assertNotIn('.gear-tab', source)

    def test_address_and_compact_list_styles(self) -> None:
        address = (WEB / "css/app/components/qr_and_address.css").read_text()
        lists = (WEB / "css/app/screens/history_and_assets.css").read_text()
        line = css_rule(address, '.address-line')
        self.assertIn("white-space: nowrap", line)
        self.assertIn("text-align: center", line)
        self.assertIn("overflow-wrap: anywhere", SYSTEM_CSS.read_text())
        self.assertIn("word-break: break-word", SYSTEM_CSS.read_text())
        for compact in (
            "#screen-history.history-compact .back-home-row:not(.back-home-row-top)",
            "#screen-tokens.tokens-compact .back-home-row:not(.back-home-row-top)",
        ):
            self.assertIn("display: none", css_rule(lists, compact))

    def test_saved_kpub_lists_scroll_and_form_heading_keeps_width(self) -> None:
        source = SYSTEM_CSS.read_text()
        for selector in ('.kpub-saved-list', '.welcome-kpub-list'):
            rule = css_rule(source, selector)
            self.assertIn('max-height:', rule)
            self.assertIn('overflow-y: auto', rule)
        self.assertIn('grid-template-columns: minmax(0, 1fr) auto', css_rule(source, '.kpub-manager-form-heading'))
        self.assertIn('width: auto', css_rule(source, '.kpub-manager-form-heading .btn-link'))

    def test_dashboard_has_four_icon_actions_and_one_line_secondary_views(self) -> None:
        html = DASHBOARD_HTML.read_text()
        css = SYSTEM_CSS.read_text()
        self.assertGreaterEqual(html.count('class="dashboard-action-icon"'), 4)
        for control in (
            'btn-send', 'btn-receive', 'btn-multisig-spend', 'btn-advanced',
            'btn-dashboard-tokens', 'btn-dashboard-history', 'btn-dashboard-portfolio',
        ):
            self.assertIn(f'id="{control}"', html)
        self.assertIn('grid-template-columns: repeat(2, minmax(0, 1fr))', css_rule(css, '.dashboard-action-grid'))
        self.assertIn('white-space: nowrap', css_rule(css, '.dashboard-secondary-links'))
        balance = html.index('class="balance-card"')
        secondary = html.index('class="dashboard-secondary-links"')
        grid = html.index('class="dashboard-action-grid"')
        self.assertLess(balance, secondary)
        self.assertLess(secondary, grid)
        self.assertIn('class="balance-refresh-button" id="btn-refresh"', html)
        self.assertIn('aria-label="Refresh balance"', html)
        self.assertNotIn('Refresh Balance', html)
        self.assertNotIn('Reset Wallet', html)
        self.assertIn('.balance-refresh-button {', (WEB / 'css/app/components/cards.css').read_text())

    def test_multisig_descriptor_management_offers_camera_and_image_import(self) -> None:
        html = MULTISIG_HTML.read_text()
        for control in ('btn-ms-descriptor-scan-saved', 'btn-ms-descriptor-upload-saved', 'input-ms-descriptor-image-saved'):
            self.assertIn(f'id="{control}"', html)


if __name__ == "__main__":
    unittest.main()
