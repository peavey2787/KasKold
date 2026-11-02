"""Warning-free feature and generated-source contracts for signer firmware."""

from __future__ import annotations

import re
from pathlib import Path

HIL_DEAD_CODE_ALLOW = '#![cfg_attr(feature = "hardware-tests", allow(dead_code))]'
HIL_DEAD_CODE_OWNERS = (
    "boot/mod.rs", "controllers.rs", "crypto/mod.rs", "hw/mod.rs",
    "runtime/mod.rs", "services/mod.rs", "ui/mod.rs", "wallet/mod.rs",
)
HARDWARE_SYMBOL_GATES = (
    ("apps/kaskold-hardware/src/runtime/signing.rs", "pub use workflow::handle_signing_operation_step;"),
    ("apps/kaskold-hardware/src/services/entropy/mod.rs", "pub use ambient::stage_touch as stage_ambient_touch;"),
    ("apps/kaskold-hardware/src/services/entropy/mod.rs", "pub use touch::touch_timestamp;"),
    ("apps/kaskold-hardware/src/runtime/interactions/camera_loop.rs", "pub use cycle::run_camera_cycle;"),
    ("apps/kaskold-hardware/src/runtime/interactions/camera_loop.rs", "pub use state::CameraSessionState;"),
)
RETIRED_SPI2_STATE = (
    "save_and_reclaim", "restore_spi_state", "USE_HW_SPI2",
    "set_hw_spi2_enabled", "SPI2_CLOCK_REG", "SPI2_USER_REG",
)


def _read(root: Path, relative: str) -> str:
    return (root / relative).read_text(encoding="utf-8")


def _require(errors: list[str], condition: bool, message: str) -> None:
    if not condition:
        errors.append(message)


def _check_main_lint_attributes(root: Path, errors: list[str]) -> None:
    main = _read(root, "apps/kaskold-hardware/src/main.rs")
    _require(
        errors,
        '#![deny(unused_imports)]' in main,
        "signer-firmware: unused imports must remain denied in every feature set",
    )
    _require(
        errors,
        '#![warn(dead_code)]' in main,
        "signer-firmware: dead-code diagnostics must remain enabled in every feature set",
    )
    _require(
        errors,
        HIL_DEAD_CODE_ALLOW not in main,
        "signer-firmware: hardware-tests dead-code suppression must not be crate-wide",
    )


def _check_hil_dead_code_owners(root: Path, errors: list[str]) -> None:
    for relative in HIL_DEAD_CODE_OWNERS:
        _require(
            errors,
            HIL_DEAD_CODE_ALLOW in _read(root, f"apps/kaskold-hardware/src/{relative}"),
            f"signer-firmware: hardware-tests dead-code suppression must be owned by {relative}",
        )


def _check_main_allowances(root: Path, errors: list[str]) -> None:
    main = _read(root, "apps/kaskold-hardware/src/main.rs")
    _require(
        errors,
        not re.search(r"#!\s*\[.*allow\s*\(\s*dead_code\s*\).*\]", main),
        "signer-firmware: crate-wide dead_code allowances are forbidden",
    )
    _require(
        errors,
        "allow(unused_imports)" not in main,
        "signer-firmware: unused imports must never be allowed, including hardware-tests",
    )


def _check_log_macro_definition(root: Path, errors: list[str]) -> None:
    main = _read(root, "apps/kaskold-hardware/src/main.rs")
    _require(
        errors,
        0 <= main.find("macro_rules! log") < main.find("mod boot;"),
        "signer-firmware: log! must be defined before the module tree",
    )
    _require(
        errors,
        'if false {\n            let _ = core::format_args!($($arg)+);' in main,
        "signer-firmware: silent log! must remain an expression and consume arguments",
    )


def _check_m5stack_test_profile(root: Path, errors: list[str]) -> None:
    main = _read(root, "apps/kaskold-hardware/src/main.rs")
    _require(
        errors,
        '#[cfg_attr(feature = "hardware-tests", allow(unused_variables))]\n'
        '    #[cfg(feature = "m5stack")]\n'
        '    let (i2c, mut boot_display, dvp_camera_opt, cam_dma_buf_opt,' in main,
        "M5Stack hardware-test profile must locally suppress only intentionally retained peripheral owners",
    )


def _check_hardware_only_symbols(root: Path, errors: list[str]) -> None:
    for relative, symbol in HARDWARE_SYMBOL_GATES:
        source = _read(root, relative)
        hardware_only_gate = f'#[cfg(not(feature = "hardware-tests"))]\n{symbol}'
        workflow_loop_gate = (
            f'#[cfg(all(not(feature = "hardware-tests"), not(feature = "workflow-test-auto")))]\n{symbol}'
        )
        _require(
            errors,
            hardware_only_gate in source or workflow_loop_gate in source,
            f"hardware-test builds must not expose production-only facade symbol: {symbol}",
        )


def _check_runtime_facades(root: Path, errors: list[str]) -> None:
    runtime_facade = _read(root, "apps/kaskold-hardware/src/runtime/mod.rs")
    input_facade = _read(root, "apps/kaskold-hardware/src/runtime/input.rs")
    sd_facade = _read(root, "apps/kaskold-hardware/src/runtime/interactions/sd/mod.rs")
    _require(
        errors,
        '#[cfg(not(feature = "hardware-tests"))]\npub(crate) mod event_loop;' in runtime_facade,
        "hardware-test builds must not compile the production event-loop facade",
    )
    _require(
        errors,
        'pub use kaskold_hardware_core::screen_flow::HandlerGroup;' in input_facade
        and '#[cfg(not(feature = "hardware-tests"))]\npub use kaskold_hardware_core::screen_flow::HandlerGroup;' not in input_facade,
        "hardware-test builds compile controllers/navigation and therefore require HandlerGroup routing",
    )
    _require(
        errors,
        'pub use common::context::SdTouchContext;' in sd_facade
        and 'pub use common::routing::handle_sd_touch;' in sd_facade
        and '#[cfg(not(feature = "hardware-tests"))]\npub use common::context::SdTouchContext;' not in sd_facade
        and '#[cfg(not(feature = "hardware-tests"))]\npub use common::routing::handle_sd_touch;' not in sd_facade,
        "hardware-test builds compile onboarding and therefore require SD touch routing",
    )
    _check_hardware_only_symbols(root, errors)


def _check_log_imports(root: Path, errors: list[str]) -> None:
    for source_path in (root / "apps/kaskold-hardware/src").rglob("*.rs"):
        source = source_path.read_text(encoding="utf-8")
        for use_decl in re.finditer(r"(?ms)^use\s+.*?;[ \t]*(?:\n|$)", source):
            _require(
                errors,
                re.search(r"\blog\b", use_decl.group(0)) is None,
                f"{source_path.relative_to(root)}: log! is lexically visible; do not import it",
            )


def _check_boot_palette_import(root: Path, errors: list[str]) -> None:
    boot_display = _read(root, "apps/kaskold-hardware/src/ui/display/boot.rs")
    palette_import = re.search(r"(?ms)^use super::\{.*?\};", boot_display)
    _require(
        errors,
        palette_import is not None and "COLOR_ORANGE" in palette_import.group(0),
        "ui/display/boot.rs: signature warning color must be imported from the display palette",
    )


def _check_fat32_facade(root: Path, errors: list[str]) -> None:
    fat32_facade = _read(root, "apps/kaskold-hardware/src/hw/shared/storage/fat32/mod.rs")
    fat32_directory_helpers = _read(root, "apps/kaskold-hardware/src/hw/shared/storage/fat32/directory/helpers.rs")
    _require(
        errors,
        "pub(super) use policy::detect_fat32_partition;" not in fat32_facade,
        "hw/shared/storage/fat32/mod.rs: internal partition detection must not leak through the facade",
    )
    _require(
        errors,
        "use super::super::policy::detect_fat32_partition;" in fat32_directory_helpers,
        "hw/shared/storage/fat32/directory/helpers.rs: import partition detection directly from its owner",
    )


def _check_build_matrix_and_lint(root: Path, errors: list[str]) -> None:
    matrix_runner = _read(root, "tools/build/firmware/matrix_runner.py")
    _require(
        errors,
        'if operation == "check":' in matrix_runner and '"-Dwarnings"' in matrix_runner,
        "firmware build matrix must reject every crate warning",
    )
    lint_runner = _read(root, "qa/checks/firmware/check_firmware_lints.py")
    _require(
        errors,
        '"-D", "dead_code"' in lint_runner,
        "firmware lint matrix must reject dead code",
    )


def _check_hash_generator(root: Path, errors: list[str]) -> None:
    hash_generator = _read(root, "tools/firmware/gen_hash/output.rs")
    _require(
        errors,
        "SigningIdentity::Production => sign_firmware_hash" in hash_generator
        and "SigningIdentity::Development => sign_test_firmware_hash" in hash_generator,
        "firmware hash generator must keep release signatures production-only",
    )
    _require(
        errors,
        "FIRMWARE_HASH_HEX" not in hash_generator,
        "firmware hash generator must not recreate the unused hex constant",
    )


def _check_hash_convergence(root: Path, errors: list[str]) -> None:
    dockerfile = _read(root, "Dockerfile")
    hash_builder = _read(root, "tools/build/firmware/build_with_hash.sh")
    _require(
        errors,
        "pub static EXPECTED_FIRMWARE_HASH: [u8; 32] = [" in hash_builder
        and "--read-generated-hash" in hash_builder
        and "FIRMWARE_HASH_HEX" not in hash_builder,
        "firmware convergence helper must require the rodata-static EXPECTED_FIRMWARE_HASH declaration and never depend on the retired hex constant",
    )
    lock_reconciler = _read(root, "tools/build/firmware/reconcile_tools_lock.py")
    _require(
        errors,
        "--offline" in lock_reconciler
        and "--locked" in lock_reconciler
        and "introduced external package identities" in lock_reconciler
        and "network fallback is forbidden" in lock_reconciler,
        "firmware hash tools lock repair must remain offline, locked-verified, and identity-monotonic",
    )
    _require(
        errors,
        "reconcile_tools_lock.py" in hash_builder
        and 'cp -p "$TOOLS_LOCK_BACKUP" "$TOOLS_LOCK"' in hash_builder,
        "firmware convergence helper must reconcile and restore the independent tools lock",
    )
    _require(
        errors,
        "build_with_hash.sh --read-generated-hash" in dockerfile
        and "FIRMWARE_HASH_HEX" not in dockerfile,
        "Docker convergence must use the canonical generated hash reader",
    )


def _check_sound_chip_id(root: Path, errors: list[str]) -> None:
    sound_m5 = _read(root, "apps/kaskold-hardware/src/hw/m5stack/sound.rs")
    _require(
        errors,
        '#[cfg(not(feature = "silent"))]\n    {\n        let chip_id = '
        in sound_m5,
        "sound_m5.rs: logging-only AW88298 chip ID must not exist in silent builds",
    )
    _require(
        errors,
        'let chip_id = ((buf[0] as u16) << 8) | buf[1] as u16;\n'
        '    #[cfg(not(feature = "silent"))]'
        not in sound_m5,
        "sound_m5.rs: chip ID binding must remain inside its non-silent block",
    )


def _check_battery_present_field(root: Path, errors: list[str]) -> None:
    battery_path = "apps/kaskold-hardware/src/hw/m5stack/power/battery.rs"
    _require(
        errors,
        "pub present: bool" not in _read(root, battery_path),
        f"{battery_path}: unread battery-present field must not return",
    )


def _check_display_security_retired(root: Path, errors: list[str]) -> None:
    display_facade = _read(root, "apps/kaskold-hardware/src/ui/display/mod.rs")
    _require(
        errors,
        "password_strength" not in display_facade
        and not (root / "apps/kaskold-hardware/src/ui/display/security.rs").exists(),
        "ui/display: retired password-strength helper must not return as dead production code",
    )


def _check_prop_fonts(root: Path, errors: list[str]) -> None:
    prop_fonts = _read(root, "apps/kaskold-hardware/src/ui/prop_fonts.rs")
    _require(
        errors,
        "OSWALD_SB_16" not in prop_fonts,
        "ui/prop_fonts.rs: unused Oswald 16px asset must not return",
    )
    for required in ("pub fn draw_prop_text", "pub fn draw_prop_text_opaque", "LATO_22_HEIGHT"):
        _require(
            errors,
            required in prop_fonts,
            f"ui/prop_fonts.rs: warning cleanup removed required {required}",
        )


def _check_mmio_helpers(root: Path, errors: list[str]) -> None:
    mmio = _read(root, "apps/kaskold-hardware/src/hw/shared/mmio.rs")
    lockdown = _read(root, "apps/kaskold-hardware/src/hw/shared/lockdown.rs")
    for helper in ("read", "write", "set_bits", "clear_bits"):
        _require(
            errors,
            f"pub(crate) unsafe fn {helper}" in mmio,
            f"hw/shared/mmio.rs: missing shared MMIO helper {helper}",
        )
    _require(
        errors,
        "use super::mmio::{clear_bits, read, set_bits, write};" in lockdown,
        "shared lockdown must consume the shared MMIO owner",
    )


def _check_gc0308_bus(root: Path, errors: list[str]) -> None:
    gc0308_bus = _read(root, "apps/kaskold-hardware/src/hw/m5stack/cameras/gc0308/bus.rs")
    _require(
        errors,
        "pub fn read_reg" not in gc0308_bus,
        "camera_gc0308/bus.rs: unused diagnostic read wrapper must not return",
    )


def _check_touch_contracts(root: Path, errors: list[str]) -> None:
    touch_types = _read(root, "apps/kaskold-hardware/src/hw/shared/touch.rs")
    touch_m5 = _read(root, "apps/kaskold-hardware/src/hw/m5stack/touch/mod.rs")
    shared_touch = _read(root, "crates/kaskold-hardware-core/src/input/touch.rs")
    _require(
        errors,
        "Two" not in touch_types
        and "decode_rotated_single_touch" in touch_m5
        and "if points != 1" in shared_touch
        and "return TouchState::NoTouch;" in shared_touch,
        "multi-touch must be ignored at the M5Stack driver boundary",
    )
    for variant in ("Hold {", "SwipeUp", "SwipeDown"):
        _require(
            errors,
            variant not in touch_m5,
            f"M5Stack touch action retains an unconstructed variant: {variant}",
        )


def _check_battery_voltage_field(root: Path, errors: list[str]) -> None:
    battery_m5 = _read(root, "apps/kaskold-hardware/src/hw/m5stack/power/battery.rs")
    _require(
        errors,
        "pub voltage_mv:" not in battery_m5,
        "battery_m5.rs: raw voltage must remain an internal percentage input",
    )


def _check_sd_absence_owner(root: Path, errors: list[str]) -> None:
    fat32_types = _read(root, "apps/kaskold-hardware/src/hw/shared/storage/fat32/types.rs")
    m5_storage = _read(root, "apps/kaskold-hardware/src/hw/m5stack/storage/mod.rs")
    _require(
        errors,
        "None" not in fat32_types
        and "manual_unlock_session_card_type() -> Option<SdCardType>" in m5_storage,
        "SD-card absence must remain private to the CoreS3 transport/session owner",
    )


def _check_retired_sd_transport(root: Path, errors: list[str]) -> None:
    m5_transport_root = root / "apps/kaskold-hardware/src/hw/m5stack/storage/transport"
    m5_transport = _read(root, "apps/kaskold-hardware/src/hw/m5stack/storage/transport/mod.rs")
    _require(
        errors,
        "mod bitbang;" not in m5_transport
        and not (m5_transport_root / "bitbang").exists()
        and not (m5_transport_root / "registers.rs").exists()
        and not (m5_transport_root / "gpio.rs").exists(),
        "M5Stack retired raw/bitbang SD transport must not return",
    )
    transport_source = "\n".join(
        path.read_text(encoding="utf-8") for path in m5_transport_root.rglob("*.rs")
    )
    for stale in RETIRED_SPI2_STATE:
        _require(
            errors,
            stale not in transport_source,
            f"M5Stack SD transport retains retired SPI2 ownership state: {stale}",
        )


def _check_shared_spi_owner(root: Path, errors: list[str]) -> None:
    m5_spi_state = _read(root, "apps/kaskold-hardware/src/hw/m5stack/spi_bus/state.rs")
    m5_spi_config = _read(root, "apps/kaskold-hardware/src/hw/m5stack/spi_bus/config.rs")
    m5_spi_gpio35 = _read(root, "apps/kaskold-hardware/src/hw/m5stack/spi_bus/gpio35.rs")
    _require(
        errors,
        "StaticCell<SharedBus>" in m5_spi_state
        and "AtomicPtr<SharedBus>" in m5_spi_state
        and "ensure_frequency" in m5_spi_config
        and "apply_config" in m5_spi_config
        and "GPIO_ENABLE1_W1TC" in m5_spi_gpio35,
        "CoreS3 shared SPI2 owner must remain explicit and fail-closed",
    )


def _check_app_data_camera_state(root: Path, errors: list[str]) -> None:
    app_data = _read(root, "apps/kaskold-hardware/src/runtime/data.rs")
    _require(
        errors,
        "pub camera: CameraState" not in app_data and "CameraState" not in app_data,
        "retired camera-tuning state must not return to AppData",
    )


def _check_camera_session_state(root: Path, errors: list[str]) -> None:
    camera_session = _read(root, "apps/kaskold-hardware/src/runtime/interactions/camera_loop/state.rs")
    event_loop = _read(root, "apps/kaskold-hardware/src/runtime/event_loop/mod.rs")
    _require(
        errors,
        "sensor_is_ov2640" not in camera_session and "is_ov2640" not in camera_session,
        "fixed CoreS3 camera sessions must not retain runtime sensor-selection state",
    )
    _require(
        errors,
        "let mut camera_session = $crate::runtime::interactions::camera_loop::CameraSessionState::new();"
        in event_loop,
        "CoreS3-family event loop must construct a sensor-neutral camera session",
    )


def _check_orphaned_documentation(root: Path, errors: list[str]) -> None:
    lockdown = _read(root, "apps/kaskold-hardware/src/hw/shared/lockdown.rs")
    signing_facade = _read(root, "apps/kaskold-hardware/src/runtime/signing.rs")
    verify_types = _read(root, "apps/kaskold-hardware/src/services/verify/types.rs")
    _require(
        errors,
        "/// disables the JTAG bridge\n\n" not in lockdown,
        "hw/shared/lockdown.rs: orphaned JTAG documentation must not return",
    )
    _require(
        errors,
        '#[cfg(not(feature = "skip-tests"))]\n\npub use qr::cycle_signed_qr;'
        not in signing_facade,
        "runtime/signing.rs: cfg attributes must stay attached to their imports",
    )
    _require(
        errors,
        "/// Equivalent address on data bus (for reading)" not in verify_types,
        "services/verify/types.rs: orphaned mapped-address documentation must not return",
    )


def _check_fat32_policy_and_menu(root: Path, errors: list[str]) -> None:
    fat32_policy = _read(root, "apps/kaskold-hardware/src/hw/shared/storage/fat32/policy.rs")
    menu = _read(root, "apps/kaskold-hardware/src/runtime/input/menu.rs")
    navigation_core = _read(root, "crates/kaskold-hardware-core/src/input/navigation.rs")
    _require(
        errors,
        "left.eq_ignore_ascii_case(&right)" in fat32_policy,
        "FAT32 name matching must use the standard ASCII-insensitive comparison",
    )
    _require(
        errors,
        "kaskold_hardware_core::input::navigation" in menu
        and "menu.items[..count].copy_from_slice(&labels[..count]);" in navigation_core,
        "fixed-capacity menu labels must remain host-tested and use slice copying",
    )


def _check_hardware_test_results(root: Path, errors: list[str]) -> None:
    hardware_tests = _read(root, "apps/kaskold-hardware/src/services/unit_tests/hardware.rs")
    boot_tests = _read(root, "apps/kaskold-hardware/src/runtime/unit_tests/boot.rs")
    _require(
        errors,
        "passed: [bool; HardwareTest::COUNT]" in hardware_tests
        and "pub all_passed: bool" not in hardware_tests,
        "hardware self-test results must use indexed outcomes instead of an excessive bool struct",
    )
    _require(
        errors,
        "test_results.all_passed()" in boot_tests
        and "test_results.passed(HardwareTest::Sram)" in boot_tests,
        "boot self-tests must consume the typed hardware result API",
    )


def _check_camera_cycle_and_qr(root: Path, errors: list[str]) -> None:
    camera_cycle = _read(root, "apps/kaskold-hardware/src/runtime/interactions/camera_loop/cycle.rs")
    signing_facade = _read(root, "apps/kaskold-hardware/src/runtime/signing.rs")
    _require(
        errors,
        "drop(platform);" not in camera_cycle
        and "let buffers = {" in camera_cycle,
        "camera cycle must end platform borrows by scope, not drop a non-Drop value",
    )
    _require(
        errors,
        (
            '#[cfg(not(feature = "hardware-tests"))]\npub use qr::cycle_signed_qr;' in signing_facade
            or '#[cfg(all(not(feature = "hardware-tests"), not(feature = "workflow-test-auto")))]\npub use qr::cycle_signed_qr;' in signing_facade
        ),
        "runtime/signing.rs: signed QR cycling must remain available in skip-tests releases",
    )


def _check_event_loop_runner_gate(root: Path, errors: list[str]) -> None:
    event_loop_facade = _read(root, "apps/kaskold-hardware/src/runtime/event_loop/mod.rs")
    _require(
        errors,
        '#[cfg(not(feature = "hardware-tests"))]\npub(crate) mod runner;'
        in event_loop_facade,
        "runtime/event_loop: ownership runner must stay out of hardware-test-only builds",
    )


def _check_event_runner(root: Path, errors: list[str]) -> None:
    main = _read(root, "apps/kaskold-hardware/src/main.rs")
    event_runner = _read(root, "apps/kaskold-hardware/src/runtime/event_loop/runner.rs")
    _require(
        errors,
        "fn main() -> ! {\n    firmware_main()\n}" in main
        and "clippy::cognitive_complexity" not in main,
        "firmware entrypoint must isolate ESP-HAL expansion without a main.rs lint exception",
    )
    _require(
        errors,
        "#[allow(clippy::cognitive_complexity)]\npub(crate) fn run(" in event_runner
        and "super::run!(" in event_runner,
        "event-loop macro complexity exception must stay on the focused ownership runner",
    )
    _require(
        errors,
        "sd_card_type: Option<SdCardType>," in event_runner,
        "event-loop ownership runner must preserve optional SD-card availability",
    )


def _check_master_runner_features(root: Path, errors: list[str]) -> None:
    commands = _read(root, "qa/linux/runner/commands.sh")
    for features in ("m5stack,verbose-boot", "m5stack-lite,verbose-boot"):
        _require(
            errors,
            f"--features {features}" in commands,
            f"master runner: missing signer-firmware compile configuration {features}",
        )


def check_firmware_warning_contract(root: Path, errors: list[str]) -> None:
    """Keep every production feature set warning-free without broad allowances."""
    _check_main_lint_attributes(root, errors)
    _check_hil_dead_code_owners(root, errors)
    _check_main_allowances(root, errors)
    _check_log_macro_definition(root, errors)
    _check_m5stack_test_profile(root, errors)
    _check_runtime_facades(root, errors)
    _check_log_imports(root, errors)
    _check_boot_palette_import(root, errors)
    _check_fat32_facade(root, errors)
    _check_build_matrix_and_lint(root, errors)
    _check_hash_generator(root, errors)
    _check_hash_convergence(root, errors)
    _check_sound_chip_id(root, errors)
    _check_battery_present_field(root, errors)
    _check_display_security_retired(root, errors)
    _check_prop_fonts(root, errors)
    _check_mmio_helpers(root, errors)
    _check_gc0308_bus(root, errors)
    _check_touch_contracts(root, errors)
    _check_battery_voltage_field(root, errors)
    _check_sd_absence_owner(root, errors)
    _check_retired_sd_transport(root, errors)
    _check_shared_spi_owner(root, errors)
    _check_app_data_camera_state(root, errors)
    _check_camera_session_state(root, errors)
    _check_orphaned_documentation(root, errors)
    _check_fat32_policy_and_menu(root, errors)
    _check_hardware_test_results(root, errors)
    _check_camera_cycle_and_qr(root, errors)
    _check_event_loop_runner_gate(root, errors)
    _check_event_runner(root, errors)
    _check_master_runner_features(root, errors)
