from __future__ import annotations

import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]


class CoreS3LiteProfileTests(unittest.TestCase):
    def test_lite_reuses_m5stack_adapter(self) -> None:
        manifest = tomllib.loads((ROOT / "apps/kaskold-hardware/Cargo.toml").read_text())
        features = manifest["features"]
        self.assertEqual(features["default"], ["m5stack"])
        self.assertIn("m5stack", features["m5stack-lite"])


if __name__ == "__main__":
    unittest.main()
