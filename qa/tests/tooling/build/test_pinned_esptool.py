#!/usr/bin/env python3
"""The release-pinned esptool is the only esptool the signing and evidence tools use."""

from __future__ import annotations

import importlib.util
from pathlib import Path
import re
import sys
import tempfile
import types
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[4]
FIRMWARE_TOOLS = ROOT / "tools/build/firmware"
SPEC = importlib.util.spec_from_file_location("pinned_esptool", FIRMWARE_TOOLS / "pinned_esptool.py")
assert SPEC and SPEC.loader
PINNED = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = PINNED
SPEC.loader.exec_module(PINNED)


class PinnedEsptoolTests(unittest.TestCase):
    def test_the_release_policy_pins_an_exact_version(self) -> None:
        self.assertRegex(PINNED.pinned_version(), r"^\d+\.\d+\.\d+$")

    def test_a_policy_without_a_pin_is_refused(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            policy = Path(temporary) / "release-policy.env"
            policy.write_text("KASKOLD_UPDATE_SEQUENCE=1\nKASKOLD_ESPTOOL_VERSION=\n", encoding="utf-8")
            with self.assertRaises(SystemExit):
                PINNED.pinned_version(policy)

    def test_each_version_has_its_own_environment_in_qa_state(self) -> None:
        self.assertEqual(
            PINNED.environment("5.3.1", Path("/repo")),
            Path("/repo/target/qa/state/tools/esptool-5.3.1"),
        )

    def test_commands_resolve_inside_the_environment_for_each_platform(self) -> None:
        venv = Path("/venv")
        with mock.patch.object(PINNED.os, "name", "nt"):
            self.assertEqual(PINNED.executable(venv, "espsecure"), venv / "Scripts" / "espsecure.exe")
        with mock.patch.object(PINNED.os, "name", "posix"):
            self.assertEqual(PINNED.executable(venv, "espsecure"), venv / "bin" / "espsecure")

    def test_an_unknown_command_is_refused(self) -> None:
        with self.assertRaises(ValueError):
            PINNED.ensure("idf.py")

    def test_bootstraps_only_when_the_pinned_esptool_is_absent_and_always_verifies(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            venv = Path(temporary) / "esptool-9.9.9"
            with (
                mock.patch.object(PINNED, "pinned_version", return_value="9.9.9"),
                mock.patch.object(PINNED, "environment", return_value=venv),
                mock.patch.object(PINNED, "_bootstrap") as bootstrap,
                mock.patch.object(PINNED, "_verify") as verify,
            ):
                self.assertEqual(PINNED.ensure("espsecure"), PINNED.executable(venv, "espsecure"))
                bootstrap.assert_called_once_with(venv, "9.9.9")
                esptool = PINNED.executable(venv, "esptool")
                esptool.parent.mkdir(parents=True)
                esptool.write_bytes(b"")
                PINNED.ensure("espsecure")
                bootstrap.assert_called_once()
                self.assertEqual(verify.call_count, 2)

    def test_a_reported_version_other_than_the_pin_is_refused(self) -> None:
        result = mock.Mock(returncode=0, stdout="esptool v4.8.1\n", stderr="")
        with mock.patch.object(PINNED.subprocess, "run", return_value=result):
            with self.assertRaises(SystemExit):
                PINNED._verify(Path("/venv"), "5.3.1")

    def test_the_app_signer_resolves_its_helpers_from_the_repository_root(self) -> None:
        path = FIRMWARE_TOOLS / "secure_bootloader/m5stack/sign_app.py"
        spec = importlib.util.spec_from_file_location("sign_app_root", path)
        assert spec and spec.loader
        signer = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(signer)
        self.assertEqual(signer.ROOT, ROOT)
        for helper in ("tools/build/firmware/secure_pad_v2.py", "tools/build/firmware/verify_image_hash.py"):
            self.assertTrue((signer.ROOT / helper).is_file(), helper)

    @staticmethod
    def _secure_padder():
        sys.path.insert(0, str(FIRMWARE_TOOLS))
        try:
            spec = importlib.util.spec_from_file_location("secure_pad_v2", FIRMWARE_TOOLS / "secure_pad_v2.py")
            assert spec and spec.loader
            padder = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(padder)
        finally:
            sys.path.remove(str(FIRMWARE_TOOLS))
        return padder

    def test_the_secure_padder_requires_the_policy_pinned_esptool(self) -> None:
        padder = self._secure_padder()
        stale = mock.Mock(__version__="0.0.0")
        with mock.patch.object(padder.importlib, "import_module", return_value=stale):
            with self.assertRaisesRegex(SystemExit, re.escape(f"esptool {PINNED.pinned_version()} is required")):
                padder.secure_pad(Path("unused.bin"), Path("unused-padded.bin"))

    def test_the_secure_padder_names_image_segments_before_esptool_pads_them(self) -> None:
        padder = self._secure_padder()
        segments = [types.SimpleNamespace(addr=0x42000020), types.SimpleNamespace(addr=0x3FC88000)]

        def save(path: str) -> None:
            self.assertEqual([segment.name for segment in segments], ["0x42000020", "0x3fc88000"])
            Path(path).write_bytes(b"\0" * padder.MMU_PAGE_SIZE)

        image = types.SimpleNamespace(append_digest=True, segments=segments, save=save)
        esptool = types.SimpleNamespace(__version__=PINNED.pinned_version())
        bin_image = types.SimpleNamespace(LoadFirmwareImage=lambda chip, data: image)
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / "app.bin"
            source.write_bytes(b"image")
            with mock.patch.object(padder.importlib, "import_module", side_effect=[esptool, bin_image]):
                padder.secure_pad(source, Path(temporary) / "padded.bin")
        self.assertEqual(image.secure_pad, "2")

    def test_the_app_signer_pads_on_the_pinned_interpreter(self) -> None:
        source = (FIRMWARE_TOOLS / "secure_bootloader/m5stack/sign_app.py").read_text(encoding="utf-8")
        self.assertIn('pinned_python = str(pinned_esptool.ensure("python"))', source)
        self.assertIn('run([pinned_python, str(ROOT / "tools/build/firmware/secure_pad_v2.py")', source)

    def test_signing_and_evidence_tools_never_take_esptool_from_path(self) -> None:
        for relative in (
            "tools/build/firmware/secure_bootloader/m5stack/sign_app.py",
            "tools/build/firmware/owner_authority.py",
            "qa/windows/run-m5stack-security-hil.ps1",
            "qa/linux/run-m5stack-security-hil.sh",
        ):
            source = (ROOT / relative).read_text(encoding="utf-8")
            self.assertIn("pinned_esptool", source, relative)
            self.assertNotRegex(source, re.compile(r"which\(\"espsecure\"\)|esptool==|-m venv"), relative)


if __name__ == "__main__":
    unittest.main()
