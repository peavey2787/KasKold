#!/usr/bin/env python3
"""The release policy's pinned esptool, bootstrapped once into repository QA state.

Every caller that signs, digests or inspects a CoreS3 image resolves esptool,
espsecure or espefuse here, so one pinned version serves them all and nothing
depends on whatever happens to be on PATH.
"""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[3]
POLICY = ROOT / "apps" / "kaskold-hardware" / "release-policy.env"
COMMANDS = ("python", "esptool", "espsecure", "espefuse")
MINIMUM_PYTHON = (3, 10)


def pinned_version(policy: Path = POLICY) -> str:
    for line in policy.read_text(encoding="utf-8").splitlines():
        key, _, value = line.strip().partition("=")
        if key == "KASKOLD_ESPTOOL_VERSION" and value.strip():
            return value.strip()
    raise SystemExit(f"KASKOLD_ESPTOOL_VERSION is missing from {policy}")


def environment(version: str, root: Path = ROOT) -> Path:
    return root / "target" / "qa" / "state" / "tools" / f"esptool-{version}"


def executable(venv: Path, name: str) -> Path:
    if os.name == "nt":
        return venv / "Scripts" / f"{name}.exe"
    return venv / "bin" / name


def _bootstrap(venv: Path, version: str) -> None:
    if sys.version_info < MINIMUM_PYTHON:
        raise SystemExit(f"esptool {version} requires Python 3.10 or newer")
    print(f"==> Bootstrapping pinned esptool {version} into repository QA state", file=sys.stderr, flush=True)
    venv.parent.mkdir(parents=True, exist_ok=True)
    subprocess.run([sys.executable, "-m", "venv", str(venv)], check=True)
    subprocess.run(
        [str(executable(venv, "python")), "-m", "pip", "install", "--disable-pip-version-check", f"esptool=={version}"],
        check=True,
        stdout=sys.stderr,
    )


def _verify(venv: Path, version: str) -> None:
    result = subprocess.run(
        [str(executable(venv, "esptool")), "version"], check=False, text=True, capture_output=True
    )
    reported = (result.stdout + result.stderr).strip()
    if result.returncode != 0 or version not in reported:
        raise SystemExit(f"pinned esptool {version} mismatch: {reported}")


def ensure(name: str) -> Path:
    """Return a command from the pinned esptool environment, bootstrapping it when absent."""
    if name not in COMMANDS:
        raise ValueError(f"unknown pinned esptool command: {name}")
    version = pinned_version()
    venv = environment(version)
    if not executable(venv, "esptool").is_file():
        _bootstrap(venv, version)
    _verify(venv, version)
    return executable(venv, name)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=COMMANDS, help="pinned command whose path to print")
    print(ensure(parser.parse_args().command))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
