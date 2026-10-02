#!/usr/bin/env python3
"""Hardware home-icon transparency and multisig descriptor-management regressions."""
from __future__ import annotations

import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[3] / "qa/checks"))
from portal_source import kaskold_source  # noqa: E402

from pathlib import Path
import struct
import unittest

ROOT = Path(__file__).resolve().parents[3]
ASSETS = ROOT / "apps/kaskold-hardware/assets"
SRC = ROOT / "apps/kaskold-hardware/src"


def read(path: str) -> str:
    return kaskold_source(str(path)).read_text()


def read_rgb565(path: Path) -> list[int]:
    data = path.read_bytes()
    if len(data) % 2 != 0:
        raise AssertionError(f"{path} is not RGB565 LE data")
    return list(struct.unpack("<" + "H" * (len(data) // 2), data))


class HardwareHomeAndMultisigDescriptorTests(unittest.TestCase):
    def test_home_grid_uses_settings_icon_and_companion_qr_title(self) -> None:
        home = read("apps/kaskold-hardware/src/ui/screens/navigation/home.rs")
        qr = read("apps/kaskold-hardware/src/ui/screens/signing/qr.rs")

        self.assertIn('include_bytes!("../../../../assets/icon_connect_companion_56.raw")', home)
        self.assertIn('include_bytes!("../../../../assets/icon_send.raw")', home)
        self.assertIn('include_bytes!("../../../../assets/icon_wallet_56.raw")', home)
        self.assertIn('include_bytes!("../../../../assets/icon_settings.raw")', home)
        self.assertNotIn('include_bytes!("../../../../assets/icon_about.raw")', home)
        self.assertIn('let title = "COMPANION QR";', qr)
        self.assertNotIn('let title = "CONNECT COMPANION";', qr)

    def test_home_icon_assets_have_black_transparent_background_pixels(self) -> None:
        for name, size in (("icon_connect_companion_56.raw", 56), ("icon_send.raw", 56), ("icon_wallet_56.raw", 56), ("icon_settings.raw", 56)):
            pixels = read_rgb565(ASSETS / name)
            self.assertEqual(len(pixels), size * size, name)
            corners = [pixels[0], pixels[size - 1], pixels[-size], pixels[-1]]
            self.assertEqual(corners, [0, 0, 0, 0], name)
            self.assertGreater(pixels.count(0), 0, name)

    def test_wallet_and_settings_menu_use_dedicated_asset_icons(self) -> None:
        drawing = read("apps/kaskold-hardware/src/ui/display/icons/drawing.rs")
        for label, asset in (
            ("Receive", "icon_wallet_receive_24.raw"),
            ("Backup", "icon_wallet_backup_24.raw"),
            ("Recovery", "icon_wallet_recovery_24.raw"),
            ("Security", "icon_settings_security_24.raw"),
            ("Storage", "icon_settings_storage_24.raw"),
        ):
            self.assertIn(f'"{label}" => Some(include_bytes!("../../../../assets/{asset}").as_slice())', drawing)


    def test_vault_home_reuses_hardware_icon_sources_and_post_home_shortcut(self) -> None:
        source = ASSETS / "source"
        web = ROOT / "apps/kaskold-vault-web/web/img/hardware"
        android = ROOT / "apps/kaskold-vault-android/app/src/main/res/drawable-nodpi"
        ios = ROOT / "apps/kaskold-vault-ios/KasKoldVault/Assets.xcassets"
        mappings = {
            "icon_connect_companion_56.png": "HomeConnect.imageset",
            "icon_send.png": "HomeScan.imageset",
            "icon_wallet_56.png": "HomeWallet.imageset",
            "icon_settings.png": "HomeSettings.imageset",
            "icon_home_24.png": "HomeShortcut.imageset",
        }
        for filename, imageset in mappings.items():
            canonical = (source / filename).read_bytes()
            self.assertEqual((web / filename).read_bytes(), canonical, filename)
            self.assertEqual((android / filename).read_bytes(), canonical, filename)
            self.assertEqual((ios / imageset / filename).read_bytes(), canonical, filename)

        web_html = read("apps/kaskold-vault-web/web/index.html")
        web_js = read("apps/kaskold-vault-web/web/js/vault_main.js")
        web_css = read("apps/kaskold-vault-web/web/css/vault.css")
        web_settings = read("apps/kaskold-vault-web/web/js/vault_settings.js")
        web_builder = read("tools/build/web/build_vault_runtime.py")
        web_revision = read("tools/build/web/static_asset_revision.py")
        android_main = read("apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/MainActivity.kt")
        android_controller = read("apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/runtime/VaultController.kt")
        ios_home = read("apps/kaskold-vault-ios/KasKoldVault/Features/VaultHomeView.swift")
        ios_model = read("apps/kaskold-vault-ios/KasKoldVault/Features/VaultViewModel.swift")

        self.assertIn('class="home-menu-grid"', web_html)
        self.assertIn('id="brand-home"', web_html)
        self.assertIn('screen-home-shortcut', web_js)
        self.assertIn("homeReached = true", web_js)
        self.assertIn("grid-template-columns:repeat(2,minmax(0,1fr))", web_css)
        self.assertIn("grid-auto-flow:row", web_css)
        self.assertIn("width:min(100%,360px)", web_css)
        self.assertIn("aspect-ratio:1/1", web_css)
        self.assertIn("background:#000", web_css)
        self.assertIn("const bindClick = (id, handler) =>", web_settings)
        self.assertIn("bindClick('settings-security', () => show('security'))", web_settings)
        self.assertIn("authored_asset_revision", web_builder)
        self.assertIn("stamp_site_asset_revision", web_builder)
        self.assertIn("?rev={revision}", web_revision)
        self.assertIn('HomeMenuButton("Connect", R.drawable.icon_connect_companion_56', android_main)
        self.assertIn("controller.homeShortcutVisible(state)", android_main)
        self.assertIn("private var homeReached = false", android_controller)
        self.assertIn('LazyVGrid(columns: [GridItem(.flexible()), GridItem(.flexible())]', ios_home)
        self.assertIn('Image("HomeShortcut")', ios_home)
        self.assertIn("@Published private(set) var homeReached = false", ios_model)

    def test_multisig_creation_prompts_for_descriptor_save_and_persists_store(self) -> None:
        output = read("apps/kaskold-hardware/src/runtime/interactions/tx/multisig_output.rs")
        menu = read("apps/kaskold-hardware/src/runtime/interactions/menu/signing/multisig.rs")
        config = read("apps/kaskold-hardware/src/runtime/interactions/multisig_config.rs")
        menus = read("apps/kaskold-hardware/src/runtime/navigation/ui_graph/menus.rs")

        self.assertIn("descriptor_save_prompted = false;", menu)
        self.assertIn("MultisigSaveDescriptorAsk", output)
        self.assertIn("save_creating_descriptor(ad)", output)
        self.assertIn("persist_multisig_store(ad)", output)
        self.assertIn('"Descriptor saved"', output)
        self.assertIn('"Could not save descriptor"', output)
        self.assertIn('"Descriptors"', menus)
        self.assertIn('"View"', menus)
        self.assertIn('"Backup to SD"', menus)
        self.assertIn('"Restore from SD"', menus)
        self.assertIn('"Delete"', menus)
        self.assertIn("visible_descriptor_indices", config)
        self.assertIn("belongs_to_active_wallet", config)
        self.assertIn("descriptor_persisted[index]", config)

    def test_recovery_menu_does_not_expose_multisig_descriptor_restore(self) -> None:
        menus = read("apps/kaskold-hardware/src/runtime/navigation/ui_graph/menus.rs")
        routes = read("apps/kaskold-hardware/src/runtime/navigation/menu_reducer/routes.rs")
        imports = read("apps/kaskold-hardware/src/runtime/interactions/sd/imports/import_menu.rs")
        multisig = read("apps/kaskold-hardware/src/runtime/interactions/menu/signing/multisig.rs")

        sd_menu_start = menus.index("pub(crate) const SD_IMPORT_MENU_ITEMS")
        sd_menu_end = menus.index("pub(crate) const SD_IMPORT_MENU_LABELS", sd_menu_start)
        recovery_sd_menu = menus[sd_menu_start:sd_menu_end]
        self.assertNotIn("Multisig Descriptor", recovery_sd_menu)
        self.assertNotIn("sd.import.multisig_descriptor", recovery_sd_menu)
        self.assertIn('"Restore from SD"', menus)
        self.assertIn("scan_multisig_descriptor_files", multisig)
        self.assertIn("MULTISIG_DESCRIPTOR_RULE", imports)
        self.assertNotIn("MULTISIG_DESCRIPTOR_RULE,", imports[imports.index("const IMPORT_RULES"):imports.index("fn scan_rule_plan")])
        self.assertNotIn("(SdImportMenu, 6)", routes)

    def test_removed_bip39_last_word_calculator_is_not_mutation_surface(self) -> None:
        encoding = read("crates/offline-signer/src/derivation/bip39/encoding.rs")
        module = read("crates/offline-signer/src/derivation/bip39/mod.rs")
        self.assertNotIn("complete_last_word_12", encoding)
        self.assertNotIn("complete_last_word_24", encoding)
        self.assertNotIn("calculate_last_word", encoding)
        self.assertNotIn("write_word_bits", encoding)
        self.assertNotIn("checksum_matches", encoding)
        self.assertNotIn("complete_last_word_12", module)
        self.assertNotIn("complete_last_word_24", module)


if __name__ == "__main__":
    unittest.main()
