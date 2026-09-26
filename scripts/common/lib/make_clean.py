#!/usr/bin/env python3
"""Workspace cleanup kept out of the Make task dispatcher."""
from __future__ import annotations

import shutil
from pathlib import Path
from typing import Callable, Sequence

Run = Callable[[Sequence[str]], int]


def clean_workspace(root: Path, run: Run) -> int:
    manifests = (
        "Cargo.toml",
        "apps/kaskold-hardware/Cargo.toml",
        "apps/kaskold-companion-web/Cargo.toml",
        "apps/kaskold-vault-web/Cargo.toml",
        "tools/Cargo.toml",
        "qa/Cargo.toml",
    )
    for manifest in manifests:
        rc = run(["cargo", "clean", "--manifest-path", str(root / manifest)])
        if rc:
            return rc
    paths = (
        "apps/kaskold-vault-web/web/pkg",
        "target/kaskold-vault-web",
        "target/kaskold-vault-web-wasm",
        "apps/kaskold-companion-ios/.build",
        "apps/kaskold-companion-ios/.swiftpm",
        "target/qa/ios-crap",
        "apps/kaskold-companion-android/.gradle",
        "apps/kaskold-companion-android/build",
        "apps/kaskold-companion-android/app/build",
        "apps/kaskold-companion-android/app/src/main/assets/web/pkg",
        "apps/kaskold-companion-android/app/src/main/assets/web/shared-js",
        "target/qa/android-crap",
        "qa/target",
        "qa/fuzz/target",
    )
    for relative in paths:
        path = root / relative
        if path.is_dir():
            shutil.rmtree(path)
        elif path.exists():
            path.unlink()
    return 0
