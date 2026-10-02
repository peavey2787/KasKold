"""Canonical account-key boundary and conformance guards."""

from __future__ import annotations

import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[5] / "qa/checks"))
from portal_source import kaskold_source, resolve_path  # noqa: E402

from pathlib import Path
import re


def _source(path: Path) -> str:
    return resolve_path(path).read_text(errors="ignore")


def check(root: Path) -> list[str]:
    errors: list[str] = []
    shared_path = kaskold_source("crates/shared-signer/src/account_key.rs")
    if not shared_path.is_file():
        return ["shared canonical account-key codec is missing"]

    shared = _source(shared_path)
    for required in (
        "ACCOUNT_KEY_TEXT_PREFIX",
        "validate_account_key_payload",
        "encode_account_key_text",
        "decode_account_key_text",
    ):
        if required not in shared:
            errors.append(f"shared account-key contract is missing: {required}")

    consumers = {
        "crates/offline-signer/src/derivation/xpub/kpub.rs": (
            "wallet::key::account",
            "encode_account_key_text",
            "decode_account_key_text",
        ),
        "crates/kaskold-protocol/src/account/bip32.rs": (
            "shared_signer::account_key",
            "encode_account_key_text",
            "kaspa_portal::wallet::key::xpub::decode_kpub_or_xpub",
        ),
    }
    for relative, required_symbols in consumers.items():
        source = _source(root / relative)
        for required in required_symbols:
            if required not in source:
                errors.append(
                    f"account-key consumer bypasses shared codec: {relative} missing {required}"
                )
        if re.search(r"\bbs58\b", source, re.IGNORECASE):
            errors.append(f"account-key consumer implements Base58 directly: {relative}")
        for retired in ("decode_legacy_kpub", "decode_kpub_compatible", "legacy_account_key"):
            if retired in source:
                errors.append(f"account-key consumer reintroduced retired Base58 kpub support: {relative}")

    online_bip32 = _source(root / "crates/online-watcher/src/account/bip32.rs")
    if "kaskold_protocol" not in online_bip32:
        errors.append("online watcher account/BIP32 facade must delegate to kaskold-protocol")

    online_manifest = _source(root / "crates/online-watcher/Cargo.toml")
    if re.search(r"(?m)^bs58\s*=", online_manifest):
        errors.append("online watcher still depends on bs58 for account-key import")

    conformance = root / "qa/tests/conformance/account_key.rs"
    if not conformance.is_file():
        errors.append("canonical account-key conformance tests are missing")
    else:
        source = _source(conformance)
        for required in (
            "canonical_account_key_round_trips",
            "account_key_rejects_noncanonical_metadata_and_text",
        ):
            if required not in source:
                errors.append(f"account-key conformance coverage is missing: {required}")
    return errors
