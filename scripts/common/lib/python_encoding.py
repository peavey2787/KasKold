"""The text encoding every Python tool a make target starts must use."""
from __future__ import annotations

from collections.abc import MutableMapping


def use_utf8_python_children(environment: MutableMapping[str, str]) -> None:
    """Give every Python tool a make target starts the encoding run-all gives it.

    Repository sources and device output are UTF-8; a Windows console's ANSI code
    page would otherwise decide how child tools read them.
    """
    environment["PYTHONUTF8"] = "1"
    environment["PYTHONIOENCODING"] = "utf-8"
