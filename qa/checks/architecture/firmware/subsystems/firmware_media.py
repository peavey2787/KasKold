"""Firmware QR encoding and GC0308 driver decomposition contracts."""

from __future__ import annotations

from pathlib import Path


QR_MODULE_LIMITS = {
    "mod.rs": 100,
    "bit_writer.rs": 80,
    "modes/mod.rs": 40,
    "modes/byte_mode.rs": 120,
    "ecc/mod.rs": 40,
    "ecc/codewords.rs": 140,
    "constants.rs": 80,
    "error.rs": 40,
    "matrix/mod.rs": 40,
    "matrix/format.rs": 120,
    "ecc/gf.rs": 80,
    "matrix/masking.rs": 180,
    "matrix/matrix.rs": 240,
    "modes/numeric_mode.rs": 140,
    "ecc/reed_solomon.rs": 100,
    "matrix/version.rs": 80,
}

CAMERA_MODULE_LIMITS = {
    "mod.rs": 80,
    "bus.rs": 80,
    "initialization.rs": 220,
    "power.rs": 140,
    "registers.rs": 400,
    "types.rs": 80,
}


def check_inventory(
    root: Path,
    module_root: Path,
    limits: dict[str, int],
    label: str,
) -> tuple[list[str], str]:
    errors: list[str] = []
    expected = set(limits)
    actual = {path.relative_to(module_root).as_posix() for path in module_root.rglob("*.rs")}
    if actual != expected:
        errors.append(
            f"{label} module inventory changed: expected {sorted(expected)}, got {sorted(actual)}"
        )

    combined = ""
    for name, limit in limits.items():
        path = module_root / name
        if not path.is_file():
            errors.append(f"required {label} module is missing: {path.relative_to(root)}")
            continue
        source = path.read_text(errors="ignore")
        combined += "\n" + source
        line_count = len(source.splitlines())
        if line_count > limit:
            errors.append(
                f"{label} module exceeds SRP limit: "
                f"{path.relative_to(root)} ({line_count} > {limit})"
            )
    return errors, combined


def check(root: Path) -> list[str]:
    errors: list[str] = []
    firmware = root / "apps/kaskold-hardware/src"

    legacy_qr = firmware / "qr/encoder.rs"
    qr_root = firmware / "qr/encoder"
    if legacy_qr.exists():
        errors.append("legacy monolithic qr/encoder.rs must not exist")
    qr_errors, qr_source = check_inventory(root, qr_root, QR_MODULE_LIMITS, "QR encoder")
    errors.extend(qr_errors)

    qr_facade = (qr_root / "mod.rs").read_text(errors="ignore") if qr_root.exists() else ""
    for contract in (
        "pub use modes::byte_mode::encode;",
        "pub use matrix::matrix::QrCode;",
        "pub use modes::numeric_mode::encode_numeric;",
    ):
        if contract not in qr_facade:
            errors.append(f"QR encoder façade is missing required export: {contract}")

    if "pub use matrix::version::select_version;" in qr_facade:
        errors.append("QR encoder façade leaks the internal version selector")

    if qr_source.count("fn interleave_with_error_correction") != 1:
        errors.append("QR encoder must contain exactly one ECC interleaving pipeline")
    if "reed_solomon::encode" in (qr_root / "modes/byte_mode.rs").read_text(errors="ignore"):
        errors.append("byte mode bypasses the shared QR ECC pipeline")
    if "reed_solomon::encode" in (qr_root / "modes/numeric_mode.rs").read_text(errors="ignore"):
        errors.append("numeric mode bypasses the shared QR ECC pipeline")

    legacy_camera = firmware / "hw/camera/camera_gc0308.rs"
    camera_root = firmware / "hw/m5stack/cameras/gc0308"
    if legacy_camera.exists():
        errors.append("legacy monolithic camera_gc0308.rs must not exist")
    camera_errors, camera_source = check_inventory(
        root, camera_root, CAMERA_MODULE_LIMITS, "GC0308 driver"
    )
    errors.extend(camera_errors)

    camera_facade = (
        (camera_root / "mod.rs").read_text(errors="ignore") if camera_root.exists() else ""
    )
    for symbol in (
        "init_gc0308",
        "begin_entropy_capture",
        "end_entropy_capture",
        "CameraStatus",
    ):
        if symbol not in camera_facade:
            errors.append(f"GC0308 façade is missing required export: {symbol}")

    m5stack_mod = (firmware / "hw/m5stack/mod.rs").read_text(errors="ignore")
    m5stack_cameras = (firmware / "hw/m5stack/cameras/mod.rs").read_text(errors="ignore")
    if 'pub(crate) mod cameras;' not in m5stack_mod or 'pub(crate) mod gc0308;' not in m5stack_cameras:
        errors.append("M5Stack camera façade does not use the standard cameras/gc0308 module tree")

    for forbidden in ("crate::ui", "crate::services"):
        if forbidden in camera_source:
            errors.append(f"GC0308 hardware driver depends upward on {forbidden}")

    for forbidden in (
        "IO_MUX::PTR", "GPIO::PTR", "SYSTEM_PERIP_CLK_EN1",
        "SYSTEM_PERIP_RST_EN1", "start_sensor_xclk", "setup_cam_gpio_routing",
        "configure_cam_vsync_eof", "enable_lcd_cam_clocks", "ensure_lcd_clk_enabled",
    ):
        if forbidden in camera_source:
            errors.append(
                f"GC0308 must leave LCD_CAM/GPIO/IO_MUX ownership to ESP-HAL: {forbidden}"
            )

    m5stack_boot = (firmware / "boot/m5stack/mod.rs").read_text(errors="ignore")
    if m5stack_boot.find(".with_master_clock($peripherals.GPIO2)") > m5stack_boot.find(
        "camera::initialize_sensor("
    ):
        errors.append("CoreS3 GC0308 sensor init must occur after HAL master-clock ownership")



    return errors
