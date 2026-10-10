from __future__ import annotations

from pathlib import Path
import re

from architecture.core.common import rust_code_only

_STALE_ROOT_PATHS = (
    (re.compile(r"\bcrate::address\b"), "crate::account::address"),
    (re.compile(r"\bcrate::adaptor\b"), "crate::privacy::adaptor"),
    (re.compile(r"\bcrate::bip32\b"), "crate::account::bip32"),
    (re.compile(r"\bcrate::transaction\b"), "crate::protocol::transaction"),
)

_DUPLICATE_PROTOCOL_PATHS = (
    re.compile(r"\bprotocol\s*::\s*protocol\b"),
    re.compile(r"\bprotocol\s*::\s*\{[^{}]*\bprotocol\s*::", re.DOTALL),
)

def check(root: Path) -> list[str]:
    online_root = root / "crates/online-watcher/src"
    errors: list[str] = []
    for path in online_root.rglob("*.rs"):
        source = rust_code_only(path.read_text(errors="ignore"))
        for pattern, replacement in _STALE_ROOT_PATHS:
            if pattern.search(source):
                errors.append(
                    f"online watcher retains moved root path in {path.relative_to(root)}; "
                    f"use {replacement}"
                )
        if any(pattern.search(source) for pattern in _DUPLICATE_PROTOCOL_PATHS):
            errors.append(
                f"online watcher repeats the protocol module in {path.relative_to(root)}; "
                "use crate::protocol::transaction"
            )
    for script_root in (
        online_root / "contracts/covenant/script",
        online_root / "contracts/oracle/script",
    ):
        for path in script_root.rglob("*.rs"):
            if path.name != "mod.rs" and re.search(
                r"(?m)^\s*use\s+covenant_ops::\*;",
                rust_code_only(path.read_text(errors="ignore")),
            ):
                errors.append(
                    f"nested covenant script must import its parent opcode alias: "
                    f"{path.relative_to(root)}"
                )
    covenant_family_root = online_root / "wasm_api/contracts/covenant/families"
    low_level_locktime = re.compile(
        r"\b(?:crate\s*::\s*)?protocol\s*::\s*script\s*::\s*"
        r"extract_(?:csv_sequence|cltv_locktime)\b"
    )
    for path in covenant_family_root.rglob("*.rs"):
        if low_level_locktime.search(rust_code_only(path.read_text(errors="ignore"))):
            errors.append(
                f"covenant WASM family bypasses the normalized locktime façade in "
                f"{path.relative_to(root)}; use the covenant extract_* adapter"
            )
    for path in (online_root / "protocol/pskt").rglob("*.rs"):
        if "unit_tests" in path.parts:
            continue
        if "pub(super)" in rust_code_only(path.read_text(errors="ignore")):
            errors.append(f"PSKT sibling façade item is too narrow: {path.relative_to(root)}")

    return errors
