#!/usr/bin/env python3
"""Fail on any RustSec advisory, warnings included, in every committed lockfile."""
from __future__ import annotations

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))

from toolchains import load_toolchains  # noqa: E402

# Every accepted advisory states why it cannot reach a shipped binary.
ACCEPTED_ADVISORIES = {
    # derivative (unmaintained) is a proc-macro arkworks runs at compile time.
    "RUSTSEC-2024-0388",
    # paste (unmaintained) is a proc-macro embedded-iconoir runs at compile time.
    "RUSTSEC-2024-0436",
    # basic-toml and unsafe-libyaml (unmaintained) parse esp-hal's own chip
    # metadata and config in build scripts; no firmware code links them.
    "RUSTSEC-2026-0374",
    "RUSTSEC-2024-0449",
    # instant (unmaintained) reaches only minifb in the desktop firmware
    # mirror developer tool, never a shipped binary.
    "RUSTSEC-2024-0384",
}


def installed_version() -> str:
    if shutil.which("cargo-audit") is None:
        return ""
    output = subprocess.run(
        ["cargo", "audit", "--version"], check=True, capture_output=True, text=True
    ).stdout
    return output.split()[-1]


def lockfiles() -> list[Path]:
    listed = subprocess.run(
        ["git", "ls-files", "*Cargo.lock"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout.split()
    return [ROOT / path for path in listed]


def audit(lockfile: Path) -> bool:
    command = ["cargo", "audit", "--deny", "warnings", "--file", str(lockfile)]
    for advisory in sorted(ACCEPTED_ADVISORIES):
        command += ["--ignore", advisory]
    print(f"==> cargo audit {lockfile.relative_to(ROOT).as_posix()}", flush=True)
    return subprocess.run(command, cwd=ROOT).returncode == 0


def main() -> int:
    required = load_toolchains()["KASKOLD_CARGO_AUDIT_VERSION"]
    found = installed_version()
    if found != required:
        print(f"FAIL: cargo-audit {required} is required (found {found or 'none'}); "
              f"install it with: cargo install cargo-audit --version {required} --locked")
        return 1
    failed = [lock for lock in lockfiles() if not audit(lock)]
    for lock in failed:
        print(f"FAIL: dependency advisories in {lock.relative_to(ROOT).as_posix()}")
    if failed:
        return 1
    print("PASS: no unaccepted RustSec advisory in any committed lockfile")
    return 0


if __name__ == "__main__":
    sys.exit(main())
