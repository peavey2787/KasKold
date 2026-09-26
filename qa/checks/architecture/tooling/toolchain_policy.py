from __future__ import annotations

import json
from pathlib import Path
import re
import sys

CHECKS_DIR = Path(__file__).resolve().parents[2]
if str(CHECKS_DIR) not in sys.path:
    sys.path.insert(0, str(CHECKS_DIR))

from toolchains import load_toolchains  # noqa: E402
from security.fuzz_targets import registered_targets, validate_targets  # noqa: E402


def _consumer_requirements() -> dict[str, tuple[str, ...]]:
    return {
        "Makefile": ("include qa/config/toolchains.env",),
        "qa/linux/runner/environment.sh": ("qa/config/toolchains.env", "KASKOLD_ANDROID_JDK"),
        "tools/build/web/build_companion_runtime.py": (
            "qa/config/toolchains.env", "KASKOLD_STABLE_RUST", "KASKOLD_WASM_BINDGEN_CLI_VERSION",
        ),
        "scripts/linux/build/android-studio.sh": (
            "qa/config/toolchains.env", "KASKOLD_ANDROID_JDK",
        ),
        "qa/linux/runner/commands.sh": (
            "KASKOLD_STABLE_RUST", "KASKOLD_BRANCH_RUST",
            "KASKOLD_CARGO_FUZZ_VERSION", "fuzz_targets.py",
        ),
        "qa/linux/run-pinned-branch-coverage.sh": (
            "qa/config/toolchains.env", "KASKOLD_BRANCH_RUST",
            "KASKOLD_CARGO_LLVM_COV_VERSION", "KASKOLD_CARGO_CRAP_VERSION",
        ),
        "qa/linux/run-security-fuzz.sh": (
            "qa/config/toolchains.env", "KASKOLD_CARGO_FUZZ_VERSION", "fuzz_targets.py",
        ),
        "scripts/linux/quality/branch-coverage-setup.sh": (
            "qa/config/toolchains.env", "KASKOLD_BRANCH_RUST",
            "KASKOLD_CARGO_LLVM_COV_VERSION", "KASKOLD_CARGO_CRAP_VERSION",
        ),
        "scripts/linux/install/install.sh": (
            "qa/config/toolchains.env", "KASKOLD_STABLE_RUST", "KASKOLD_BRANCH_RUST",
            "KASKOLD_CARGO_FUZZ_VERSION", "KASKOLD_CARGO_MUTANTS_VERSION",
            "KASKOLD_ANDROID_JDK", "KASKOLD_GRADLE_VERSION", "KASKOLD_KOTLIN_CLI_VERSION",
            "KASKOLD_ANDROID_API", "KASKOLD_ANDROID_BUILD_TOOLS", "KASKOLD_ANDROID_CMDLINE_TOOLS",
            "KASKOLD_ANDROID_CMDLINE_TOOLS_LINUX_SHA256",
        ),
        "scripts/windows/install/install.ps1": (
            "Import-KasKoldToolchains", "KASKOLD_STABLE_RUST", "KASKOLD_BRANCH_RUST",
            "KASKOLD_CARGO_FUZZ_VERSION", "KASKOLD_CARGO_MUTANTS_VERSION",
            "KASKOLD_ANDROID_JDK", "KASKOLD_GRADLE_VERSION", "KASKOLD_KOTLIN_CLI_VERSION",
            "KASKOLD_ANDROID_API", "KASKOLD_ANDROID_BUILD_TOOLS", "KASKOLD_ANDROID_CMDLINE_TOOLS",
            "KASKOLD_ANDROID_CMDLINE_TOOLS_WINDOWS_SHA256",
        ),
        "scripts/linux/lib/qemu-common.sh": ("qa/config/toolchains.env", "KASKOLD_ESP_IDF_VERSION"),
        "scripts/linux/lib/qemu-rust.sh": (
            "KASKOLD_ESPUP_VERSION", "KASKOLD_ESP_RUST", "KASKOLD_ESPFLASH_VERSION",
        ),
        "Dockerfile.base": (
            "COPY qa/config/toolchains.env /etc/kaskold/toolchains.env",
            "BUILD-INPUT-SHA256SUMS", "CARGO_NET_OFFLINE=true", "--offline",
        ),
        "scripts/linux/build/reproducible/prefetch.py": (
            "qa/config/toolchains.env", "KASKOLD_UBUNTU_BASE_DIGEST", "KASKOLD_UBUNTU_SNAPSHOT",
        ),
        "scripts/linux/build/reproducible/toolchains.py": (
            "KASKOLD_RUSTUP_VERSION", "KASKOLD_REPRO_HOST_RUST", "KASKOLD_ESPUP_VERSION",
            "KASKOLD_ESP_RUST", "KASKOLD_ESPFLASH_VERSION",
        ),
        "scripts/linux/build/reproducible-build.sh": ("prefetch.py", "--network=none", "--pull=false"),
        "Dockerfile": (
            "/etc/kaskold/toolchains.env", "KASKOLD_REPRO_HOST_RUST", "KASKOLD_ESPFLASH_VERSION",
        ),
        ".github/workflows/core.yml": (
            "qa/config/toolchains.env", "KASKOLD_STABLE_RUST",
            "make test STRICT_LOCKFILES=1",
        ),
        ".github/workflows/kaskold-companion-ci.yml": (
            "qa/config/toolchains.env", "KASKOLD_STABLE_RUST",
        ),
        ".github/workflows/android.yml": (
            "qa/config/toolchains.env", "KASKOLD_ANDROID_JDK",
            "KASKOLD_ANDROID_API", "KASKOLD_ANDROID_BUILD_TOOLS",
            "KASKOLD_ANDROID_CMDLINE_TOOLS",
            "KASKOLD_ANDROID_CMDLINE_TOOLS_LINUX_SHA256",
        ),
        ".github/workflows/ios.yml": (
            "qa/config/toolchains.env", "KASKOLD_STABLE_RUST",
        ),
        ".github/workflows/rqrr.yml": (
            "qa/config/toolchains.env", "KASKOLD_STABLE_RUST", "external/rqrr-nostd",
        ),
        ".github/workflows/fuzz.yml": (
            "FUZZ_SECONDS=300", "scripts/linux/quality/security-fuzz.sh",
        ),
    }


def _pin_consumers() -> dict[str, tuple[str, ...]]:
    return {
        "qa/linux/runner/commands.sh": ("KASKOLD_STABLE_RUST", "KASKOLD_BRANCH_RUST", "KASKOLD_CARGO_FUZZ_VERSION"),
        "tools/build/web/build_companion_runtime.py": ("KASKOLD_STABLE_RUST", "KASKOLD_WASM_BINDGEN_CLI_VERSION"),
        "qa/linux/run-pinned-branch-coverage.sh": ("KASKOLD_BRANCH_RUST", "KASKOLD_CARGO_LLVM_COV_VERSION", "KASKOLD_CARGO_CRAP_VERSION"),
        "qa/linux/run-security-fuzz.sh": ("KASKOLD_STABLE_RUST", "KASKOLD_BRANCH_RUST", "KASKOLD_CARGO_FUZZ_VERSION"),
        "scripts/linux/quality/branch-coverage-setup.sh": ("KASKOLD_BRANCH_RUST", "KASKOLD_CARGO_LLVM_COV_VERSION", "KASKOLD_CARGO_CRAP_VERSION"),
        "scripts/linux/install/install.sh": (
            "KASKOLD_STABLE_RUST", "KASKOLD_BRANCH_RUST", "KASKOLD_CARGO_FUZZ_VERSION",
            "KASKOLD_CARGO_MUTANTS_VERSION", "KASKOLD_GRADLE_VERSION",
            "KASKOLD_KOTLIN_CLI_VERSION", "KASKOLD_ANDROID_BUILD_TOOLS",
            "KASKOLD_ANDROID_CMDLINE_TOOLS", "KASKOLD_ANDROID_CMDLINE_TOOLS_LINUX_SHA256",
        ),
        "scripts/windows/install/install.ps1": (
            "KASKOLD_STABLE_RUST", "KASKOLD_BRANCH_RUST", "KASKOLD_CARGO_FUZZ_VERSION",
            "KASKOLD_CARGO_MUTANTS_VERSION", "KASKOLD_GRADLE_VERSION",
            "KASKOLD_KOTLIN_CLI_VERSION", "KASKOLD_ANDROID_BUILD_TOOLS",
            "KASKOLD_ANDROID_CMDLINE_TOOLS", "KASKOLD_ANDROID_CMDLINE_TOOLS_WINDOWS_SHA256",
        ),
        "scripts/linux/lib/qemu-common.sh": ("KASKOLD_ESP_IDF_VERSION",),
        "scripts/linux/lib/qemu-rust.sh": ("KASKOLD_ESPUP_VERSION", "KASKOLD_ESP_RUST", "KASKOLD_ESPFLASH_VERSION"),
        "Dockerfile.base": ("KASKOLD_REPRO_HOST_RUST", "KASKOLD_ESPFLASH_VERSION"),
        "scripts/linux/build/reproducible/toolchains.py": (
            "KASKOLD_RUSTUP_VERSION", "KASKOLD_REPRO_HOST_RUST", "KASKOLD_ESPUP_VERSION",
            "KASKOLD_ESP_RUST", "KASKOLD_ESPFLASH_VERSION",
        ),
        "scripts/linux/build/reproducible/prefetch.py": ("KASKOLD_UBUNTU_BASE_DIGEST", "KASKOLD_UBUNTU_SNAPSHOT"),
        "Dockerfile": ("KASKOLD_REPRO_HOST_RUST", "KASKOLD_ESPFLASH_VERSION"),
        ".github/workflows/core.yml": ("KASKOLD_STABLE_RUST", "KASKOLD_ESP_RUST"),
        ".github/workflows/kaskold-companion-ci.yml": ("KASKOLD_STABLE_RUST",),
        ".github/workflows/android.yml": (
            "KASKOLD_ANDROID_JDK", "KASKOLD_ANDROID_API", "KASKOLD_ANDROID_BUILD_TOOLS",
            "KASKOLD_ANDROID_CMDLINE_TOOLS", "KASKOLD_ANDROID_CMDLINE_TOOLS_LINUX_SHA256",
        ),
        ".github/workflows/ios.yml": ("KASKOLD_STABLE_RUST",),
        ".github/workflows/rqrr.yml": ("KASKOLD_STABLE_RUST",),
    }


def _contains_pin_literal(source: str, literal: str) -> bool:
    # Avoid false positives where a short numeric pin is merely a substring of
    # an unrelated identifier (for example Android JDK 25 inside `sha256sum`).
    # Version/hash characters form one token; punctuation and quotes delimit it.
    token_char = r"A-Za-z0-9_.-"
    pattern = rf"(?<![{token_char}]){re.escape(literal)}(?![{token_char}])"
    return re.search(pattern, source) is not None


def _check_consumers(root: Path, pins: dict[str, str]) -> list[str]:
    errors: list[str] = []
    for relative, required in _consumer_requirements().items():
        path = root / relative
        if not path.is_file():
            errors.append(f"toolchain-policy consumer is missing: {relative}")
            continue
        source = path.read_text(errors="replace")
        for token in required:
            if token not in source:
                errors.append(f"{relative} does not consume central toolchain policy: {token}")
    for relative, keys in _pin_consumers().items():
        source = (root / relative).read_text(errors="replace")
        for key in keys:
            literal = pins[key]
            if _contains_pin_literal(source, literal):
                errors.append(f"{relative} duplicates central toolchain pin {key}={literal}")
    return errors


def _check_offline_docker(root: Path) -> list[str]:
    errors: list[str] = []
    docker_base = (root / "Dockerfile.base").read_text(errors="replace")
    docker_release = (root / "Dockerfile").read_text(errors="replace")
    docker_runner = (root / "scripts/linux/build/reproducible-build.sh").read_text(errors="replace")
    for relative, source in (("Dockerfile.base", docker_base), ("Dockerfile", docker_release)):
        for token in ("apt-get update", "curl ", "wget ", "espup install", "rustup target add"):
            if token in source:
                errors.append(f"{relative} performs network-capable provisioning inside Docker: {token.strip()}")
    if docker_runner.count("--network=none") < 2:
        errors.append("reproducible-build runner must disable Docker networking for both Docker builds")
    if "docker pull" in docker_runner:
        errors.append("reproducible-build runner must not pull images through Docker")
    return errors


def _check_security_registry(root: Path) -> list[str]:
    errors: list[str] = []
    security_policy = json.loads((root / "qa/checks/security/policy.json").read_text())
    duplicated_policy_keys = {
        "cargo_mutants_version", "toolchain", "cargo_fuzz_version",
        "installer_toolchain", "execution_toolchain", "targets",
    }
    for section in ("mutation", "fuzz"):
        overlap = sorted(duplicated_policy_keys & security_policy.get(section, {}).keys())
        if overlap:
            errors.append(f"security policy duplicates toolchain/registry ownership in {section}: {overlap}")
    if (root / "qa/fuzz/rust-toolchain.toml").exists():
        errors.append("qa/fuzz/rust-toolchain.toml duplicates the central nightly pin")
    try:
        targets = registered_targets(root / "qa/fuzz/Cargo.toml")
    except (OSError, ValueError) as error:
        errors.append(f"authoritative fuzz registry is invalid: {error}")
        targets = ()
    errors.extend(validate_targets())
    if len(targets) != 13:
        errors.append(f"authoritative fuzz registry must contain 13 targets, got {len(targets)}")
    return errors


def check(root: Path) -> list[str]:
    try:
        pins = load_toolchains(root / "qa/config/toolchains.env")
    except (OSError, ValueError) as error:
        return [f"central toolchain policy is invalid: {error}"]
    return [
        *_check_consumers(root, pins),
        *_check_offline_docker(root),
        *_check_security_registry(root),
    ]
