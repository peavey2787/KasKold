"""Load the repository's single pinned toolchain/version policy."""

from __future__ import annotations

from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
POLICY_PATH = ROOT / "qa/config/toolchains.env"

REQUIRED = {
    "KASKOLD_STABLE_RUST",
    "KASKOLD_BRANCH_RUST",
    "KASKOLD_REPRO_HOST_RUST",
    "KASKOLD_ESP_RUST",
    "KASKOLD_ESPUP_VERSION",
    "KASKOLD_ESPFLASH_VERSION",
    "KASKOLD_ESP_IDF_VERSION",
    "KASKOLD_CARGO_LLVM_COV_VERSION",
    "KASKOLD_CARGO_CRAP_VERSION",
    "KASKOLD_CARGO_FUZZ_VERSION",
    "KASKOLD_CARGO_MUTANTS_VERSION",
    "KASKOLD_CARGO_NDK_VERSION",
    "KASKOLD_RUSTUP_VERSION",
    "KASKOLD_WASM_BINDGEN_CLI_VERSION",
    "KASKOLD_ANDROID_JDK",
    "KASKOLD_GRADLE_VERSION",
    "KASKOLD_KOTLIN_CLI_VERSION",
    "KASKOLD_ANDROID_API",
    "KASKOLD_ANDROID_BUILD_TOOLS",
    "KASKOLD_ANDROID_NDK",
    "KASKOLD_ANDROID_CMDLINE_TOOLS",
    "KASKOLD_ANDROID_CMDLINE_TOOLS_LINUX_SHA256",
    "KASKOLD_ANDROID_CMDLINE_TOOLS_WINDOWS_SHA256",
    "KASKOLD_UBUNTU_BASE_DIGEST",
    "KASKOLD_UBUNTU_SNAPSHOT",
    "KASKOLD_UBUNTU_CA_CERTIFICATES",
    "KASKOLD_UBUNTU_CURL",
    "KASKOLD_UBUNTU_GCC",
    "KASKOLD_UBUNTU_GXX",
    "KASKOLD_UBUNTU_LIBSSL_DEV",
    "KASKOLD_UBUNTU_LIBUDEV_DEV",
    "KASKOLD_UBUNTU_LIBUSB_DEV",
    "KASKOLD_UBUNTU_PKG_CONFIG",
    "KASKOLD_UBUNTU_PYTHON3",
}


def load_toolchains(path: Path = POLICY_PATH) -> dict[str, str]:
    values: dict[str, str] = {}
    for number, raw in enumerate(path.read_text().splitlines(), start=1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        if "=" not in line:
            raise ValueError(f"invalid toolchain policy line {number}: {raw!r}")
        key, value = line.split("=", 1)
        key = key.strip()
        value = value.strip()
        if not key or not value:
            raise ValueError(f"invalid toolchain policy line {number}: {raw!r}")
        if key in values:
            raise ValueError(f"duplicate toolchain policy key: {key}")
        values[key] = value
    missing = sorted(REQUIRED - values.keys())
    if missing:
        raise ValueError("missing toolchain policy keys: " + ", ".join(missing))
    unknown = sorted(values.keys() - REQUIRED)
    if unknown:
        raise ValueError("unknown toolchain policy keys: " + ", ".join(unknown))
    return values


def qa_bash_executable() -> str:
    """Return the exact Bash selected by the repository QA bootstrap.

    On native Windows, invoking the bare ``bash`` command is ambiguous because
    Windows may resolve ``System32\\bash.exe`` (the WSL launcher) before PATH.
    Shared repository QA exports KASKOLD_QA_BASH with the absolute Git Bash
    executable path. POSIX hosts keep using the ordinary ``bash`` command.
    """
    import os

    configured = os.environ.get("KASKOLD_QA_BASH", "").strip()
    return configured or "bash"
