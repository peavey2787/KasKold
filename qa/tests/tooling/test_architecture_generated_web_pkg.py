#!/usr/bin/env python3
"""Regression coverage for generated Companion/Vault wasm-bindgen output."""

from __future__ import annotations

from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))

from architecture.core.source_quality import production_module_sources  # noqa: E402


class GeneratedWebPkgTests(unittest.TestCase):
    def test_generated_wasm_pkg_is_not_authored_production_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            generated = root / "target/kaskold-companion-web/site/pkg/companion_web.js"
            local_generated = root / "apps/kaskold-companion-web/web/pkg/companion_web.js"
            vault_generated = root / "apps/kaskold-vault-web/web/pkg/vault_web.js"
            authored = root / "apps/kaskold-companion-web/web/js/app.js"
            for path in (generated, local_generated, vault_generated):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("\n".join("generated();" for _ in range(1000)))
            authored.parent.mkdir(parents=True)
            authored.write_text("export function app() {}\n")

            sources = production_module_sources(root)

            self.assertIn(authored, sources)
            self.assertNotIn(generated, sources)
            self.assertNotIn(local_generated, sources)
            self.assertNotIn(vault_generated, sources)

    def test_android_gradle_and_generated_web_assets_are_not_authored_production_source(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            generated_paths = [
                root / "apps/kaskold-companion-android/.kotlin/sessions/generated.js",
                root / "apps/kaskold-companion-android/app/build/intermediates/assets/debug/mergeDebugAssets/companion/pkg/companion_web.js",
                root / "apps/kaskold-companion-android/app/build/generated/kaskold-runtime/web/pkg/companion_web.js",
            ]
            authored = root / "apps/kaskold-companion-android/app/src/main/java/example/source.js"
            for generated in generated_paths:
                generated.parent.mkdir(parents=True, exist_ok=True)
                generated.write_text("\n".join("generated();" for _ in range(1000)))
            authored.parent.mkdir(parents=True, exist_ok=True)
            authored.write_text("export function authored() {}\n")

            sources = production_module_sources(root)

            self.assertIn(authored, sources)
            for generated in generated_paths:
                self.assertNotIn(generated, sources)


if __name__ == "__main__":
    unittest.main()
