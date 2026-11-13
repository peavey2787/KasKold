#!/usr/bin/env python3
"""Hash-converging firmware builds run the platform's builder and leave the checkout as committed."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[4]
FIRMWARE_TOOLS = ROOT / "tools/build/firmware"
SPEC = importlib.util.spec_from_file_location("hash_builder", FIRMWARE_TOOLS / "hash_builder.py")
assert SPEC and SPEC.loader
HASH_BUILDER = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = HASH_BUILDER
SPEC.loader.exec_module(HASH_BUILDER)

COMMITTED = b"pub static EXPECTED_FIRMWARE_HASH: [u8; 32] = [0; 32];\n"
GENERATED = b"pub static EXPECTED_FIRMWARE_HASH: [u8; 32] = [1; 32];\n"


class PreservedHashSourceTests(unittest.TestCase):
    def test_the_tracked_hash_source_is_the_firmware_crate_file(self) -> None:
        self.assertEqual(HASH_BUILDER.HASH_SOURCE, ROOT / "apps/kaskold-hardware/src/firmware_hash.rs")
        self.assertTrue(HASH_BUILDER.HASH_SOURCE.is_file())

    def test_the_generated_hash_is_readable_inside_and_reverted_after(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "firmware_hash.rs"
            source.write_bytes(COMMITTED)
            with HASH_BUILDER.preserved_hash_source(source):
                source.write_bytes(GENERATED)
                self.assertEqual(source.read_bytes(), GENERATED)
            self.assertEqual(source.read_bytes(), COMMITTED)

    def test_a_failed_build_still_reverts_the_hash(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "firmware_hash.rs"
            source.write_bytes(COMMITTED)
            with self.assertRaises(RuntimeError):
                with HASH_BUILDER.preserved_hash_source(source):
                    source.write_bytes(GENERATED)
                    raise RuntimeError("build failed")
            self.assertEqual(source.read_bytes(), COMMITTED)

    def test_an_untouched_hash_source_is_not_rewritten(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "firmware_hash.rs"
            source.write_bytes(COMMITTED)
            with mock.patch.object(Path, "write_bytes") as write:
                with HASH_BUILDER.preserved_hash_source(source):
                    pass
            write.assert_not_called()


class HashBuilderCommandTests(unittest.TestCase):
    def test_posix_runs_the_shell_builder_with_the_board_first(self) -> None:
        command = HASH_BUILDER.hash_builder_command("m5stack", "m5stack-hil", ["--features", "x"], windows=False)
        self.assertEqual(
            command,
            [str(FIRMWARE_TOOLS / "build_with_hash.sh"), "--board", "m5stack", "m5stack-hil", "--features", "x"],
        )

    def test_windows_runs_the_powershell_twin_with_the_label_first(self) -> None:
        with mock.patch.object(HASH_BUILDER.shutil, "which", side_effect=lambda name: "C:/ps/pwsh.exe" if name == "pwsh.exe" else None):
            command = HASH_BUILDER.hash_builder_command("m5stack", "m5stack-hil", ["--features", "x"], windows=True)
        self.assertEqual(command[:6], ["C:/ps/pwsh.exe", "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(FIRMWARE_TOOLS / "build_with_hash.ps1")])
        self.assertEqual(command[6:], ["m5stack-hil", "-Board", "m5stack", "--features", "x"])

    def test_windows_without_powershell_is_refused(self) -> None:
        with mock.patch.object(HASH_BUILDER.shutil, "which", return_value=None):
            with self.assertRaisesRegex(RuntimeError, "PowerShell is required"):
                HASH_BUILDER.hash_builder_command("m5stack", "m5stack-hil", [], windows=True)


if __name__ == "__main__":
    unittest.main()
