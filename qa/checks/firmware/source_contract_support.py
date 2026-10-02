"""Shared helpers for firmware source contracts."""
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "qa/checks"))
from portal_source import kaskold_source  # noqa: E402

def read(relative: str) -> str:
    return kaskold_source(relative).read_text(encoding="utf-8")

def require(errors: list[str], condition: bool, message: str) -> None:
    if not condition:
        errors.append(message)
