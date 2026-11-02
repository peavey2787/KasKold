"""Hardware display transport boundary checks."""

from __future__ import annotations

import re
from pathlib import Path

BOARD_CONTRACTS = {
    "m5stack/display.rs": {
        "model": "models::ILI9342CRgb565",
        "orientation": "Rotation::Deg180",
        "color_order": "ColorOrder::Bgr",
        "extra": ".set_orientation(",
        "transport": "lcd_device(cs_pin)?",
    },
}

PRESENTATION_SYMBOLS = (
    "draw_lato_title",
    "draw_lato_title_opaque",
    "draw_lato_body",
    "draw_lato_18",
    "draw_lato_22",
    "draw_lato_22_opaque",
    "draw_lato_hint",
    "draw_oswald_header",
    "draw_rubik_big",
    "draw_oswald_sub",
    "measure_title",
    "measure_body",
    "measure_18",
    "measure_22",
    "measure_header",
    "measure_big",
    "measure_sub",
    "measure_hint",
    "draw_menu_icon",
)


FORBIDDEN_TRANSPORT_OWNERSHIP = (
    "crate::ui::",
    "crate::services::",
    "embedded_iconoir",
    "BootPresentation",
    "MenuIconProfile",
    "TransactionMenuIcon",
    "board_label:",
    "verification_version_y:",
    "verification_hash_y:",
)

UPWARD_DEPENDENCY_TOKENS = ("crate::ui::", "crate::services::", "embedded_iconoir", "prop_fonts")

STALE_REFERENCES = (
    "crate::hw::icon_data",
    "crate::hw::display::COLOR_",
    "crate::hw::display::KASPA_",
    "crate::hw::display::draw_",
    "crate::hw::display::measure_",
    "use crate::hw::display::*;",
)


def impl_method_names(source: str, type_name: str) -> tuple[str, ...]:
    impl_match = re.search(rf"impl<'a>\s+{re.escape(type_name)}<'a>\s*\{{", source)
    if not impl_match:
        return ()
    start = source.find("{", impl_match.start())
    depth = 0
    index = start
    while index < len(source):
        if source[index] == "{":
            depth += 1
        elif source[index] == "}":
            depth -= 1
            if depth == 0:
                body = source[start + 1:index]
                return tuple(re.findall(r"(?m)^\s*pub fn\s+([A-Za-z_]\w*)", body))
        index += 1
    return ()


def _shared_primitive_errors(hardware_root: Path) -> list[str]:
    common_source = (hardware_root / "shared/display.rs").read_text(errors="ignore")
    errors = [
        f"shared display transport primitive is missing: {fragment}"
        for fragment in ("clear_and_settle", "Rgb565::BLACK")
        if fragment not in common_source
    ]
    errors.extend(
        f"board-specific display transport leaked into shared display: {fragment}"
        for fragment in ("DisplayInterface", "spi_interface", "SPI_BUFFER")
        if fragment in common_source
    )
    return errors


def _contract_errors(source: str, facade_name: str, contract: dict[str, str]) -> list[str]:
    required_fragments = (
        "pub struct BootDisplay<'a>",
        "pub type TeeDisplay<'a> = crate::hw::shared::display_support::TeeDisplay<PanelDisplay<'a>>;",
        contract["transport"],
        "shared::display::clear_and_settle",
        contract["model"],
        contract["orientation"],
        contract["color_order"],
        contract["extra"],
    )
    return [
        f"firmware display transport lost board contract: {facade_name}: {fragment}"
        for fragment in required_fragments
        if fragment not in source
    ]


def _facade_errors(source: str, facade_name: str, contract: dict[str, str]) -> list[str]:
    errors = _contract_errors(source, facade_name, contract)
    methods = impl_method_names(source, "BootDisplay")
    if methods != ("new",):
        errors.append(
            f"hardware display facade owns non-transport methods in {facade_name}: {methods}"
        )
    errors.extend(
        f"UI presentation remains in hardware display {facade_name}: {token}"
        for token in FORBIDDEN_TRANSPORT_OWNERSHIP
        if token in source
    )
    errors.extend(
        f"UI display helper remains implemented in {facade_name}: {symbol}"
        for symbol in PRESENTATION_SYMBOLS
        if re.search(rf"(?m)^pub\(crate\) fn\s+{re.escape(symbol)}\b", source)
    )
    return errors


def _facade_inventory_errors(hardware_root: Path) -> tuple[list[str], str]:
    errors: list[str] = []
    combined_hardware = ""
    for facade_name, contract in BOARD_CONTRACTS.items():
        path = hardware_root / facade_name
        if not path.is_file():
            continue
        source = path.read_text(errors="ignore")
        combined_hardware += "\n" + source
        errors.extend(_facade_errors(source, facade_name, contract))
    return errors, combined_hardware


def _upward_dependency_errors(combined_hardware: str) -> list[str]:
    return [
        f"hardware display depends upward on presentation/service code: {token}"
        for token in UPWARD_DEPENDENCY_TOKENS
        if token in combined_hardware
    ]


def _exposure_errors(firmware_root: Path) -> list[str]:
    errors: list[str] = []
    hw_mod = (firmware_root / "hw/mod.rs").read_text(errors="ignore")
    if "icon_data" in hw_mod:
        errors.append("UI icon data is still exposed through hw/mod.rs")
    ui_mod = (firmware_root / "ui/mod.rs").read_text(errors="ignore")
    if "pub mod display;" not in ui_mod:
        errors.append("ui/mod.rs must expose the display presentation facade")
    return errors


def _stale_reference_errors(firmware_root: Path) -> list[str]:
    all_firmware = "\n".join(path.read_text(errors="ignore") for path in firmware_root.rglob("*.rs"))
    return [
        f"stale hardware-owned display presentation reference remains: {stale}"
        for stale in STALE_REFERENCES
        if stale in all_firmware
    ]


def check_hardware_transport(root: Path) -> list[str]:
    firmware_root = root / "apps/kaskold-hardware/src"
    hardware_root = firmware_root / "hw"
    shared_errors = _shared_primitive_errors(hardware_root)
    facade_errors, combined_hardware = _facade_inventory_errors(hardware_root)
    return [
        *shared_errors,
        *facade_errors,
        *_upward_dependency_errors(combined_hardware),
        *_exposure_errors(firmware_root),
        *_stale_reference_errors(firmware_root),
    ]
