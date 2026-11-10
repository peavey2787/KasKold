"""The platform-native command that builds firmware with a converged image hash."""

from __future__ import annotations

import os
from pathlib import Path
import shutil

FIRMWARE_TOOLS = Path(__file__).resolve().parent
POWERSHELL_HOSTS = ("pwsh.exe", "pwsh", "powershell.exe", "powershell")


def _powershell() -> str:
    for name in POWERSHELL_HOSTS:
        found = shutil.which(name)
        if found:
            return found
    raise RuntimeError("PowerShell is required to build firmware on Windows")


def hash_builder_command(board: str, label: str, cargo_args: list[str], *, windows: bool | None = None) -> list[str]:
    """Return the build_with_hash invocation for this platform.

    Windows cannot execute the POSIX script, so it runs the PowerShell twin,
    whose parameters are the label first and then ``-Board``.
    """
    use_windows = os.name == "nt" if windows is None else windows
    if use_windows:
        script = FIRMWARE_TOOLS / "build_with_hash.ps1"
        return [_powershell(), "-NoProfile", "-ExecutionPolicy", "Bypass", "-File", str(script), label, "-Board", board, *cargo_args]
    return [str(FIRMWARE_TOOLS / "build_with_hash.sh"), "--board", board, label, *cargo_args]
