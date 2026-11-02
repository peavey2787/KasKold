#!/usr/bin/env python3
"""Board-specific ESP flash layout policy for KasKold firmware."""

from __future__ import annotations

import argparse
import csv
import hashlib
from dataclasses import dataclass
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[3]
SECTOR_SIZE = 0x1000
APP_ALIGNMENT = 0x10000
PARTITION_TABLE_OFFSET = 0x8000
PARTITION_TABLE_SIZE = 0x1000


@dataclass(frozen=True)
class BoardLayout:
    board: str
    partition_table: str | None = None
    flash_size_cli: str | None = None
    flash_size_bytes: int | None = None
    target_app_partition: str | None = None
    app_offset: int | None = None
    app_size: int | None = None
    state_offset: int | None = None
    state_size: int | None = None

    def partition_path(self) -> Path | None:
        if self.partition_table is None:
            return None
        return ROOT / self.partition_table

    def espflash_args(self) -> list[str]:
        args: list[str] = []
        if self.partition_table is not None:
            args.extend(("--partition-table", str(self.partition_path())))
        if self.flash_size_cli is not None:
            args.extend(("--flash-size", self.flash_size_cli))
        if self.target_app_partition is not None:
            args.extend(("--target-app-partition", self.target_app_partition))
        return args

    def espflash_connection_args(self) -> list[str]:
        """Connection policy shared by public flash and connected QA runners."""
        args = ["--chip", "esp32s3"]
        if self.board in {"m5stack", "m5stack-lite"}:
            # CoreS3 uses the ESP32-S3 native USB Serial/JTAG peripheral.
            # Force its dedicated reset sequence instead of relying on USB PID
            # classification through the host serial library.
            args.extend(("--before", "usb-reset"))
        return args


LAYOUTS = {
    "m5stack": BoardLayout(
        board="m5stack",
        partition_table="apps/kaskold-hardware/partitions/m5stack-cores3.csv",
        flash_size_cli="16mb",
        flash_size_bytes=0x0100_0000,
        target_app_partition="ota_0",
        app_offset=0x0001_0000,
        app_size=0x0020_0000,
        state_offset=0x00FF_C000,
        state_size=0x0000_4000,
    ),
    "m5stack-lite": BoardLayout(
        board="m5stack-lite",
        partition_table="apps/kaskold-hardware/partitions/m5stack-cores3.csv",
        flash_size_cli="16mb",
        flash_size_bytes=0x0100_0000,
        target_app_partition="ota_0",
        app_offset=0x0001_0000,
        app_size=0x0020_0000,
        state_offset=0x00FF_C000,
        state_size=0x0000_4000,
    ),
}



def layout_for(board: str) -> BoardLayout:
    try:
        return LAYOUTS[board]
    except KeyError as error:
        raise ValueError(f"unsupported firmware board: {board}") from error


def _parse_number(value: str) -> int:
    text = value.strip().lower()
    if not text:
        raise ValueError("blank numeric field")
    multiplier = 1
    if text.endswith("k"):
        multiplier, text = 1024, text[:-1]
    elif text.endswith("m"):
        multiplier, text = 1024 * 1024, text[:-1]
    return int(text, 0) * multiplier


def _read_partitions(path: Path) -> list[tuple[str, str, str, int, int]]:
    rows: list[tuple[str, str, str, int, int]] = []
    with path.open(newline="", encoding="utf-8") as handle:
        for raw in csv.reader(handle):
            if not raw or not any(field.strip() for field in raw):
                continue
            if raw[0].lstrip().startswith("#"):
                continue
            if len(raw) < 5:
                raise ValueError(f"{path}: partition row has fewer than five fields: {raw!r}")
            name, kind, subtype, offset, size = (field.strip() for field in raw[:5])
            rows.append((name, kind, subtype, _parse_number(offset), _parse_number(size)))
    return rows


_CORES3_BOARDS = frozenset({"m5stack", "m5stack-lite"})
_CORES3_PARTITION_NAMES = frozenset({
    "nvs", "otadata", "phy_init", "ota_0", "ota_1", "owner_stage",
    "kaskold_bootctl", "kaskold_qa", "kaskold_state",
})
_CORES3_FIXED_PARTITIONS = (
    (("nvs", "data", "nvs", 0x9000, 0x4000), "NVS layout"),
    (("otadata", "data", "ota", 0xD000, 0x2000), "OTA data layout"),
    (("phy_init", "data", "phy", 0xF000, 0x1000), "PHY layout"),
    (("ota_1", "app", "ota_1", 0x210000, 0x200000), "secondary OTA slot"),
    (("owner_stage", "data", "undefined", 0x410000, 0x200000), "owner firmware staging partition"),
    (("kaskold_bootctl", "data", "undefined", 0x610000, 0x1000), "boot-control partition"),
)
_PartitionRow = tuple[str, str, str, int, int]


def _require(condition: bool, board: str, message: str) -> None:
    if not condition:
        raise ValueError(f"{board}: {message}")


def _validate_partition_placement(layout: BoardLayout, rows: list[_PartitionRow]) -> None:
    board = layout.board
    flash_size_bytes = layout.flash_size_bytes
    _require(flash_size_bytes is not None, board, "custom partition table requires flash_size_bytes")
    assert flash_size_bytes is not None
    previous_end = PARTITION_TABLE_OFFSET + PARTITION_TABLE_SIZE
    for name, kind, _subtype, offset, size in sorted(rows, key=lambda row: row[3]):
        aligned = offset % SECTOR_SIZE == 0 and size > 0 and size % SECTOR_SIZE == 0
        _require(aligned, board, f"{name} must be non-empty and 4-KiB aligned")
        _require(kind != "app" or offset % APP_ALIGNMENT == 0, board, f"app partition {name} must be 64-KiB aligned")
        _require(offset >= previous_end, board, f"partition {name} overlaps a previous partition")
        end = offset + size
        _require(end <= flash_size_bytes, board, f"partition {name} exceeds physical flash capacity")
        previous_end = end


def _validate_cores3_contract(board: str, rows: list[_PartitionRow], by_name: dict[str, _PartitionRow]) -> None:
    _require(
        set(by_name) == _CORES3_PARTITION_NAMES,
        board,
        f"partition names must be exactly {sorted(_CORES3_PARTITION_NAMES)}",
    )
    for expected, description in _CORES3_FIXED_PARTITIONS:
        _require(by_name[expected[0]] == expected, board, f"{description} drifted from the CoreS3 contract")
    rollback_safe = not any(row[1] == "app" and row[2] in {"factory", "test"} for row in rows)
    _require(rollback_safe, board, "anti-rollback forbids factory/test app partitions")


def _validate_target_app(layout: BoardLayout, by_name: dict[str, _PartitionRow]) -> _PartitionRow:
    board = layout.board
    app_name = layout.target_app_partition
    _require(app_name is not None and app_name in by_name, board, "configured target app partition is missing")
    assert app_name is not None
    app = by_name[app_name]
    _require(app[1:3] == ("app", "ota_0"), board, f"{app_name} must be an app/ota_0 partition")
    _require(
        app[3] == layout.app_offset and app[4] == layout.app_size,
        board,
        f"{app_name} must remain at 0x{layout.app_offset:08X} with size 0x{layout.app_size:X}",
    )
    return app


def _validate_state_reservation(layout: BoardLayout, app: _PartitionRow, by_name: dict[str, _PartitionRow]) -> None:
    board = layout.board
    state = by_name.get("kaskold_state")
    _require(state is not None, board, "kaskold_state reservation is missing")
    assert state is not None
    _require(
        state[3] == layout.state_offset and state[4] == layout.state_size,
        board,
        f"kaskold_state must remain at 0x{layout.state_offset:08X} with size 0x{layout.state_size:X}",
    )
    _require(state[3] + state[4] == layout.flash_size_bytes, board, "kaskold_state must terminate at the end of flash")
    _require(state[1:3] == ("data", "undefined"), board, "kaskold_state must remain a data/undefined reservation")
    _require(app[3] + app[4] <= state[3], board, "application partition overlaps persistent-wallet storage")


def validate_layout(layout: BoardLayout) -> None:
    path = layout.partition_path()
    if path is None:
        return
    board = layout.board
    _require(path.is_file(), board, f"partition table not found: {path}")
    _require(layout.flash_size_bytes is not None, board, "custom partition table requires flash_size_bytes")

    rows = _read_partitions(path)
    _require(bool(rows), board, "partition table is empty")
    by_name = {row[0]: row for row in rows}
    _require(len(rows) == len(by_name), board, "duplicate partition names are forbidden")
    _validate_partition_placement(layout, rows)

    cores3 = board in _CORES3_BOARDS
    if cores3:
        _validate_cores3_contract(board, rows, by_name)
    app = _validate_target_app(layout, by_name)
    if cores3:
        qa = by_name.get("kaskold_qa")
        _require(qa == ("kaskold_qa", "data", "undefined", 0xFF8000, 0x4000), board, "kaskold_qa must reserve 16 KiB at 0x00FF8000")
    _validate_state_reservation(layout, app, by_name)


def partition_sha256(layout: BoardLayout) -> str | None:
    path = layout.partition_path()
    if path is None:
        return None
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    for name in ("check", "espflash-args", "connection-args", "sha256"):
        command = sub.add_parser(name)
        command.add_argument("--board", required=True, choices=tuple(LAYOUTS))
    return parser.parse_args()


def main() -> int:
    args = _parse_args()
    try:
        layout = layout_for(args.board)
        validate_layout(layout)
        if args.command == "espflash-args":
            for value in layout.espflash_args():
                print(value)
        elif args.command == "connection-args":
            for value in layout.espflash_connection_args():
                print(value)
        elif args.command == "sha256":
            digest = partition_sha256(layout)
            if digest is not None:
                print(digest)
        return 0
    except (OSError, ValueError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
