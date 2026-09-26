from __future__ import annotations

from pathlib import Path
import re


def _check_hardware_roots(root: Path) -> list[str]:
    errors: list[str] = []
    firmware = root / "apps/kaskold-hardware/src"
    hardware = firmware / "hw"
    boot = firmware / "boot"

    expected_hw_entries = {"mod.rs", "shared", "m5stack"}
    actual_hw_entries = {path.name for path in hardware.iterdir()}
    if actual_hw_entries != expected_hw_entries:
        errors.append(
            "firmware hardware root must contain only the selector, shared policy, and CoreS3 adapter: "
            f"expected {sorted(expected_hw_entries)}, got {sorted(actual_hw_entries)}"
        )

    for required in (
        hardware / "shared/mod.rs",
        hardware / "m5stack/mod.rs",
        hardware / "shared/storage/fat32/mod.rs",
        hardware / "m5stack/storage/mod.rs",
        hardware / "m5stack/profile.rs",
        boot / "shared/mod.rs",
        boot / "m5stack/mod.rs",
    ):
        if not required.is_file():
            errors.append(f"required CoreS3 boundary module is missing: {required.relative_to(root)}")

    selector = (hardware / "mod.rs").read_text(errors="ignore")
    for contract in (
        "pub(crate) mod shared;",
        "mod m5stack;",
        "pub(crate) use m5stack::{battery, camera, display, pmu, sdcard, sound, touch};",
        "pub(crate) const ACTIVE_BOARD_NAME: &str = m5stack::BOARD_NAME;",
    ):
        if contract not in selector:
            errors.append(f"hardware selector lost stable CoreS3 facade: {contract}")
    if "#[path" in selector:
        errors.append("hardware selector must use the standard Rust module hierarchy")

    shared_source = "\n".join(
        path.read_text(errors="ignore") for path in (hardware / "shared").rglob("*.rs")
    )
    if re.search(r'#\[cfg\([^\n]*feature\s*=\s*"m5stack', shared_source):
        errors.append("shared hardware code contains a concrete-board feature gate")
    if "crate::hw::m5stack" in shared_source:
        errors.append("shared hardware depends upward on the concrete CoreS3 adapter")

    boot_shared = "\n".join(
        path.read_text(errors="ignore") for path in (boot / "shared").rglob("*.rs")
    )
    if re.search(r'#\[cfg\([^\n]*feature\s*=\s*"m5stack', boot_shared):
        errors.append("shared boot policy contains a concrete-board feature gate")

    for stale_name in (
        "camera", "display", "power", "sound", "storage", "touch", "board",
        "registers", "decode_core", "display_support", "diagnostics",
    ):
        stale = hardware / stale_name
        if stale.exists():
            errors.append(f"interleaved legacy hardware root must not exist: {stale.relative_to(root)}")
    return errors


def check(root: Path) -> list[str]:
    errors: list[str] = _check_hardware_roots(root)
    firmware_main = root / "apps/kaskold-hardware/src/main.rs"
    boot_root = root / "apps/kaskold-hardware/src/boot"
    runtime_root = root / "apps/kaskold-hardware/src/runtime"

    inactive_board_map = root / "apps/kaskold-hardware/src/hw/board/board.rs"
    hw_mod = root / "apps/kaskold-hardware/src/hw/mod.rs"
    if inactive_board_map.exists():
        errors.append("inactive duplicate board pin map must not return")
    if hw_mod.exists() and re.search(r"(?m)^pub mod board;", hw_mod.read_text(errors="ignore")):
        errors.append("hardware module must not export the inactive board pin map")

    required_main_modules = (
        boot_root / "mod.rs",
        boot_root / "shared/security.rs",
        boot_root / "m5stack/mod.rs",
        runtime_root / "event_loop/mod.rs",
        runtime_root / "event_loop/touch.rs",
        runtime_root / "event_loop/dispatch.rs",
        runtime_root / "event_loop/frame.rs",
        runtime_root / "event_loop/camera.rs",
        runtime_root / "event_loop/runner.rs",
        runtime_root / "touch_dispatch.rs",
        runtime_root / "power_state.rs",
    )
    for required in required_main_modules:
        if not required.exists():
            errors.append(f"required staged main module is missing: {required.relative_to(root)}")

    if firmware_main.exists():
        source = firmware_main.read_text(errors="ignore")
        if len(source.splitlines()) > 260:
            errors.append(f"firmware main.rs exceeds completed 260-line limit: {len(source.splitlines())} lines")
        for required in (
            "#[main]\nfn main() -> !",
            "runtime::secret_state::initialize()",
            "runtime::event_loop::runner::run(",
            "pub use runtime::power_state::halt_forever;",
            "boot::m5stack::initialize!(peripherals, delay)",
        ):
            if required not in source:
                errors.append(f"firmware main lost staged boundary: {required}")
        if "loop {" in source:
            errors.append("firmware main.rs must delegate the outer loop to runtime::event_loop")
        for moved in (
            "init_pmu_m5", "init_sd_card_m5", "touch_zones", "handle_wake", "handle_idle",
            "continue_without_display", "cam_tune_apply_all", "cam_tune_apply_gc0308",
        ):
            if re.search(rf"(?m)^fn\s+{moved}", source):
                errors.append(f"firmware main retains extracted helper: {moved}")
        boot_markers = (
            "ESP32-S3 initialization", "Hardware self-tests", "Verify firmware integrity",
            "Boot into main application", "Main loop",
        )
        positions = [source.find(marker) for marker in boot_markers]
        if any(position < 0 for position in positions) or positions != sorted(positions):
            errors.append("firmware main top-level boot and loop order changed")

    limits = {
        boot_root / "mod.rs": 80,
        boot_root / "shared/security.rs": 80,
        boot_root / "m5stack/mod.rs": 300,
        runtime_root / "event_loop/mod.rs": 100,
        runtime_root / "event_loop/touch.rs": 140,
        runtime_root / "event_loop/dispatch.rs": 260,
        runtime_root / "event_loop/frame.rs": 140,
        runtime_root / "event_loop/camera.rs": 140,
        runtime_root / "event_loop/runner.rs": 100,
        runtime_root / "touch_dispatch.rs": 100,
        runtime_root / "power_state.rs": 180,
    }
    for path, maximum in limits.items():
        if path.exists() and len(path.read_text(errors="ignore").splitlines()) > maximum:
            errors.append(f"staged main module exceeds SRP limit: {path.relative_to(root)}")

    m5stack_boot = boot_root / "m5stack/mod.rs"
    if m5stack_boot.exists():
        boot_source = m5stack_boot.read_text(errors="ignore")
        if len(re.findall(r"macro_rules!\s+initialize", boot_source)) != 1:
            errors.append("CoreS3 boot must have exactly one ownership-preserving initializer")

    power_state = runtime_root / "power_state.rs"
    if power_state.exists() and not re.search(r"(?m)^pub\s+fn\s+halt_forever\s*\(", power_state.read_text(errors="ignore")):
        errors.append("runtime power_state must own the public halt implementation")
    return errors
