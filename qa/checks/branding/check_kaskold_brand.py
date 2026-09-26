#!/usr/bin/env python3
"""Fail on stale KasSigner/KasSee product branding outside narrow compatibility exceptions.

KasKold is derived from KasSigner, so upstream attribution and cryptographic/wire
identifiers that are part of compatibility contracts intentionally retain their
historical spelling.  Everything user-facing and all new product/package identity
must use KasKold/Companion naming.
"""
from __future__ import annotations

from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]

# Legal/history is not product branding.  These files are deliberately allowed to
# describe KasSigner, KasSee, and the upstream maintainer/project by name.
_ALLOWED_PATHS = {
    "CHANGELOG.md",
    # The repository inventory is a mechanical path manifest checked separately.
    # Ignoring its text here avoids duplicate/stale-path diagnostics while it is
    # being reconciled by the inventory gate itself.
    "qa/baselines/repository_inventory.txt",
    # The detector and its fixture tests must spell the forbidden tokens in
    # order to define and exercise the policy; they are not product surfaces.
    "qa/checks/branding/check_kaskold_brand.py",
    "qa/tests/tooling/test_kaskold_branding.py",
}
_ALLOWED_PREFIXES = (
    "docs/legal/",
)

# Historical byte strings below are compatibility/security domains, encrypted
# storage labels, or wire-format keys.  Renaming any of them is a protocol or
# cryptographic migration, not a brand change.  The list is intentionally exact
# enough that a normal UI/help-text occurrence of "KasSigner" will still fail.
_COMPATIBILITY_MARKERS = (
    "kassignerDerivation",
    "KasSigner/anti-klepto/",
    "KasSigner-KSPT-v1",
    "KasSigner-ambient-stage-v2",
    "KasSigner-ambient-touch-v2",
    "KasSigner-seed-imu-v2",
    "KasSigner/additive-dice/v1",
    "KasSigner/additive-touch/v1",
    "KasSigner/optional-unique-id/v1",
    "KasSigner-touch-seed-v2",
    "KasSigner immutable advanced policy v1",
    "KasSigner duress credential verifier v1",
    "KasSigner wallet activation verifier v1",
    "KasSigner/CoreS3/dev-storage-test-key/v1",
    "KasSigner device-bound SD wallet slot A v4",
    "KasSigner device-bound SD wallet slot B v4",
    "KasSigner device-bound SD wallet slot A v3",
    "KasSigner device-bound SD wallet slot B v3",
    "KasSigner/stego-descriptor-credential/v2",
    "KasSigner/stego-wallet/envelope-aad/v4",
    "KasSigner-dice-entropy-1:",
    "KasSigner-dice-entropy-2:",
    "KasSigner-touch-entropy-v2",
    "KasSigner-stego-perm-v1",
    "KasSigner/persistent-credential-confirm/v1",
    "KasSigner/FirmwareManifest/v3",
    "KasSigner Private Swap Adaptor Nonce v2",
    "KasSigner Private Swap Anti-Klepto v2",
    "KasSigner/device-bound-wallet/context/v1",
    "KasSigner/device-bound-wallet/device-mix/v1",
    "KasSigner/device-bound-wallet/aes-256-gcm/v1",
    "KasSigner-ECIES-v1",
    "KasSigner Signed Message v1",
    "KasSigner/Argon2id/effective-salt/v1",
    "KasSigner/password-kdf/portable-backup/v1",
    "KasSigner/password-kdf/persistent-wallet/v1",
    "KasSigner/password-kdf/encrypted-transport/v1",
    "KasSigner/password-kdf/device-bound-backup/v1",
    "KasSigner Covenant Key v1",
    "KasSigner Covenant Binding Record v1",
    "KasSigner Private Swap Binding Record v2",
    "KasSigner/BIP340/input-aux/v1",
    "KasSigner Oracle v1",
    "KasSignerCrowdfundV2",
    "KasSigner-Stealth-Announce-v1",
    "KasSigner Private Swap Session v2",
    "KasSigner Private Swap KSPT v2",
    "KasSigner privacy account fingerprint v1",
    "KasSigner/multi-frame/v1",
)

# Compatibility documentation can name the upstream spelling, but only at the
# exact generator site that owns that explanatory sentence. Keeping this path-
# scoped prevents a UI/help surface from bypassing the product-brand check by
# borrowing generic compatibility wording.
_ALLOWED_COMPATIBILITY_DOC_MARKERS = {
    "tools/build/docs/generate_kaskold_guides.py": ("historical KasSigner spelling",),
}

# These are always stale product identity outside legal/history.  Mixed-case
# KasSigner/kassigner is handled separately so compatibility bytes can survive.
_FORBIDDEN_ALWAYS = (
    "KasSee",
    "kassee",
    "InKasWeRust",
    "kassigner.org",
    "org.kassigner",
    "@kassigner/",
    "KASSIGNER_",
    "kassigner-",
    "signer_firmware_core",
    "signer-firmware-core",
)


def _inventory_worktree_files(root: Path) -> list[Path]:
    inventory = root / "qa/baselines/repository_inventory.txt"
    files: list[Path] = []
    for line in inventory.read_text(encoding="utf-8").splitlines():
        kind, separator, relative = line.partition("\t")
        if separator and kind == "F":
            path = root / relative
            if path.is_file():
                files.append(path)
    return files


def _tracked_worktree_files(root: Path) -> list[Path]:
    try:
        result = subprocess.run(
            ["git", "ls-files", "-z"], cwd=root, check=True, capture_output=True
        )
    except (OSError, subprocess.CalledProcessError):
        inventory = root / "qa/baselines/repository_inventory.txt"
        if inventory.is_file():
            return _inventory_worktree_files(root)
        raise

    files: list[Path] = []
    for raw in result.stdout.split(b"\0"):
        if not raw:
            continue
        relative = raw.decode("utf-8", errors="strict")
        path = root / relative
        # A tracked path may be deleted in the current working tree; it should not
        # be reported as stale branding because it will not ship.
        if path.is_file():
            files.append(path)
    return files


def _is_legal_history(relative: str) -> bool:
    name = Path(relative).name
    return (
        relative in _ALLOWED_PATHS
        or relative.startswith(_ALLOWED_PREFIXES)
        or name.startswith("LICENSE")
    )


def _is_upstream_copyright(line: str) -> bool:
    return "Copyright" in line and (
        "KasSigner Project" in line or "kassigner@proton.me" in line
    )


def _has_compatibility_marker(line: str) -> bool:
    return any(marker in line for marker in _COMPATIBILITY_MARKERS)


def _has_compatibility_doc_marker(relative: str, line: str) -> bool:
    return any(
        marker in line
        for marker in _ALLOWED_COMPATIBILITY_DOC_MARKERS.get(relative, ())
    )


def check(root: Path = ROOT) -> list[str]:
    errors: list[str] = []
    try:
        files = _tracked_worktree_files(root)
    except (OSError, subprocess.CalledProcessError, UnicodeDecodeError) as exc:
        return [f"branding audit could not enumerate tracked files: {exc}"]

    for path in files:
        relative = path.relative_to(root).as_posix()
        if _is_legal_history(relative):
            continue

        lower_name = relative.lower()
        for token in ("kassigner", "kassee", "kas-see", "inkaswerust", "waveshare"):
            if token in lower_name:
                errors.append(f"stale product/board name in path: {relative}")
                break

        try:
            data = path.read_bytes()
        except OSError as exc:
            errors.append(f"branding audit could not read {relative}: {exc}")
            continue
        if b"\0" in data:
            continue
        try:
            text = data.decode("utf-8")
        except UnicodeDecodeError:
            continue

        for line_number, line in enumerate(text.splitlines(), 1):
            if _is_upstream_copyright(line):
                continue
            forbidden = [token for token in _FORBIDDEN_ALWAYS if token in line]
            if forbidden:
                errors.append(
                    f"{relative}:{line_number}: stale brand identifier {forbidden[0]!r}"
                )
                continue

            if "KasSigner" in line or "kassigner" in line:
                if not (
                    _has_compatibility_marker(line)
                    or _has_compatibility_doc_marker(relative, line)
                ):
                    errors.append(
                        f"{relative}:{line_number}: unexpected KasSigner/kassigner branding"
                    )

            if "Waveshare" in line or "waveshare" in line:
                errors.append(
                    f"{relative}:{line_number}: retired Waveshare board reference"
                )

    return errors


def main() -> int:
    errors = check(ROOT)
    if errors:
        for error in errors:
            print(f"ERROR: {error}")
        return 1
    print(
        "PASS: KasKold brand audit found no stale product identity; "
        "only explicit legal/history and compatibility identifiers remain"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
