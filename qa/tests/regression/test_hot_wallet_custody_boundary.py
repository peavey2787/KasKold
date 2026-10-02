from __future__ import annotations

import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class VaultCustodyBoundaryTests(unittest.TestCase):
    def test_companion_apps_are_strictly_watch_only(self) -> None:
        manifest = tomllib.loads((ROOT / "apps/kaskold-companion-web/Cargo.toml").read_text(encoding="utf-8"))
        for forbidden in ("hot-wallet", "vault-runtime", "offline-signer"):
            self.assertNotIn(forbidden, manifest.get("dependencies", {}))
        web_root = ROOT / "apps/kaskold-companion-web/web"
        authored = "\n".join(path.read_text(encoding="utf-8", errors="ignore") for path in web_root.rglob("*") if path.is_file())
        for forbidden in ("Session Hot Wallet", "Persistent Hot Wallet", "kaskold_hot_wallet", "persistentHotWallet", "HotWallet", "hotWallet", "hot_wallet", "hot-wallet"):
            self.assertNotIn(forbidden, authored)
        self.assertFalse((web_root / "js/features/wallet/hot_wallet").exists())
        for relative in (
            "apps/kaskold-companion-android/app/src/main/java",
            "apps/kaskold-companion-ios/KasKold",
        ):
            source = "\n".join(path.read_text(encoding="utf-8", errors="ignore") for path in (ROOT / relative).rglob("*") if path.is_file())
            for forbidden in ("PersistentHotWalletStore", "persistentHotWallet", "kaskold_hot_wallet"):
                self.assertNotIn(forbidden, source)

    def test_watch_only_and_protocol_crates_cannot_depend_on_hot_wallet(self) -> None:
        for relative in (
            "crates/online-watcher/Cargo.toml",
            "crates/kaskold-sdk/Cargo.toml",
            "crates/kaskold-protocol/Cargo.toml",
            "crates/shared-signer/Cargo.toml",
        ):
            manifest = tomllib.loads((ROOT / relative).read_text(encoding="utf-8"))
            self.assertNotIn("hot-wallet", manifest.get("dependencies", {}), relative)

    def test_internal_hot_wallet_backend_exposes_secrets_only_through_explicit_backup_operations(self) -> None:
        source = "\n".join(path.read_text(encoding="utf-8") for path in sorted((ROOT / "crates/hot-wallet/src").rglob("*.rs")))
        self.assertIn("seed: Seed", source)
        self.assertNotIn("pub fn seed", source)
        self.assertNotIn("pub fn private_key", source)
        self.assertIn("pub fn backup_recovery_phrase", source)
        self.assertIn("pub fn backup_receive_private_key_hex", source)
        self.assertIn("sign_compact_kspt", source)
        self.assertIn("fill_random(&mut signing_entropy)", source)

    def test_vault_runtime_is_the_software_custody_facade(self) -> None:
        manifest = tomllib.loads((ROOT / "crates/vault-runtime/Cargo.toml").read_text(encoding="utf-8"))
        self.assertIn("hot-wallet", manifest.get("dependencies", {}))
        source = "\n".join(path.read_text(encoding="utf-8") for path in sorted((ROOT / "crates/vault-runtime/src").rglob("*.rs")))
        self.assertIn("create_wallet_12", source)
        self.assertIn("create_wallet_24", source)
        self.assertIn("restore_wallet", source)
        self.assertIn("sign_transaction", source)
        self.assertIn("prepare_anti_klepto", source)

    def test_vault_web_owns_browser_create_restore_and_signing(self) -> None:
        manifest = tomllib.loads((ROOT / "apps/kaskold-vault-web/Cargo.toml").read_text(encoding="utf-8"))
        self.assertIn("vault-runtime", manifest.get("dependencies", {}))
        self.assertNotIn("online-watcher", manifest.get("dependencies", {}))
        self.assertNotIn("kaskold-sdk", manifest.get("dependencies", {}))
        rust = "\n".join(path.read_text(encoding="utf-8") for path in sorted((ROOT / "apps/kaskold-vault-web/src").rglob("*.rs")))
        html = (ROOT / "apps/kaskold-vault-web/web/index.html").read_text(encoding="utf-8")
        for symbol in ("kaskold_vault_create_12", "kaskold_vault_restore", "kaskold_vault_approve"):
            self.assertIn(symbol, rust)
        for control in ('id="onboarding-create"', 'id="onboarding-12"', 'id="onboarding-24"', 'id="recovery-material-submit"', 'id="review-approve"'):
            self.assertIn(control, html)
        for asset in ('css/vault.css', 'js/vault_main.js', 'lib/vault_jsQR.js'):
            self.assertIn(asset, html)
        js = (ROOT / "apps/kaskold-vault-web/web/js/vault_main.js").read_text(encoding="utf-8")
        self.assertIn("import('../pkg/vault_web.js')", js)
        companion = ROOT / "apps/kaskold-companion-web/web"
        vault = ROOT / "apps/kaskold-vault-web/web"
        companion_assets = {
            path.relative_to(companion).as_posix()
            for path in companion.rglob("*")
            if path.is_file() and path.suffix.lower() in {".js", ".css"}
        }
        vault_assets = {
            path.relative_to(vault).as_posix()
            for path in vault.rglob("*")
            if path.is_file() and path.suffix.lower() in {".js", ".css"}
        }
        self.assertFalse(companion_assets & vault_assets)

    def test_software_vault_consumer_creation_and_navigation_contract(self) -> None:
        web = (ROOT / "apps/kaskold-vault-web/web/index.html").read_text(encoding="utf-8")
        web_css = (ROOT / "apps/kaskold-vault-web/web/css/vault.css").read_text(encoding="utf-8")
        web_main = (ROOT / "apps/kaskold-vault-web/web/js/vault_main.js").read_text(encoding="utf-8")
        recovery_words = (ROOT / "apps/kaskold-vault-web/web/js/workflows/vault_recovery_words.js").read_text(encoding="utf-8")
        web_wallets = (ROOT / "apps/kaskold-vault-web/web/js/vault_wallet_workflows.js").read_text(encoding="utf-8")
        onboarding = (ROOT / "apps/kaskold-vault-web/web/js/vault_onboarding.js").read_text(encoding="utf-8")
        runtime = (ROOT / "crates/vault-runtime/src/wallet_tools.rs").read_text(encoding="utf-8")
        android_creation = (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/CreationFlowScreens.kt").read_text(encoding="utf-8")
        android_controller = (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/runtime/VaultController.kt").read_text(encoding="utf-8")
        android_backup = (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/MainActivity.kt").read_text(encoding="utf-8")
        ios_creation = (ROOT / "apps/kaskold-vault-ios/KasKoldVault/Features/CreationFlowView.swift").read_text(encoding="utf-8")
        ios_model = (ROOT / "apps/kaskold-vault-ios/KasKoldVault/Features/VaultViewModel+Creation.swift").read_text(encoding="utf-8")

        for retired in (
            ">SIGNER<",
            "Create new recovery words securely.",
            "Words, SeedQR, local backup file, or advanced restore.",
            "Maximum 20 UTF-8 bytes, matching the M5 wallet slot limit.",
            "Web equivalent: password-encrypted ciphertext is stored locally in this browser.",
            "Wallet is cleared when this Vault session is locked or closed.",
            "Vault unlocked.",
        ):
            self.assertNotIn(retired, web + web_main)

        self.assertIn('id="screen-locked"', web)
        self.assertIn('<h2>Wallets</h2>', web)
        self.assertIn('id="wallet-add-open"', web)
        self.assertNotIn('id="wallet-list"', web)
        self.assertIn('openWalletPicker', web_main)
        self.assertIn('.screen h2 { text-align:center; }', web_css)
        self.assertLess(web.index('id="onboarding-24"'), web.index('id="onboarding-12"'))
        self.assertIn('let wordCount = 24;', onboarding)
        self.assertIn('normalizeRecoveryPhraseInput', recovery_words)
        self.assertIn('position !== index + 1', recovery_words)
        self.assertIn("normalizeRecoveryPhraseInput($('recovery-material').value)", web_wallets)
        self.assertIn('maxlength="64"', web)
        self.assertIn('WALLET_NAME_MAX: usize = 64;', runtime)
        self.assertIn('id="backup-show-all"', web)
        self.assertIn("$('backup-show-all').onclick = toggleAll;", recovery_words)
        self.assertIn('let showingAll = false;', recovery_words)
        self.assertIn('if (!showingAll && index + 1 < words.length)', recovery_words)
        self.assertIn('id="response-copy-all"', web)
        self.assertIn('balanced-address-line', web_css)
        self.assertIn("createRecoveryWordPager", web_main)
        self.assertIn('button:not(:disabled):hover', web_css)
        self.assertIn('border-color:var(--teal)', web_css)
        self.assertIn('id="saved-wallets"', web)
        self.assertIn('id="saved-wallet-list"', web)
        self.assertNotIn('id="unlock-saved"', web)
        self.assertIn('async function refreshSavedWalletList()', onboarding)
        self.assertIn('selectedSavedWalletId = wallet.id;', onboarding)
        self.assertIn("$('unlock-saved-credential').onkeydown", onboarding)
        self.assertIn("event.key !== 'Enter'", onboarding)
        self.assertLess(web.index('id="kpub-qr"'), web.index('id="home-kpub"'))
        self.assertIn('id="scan-cancel" data-back', web)
        self.assertIn('const saved = wallets.find(wallet => wallet.id === selectedSavedWalletId);', onboarding)
        self.assertIn('saveEncryptedWallet({', onboarding)
        self.assertNotIn('for (const saved of record.wallets)', onboarding)

        self.assertLess(android_creation.index('Text("24 Words")'), android_creation.index('Text("12 Words")'))
        self.assertIn('wordCount: Int = 24', android_controller)
        self.assertIn('1–64 UTF-8 bytes', android_controller)
        self.assertIn('"Show All"', android_backup)
        self.assertLess(ios_creation.index('Button("24 Words")'), ios_creation.index('Button("12 Words")'))
        self.assertIn('var wordCount = 24', ios_model)
        self.assertIn('1–64 UTF-8 bytes', ios_model)
        self.assertIn('"Show All"', ios_creation)

        source_wordmark = (ROOT / "branding/source/kaskold-vault-wordmark.png").read_bytes()
        self.assertEqual((ROOT / "apps/kaskold-vault-web/web/img/kaskold-wordmark.png").read_bytes(), source_wordmark)
        self.assertEqual((ROOT / "apps/kaskold-vault-android/app/src/main/res/drawable-nodpi/kaskold_wordmark.png").read_bytes(), source_wordmark)
        self.assertEqual((ROOT / "apps/kaskold-vault-ios/KasKoldVault/Assets.xcassets/KasKoldWordmark.imageset/kaskold-wordmark.png").read_bytes(), source_wordmark)
        self.assertNotIn('about-wordmark', (ROOT / "apps/kaskold-vault-web/web/js/vault_settings.js").read_text(encoding="utf-8"))
        self.assertIn('R.drawable.kaskold_wordmark', (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/MainActivity.kt").read_text(encoding="utf-8"))
        self.assertNotIn('state.title == "About"', (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/WorkflowScreens.kt").read_text(encoding="utf-8"))
        self.assertIn('Image("KasKoldWordmark")', (ROOT / "apps/kaskold-vault-ios/KasKoldVault/Features/VaultHomeView.swift").read_text(encoding="utf-8"))

    def test_software_vault_security_boundaries_are_fail_closed(self) -> None:
        review = (ROOT / "crates/hot-wallet/src/transaction_review.rs").read_text(encoding="utf-8")
        signing = (ROOT / "crates/hot-wallet/src/transaction_signing.rs").read_text(encoding="utf-8")
        session = (ROOT / "crates/vault-runtime/src/signing_session.rs").read_text(encoding="utf-8")
        web = (ROOT / "apps/kaskold-vault-web/web/index.html").read_text(encoding="utf-8")
        onboarding = (ROOT / "apps/kaskold-vault-web/web/js/vault_onboarding.js").read_text(encoding="utf-8")
        android = (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/MainActivity.kt").read_text(encoding="utf-8")
        ios = (ROOT / "apps/kaskold-vault-ios/KasKoldVault/Features/VaultViewModel.swift").read_text(encoding="utf-8")
        ios_project = (ROOT / "apps/kaskold-vault-ios/KasKoldVault.xcodeproj/project.pbxproj").read_text(encoding="utf-8")

        for token in (
            "transaction.version != 1", "transaction.locktime != 0",
            "transaction.subnetwork_id != SUBNETWORK_ID_NATIVE", "transaction.gas != 0",
            "!transaction.payload.is_empty()", "input.sequence != u64::MAX",
            "input.sig_op_count != 1", "output.has_covenant",
        ):
            self.assertIn(token, review)
        self.assertIn("validate_generic_vault_semantics(&parsed.transaction)?;", signing)
        self.assertGreaterEqual(session.count("self.check_session_signing_policy(now_unix)?;"), 2)

        self.assertNotIn('id="storage-use-pin"', web)
        self.assertIn("value.length >= 12", onboarding)
        self.assertIn("credentialType !== 'password'", onboarding)

        self.assertIn("createConfirmDeviceCredentialIntent", android)
        self.assertIn("VaultScreenState.Locked", android)
        self.assertNotIn("mutableStateOf(controller.unlockPersisted())", android)
        self.assertIn("LAContext()", ios)
        self.assertIn(".deviceOwnerAuthentication", ios)
        self.assertIn("failureReturn = .locked", ios)
        self.assertIn("INFOPLIST_KEY_NSFaceIDUsageDescription", ios_project)

    def test_companion_and_vault_do_not_force_ui_text_uppercase(self) -> None:
        roots = (
            ROOT / "apps/kaskold-companion-web/web/css",
            ROOT / "apps/kaskold-vault-web/web/css",
        )
        for root in roots:
            for path in root.rglob("*.css"):
                self.assertNotIn("text-transform: uppercase", path.read_text(encoding="utf-8"), path.as_posix())
        vault_html = (ROOT / "apps/kaskold-vault-web/web/index.html").read_text(encoding="utf-8")
        self.assertIn("<h2>Wallets</h2>", vault_html)
        self.assertNotIn("<h2>WALLETS</h2>", vault_html)

    def test_companion_startup_is_kpub_management_and_watch_only_tools(self) -> None:
        welcome = (ROOT / "apps/kaskold-companion-web/web/html/screens/system/welcome.html").read_text(encoding="utf-8")
        self.assertIn("Manage Wallets", welcome)
        self.assertIn("Multisig", welcome)
        self.assertNotIn("Multisig / Multi-spend", welcome)
        self.assertIn("Broadcast TX", welcome)
        self.assertNotIn("Watch only · no private keys", welcome)
        for forbidden in ("Create 12", "Create 24", "Restore wallet", "Hot Wallet"):
            self.assertNotIn(forbidden, welcome)


if __name__ == "__main__":
    unittest.main()
