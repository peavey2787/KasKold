from __future__ import annotations

import json
import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class KasKoldCustodyConformanceTests(unittest.TestCase):
    def test_online_watcher_sdk_and_companion_have_no_production_custody_dependency(self) -> None:
        watcher = tomllib.loads((ROOT / "crates/online-watcher/Cargo.toml").read_text())
        sdk = tomllib.loads((ROOT / "crates/kaskold-sdk/Cargo.toml").read_text())
        protocol = tomllib.loads((ROOT / "crates/kaskold-protocol/Cargo.toml").read_text())
        companion = tomllib.loads((ROOT / "apps/kaskold-companion-web/Cargo.toml").read_text())
        for graph in (watcher, sdk, protocol, companion):
            self.assertNotIn("hot-wallet", graph.get("dependencies", {}))
            self.assertNotIn("offline-signer", graph.get("dependencies", {}))
        self.assertNotIn("vault-runtime", companion.get("dependencies", {}))
        self.assertIn("offline-signer", watcher.get("dev-dependencies", {}))

    def test_wallet_creation_flow_policy_is_shared_across_hardware_web_android_and_ios(self) -> None:
        shared = (ROOT / "crates/shared-signer/src/creation_flow.rs").read_text()
        native = "\n".join(
            path.read_text(errors="ignore")
            for path in sorted((ROOT / "crates/vault-runtime/src").rglob("*.rs"))
        )
        hardware_dice = (
            ROOT / "apps/kaskold-hardware/src/runtime/interactions/menu/seed_generation/additive.rs"
        ).read_text()
        hardware_touch = (ROOT / "apps/kaskold-hardware/src/wallet/mnemonic/touch.rs").read_text()
        web_onboarding = (ROOT / "apps/kaskold-vault-web/web/js/vault_onboarding.js").read_text()
        web_wallets = (ROOT / "apps/kaskold-vault-web/web/js/vault_wallet_workflows.js").read_text()
        android_controller = (
            ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/runtime/VaultController.kt"
        ).read_text()
        android_main = (
            ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/MainActivity.kt"
        ).read_text()
        ios_creation = (
            ROOT / "apps/kaskold-vault-ios/KasKoldVault/Features/VaultViewModel+Creation.swift"
        ).read_text()
        ios_home = (ROOT / "apps/kaskold-vault-ios/KasKoldVault/Features/VaultHomeView.swift").read_text()

        for stage in (
            "WalletNameEntry",
            "StorageSeedWordCountChoice",
            "StorageSeedDiceChoice",
            "StorageSeedDiceCountChoice",
            "DiceRoll",
            "StorageSeedTouchChoice",
            "TouchEntropy",
            "PassphraseChoice",
            "PassphraseEntry",
            "SeedBackup",
            "StorageRecoveryAcknowledgement",
            "StorageFinalizeChoice",
            "StorageProtectionChoice",
        ):
            self.assertIn(stage, shared)
        self.assertIn("DICE_ROLL_TARGETS", shared)
        self.assertIn("TOUCH_ENTROPY_TARGET", shared)
        self.assertIn("shared_signer::creation_flow::DICE_ROLL_TARGETS", hardware_dice)
        self.assertIn("shared_signer::creation_flow::TOUCH_ENTROPY_TARGET", hardware_touch)

        for operation in ('"creation_flow"', '"create_with_entropy"', '"add_create_with_entropy"'):
            self.assertIn(operation, native)

        self.assertIn("kaskold_vault_creation_flow", web_onboarding)
        self.assertIn("kaskold_vault_create_with_entropy", web_onboarding)
        self.assertIn("kaskold_vault_add_create_with_entropy", web_onboarding)
        self.assertIn("openWalletPicker", web_wallets)
        self.assertNotIn("renderWalletList", web_wallets)
        self.assertNotIn("kaskold_vault_add_create_12", web_wallets)
        self.assertNotIn("kaskold_vault_add_create_24", web_wallets)

        self.assertIn('runtime.workflowText("creation_flow")', android_controller)
        self.assertIn('"create_with_entropy"', android_controller)
        self.assertIn('"add_create_with_entropy"', android_controller)
        self.assertIn("CreationTouch", android_main)
        self.assertNotIn("controller.create(12)", android_main)
        self.assertNotIn("controller.create(24)", android_main)

        self.assertIn('runtime.workflowText("creation_flow")', (ROOT / "apps/kaskold-vault-ios/KasKoldVault/Features/VaultViewModel.swift").read_text())
        self.assertIn('"create_with_entropy"', ios_creation)
        self.assertIn('"add_create_with_entropy"', ios_creation)
        self.assertIn("creationTouch", ios_home)
        self.assertNotIn("model.create(12)", ios_home)
        self.assertNotIn("model.create(24)", ios_home)

    def test_all_software_vaults_reuse_one_signing_implementation(self) -> None:
        hot = "\n".join(path.read_text() for path in sorted((ROOT / "crates/hot-wallet/src").rglob("*.rs")))
        vault = "\n".join(path.read_text() for path in sorted((ROOT / "crates/vault-runtime/src").rglob("*.rs")))
        web = "\n".join(path.read_text() for path in sorted((ROOT / "apps/kaskold-vault-web/src").rglob("*.rs")))
        self.assertIn("sign_transaction_multi_addr_with_entropy", hot)
        self.assertIn(".sign_transaction(request)", vault)
        self.assertIn("VaultRuntime", web)
        self.assertNotIn("schnorr_sign", web)
        self.assertNotIn("privateKey", web)

    def test_software_vault_menus_follow_hardware_production_graph_with_software_settings_projection(self) -> None:
        graph = json.loads((ROOT / "qa/config/workflow/production_ui_graph.json").read_text())
        menus = {menu["state"]: [item["label"] for item in menu["items"]] for menu in graph["menus"]}
        android_main = (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/MainActivity.kt").read_text()
        android_workflows = (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/WorkflowScreens.kt").read_text()
        ios = (ROOT / "apps/kaskold-vault-ios/KasKoldVault/Features/VaultHomeView.swift").read_text()

        android_ranges = {
            "MainMenu": (android_main, "private fun MainMenuScreen", "private fun ConnectScreen"),
            "SeedsMenu": (android_main, "private fun WalletMenuScreen", "private fun BackupMethodsScreen"),
            "WalletBackupMethodsMenu": (android_main, "private fun BackupMethodsScreen", "private fun RecoveryWordsScreen"),
            "BackupRecoveryMenu": (android_main, "private fun AdvancedBackupScreen", "private fun SecretTextScreen"),
            "WalletAdvancedMenu": (android_workflows, "internal fun AdvancedToolsMenuScreen", "internal fun SettingsWorkflowMenuScreen"),
            "SettingsMenu": (android_workflows, "internal fun SettingsWorkflowMenuScreen", "internal fun XprvExportMenuScreen"),
            "MultisigMenu": (android_workflows, "internal fun MultisigMenuScreen", "internal fun WalletInventoryScreen"),
            "SdImportMenu": (android_workflows, "internal fun RecoveryMenuScreen", "internal fun MultisigMenuScreen"),
            "XprvExportMenu": (android_workflows, "internal fun XprvExportMenuScreen", None),
        }
        ios_ranges = {
            "MainMenu": ("case .mainMenu:", "case .connect(let kpub):"),
            "SeedsMenu": ("case .walletMenu:", "case .backup(let phrase):"),
            "WalletBackupMethodsMenu": ("case .backupMethods:", "case .recoveryWords(let phrase):"),
            "BackupRecoveryMenu": ("case .advancedBackup:", "case .secretText(let title"),
            "WalletAdvancedMenu": ("case .walletAdvanced:", "case .settings:"),
            "SettingsMenu": ("case .settings:", "case .recoveryMenu:"),
            "SdImportMenu": ("case .recoveryMenu:", "case .multisigMenu:"),
            "MultisigMenu": ("case .multisigMenu:", "case .receive(let address"),
            "XprvExportMenu": ("case .xprvExport:", "case .secretText(let title"),
        }

        def segment(source: str, start_marker: str, end_marker: str | None) -> str:
            start = source.index(start_marker)
            return source[start:] if end_marker is None else source[start:source.index(end_marker, start)]

        def assert_labels_in_order(source: str, expected: list[str]) -> None:
            cursor = 0
            for label in expected:
                found = source.find(f'"{label}"', cursor)
                self.assertGreaterEqual(found, 0, f"missing hardware menu label {label!r}")
                cursor = found + len(label) + 2

        software_settings = [label for label in menus["SettingsMenu"] if label not in {"Display", "Audio", "Advanced"}]
        for menu_name, (source, start_marker, end_marker) in android_ranges.items():
            expected = software_settings if menu_name == "SettingsMenu" else menus[menu_name]
            assert_labels_in_order(segment(source, start_marker, end_marker), expected)
        for menu_name, bounds in ios_ranges.items():
            expected = software_settings if menu_name == "SettingsMenu" else menus[menu_name]
            assert_labels_in_order(segment(ios, *bounds), expected)

        android_settings = segment(android_workflows, "internal fun SettingsWorkflowMenuScreen", "internal fun XprvExportMenuScreen")
        ios_settings = segment(ios, "case .settings:", "case .recoveryMenu:")
        for hardware_only in ("Display", "Audio", "Advanced"):
            self.assertNotIn(f'"{hardware_only}"', android_settings)
            self.assertNotIn(f'"{hardware_only}"', ios_settings)
        for retired in ("AdvancedSettings", "OwnerFirmware", "FactoryResetConfirm"):
            self.assertNotIn(retired, android_main + android_workflows)
        for retired in ("case .advancedSettings", "case .ownerFirmware", "case .factoryResetConfirm"):
            self.assertNotIn(retired, ios)

        stale = "shown only for this explicit creation flow"
        self.assertNotIn(stale, android_main)
        self.assertNotIn(stale, ios)
        android_backup = segment(android_main, "private fun BackupScreen", "private fun MainMenuScreen")
        ios_backup = segment(ios, "case .backup(let phrase):", "case .backupMethods:")
        self.assertNotIn("kpub", android_backup.lower())
        self.assertNotIn("kpub", ios_backup.lower())

    def test_software_vault_shells_have_no_feature_placeholder_routes(self) -> None:
        web = (ROOT / "apps/kaskold-vault-web/web/index.html").read_text() + "\n" + "\n".join(
            path.read_text(errors="ignore") for path in sorted((ROOT / "apps/kaskold-vault-web/web/js").rglob("*.js"))
        )
        android = "\n".join(
            path.read_text(errors="ignore") for path in sorted((ROOT / "apps/kaskold-vault-android/app/src/main").rglob("*.kt"))
        )
        ios = "\n".join(
            path.read_text(errors="ignore") for path in sorted((ROOT / "apps/kaskold-vault-ios/KasKoldVault").rglob("*.swift"))
        )
        for source in (web, android, ios):
            lowered = source.lower()
            for forbidden in (
                "data-unavailable",
                "not available in this web vault build",
                "not yet implemented",
                "not yet exposed by the shared software vault runtime",
                "will not substitute the unrelated bip39 restore workflow",
            ):
                self.assertNotIn(forbidden, lowered)

        native = "\n".join(path.read_text(errors="ignore") for path in sorted((ROOT / "crates/vault-runtime/src/native_ffi").rglob("*.rs")))
        for operation in (
            '"receive_address"', '"wallets"', '"add_create_12"', '"add_create_24"', '"add_restore"',
            '"normalize_kpub"', '"validate_address"', '"normalize_covenant_backup"',
            '"create_multisig"', '"import_multisig"', '"bip85"', '"sign_message"',
            '"commit_secret"', '"decrypt_secret"', '"portable_backup"', '"stego_backup"',
        ):
            self.assertIn(operation, native)


    def test_all_vault_shells_use_shared_backup_runtime_exports(self) -> None:
        runtime = "\n".join(path.read_text() for path in sorted((ROOT / "crates/vault-runtime/src").rglob("*.rs")))
        native_root = ROOT / "crates/vault-runtime/src"
        native = "\n".join(
            path.read_text(errors="ignore")
            for path in (native_root / "native_ffi.rs", *(native_root / "native_ffi").rglob("*.rs"))
        )
        web = "\n".join(path.read_text() for path in sorted((ROOT / "apps/kaskold-vault-web/src").rglob("*.rs")))
        android = (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/runtime/RuntimeBridge.kt").read_text()
        ios = "\n".join(
            path.read_text(errors="ignore")
            for path in sorted((ROOT / "apps/kaskold-vault-ios/KasKoldVault").rglob("*.swift"))
        )
        for symbol in (
            "backup_recovery_phrase",
            "backup_seedqr",
            "backup_compact_seedqr",
            "backup_account_xprv",
            "backup_receive_private_key_hex",
        ):
            self.assertIn(symbol, runtime)
        for symbol in (
            "kaskold_vault_backup_words",
            "kaskold_vault_backup_seedqr",
            "kaskold_vault_backup_xprv",
            "kaskold_vault_export_receive_key",
        ):
            self.assertIn(symbol, native)
        self.assertIn("kaskold_vault_seedqr_svg", web)
        self.assertIn("nativeBackupSeedQr", android)
        self.assertIn("kkVaultBackupSeedQR", ios)


    def test_web_vault_exposes_real_hardware_menu_workflows_without_placeholders(self) -> None:
        graph = json.loads((ROOT / "qa/config/workflow/production_ui_graph.json").read_text())
        menus = {menu["state"]: [item["label"] for item in menu["items"]] for menu in graph["menus"]}
        html = (ROOT / "apps/kaskold-vault-web/web/index.html").read_text()
        javascript = "\n".join(
            path.read_text(errors="ignore")
            for path in sorted((ROOT / "apps/kaskold-vault-web/web/js").rglob("vault_*.js"))
        )
        rust = "\n".join(
            path.read_text(errors="ignore")
            for path in (
                ROOT / "apps/kaskold-vault-web/src/lib.rs",
                ROOT / "apps/kaskold-vault-web/src/workflows.rs",
            )
        )

        for forbidden in (
            "data-unavailable",
            "not available in this Web Vault build",
            "not yet implemented",
            "fails closed rather than",
        ):
            self.assertNotIn(forbidden.lower(), (html + javascript).lower())

        def menu_section(name: str) -> str:
            marker = f'data-workflow-menu="{name}"'
            start = html.index(marker)
            end = html.index("</section>", start)
            return html[start:end]

        for menu_name in (
            "MainMenu",
            "SeedsMenu",
            "WalletBackupMethodsMenu",
            "BackupRecoveryMenu",
            "WalletAdvancedMenu",
            "SettingsMenu",
            "MultisigMenu",
        ):
            section = menu_section(menu_name)
            cursor = 0
            expected = [label for label in menus[menu_name] if menu_name != "SettingsMenu" or label not in {"Display", "Audio", "Advanced"}]
            for label in expected:
                found = section.find(f">{label}<", cursor)
                self.assertGreaterEqual(found, 0, f"Web Vault missing {menu_name} label {label!r}")
                cursor = found + len(label) + 2
        web_settings = menu_section("SettingsMenu")
        for hardware_only in ("Display", "Audio", "Advanced"):
            self.assertNotIn(f">{hardware_only}<", web_settings)
        self.assertNotIn("security-device-storage", html)
        self.assertNotIn("security-rtc", html)

        for element_id in (
            "wallet-receive",
            "wallet-recovery",
            "wallet-switch",
            "wallet-multisig",
            "advanced-bip85",
            "advanced-sign-message",
            "advanced-commit-secret",
            "advanced-decrypt-secret",
            "backup-encrypted-sd",
            "advanced-stego",
        ):
            self.assertIn(f"$('" + element_id + "').onclick", javascript)

        for export in (
            "kaskold_vault_receive_address",
            "kaskold_vault_recover_material",
            "kaskold_vault_wallets",
            "kaskold_vault_switch_wallet",
            "kaskold_vault_multisig_kpub",
            "kaskold_vault_create_multisig",
            "kaskold_vault_import_multisig",
            "kaskold_vault_bip85",
            "kaskold_vault_sign_message",
            "kaskold_vault_commit_secret",
            "kaskold_vault_decrypt_secret",
            "kaskold_vault_portable_backup",
            "kaskold_vault_restore_portable",
            "kaskold_vault_stego_backup",
            "kaskold_vault_restore_stego",
        ):
            self.assertIn(f"pub fn {export}", rust)

    def test_steganography_codec_has_one_shared_owner_and_hardware_reexports_it(self) -> None:
        shared = (ROOT / "crates/shared-signer/src/stego_picture/mod.rs").read_text()
        shim = (ROOT / "crates/kaskold-hardware-core/src/backup/stego_picture.rs").read_text()
        self.assertIn("pub use shared_signer::stego_picture::*;", shim)
        self.assertIn("pub fn embed", shared)
        self.assertIn("pub fn extract", shared)
        self.assertFalse((ROOT / "crates/kaskold-hardware-core/src/backup/stego_picture").exists())

    def test_companion_has_one_watch_only_custody_mode(self) -> None:
        welcome = (ROOT / "apps/kaskold-companion-web/web/html/screens/system/welcome.html").read_text()
        header = (ROOT / "apps/kaskold-companion-web/web/html/document/open.html").read_text()
        self.assertNotIn("custody-mode-badge", header)
        self.assertIn("Manage Wallets", welcome)
        for forbidden in ("Session Hot Wallet", "Persistent Hot Wallet", "Store signing keys on this device"):
            self.assertNotIn(forbidden, welcome + header)

    def test_vault_review_is_a_separate_authorization_step(self) -> None:
        runtime = (ROOT / "crates/vault-runtime/src/signing_session.rs").read_text()
        accept = runtime[runtime.index("pub fn accept_qr_frame"):runtime.index("pub fn scan_progress")]
        approve = runtime[runtime.index("pub fn approve"):runtime.index("pub fn anti_klepto_awaiting_reveal")]
        self.assertNotIn("sign_transaction", accept)
        self.assertIn("review_transaction", accept)
        self.assertIn("sign_transaction", approve)
        self.assertIn("anti_klepto.approve_request", approve)

    def test_documented_matrix_does_not_claim_unperformed_physical_qualification(self) -> None:
        matrix = (ROOT / "docs/development/CUSTODY_TEST_MATRIX.md").read_text()
        self.assertIn("CoreS3 hardware-qualified", matrix)
        self.assertIn("CoreS3 Lite Hardware", matrix)
        self.assertIn("qualification pending", matrix)
        self.assertIn("physical cross-device release smoke required", matrix)
        self.assertIn("signed app/device smoke required", matrix)

    def test_firmware_uses_kaskold_hardware_product_identity(self) -> None:
        navigation = (ROOT / "apps/kaskold-hardware/src/ui/redraw/navigation.rs").read_text()
        about = (ROOT / "apps/kaskold-hardware/src/ui/screens/navigation/secondary.rs").read_text()
        qr = (ROOT / "apps/kaskold-hardware/src/ui/screens/signing/qr.rs").read_text()
        self.assertIn('Mainnet => "KASKOLD HARDWARE"', navigation)
        self.assertIn("format_args!(\"V{}\", vtxt.trim_start_matches('v'))", about)
        self.assertIn('let s4 = "kaspa.org";', about)
        self.assertNotIn('KasKold Hardware";', about)
        self.assertNotIn('Open Source | Rust | Air-Gapped', about)
        self.assertIn('"COMPANION QR"', qr)


if __name__ == "__main__":
    unittest.main()
