from __future__ import annotations
from pathlib import Path
import re
from .firmware_cores3_spi import check_cores3_spi_ownership
def _roots(root: Path) -> tuple[Path, Path]:
    firmware = root / "apps/kaskold-hardware/src/hw"
    return (
        firmware / "m5stack/storage",
        firmware / "shared/storage/fat32",
    )


def _sdcard_limits(root: Path, m5_root: Path, fat32_root: Path) -> dict[Path, int]:
    return {
        m5_root / "transport/mod.rs": 40,
        m5_root / "transport/protocol/mod.rs": 40,
        m5_root / "transport/protocol/initialization.rs": 140,
        m5_root / "transport/protocol/wire.rs": 140,
        m5_root / "transport/block.rs": 120,
        m5_root / "transport/multi_block.rs": 100,
        m5_root / "transport/card.rs": 100,
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/mod.rs": 40,
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/config.rs": 80,
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/state.rs": 180,
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/lcd.rs": 150,
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/gpio35.rs": 60,
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/power_cycle.rs": 60,
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/sd_power_lines.rs": 80,
        fat32_root / "types.rs": 100,
        fat32_root / "policy.rs": 360,
        fat32_root / "allocation.rs": 120,
        fat32_root / "directory.rs": 230,
        fat32_root / "files.rs": 190,
        fat32_root / "lfn.rs": 180,
        root / "crates/kaskold-hardware-core/src/storage/fat32_lfn.rs": 320,
        fat32_root / "lfn/scanner.rs": 120,
        fat32_root / "format.rs": 150,
    }


def _read_fat32_sources(fat32_root: Path) -> tuple[str, str]:
    top_level = "\n".join(
        path.read_text(errors="ignore") for path in fat32_root.glob("*.rs")
    ) if fat32_root.exists() else ""
    recursive = "\n".join(
        path.read_text(errors="ignore") for path in fat32_root.rglob("*.rs")
    ) if fat32_root.exists() else ""
    return top_level, recursive


def _check_required_sdcard_paths(root: Path, m5_root: Path, fat32_root: Path) -> list[str]:
    required = (
        m5_root / "mod.rs",
        m5_root / "transport/mod.rs",
        m5_root / "transport/protocol/mod.rs",
        m5_root / "transport/protocol/initialization.rs",
        m5_root / "transport/protocol/wire.rs",
        m5_root / "transport/block.rs",
        m5_root / "transport/multi_block.rs",
        m5_root / "transport/card.rs",
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/mod.rs",
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/config.rs",
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/state.rs",
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/lcd.rs",
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/gpio35.rs",
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/power_cycle.rs",
        root / "apps/kaskold-hardware/src/hw/m5stack/spi_bus/sd_power_lines.rs",
        fat32_root / "mod.rs",
        fat32_root / "types.rs",
        fat32_root / "policy.rs",
        fat32_root / "allocation.rs",
        fat32_root / "directory.rs",
        fat32_root / "files.rs",
        fat32_root / "lfn.rs",
        fat32_root / "lfn/scanner.rs",
        fat32_root / "format.rs",
    )
    return [
        f"required firmware SD-card module is missing: {path.relative_to(root)}"
        for path in required
        if not path.exists()
    ]


def _check_sdcard_facade(root: Path, m5_root: Path) -> list[str]:
    facade = m5_root / "mod.rs"
    if not facade.exists():
        return []
    source = facade.read_text(errors="ignore")
    errors: list[str] = []
    if len(source.splitlines()) > 100:
        errors.append(f"SD-card facade exceeds 100 lines: {facade.relative_to(root)}")
    if "mod transport;" not in source:
        errors.append(f"SD-card facade lost board transport ownership: {facade.relative_to(root)}")
    if "pub use crate::hw::shared::storage::fat32::{" not in source:
        errors.append(f"SD-card facade no longer consumes shared FAT32: {facade.relative_to(root)}")
    if "include!(" in source:
        errors.append(f"SD-card facade must use real Rust modules: {facade.relative_to(root)}")
    return errors


def _check_sdcard_module_limits(root: Path, m5_root: Path, fat32_root: Path) -> list[str]:
    return [
        f"firmware SD-card module exceeds SRP limit: {path.relative_to(root)}"
        for path, maximum in _sdcard_limits(root, m5_root, fat32_root).items()
        if path.exists() and len(path.read_text(errors="ignore").splitlines()) > maximum
    ]


def _check_fat32_platform_isolation(fat32_all_source: str) -> list[str]:
    errors = [
        f"shared FAT32 contains platform transport concern: {forbidden}"
        for forbidden in (
            "SDHOST_", "SPI2_", "GPIO_OUT", "GPIO_ENABLE", "IO_MUX", "reg_write(",
            "bb_transfer", "sdhost_send_cmd", "restore_display_state", "restore_spi_state",
        )
        if forbidden in fat32_all_source
    ]
    if re.search(r"\b(?:struct|impl)\s+BmpInfo\b", fat32_all_source):
        errors.append("unused FAT32 BmpInfo type must not return")
    return errors


def _check_lfn_decomposition(root: Path, fat32_root: Path) -> list[str]:
    lfn_helpers = {
        root / "crates/kaskold-hardware-core/src/storage/fat32_lfn.rs": (
            "struct LfnAccumulator", "fn record", "fn display_name", "fn map_latin1",
            "enum DirectoryEntryKind", "fn classify_directory_entry",
        ),
        fat32_root / "lfn/scanner.rs": (
            "enum ScanControl", "fn scan_cluster", "fn process_entry",
        ),
    }
    errors: list[str] = []
    for module_path, symbols in lfn_helpers.items():
        if not module_path.is_file():
            errors.append(
                f"FAT32 LFN decomposition source is missing: {module_path.relative_to(root)}"
            )
            continue
        source = module_path.read_text(errors="ignore")
        errors.extend(
            f"FAT32 LFN decomposition lost {symbol} in {module_path.relative_to(root)}"
            for symbol in symbols
            if symbol not in source
        )
    return errors


def _check_fat32_inventory(fat32_source: str) -> list[str]:
    expected_functions = {
        "from_boot_sector", "cluster_to_sector", "cluster_bytes", "from_bytes",
        "first_cluster", "is_dir", "matches", "to_83_name",
        "checked_fat_entry_location", "read_fat_entry", "read_file_progress",
        "detect_fat32_partition", "write_fat_entry", "allocate_cluster",
        "allocate_chain", "mount_fat32", "find_file_in_root", "read_file",
        "create_file", "create_file_progress", "write_dir_entry_to_root",
        "delete_file", "overwrite_file", "list_root_dir", "list_root_dir_lfn",
        "format_83_display", "format_fat32", "do_format_fat32",
    }
    actual_functions = set(re.findall(
        r"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(?:<|\()",
        fat32_source,
    ))
    if actual_functions == expected_functions:
        return []
    return [
        "shared FAT32 responsibility inventory changed: expected "
        f"{sorted(expected_functions)}, got {sorted(actual_functions)}"
    ]


def _check_sdcard_layout(root: Path) -> list[str]:
    m5_root, fat32_root = _roots(root)
    fat32_source, fat32_all_source = _read_fat32_sources(fat32_root)
    return [
        *_check_required_sdcard_paths(root, m5_root, fat32_root),
        *_check_sdcard_facade(root, m5_root),
        *_check_sdcard_module_limits(root, m5_root, fat32_root),
        *_check_fat32_platform_isolation(fat32_all_source),
        *_check_lfn_decomposition(root, fat32_root),
        *_check_fat32_inventory(fat32_source),
    ]


def _check_shared_fat32_policy(root: Path) -> list[str]:
    errors: list[str] = []
    m5_root, fat32_root = _roots(root)
    shared_policy = fat32_root / "policy.rs"
    for stale in (m5_root / "fat32_policy.rs",):
        if stale.exists():
            errors.append(f"board-specific FAT32 policy copy must not exist: {stale.relative_to(root)}")

    fat32_mod = fat32_root / "mod.rs"
    if fat32_mod.exists():
        source = fat32_mod.read_text(errors="ignore")
        if "mod policy;" not in source or "include!(" in source:
            errors.append("shared FAT32 facade must own policy.rs through a real module")

    policy_source = shared_policy.read_text(errors="ignore") if shared_policy.exists() else ""
    boot_policy = fat32_root / "policy" / "boot.rs"
    metadata_policy = root / "crates/kaskold-hardware-core/src/storage/fat32_metadata/mod.rs"
    guard_source = policy_source + (boot_policy.read_text(errors="ignore") if boot_policy.exists() else "")
    guard_source += metadata_policy.read_text(errors="ignore") if metadata_policy.exists() else ""
    required_guards = {
        "invalid-cluster guard": "if cluster < 2",
        "checked FAT geometry": "checked_mul(fat_size_32)",
        "saturating cluster arithmetic": "saturating_mul(self.sectors_per_cluster",
        "checked FAT entry location": "checked_fat_entry_location",
        "FAT entry bounds check": 'return Err("FAT offset out of range")',
        "bounded chain traversal": "MAX_FAT_CHAIN_STEPS",
        "premature-chain rejection": 'return Err("FAT chain ended before file size")',
        "MBR signature validation": 'return Err("Invalid MBR signature")',
    }
    for description, marker in required_guards.items():
        if marker not in guard_source:
            errors.append(f"shared FAT32 policy lost {description}")

    storage_source = "\n".join(
        path.read_text(errors="ignore")
        for base in (m5_root, fat32_root)
        for path in base.rglob("*.rs")
    )
    for name in (
        "checked_fat_entry_location", "read_fat_entry", "read_file_progress",
        "to_83_name", "detect_fat32_partition",
    ):
        count = len(re.findall(
            rf"(?m)^\s*(?:pub(?:\([^)]*\))?\s+)?fn\s+{name}\s*\(", storage_source
        ))
        if count != 1:
            errors.append(f"FAT32 policy function {name} must have one shared implementation, found {count}")
    return errors


def _check_real_module_boundaries(root: Path) -> list[str]:
    errors: list[str] = []
    m5_root, fat32_root = _roots(root)
    targets = (
        m5_root / "mod.rs",
        m5_root / "transport/mod.rs",
        fat32_root / "mod.rs",
    )
    for path in targets:
        if not path.exists():
            continue
        source = path.read_text(errors="ignore")
        if "include!(" in source:
            errors.append(f"storage boundary uses include! instead of modules: {path.relative_to(root)}")
        if not re.search(r"(?m)^\s*(?:#\[path = [^]]+\]\s*)?mod\s+[A-Za-z_][A-Za-z0-9_]*;", source):
            errors.append(f"storage facade has no real child modules: {path.relative_to(root)}")
    return errors


def _check_transport_owner_imports(root: Path) -> list[str]:
    errors: list[str] = []
    m5_root, _ = _roots(root)

    m5_transport = m5_root / "transport"
    wire = m5_transport / "protocol/wire.rs"
    for path in m5_transport.rglob("*.rs"):
        source = path.read_text(errors="ignore")
        if re.search(r"use\s+super(?:::\s*super)*::\*", source):
            errors.append(f"M5 SD transport leaf uses wildcard inheritance: {path.relative_to(root)}")
        for forbidden in (
            "0x6002_", "SPI2_CLOCK_REG", "SPI2_USER_REG", "save_and_reclaim",
            "restore_spi_state", "GPIO_FUNC_OUT_SEL", "GPIO_ENABLE1_",
        ):
            if forbidden in source:
                errors.append(
                    f"M5 SD transport bypasses the shared SPI2 owner with {forbidden}: "
                    f"{path.relative_to(root)}"
                )
        if path != wire and "m5stack::spi_bus" in source:
            errors.append(
                f"M5 SD transport bypasses its wire owner: {path.relative_to(root)}"
            )
    wire_source = wire.read_text(errors="ignore") if wire.exists() else ""
    if "crate::hw::m5stack::spi_bus::with_sd_selected" not in wire_source:
        errors.append("M5 SD wire layer no longer uses the shared SPI2 owner")
    return errors


def check(root: Path) -> list[str]:
    return [
        *_check_sdcard_layout(root),
        *_check_shared_fat32_policy(root),
        *_check_real_module_boundaries(root),
        *_check_transport_owner_imports(root),
        *check_cores3_spi_ownership(root),
    ]
