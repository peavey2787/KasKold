#!/usr/bin/env python3
"""Check rustfmt compliance of every Rust crate in the repository.

The root workspace is checked with ``cargo fmt --all``. Each standalone
workspace the root manifest excludes is checked by its own package only, so a
path dependency is never formatted through a consumer. Path-only packages that
belong to no workspace are checked by passing their crate roots to rustfmt.
"""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))

from toolchains import load_toolchains  # noqa: E402

PRUNED_DIRECTORIES = {"target", "node_modules", "build"}


def _manifest(directory: Path) -> dict:
    return tomllib.loads((directory / "Cargo.toml").read_text(encoding="utf-8"))


def discover_manifest_dirs(root: Path) -> list[Path]:
    """Every directory below ``root`` holding a Cargo.toml, build output pruned."""
    found: list[Path] = []
    for current, directories, files in os.walk(root):
        directories[:] = sorted(
            name for name in directories if not name.startswith(".") and name not in PRUNED_DIRECTORIES
        )
        if "Cargo.toml" in files:
            found.append(Path(current))
    return found


def crate_roots(directory: Path, manifest: dict) -> list[Path]:
    """Library and binary root files rustfmt follows to reach a package's modules."""
    library = manifest.get("lib", {}).get("path", "src/lib.rs")
    candidates = [library, "src/main.rs", *(item["path"] for item in manifest.get("bin", []) if "path" in item)]
    return sorted({directory / candidate for candidate in candidates if (directory / candidate).is_file()})


def standalone_command(root: Path, directory: Path, toolchain: str) -> list[str]:
    relative = directory.relative_to(root).as_posix()
    package = _manifest(directory)["package"]["name"]
    return [
        "rustup", "run", toolchain, "cargo", "fmt",
        "--manifest-path", f"{relative}/Cargo.toml", "--package", package, "--", "--check",
    ]


def path_only_command(root: Path, directory: Path, toolchain: str) -> list[str]:
    manifest = _manifest(directory)
    relative = directory.relative_to(root).as_posix()
    if "workspace" in manifest:
        raise ValueError(f"standalone workspace {relative} must be listed in the root workspace exclude")
    roots = crate_roots(directory, manifest)
    if not roots:
        raise ValueError(f"path-only package {relative} has no library or binary crate root")
    edition = manifest["package"].get("edition", "2015")
    return [
        "rustup", "run", toolchain, "rustfmt", "--check", "--edition", edition,
        *(path.relative_to(root).as_posix() for path in roots),
    ]


def format_commands(root: Path, toolchain: str) -> list[list[str]]:
    """Every rustfmt check needed to cover each Rust crate below ``root``."""
    workspace = _manifest(root)["workspace"]
    members = {root / member for member in workspace.get("members", [])}
    standalone = [root / entry for entry in workspace.get("exclude", [])]
    commands = [["rustup", "run", toolchain, "cargo", "fmt", "--all", "--", "--check"]]
    commands.extend(standalone_command(root, directory, toolchain) for directory in standalone)
    covered = {root, *members, *standalone}
    commands.extend(
        path_only_command(root, directory, toolchain)
        for directory in discover_manifest_dirs(root)
        if directory not in covered
    )
    return commands


def main() -> int:
    toolchain = load_toolchains()["KASKOLD_STABLE_RUST"]
    failed: list[str] = []
    for command in format_commands(ROOT, toolchain):
        print("+ " + subprocess.list2cmdline(command), flush=True)
        if subprocess.run(command, cwd=ROOT, check=False).returncode != 0:
            failed.append(subprocess.list2cmdline(command))
    for command in failed:
        print(f"FAIL: rustfmt drift: {command}")
    if failed:
        return 1
    print("PASS: every Rust crate matches pinned stable rustfmt")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
