"""Firmware flashing helpers for the shared GNU Make dispatcher."""
from __future__ import annotations

import hashlib
import os
import shutil
import subprocess
import sys
from collections.abc import Callable
from pathlib import Path
from typing import TextIO

from serial_access import SerialAccessError, prepare_serial_command, resolve_serial_port

BuildFirmware = Callable[[str], tuple[int, Path | None]]
RunCommand = Callable[[list[str]], int]


def _board_args(root: Path, board: str, mode: str) -> tuple[int, list[str]]:
    helper = str(root / "tools/build/firmware/board_layout.py")
    result = subprocess.run(
        [sys.executable, helper, mode, "--board", board],
        cwd=root,
        check=False,
        text=True,
        stdout=subprocess.PIPE,
    )
    return result.returncode, [line for line in result.stdout.splitlines() if line]


def _selected_port(port: str, stderr: TextIO) -> str | None:
    try:
        return resolve_serial_port(port)
    except SerialAccessError as error:
        print(f"ERROR: {error}", file=stderr)
        return None


def _serial_command(command: list[str], port: str, stderr: TextIO) -> list[str] | None:
    try:
        return prepare_serial_command(command, port)
    except SerialAccessError as error:
        print(f"ERROR: {error}", file=stderr)
        return None


def _monitor_command(elf: Path, port: str) -> list[str]:
    """Attach to the freshly flashed device without resetting or resynchronizing it."""
    return [
        "espflash",
        "monitor",
        "--chip",
        "esp32s3",
        "--before",
        "no-reset-no-sync",
        "--elf",
        str(elf),
        "--port",
        port,
    ]


def _spawn_windows_monitor(root: Path, elf: Path, port: str, stdout: TextIO, stderr: TextIO) -> int:
    """Open espflash's interactive monitor in an independent Windows console."""
    command = _monitor_command(elf, port)
    resolved = shutil.which(command[0])
    if resolved:
        command[0] = resolved
    try:
        subprocess.Popen(
            command,
            cwd=root,
            close_fds=True,
            creationflags=getattr(subprocess, "CREATE_NEW_CONSOLE", 0x00000010),
        )
    except OSError as error:
        print(
            f"ERROR: firmware flashed on {port}, but the UART monitor console could not be opened: {error}",
            file=stderr,
        )
        return 1
    print(
        f"Flash completed successfully on {port}. UART monitor opened in a new console; "
        "press CTRL+C in that console to stop it.",
        file=stdout,
        flush=True,
    )
    return 0


def flash_firmware(
    root: Path,
    board: str,
    port: str,
    build_firmware: BuildFirmware,
    run_command: RunCommand,
    *,
    stdin: TextIO,
    stdout: TextIO,
    stderr: TextIO,
) -> int:
    """Build, flash, and preserve UART monitoring across interactive and managed launchers."""
    rc, elf = build_firmware(board)
    if rc != 0 or elf is None:
        return rc or 1

    connection_rc, connection_args = _board_args(root, board, "connection-args")
    if connection_rc != 0:
        return connection_rc
    layout_rc, layout_args = _board_args(root, board, "espflash-args")
    if layout_rc != 0:
        return layout_rc

    selected_port = _selected_port(port, stderr)
    if selected_port is None:
        return 1

    interactive_monitor = bool(stdin.isatty() and stdout.isatty())
    command = ["espflash", "flash"]
    if interactive_monitor:
        command.append("--monitor")
    # The pinned espflash 3.3.0 has no flash-level --non-interactive option. An
    # explicit --port bypasses its dialog UI; omitting --monitor avoids raw mode.
    command.extend(connection_args)
    command.extend(layout_args)
    command.extend(["--port", selected_port, str(elf)])

    prepared = _serial_command(command, selected_port, stderr)
    if prepared is None:
        return 1

    if interactive_monitor:
        print("Flash will remain in the interactive UART monitor; press CTRL+C to exit.", file=stdout, flush=True)
    else:
        if os.name == "nt" or sys.platform.startswith("win"):
            print(
                "Managed/non-terminal launcher detected; flashing first, then opening the "
                "interactive UART monitor in a separate console window.",
                file=stdout,
                flush=True,
            )
        else:
            print(
                "Managed/non-terminal launcher detected; flashing without espflash's interactive "
                "UART monitor because no portable detached-terminal launcher is configured on this host.",
                file=stdout,
                flush=True,
            )

    rc = run_command(prepared)
    if interactive_monitor and rc == 130:
        print("Serial monitor closed by user after successful flash.", file=stdout)
        return 0
    if rc != 0 or interactive_monitor:
        return rc
    if os.name == "nt" or sys.platform.startswith("win"):
        return _spawn_windows_monitor(root, elf, selected_port, stdout, stderr)
    print(
        f"Flash completed successfully on {selected_port}. Start `espflash monitor --port {selected_port}` "
        "from a terminal to attach UART output.",
        file=stdout,
    )
    return 0


def _verified_release_image(root: Path, board: str, release_dir: str, stderr: TextIO) -> Path | None:
    names = {"m5stack": "kaskold-m5stack-full.bin"}
    base = Path(release_dir).expanduser()
    if not base.is_absolute():
        base = root / base
    image = base / names[board]
    guidance = "ERROR: first run make release SIGNING_KEY=/path/to/signing-key"
    if not image.is_file() or image.stat().st_size == 0:
        print(f"ERROR: signed merged release image is missing: {image}", file=stderr)
        print(guidance, file=stderr)
        return None

    sums = base / "SHA256SUMS"
    if not sums.is_file():
        print(f"ERROR: release checksum manifest is missing: {sums}", file=stderr)
        print(guidance, file=stderr)
        return None
    expected = next(
        (
            parts[0].lower()
            for line in sums.read_text(encoding="utf-8").splitlines()
            if len(parts := line.split(maxsplit=1)) == 2 and parts[1].lstrip("*").strip() == image.name
        ),
        None,
    )
    if expected is None or len(expected) != 64:
        print(f"ERROR: {image.name} is not recorded in {sums}", file=stderr)
        return None
    if hashlib.sha256(image.read_bytes()).hexdigest() != expected:
        print(f"ERROR: signed merged release image checksum mismatch: {image}", file=stderr)
        return None
    return image


def flash_release(
    root: Path,
    board: str,
    port: str,
    release_dir: str,
    run_command: RunCommand,
    *,
    stdout: TextIO,
    stderr: TextIO,
) -> int:
    """Flash an existing checksum-verified signed normal-release image only."""
    image = _verified_release_image(root, board, release_dir, stderr)
    if image is None:
        return 2

    connection_rc, connection_args = _board_args(root, board, "connection-args")
    if connection_rc != 0:
        return connection_rc
    selected_port = _selected_port(port, stderr)
    if selected_port is None:
        return 1

    command = [
        "espflash",
        "write-bin",
        *connection_args,
        "--port",
        selected_port,
        "0x0",
        str(image),
    ]
    prepared = _serial_command(command, selected_port, stderr)
    if prepared is None:
        return 1
    print(f"Flashing existing signed normal-release image only: {image}", file=stdout)
    return run_command(prepared)
