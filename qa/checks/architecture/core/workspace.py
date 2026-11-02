from __future__ import annotations

from pathlib import Path
import re
import tomllib

from firmware.firmware_lint_policy import check_policy as check_firmware_lint_policy
from .common import is_generated_tree
from architecture.web.generated_output import check_web_pkg_policy
from .quality_ownership import check_quality_ownership
from architecture.tooling.workspace_delivery import _check_firmware_delivery_paths, _check_master_test_runner
from .sdk.workspace import check as check_sdk_workspace


def _check_qa_test_layout(root: Path) -> list[str]:
    """Reject Rust integration-test roots that collide with sibling module directories."""
    errors: list[str] = []
    tests_root = root / "qa/tests"
    if not tests_root.is_dir():
        return errors

    for test_root in sorted(tests_root.glob("*.rs")):
        module_name = test_root.stem
        module_dir = tests_root / module_name
        module_mod = module_dir / "mod.rs"
        if not module_mod.is_file():
            continue

        source = test_root.read_text(errors="ignore")
        ambiguous = re.search(
            rf"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+{re.escape(module_name)}\s*;",
            source,
        )
        if ambiguous:
            errors.append(
                f"ambiguous Rust integration-test module: {test_root.relative_to(root)} "
                f"declares `mod {module_name};` while "
                f"{module_mod.relative_to(root)} also exists; use an explicit #[path] alias"
            )
    return errors

def _check_repository_metadata(root: Path) -> list[str]:
    errors: list[str] = []
    attributes = (root / ".gitattributes").read_text(errors="ignore").splitlines()
    active = [line.strip() for line in attributes if line.strip() and not line.lstrip().startswith("#")]
    duplicates = sorted({line for line in active if active.count(line) > 1})
    if duplicates:
        errors.append(f".gitattributes contains duplicate rules: {duplicates}")
    for line in active:
        token = line.split(maxsplit=1)[0]
        if token.startswith("companion/") or token.startswith("web/"):
            errors.append(f".gitattributes retains stale repository path: {token}")
    ignore = (root / ".gitignore").read_text(errors="ignore")
    for required in ("__pycache__/", "*.py[cod]"):
        if required not in ignore:
            errors.append(f".gitignore must exclude Python cache artifacts: {required}")
    return errors

def _check_root_manifest_and_lints(root: Path) -> list[str]:
    errors: list[str] = []
    root_manifest = tomllib.loads((root / "Cargo.toml").read_text())
    if "package" in root_manifest or "lib" in root_manifest:
        errors.append("root Cargo.toml must be a virtual workspace without a root package")
    errors.extend(check_firmware_lint_policy(root))
    return errors

def _check_business_facades(root: Path) -> list[str]:
    errors: list[str] = []
    business_facades = {
        root / "crates/offline-signer/src/facade.rs": "OfflineSigner",
        root / "crates/online-watcher/src/facade.rs": "WatchWallet",
    }
    for path, expected_struct in business_facades.items():
        if not path.is_file():
            errors.append(f"missing business facade module: {path.relative_to(root)}")
            continue
        source = path.read_text(errors="ignore")
        structs = re.findall(r"\bpub\s+struct\s+([A-Za-z_][A-Za-z0-9_]*)", source)
        if structs != [expected_struct]:
            errors.append(
                f"business facade {path.relative_to(root)} must expose only {expected_struct}, got {structs}"
            )
        line_count = len(source.splitlines())
        if line_count > 250:
            errors.append(
                f"business facade exceeds coordination limit: {path.relative_to(root)} ({line_count} lines)"
            )

    unexpected: list[str] = []
    facade_name = re.compile(r"\bpub\s+struct\s+([A-Za-z_][A-Za-z0-9_]*Facade)\b")
    for path in root.rglob("*.rs"):
        if path in business_facades or any(part in {"target", "external"} for part in path.parts):
            continue
        for name in facade_name.findall(path.read_text(errors="ignore")):
            unexpected.append(f"{path.relative_to(root)}::{name}")
    if unexpected:
        errors.append("unexpected broad business facade types: " + ", ".join(sorted(unexpected)))
    return errors

def _check_core_groups(root: Path, core: Path) -> list[str]:
    errors: list[str] = []
    expected_groups = {
        "backup", "camera", "entropy", "input", "power", "presentation",
        "qr", "runtime", "screen_flow", "security", "storage", "time", "update",
    }
    actual_groups = {
        path.name for path in core.iterdir()
        if path.is_dir() and path.name != "unit_tests"
    }
    if actual_groups != expected_groups:
        errors.append(
            "kaskold-hardware-core must group device decisions by backup, camera, entropy, "
            "input, power, presentation, qr, runtime, screen_flow, security, storage, time, and update"
        )
    for group in sorted(core.iterdir()):
        if not group.is_dir() or group.name == "unit_tests":
            continue
        direct_modules = [
            path for path in group.iterdir()
            if path.is_file() and path.name != "mod.rs"
        ]
        if len(direct_modules) > 6:
            errors.append(
                f"crowded kaskold-hardware-core group: {group.relative_to(root)} has "
                f"{len(direct_modules)} modules (maximum 6)"
            )
    return errors

def _check_shared_signer_policy(root: Path) -> list[str]:
    errors: list[str] = []
    shared_lib = (root / "crates/shared-signer/src/lib.rs").read_text(errors="ignore")
    for forbidden in ("firmware", "entropy", "release"):
        if f"pub mod {forbidden};" in shared_lib:
            errors.append(f"shared-signer must not expose firmware-owned module: {forbidden}")
    hardware_time = root / "crates/kaskold-hardware-core/src/time/mod.rs"
    if "pub use kaspa_portal::transaction::policy::time as advanced_policy;" not in shared_lib:
        errors.append("portable signing policy must be Kaspa Portal's transaction::policy::time")
    if (root / "crates/shared-signer/src/advanced_policy").exists():
        errors.append("shared-signer must not keep a copy of Kaspa Portal's signing policy")
    if not hardware_time.is_file() or "pub use shared_signer::advanced_policy;" not in hardware_time.read_text(errors="ignore"):
        errors.append("hardware-core advanced policy must remain a thin re-export of shared-signer")
    return errors

def _check_stego_modules(root: Path) -> list[str]:
    errors: list[str] = []
    shared_stego = root / "crates/shared-signer/src/stego_picture/mod.rs"
    hardware_stego = root / "crates/kaskold-hardware-core/src/backup/stego_picture.rs"
    if not shared_stego.is_file():
        errors.append("shared steganography codec must live in shared-signer/src/stego_picture")
    if not hardware_stego.is_file():
        errors.append("hardware steganography compatibility shim is missing")
    else:
        shim = hardware_stego.read_text(errors="ignore")
        if "pub use shared_signer::stego_picture::*;" not in shim:
            errors.append(
                "hardware steganography module must be a thin re-export of shared-signer"
            )
        if any(token in shim for token in ("fn embed(", "fn extract(", "struct Jpeg", "enum PictureError")):
            errors.append(
                "hardware steganography shim must not duplicate the shared carrier implementation"
            )
    if (root / "crates/kaskold-hardware-core/src/backup/stego_picture").exists():
        errors.append(
            "hardware-core must not retain a duplicate stego_picture implementation directory"
        )
    return errors

def _check_descriptor_and_firmware_residue(root: Path) -> list[str]:
    errors: list[str] = []
    if (root / "crates/shared-signer/src/firmware").exists():
        errors.append("shared-signer/src/firmware must not exist after firmware-core extraction")
    descriptor = root / "crates/kaskold-protocol/src/wire/multisig_descriptor.rs"
    if not descriptor.is_file():
        errors.append("canonical multisig descriptor parser must live in kaskold-protocol wire")
    return errors

def _check_firmware_core_ownership(root: Path) -> list[str]:
    core = root / "crates/kaskold-hardware-core/src"
    errors = _check_core_groups(root, core)
    errors.extend(_check_shared_signer_policy(root))
    errors.extend(_check_stego_modules(root))
    errors.extend(_check_descriptor_and_firmware_residue(root))
    return errors

def _check_workspace_manifests(root: Path) -> list[str]:
    errors: list[str] = []
    root_workspace = tomllib.loads((root / "Cargo.toml").read_text()).get("workspace", {})
    expected_workspace_members = {
        "crates/kaskold-protocol",
        "crates/kaskold-sdk",
        "crates/offline-signer",
        "crates/online-watcher",
        "crates/shared-signer",
        "crates/kaskold-hardware-core",
        "crates/hot-wallet",
        "crates/vault-runtime",
    }
    expected_workspace_excludes = {
        "apps/kaskold-hardware",
        "apps/kaskold-companion-web",
        "apps/kaskold-vault-web",
        "tools",
        "qa",
        "qa/firmware-sim",
        "qa/fuzz",
        "external/rqrr-nostd",
    }
    if set(root_workspace.get("members", [])) != expected_workspace_members:
        errors.append("root workspace must contain only reusable library crates")
    if set(root_workspace.get("exclude", [])) != expected_workspace_excludes:
        errors.append("independent applications, tools, and QA must be excluded from the root workspace")
    for relative in (
        "apps/kaskold-hardware/Cargo.toml",
        "apps/kaskold-companion-web/Cargo.toml",
        "apps/kaskold-vault-web/Cargo.toml",
        "tools/Cargo.toml",
        "qa/Cargo.toml",
        "qa/fuzz/Cargo.toml",
        "external/rqrr-nostd/Cargo.toml",
    ):
        manifest = tomllib.loads((root / relative).read_text())
        if manifest.get("workspace", {}).get("resolver") != "2":
            errors.append(f"independent workspace marker is missing: {relative}")

    if (root / "rust-toolchain.toml").exists():
        errors.append("the ESP toolchain pin must not apply to every workspace")
    firmware_toolchain = root / "apps/kaskold-hardware/rust-toolchain.toml"
    if not firmware_toolchain.is_file() or 'channel = "esp"' not in firmware_toolchain.read_text():
        errors.append("firmware-local ESP toolchain pin is missing")
    return errors

def _check_obsolete_paths(root: Path) -> list[str]:
    errors: list[str] = []
    for obsolete in (
        "bootloader", "companion", "rqrr_nostd", "platforms", "vendor", "hardware",
        "src", "tests", "benches", "fuzz",
    ):
        if (root / obsolete).exists():
            errors.append(f"obsolete or misplaced top-level path exists: {obsolete}")
    return errors

def _check_crowded_source_roots(root: Path) -> list[str]:
    errors: list[str] = []
    organized_roots = {
        root / "crates/kaskold-protocol/src": 1,
        root / "crates/kaskold-sdk/src": 1,
        root / "crates/offline-signer/src": 2,
        root / "crates/online-watcher/src": 2,
        root / "crates/kaskold-hardware-core/src": 3,
        root / "apps/kaskold-hardware/src/hw": 1,
    }
    for directory, maximum_files in organized_roots.items():
        direct_files = [path for path in directory.iterdir() if path.is_file()]
        if len(direct_files) > maximum_files:
            errors.append(
                f"crowded source root: {directory.relative_to(root)} has "
                f"{len(direct_files)} direct files (maximum {maximum_files})"
            )
    return errors

def _check_qa_paths(root: Path) -> list[str]:
    errors: list[str] = []
    required_qa_paths = [
        root / "qa/Cargo.toml",
        root / "qa/tests/common",
        root / "qa/tests/conformance",
        root / "qa/tests/integration",
        root / "qa/tests/fixtures",
        root / "qa/benches",
        root / "qa/fuzz/Cargo.toml",
        root / "qa/fuzz/unwrap_qr_payload.rs",
    ]
    for required in required_qa_paths:
        if not required.exists():
            errors.append(f"required QA path is missing: {required.relative_to(root)}")

    if (root / "qa/src").exists():
        errors.append("qa must not contain an artificial src/ package target")
    if (root / "qa/fuzz/fuzz_targets").exists():
        errors.append("qa/fuzz must be flat; fuzz_targets/ is not allowed")
    return errors

def _check_forbidden_test_names(root: Path) -> list[str]:
    errors: list[str] = []
    for forbidden_name in ("boot_test.rs", "self_test.rs"):
        for path in root.rglob(forbidden_name):
            if is_generated_tree(path, root):
                continue
            errors.append(f"test implementation must live under unit_tests/: {path.relative_to(root)}")
    return errors

def _check_inline_tests(root: Path) -> list[str]:
    errors: list[str] = []
    for path in root.rglob("*.rs"):
        relative = path.relative_to(root)
        if (
            is_generated_tree(path, root)
            or "unit_tests" in relative.parts
            or "tests" in relative.parts
            or relative.parts[0] == "qa"
        ):
            continue
        source = path.read_text(errors="ignore")
        if "#[test]" in source or re.search(r"\bmod\s+tests\b", source):
            errors.append(f"inline test implementation remains in production source: {relative}")
    return errors

def _check_duplicate_crate_roots(root: Path) -> list[str]:
    errors: list[str] = []
    for duplicate_root in root.rglob("src/mod.rs"):
        if is_generated_tree(duplicate_root, root):
            continue
        if (duplicate_root.parent / "lib.rs").exists():
            errors.append(f"duplicate crate root module exists: {duplicate_root.relative_to(root)}")
    return errors

def _check_production_lint_allowances(root: Path) -> list[str]:
    errors: list[str] = []
    production_roots = (
        root / "apps/kaskold-hardware/src",
        root / "crates/offline-signer/src",
        root / "crates/online-watcher/src",
        root / "crates/shared-signer/src",
        root / "crates/kaskold-hardware-core/src",
    )
    for source_root in production_roots:
        for path in source_root.rglob("*.rs"):
            source = path.read_text(errors="ignore")
            relative = path.relative_to(root)
            if re.search(r"(?m)^#!\[allow\(", source):
                errors.append(f"crate/file-wide lint allowance is forbidden: {relative}")
            if "#[allow(deprecated)]" in source:
                errors.append(f"deprecated API suppression is forbidden: {relative}")
            if "#[allow(dead_code)]" in source:
                errors.append(f"retained dead-code suppression is forbidden: {relative}")
    return errors


def check(root: Path) -> list[str]:
    errors: list[str] = [
        *_check_firmware_delivery_paths(root),
        *_check_master_test_runner(root),
        *check_quality_ownership(root),
        *_check_qa_test_layout(root),
        *_check_repository_metadata(root),
        *check_web_pkg_policy(root),
    ]
    errors.extend(_check_obsolete_paths(root))
    errors.extend(check_sdk_workspace(root))
    errors.extend(_check_business_facades(root))
    errors.extend(_check_workspace_manifests(root))
    errors.extend(_check_crowded_source_roots(root))
    errors.extend(_check_firmware_core_ownership(root))
    errors.extend(_check_qa_paths(root))
    errors.extend(_check_forbidden_test_names(root))
    errors.extend(_check_inline_tests(root))
    errors.extend(_check_duplicate_crate_roots(root))
    errors.extend(_check_production_lint_allowances(root))
    errors.extend(_check_root_manifest_and_lints(root))
    return errors
