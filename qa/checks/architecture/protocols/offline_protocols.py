from __future__ import annotations

import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[4] / "qa/checks"))
from portal_source import kaskold_source  # noqa: E402

from pathlib import Path
import re

from architecture.protocols.compact_protocols import check_kspt, check_pskt

def _check_bip32_and_transaction_models(root: Path) -> list[str]:
    ROOT = root
    errors: list[str] = []
    offline_root = ROOT / "crates/offline-signer/src"
    offline_lib_source = (offline_root / "lib.rs").read_text(errors="ignore")
    for required in ("pub mod derivation", "pub mod crypto", "pub mod transaction"):
        if required not in offline_lib_source:
            errors.append(f"offline signer domain wiring is missing: {required}")
    if "#[path" in offline_lib_source:
        errors.append("offline signer crate root must use canonical domain modules, not path aliases")
    for retired_root in (
        "bip32", "bip39", "bip39_wordlist", "bip85", "ecies", "hmac",
        "kspt", "pbkdf2", "schnorr", "sighash", "std_pskt", "xpub",
    ):
        if f"pub mod {retired_root};" in offline_lib_source:
            errors.append(f"offline signer crate-root compatibility module must not return: {retired_root}")

    # The signing core is Kaspa Portal's. offline-signer exposes it through thin
    # re-export facades and must never regain a local copy that could drift.
    derivation_facade = offline_lib_source.split("pub mod derivation", 1)[-1].split("\n}", 1)[0]
    transaction_facade = offline_lib_source.split("pub mod transaction", 1)[-1].split("\n}", 1)[0]
    for token in ("pub use kaspa_portal::wallet::", "bip32", "bip39", "xpub"):
        if token not in derivation_facade:
            errors.append(f"offline signer derivation facade must re-export Kaspa Portal: {token}")
    for token in ("pub use kaspa_portal::transaction::", "model", "sighash", "kspt"):
        if token not in transaction_facade:
            errors.append(f"offline signer transaction facade must re-export Kaspa Portal: {token}")
    for copied in ("derivation", "transaction", "address"):
        if (offline_root / copied).is_dir():
            errors.append(f"offline signer must not carry a local copy of the signing core: src/{copied}/")
    for copied in ("adaptor", "anti_klepto", "ecies", "message", "password_kdf", "schnorr"):
        if (offline_root / f"crypto/{copied}.rs").exists():
            errors.append(f"offline signer must not carry a local copy of Kaspa Portal crypto: {copied}")

    retired_path = re.compile(
        r"\boffline_signer::(?:bip32|bip39|bip39_wordlist|bip85|ecies|hmac|kspt|"
        r"pbkdf2|schnorr|sighash|std_pskt|xpub)\b|"
        r"\boffline_signer::transaction::(?:Transaction|TransactionInput|TransactionOutput|"
        r"ScriptPublicKey|ScriptType|SigHashType|MultisigConfig|MultisigStore)\b"
    )
    for source_root in (ROOT / "apps", ROOT / "crates", ROOT / "tools", ROOT / "qa"):
        if not source_root.exists():
            continue
        for path in source_root.rglob("*.rs"):
            match = retired_path.search(path.read_text(errors="ignore"))
            if match:
                errors.append(
                    f"retired offline-signer crate-root path remains in {path.relative_to(ROOT)}: "
                    f"{match.group(0)}"
                )

    return errors
def _check_xpub_and_sighash(root: Path) -> list[str]:
    errors: list[str] = []
    # xpub and sighash structure, single Base58Check codec and single keyed
    # Blake2b are enforced by Kaspa Portal's own architecture gates. KasKold
    # only requires that the shipped sources expose the APIs firmware uses.
    xpub_facade = kaskold_source("crates/offline-signer/src/derivation/xpub/mod.rs").read_text(errors="ignore")
    for symbol in (
        "KPUB_MAX_LEN", "XPUB_PAYLOAD_LEN", "XPRV_MAX_LEN", "serialize_kpub",
        "derive_and_serialize_kpub", "derive_account_raw_kpub_payload", "kpub_text_to_raw",
        "derive_and_serialize_xprv", "import_xprv", "import_kpub", "import_kpub_raw", "import_kpub_qr",
    ):
        if symbol not in xpub_facade:
            errors.append(f"offline xpub façade is missing required export: {symbol}")
    sighash_facade = kaskold_source("crates/offline-signer/src/transaction/sighash/mod.rs").read_text(errors="ignore")
    for symbol in ("KaspaBlake2b", "blake2b_hash", "calculate_sighash", "sign_input"):
        if symbol not in sighash_facade:
            errors.append(f"offline sighash façade is missing required export: {symbol}")
    return errors

def _check_password_kdf_policy(root: Path) -> list[str]:
    errors: list[str] = []
    offline_root = root / "crates/offline-signer/src"
    password_kdf = kaskold_source("crates/offline-signer/src/crypto/password_kdf.rs")
    password_tests = kaskold_source("crates/offline-signer/src/crypto/unit_tests/password_kdf_tests.rs")
    bip39 = kaskold_source("crates/offline-signer/src/derivation/bip39/seed.rs")
    retired = (
        offline_root / "crypto/pbkdf2.rs",
        offline_root / "crypto/unit_tests/pbkdf2_tests.rs",
        offline_root / "crypto/legacy_pbkdf2.rs",
        offline_root / "crypto/unit_tests/legacy_pbkdf2_tests.rs",
    )

    for path in (password_kdf, password_tests, bip39):
        if not path.exists():
            errors.append(f"password-KDF contract file is missing: {path}")
    for path in retired:
        if path.exists():
            errors.append(f"retired non-BIP39 PBKDF2 implementation returned: {path.relative_to(root)}")
    if errors:
        return errors

    current = password_kdf.read_text(errors="ignore")
    bip39_source = bip39.read_text(errors="ignore")
    for token in (
        "Algorithm::Argon2id", "Version::V0x13", "PasswordKdfPurpose",
        "try_reserve_exact", "AllocationFailed", "UnsupportedParameters",
        "parameters.is_current()", "derive_key_32_with_workspace",
        "workspace_block_count", "zeroize_workspace", "PasswordKdfBlock",
    ):
        if token not in current:
            errors.append(f"central Argon2 password-KDF contract missing: {token}")
    for forbidden in ("derive_legacy_32", "pbkdf2_hmac", "LEGACY_PBKDF2_ROUNDS", "legacy_pbkdf2"):
        if forbidden in current:
            errors.append(f"current Argon2 password-KDF contains retired PBKDF2 logic: {forbidden}")

    for token in (
        "PBKDF2-HMAC-SHA512", "BIP39_PBKDF2_ROUNDS: u16 = 2048",
        "pbkdf2_hmac_sha512", "iterations=2048", "dklen=64",
    ):
        if token not in bip39_source:
            errors.append(f"BIP39 standard KDF contract changed: {token}")
    for forbidden in ("password_kdf", "Argon2", "legacy_pbkdf2"):
        if forbidden in bip39_source:
            errors.append(f"BIP39 seed derivation must remain outside KasKold password KDFs: {forbidden}")

    for source_root in (root / "apps", root / "crates"):
        for path in source_root.rglob("*.rs"):
            relative = path.relative_to(root)
            text = path.read_text(errors="ignore")
            if any(token in text for token in ("legacy_pbkdf2", "derive_legacy_32", "LEGACY_PBKDF2")):
                errors.append(f"retired PBKDF2 compatibility logic remains: {relative}")
            if re.search(r"\bpbkdf2_hmac(?:_sha256)?\b", text) and relative != Path("crates/offline-signer/src/derivation/bip39/seed.rs"):
                errors.append(f"generic PBKDF2 implementation is forbidden outside BIP39: {relative}")

    current_only = {
        (
            "apps/kaskold-hardware/src/services/backup/container.rs",
            "crates/offline-signer/src/crypto/container_framing.rs",
        ): ("KASDB005", "PasswordKdfPurpose::DeviceBoundBackup", "header.parameters"),
        (
            "apps/kaskold-hardware/src/runtime/interactions/sd/exports/kspt_export/crypto.rs",
            "crates/offline-signer/src/crypto/container_framing.rs",
        ): ("KAS\\x04", "open_current", "PasswordKdfPurpose::EncryptedTransport"),
        (
            "apps/kaskold-hardware/src/services/persistent_wallet/crypto.rs",
            "apps/kaskold-hardware/src/services/persistent_wallet/crypto/record.rs",
        ): ("KSWLT004", "parse_current_header", "CredentialKdf::from_parameters"),
    }
    for names, tokens in current_only.items():
        text = "\n".join(kaskold_source(str(name)).read_text(errors="ignore") for name in names)
        label = " + ".join(names)
        for token in tokens:
            if token not in text:
                errors.append(f"Argon2-only KDF selector missing in {label}: {token}")
        for forbidden in ("KASDB004", "KAS\\x03", "KSWLT003", "open_legacy", "parse_legacy"):
            if forbidden in text:
                errors.append(f"retired password-format compatibility remains in {label}: {forbidden}")
    return errors


def _check_offline_protocol_hygiene(root: Path) -> list[str]:
    ROOT = root
    errors: list[str] = []
    offline_root = ROOT / "crates/offline-signer/src"
    online_root = ROOT / "crates/online-watcher/src"
    online_source = "\n".join(path.read_text(errors="ignore") for path in online_root.rglob("*.rs"))
    legacy_storage = offline_root / "storage"
    if legacy_storage.exists():
        errors.append("dormant offline-signer PIN/storage subsystem must not return")
    errors.extend(_check_password_kdf_policy(root))

    qr_payload_source = (ROOT / "crates/kaskold-protocol/src/wire/qr_payload.rs").read_text(errors="ignore")
    if "#[allow(dead_code)]" in qr_payload_source:
        errors.append("public QR compatibility constants must not use dead-code suppression")

    if "#[path" in online_source:
        errors.append("online watcher must use ordinary Rust module structure, not #[path] wiring")
    if "#[allow(dead_code)]" in online_source:
        errors.append("online watcher contains retained dead code")
    for forbidden_pattern, description in (
        (r"\bkspt::create_", "legacy KSPT transaction builder"),
        (r"\bserialize_pskb_(?:single_sig|with_covenants)", "legacy PSKB serializer"),
    ):
        if re.search(forbidden_pattern, online_source):
            errors.append(f"online watcher retains {description}")

    return errors

def check(root: Path) -> list[str]:
    return [
        *check_pskt(root),
        *check_kspt(root),
        *_check_bip32_and_transaction_models(root),
        *_check_xpub_and_sighash(root),
        *_check_offline_protocol_hygiene(root),
    ]
