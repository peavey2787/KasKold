"""Both web apps ship the unmodified jsQR 1.4.0 npm release."""
from __future__ import annotations

from hashlib import sha256
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[3]
JSQR_1_4_0_SHA256 = "bc40c8a15196236b2314db0856f72ca0b49980cd5413b8c852a7349f5fee0859"
COPIES = (
    "apps/kaskold-companion-web/web/lib/jsQR.js",
    "apps/kaskold-vault-web/web/lib/vault_jsQR.js",
)


class VendoredJsQrTests(unittest.TestCase):
    def test_each_copy_is_the_pinned_release(self) -> None:
        for relative in COPIES:
            with self.subTest(relative):
                digest = sha256((ROOT / relative).read_bytes()).hexdigest()
                self.assertEqual(digest, JSQR_1_4_0_SHA256)


if __name__ == "__main__":
    unittest.main()
