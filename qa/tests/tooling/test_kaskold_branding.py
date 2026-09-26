#!/usr/bin/env python3
"""Regression coverage for the KasKold product-brand audit."""
from __future__ import annotations

from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))

from branding import check_kaskold_brand  # noqa: E402


class KasKoldBrandingTests(unittest.TestCase):
    def test_repository_brand_audit_is_clean(self) -> None:
        self.assertEqual(check_kaskold_brand.check(ROOT), [])

    def test_audit_uses_repository_inventory_without_git_metadata(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "src/lib.rs"
            source.parent.mkdir(parents=True)
            source.write_text('const PRODUCT: &str = "KasKold Hardware";\n')
            inventory = root / "qa/baselines/repository_inventory.txt"
            inventory.parent.mkdir(parents=True)
            inventory.write_text("D\tsrc\nF\tsrc/lib.rs\n", encoding="utf-8")
            self.assertEqual(check_kaskold_brand.check(root), [])

            source.write_text('const PRODUCT: &str = "KasSigner Hardware";\n')
            errors = check_kaskold_brand.check(root)
            self.assertTrue(any("src/lib.rs" in error for error in errors), errors)

    def test_audit_allows_legal_attribution_and_legacy_domain_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self._init_repo(root)
            (root / "CHANGELOG.md").write_text("Derived from KasSigner / InKasWeRust\n")
            source = root / "src/lib.rs"
            source.parent.mkdir()
            source.write_text(
                '// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)\n'
                'const DOMAIN: &[u8] = b"KasSigner Signed Message v1\\0";\n'
            )
            self._track(root)
            self.assertEqual(check_kaskold_brand.check(root), [])

    def test_audit_allows_only_the_owned_compatibility_documentation_copy(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self._init_repo(root)
            allowed = root / "tools/build/docs/generate_kaskold_guides.py"
            allowed.parent.mkdir(parents=True)
            allowed.write_text(
                'TEXT = "Certain compatibility identifiers retain the historical KasSigner spelling."\n'
            )
            self._track(root)
            self.assertEqual(check_kaskold_brand.check(root), [])

            stale = root / "src/ui.rs"
            stale.parent.mkdir(parents=True)
            stale.write_text(
                'const COPY: &str = "UI uses the historical KasSigner spelling";\n'
            )
            self._track(root)
            errors = check_kaskold_brand.check(root)
            self.assertTrue(any("src/ui.rs" in error for error in errors), errors)

    def test_audit_rejects_stale_product_and_retired_board_identity(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self._init_repo(root)
            source = root / "src/lib.rs"
            source.parent.mkdir()
            source.write_text(
                'const PRODUCT: &str = "KasSigner Hardware";\n'
                'const OLD_UI: &str = "Open KasSee";\n'
                'const BOARD: &str = "Waveshare";\n'
                'use signer_firmware_core::security::SigningAuthorization;\n'
                'const OLD_CRATE: &str = "signer-firmware-core";\n'
            )
            self._track(root)
            errors = check_kaskold_brand.check(root)
            joined = "\n".join(errors)
            self.assertIn("unexpected KasSigner/kassigner branding", joined)
            self.assertIn("stale brand identifier 'KasSee'", joined)
            self.assertIn("retired Waveshare board reference", joined)
            self.assertIn("stale brand identifier 'signer_firmware_core'", joined)
            self.assertIn("stale brand identifier 'signer-firmware-core'", joined)

    @staticmethod
    def _init_repo(root: Path) -> None:
        subprocess.run(["git", "init", "-q"], cwd=root, check=True)

    @staticmethod
    def _track(root: Path) -> None:
        subprocess.run(["git", "add", "."], cwd=root, check=True)


if __name__ == "__main__":
    unittest.main()
