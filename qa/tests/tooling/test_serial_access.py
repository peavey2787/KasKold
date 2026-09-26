#!/usr/bin/env python3
"""Regression coverage for Linux dialout remediation used by firmware flashing."""
from __future__ import annotations

import importlib.util
import io
import sys
from pathlib import Path
import subprocess
import types
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[3]
HELPER = ROOT / "scripts/common/lib/serial_access.py"
SPEC = importlib.util.spec_from_file_location("serial_access", HELPER)
assert SPEC and SPEC.loader
SERIAL = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(SERIAL)

TEST_USERNAME = "qauser"
TEST_HOME = f"/home/{TEST_USERNAME}"


class SerialAccessTests(unittest.TestCase):
    @unittest.skipUnless(sys.platform.startswith("linux"), "dialout remediation is Linux-specific")
    def test_no_protected_serial_device_is_a_noop(self) -> None:
        command = ["espflash", "flash", "firmware"]
        group = types.SimpleNamespace(gr_gid=20, gr_mem=[])
        with (
            mock.patch.object(SERIAL.os, "geteuid", return_value=1000),
            mock.patch.object(Path, "exists", return_value=True),
            mock.patch.object(SERIAL, "_dialout_group", return_value=group),
            mock.patch.object(SERIAL, "_candidate_ports", return_value=[]),
        ):
            self.assertEqual(SERIAL.prepare_serial_command(command), command)

    @unittest.skipUnless(sys.platform.startswith("linux"), "dialout remediation is Linux-specific")
    def test_active_dialout_membership_does_not_wrap_command(self) -> None:
        command = ["espflash", "flash", "firmware"]
        group = types.SimpleNamespace(gr_gid=20, gr_mem=[TEST_USERNAME])
        port = Path("/dev/ttyACM0")
        with (
            mock.patch.object(SERIAL.os, "geteuid", return_value=1000),
            mock.patch.object(Path, "exists", return_value=True),
            mock.patch.object(SERIAL, "_dialout_group", return_value=group),
            mock.patch.object(SERIAL, "_candidate_ports", return_value=[port]),
            mock.patch.object(SERIAL, "_needs_dialout", return_value=True),
            mock.patch.object(SERIAL, "_username", return_value=TEST_USERNAME),
            mock.patch.object(SERIAL, "_persistent_member", return_value=True),
            mock.patch.object(SERIAL, "_active_member", return_value=True),
        ):
            self.assertEqual(SERIAL.prepare_serial_command(command, str(port)), command)

    @unittest.skipUnless(sys.platform.startswith("linux"), "dialout remediation is Linux-specific")
    def test_stale_shell_reexecs_flash_under_dialout_without_sudo(self) -> None:
        command = [f"{TEST_HOME}/.cargo/bin/espflash", "flash", "--port", "/dev/ttyACM0", "firmware path"]
        group = types.SimpleNamespace(gr_gid=20, gr_mem=[TEST_USERNAME])
        port = Path("/dev/ttyACM0")
        with (
            mock.patch.object(SERIAL.os, "geteuid", return_value=1000),
            mock.patch.object(Path, "exists", return_value=True),
            mock.patch.object(SERIAL, "_dialout_group", return_value=group),
            mock.patch.object(SERIAL, "_candidate_ports", return_value=[port]),
            mock.patch.object(SERIAL, "_needs_dialout", return_value=True),
            mock.patch.object(SERIAL, "_username", return_value=TEST_USERNAME),
            mock.patch.object(SERIAL, "_persistent_member", return_value=True),
            mock.patch.object(SERIAL, "_active_member", return_value=False),
            mock.patch.object(SERIAL, "_sudo_add_membership") as add,
            mock.patch.object(SERIAL.shutil, "which", side_effect=lambda name: "/usr/bin/sg" if name == "sg" else None),
        ):
            wrapped = SERIAL.prepare_serial_command(command, str(port))
        add.assert_not_called()
        self.assertEqual(wrapped[:3], ["/usr/bin/sg", "dialout", "-c"])
        self.assertIn("espflash", wrapped[3])
        self.assertIn("'firmware path'", wrapped[3])

    @unittest.skipUnless(sys.platform.startswith("linux"), "dialout remediation is Linux-specific")
    def test_missing_membership_is_added_before_sg_reexec(self) -> None:
        command = ["espflash", "flash", "firmware"]
        group = types.SimpleNamespace(gr_gid=20, gr_mem=[])
        port = Path("/dev/ttyACM0")
        with (
            mock.patch.object(SERIAL.os, "geteuid", return_value=1000),
            mock.patch.object(Path, "exists", return_value=True),
            mock.patch.object(SERIAL, "_dialout_group", return_value=group),
            mock.patch.object(SERIAL, "_candidate_ports", return_value=[port]),
            mock.patch.object(SERIAL, "_needs_dialout", return_value=True),
            mock.patch.object(SERIAL, "_username", return_value=TEST_USERNAME),
            mock.patch.object(SERIAL, "_persistent_member", return_value=False),
            mock.patch.object(SERIAL, "_active_member", return_value=False),
            mock.patch.object(SERIAL, "_sudo_add_membership") as add,
            mock.patch.object(SERIAL.shutil, "which", side_effect=lambda name: "/usr/bin/sg" if name == "sg" else None),
        ):
            wrapped = SERIAL.prepare_serial_command(command, str(port))
        add.assert_called_once_with(TEST_USERNAME)
        self.assertEqual(wrapped[:3], ["/usr/bin/sg", "dialout", "-c"])

    def test_sudo_prompt_explains_scope_and_never_runs_espflash_as_root(self) -> None:
        group = types.SimpleNamespace(gr_gid=20, gr_mem=[TEST_USERNAME])
        completed = subprocess.CompletedProcess(args=[], returncode=0)
        output = io.StringIO()
        with (
            mock.patch.object(SERIAL.sys.stdin, "isatty", return_value=True),
            mock.patch.object(SERIAL.shutil, "which", side_effect=lambda name: {"sudo": "/usr/bin/sudo", "usermod": "/usr/sbin/usermod"}.get(name)),
            mock.patch.object(SERIAL.subprocess, "run", return_value=completed) as run,
            mock.patch.object(SERIAL, "_dialout_group", return_value=group),
            mock.patch.object(SERIAL, "_persistent_member", return_value=True),
            mock.patch("sys.stdout", output),
        ):
            SERIAL._sudo_add_membership(TEST_USERNAME)
        run.assert_called_once_with(
            ["/usr/bin/sudo", "/usr/sbin/usermod", "-aG", "dialout", TEST_USERNAME], check=False
        )
        text = output.getvalue()
        self.assertIn("sudo password may be requested", text)
        self.assertIn("espflash are NOT run as root", text)
        self.assertIn("newgrp-equivalent via sg", text)

    def test_noninteractive_missing_membership_fails_without_sudo(self) -> None:
        with (
            mock.patch.object(SERIAL.sys.stdin, "isatty", return_value=False),
            mock.patch.object(SERIAL.shutil, "which", return_value="/bin/tool"),
            mock.patch.object(SERIAL.subprocess, "run") as run,
        ):
            with self.assertRaises(SERIAL.SerialAccessError):
                SERIAL._sudo_add_membership(TEST_USERNAME)
        run.assert_not_called()

    def test_windows_enumerator_extracts_com_port_and_pnp_identity(self) -> None:
        result = subprocess.CompletedProcess(
            args=[],
            returncode=0,
            stdout=(
                "COM11\tUSB\\VID_303A&PID_1001\\ABC\t"
                "USB JTAG/serial debug unit (COM11)\n"
            ),
            stderr="",
        )
        with mock.patch.object(SERIAL.shutil, "which", return_value="powershell.exe"), \
             mock.patch.object(SERIAL.subprocess, "run", return_value=result) as run:
            self.assertEqual(
                SERIAL._windows_serial_ports(),
                (("COM11", r"USB\VID_303A&PID_1001\ABC USB JTAG/serial debug unit (COM11)"),),
            )
        command = run.call_args.args[0]
        self.assertEqual(command[:3], ["powershell.exe", "-NoProfile", "-Command"])
        self.assertIn("Win32_PnPEntity", command[3])

    def test_explicit_port_bypasses_host_enumeration(self) -> None:
        with mock.patch.object(SERIAL, "_enumerate_serial_ports") as listing:
            self.assertEqual(SERIAL.resolve_serial_port(" COM17 "), "COM17")
        listing.assert_not_called()

    def test_espflash_port_environment_bypasses_enumeration(self) -> None:
        with mock.patch.object(SERIAL, "_enumerate_serial_ports") as listing:
            self.assertEqual(
                SERIAL.resolve_serial_port(None, environ={"ESPFLASH_PORT": "COM21"}),
                "COM21",
            )
        listing.assert_not_called()

    def test_auto_port_uses_only_serial_device(self) -> None:
        with mock.patch.object(SERIAL, "_enumerate_serial_ports", return_value=(("COM7", "USB serial"),)):
            self.assertEqual(SERIAL.resolve_serial_port(None, environ={}), "COM7")

    def test_auto_port_prefers_unique_native_core_s3_when_multiple(self) -> None:
        ports = (
            ("COM3", r"USB\VID_10C4&PID_EA60 Silicon Labs CP210x"),
            ("COM11", r"USB\VID_303A&PID_1001 Espressif USB JTAG/serial debug unit"),
        )
        with mock.patch.object(SERIAL, "_enumerate_serial_ports", return_value=ports):
            self.assertEqual(SERIAL.resolve_serial_port(None, environ={}), "COM11")

    def test_auto_port_refuses_ambiguous_serial_devices(self) -> None:
        ports = (
            ("COM3", r"USB\VID_10C4&PID_EA60 Silicon Labs CP210x"),
            ("COM4", r"USB\VID_1A86&PID_7523 QinHeng CH340"),
        )
        with mock.patch.object(SERIAL, "_enumerate_serial_ports", return_value=ports):
            with self.assertRaisesRegex(SERIAL.SerialAccessError, "multiple serial ports"):
                SERIAL.resolve_serial_port(None, environ={})

    def test_auto_port_reports_no_visible_ports_without_prompting(self) -> None:
        with mock.patch.object(SERIAL, "_enumerate_serial_ports", return_value=()):
            with self.assertRaisesRegex(SERIAL.SerialAccessError, "no serial port was found"):
                SERIAL.resolve_serial_port(None, environ={})

    def test_non_linux_hosts_are_noop(self) -> None:
        command = ["espflash", "flash", "firmware"]
        with mock.patch.object(SERIAL.sys, "platform", "win32"):
            self.assertEqual(SERIAL.prepare_serial_command(command), command)


if __name__ == "__main__":
    unittest.main()
