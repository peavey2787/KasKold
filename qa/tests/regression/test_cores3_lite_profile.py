from __future__ import annotations

import tomllib
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


class CoreS3LiteProfileTests(unittest.TestCase):
    def test_lite_reuses_m5stack_adapter(self) -> None:
        manifest = tomllib.loads((ROOT / "apps/kaskold-hardware/Cargo.toml").read_text())
        features = manifest["features"]
        self.assertEqual(features["default"], ["m5stack"])
        self.assertIn("m5stack", features["m5stack-lite"])

    def test_profile_does_not_claim_lite_is_qualified(self) -> None:
        profile = (ROOT / "apps/kaskold-hardware/src/hw/m5stack/profile.rs").read_text()
        self.assertIn('model: "M5Stack CoreS3 Lite"', profile)
        self.assertIn("hardware_qualified: false", profile)
        self.assertIn("hardware qualification pending", profile)


if __name__ == "__main__":
    unittest.main()
