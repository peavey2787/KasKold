from __future__ import annotations

import re
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def read(relative: str) -> str:
    return (ROOT / relative).read_text(errors="ignore")


class CredentialKdfSecurityTests(unittest.TestCase):
    def test_bip39_remains_exactly_standard_pbkdf2_sha512(self) -> None:
        source = read("crates/offline-signer/src/derivation/bip39/seed.rs")
        self.assertIn("BIP39_PBKDF2_ROUNDS: u16 = 2048", source)
        self.assertIn("pbkdf2_hmac_sha512", source)
        self.assertIn("iterations=2048", source)
        self.assertIn("dklen=64", source)
        self.assertIn("hmac_sha512", source)
        self.assertNotIn("password_kdf", source)
        self.assertNotIn("Argon2", source)

    def test_extended_private_key_trait_surface_is_not_widened_for_tests(self) -> None:
        source = read("crates/offline-signer/src/derivation/bip32/extended_private.rs")
        xpub_tests = read("crates/offline-signer/src/derivation/unit_tests/xpub_tests.rs")
        bip32_tests = read("crates/offline-signer/src/derivation/unit_tests/bip32_tests.rs")
        declaration = source[source.index("#[derive("):source.index("pub struct ExtendedPrivKey")]
        self.assertIn("Zeroize", declaration)
        self.assertIn("ZeroizeOnDrop", declaration)
        self.assertNotIn("Debug", declaration)
        self.assertNotIn("PartialEq", declaration)
        for tests in (xpub_tests, bip32_tests):
            self.assertIn("use crate::derivation::bip32::Bip32Error;", tests)
            self.assertIn("matches!", tests)
            self.assertIn("Err(Bip32Error::InvalidKey)", tests)
        self.assertNotRegex(
            bip32_tests,
            re.compile(r"assert_eq!\s*\(\s*derive_multisig_(?:account|address)_key[\s\S]{0,180}Err\(Bip32Error::InvalidKey\)"),
        )

    def test_current_compile_profile_imports_match_argon2_and_portable_owners(self) -> None:
        stego_import = read("apps/kaskold-hardware/src/runtime/interactions/stego/import_decrypt.rs")
        wallet_crypto = read("apps/kaskold-hardware/src/services/persistent_wallet/crypto.rs")
        device_tests = read("crates/offline-signer/src/crypto/unit_tests/device_bound_storage_tests.rs")
        credential_tests = read("crates/kaskold-hardware-core/src/unit_tests/credential_policy_tests.rs")

        self.assertIn("handle_portable_password(ad, boot_display, delay, liveness, i2c, x, y, is_back)", stego_import)
        self.assertRegex(
            stego_import,
            re.compile(r"fn handle_portable_password\([\s\S]{0,240}liveness: &mut dyn FnMut\(\)[\s\S]{0,120}i2c: &mut esp_hal::i2c::master::I2c"),
        )
        self.assertIn("try_alternate_picture_portable(ad, boot_display, delay, liveness, i2c)", stego_import)
        self.assertNotIn("        password_kdf,", wallet_crypto)
        self.assertIn(
            "use crate::crypto::password_kdf::{derive_key_32, PasswordKdfPurpose};",
            device_tests,
        )
        self.assertNotIn("credential_policy::{self, CredentialKind}", device_tests)
        header = credential_tests.split("#[test]", 1)[0]
        self.assertNotIn("SALT_SIZE", header)

    def test_current_password_kdf_is_central_argon2id_v19(self) -> None:
        source = read("crates/offline-signer/src/crypto/password_kdf.rs")
        manifest = read("crates/offline-signer/Cargo.toml")
        for token in (
            'argon2 = { version = "=0.5.3", default-features = false, features = ["zeroize"] }',
            "Algorithm::Argon2id", "Version::V0x13", "ARGON2_VERSION_13: u8 = 0x13",
            "PortableBackup", "PersistentWallet",
            "EncryptedTransport", "DeviceBoundBackup",
            "try_reserve_exact", "AllocationFailed", "derive_key_32_with_workspace",
            "workspace_block_count", "zeroize_workspace", "PasswordKdfBlock",
        ):
            self.assertIn(token, manifest + source)
        self.assertNotIn("pbkdf2_hmac", source)

    def test_non_bip39_pbkdf2_compatibility_is_fully_retired(self) -> None:
        self.assertFalse((ROOT / "crates/offline-signer/src/crypto/pbkdf2.rs").exists())
        self.assertFalse((ROOT / "crates/offline-signer/src/crypto/legacy_pbkdf2.rs").exists())
        self.assertFalse((ROOT / "crates/offline-signer/src/crypto/unit_tests/legacy_pbkdf2_tests.rs").exists())
        offenders = []
        for root_name in ("apps", "crates"):
            for path in (ROOT / root_name).rglob("*.rs"):
                rel = path.relative_to(ROOT).as_posix()
                text = path.read_text(errors="ignore")
                if "legacy_pbkdf2" in text or "derive_legacy_32" in text or "LEGACY_PBKDF2" in text:
                    offenders.append(rel)
        self.assertEqual(offenders, [])

    def test_current_formats_are_argon2_only_and_reject_retired_pbkdf2_magics(self) -> None:
        backup = read("apps/kaskold-hardware/src/services/backup/container.rs")
        framing = read("crates/offline-signer/src/crypto/container_framing.rs")
        framing_tests = read("crates/offline-signer/src/crypto/unit_tests/container_framing_tests.rs")
        kspt = read("apps/kaskold-hardware/src/runtime/interactions/sd/exports/kspt_export/crypto.rs")
        wallet = read("apps/kaskold-hardware/src/services/persistent_wallet/crypto.rs")
        sd_wallet = read("apps/kaskold-hardware/src/services/persistent_wallet/sd_backend.rs")
        self.assertIn('BACKUP_CURRENT_MAGIC: [u8; 8] = *b"KASDB005"', framing)
        self.assertIn("header.parameters", backup)
        self.assertNotIn("BackupReaderKdf", framing + backup)
        self.assertNotIn("BACKUP_LEGACY_MAGIC", framing + backup)
        self.assertIn('TRANSPORT_CURRENT_MAGIC: [u8; 4] = *b"KAS\\x04"', framing)
        self.assertNotIn("TRANSPORT_LEGACY_MAGIC", framing + kspt)
        self.assertIn("parse_transport_header", kspt)
        self.assertIn('CURRENT_MAGIC: [u8; 8] = *b"KSWLT004"', wallet)
        self.assertNotIn("KSWLT003", wallet)
        self.assertRegex(
            sd_wallet,
            re.compile(r'const\s+CURRENT_MAGIC\s*:\s*\[u8\s*;\s*4\]\s*=\s*\*b"KSW4"\s*;'),
        )
        self.assertNotIn("open_legacy", sd_wallet)
        self.assertIn('b"KASDB004"', framing_tests)
        self.assertIn('b"KAS\\x03"', framing_tests)
        for source in (backup, framing, kspt, wallet, sd_wallet):
            self.assertNotIn("legacy_pbkdf2", source)
            self.assertNotIn("derive_legacy_32", source)

    def test_current_kdf_profile_is_versioned_v2_and_legacy_v1_is_read_only(self) -> None:
        source = read("crates/offline-signer/src/crypto/password_kdf.rs")
        portable = read("crates/hot-wallet/src/portable_backup.rs")
        for token in (
            "PROFILE_VERSION_2: u8 = 2",
            "V2_MEMORY_KIB: u32 = 8_192",
            "V2_TIME_COST: u32 = 3",
            "profile_version: PROFILE_VERSION_2",
            "pub const fn is_supported",
        ):
            self.assertIn(token, source)
        self.assertIn("if !parameters.is_current()", source)
        self.assertIn("derive_key_32_with_params", portable)
        self.assertIn("let parameters = parse_metadata", portable)

    def test_portable_jpeg_is_password_only_and_authenticates_kdf_metadata(self) -> None:
        payload = read("apps/kaskold-hardware/src/services/stego/payload.rs")
        portable = read("apps/kaskold-hardware/src/services/stego/portable.rs")
        prompts = read("apps/kaskold-hardware/src/ui/screens/storage/steganography/prompts.rs")
        docs = read("docs/security/STEGANOGRAPHY.md")
        self.assertRegex(
            payload,
            re.compile(r"const\s+PORTABLE_FORMAT_VERSION\s*:\s*u8\s*=\s*4\s*;"),
        )
        self.assertIn("password_kdf::encode_metadata", payload)
        self.assertIn("password_kdf::parse_metadata", payload)
        self.assertRegex(
            payload,
            re.compile(r"build_aad\s*\(\s*&output\[\.\.HEADER_SIZE\]\s*,\s*descriptor\s*\)"),
        )
        self.assertIn("PasswordKdfPurpose::PortableBackup", portable)
        self.assertIn("PORTABLE BACKUP", prompts)
        self.assertIn("Restore: JPEG + Password", prompts)
        self.assertIn("Works on another KasKold", prompts)
        self.assertIn("offline password-guessing target", docs)
        combined = payload + portable + prompts
        self.assertNotIn("portable_recovery", combined)
        self.assertNotIn("RECOVERY_KEY_SIZE", combined)
        self.assertNotIn("StegoPortableRecoveryKey", combined)

    def test_argon2_benchmark_is_dev_only_and_compiled_for_both_boards(self) -> None:
        bench = read("apps/kaskold-hardware/src/diagnostics/argon2_bench.rs")
        navigation = read("apps/kaskold-hardware/src/runtime/navigation/production.rs")
        controller = read("apps/kaskold-hardware/src/runtime/interactions/menu/primary/production.rs")
        matrix = read("tools/build/firmware/build_matrix.py")
        policy = read("apps/kaskold-hardware/src/feature_policy.rs")
        manifest = read("apps/kaskold-hardware/Cargo.toml")
        for token in (
            "fixed non-secret calibration inputs only", "Argon2id", "version=19",
            "2_048", "3_072", "4_096", "5_120", "6_144",
            "free_before", "free_after", "largest_before", "largest_after",
            "cycles", "ms", "watchdog_ok", "vector_ok",
        ):
            self.assertIn(token, bench)
        self.assertIn('"Argon2 Bench"', navigation)
        self.assertIn('label == "Argon2 Bench"', controller)
        self.assertIn('argon2-bench = ["diagnostics"]', manifest)
        self.assertIn('FirmwareBuild("m5stack,argon2-bench")', matrix)
        self.assertIn('FirmwareBuild("m5stack-lite")', matrix)
        self.assertNotIn('FirmwareBuild("m5stack-lite,production")', matrix)
        self.assertIn('feature = "argon2-bench"', policy)
        self.assertIn('feature = "silent"', policy)

    def test_sha_benchmark_no_longer_contains_pbkdf2_calibration(self) -> None:
        source = read("apps/kaskold-hardware/src/diagnostics/sha_bench.rs")
        self.assertIn("SHA-256", source)
        self.assertNotIn("pbkdf2_sha256", source)
        self.assertNotIn("legacy_pbkdf2", source)
        self.assertNotIn("100_000", source)
        self.assertIn("Argon2 Bench", source)

    def test_argon2_and_backup_tamper_vectors_are_present(self) -> None:
        password_tests = read("crates/offline-signer/src/crypto/unit_tests/password_kdf_tests.rs")
        backup_tests = read("apps/kaskold-hardware/src/services/unit_tests/backup_tests.rs")
        kspt_tests = read("apps/kaskold-hardware/src/runtime/interactions/sd/exports/kspt_export/crypto/unit_tests/mod.rs")
        for token in (
            "known_answers", "parameter_downgrade", "unknown_kdf",
            "allocation_failure", "invalid_password_lengths",
        ):
            self.assertIn(token, password_tests)
        for token in (
            "portable_jpeg_is_password_only_cross_device", "wrong_password",
            "bad_kdf", "tamper",
        ):
            self.assertIn(token, backup_tests)
        for token in (
            "current_envelope_round_trip_authenticates_kdf_metadata_and_has_no_fallback",
            "historical_pbkdf2_transport_magic_is_rejected", "unsupported", "WrongHorse9",
        ):
            self.assertIn(token, kspt_tests + read("apps/kaskold-hardware/src/runtime/interactions/sd/exports/kspt_export/crypto.rs"))


if __name__ == "__main__":
    unittest.main()
