from __future__ import annotations

import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]


class VaultCustodyBoundaryTests(unittest.TestCase):

    def test_watch_only_and_protocol_crates_cannot_depend_on_hot_wallet(self) -> None:
        for relative in (
            "crates/online-watcher/Cargo.toml",
            "crates/kaskold-sdk/Cargo.toml",
            "crates/kaskold-protocol/Cargo.toml",
            "crates/shared-signer/Cargo.toml",
        ):
            manifest = tomllib.loads((ROOT / relative).read_text(encoding="utf-8"))
            self.assertNotIn("hot-wallet", manifest.get("dependencies", {}), relative)


if __name__ == "__main__":
    unittest.main()
