#!/usr/bin/env python3
"""Regression tests for Rust use-tree expansion and stale wallet-module detection."""

from __future__ import annotations

from pathlib import Path, PureWindowsPath
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))

from architecture.core.common import (  # noqa: E402
    has_exact_child,
    is_generated_tree,
    relative_posix,
    rust_use_paths,
)
from architecture.firmware.firmware_state import (  # noqa: E402
    _check_wallet_domain_boundary,
)
from architecture.firmware.guards.source_integrity import (  # noqa: E402
    _check_firmware_include_bytes_paths,
)
from architecture.firmware.subsystems import firmware_boot  # noqa: E402
from architecture.tooling import workspace_delivery  # noqa: E402


class RustUsePathTests(unittest.TestCase):
    def test_expands_direct_and_grouped_imports(self) -> None:
        source = """
use crate::ui::display;
use crate::ui::{seed_manager, setup_wizard};
use crate::{runtime::data::AppData, ui::{display as screen, seed_manager::MAX_SLOTS}};
"""
        self.assertEqual(
            rust_use_paths(source),
            {
                "crate::ui::display",
                "crate::ui::seed_manager",
                "crate::ui::setup_wizard",
                "crate::runtime::data::AppData",
                "crate::ui::seed_manager::MAX_SLOTS",
            },
        )

    def test_ignores_import_text_inside_comments_and_literals(self) -> None:
        source = '''
// use crate::ui::seed_manager;
const SAMPLE: &str = "use crate::ui::{seed_manager, setup_wizard};";
use crate::wallet::{mnemonic, seed_manager::MAX_SLOTS};
'''
        self.assertEqual(
            rust_use_paths(source),
            {
                "crate::wallet::mnemonic",
                "crate::wallet::seed_manager::MAX_SLOTS",
            },
        )


class GeneratedTreeExclusionTests(unittest.TestCase):
    def test_target_trees_are_not_treated_as_authored_source(self) -> None:
        root = Path("/repo")
        self.assertTrue(is_generated_tree(root / "target/debug/build/out/tests.rs", root))
        self.assertTrue(is_generated_tree(root / "target/qa/state/reproducible-build-inputs/root-home/.cargo/registry/src/dependency/src/lib.rs", root))
        self.assertTrue(
            is_generated_tree(
                root / "apps/kaskold-companion-web/target/wasm32/release/build/out/tests.rs",
                root,
            )
        )
        self.assertFalse(is_generated_tree(root / "crates/shared-signer/src/lib.rs", root))
        self.assertTrue(
            is_generated_tree(root / "target/kaskold-companion-web/site/pkg/companion_web.js", root)
        )
        self.assertTrue(
            is_generated_tree(root / "apps/kaskold-companion-web/web/pkg/companion_web.js", root)
        )
        self.assertTrue(
            is_generated_tree(root / "apps/kaskold-companion-android/.kotlin/sessions/generated.js", root)
        )



class CrossPlatformArchitecturePathTests(unittest.TestCase):
    def test_repository_relative_paths_are_posix_on_windows(self) -> None:
        root = PureWindowsPath(r"C:\repo\KasKold")
        module = root / "apps" / "kaskold-companion-web" / "web" / "js" / "app" / "bootstrap.js"
        self.assertEqual(relative_posix(module, root), "apps/kaskold-companion-web/web/js/app/bootstrap.js")

    def test_exact_child_spelling_does_not_alias_case_on_case_insensitive_hosts(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "Makefile").write_text("help:\n\t@true\n", encoding="utf-8")
            self.assertTrue(has_exact_child(root, "Makefile"))
            self.assertFalse(has_exact_child(root, "makefile"))

    def test_windows_does_not_require_posix_execute_mode_bits(self) -> None:
        with patch.object(workspace_delivery.os, "name", "nt"):
            self.assertFalse(workspace_delivery._requires_posix_executable_mode())
        with patch.object(workspace_delivery.os, "name", "posix"):
            self.assertTrue(workspace_delivery._requires_posix_executable_mode())

    def test_web_inventory_checks_use_canonical_repository_paths(self) -> None:
        for relative in (
            "qa/checks/architecture/web/web_css.py",
            "qa/checks/architecture/web/web_js.py",
        ):
            source = (ROOT / relative).read_text(encoding="utf-8")
            self.assertIn("relative_posix", source, relative)
            self.assertNotIn("str(path.relative_to(", source, relative)


class WalletDomainBoundaryTests(unittest.TestCase):
    def _check_source(self, source: str) -> list[str]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_root = root / "apps/kaskold-hardware/src"
            (source_root / "ui").mkdir(parents=True)
            (source_root / "probe.rs").write_text(source)
            return _check_wallet_domain_boundary(root)

    def test_rejects_grouped_stale_wallet_import(self) -> None:
        errors = self._check_source(
            "use crate::{runtime::data::AppData, ui::{display, seed_manager::MAX_SLOTS}};"
        )
        self.assertTrue(any("ui::seed_manager::MAX_SLOTS" in error for error in errors))

    def test_rejects_direct_stale_wallet_reference(self) -> None:
        errors = self._check_source(
            "fn probe() { crate::ui::setup_wizard::generate_from_entropy(); }"
        )
        self.assertTrue(any("ui::setup_wizard::generate_from_entropy" in error for error in errors))

    def test_accepts_current_wallet_domain_imports(self) -> None:
        errors = self._check_source(
            "use crate::{ui::display, wallet::{mnemonic, seed_manager::MAX_SLOTS}};"
        )
        self.assertEqual(errors, [])


class FirmwareIncludeBytesPathTests(unittest.TestCase):
    def test_rejects_missing_literal_firmware_asset_and_accepts_present_asset(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "apps/kaskold-hardware/src/ui/home.rs"
            source.parent.mkdir(parents=True)
            source.write_text(
                'static ICON: &[u8] = include_bytes!("../../assets/icon.raw");\n',
                encoding="utf-8",
            )
            errors = _check_firmware_include_bytes_paths(root)
            self.assertTrue(any("icon.raw" in error for error in errors), errors)

            asset = root / "apps/kaskold-hardware/assets/icon.raw"
            asset.parent.mkdir(parents=True)
            asset.write_bytes(b"asset")
            self.assertEqual(_check_firmware_include_bytes_paths(root), [])


class FirmwareBootArchitectureTests(unittest.TestCase):
    def test_hardware_root_contract_is_part_of_the_live_boot_check(self) -> None:
        sentinel = "hardware-root-contract-ran"
        with tempfile.TemporaryDirectory() as temporary, patch.object(
            firmware_boot, "_check_hardware_roots", return_value=[sentinel]
        ) as hardware_roots:
            errors = firmware_boot.check(Path(temporary))
        hardware_roots.assert_called_once()
        self.assertIn(sentinel, errors)


if __name__ == "__main__":
    unittest.main()
