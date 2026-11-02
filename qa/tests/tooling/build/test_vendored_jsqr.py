"""Both web apps ship one copy of the unmodified jsQR 1.4.0 npm release."""
from __future__ import annotations

from hashlib import sha256
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[4]
JSQR_1_4_0_SHA256 = "bc40c8a15196236b2314db0856f72ca0b49980cd5413b8c852a7349f5fee0859"
VENDORED = ROOT / "apps/web-common/lib/jsQR.js"


class VendoredJsQrTests(unittest.TestCase):
    def test_the_shared_copy_is_the_pinned_release(self) -> None:
        self.assertEqual(sha256(VENDORED.read_bytes()).hexdigest(), JSQR_1_4_0_SHA256)


if __name__ == "__main__":
    unittest.main()
