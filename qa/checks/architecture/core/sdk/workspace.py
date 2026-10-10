from __future__ import annotations

import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[5] / "qa/checks"))
from portal_source import kaskold_source  # noqa: E402

from pathlib import Path
import tomllib


SDK_CRATES = ("kaskold-protocol", "kaskold-sdk")


def check(root: Path) -> list[str]:
    errors: list[str] = []
    for sdk_name in SDK_CRATES:
        errors.extend(_check_sdk_crate(root, sdk_name))
    errors.extend(_check_dependency_direction(root))
    errors.extend(_check_protocol_boundaries(root))
    errors.extend(_check_licensing_boundaries(root))
    errors.extend(_check_wallet_policy_boundary(root))
    errors.extend(_check_public_release_contract(root))
    errors.extend(_check_crate_inventory(root))
    return errors


def _manifest(root: Path, name: str) -> dict:
    return tomllib.loads((root / "crates" / name / "Cargo.toml").read_text())


def _check_sdk_crate(root: Path, sdk_name: str) -> list[str]:
    errors: list[str] = []
    sdk_root = root / "crates" / sdk_name
    manifest_path = sdk_root / "Cargo.toml"
    if not manifest_path.is_file():
        return [f"official Rust/WASM SDK manifest is missing: crates/{sdk_name}/Cargo.toml"]
    manifest = tomllib.loads(manifest_path.read_text())
    if set(manifest.get("lib", {}).get("crate-type", [])) != {"rlib"}:
        errors.append(f"{sdk_name} manifest must remain Rust rlib-only; WASM cdylib is requested by the dedicated build pipeline")
    if (sdk_root / "javascript").exists():
        errors.append(f"{sdk_name} must not contain an authored JavaScript SDK layer")
    authored_js = [path for path in sdk_root.rglob("*.js") if "pkg" not in path.relative_to(sdk_root).parts]
    if authored_js:
        errors.append(f"{sdk_name} implementation must remain Rust/WASM; authored JavaScript found")
    for required in (
        sdk_root / "src/lib.rs",
        sdk_root / "src/wasm/mod.rs",
        sdk_root / "README.md",
        sdk_root / "build.sh",
        sdk_root / "build.ps1",
    ):
        if not required.is_file():
            errors.append(f"required Rust/WASM SDK path is missing: {required.relative_to(root)}")
    return errors


def _check_dependency_direction(root: Path) -> list[str]:
    errors: list[str] = []
    protocol_deps = _manifest(root, "kaskold-protocol").get("dependencies", {})
    sdk_deps = _manifest(root, "kaskold-sdk").get("dependencies", {})
    online_deps = _manifest(root, "online-watcher").get("dependencies", {})
    if "online-watcher" in protocol_deps:
        errors.append("kaskold-protocol must not depend on Companion/online-watcher")
    if "kaskold-hardware-core" in protocol_deps:
        errors.append("kaskold-protocol must not depend on GPL/device-policy kaskold-hardware-core")
    if "kaskold-protocol" not in sdk_deps or "online-watcher" in sdk_deps:
        errors.append("kaskold-sdk must depend on kaskold-protocol and not online-watcher")
    if "kaskold-hardware-core" in sdk_deps:
        errors.append("kaskold-sdk must not expose kaskold-hardware-core in the public SDK dependency graph")
    if "kaskold-protocol" not in online_deps:
        errors.append("Companion/online-watcher must consume kaskold-protocol rather than own reusable relay logic")
    return errors


def _check_protocol_boundaries(root: Path) -> list[str]:
    errors: list[str] = []
    protocol_root = root / "crates/kaskold-protocol/src"
    source = "\n".join(path.read_text() for path in protocol_root.rglob("*.rs"))
    if "svg:" in source or "qrcode::" in source:
        errors.append("kaskold-protocol QR frames must expose raw payloads, not rendered SVG/UI")
    if "thread_local!" in source:
        errors.append("kaskold-protocol QR/session state must be instance-owned, not thread-global")
    if '_ => "kaspa"' in source:
        errors.append("kaskold-protocol network parsing must never fall back to mainnet")
    pairing = (protocol_root / "pairing/mod.rs").read_text()
    for marker in ("nonce", "account_fingerprint", "DerivedAddress"):
        if marker not in pairing:
            errors.append(f"privacy pairing binding is missing {marker}")

    descriptor = (protocol_root / "wire/multisig_descriptor.rs").read_text()
    for marker in (
        "pub const MAX_DESCRIPTOR_PARTICIPANTS",
        "pub fn parse_multisig_descriptor",
        "multi_hd45(",
        "multi_hd(",
        "multi(",
        "DuplicateParticipant",
    ):
        if marker not in descriptor:
            errors.append(f"canonical multisig descriptor parser is missing {marker}")
    watcher_descriptor = (root / "crates/online-watcher/src/multisig/descriptor.rs").read_text()
    if "parse_multisig_descriptor(value.as_bytes())" not in watcher_descriptor:
        errors.append("Companion multisig descriptor facade must delegate syntax parsing to kaskold-protocol")
    for duplicate_parser in ("fn parse_hd44", "fn parse_hd45", "fn parse_static", "fn decode_legacy_kpub"):
        if duplicate_parser in watcher_descriptor:
            errors.append(f"Companion must not re-own canonical descriptor grammar: {duplicate_parser}")
    firmware_descriptor = (root / "apps/kaskold-hardware/src/runtime/interactions/camera_loop/dispatch/descriptor.rs").read_text()
    sd_descriptor = (root / "apps/kaskold-hardware/src/runtime/interactions/sd/common/shared.rs").read_text()
    if "kaskold_protocol::wire::multisig_descriptor::parse_multisig_descriptor" not in firmware_descriptor:
        errors.append("firmware camera descriptor import must use the canonical kaskold-protocol parser")
    if "kaskold_protocol::wire::multisig_descriptor::parse_multisig_descriptor" not in sd_descriptor:
        errors.append("firmware SD descriptor import must use the canonical kaskold-protocol parser")

    watcher_qr = (root / "crates/online-watcher/src/protocol/qr.rs").read_text()
    for marker in ("kaskold_protocol::qr::{encode_frames, QrDecoder}", ".accept(&payload)", ".progress()"):
        if marker not in watcher_qr:
            errors.append(f"Companion QR facade must delegate to canonical kaskold-protocol QR state: {marker}")
    for duplicate in (
        "struct DecoderState",
        "fn accept_session",
        "shared_signer::qr_frame::encode_frame",
        "shared_signer::qr_frame::verify_session",
        "shared_signer::qr_frame::session_id",
    ):
        if duplicate in watcher_qr:
            errors.append(f"Companion must not re-own canonical QR framing/session logic: {duplicate}")
    return errors



def _check_licensing_boundaries(root: Path) -> list[str]:
    errors: list[str] = []
    shared_lib = (root / "crates/shared-signer/src/lib.rs").read_text()
    for retired in ("persistent_credential", "qr_payload"):
        if f"pub mod {retired};" in shared_lib or (root / f"crates/shared-signer/src/{retired}.rs").exists():
            errors.append(f"shared-signer must not retain non-shared ownership of {retired}")

    qr_payload = root / "crates/kaskold-protocol/src/wire/qr_payload.rs"
    if not qr_payload.is_file():
        errors.append("canonical public raw QR envelope must live in kaskold-protocol::wire::qr_payload")
    else:
        text = qr_payload.read_text(errors="ignore")
        if "GPL-3.0-only" not in text:
            errors.append("QR payload module must state its GPL-3.0-only license")

    credential_meta = root / "crates/offline-signer/src/crypto/credential.rs"
    credential_policy = root / "crates/kaskold-hardware-core/src/security/credential.rs"
    if not credential_meta.is_file() or "pub enum CredentialKind" not in credential_meta.read_text(errors="ignore"):
        errors.append("encrypted-storage credential metadata must live in offline-signer")
    if not credential_policy.is_file() or "pub fn validate" not in credential_policy.read_text(errors="ignore"):
        errors.append("PIN/password acceptance and retry policy must live in kaskold-hardware-core")

    contributing = (root / "CONTRIBUTING.md").read_text(errors="ignore")
    for marker in ("shared-signer", "kaskold-protocol", "kaskold-sdk", "GPL-3.0-only", "UPSTREAM_ATTRIBUTION.md"):
        if marker not in contributing:
            errors.append(f"contribution licensing policy is missing marker: {marker}")
    if "MIT OR Apache-2.0" in contributing:
        errors.append("first-party KasKold code must not be offered under a permissive license")

    # BIP32 derivation and address encoding are Kaspa Portal's; KasKold must
    # not carry a second copy of either.
    account = root / "crates/kaskold-protocol/src/account"
    for fork in ("bip32.rs", "address.rs"):
        if (account / fork).exists():
            errors.append(f"kaskold-protocol forks Kaspa Portal account code: account/{fork}")
    account_mod = (account / "mod.rs").read_text(errors="ignore")
    for owner in ("kaspa_portal::wallet::account::derivation", "kaspa_portal::primitives::address"):
        if owner not in account_mod:
            errors.append(f"kaskold-protocol account model must import {owner}")
    return errors

def _check_wallet_policy_boundary(root: Path) -> list[str]:
    source = (root / "crates/kaskold-sdk/src/lib.rs").read_text()
    forbidden = (
        "pub fn create_transaction", "pub fn prepare_send", "pub fn broadcast",
        "pub fn send_tx", "pub struct CreateTransaction", "pub struct SpendUtxo",
        "available_utxos", "selected_utxos", "fee_policy",
        "pub change_address:", "change_address: &str",
    )
    return [
        f"kaskold-sdk must leave transaction policy to host wallets: {marker}"
        for marker in forbidden if marker in source.lower()
    ]


def _check_crate_inventory(root: Path) -> list[str]:
    expected = {"hot-wallet", "kaskold-hardware-core", "kaskold-protocol", "kaskold-sdk", "offline-signer", "online-watcher", "shared-signer", "vault-runtime"}
    actual = {path.name for path in (root / "crates").iterdir() if path.is_dir()}
    if actual == expected:
        return []
    return [f"crates/ must contain exactly {sorted(expected)}, got {sorted(actual)}"]


def _check_public_release_contract(root: Path) -> list[str]:
    errors: list[str] = []
    manifests = {name: _manifest(root, name) for name in ("shared-signer", "kaskold-protocol", "kaskold-sdk")}
    for name, manifest in manifests.items():
        if manifest.get("package", {}).get("license") != "GPL-3.0-only":
            errors.append(f"{name} must be licensed GPL-3.0-only like the upstream project")
        for license_name in ("LICENSE-MIT", "LICENSE-APACHE"):
            if (root / "crates" / name / license_name).exists():
                errors.append(f"{name} must not ship a permissive {license_name}")
    protocol = manifests["kaskold-protocol"]
    host_features = set(protocol.get("features", {}).get("host", []))
    if any("wasm-bindgen" in feature or "js-sys" in feature for feature in host_features):
        errors.append("kaskold-protocol host feature must not pull WASM-only dependencies")
    if "wasm" not in protocol.get("features", {}):
        errors.append("kaskold-protocol must expose an explicit wasm feature")
    sdk = manifests["kaskold-sdk"]
    if "wasm" not in sdk.get("features", {}):
        errors.append("kaskold-sdk must expose an explicit wasm feature")
    if "wasm-bindgen" in sdk.get("dependencies", {}):
        errors.append("kaskold-sdk native dependencies must not unconditionally include wasm-bindgen")
    linux_wasm_build = (root / "scripts/linux/lib/rust-wasm-sdk.sh").read_text()
    windows_wasm_build = (root / "scripts/windows/lib/rust-wasm-sdk.ps1").read_text()
    if (
        "cargo rustc" not in linux_wasm_build
        or "--crate-type=cdylib" not in linux_wasm_build
        or "-- --crate-type=cdylib" in linux_wasm_build
    ):
        errors.append("Linux SDK WASM packaging must pass cdylib to cargo rustc, not directly to rustc")
    if (
        "'cargo','rustc'" not in windows_wasm_build
        or "'wasm','--crate-type=cdylib'" not in windows_wasm_build
        or "'--','--crate-type=cdylib'" in windows_wasm_build
    ):
        errors.append("Windows SDK WASM packaging must pass cdylib to cargo rustc, not directly to rustc")
    sdk_source = (root / "crates/kaskold-sdk/src/lib.rs").read_text()
    protocol_errors = (root / "crates/kaskold-protocol/src/error/mod.rs").read_text()
    sdk_errors = (root / "crates/kaskold-sdk/src/error/mod.rs").read_text()
    network = (root / "crates/kaskold-protocol/src/network/mod.rs").read_text()
    if "SdkResult<" not in sdk_source or "SdkErrorKind" not in sdk_errors:
        errors.append("kaskold-sdk public API must expose typed stable errors")
    for marker in ("WrongNetwork", "TransactionMismatch", "PairingMismatch", "Qr", "Finalization"):
        if marker not in protocol_errors:
            errors.append(f"kaskold-protocol public error categories are missing {marker}")
    if "#[non_exhaustive]" not in network:
        errors.append("public Network must remain future-extensible without a major version bump")
    if (root / "crates/offline-signer/src/transaction").exists():
        errors.append("offline-signer must re-export Kaspa Portal's KSPT codec, not carry its own tree")
    return errors
