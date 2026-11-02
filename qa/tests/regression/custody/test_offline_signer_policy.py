#!/usr/bin/env python3
"""Regression tests for portable offline-signer dependency boundaries."""

from __future__ import annotations

import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[4] / "qa/checks"))

from pathlib import Path
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "qa/checks"))

from architecture.protocols.offline_portability import (  # noqa: E402
    portable_crate_boundary_errors,
)


class OfflineSignerBoundaryTests(unittest.TestCase):

    def _check_source(self, source: str) -> list[str]:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source_root = root / "crates/offline-signer/src"
            source_root.mkdir(parents=True)
            (source_root / "probe.rs").write_text(source)
            return portable_crate_boundary_errors(root)

    def test_rejects_firmware_logging_dependency(self) -> None:
        errors = self._check_source("use esp_println::println;")
        self.assertTrue(any("firmware logging" in error for error in errors))

    def test_rejects_firmware_only_silent_feature(self) -> None:
        errors = self._check_source('#[cfg(feature = "silent")] fn probe() {}')
        self.assertTrue(any("silent feature" in error for error in errors))

    def test_accepts_crate_owned_noop_logging(self) -> None:
        errors = self._check_source('crate::log!("derived {}", 7);')
        self.assertEqual(errors, [])


if __name__ == "__main__":
    unittest.main()
