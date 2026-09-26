from __future__ import annotations

import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
VAULT = ROOT / "apps/kaskold-vault-android"


class AndroidVaultIsolationTests(unittest.TestCase):
    def test_manifest_has_no_internet_permission(self) -> None:
        manifest = (VAULT / "app/src/main/AndroidManifest.xml").read_text()
        self.assertNotIn("android.permission.INTERNET", manifest.replace(
            "<!-- Deliberately NO android.permission.INTERNET. This is a security boundary. -->", ""
        ))
        self.assertIn("android.permission.CAMERA", manifest)
        self.assertIn('android:usesCleartextTraffic="false"', manifest)

    def test_gradle_has_no_network_client_dependencies(self) -> None:
        source = (VAULT / "app/build.gradle.kts").read_text().lower()
        for forbidden in ("okhttp", "retrofit", "ktor-client", "androidx.webkit", "websocket"):
            self.assertNotIn(forbidden, source)

    def test_keystore_protects_rust_sealing_key_while_blob_store_accepts_only_ciphertext(self) -> None:
        blob = (VAULT / "app/src/main/java/com/kaskold/vault/security/VaultBlobStore.kt").read_text()
        wrapping = (VAULT / "app/src/main/java/com/kaskold/vault/security/VaultWrappingKeyStore.kt").read_text()
        self.assertIn("CURRENT_SEALED_WALLET_LENGTH = 216", blob)
        self.assertIn("LEGACY_SEALED_WALLET_LENGTH = 96", blob)
        self.assertIn("INVENTORY_MAGIC", blob)
        self.assertIn("isInventory(sealedWallet) || isCurrent(sealedWallet)", blob)
        self.assertIn("isInventory(sealedWallet) || isCurrent(sealedWallet) || isV2(sealedWallet) || isLegacy(sealedWallet)", blob)
        self.assertIn("invalid sealed Vault wallet container", blob)
        for forbidden in ("mnemonic:", "seed:", "xprv:"):
            self.assertNotIn(forbidden, blob.lower())
        self.assertIn('KeyStore.getInstance("AndroidKeyStore")', wrapping)
        self.assertIn('Cipher.getInstance("AES/GCM/NoPadding")', wrapping)
        self.assertIn("wrappingKey.fill(0)", wrapping)
        self.assertIn("ciphertext.fill(0)", wrapping)


if __name__ == "__main__":
    unittest.main()
