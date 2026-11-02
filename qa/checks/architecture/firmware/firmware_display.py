"""Firmware display transport and UI presentation boundaries."""

from __future__ import annotations

from pathlib import Path

from .firmware_display_hardware import (
    check_hardware_transport,
    impl_method_names,
)

UI_DISPLAY_MODULES = {
    "mod.rs": 80,
    "boot.rs": 420,
    "icons.rs": 180,
    "palette.rs": 80,
    "typography.rs": 320,
}

ICON_SUBMODULES = {
    "classification.rs": 120,
    "drawing.rs": 300,
}

TRANSPORT_MODULES = {
    "shared/display.rs": 100,
    "m5stack/display.rs": 160,
}

EXPECTED_PRESENTATION_METHODS = (
    "show_verification_screen",
    "show_logo_screen",
    "show_panic_screen",
    "clear_screen",
    "draw_frame_counter",
    "draw_sig_status",
    "draw_back_button",
    "clear_keep_nav",
)

UI_PROFILE_FRAGMENTS = (
    "attestation::draw_attestation_details",
    'format_args!("V{}"',
    'let site = "kaspa.org";',
    "TransactionMenuIcon::Download",
    "audio: true",
)

FORBIDDEN_UI_TRANSPORT_TOKENS = (
    "esp_hal::",
    "embedded_hal_bus::",
    "mipidsi::",
    "StaticCell",
    "ST7789",
    "ILI9342",
)

UI_OWNERSHIP = {
    "palette.rs": ("KASPA_TEAL", "COLOR_BG"),
    "typography.rs": ("draw_lato_title", "measure_hint", "crate::ui::prop_fonts"),
    "icons.rs": ("draw_menu_icon", "MenuIconProfile", "mod classification", "mod drawing"),
    "boot.rs": (
        "show_verification_screen",
        "clear_keep_nav",
        "crate::services::fw_update",
        "crate::hw::display::BootDisplay",
        'include_bytes!("../../../assets/kascoin_90.raw")',
        'include_bytes!("../../../assets/logo_320x240.raw")',
    ),
    "icon_data.rs": ("ICON_BACK", "ICON_TRASH"),
}

ICON_OWNERSHIP_SYMBOLS = ("PREFIX_ICONS", "embedded_iconoir", "draw_classified_icon")

RETIRED_BOOT_TEXT = ("BootPresentation", "board_label:", "ACTIVE_BOARD_NAME")


def _shadow_support_errors(firmware_root: Path) -> list[str]:
    shadow_support = firmware_root / "hw/shared/display_support/mod.rs"
    if not shadow_support.is_file():
        return ["required screenshot display support is missing"]
    if len(shadow_support.read_text(errors="ignore").splitlines()) > 180:
        return ["screenshot display support exceeds 180-line SRP limit"]
    return []


def _transport_display_errors(root: Path, firmware_root: Path) -> list[str]:
    hardware_root = firmware_root / "hw"
    errors: list[str] = []
    for board_root in (hardware_root / "shared", hardware_root / "m5stack"):
        if (board_root / "presentation").exists():
            errors.append(f"display presentation must not live under {board_root.relative_to(root)}")

    actual_transport = {
        relative for relative in TRANSPORT_MODULES
        if (hardware_root / relative).is_file()
    }
    if actual_transport != set(TRANSPORT_MODULES):
        errors.append(
            "firmware hardware display inventory changed: expected "
            f"{sorted(TRANSPORT_MODULES)}, got {sorted(actual_transport)}"
        )

    for name, limit in TRANSPORT_MODULES.items():
        path = hardware_root / name
        if not path.is_file():
            errors.append(f"required firmware display transport is missing: {path.relative_to(root)}")
            continue
        line_count = len(path.read_text(errors="ignore").splitlines())
        if line_count > limit:
            errors.append(
                f"firmware display transport exceeds SRP limit: "
                f"{path.relative_to(root)} ({line_count} > {limit})"
            )
    errors.extend(_shadow_support_errors(firmware_root))
    return errors


def _ui_module_errors(root: Path, ui_root: Path) -> tuple[list[str], str]:
    errors: list[str] = []
    ui_modules = {path.name for path in ui_root.glob("*.rs")}
    expected_ui_modules = set(UI_DISPLAY_MODULES) | {"icon_data.rs"}
    if ui_modules != expected_ui_modules:
        errors.append(
            "firmware UI display inventory changed: expected "
            f"{sorted(expected_ui_modules)}, got {sorted(ui_modules)}"
        )

    combined_ui = ""
    for name, limit in UI_DISPLAY_MODULES.items():
        path = ui_root / name
        if not path.is_file():
            errors.append(f"required firmware UI display module is missing: {path.relative_to(root)}")
            continue
        source = path.read_text(errors="ignore")
        combined_ui += "\n" + source
        line_count = len(source.splitlines())
        if line_count > limit:
            errors.append(
                f"firmware UI display module exceeds SRP limit: "
                f"{path.relative_to(root)} ({line_count} > {limit})"
            )
    return errors, combined_ui


def _icon_submodule_errors(root: Path, ui_root: Path) -> tuple[list[str], str]:
    errors: list[str] = []
    icon_root = ui_root / "icons"
    actual_icon_modules = {path.name for path in icon_root.glob("*.rs")}
    if actual_icon_modules != set(ICON_SUBMODULES):
        errors.append(
            "firmware menu-icon submodule inventory changed: expected "
            f"{sorted(ICON_SUBMODULES)}, got {sorted(actual_icon_modules)}"
        )

    combined_icons = ""
    for name, limit in ICON_SUBMODULES.items():
        path = icon_root / name
        if not path.is_file():
            errors.append(f"required firmware menu-icon module is missing: {path.relative_to(root)}")
            continue
        source = path.read_text(errors="ignore")
        combined_icons += "\n" + source
        if len(source.splitlines()) > limit:
            errors.append(
                f"firmware menu-icon module exceeds SRP limit: "
                f"{path.relative_to(root)} ({len(source.splitlines())} > {limit})"
            )
    return errors, combined_icons


def _check_display_inventories(root: Path) -> tuple[list[str], str]:
    firmware_root = root / "apps/kaskold-hardware/src"
    ui_root = firmware_root / "ui/display"
    errors = _transport_display_errors(root, firmware_root)
    ui_errors, combined_ui = _ui_module_errors(root, ui_root)
    icon_errors, combined_icons = _icon_submodule_errors(root, ui_root)
    errors.extend(ui_errors)
    errors.extend(icon_errors)
    if not (ui_root / "icon_data.rs").is_file():
        errors.append("firmware UI icon data is missing")
    return errors, combined_ui + combined_icons


def _transport_leak_errors(combined_ui: str) -> list[str]:
    return [
        f"display transport dependency leaked into UI presentation: {token}"
        for token in FORBIDDEN_UI_TRANSPORT_TOKENS
        if token in combined_ui
    ]


def _ui_ownership_errors(ui_root: Path) -> list[str]:
    errors: list[str] = []
    for module_name, symbols in UI_OWNERSHIP.items():
        source = (ui_root / module_name).read_text(errors="ignore")
        errors.extend(
            f"firmware UI display ownership changed: {symbol} missing from {module_name}"
            for symbol in symbols
            if symbol not in source
        )
    return errors


def _icon_source(ui_root: Path) -> str:
    return "\n".join(
        path.read_text(errors="ignore")
        for path in (ui_root / "icons").glob("*.rs")
    )


def _icon_ownership_errors(icon_source: str) -> list[str]:
    return [
        f"firmware menu-icon decomposition lost ownership symbol: {symbol}"
        for symbol in ICON_OWNERSHIP_SYMBOLS
        if symbol not in icon_source
    ]


def _profile_fragment_errors(ui_root: Path, icon_source: str) -> list[str]:
    combined_profiles = (
        (ui_root / "boot.rs").read_text(errors="ignore")
        + "\n"
        + (ui_root / "icons.rs").read_text(errors="ignore")
        + "\n"
        + icon_source
    )
    return [
        f"firmware UI display profile lost presentation contract: {fragment}"
        for fragment in UI_PROFILE_FRAGMENTS
        if fragment not in combined_profiles
    ]


def _boot_api_errors(ui_root: Path) -> list[str]:
    boot_source = (ui_root / "boot.rs").read_text(errors="ignore")
    errors = [
        f"simplified boot/about screen regained retired board/profile text: {retired}"
        for retired in RETIRED_BOOT_TEXT
        if retired in boot_source
    ]
    presentation_methods = impl_method_names(boot_source, "BootDisplay")
    if presentation_methods != EXPECTED_PRESENTATION_METHODS:
        errors.append(
            "firmware display presentation API changed: expected "
            f"{EXPECTED_PRESENTATION_METHODS}, got {presentation_methods}"
        )
    return errors


def _check_ui_presentation(root: Path, combined_ui: str) -> list[str]:
    ui_root = root / "apps/kaskold-hardware/src/ui/display"
    icon_source = _icon_source(ui_root)
    return [
        *_transport_leak_errors(combined_ui),
        *_ui_ownership_errors(ui_root),
        *_icon_ownership_errors(icon_source),
        *_profile_fragment_errors(ui_root, icon_source),
        *_boot_api_errors(ui_root),
    ]


def _check_font_transport_batching(root: Path) -> list[str]:
    errors: list[str] = []
    fonts = (root / "apps/kaskold-hardware/src/ui/prop_fonts.rs").read_text(errors="ignore")
    if "display.draw_iter(core::iter::once" in fonts:
        errors.append(
            "firmware proportional-font renderer regressed to one display transaction per pixel"
        )
    draw_start = fonts.find("pub fn draw_prop_text<")
    opaque_start = fonts.find("pub fn draw_prop_text_opaque<")
    transparent = fonts[draw_start:opaque_start] if draw_start >= 0 and opaque_start > draw_start else ""
    for fragment in ("let pixels = (0..height as usize).flat_map", "display.draw_iter(pixels)"):
        if fragment not in transparent:
            errors.append(f"firmware proportional-font batching contract changed: missing {fragment}")

    opaque = fonts[opaque_start:] if opaque_start >= 0 else ""
    for fragment in (
        "let cell_width = cw.saturating_add(1)",
        "(0..cell_width).map",
        "display.fill_contiguous(&area, pixel_iter)",
    ):
        if fragment not in opaque:
            errors.append(f"firmware opaque-font batching contract changed: missing {fragment}")
    if "let gap_area = Rectangle::new" in opaque:
        errors.append("firmware opaque-font renderer regressed to a second display submit for glyph spacing")

    receive = (root / "apps/kaskold-hardware/src/ui/screens/wallet/address/receive.rs").read_text(errors="ignore")
    if "draw_lato_title_opaque" not in receive:
        errors.append("Receive address redraw must use opaque batched title glyphs")
    return errors


def check(root: Path) -> list[str]:
    inventory_errors, combined_ui = _check_display_inventories(root)
    return [
        *inventory_errors,
        *_check_ui_presentation(root, combined_ui),
        *check_hardware_transport(root),
        *_check_font_transport_batching(root),
    ]
