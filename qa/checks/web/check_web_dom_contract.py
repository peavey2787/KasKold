#!/usr/bin/env python3
"""Validate required Companion wallet-import and covenant DOM contracts."""

from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[3]
HTML_ROOT = ROOT / "apps/kaskold-companion-web/web/html"
JS_ROOT = ROOT / "apps/kaskold-companion-web/web/js"

REQUIRED_PRIVATE_SWAP_IDS = {
    "cov-private-swap-panel", "private-swap-hub", "private-swap-create",
    "private-swap-join", "private-swap-dashboard", "btn-private-swap-create",
    "btn-private-swap-join", "btn-private-swap-create-key", "btn-private-swap-join-key",
    "btn-private-swap-bind", "btn-private-swap-fund", "btn-private-swap-presign",
    "btn-private-swap-share-presig", "btn-private-swap-scan-presig",
    "btn-private-swap-share-ready", "btn-private-swap-scan-ready",
    "btn-private-swap-complete", "btn-private-swap-bob-claim", "btn-private-swap-refund",
}
REQUIRED_KPUB_IMPORT_IDS = {
    "btn-scan-kpub", "companion-startup-status", "screen-kpub-manager",
    "kpub-saved-list", "btn-open-kpub-import", "kpub-import-form",
    "btn-scan-managed-kpub", "btn-upload-managed-kpub",
    "input-managed-kpub-image", "input-managed-kpub",
    "input-kpub-friendly-name", "chk-new-kpub-auto-load",
    "btn-save-managed-kpub",
}
REQUIRED_WELCOME_KPUB_IDS = {
    "btn-scan-kpub", "companion-startup-status",
    "welcome-saved-kpubs", "welcome-kpub-list",
}
RETIRED_KPUB_IMPORT_IDS = {
    "btn-load-kpub-image", "input-kpub-image", "input-kpub", "btn-import-kpub",
}
RETIRED_CREATION_TOKENS = {
    "buildTreasury", "cov-fields-treasury", "buildTimelockedEscrow",
    "cov-fields-tl-escrow", "deprecated.js",
}


def main() -> int:
    html = "\n".join(path.read_text(encoding="utf-8") for path in HTML_ROOT.rglob("*.html"))
    js = "\n".join(path.read_text(encoding="utf-8") for path in JS_ROOT.rglob("*.js"))
    ids = set(re.findall(r'\bid=["\']([^"\']+)["\']', html))
    errors: list[str] = []

    missing_kpub = sorted(REQUIRED_KPUB_IMPORT_IDS - ids)
    if missing_kpub:
        errors.append(f"kpub management/import DOM is incomplete: {missing_kpub}")
    retired_kpub = sorted(RETIRED_KPUB_IMPORT_IDS & ids)
    if retired_kpub:
        errors.append(f"retired standalone kpub-import controls remain: {retired_kpub}")
    missing_welcome_kpub = sorted(REQUIRED_WELCOME_KPUB_IDS - ids)
    if missing_welcome_kpub:
        errors.append(f"welcome saved-kpub selection DOM is incomplete: {missing_welcome_kpub}")

    welcome = (HTML_ROOT / 'screens/system/welcome.html').read_text(encoding="utf-8")
    scanner = (HTML_ROOT / 'screens/system/scanner.html').read_text(encoding="utf-8")
    manager = (HTML_ROOT / 'screens/wallet/kpub_manager.html').read_text(encoding="utf-8")
    if 'id="btn-scan-kpub"' not in welcome or 'Manage Wallets' not in welcome:
        errors.append('welcome screen must expose one centralized watch-only wallet-management entry point')
    for control in REQUIRED_KPUB_IMPORT_IDS - {"btn-scan-kpub", "companion-startup-status"}:
        if control in welcome:
            errors.append(f'welcome screen must not duplicate managed kpub control: {control}')
    for control in (
        'btn-open-kpub-import', 'btn-scan-managed-kpub', 'btn-upload-managed-kpub',
        'input-managed-kpub-image', 'input-managed-kpub', 'input-kpub-friendly-name',
        'btn-save-managed-kpub', 'kpub-saved-list',
    ):
        if control not in manager:
            errors.append(f'kpub management must own import control: {control}')
        if control in scanner:
            errors.append(f'camera scanner must not duplicate managed kpub control: {control}')
    if 'Scan wallet QR with camera' not in js or 'decodeKpubQrImage' not in js:
        errors.append('wallet management must support camera scanning and QR image upload')
    if "showKpubManager('welcome', { openImport: true })" not in js:
        errors.append('the welcome Manage Wallets button must open centralized wallet management')

    app_css_root = ROOT / 'apps/kaskold-companion-web/web/css/app'
    header_css = (app_css_root / 'layout/header_and_menu.css').read_text(encoding="utf-8")
    buttons_css = (app_css_root / 'components/buttons.css').read_text(encoding="utf-8")
    system_css = (app_css_root / 'screens/system.css').read_text(encoding="utf-8")
    tokens_css = (app_css_root / 'foundation/tokens.css').read_text(encoding="utf-8")
    too_small = []
    for css_path in app_css_root.rglob('*.css'):
        for match in re.finditer(r'font-size\s*:\s*(\d+)px', css_path.read_text(encoding="utf-8")):
            if int(match.group(1)) <= 13:
                too_small.append(f"{css_path.relative_to(ROOT)}:{match.group(1)}px")
    if too_small:
        errors.append(f'Companion authored app CSS contains text below the 14px readability floor: {too_small[:12]}')
    for selector_snippet in (
        '.header-logo {\n    width: 50px;\n    height: 50px;',
        '.header-social svg {\n    width: 32px;\n    height: 32px;',
        '.explorer-network-icon {\n    width: 50px;\n    height: 50px;',
        'transform: scale(1.2);',
        '.header-settings-btn {\n    width: 50px;\n    height: 50px;',
        'font-size: 20px;',
        '.dot {\n    width: 15px;\n    height: 15px;',
    ):
        if selector_snippet not in header_css:
            errors.append(f'Companion header sizing contract drifted: missing {selector_snippet!r}')
    if '<hr class="welcome-tools-separator" aria-hidden="true">' not in welcome or 'watch-only tools' in welcome:
        errors.append('Companion startup tools must use one unlabeled solid separator line')
    if '#btn-multisig-welcome { margin-top: 10px; }' not in system_css:
        errors.append('Companion startup Broadcast TX and Multisig buttons must retain visible spacing')
    if 'Scan with a camera, upload a QR image, or paste the account public key.' in manager:
        errors.append('retired managed-kpub explanatory paragraph remains')
    if 'Use once loads this watch-only kpub for the current session only.' in manager:
        errors.append('retired one-time-kpub explanatory paragraph remains')
    if 'placeholder="Example: Main wallet"' not in manager:
        errors.append('Friendly Name placeholder must stay concise')
    if '.btn-back,\n.btn-home-nav {' not in buttons_css or 'background: var(--surface-2);' not in buttons_css:
        errors.append('Companion Back and Home controls must share the same themed button shell')
    if 'linear-gradient(135deg, #2563eb, #1d4ed8)' in buttons_css:
        errors.append('retired blue Back-button theme remains in Companion CSS')

    kpub_css = (ROOT / 'apps/kaskold-companion-web/web/css/app/screens/system.css').read_text(encoding="utf-8")
    saved_list_rule = re.search(r'\.kpub-saved-list\s*\{([^}]*)\}', kpub_css, re.S)
    if not saved_list_rule or 'max-height:' not in saved_list_rule.group(1) or 'overflow-y: auto' not in saved_list_rule.group(1):
        errors.append('saved kpubs must render in a bounded scrollable list')

    for retired_social in ('https://x.com/KasKold', 'https://www.youtube.com/@KasKold', 'https://t.me/KasKold'):
        if retired_social in html:
            errors.append(f'unsupported project social link remains: {retired_social}')

    retired_donation_paths = (
        ROOT / 'apps/kaskold-companion-web/web/html/screens/system/donate.html',
        ROOT / 'apps/kaskold-companion-web/web/css/app/screens/donation_and_safe_area.css',
        ROOT / 'apps/kaskold-companion-web/web/img/kaskold-donate.png',
        ROOT / 'apps/kaskold-companion-web/web/img/kaskold-donation-qr.png',
        JS_ROOT / 'core/config/donations.js',
        JS_ROOT / 'features/donations/screen.js',
    )
    lingering_donation_paths = [path.relative_to(ROOT).as_posix() for path in retired_donation_paths if path.exists()]
    if lingering_donation_paths:
        errors.append(f'retired Companion donation surface remains: {lingering_donation_paths}')
    for retired_token in ('screen-donate', 'btn-copy-donate', 'btn-donate-skip', 'DONATE_ADDRESS', 'features/donations'):
        if retired_token in html or retired_token in js:
            errors.append(f'retired Companion donation token remains: {retired_token}')
    header = (HTML_ROOT / 'document/open.html').read_text(encoding='utf-8')
    if 'id="btn-logo"' not in header or 'title="Home"' not in header or 'aria-label="Home"' not in header:
        errors.append('Companion header logo must be exposed as the Home control')
    shell_controls = (JS_ROOT / 'app/shell_controls.js').read_text(encoding='utf-8')
    if "logo.onclick = () => navigateHome();" not in shell_controls or 'navigateHome();' not in shell_controls:
        errors.append('Companion header logo must route through the shared navigateHome action')

    settings_events = (JS_ROOT / 'app/events/wallet/settings_and_wallet.js').read_text(encoding="utf-8")
    header = (HTML_ROOT / 'document/open.html').read_text(encoding='utf-8')
    if 'id="gear-menu"' in header or 'class="gear-tab' in header:
        errors.append('retired settings submenu remains; the cog must open Node Connection directly')
    if "bindClick('btn-header-settings', () => showSettings(" not in settings_events:
        errors.append('settings cog must route directly to Node Connection')
    for dashboard_target in ('btn-dashboard-tokens', 'btn-dashboard-history', 'btn-dashboard-portfolio'):
        if dashboard_target not in html:
            errors.append(f'dashboard secondary wallet view is missing: {dashboard_target}')
    verify = (HTML_ROOT / 'screens/system/verify.html').read_text(encoding='utf-8')
    if 'Compare with your KasKold or other wallet.' in verify or 'Every character must match exactly.' in verify:
        errors.append('Verify Address must not show the retired manual-comparison instruction')

    multisig = (HTML_ROOT / 'screens/wallet/multisig.html').read_text(encoding='utf-8')
    if 'id="select-ms-saved-descriptor"' not in multisig or 'Saved Descriptor' not in multisig:
        errors.append('Multisig Spend must expose saved descriptor selection alongside paste/QR import')
    pskt_css = (app_css_root / 'screens/pskt.css').read_text(encoding='utf-8')
    if '#screen-multisig-spend .input-label {' not in pskt_css or 'text-transform: none;' not in pskt_css:
        errors.append('Multisig Spend labels must use normal authored capitalization rather than forced uppercase')

    advanced = (HTML_ROOT / 'screens/system/advanced.html').read_text(encoding='utf-8')
    for advanced_target in ('btn-broadcast', 'btn-covenant', 'btn-advanced-addresses', 'btn-stealth', 'btn-advanced-utxos'):
        if advanced_target not in advanced:
            errors.append(f'Advanced menu is missing: {advanced_target}')


    back_buttons = re.findall(
        r'<button\b[^>]*\bclass=["\']([^"\']*)["\'][^>]*\bid=["\']([^"\']*(?:-back(?:-[^"\']*)?|btn-scanner-cancel))["\']',
        html,
    )
    unstyled_back = sorted(button_id for classes, button_id in back_buttons if 'btn-back' not in classes.split())
    if unstyled_back:
        errors.append(f'back controls must use the shared btn-back style: {unstyled_back}')
    buttons_css = (ROOT / 'apps/kaskold-companion-web/web/css/app/components/buttons.css').read_text(encoding="utf-8")
    if '.btn-back {' not in buttons_css or "content: '←'" not in buttons_css:
        errors.append('shared back controls must be prominent and visually identifiable')
    if '.back-home-row {' not in buttons_css or '.btn-home-nav {' not in buttons_css:
        errors.append('every shared Back control must split its row with a Home control')
    navigation_controls = (JS_ROOT / 'core/ui/navigation_controls.js').read_text(encoding="utf-8")
    if "document.querySelectorAll('.btn-back')" not in navigation_controls or "homeButton.textContent = 'Home'" not in navigation_controls:
        errors.append('shared navigation controls must attach Home beside every Back button')
    navigation_js = (JS_ROOT / 'app/navigation.js').read_text(encoding="utf-8")
    if 'screenHistory' not in navigation_js or 'export function navigateBack' not in navigation_js:
        errors.append('shared navigation must maintain bounded screen history for Back')
    shell_js = (JS_ROOT / 'app/shell_controls.js').read_text(encoding="utf-8")
    settings_js = (JS_ROOT / 'features/settings/screen.js').read_text(encoding="utf-8")
    if "const source = visibleScreenName();" not in shell_js or "setScreenReturn('settings', source)" not in shell_js:
        errors.append('shell settings navigation must remember the visible return screen')
    if "if (source === 'settings') {" not in shell_js or "takeScreenReturn('settings', 'welcome')" not in shell_js:
        errors.append('shell settings cog must toggle Node Connection closed when it is already open')
    if 'takeScreenReturn(' not in settings_js or "'settings'" not in settings_js:
        errors.append('settings Back must consume the remembered return screen')
    if "if (source === 'settings') {" not in settings_js or 'exitSettings();' not in settings_js:
        errors.append('full settings cog must toggle Node Connection closed when it is already open')

    missing = sorted(REQUIRED_PRIVATE_SWAP_IDS - ids)
    if missing:
        errors.append(f"Private Swap v2 DOM is incomplete: {missing}")
    if 'data-cov-panel="private-swap"' not in html or 'id="cov-private-swap-panel"' not in html:
        errors.append("Private Swap must be selectable from the covenant card UI")
    for token in ("openPrivateSwap", "preparePrivateSwapPreSignature", "completeAlicePrivateSwap"):
        if token not in js:
            errors.append(f"Private Swap controller is missing {token}")
    for retired in ('data-cov-type="atomic-swap"', "buildAtomicSwap", "case 'atomic-swap'", 'HTLC'):
        if retired in html or retired in js:
            errors.append(f"retired HTLC Atomic Swap surface remains: {retired}")

    if "context.covSelectType" in js:
        errors.append("covenant type selection must use the direct module function")
    if js.count("export function covSelectType") != 1:
        errors.append("covenant type selection must have one direct exported implementation")

    for token in sorted(RETIRED_CREATION_TOKENS):
        if token in html or token in js:
            errors.append(f"retired covenant creation surface remains: {token}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1
    print(f"PASS: centralized kpub management and covenant DOM contract ({len(ids)} authored ids, Private Swap v2 coherent)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
