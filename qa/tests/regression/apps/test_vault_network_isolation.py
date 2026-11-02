from __future__ import annotations

import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]


class VaultNetworkIsolationTests(unittest.TestCase):
    def test_vault_runtime_has_no_network_dependencies(self) -> None:
        manifest = tomllib.loads((ROOT / "crates/vault-runtime/Cargo.toml").read_text())
        dependencies = set(manifest.get("dependencies", {}))
        for forbidden in (
            "online-watcher",
            "reqwest",
            "web-sys",
            "tokio",
            "tungstenite",
            "tokio-tungstenite",
            "wasm-bindgen",
        ):
            self.assertNotIn(forbidden, dependencies)


if __name__ == "__main__":
    unittest.main()
