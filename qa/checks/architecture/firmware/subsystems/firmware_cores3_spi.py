from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

_TRANSPORT_VISIBILITY = "pub(in crate::hw::m5stack::storage::transport)"
_SPI2_REGISTER_TOKENS = (
    "SPI2_CLOCK_REG", "SPI2_USER_REG", "save_and_reclaim",
    "restore_spi_state", "0x6002_400C", "0x6002_4010",
)


@dataclass(frozen=True)
class _Cores3Sources:
    firmware: Path
    m5: Path
    spi_root: Path
    boot: str
    display: str
    state: str
    config: str
    gpio35: str
    hw_selector: str
    protocol_mod: str
    protocol_init: str
    protocol_wire: str


def _load_sources(root: Path) -> _Cores3Sources:
    firmware = root / "apps/kaskold-hardware/src"
    m5 = firmware / "hw/m5stack"
    spi_root = m5 / "spi_bus"
    protocol_root = m5 / "storage/transport/protocol"
    return _Cores3Sources(
        firmware=firmware,
        m5=m5,
        spi_root=spi_root,
        boot=(firmware / "boot/m5stack/mod.rs").read_text(errors="ignore"),
        display=(m5 / "display.rs").read_text(errors="ignore"),
        state=(spi_root / "state.rs").read_text(errors="ignore"),
        config=(spi_root / "config.rs").read_text(errors="ignore"),
        gpio35=(spi_root / "gpio35.rs").read_text(errors="ignore"),
        hw_selector=(firmware / "hw/mod.rs").read_text(errors="ignore"),
        protocol_mod=(protocol_root / "mod.rs").read_text(errors="ignore"),
        protocol_init=(protocol_root / "initialization.rs").read_text(errors="ignore"),
        protocol_wire=(protocol_root / "wire.rs").read_text(errors="ignore"),
    )


def _check_hw_and_boot_entry(src: _Cores3Sources) -> list[str]:
    errors: list[str] = []
    if "pub(crate) use m5stack::spi_bus::initialize as initialize_cores3_spi;" not in src.hw_selector:
        errors.append("CoreS3 SPI2 initializer must be exposed through the hw board-selection facade")
    if "$crate::hw::m5stack::" in src.boot:
        errors.append("CoreS3 boot macro must not expand through the private hw::m5stack module")
    return errors


def _check_sd_protocol_visibility(src: _Cores3Sources) -> list[str]:
    errors: list[str] = []
    if "pub(super) use initialization::initialize_card;" not in src.protocol_mod:
        errors.append("CoreS3 SD protocol facade lost initialize_card re-export")
    if f"{_TRANSPORT_VISIBILITY} fn initialize_card" not in src.protocol_init:
        errors.append("CoreS3 SD initialize_card visibility must be scoped to the transport ancestor")
    for helper in (
        "command_data", "finish_transaction", "read_exact",
        "require_success", "transfer_byte", "write_all",
    ):
        if f"{_TRANSPORT_VISIBILITY} fn {helper}" not in src.protocol_wire:
            errors.append(
                f"CoreS3 SD protocol helper visibility must reach transport siblings: {helper}"
            )
    return errors


def _check_boot_spi_contract(boot: str) -> list[str]:
    errors: list[str] = []
    if boot.count("Spi::new(") != 1 or "$peripherals.SPI2" not in boot:
        errors.append("CoreS3 boot must construct exactly one HAL-owned SPI2 bus")
    for marker in (
        ".with_sck($peripherals.GPIO36)",
        ".with_mosi($peripherals.GPIO37)",
        ".with_miso($peripherals.GPIO35)",
        "initialize_cores3_spi(spi, sd_cs)",
    ):
        if marker not in boot:
            errors.append(f"CoreS3 shared SPI2 boot contract missing: {marker}")
    return errors


def _check_boot_order(boot: str) -> list[str]:
    owner_at = boot.find("initialize_cores3_spi(spi, sd_cs)")
    sd_at = boot.find("m5stack::sd::initialize")
    display_at = boot.find("m5stack::display::initialize")
    if min(owner_at, sd_at, display_at) < 0 or not (owner_at < sd_at < display_at):
        return ["CoreS3 SPI2 owner must initialize before SD and display clients"]
    return []


def _check_display_client(display: str) -> list[str]:
    if "lcd_device(cs_pin)?" not in display:
        return ["CoreS3 display must be a client of the shared SPI2 owner"]
    return []


def _check_spi_state(state: str) -> list[str]:
    return [
        f"CoreS3 SPI2 serialization contract missing: {marker}"
        for marker in (
            "StaticCell<SharedBus>", "AtomicPtr<SharedBus>", "try_borrow_mut()",
            "CoreS3 SPI2 bus re-entry", "frequency_hz: Cell<u32>",
        )
        if marker not in state
    ]


def _check_interrupt_masking(state: str) -> list[str]:
    if "critical_section::with" in state:
        return ["CoreS3 SPI2 transfers must not mask interrupts for whole SD/LCD transactions"]
    return []


def _check_spi_config(config: str) -> list[str]:
    return [
        f"CoreS3 SPI2 configuration contract missing: {marker}"
        for marker in ("ensure_frequency", "apply_config", "current_hz.get() == frequency_hz")
        if marker not in config
    ]


def _check_gpio35(gpio35: str) -> list[str]:
    return [
        f"CoreS3 GPIO35 ownership contract missing: {marker}"
        for marker in ("select_lcd_dc", "select_sd_miso", "GPIO_ENABLE1_W1TC")
        if marker not in gpio35
    ]


def _check_retired_paths(root: Path, m5: Path) -> list[str]:
    retired = (
        m5 / "storage/transport/registers.rs",
        m5 / "storage/transport/gpio.rs",
        m5 / "storage/transport/bitbang",
    )
    return [
        f"retired CoreS3 SPI2-reclaim path returned: {path.relative_to(root)}"
        for path in retired
        if path.exists()
    ]


def _check_register_bypass(root: Path, m5: Path, spi_root: Path) -> list[str]:
    errors: list[str] = []
    for path in m5.rglob("*.rs"):
        if path.is_relative_to(spi_root):
            continue
        source = path.read_text(errors="ignore")
        for forbidden in _SPI2_REGISTER_TOKENS:
            if forbidden in source:
                errors.append(
                    f"CoreS3 module bypasses shared SPI2 ownership with {forbidden}: "
                    f"{path.relative_to(root)}"
                )
    return errors


def _check_interrupt_handlers(root: Path, firmware: Path) -> list[str]:
    errors: list[str] = []
    for path in firmware.rglob("*.rs"):
        source = path.read_text(errors="ignore")
        if "spi_bus" in source and ("#[handler]" in source or "#[esp_hal::handler]" in source):
            errors.append(f"interrupt handler must not access CoreS3 shared SPI2: {path.relative_to(root)}")
    return errors


def check_cores3_spi_ownership(root: Path) -> list[str]:
    src = _load_sources(root)
    return [
        *_check_hw_and_boot_entry(src),
        *_check_sd_protocol_visibility(src),
        *_check_boot_spi_contract(src.boot),
        *_check_boot_order(src.boot),
        *_check_display_client(src.display),
        *_check_spi_state(src.state),
        *_check_spi_config(src.config),
        *_check_interrupt_masking(src.state),
        *_check_gpio35(src.gpio35),
        *_check_retired_paths(root, src.m5),
        *_check_register_bypass(root, src.m5, src.spi_root),
        *_check_interrupt_handlers(root, src.firmware),
    ]
