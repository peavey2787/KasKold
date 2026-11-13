#!/usr/bin/env python3
"""Serial-port selection and Linux permission preflight for firmware commands.

Port selection is deliberately non-interactive so Make/QA also works when a
launcher captures stdin/stdout instead of providing a real terminal. Linux
permission remediation never runs firmware tooling as root. If a Linux serial
device is protected by the ``dialout`` group and the invoking account lacks that
membership, this helper explains the sudo request, adds only the supplementary
group membership, and returns a command wrapped by ``sg dialout -c`` so the current invocation can
continue without requiring a logout/login. The parent shell is intentionally
left unchanged; future login sessions inherit the persistent membership.
"""
from __future__ import annotations

import glob
import os
from pathlib import Path
import shlex
import shutil
import stat
import subprocess
import sys
from typing import Any, Mapping

if sys.platform == "win32":  # pragma: no cover - exercised by native Windows wrapper validation
    grp: Any = None
    pwd: Any = None
else:
    import grp
    import pwd

DIALOUT = "dialout"
SERIAL_GLOBS = ("/dev/ttyACM*", "/dev/ttyUSB*")
FLASH_MONITOR_ARGS = ("monitor", "--chip", "esp32s3", "--before", "no-reset-no-sync")
MONITOR_LISTING = (
    "$ErrorActionPreference='Stop';"
    "Get-CimInstance Win32_Process -Filter \"Name='espflash.exe'\" | "
    "ForEach-Object { [string]$_.ProcessId + \"`t\" + [string]$_.CommandLine }"
)


class SerialAccessError(RuntimeError):
    """Serial selection or permission remediation could not be completed safely."""


def windows_host() -> bool:
    return os.name == "nt" or sys.platform.startswith("win")


def flash_monitor_command(elf: Path, port: str) -> list[str]:
    """Attach to a freshly flashed device without resetting or resynchronizing it."""
    return ["espflash", *FLASH_MONITOR_ARGS, "--elf", str(elf), "--port", port]


def _powershell(script: str, unavailable: str) -> str:
    """Run one PowerShell script and return its standard output."""
    shell = next(
        (
            path
            for name in ("pwsh.exe", "pwsh", "powershell.exe", "powershell")
            if (path := shutil.which(name))
        ),
        None,
    )
    if shell is None:
        raise SerialAccessError(f"PowerShell is required to {unavailable} on Windows")
    try:
        result = subprocess.run(
            [shell, "-NoProfile", "-Command", script],
            check=False,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except OSError as exc:
        raise SerialAccessError(f"could not {unavailable}: {exc}") from exc
    if result.returncode != 0:
        detail = (result.stderr or result.stdout).strip()
        suffix = f": {detail}" if detail else ""
        raise SerialAccessError(f"could not {unavailable}{suffix}")
    return result.stdout


def _flash_monitor_pids(listing: str, port: str) -> list[int]:
    """Process ids in a ``pid<TAB>command line`` listing that run the flash monitor on *port*."""
    signature = " ".join(FLASH_MONITOR_ARGS)
    suffix = f"--port {port}".lower()
    pids: list[int] = []
    for line in listing.splitlines():
        pid, _, command = line.strip().partition("\t")
        lowered = command.strip().lower()
        if pid.isdigit() and signature in lowered and lowered.endswith(suffix):
            pids.append(int(pid))
    return pids


def release_flash_monitor(port: str) -> None:
    """Close the UART monitor console an earlier ``make flash`` left open on *port*.

    A managed ``make flash`` on Windows opens espflash's monitor in its own
    console. Windows grants a COM port to one process, so until that console is
    closed every later serial command fails with "Serial port not found".
    """
    if not windows_host():
        return
    listing = _powershell(MONITOR_LISTING, "list the UART monitors holding a serial port")
    pids = _flash_monitor_pids(listing, port)
    if not pids:
        return
    ids = ",".join(str(pid) for pid in pids)
    print(f"Closing the KasKold UART monitor (PID {ids}) that holds {port}", flush=True)
    _powershell(
        f"Stop-Process -Id {ids} -Force -ErrorAction SilentlyContinue; "
        f"Wait-Process -Id {ids} -Timeout 10 -ErrorAction SilentlyContinue",
        f"close the UART monitor holding {port}",
    )


def _windows_serial_ports() -> tuple[tuple[str, str], ...]:
    """Enumerate Windows COM ports and PnP identity without opening a device."""
    script = (
        "$ErrorActionPreference='Stop';"
        "[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new();"
        "Get-CimInstance Win32_PnPEntity | "
        "Where-Object { $_.Name -match '\\(COM[0-9]+\\)' } | "
        "ForEach-Object { if ($_.Name -match '\\((COM[0-9]+)\\)') { "
        "$Matches[1] + \"`t\" + [string]$_.PNPDeviceID + \"`t\" + [string]$_.Name } }"
    )
    stdout = _powershell(script, "auto-detect the CoreS3 serial port (set PORT explicitly instead)")

    entries: list[tuple[str, str]] = []
    for line in stdout.splitlines():
        fields = line.strip().split("\t", 2)
        if not fields or not fields[0].strip():
            continue
        port = fields[0].strip()
        detail = " ".join(field.strip() for field in fields[1:] if field.strip())
        entries.append((port, detail))
    return tuple(dict.fromkeys(entries))


def _posix_serial_ports() -> tuple[tuple[str, str], ...]:
    """Enumerate common USB serial endpoints without pulling in pyserial."""
    patterns = list(SERIAL_GLOBS)
    if sys.platform == "darwin":
        patterns = [
            "/dev/cu.usbmodem*",
            "/dev/cu.usbserial*",
            "/dev/cu.SLAB_USBtoUART*",
            "/dev/cu.wchusbserial*",
        ]

    entries: dict[str, str] = {}
    if sys.platform.startswith("linux"):
        for alias in sorted(Path("/dev/serial/by-id").glob("*")):
            try:
                target = str(alias.resolve(strict=True))
            except (FileNotFoundError, OSError):
                continue
            entries.setdefault(target, alias.name)
    for pattern in patterns:
        for match in sorted(glob.glob(pattern)):
            entries.setdefault(match, Path(match).name)
    return tuple(entries.items())


def _enumerate_serial_ports() -> tuple[tuple[str, str], ...]:
    if windows_host():
        return _windows_serial_ports()
    if os.name == "posix":
        return _posix_serial_ports()
    return ()


def _preferred_core_s3_port(entries: tuple[tuple[str, str], ...]) -> str | None:
    """Prefer one uniquely identifiable CoreS3/native ESP USB endpoint."""
    scored: list[tuple[int, str]] = []
    for port, detail in entries:
        upper = detail.upper()
        score = 0
        if ("VID_303A" in upper and "PID_1001" in upper) or (
            "303A" in upper and "1001" in upper
        ):
            score = 5
        elif any(marker in upper for marker in ("ESPRESSIF", "M5STACK", "USB JTAG", "USB SERIAL/JTAG")):
            score = 4
        elif any(marker in upper for marker in ("CP210", "CH340", "CH910", "USB-SERIAL")):
            score = 2
        scored.append((score, port))
    best = max((score for score, _ in scored), default=0)
    winners = [port for score, port in scored if score == best and best > 0]
    return winners[0] if len(winners) == 1 else None


def resolve_serial_port(
    explicit_port: str | None, *, environ: Mapping[str, str] | None = None
) -> str:
    """Resolve one serial port without ever invoking espflash's dialog UI.

    An explicit Make/CLI port wins, followed by ESPFLASH_PORT. Otherwise the
    host serial inventory is inspected directly. Ambiguous or absent devices
    fail with an actionable error instead of attempting to prompt on captured
    stdin. This remains compatible with the repository-pinned espflash 3.3.0,
    which predates the `espflash list-ports` command.
    """
    requested = (explicit_port or "").strip()
    if requested:
        return requested
    environment = os.environ if environ is None else environ
    configured = environment.get("ESPFLASH_PORT", "").strip()
    if configured:
        print(f"Using ESPFLASH_PORT serial device: {configured}", flush=True)
        return configured

    entries = _enumerate_serial_ports()
    if len(entries) == 1:
        port = entries[0][0]
        print(f"Auto-selected serial port: {port}", flush=True)
        return port
    if len(entries) > 1:
        preferred = _preferred_core_s3_port(entries)
        if preferred:
            print(f"Auto-selected CoreS3 serial port: {preferred}", flush=True)
            return preferred
        visible = ", ".join(port for port, _ in entries)
        raise SerialAccessError(
            "multiple serial ports are connected and KasKold will not guess; "
            f"set PORT explicitly (for example make flash BOARD=m5stack PORT={entries[0][0]}). "
            f"Visible ports: {visible}"
        )

    raise SerialAccessError(
        "no serial port was found. Connect the CoreS3 or set PORT explicitly if it uses "
        "an unusual USB-UART adapter."
    )


def _username() -> str:
    assert pwd is not None
    if sys.platform != "linux":
        raise SerialAccessError("dialout account checks are Linux-only")
    return pwd.getpwuid(os.getuid()).pw_name


def _dialout_group() -> Any:
    assert grp is not None
    try:
        return grp.getgrnam(DIALOUT)
    except KeyError as exc:
        raise SerialAccessError(
            "Linux serial group 'dialout' does not exist on this host; "
            "KasKold will not invent or create a privileged system group."
        ) from exc


def _persistent_member(username: str, group: Any) -> bool:
    assert pwd is not None
    try:
        primary_gid = pwd.getpwnam(username).pw_gid
    except KeyError as exc:
        raise SerialAccessError(f"cannot resolve local account {username!r}") from exc
    return primary_gid == group.gr_gid or username in group.gr_mem


def _active_member(group: Any) -> bool:
    if sys.platform != "linux":
        raise SerialAccessError("dialout account checks are Linux-only")
    return os.getegid() == group.gr_gid or group.gr_gid in os.getgroups()


def _candidate_ports(explicit_port: str | None) -> list[Path]:
    if explicit_port:
        return [Path(explicit_port)]
    paths = {Path(match) for pattern in SERIAL_GLOBS for match in glob.glob(pattern)}
    return sorted(paths)


def _needs_dialout(path: Path, group: Any) -> bool:
    try:
        info = path.stat()
    except FileNotFoundError:
        return False
    if not stat.S_ISCHR(info.st_mode):
        return False
    if os.access(path, os.R_OK | os.W_OK):
        return False
    return info.st_gid == group.gr_gid


def _sudo_add_membership(username: str) -> None:
    sudo = shutil.which("sudo")
    usermod = shutil.which("usermod")
    if not sudo or not usermod:
        raise SerialAccessError(
            "serial access requires dialout membership, but sudo/usermod is unavailable"
        )
    if not sys.stdin.isatty():
        raise SerialAccessError(
            f"account {username!r} is not in dialout and this is not an interactive terminal; "
            f"run: sudo usermod -aG {DIALOUT} {shlex.quote(username)}"
        )

    print("\nKasKold needs read/write access to the connected ESP serial device.", flush=True)
    print(
        f"Your account {username!r} is not a member of Linux group {DIALOUT!r}, "
        "which owns /dev/ttyACM* and /dev/ttyUSB* devices on this system.",
        flush=True,
    )
    print(
        "KasKold will request sudo only to add your existing account to that supplementary group.",
        flush=True,
    )
    print(
        "Your sudo password may be requested. The firmware build and espflash are NOT run as root.",
        flush=True,
    )
    print(f"  + sudo usermod -aG {DIALOUT} {username}", flush=True)
    result = subprocess.run([sudo, usermod, "-aG", DIALOUT, username], check=False)
    if result.returncode != 0:
        raise SerialAccessError("sudo usermod failed; serial permissions were not changed")

    refreshed = _dialout_group()
    if not _persistent_member(username, refreshed):
        raise SerialAccessError("dialout membership was not visible after usermod; refusing to continue")
    print(
        "dialout membership updated. Applying it to this flash command now (newgrp-equivalent via sg).",
        flush=True,
    )


def prepare_serial_command(command: list[str], explicit_port: str | None = None) -> list[str]:
    """Return *command* unchanged or wrapped to obtain current dialout access.

    This is a Linux-only preflight. On other operating systems it is a no-op.
    It never broadens permissions on the device node and never executes espflash
    through sudo/root.
    """
    if sys.platform != "linux":
        return command
    if os.geteuid() == 0:
        return command

    ports = _candidate_ports(explicit_port)
    if not ports or not any(path.exists() for path in ports):
        return command
    group = _dialout_group()
    protected = [path for path in ports if _needs_dialout(path, group)]
    if not protected:
        return command

    username = _username()
    persistent = _persistent_member(username, group)
    active = _active_member(group)
    if active:
        return command

    if not persistent:
        _sudo_add_membership(username)
    else:
        print(
            f"Account {username!r} is already in dialout, but this shell has stale group credentials. "
            "Applying dialout to this flash command now (newgrp-equivalent via sg).",
            flush=True,
        )

    sg = shutil.which("sg")
    if not sg:
        raise SerialAccessError(
            "dialout membership is configured, but 'sg' is unavailable; log out/in or run 'newgrp dialout'"
        )
    return [sg, DIALOUT, "-c", shlex.join(command)]
