#!/usr/bin/env python3
"""Standalone, network-free Vault Web signer runtime."""
from __future__ import annotations

from pathlib import Path
import sys

WEB_BUILD_DIR = Path(__file__).resolve().parent
if str(WEB_BUILD_DIR) not in sys.path:
    sys.path.insert(0, str(WEB_BUILD_DIR))
from wasm_runtime import ROOT, WebRuntime, main

RUNTIME = WebRuntime(
    label="KasKold Vault Web",
    app=ROOT / "apps/kaskold-vault-web",
    bindgen_stem="vault_web",
    site_env="KASKOLD_VAULT_WEB_SITE",
    site_default=ROOT / "target/kaskold-vault-web/site",
    target_dir=ROOT / "target/kaskold-vault-web-wasm",
    # Vault and Companion may share an origin, so their cache keys stay distinct.
    jsqr_url="lib/vault_jsQR.js",
)

if __name__ == "__main__":
    raise SystemExit(main(RUNTIME))
