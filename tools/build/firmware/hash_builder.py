"""The platform-native command that builds firmware with a converged image hash."""

from __future__ import annotations

from collections.abc import Iterator
from contextlib import contextmanager
import os
from pathlib import Path
import shutil

FIRMWARE_TOOLS = Path(__file__).resolve().parent
HASH_SOURCE = FIRMWARE_TOOLS.parents[2] / "apps" / "kaskold-hardware" / "src" / "firmware_hash.rs"
POWERSHELL_HOSTS = ("pwsh.exe", "pwsh", "powershell.exe", "powershell")


@contextmanager
def preserved_hash_source(source: Path = HASH_SOURCE) -> Iterator[None]:
    """Put the committed firmware_hash.rs back once a hash-converging build ends.

    The build rewrites it so the image embeds its own hash, and steps inside the
    block (signing, verification) may read the generated value. Afterwards the
    checkout returns to its committed bytes whether the build passed or failed;
    an untouched file is left alone so its timestamp does not force a rebuild.
    """
    committed = source.read_bytes()
    try:
        yield
    finally:
        if source.read_bytes() != committed:
            source.write_bytes(committed)


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
