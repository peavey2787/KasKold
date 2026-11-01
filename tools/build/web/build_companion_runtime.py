#!/usr/bin/env python3
"""Companion Web/WASM runtime. All platform wrappers (Web, Android, iOS) invoke this entry point."""
from __future__ import annotations

from pathlib import Path
import sys

WEB_BUILD_DIR = Path(__file__).resolve().parent
if str(WEB_BUILD_DIR) not in sys.path:
    sys.path.insert(0, str(WEB_BUILD_DIR))
from wasm_runtime import ROOT, WebRuntime, main

RUNTIME = WebRuntime(
    label="KasKold Companion Web",
    app=ROOT / "apps/kaskold-companion-web",
    bindgen_stem="companion_web",
    site_env="KASKOLD_KASKOLD_COMPANION_WEB_SITE",
    site_default=ROOT / "target/kaskold-companion-web/site",
    target_dir=ROOT / "target/kaskold-companion-web-wasm",
    authored_generators=(
        ROOT / "tools/build/web/build_web_index.py",
        ROOT / "tools/build/web/build_app_css.py",
    ),
)

if __name__ == "__main__":
    raise SystemExit(main(RUNTIME))
