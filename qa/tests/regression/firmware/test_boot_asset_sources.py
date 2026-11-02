#!/usr/bin/env python3
"""Editable PNG sources round-trip to the firmware's raw boot assets."""
import pathlib
import subprocess
import sys
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[4]


class BootAssetSourceTests(unittest.TestCase):
    def test_teal_and_white_assets_have_editable_png_sources_and_exact_roundtrip(self):
        converter = ROOT / "tools/kaskold_image.py"
        assets = []
        for name in ("kascoin_90", "kascoin_teal_90"):
            raw = ROOT / f"apps/kaskold-hardware/assets/{name}.raw"
            png = ROOT / f"apps/kaskold-hardware/assets/source/{name}.png"
            self.assertEqual(raw.stat().st_size, 90 * 90 * 2)
            header = png.read_bytes()[:24]
            self.assertEqual(header[:8], b"\x89PNG\r\n\x1a\n")
            self.assertEqual(int.from_bytes(header[16:20], "big"), 90)
            self.assertEqual(int.from_bytes(header[20:24], "big"), 90)
            assets.append((raw, png))

        try:
            import PIL  # noqa: F401
        except ImportError:
            self.skipTest("Pillow is optional and only required for hardware-asset conversion")

        for raw, _png in assets:
            with tempfile.TemporaryDirectory() as td:
                decoded = pathlib.Path(td) / "decoded.png"
                encoded = pathlib.Path(td) / "encoded.raw"
                subprocess.run(
                    [sys.executable, str(converter), "decode", str(raw), str(decoded), "90", "90"],
                    check=True,
                )
                subprocess.run(
                    [sys.executable, str(converter), "encode", str(decoded), str(encoded), "90", "90"],
                    check=True,
                )
                self.assertEqual(raw.read_bytes(), encoded.read_bytes())


if __name__ == "__main__":
    unittest.main()
