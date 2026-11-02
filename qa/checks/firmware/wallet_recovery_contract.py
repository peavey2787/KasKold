"""Typed wallet recovery and session-bound QR source contracts."""

from __future__ import annotations

import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[3] / "qa/checks"))
from portal_source import kaskold_source  # noqa: E402

from pathlib import Path
import re


def _read(relative: str) -> str:
    return kaskold_source(str(relative)).read_text(encoding="utf-8")


def _require(errors: list[str], condition: bool, message: str) -> None:
    if not condition:
        errors.append(message)


def _check_seed_source_types(root: Path, errors: list[str]) -> None:
    source = _read("apps/kaskold-hardware/src/wallet/seed_manager/source.rs")
    for variant in (
        "Mnemonic12",
        "Mnemonic24",
        "RawPrivateKey",
        "AccountXprv",
    ):
        _require(errors, variant in source, f"wallet recovery: missing typed source {variant}")

    firmware = root / "apps/kaskold-hardware/src"
    firmware_source = "\n".join(
        path.read_text(encoding="utf-8") for path in firmware.rglob("*.rs")
    )
    for sentinel_value in (1, 2):
        _require(
            errors,
            re.search(rf"\bword_count\s*==\s*{sentinel_value}\b", firmware_source)
            is None,
            "wallet recovery: source typing regressed to mnemonic sentinel "
            f"word_count == {sentinel_value}",
        )


def _check_derivation_contracts(errors: list[str]) -> None:
    derivation = _read("apps/kaskold-hardware/src/runtime/signing/derivation.rs")
    wallet_keys = _read("apps/kaskold-hardware/src/services/wallet_keys.rs")
    for required in (
        "derive_slot_account_key",
        "serialize_active_xprv",
    ):
        _require(errors, required in derivation, f"wallet recovery: missing {required}")
    _require(
        errors,
        'Err("Wallet source is not a mnemonic")' in wallet_keys
        and "pub(crate) use crate::services::wallet_keys" in derivation,
        "wallet recovery: mnemonic source rejection must remain owned by the wallet-key service",
    )

    checkpoint_derivation = _read(
        "apps/kaskold-hardware/src/runtime/signing/derivation/checkpoint.rs"
    )
    kpub_deferred = _read(
        "apps/kaskold-hardware/src/runtime/event_loop/runner/deferred/kpub.rs"
    )
    wallet_key_worker = _read(
        "apps/kaskold-hardware/src/services/wallet_keys/worker.rs"
    )
    _require(
        errors,
        "begin_mnemonic_seed" in checkpoint_derivation
        and "SeedDerivation" in checkpoint_derivation
        and "kpub_seed_derivation" in kpub_deferred
        and "work.advance(KPUB_PBKDF2_ROUNDS_PER_STEP)" in kpub_deferred
        and "submit_seed(&mut seed)" in kpub_deferred
        and "serialize_account_kpub" in wallet_key_worker,
        "wallet recovery: cooperative PBKDF2 and canonical kpub finalization are missing",
    )

    strategy = _read("apps/kaskold-hardware/src/runtime/signing/strategy.rs")
    _require(
        errors,
        "AccountKey" in strategy and "slot.is_account_key()" in strategy,
        "wallet recovery: imported account XPrv lacks its own signing strategy",
    )


def _check_signing_paths(errors: list[str]) -> None:
    kspt = _read("apps/kaskold-hardware/src/runtime/signing/kspt.rs")
    pskt = _read("apps/kaskold-hardware/src/runtime/signing/pskt.rs")
    workflow = _read("apps/kaskold-hardware/src/runtime/signing/workflow.rs")
    _require(
        errors,
        "sign_account_input_with_entropy" in kspt
        and "SigningStrategy::AccountKey | super::strategy::SigningStrategy::Mnemonic" in kspt,
        "wallet recovery: mnemonic/account XPrv signing is absent from the shared per-input signer",
    )
    _require(
        errors,
        "sign_matching_input_in_place_with_entropy" in kspt,
        "wallet recovery: raw keys must sign only the matching P2PK input",
    )
    _require(
        errors,
        "kspt::sign_input(ad, signing_strategy, input_idx, &signing_entropy, liveness)" in workflow
        and "serialize_pskt_vec" in pskt,
        "wallet recovery: PSKT and KSPT must share the authoritative per-input signing path",
    )


def _check_account_export(errors: list[str]) -> None:
    fingerprint = _read("crates/offline-signer/src/derivation/xpub/fingerprint.rs")
    _require(
        errors,
        "Ripemd160" in fingerprint
        and "Sha256" in fingerprint
        and "parent_fingerprint" in fingerprint,
        "wallet recovery: original v1.0.5 exports require standard BIP32 HASH160 fingerprints",
    )
    _require(
        errors,
        "fn test_account_export_vectors()" in _read("crates/offline-signer/src/self_test/xpub.rs"),
        "wallet recovery: fixed original-v1.0.5 kpub/XPrv vectors are missing",
    )

    xprv = _read("crates/offline-signer/src/derivation/xpub/xprv.rs")
    _require(
        errors,
        "ImportedAccountXprv" in xprv
        and "parent_fingerprint" in xprv
        and "serialize_imported_xprv" in xprv,
        "wallet recovery: imported account XPrv metadata is not preserved",
    )


def _check_backup_container(root: Path, errors: list[str]) -> None:
    _read("apps/kaskold-hardware/src/services/backup/mod.rs")
    backup_container = _read("apps/kaskold-hardware/src/services/backup/container.rs")
    backup_framing = _read("crates/offline-signer/src/crypto/container_framing.rs")
    backup_contract = backup_container + "\n" + backup_framing
    backup_seed = _read("apps/kaskold-hardware/src/services/backup/seed.rs")
    backup_xprv = _read("apps/kaskold-hardware/src/services/backup/xprv.rs")
    backup_tests = _read("apps/kaskold-hardware/src/services/unit_tests/backup_tests.rs")
    _require(
        errors,
        (root / "apps/kaskold-hardware/src/services/backup/container.rs").exists()
        and not (root / "apps/kaskold-hardware/src/services/backup/legacy.rs").exists(),
        "wallet recovery: current device-bound container must exist without a generic legacy facade",
    )
    for required in (
        'KASDB005', "KDF_ID_DEVICE_HMAC_SHA256",
        "PasswordKdfPurpose::DeviceBoundBackup", "header.parameters",
        "StoragePurpose::SdSeedBackup",
        "StoragePurpose::SdXprvBackup",
    ):
        _require(errors, required in backup_contract, f"wallet recovery: backup container missing {required}")
    for name, contents in (("seed", backup_seed), ("xprv", backup_xprv)):
        _require(
            errors,
            "BackupDevice" in contents
            and "container::" in contents
            and "BackupError::DeviceBoundStorageRequired" not in contents,
            f"wallet recovery: current device-bound {name} backup is not active",
        )
    _require(
        errors,
        not (root / "apps/kaskold-hardware/src/services/backup/raw.rs").exists(),
        "wallet recovery: retired raw recovery-hint backup facade must stay removed",
    )
    _require(
        errors,
        "current_device_bound_backup_uses_argon2_metadata" in backup_tests
        and "historical_pbkdf2_backup_magic_is_rejected" in backup_tests,
        "wallet recovery: current Argon2 backup and retired-format rejection regressions are missing",
    )


def _check_device_bound_storage(errors: list[str]) -> None:
    device_bound = _read("crates/offline-signer/src/crypto/device_bound_storage.rs")
    for required in (
        "HardwareHmac",
        "KDF_ID_DEVICE_HMAC_SHA256",
        "InternalWallet = 1",
        "SdWallet = 2",
        "SdSeedBackup = 3",
        "SdXprvBackup = 4",
        "StegoWallet = 5",
        "AuthenticationFailed",
    ):
        _require(errors, required in device_bound, f"wallet recovery: device-bound storage missing {required}")
    _require(
        errors,
        "fn hmac_sha256(" in device_bound
        and "raw_key" not in device_bound
        and "export_key" not in device_bound,
        "wallet recovery: hardware HMAC boundary must not expose a raw device key",
    )


def _check_stego_payload(errors: list[str]) -> None:
    stego_payload = _read("apps/kaskold-hardware/src/services/stego/payload.rs")
    stego_controller = _read("apps/kaskold-hardware/src/runtime/interactions/stego/export_confirm/mod.rs")
    stego_import = _read("apps/kaskold-hardware/src/runtime/interactions/stego/import_decrypt.rs")
    for required in (
        "StoragePurpose::StegoWallet", "PAYLOAD_SIZE", "PORTABLE_FORMAT_VERSION",
        "password_kdf::parse_metadata", "build_aad", "descriptor_credential", "zeroize_bytes",
    ):
        _require(errors, required in stego_payload, f"wallet recovery: current stego payload missing {required}")
    _require(
        errors,
        re.search(r"input\.len\(\)\s*!=\s*PAYLOAD_SIZE", stego_payload) is not None,
        "wallet recovery: current stego payload missing bounded PAYLOAD_SIZE input validation",
    )
    _require(
        errors,
        "pack_payload" in stego_controller and "BackupDevice" in stego_controller,
        "wallet recovery: JPEG stego export is not wired to device-bound sealing",
    )
    _require(
        errors,
        "unpack_device_bound_payload" in stego_import
        and "unpack_portable_payload" in stego_import
        and "BackupDevice" in stego_import,
        "wallet recovery: JPEG stego import is not wired to both current security modes",
    )
    _require(
        errors,
        "PayloadError::Retired" not in stego_payload and "is_legacy_base64" not in stego_payload,
        "wallet recovery: current JPEG stego payload must not route through the retired facade",
    )


RETIRED_KPUB_TOKENS = ("decode_legacy_kpub", "decode_kpub_compatible", "legacy_account_key")


def _check_kpub_import(root: Path, errors: list[str]) -> None:
    # Account keys are canonical kpub1 text or an account-level BIP32 xpub. The
    # retired Base58Check kpub decoder must not return anywhere.
    _require(
        errors,
        not (root / "crates/shared-signer/src/legacy_account_key.rs").exists(),
        "wallet recovery: retired Base58 kpub adapter must stay removed",
    )
    for path in (
        "crates/offline-signer/src/derivation/xpub/kpub.rs",
        "apps/kaskold-hardware/src/runtime/interactions/camera_loop/dispatch/kpub.rs",
        "apps/kaskold-hardware/src/runtime/interactions/sd/common/loaded_text.rs",
    ):
        contents = _read(path)
        _require(
            errors,
            "decode_kpub_or_xpub" in contents
            or "decode_account_key_text" in contents
            or "normalize_kpub_text" in contents,
            f"wallet recovery: canonical kpub import is missing from {path}",
        )
        _require(
            errors,
            not any(token in contents for token in RETIRED_KPUB_TOKENS),
            f"wallet recovery: retired Base58 kpub support returned in {path}",
        )

    protocol_bip32 = _read("crates/kaskold-protocol/src/account/bip32.rs")
    watcher_bip32 = _read("crates/online-watcher/src/account/bip32.rs")
    _require(
        errors,
        "decode_kpub_or_xpub" in protocol_bip32
        and "canonical_kpub_text" in protocol_bip32
        and not any(token in protocol_bip32 for token in RETIRED_KPUB_TOKENS),
        "wallet recovery: protocol-owned watcher import must accept kpub1/BIP32 xpub through Kaspa Portal and canonicalize output",
    )
    for delegated in (
        "kaskold_protocol::companion::import_kpub",
        "kaskold_protocol::companion::import_kpub_raw",
    ):
        _require(
            errors,
            delegated in watcher_bip32,
            f"wallet recovery: watcher facade must delegate account import through protocol owner ({delegated})",
        )


def _check_wallet_session_ui(errors: list[str]) -> None:
    production_menu_graph = _read("apps/kaskold-hardware/src/runtime/navigation/ui_graph/menus.rs")
    _require(
        errors,
        '"Backup to SD"' in production_menu_graph
        and '"Encrypt to SD"' in production_menu_graph
        and '"Encrypted SD Card"' in production_menu_graph
        and '"Seed / XPrv Backup"' in production_menu_graph
        and '"Steganographic"' in production_menu_graph,
        "wallet recovery: current device-bound backup menu parity is missing",
    )

    wallet_session = _read("apps/kaskold-hardware/src/services/wallet_session.rs")
    _require(
        errors,
        "fingerprint_hasher.update(raw)" in wallet_session
        and "fingerprint_hasher.update(imported.parent_fingerprint)" in wallet_session,
        "wallet recovery: account fingerprints must cover the complete XPrv and BIP32 metadata",
    )

    loaded_accounts = _read("apps/kaskold-hardware/src/runtime/signing/loaded_accounts.rs")
    _require(
        errors,
        "loaded.push_slot(&seed_manager.slots[active_manager_slot], checkpoint)" in loaded_accounts
        and "loaded.active_index = Some(0)" in loaded_accounts
        and "&self.entries[..self.count]" in loaded_accounts,
        "wallet recovery: the active account must remain signable when more than eight wallets are loaded",
    )


def _check_qr_frame_protocol(errors: list[str]) -> str:
    frame = _read("crates/shared-signer/src/qr_frame.rs")
    _require(
        errors,
        "SESSION_ID_LEN" in frame
        and "session_id(payload)" in frame
        and "verify_session" in frame,
        "multi-frame QR: payload-bound session protocol is missing",
    )
    _require(
        errors,
        "MAX_FRAMES" in frame,
        "multi-frame QR: shared frame-count bound is missing",
    )
    return frame


def _check_qr_capture_state(errors: list[str], frame: str) -> None:
    camera_state = _read("apps/kaskold-hardware/src/runtime/interactions/camera_loop/state.rs")
    _require(
        errors,
        "MF_SLOT_SIZE: usize = u8::MAX as usize" in camera_state
        and "zeroize_bytes(&mut self.buffer)" in camera_state,
        "multi-frame QR: abandoned session payloads must be bounded and wiped",
    )
    _require(
        errors,
        "SessionMismatch" not in frame,
        "multi-frame QR: unused session-error variants must not remain",
    )

    entropy_collection = _read("apps/kaskold-hardware/src/services/entropy/collection.rs")
    _require(
        errors,
        re.search(r"pub fn fill\(output: &mut \[u8\]\).*?output\.fill\(0\);", entropy_collection, re.S)
        is not None,
        "entropy: failed randomness requests must leave the caller buffer zeroed",
    )


def _check_firmware_qr_decoder(errors: list[str]) -> None:
    firmware_decoder = _read(
        "apps/kaskold-hardware/src/runtime/interactions/camera_loop/multiframe.rs"
    )
    _require(
        errors,
        "Mixed multi-frame QR session rejected" in firmware_decoder
        and "verify_session" in firmware_decoder,
        "multi-frame QR: firmware decoder lacks session locking and digest verification",
    )
    _require(
        errors,
        "frame_index != 0" not in firmware_decoder,
        "multi-frame QR: firmware decoder permits foreign frame zero to replace an active session",
    )
    _require(
        errors,
        "Conflicting duplicate" in firmware_decoder
        or "conflicting duplicate" in firmware_decoder,
        "multi-frame QR: firmware decoder accepts conflicting duplicate frames",
    )


def _check_watcher_qr_adapter(errors: list[str]) -> None:
    web_adapter = _read("crates/online-watcher/src/protocol/qr.rs")
    protocol_decoder = _read("crates/kaskold-protocol/src/qr/mod.rs")
    _require(
        errors,
        "use kaskold_protocol::qr::{encode_frames, QrDecoder};" in web_adapter
        and "cell.borrow_mut()" in web_adapter
        and ".accept(&payload)" in web_adapter,
        "multi-frame QR: watcher must remain a thin adapter over kaskold-protocol::QrDecoder",
    )
    _require(
        errors,
        "authorize_frame_session(" in protocol_decoder
        and "verify_session(&complete, &expected)" in protocol_decoder
        and "mixed multi-frame QR session rejected" in protocol_decoder,
        "multi-frame QR: canonical protocol decoder lacks session locking and digest verification",
    )
    _require(
        errors,
        "conflicting duplicate QR frame rejected" in protocol_decoder,
        "multi-frame QR: canonical protocol decoder accepts conflicting duplicate frames",
    )


def _check_restore_sources(errors: list[str]) -> None:
    # Restore-source compile contracts: keep parent/submodule visibility and
    # offline-signer BIP39 Result handling aligned with their actual APIs.
    restore_passphrase = _read("apps/kaskold-hardware/src/runtime/interactions/seed/passphrase.rs")
    restore_installed = _read("apps/kaskold-hardware/src/runtime/interactions/seed/passphrase/installed.rs")
    restore_qr = _read("apps/kaskold-hardware/src/runtime/interactions/camera_loop/dispatch/seed.rs")
    _require(
        errors,
        "installed::apply_pending_wallet_name(ad, slot);" in restore_passphrase
        and "pub(super) fn apply_pending_wallet_name" in restore_installed,
        "wallet recovery: pending restored-wallet naming helper must be parent-visible and called through the installed submodule",
    )
    _require(
        errors,
        "let Ok(index) = offline_signer::derivation::bip39::word_to_index(word) else" in restore_qr
        and "let Some(index) = offline_signer::derivation::bip39::word_to_index(word) else" not in restore_qr,
        "wallet recovery: plain-text SeedQR must handle word_to_index as Result<u16, Bip39Error>",
    )


def _check_sessionless_framing_absent(errors: list[str]) -> None:
    firmware_decoder = _read(
        "apps/kaskold-hardware/src/runtime/interactions/camera_loop/multiframe.rs"
    )
    _require(
        errors,
        "data[0] as usize" not in firmware_decoder
        and "data[1] as usize" not in firmware_decoder,
        "multi-frame QR: sessionless firmware framing must not return",
    )


def check_wallet_recovery_contract(root: Path, errors: list[str]) -> None:
    _check_seed_source_types(root, errors)
    _check_derivation_contracts(errors)
    _check_signing_paths(errors)
    _check_account_export(errors)
    _check_backup_container(root, errors)
    _check_device_bound_storage(errors)
    _check_stego_payload(errors)
    _check_kpub_import(root, errors)
    _check_wallet_session_ui(errors)
    frame = _check_qr_frame_protocol(errors)
    _check_qr_capture_state(errors, frame)
    _check_firmware_qr_decoder(errors)
    _check_watcher_qr_adapter(errors)
    _check_restore_sources(errors)
    _check_sessionless_framing_absent(errors)
