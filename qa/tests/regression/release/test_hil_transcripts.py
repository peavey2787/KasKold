#!/usr/bin/env python3
"""Durable HIL build logs and UART transcripts."""

from __future__ import annotations

import io
import os
import subprocess
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[4]
FIRMWARE_CHECKS = ROOT / "qa" / "checks" / "firmware"
SECURITY_CHECKS = ROOT / "qa" / "checks" / "security"
for path in (FIRMWARE_CHECKS, SECURITY_CHECKS):
    if str(path) not in sys.path:
        sys.path.insert(0, str(path))

import run_hardware_tests  # noqa: E402
import run_workflow_tests  # noqa: E402


class HilTranscriptTests(unittest.TestCase):

    def test_workflow_hil_build_log_is_tee_persisted(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            transient = root / "transient-build.log"
            durable = root / "run" / "build.log"
            with mock.patch.object(run_workflow_tests, "BUILD_LOG", transient):
                code, warnings = run_workflow_tests.run_logged_build(
                    [sys.executable, "-c", "print('compiler-line')"],
                    os.environ.copy(),
                    durable,
                )
            self.assertEqual(code, 0)
            self.assertEqual(warnings, [])
            self.assertEqual(transient.read_bytes(), durable.read_bytes())
            self.assertIn("compiler-line", durable.read_text(encoding="utf-8"))

    def test_uart_supervisor_can_persist_exact_monitor_transcript(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            image = root / "firmware"
            image.write_bytes(b"ELF")
            transcript = io.StringIO()
            real_popen = subprocess.Popen

            def portable_popen(command, *args, **kwargs):
                if command and command[0] == "espflash":
                    command = [
                        sys.executable,
                        "-c",
                        "print('Flashing has completed!'); print('UART-LINE'); "
                        "print('KASKOLD_HARDWARE_TESTS: PASS')",
                    ]
                return real_popen(command, *args, **kwargs)

            with mock.patch.object(
                run_hardware_tests.subprocess, "Popen", side_effect=portable_popen
            ):
                result = run_hardware_tests.flash_and_monitor(
                    "m5stack-lite", image, None, 3, uart_log=transcript, connected_transport=False
                )
            self.assertEqual(result, 0)
            self.assertIn("Flashing has completed!", transcript.getvalue())
            self.assertIn("UART-LINE", transcript.getvalue())
            self.assertIn(run_hardware_tests.PASS_MARKER, transcript.getvalue())


if __name__ == "__main__":
    unittest.main()
