from __future__ import annotations

import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[5] / "qa/checks"))
from portal_source import portal_text  # noqa: E402

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
    protocol_root = root / "crates/kaskold-protocol/src"
    source = "\n".join(path.read_text() for path in protocol_root.rglob("*.rs"))
    return [
        *_check_protocol_source(source),
        *_check_pairing_binding(protocol_root),
        *_check_descriptor_binding(protocol_root),
        *_check_descriptor_consumers(root),
        *_check_companion_qr_facade(root),
    ]


def _check_protocol_source(source: str) -> list[str]:
    errors: list[str] = []
    if "svg:" in source or "qrcode::" in source:
        errors.append("kaskold-protocol QR frames must expose raw payloads, not rendered SVG/UI")
    if "thread_local!" in source:
        errors.append("kaskold-protocol QR/session state must be instance-owned, not thread-global")
    if '_ => "kaspa"' in source:
        errors.append("kaskold-protocol network parsing must never fall back to mainnet")
    return errors


def _check_pairing_binding(protocol_root: Path) -> list[str]:
    pairing = (protocol_root / "pairing/mod.rs").read_text()
    return [
        f"privacy pairing binding is missing {marker}"
        for marker in ("nonce", "account_fingerprint", "DerivedAddress")
        if marker not in pairing
    ]


def _check_descriptor_binding(protocol_root: Path) -> list[str]:
    # Portal owns the canonical grammar; KasKold binds it at signer capacity.
    descriptor = (protocol_root / "wire/multisig_descriptor.rs").read_text()
    errors = [
        f"KasKold multisig descriptor binding is missing {marker}"
        for marker in (
            "kaspa_portal::wallet::multisig::grammar",
            "pub const MAX_DESCRIPTOR_PARTICIPANTS",
            "SIGNER_CAPABILITIES.max_multisig_keys",
            "pub fn parse_multisig_descriptor",
        )
        if marker not in descriptor
    ]
    grammar = portal_text("wallet/multisig/grammar.rs")
    errors.extend(
        f"canonical multisig descriptor grammar is missing {marker}"
        for marker in ("multi_hd45(", "multi_hd(", "multi(", "DuplicateParticipant", "fn sort_hd45_by_encoded")
        if marker not in grammar
    )
    return errors


def _check_descriptor_consumers(root: Path) -> list[str]:
    errors: list[str] = []
    for fork in ("crates/online-watcher/src/multisig/descriptor.rs", "crates/kaskold-protocol/src/wire/multisig_descriptor/"):
        if (root / fork).exists():
            errors.append(f"KasKold must not re-own the canonical descriptor grammar: {fork}")
    companion_policy = (root / "crates/online-watcher/src/multisig/mod.rs").read_text()
    if "kaskold_protocol::wire::multisig_descriptor::parse_multisig_descriptor" not in companion_policy:
        errors.append("Companion multisig must enforce the KasKold signer descriptor capacity")
    firmware_descriptor = (root / "apps/kaskold-hardware/src/runtime/interactions/camera_loop/dispatch/descriptor.rs").read_text()
    sd_descriptor = (root / "apps/kaskold-hardware/src/runtime/interactions/sd/common/shared.rs").read_text()
    if "kaskold_protocol::wire::multisig_descriptor::parse_multisig_descriptor" not in firmware_descriptor:
        errors.append("firmware camera descriptor import must use the canonical kaskold-protocol parser")
    if "kaskold_protocol::wire::multisig_descriptor::parse_multisig_descriptor" not in sd_descriptor:
        errors.append("firmware SD descriptor import must use the canonical kaskold-protocol parser")
    return errors


def _check_companion_qr_facade(root: Path) -> list[str]:
    watcher_qr = (root / "crates/online-watcher/src/protocol/qr.rs").read_text()
    errors = [
        f"Companion QR facade must delegate to canonical kaskold-protocol QR state: {marker}"
        for marker in ("kaskold_protocol::qr::{encode_frames, QrDecoder}", ".accept(&payload)", ".progress()")
        if marker not in watcher_qr
    ]
    errors.extend(
        f"Companion must not re-own canonical QR framing/session logic: {duplicate}"
        for duplicate in (
            "struct DecoderState",
            "fn accept_session",
            "shared_signer::qr_frame::encode_frame",
            "shared_signer::qr_frame::verify_session",
            "shared_signer::qr_frame::session_id",
        )
        if duplicate in watcher_qr
    )
    return errors


def _check_licensing_boundaries(root: Path) -> list[str]:
    return [
        *_check_retired_shared_modules(root),
        *_check_qr_payload_module(root),
        *_check_credential_ownership(root),
        *_check_contributing_policy(root),
        *_check_account_forks(root),
    ]


def _check_retired_shared_modules(root: Path) -> list[str]:
    shared_lib = (root / "crates/shared-signer/src/lib.rs").read_text()
    return [
        f"shared-signer must not retain non-shared ownership of {retired}"
        for retired in ("persistent_credential", "qr_payload")
        if f"pub mod {retired};" in shared_lib or (root / f"crates/shared-signer/src/{retired}.rs").exists()
    ]


def _check_qr_payload_module(root: Path) -> list[str]:
    qr_payload = root / "crates/kaskold-protocol/src/wire/qr_payload.rs"
    if not qr_payload.is_file():
        return ["canonical public raw QR envelope must live in kaskold-protocol::wire::qr_payload"]
    if "GPL-3.0-only" not in qr_payload.read_text(errors="ignore"):
        return ["QR payload module must state its GPL-3.0-only license"]
    return []


def _check_credential_ownership(root: Path) -> list[str]:
    errors: list[str] = []
    credential_meta = root / "crates/offline-signer/src/crypto/credential.rs"
    credential_acceptance = root / "crates/shared-signer/src/credential_policy.rs"
    credential_policy = root / "crates/kaskold-hardware-core/src/security/credential.rs"
    if not credential_meta.is_file() or "pub enum CredentialKind" not in credential_meta.read_text(errors="ignore"):
        errors.append("encrypted-storage credential metadata must live in offline-signer")
    if not credential_acceptance.is_file() or "pub fn validate" not in credential_acceptance.read_text(errors="ignore"):
        errors.append("PIN/password acceptance shared by device and vault must live in shared-signer")
    device_policy = credential_policy.read_text(errors="ignore") if credential_policy.is_file() else ""
    if "pub fn retry_delay_millis" not in device_policy or "pub use shared_signer::credential_policy::" not in device_policy:
        errors.append("device retry policy must live in kaskold-hardware-core over the shared acceptance policy")
    return errors


def _check_contributing_policy(root: Path) -> list[str]:
    contributing = (root / "CONTRIBUTING.md").read_text(errors="ignore")
    errors = [
        f"contribution licensing policy is missing marker: {marker}"
        for marker in ("shared-signer", "kaskold-protocol", "kaskold-sdk", "GPL-3.0-only", "UPSTREAM_ATTRIBUTION.md")
        if marker not in contributing
    ]
    if "MIT OR Apache-2.0" in contributing:
        errors.append("first-party KasKold code must not be offered under a permissive license")
    return errors


def _check_account_forks(root: Path) -> list[str]:
    # BIP32 derivation and address encoding are Kaspa Portal's; KasKold must
    # not carry a second copy of either.
    account = root / "crates/kaskold-protocol/src/account"
    errors = [
        f"kaskold-protocol forks Kaspa Portal account code: account/{fork}"
        for fork in ("bip32.rs", "address.rs")
        if (account / fork).exists()
    ]
    account_mod = (account / "mod.rs").read_text(errors="ignore")
    errors.extend(
        f"kaskold-protocol account model must import {owner}"
        for owner in ("kaspa_portal::wallet::account::derivation", "kaspa_portal::primitives::address")
        if owner not in account_mod
    )
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
    manifests = {name: _manifest(root, name) for name in ("shared-signer", "kaskold-protocol", "kaskold-sdk")}
    return [
        *_check_manifest_licenses(root, manifests),
        *_check_protocol_features(manifests["kaskold-protocol"]),
        *_check_sdk_features(manifests["kaskold-sdk"]),
        *_check_wasm_packaging(root),
        *_check_public_api_surface(root),
    ]


def _check_manifest_licenses(root: Path, manifests: dict[str, dict]) -> list[str]:
    errors: list[str] = []
    for name, manifest in manifests.items():
        if manifest.get("package", {}).get("license") != "GPL-3.0-only":
            errors.append(f"{name} must be licensed GPL-3.0-only like the upstream project")
        for license_name in ("LICENSE-MIT", "LICENSE-APACHE"):
            if (root / "crates" / name / license_name).exists():
                errors.append(f"{name} must not ship a permissive {license_name}")
    return errors


def _check_protocol_features(protocol: dict) -> list[str]:
    errors: list[str] = []
    host_features = set(protocol.get("features", {}).get("host", []))
    if any("wasm-bindgen" in feature or "js-sys" in feature for feature in host_features):
        errors.append("kaskold-protocol host feature must not pull WASM-only dependencies")
    if "wasm" not in protocol.get("features", {}):
        errors.append("kaskold-protocol must expose an explicit wasm feature")
    return errors


def _check_sdk_features(sdk: dict) -> list[str]:
    errors: list[str] = []
    if "wasm" not in sdk.get("features", {}):
        errors.append("kaskold-sdk must expose an explicit wasm feature")
    if "wasm-bindgen" in sdk.get("dependencies", {}):
        errors.append("kaskold-sdk native dependencies must not unconditionally include wasm-bindgen")
    return errors


def _check_wasm_packaging(root: Path) -> list[str]:
    errors: list[str] = []
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
    return errors


def _check_public_api_surface(root: Path) -> list[str]:
    errors: list[str] = []
    sdk_source = (root / "crates/kaskold-sdk/src/lib.rs").read_text()
    protocol_errors = (root / "crates/kaskold-protocol/src/error/mod.rs").read_text()
    sdk_errors = (root / "crates/kaskold-sdk/src/error/mod.rs").read_text()
    network = (root / "crates/kaskold-protocol/src/network/mod.rs").read_text()
    if "SdkResult<" not in sdk_source or "SdkErrorKind" not in sdk_errors:
        errors.append("kaskold-sdk public API must expose typed stable errors")
    errors.extend(
        f"kaskold-protocol public error categories are missing {marker}"
        for marker in ("WrongNetwork", "TransactionMismatch", "PairingMismatch", "Qr", "Finalization")
        if marker not in protocol_errors
    )
    if "#[non_exhaustive]" not in network:
        errors.append("public Network must remain future-extensible without a major version bump")
    if (root / "crates/offline-signer/src/transaction").exists():
        errors.append("offline-signer must re-export Kaspa Portal's KSPT codec, not carry its own tree")
    return errors
