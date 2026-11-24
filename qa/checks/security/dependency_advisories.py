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
    # chrono (unmaintained) is locked only as an optional dependency of
    # serde_with under ark-serialize, behind a feature no build enables;
    # `cargo tree -i chrono --all-features --target all` is empty.
    "RUSTSEC-2026-0543",
}

# heapless 0.8.0 has nine advisories fixed only in 0.9.4. KasKold's own code
# uses 0.9.4. The latest esp-hal (through embassy-sync 0.6/0.7 and usb-device),
# mipidsi and usb-device releases still require ^0.8, so 0.8.0 stays locked
# beside 0.9.4 in the two firmware lockfiles. Those crates use only `Vec`,
# `String`, `Deque` and `BinaryHeap`. None of them uses the affected `IndexMap`,
# `HistoryBuf`, `SortedLinkedList`, `spsc`, the pools,
# `Deque::make_contiguous` or `Deque::try_from`. They do use `Deque::clear` and
# `Vec`'s by-value iterator, which RUSTSEC-2026-0577 makes unsound only while
# unwinding through an element's panicking `Drop`. The firmware target aborts
# on panic. In the host simulator, only mipidsi's `Vec` uses 0.8.0, and it
# holds colour values, which have no `Drop`.
HEAPLESS_0_8_ADVISORIES = frozenset({
    "RUSTSEC-2026-0574", "RUSTSEC-2026-0575", "RUSTSEC-2026-0576",
    "RUSTSEC-2026-0577", "RUSTSEC-2026-0578", "RUSTSEC-2026-0579",
    "RUSTSEC-2026-0580", "RUSTSEC-2026-0581", "RUSTSEC-2026-0582",
})
ACCEPTED_BY_LOCKFILE = {
    "apps/kaskold-hardware/Cargo.lock": HEAPLESS_0_8_ADVISORIES,
    "qa/firmware-sim/Cargo.lock": HEAPLESS_0_8_ADVISORIES,
}


def accepted_for(relative_lockfile: str) -> list[str]:
    """Advisories accepted everywhere, plus those accepted for this lockfile only."""
    scoped = ACCEPTED_BY_LOCKFILE.get(relative_lockfile, frozenset())
    return sorted(ACCEPTED_ADVISORIES | scoped)


def installed_version() -> str:
    if shutil.which("cargo-audit") is None:
        return ""
    output = subprocess.run(
        ["cargo", "audit", "--version"], check=True, capture_output=True, text=True
    ).stdout
    return output.split()[-1]


def ensure_pinned(toolchains: dict[str, str]) -> bool:
    """Install the pinned cargo-audit when it is missing or another version."""
    required = toolchains["KASKOLD_CARGO_AUDIT_VERSION"]
    if installed_version() != required:
        subprocess.run(
            ["rustup", "run", toolchains["KASKOLD_STABLE_RUST"], "cargo", "install",
             "cargo-audit", "--version", required, "--locked", "--force"],
            cwd=ROOT, check=False,
        )
    found = installed_version()
    if found != required:
        print(f"FAIL: cargo-audit {required} is required (found {found or 'none'}) "
              "and installing it failed")
    return found == required


def lockfiles() -> list[Path]:
    listed = subprocess.run(
        ["git", "ls-files", "*Cargo.lock"],
        cwd=ROOT, check=True, capture_output=True, text=True,
    ).stdout.split()
    return [ROOT / path for path in listed]


def audit(lockfile: Path) -> bool:
    relative = lockfile.relative_to(ROOT).as_posix()
    command = ["cargo", "audit", "--deny", "warnings", "--file", str(lockfile)]
    for advisory in accepted_for(relative):
        command += ["--ignore", advisory]
    print(f"==> cargo audit {relative}", flush=True)
    return subprocess.run(command, cwd=ROOT).returncode == 0


def main() -> int:
    if not ensure_pinned(load_toolchains()):
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
