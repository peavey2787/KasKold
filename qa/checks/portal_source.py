"""Locate the pinned Kaspa Portal sources that KasKold links.

The signing core (keys, derivation, mnemonics, transaction model, sighash,
KSPT/PSKT, anti-klepto, covenant and Private Swap protocols, password KDF,
Schnorr) lives in Kaspa Portal. Source-level QA contracts read those files from
the exact revision pinned in the workspace lockfile, so a guard keeps checking
the code that actually ships rather than a stale local copy.
"""

from __future__ import annotations

from functools import lru_cache
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]


@lru_cache(maxsize=1)
def portal_root() -> Path:
    """Return the `src/` directory of the locked kaspa-portal checkout."""
    metadata = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked"],
        cwd=ROOT,
        check=True,
        capture_output=True,
        text=True,
    )
    for package in json.loads(metadata.stdout)["packages"]:
        if package["name"] == "kaspa-portal":
            return Path(package["manifest_path"]).parent / "src"
    raise RuntimeError("kaspa-portal is not in the locked KasKold dependency graph")


def portal_path(relative: str) -> Path:
    """Path of a file inside the locked Kaspa Portal `src/` tree."""
    path = portal_root() / relative
    if not path.is_file():
        raise FileNotFoundError(f"kaspa-portal source is missing: src/{relative}")
    return path


def portal_text(relative: str) -> str:
    return portal_path(relative).read_text(encoding="utf-8", errors="ignore")


# Former KasKold source locations of the signing core and where that code now
# lives in Kaspa Portal. Longest prefix wins; directory prefixes end in "/".
MOVED_TO_PORTAL = {
    "crates/offline-signer/src/address/mod.rs": "primitives/address/mod.rs",
    "crates/offline-signer/src/crypto/schnorr.rs": "crypto/schnorr/mod.rs",
    "crates/offline-signer/src/crypto/password_kdf.rs": "crypto/kdf/password.rs",
    "crates/offline-signer/src/crypto/adaptor.rs": "crypto/adaptor.rs",
    "crates/offline-signer/src/crypto/anti_klepto.rs": "crypto/anti_klepto.rs",
    "crates/offline-signer/src/crypto/ecies.rs": "crypto/ecies.rs",
    "crates/offline-signer/src/crypto/message.rs": "crypto/message.rs",
    "crates/offline-signer/src/crypto/unit_tests/schnorr_tests.rs": "crypto/schnorr/unit-tests/schnorr_tests.rs",
    "crates/offline-signer/src/crypto/unit_tests/password_kdf_tests.rs": "crypto/kdf/unit-tests/password.rs",
    "crates/offline-signer/src/crypto/unit_tests/message_tests.rs": "crypto/unit-tests/message.rs",
    "crates/offline-signer/src/crypto/unit_tests/adaptor_tests.rs": "crypto/unit-tests/adaptor.rs",
    "crates/offline-signer/src/crypto/unit_tests/ecies_tests.rs": "crypto/unit-tests/ecies.rs",
    "crates/offline-signer/src/crypto/unit_tests/anti_klepto_tests.rs": "crypto/unit-tests/anti_klepto.rs",
    "crates/offline-signer/src/derivation/unit_tests/bip32_tests.rs": "wallet/derivation/bip32/unit-tests/mod.rs",
    "crates/offline-signer/src/derivation/unit_tests/bip39_tests.rs": "wallet/mnemonic/bip39/unit-tests/mod.rs",
    "crates/offline-signer/src/derivation/unit_tests/bip85_tests.rs": "wallet/derivation/unit-tests/bip85.rs",
    "crates/offline-signer/src/derivation/unit_tests/xpub_tests.rs": "wallet/key/xpub/unit-tests/mod.rs",
    "crates/offline-signer/src/derivation/bip32/": "wallet/derivation/bip32/",
    "crates/offline-signer/src/derivation/bip39/": "wallet/mnemonic/bip39/",
    "crates/offline-signer/src/derivation/bip39_wordlist.rs": "wallet/mnemonic/wordlist.rs",
    "crates/offline-signer/src/derivation/xpub/": "wallet/key/xpub/",
    "crates/offline-signer/src/derivation/covenant/unit_tests/mod.rs": "wallet/derivation/covenant/unit-tests/mod.rs",
    "crates/offline-signer/src/derivation/": "wallet/derivation/",
    "crates/offline-signer/src/transaction/kspt/wire_adapter.rs": "transaction/interchange/kspt/codec/partial_signed.rs",
    "crates/offline-signer/src/transaction/kspt/wire_adapter/unit_tests/mod.rs": "transaction/interchange/kspt/unit-tests/codec.rs",
    "crates/offline-signer/src/transaction/kspt/kssn_io.rs": "transaction/interchange/kspt/codec/io.rs",
    "crates/offline-signer/src/transaction/kspt/": "transaction/interchange/kspt/",
    "crates/offline-signer/src/transaction/std_pskt/": "transaction/interchange/pskt/standard/",
    "crates/offline-signer/src/transaction/std_pskt.rs": "transaction/interchange/pskt/standard/mod.rs",
    "crates/offline-signer/src/transaction/private_swap.rs": "transaction/signing/private_swap/mod.rs",
    "crates/offline-signer/src/transaction/private_swap/": "transaction/signing/private_swap/",
    "crates/offline-signer/src/transaction/signature_verification.rs": "transaction/signature_verification/mod.rs",
    "crates/offline-signer/src/transaction/signature_verification/": "transaction/signature_verification/",
    "crates/offline-signer/src/transaction/unit_tests/sighash_tests.rs": "transaction/sighash/unit-tests/sighash_tests.rs",
    "crates/offline-signer/src/transaction/model/": "transaction/model/",
    "crates/offline-signer/src/transaction/sighash/": "transaction/sighash/",
    "crates/offline-signer/src/self_test/": "self_test/",
    "crates/shared-signer/src/account_key.rs": "wallet/key/account.rs",
    "crates/shared-signer/src/anti_klepto.rs": "crypto/anti_klepto.rs",
    "crates/shared-signer/src/anti_klepto/wire.rs": "transaction/signing/anti_klepto/protocol.rs",
    "crates/shared-signer/src/bytes.rs": "primitives/bytes.rs",
    "crates/shared-signer/src/covenant_branch.rs": "contract/covenant/branch.rs",
    "crates/shared-signer/src/covenant_sign/mod.rs": "transaction/signing/covenant/protocol/mod.rs",
    "crates/shared-signer/src/covenant_sign/private_swap/validation.rs": "transaction/signing/covenant/protocol/private_swap.rs",
    "crates/shared-signer/src/unit_tests/account_key_tests.rs": "wallet/key/unit-tests/account.rs",
    "crates/shared-signer/src/unit_tests/property_tests.rs": "wallet/key/unit-tests/property.rs",
    "crates/shared-signer/src/unit_tests/pskt_tests.rs": "transaction/interchange/pskt/unit-tests/shared.rs",
    "crates/shared-signer/src/unit_tests/qr_frame_tests.rs": "transaction/interchange/qr/unit-tests/frame.rs",
    "crates/shared-signer/src/covenant_sign/": "transaction/signing/covenant/protocol/",
    "crates/shared-signer/src/pskt.rs": "transaction/interchange/pskt/shared.rs",
    "crates/shared-signer/src/qr_frame.rs": "transaction/interchange/qr/frame.rs",
    "crates/shared-signer/src/security.rs": "transaction/interchange/qr/security.rs",
}


def _portal_relative(relative: str) -> str | None:
    for prefix in sorted(MOVED_TO_PORTAL, key=len, reverse=True):
        target = MOVED_TO_PORTAL[prefix]
        if prefix.endswith("/") and relative.startswith(prefix):
            return (target + relative[len(prefix):]).replace("/unit_tests/", "/unit-tests/")
        if relative == prefix:
            return target
    return None


def resolve_path(path: Path) -> Path:
    """`kaskold_source` for an absolute path inside the KasKold repository."""
    path = Path(path)
    if path.exists():
        return path
    try:
        relative = path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return path
    return kaskold_source(relative)


def kaskold_source(relative: str) -> Path:
    """Resolve a repository-relative source path to the file that ships.

    KasKold-owned files resolve locally; signing-core files that moved to Kaspa
    Portal resolve inside the locked Portal checkout.
    """
    local = ROOT / relative
    if local.exists():
        return local
    moved = _portal_relative(relative)
    if moved is None:
        directory = _portal_relative(relative.rstrip("/") + "/")
        if directory is not None and (portal_root() / directory).is_dir():
            return portal_root() / directory.rstrip("/")
        return local
    return portal_path(moved)
