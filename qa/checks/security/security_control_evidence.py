#!/usr/bin/env python3
"""Generate an internal source-security control evidence scan."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[3]
DEFAULT_OUTPUT = ROOT / "target/qa/security/current-control-evidence.json"

_SECURITY_DIR = Path(__file__).resolve().parent
import sys
if str(_SECURITY_DIR) not in sys.path:
    sys.path.insert(0, str(_SECURITY_DIR))
from security_control_scans import collect_text, evidence_matches, source_scans  # noqa: E402

CONTROLS: list[dict[str, Any]] = [
    {
        "id": "entropy",
        "area": "entropy",
        "summary": (
            "Hardware RNG gates and structural health are checked; camera contribution requires "
            "measured inter-frame temporal liveness rather than capture success alone; CoreS3 BMI270 "
            "seed windows receive point-of-use completeness/diversity checks before supplemental mixing, "
            "and changed ambient touch/timing observations are staged for later checked fills; pool and raw samples are zeroized."
        ),
        "residual_risk": (
            "Camera liveness thresholds and IMU distinctness checks are health gates, not numerical "
            "entropy estimates. Physical camera/IMU/touch sources still require device datasets and "
            "appropriate SP 800-90B characterization before any quantified min-entropy claim."
        ),
        "evidence": {
            "apps/kaskold-hardware/src/services/entropy/trng.rs": [
                "set_and_verify",
                "inspect(&samples)?",
            ],
            "apps/kaskold-hardware/src/services/entropy/collection.rs": [
                "trng::enable_hardware_rng()?",
                "validate_seed_entropy",
                "imu::mix_seed_sample",
                "ambient::mix_staged",
                "mixer::zeroize",
            ],
            "apps/kaskold-hardware/src/services/entropy/imu.rs": [
                "count == sample.len()",
                "buffer_is_healthy",
                "KasSigner-seed-imu-v2",
                "mixer::zeroize(sample)",
            ],
            "apps/kaskold-hardware/src/services/entropy/ambient.rs": [
                "stage_touch",
                "KasSigner-ambient-touch-v2",
            ],
            "apps/kaskold-hardware/src/hw/m5stack/imu.rs": [
                "BMI270_CHIP_ID",
                "buffer_is_healthy",
                "GYR_DATA_READY",
            ],
            "apps/kaskold-hardware/src/services/entropy/camera/mod.rs": [
                "CameraEntropyTracker",
                "MAX_CAMERA_HEALTH_WINDOWS",
                "tracker.report",
            ],
            "apps/kaskold-hardware/src/hw/shared/dvp.rs": [
                "receive_full_frame",
                "transfer.is_done()",
                "FrameCaptureStatus::TimedOut",
            ],
            "crates/kaskold-hardware-core/src/entropy/frame_noise.rs": [
                "MIN_CHANGED_FOR_ENTROPY",
                "MIN_AC_FOR_ENTROPY_X100",
                "max_consecutive_stale_deltas",
            ],
            "crates/kaskold-hardware-core/src/security.rs": [
                "timing_observations_usable",
                "device_identity_words_usable",
            ],
        },
    },
    {
        "id": "key-lifecycle",
        "area": "key lifecycle",
        "summary": (
            "Offline/shared/firmware-core crates have no network dependencies and secret intermediates use "
            "explicit volatile zeroization paths."
        ),
        "evidence": {
            "crates/shared-signer/src/bytes.rs": ["write_volatile", "compiler_fence"],
            "crates/offline-signer/src/transaction/kspt/signing/multi_address.rs": [
                "zeroize_bytes"
            ],
            "apps/kaskold-hardware/src/runtime/signing/loaded_accounts.rs": ["zeroize"],
        },
        "residual_risk": (
            "The custom zeroization primitive is source-reviewed but has not received an "
            "independent compiler/code-generation audit on every release toolchain."
        ),
    },
    {
        "id": "transaction-intent",
        "area": "transaction intent",
        "summary": (
            "Final signing is gated by seed availability, explicit review approval, matching "
            "input counts, and final-input position."
        ),
        "evidence": {
            "crates/kaskold-hardware-core/src/security.rs": [
                "authorize_transaction_signing",
                "ReviewIncomplete",
                "InputCountMismatch",
            ],
            "apps/kaskold-hardware/src/runtime/signing/workflow.rs": [
                "authorize_transaction_signing"
            ],
            "apps/kaskold-hardware/src/runtime/input/wallet_app.rs": [
                "review_authorized = true",
                "review_authorized = false",
            ],
        },
    },
    {
        "id": "backup-cryptography",
        "area": "wallet backup cryptography",
        "summary": (
            "All KasKold-owned password formats use the centralized Argon2id v=19 KDF "
            "with explicit purpose separation and authenticated version/KDF parameters. Persistent "
            "wallet and device-bound removable backups combine the stretched credential with the "
            "read-protected ESP32-S3 eFuse HMAC boundary before AES-256-GCM. Portable JPEG backup "
            "is deliberately cross-device and self-contained: JPEG + password, with Argon2 metadata, "
            "salt, nonce, ciphertext/tag and carrier/security metadata authenticated as AEAD AAD. "
            "PBKDF2 remains only where BIP39 requires PBKDF2-HMAC-SHA512/2048; pre-production "
            "KasKold PBKDF2 compatibility readers are retired."
        ),
        "evidence": {
            "crates/offline-signer/src/crypto/password_kdf.rs": [
                "Algorithm::Argon2id",
                "Version::V0x13",
                "PasswordKdfPurpose",
                "try_reserve_exact",
                "AllocationFailed",
            ],
            "crates/offline-signer/src/derivation/bip39/seed.rs": [
                "BIP39_PBKDF2_ROUNDS: u16 = 2048",
                "pbkdf2_hmac_sha512",
            ],
            "apps/kaskold-hardware/src/services/persistent_wallet/crypto.rs": [
                "KSWLT004",
            ],
            "apps/kaskold-hardware/src/services/persistent_wallet/crypto/record.rs": [
                "password_kdf::parse_metadata",
                "CredentialKdf::from_parameters",
            ],
            "crates/offline-signer/src/crypto/container_framing.rs": [
                "KASDB005",
                "KAS\\x04",
                "TRANSPORT_CURRENT_MAGIC",
                "parameters: PasswordKdfParams",
            ],
            "apps/kaskold-hardware/src/services/backup/container.rs": [
                "PasswordKdfPurpose::DeviceBoundBackup",
                "header.parameters",
            ],
            "apps/kaskold-hardware/src/runtime/interactions/sd/exports/kspt_export/crypto.rs": [
                "PasswordKdfPurpose::EncryptedTransport",
                "open_current",
            ],
            "apps/kaskold-hardware/src/services/stego/payload.rs": [
                "const PORTABLE_FORMAT_VERSION: u8 = 4;",
                "password_kdf::parse_metadata",
                "build_aad",
            ],
            "apps/kaskold-hardware/src/services/stego/portable.rs": [
                "PasswordKdfPurpose::PortableBackup",
                "Aes256Gcm",
                "zeroize_bytes(&mut key)",
            ],
            "apps/kaskold-hardware/src/services/unit_tests/backup_tests.rs": [
                "current_device_bound_backup_uses_argon2_metadata",
                "historical_pbkdf2_backup_magic_is_rejected",
                "portable_jpeg_is_password_only_cross_device",
            ],
        },
        "residual_risk": (
            "Physical proof that the provisioned HMAC_UP key is unique, correctly read-protected, "
            "and unavailable to software depends on eFuse provisioning/HIL. Portable Backup removes "
            "the hardware factor so another KasKold can recover it; a self-contained encrypted JPEG "
            "therefore permits offline password guessing. Argon2id raises the per-guess cost but does "
            "not compensate for a weak password. Final production Argon2 memory/time parameters also "
            "require calibration on both supported hardware families."
        ),
    },
    {
        "id": "firmware-update",
        "area": "firmware update behavior",
        "summary": (
            "Production boot rejects unconfigured hashes, unsigned images, invalid signatures, "
            "rollback, inconsistent verification passes, and flow/canary violations."
        ),
        "evidence": {
            "apps/kaskold-hardware/src/services/verify/policy.rs": [
                "cfg(feature = \"production\")",
                "verify_production",
            ],
            "apps/kaskold-hardware/src/services/verify/policy/production.rs": [
                "Hash not configured in production",
                "Unsigned build in production mode",
                "FlowViolation",
                "verify_signature",
            ],
            "apps/kaskold-hardware/src/services/verify/signature.rs": [
                "firmware signing public key not configured",
                "schnorr_verify",
                "InvalidSignature",
            ],
            "tools/build/firmware/build_production.sh": ["build_with_hash.sh", "production"],
        },
        "residual_risk": (
            "Execution of the update/boot path on physical boards and resistance to voltage/clock "
            "fault injection remain deferred HIL/laboratory work."
        ),
    },
    {
        "id": "hardware-attestation",
        "area": "hardware-rooted firmware attestation",
        "summary": (
            "Production boot reports whether the ESP32-S3 Secure Boot v2 eFuse is enforced. With plaintext flash "
            "it binds the displayed secure-padded app digest to the RSA signature block; with flash "
            "encryption it displays the already Schnorr-verified code digest rather than hashing "
            "raw ciphertext. Any attestation failure halts boot."
        ),
        "evidence": {
            "apps/kaskold-hardware/src/services/verify/attestation/mod.rs": [
                "SECURE_BOOT_EN",
                "secure_boot_enabled",
                "flash_encryption",
                "BuildIdentityMissing",
                "verify_running_image",
            ],
            "apps/kaskold-hardware/src/services/verify/attestation/image.rs": [
                "secure_boot_signature_offset",
                "parse_signature_digest",
                "constant_time::eq",
                "verify_secure_boot_digest",
            ],
            "apps/kaskold-hardware/src/runtime/signing/verification.rs": [
                "Signed identity SHA-256",
                "show_verification_screen",
                "fail_boot",
            ],
            "apps/kaskold-hardware/build.rs": [
                "KASKOLD_BUILD_COMMIT",
                "valid_commit",
            ],
            "docs/EFUSE_RUNBOOK.md": [
                "CONFIG_SECURE_BOOT=y",
                "secure_pad_v2.py",
                "one-byte-modified/unsigned/wrong-key application",
            ],
        },
        "residual_risk": (
            "Physical fault-injection resistance and the sacrificial-board proof that the "
            "provisioned Secure-Boot-v2 second-stage bootloader rejects modified applications "
            "remain HIL/manufacturing controls."
        ),
    },
    {
        "id": "side-channel",
        "area": "side-channel exposure",
        "summary": (
            "Source review confirms zeroization and rejects secret-bearing production log "
            "arguments in critical paths."
        ),
        "evidence": {
            "crates/shared-signer/src/bytes.rs": ["write_volatile", "compiler_fence"],
            "apps/kaskold-hardware/src/runtime/signing/derivation.rs": ["zeroize_seed"],
        },
        "residual_risk": (
            "Physical timing, EM, power, cache/bus leakage, fault injection, and invasive attacks "
            "are not established by host tests or source inspection."
        ),
    },
    {
        "id": "recovery-compatibility",
        "area": "wallet recovery boundary",
        "summary": (
            "Mnemonic recovery words plus the optional BIP39 passphrase remain the portable "
            "master recovery path. KasKold-owned encrypted formats are Argon2id-only. Historical "
            "PBKDF2 KASDB004/KAS\\x03/KSWLT003 formats are intentionally unsupported and "
            "explicitly rejected rather than migrated or probed."
        ),
        "evidence": {
            "crates/offline-signer/src/crypto/container_framing.rs": [
                "KASDB005",
                "TRANSPORT_CURRENT_MAGIC",
            ],
            "crates/offline-signer/src/crypto/unit_tests/container_framing_tests.rs": [
                "historical_pbkdf2_backup_magic_is_rejected",
                "historical_pbkdf2_transport_magic_is_rejected",
            ],
            "apps/kaskold-hardware/src/services/backup/container.rs": [
                "header.parameters",
                "PasswordKdfPurpose::DeviceBoundBackup",
            ],
            "apps/kaskold-hardware/src/services/stego/payload.rs": [
                "const PORTABLE_FORMAT_VERSION: u8 = 4;",
                "password_kdf::parse_metadata",
            ],
            "SECURITY.md": [
                "mnemonic recovery words",
                "JPEG + password",
                "PBKDF2 only where required by BIP39",
            ],
        },
    },

]

def audit() -> tuple[list[str], dict[str, Any]]:
    findings: list[dict[str, Any]] = []
    errors: list[str] = []
    for control in CONTROLS:
        missing: list[str] = []
        files: list[str] = []
        evidence_locations: list[dict[str, Any]] = []
        for relative_path, terms in control["evidence"].items():
            resolved, _ = collect_text(relative_path)
            files.extend(resolved)
            if not resolved:
                missing.append(f"missing evidence path {relative_path}")
                continue
            for term in terms:
                matches = evidence_matches(relative_path, term)
                if not matches:
                    missing.append(f"{relative_path} missing non-comment evidence {term!r}")
                    continue
                evidence_locations.append({
                    "path": relative_path,
                    "term": term,
                    "matches": matches,
                })
        if missing:
            status = "finding"
            errors.extend(f"{control['id']}: {item}" for item in missing)
        elif control.get("residual_risk"):
            status = "pass-with-residual-risk"
        else:
            status = "pass"
        findings.append(
            {
                "id": control["id"],
                "area": control["area"],
                "status": status,
                "summary": control["summary"],
                "evidence_files": sorted(set(files)),
                "evidence_locations": evidence_locations,
                "missing_evidence": missing,
                "residual_risk": control.get("residual_risk"),
            }
        )

    scan_errors, scans = source_scans()
    errors.extend(scan_errors)
    residual_count = sum(bool(item.get("residual_risk")) for item in findings)
    report = {
        "schema_version": 3,
        "review_type": "internal source security control evidence scan",
        "third_party_independent_review": False,
        "healthy": not errors,
        "hardware_in_the_loop": {
            "status": "deferred-by-owner",
            "reason": "No hardware device is currently available; manual execution is required later.",
        },
        "summary": {
            "areas_reviewed": len(findings),
            "areas_passed": sum(item["status"].startswith("pass") for item in findings),
            "blocking_source_findings": len(errors),
            "residual_risks": residual_count,
        },
        "areas": findings,
        "source_scans": scans,
        "limitations": [
            "This is an internal source control-evidence scan, not a third-party audit or semantic proof.",
            "Evidence probes are token-aware and exclude comments, but they do not prove full call-graph reachability or cryptographic correctness; behavioral, mutation, fuzz, HIL, and independent review remain separate gates.",
            "Host testing cannot establish resistance to physical power, timing, EM, fault-injection, or invasive attacks.",
            "The backup KDF work factor cannot be safely raised without versioning and real-device performance calibration.",
            "Mutation and fuzz results must be generated by their real runners; this review does not infer those outcomes.",
        ],
        "errors": errors,
    }
    return errors, report


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    arguments = parser.parse_args()
    errors, report = audit()
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n")
    for error in errors:
        print(f"ERROR: {error}")
    if errors:
        return 1
    print(
        f"PASS: internal source security control evidence scan passed {report['summary']['areas_passed']}/"
        f"{report['summary']['areas_reviewed']} areas with "
        f"{report['summary']['residual_risks']} explicit residual risks; HIL remains deferred"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
