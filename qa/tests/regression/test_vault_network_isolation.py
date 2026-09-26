from __future__ import annotations

import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class VaultNetworkIsolationTests(unittest.TestCase):
    def test_vault_runtime_has_no_network_dependencies(self) -> None:
        manifest = tomllib.loads((ROOT / "crates/vault-runtime/Cargo.toml").read_text())
        dependencies = set(manifest.get("dependencies", {}))
        for forbidden in (
            "online-watcher",
            "reqwest",
            "web-sys",
            "tokio",
            "tungstenite",
            "tokio-tungstenite",
            "wasm-bindgen",
        ):
            self.assertNotIn(forbidden, dependencies)

    def test_vault_runtime_source_has_no_network_surface(self) -> None:
        source = "\n".join(
            path.read_text(errors="ignore")
            for path in (ROOT / "crates/vault-runtime/src").rglob("*.rs")
        ).lower()
        for forbidden in (
            "use reqwest",
            "use web_sys",
            "use tokio",
            "use tungstenite",
            "extern crate reqwest",
            "broadcast_transaction(",
        ):
            self.assertNotIn(forbidden, source)




    def test_vault_runtime_requires_review_then_explicit_approval(self) -> None:
        runtime = "\n".join(path.read_text(errors="ignore") for path in sorted((ROOT / "crates/vault-runtime/src").rglob("*.rs")))
        native_root = ROOT / "crates/vault-runtime/src"
        ffi = "\n".join(
            path.read_text(errors="ignore")
            for path in (native_root / "native_ffi.rs", *(native_root / "native_ffi").rglob("*.rs"))
        )
        self.assertIn("pending_request: Option<Zeroizing<Vec<u8>>>", runtime)
        self.assertIn("accept_qr_frame", runtime)
        self.assertIn("review_transaction", runtime)
        self.assertIn("pub fn approve", runtime)
        self.assertIn("pub fn reject", runtime)
        self.assertIn("SigningSessionAlreadyComplete", runtime)
        self.assertIn("kaskold_vault_accept_frame", ffi)
        self.assertIn("kaskold_vault_approve", ffi)
        self.assertNotIn("kaskold_vault_sign", ffi)
        self.assertNotIn("kaskold_vault_seed", ffi)
        self.assertNotIn("kaskold_vault_private_key", ffi)

    def test_android_vault_links_rust_runtime_and_has_real_qr_review_ui(self) -> None:
        gradle = (ROOT / "apps/kaskold-vault-android/app/build.gradle.kts").read_text()
        cpp = (ROOT / "apps/kaskold-vault-android/app/src/main/cpp/vault_jni.cpp").read_text()
        controller = (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/runtime/VaultController.kt").read_text()
        main = (ROOT / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/MainActivity.kt").read_text()
        self.assertIn('build", "-p", "vault-runtime", "--release", "--locked"', gradle)
        cmake = (ROOT / "apps/kaskold-vault-android/app/src/main/cpp/CMakeLists.txt").read_text()
        self.assertIn("vault_runtime_jni", cmake)
        self.assertIn("kaskold_vault_accept_frame", cpp)
        self.assertIn("kaskold_vault_approve", cpp)
        self.assertIn("runtime.acceptQrFrame", controller)
        self.assertIn("runtime.approve", controller)
        self.assertIn("VaultScreenState.Review", main)
        self.assertNotIn("integration pending", main.lower())

    def test_ios_vault_links_rust_runtime_and_has_real_qr_review_ui(self) -> None:
        project = (ROOT / "apps/kaskold-vault-ios/KasKoldVault.xcodeproj/project.pbxproj").read_text()
        ios_root = ROOT / "apps/kaskold-vault-ios/KasKoldVault"
        source = "\n".join(path.read_text(errors="ignore") for path in sorted(ios_root.rglob("*.swift")))
        build_script = (ROOT / "apps/kaskold-vault-ios/scripts/build-vault-runtime.sh").read_text()
        self.assertIn("Build Rust Vault Runtime", project)
        self.assertIn("-lvault_runtime", project)
        self.assertIn("cargo", build_script)
        self.assertIn("kaskold_vault_accept_frame", source)
        self.assertIn("AVCaptureSession", source)
        self.assertIn("Approve & Sign", source)
        for swift_file in ("VaultViewModel.swift", "VaultViewModel+Workflows.swift", "VaultWorkflowSupport.swift", "QRScanner.swift", "VaultRuntimeBridge.swift"):
            self.assertIn(f"/* {swift_file} */ = {{isa = PBXFileReference", project)
            self.assertIn(f"/* {swift_file} in Sources */", project)
        self.assertNotIn("runtime not linked", source.lower())
        self.assertNotIn("integration pending", source.lower())

    def test_android_vault_manifest_has_no_internet_capability(self) -> None:
        manifest = (
            ROOT
            / "apps/kaskold-vault-android/app/src/main/AndroidManifest.xml"
        ).read_text()
        self.assertNotIn("android.permission.INTERNET", manifest.replace(
            "<!-- Deliberately NO android.permission.INTERNET. This is a security boundary. -->",
            "",
        ))
        self.assertIn('android:usesCleartextTraffic="false"', manifest)

    def test_android_vault_has_no_network_client_dependency(self) -> None:
        build = (ROOT / "apps/kaskold-vault-android/app/build.gradle.kts").read_text().lower()
        source = "\n".join(
            path.read_text(errors="ignore").lower()
            for path in (ROOT / "apps/kaskold-vault-android/app/src").rglob("*.kt")
        )
        for forbidden in (
            "okhttp", "retrofit", "ktor-client", "websocket", "urlconnection",
            "httpurlconnection", "java.net.", "android.net.",
        ):
            self.assertNotIn(forbidden, build + "\n" + source)

    def test_android_vault_persists_only_rust_sealed_wallet_ciphertext(self) -> None:
        storage = (
            ROOT
            / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/security/VaultBlobStore.kt"
        ).read_text()
        key_store = (
            ROOT
            / "apps/kaskold-vault-android/app/src/main/java/com/kaskold/vault/security/VaultWrappingKeyStore.kt"
        ).read_text()
        self.assertIn("writeSealedWallet", storage)
        self.assertIn("CURRENT_SEALED_WALLET_LENGTH = 216", storage)
        self.assertIn("LEGACY_SEALED_WALLET_LENGTH = 96", storage)
        self.assertIn("INVENTORY_MAGIC", storage)
        self.assertNotIn("Cipher.getInstance", storage)
        self.assertNotIn("SecretKey", storage)
        self.assertNotIn("write(plaintext", storage)
        self.assertNotIn("writeSeed", storage)
        self.assertNotIn("writeMnemonic", storage)
        self.assertIn("AndroidKeyStore", key_store)
        self.assertIn("wrappingKey.fill(0)", key_store)
        self.assertIn("encoded.fill(0)", key_store)

    def test_vault_runtime_seals_seed_before_native_persistence(self) -> None:
        runtime = "\n".join(path.read_text(errors="ignore") for path in sorted((ROOT / "crates/vault-runtime/src").rglob("*.rs")))
        hot_wallet = "\n".join(
            path.read_text(errors="ignore")
            for path in (ROOT / "crates/hot-wallet/src").rglob("*.rs")
        )
        self.assertIn("seal_wallet", runtime)
        self.assertIn("unlock_sealed_wallet", runtime)
        self.assertIn("seal_native_inventory", runtime)
        self.assertIn("unlock_native_inventory", runtime)
        self.assertIn('b"KVI1"', runtime)
        self.assertIn("seal_for_platform", hot_wallet)
        self.assertIn("restore_platform_sealed", hot_wallet)
        self.assertIn('b"KasKold/vault/platform-sealed-wallet/v2"', hot_wallet)
        self.assertIn('b"KasKold/vault/platform-sealed-wallet/v1"', hot_wallet)
        self.assertNotIn("pub fn seed", hot_wallet)
        self.assertNotIn("pub fn private_key", hot_wallet)

    def test_ios_vault_is_a_separate_network_free_application(self) -> None:
        root = ROOT / "apps/kaskold-vault-ios"
        self.assertTrue((root / "KasKoldVault.xcodeproj/project.pbxproj").is_file())
        source = "\n".join(path.read_text(errors="ignore") for path in (root / "KasKoldVault").rglob("*.swift"))
        for forbidden in (
            "URLSession", "NWConnection", "WebSocket", "CFNetwork", "import Network",
            "WKWebView", "SFSafariViewController",
        ):
            self.assertNotIn(forbidden, source)
        project = (root / "KasKoldVault.xcodeproj/project.pbxproj").read_text()
        self.assertIn("com.kaskold.vault", project)
        self.assertIn("NSCameraUsageDescription", project)

    def test_ios_vault_persists_only_rust_sealed_ciphertext(self) -> None:
        source = (
            ROOT
            / "apps/kaskold-vault-ios/KasKoldVault/Infrastructure/Security/VaultBlobStore.swift"
        ).read_text()
        self.assertIn("writeSealedWallet", source)
        self.assertIn("kSecAttrAccessibleWhenUnlockedThisDeviceOnly", source)
        self.assertIn("completeFileProtection", source)
        self.assertNotIn("CryptoKit", source)
        self.assertNotIn("AES.GCM", source)
        self.assertNotIn("writeSeed", source)
        self.assertNotIn("writeMnemonic", source)


if __name__ == "__main__":
    unittest.main()
