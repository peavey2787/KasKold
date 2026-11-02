from __future__ import annotations

from pathlib import Path
import hashlib
import re

REQUIRED_SCREEN_MODULES = (
    "components/mod.rs", "components/qr_renderer.rs",
    "navigation/mod.rs",
    "dialogs/mod.rs", "dialogs/destructive.rs", "dialogs/firmware_update.rs", "dialogs/popups.rs", "dialogs/status.rs",
    "security.rs",
    "device/mod.rs", "device/camera.rs", "device/settings.rs",
    "signing/mod.rs", "signing/confirmation.rs", "signing/errors.rs", "signing/message_preview.rs",
    "signing/message_result.rs", "signing/message_source.rs", "signing/transaction_guide.rs",
    "signing/progress.rs", "signing/qr.rs", "signing/transaction_review/mod.rs",
    "storage/mod.rs", "storage/sd/mod.rs",
    "storage/steganography/mod.rs", "storage/steganography/jpeg.rs", "storage/steganography/text.rs",
    "storage/steganography/prompts.rs",
    "wallet/mod.rs", "wallet/address/mod.rs", "wallet/keyboard.rs",
    "wallet/multisig/mod.rs", "wallet/multisig/setup.rs", "wallet/multisig/result.rs",
    "wallet/qr_export/mod.rs", "wallet/seed_generation.rs", "wallet/seed_management.rs", "wallet/seed_slots.rs",
)

DELEGATED_SCREEN_METHODS = {
    "device/settings.rs": ("draw_level_settings(",),
    "signing/confirmation.rs": ("draw_send_confirmation_layout(",),
    "wallet/qr_export/seed.rs": ("draw_seed_qr_payload(",),
}

REQUIRED_REDRAW_MODULES = (
    "covenant.rs", "device.rs", "export.rs", "messages.rs", "multisig.rs", "navigation.rs",
    "settings.rs", "signing.rs", "stego.rs", "storage.rs", "system.rs", "wallet.rs",
)

MNEMONIC_LIMITS = {
    "mod.rs": 80,
    "generation.rs": 120,
    "dice.rs": 180,
    "touch.rs": 100,
    "word_input.rs": 160,
    "validation.rs": 100,
}

SEED_LIMITS = {
    "mod.rs": 80,
    "account_key_store.rs": 100,
    "slot.rs": 180,
    "source.rs": 100,
    "manager.rs": 220,
    "mnemonic_store.rs": 100,
    "matching.rs": 100,
    "network.rs": 120,
    "seedqr.rs": 180,
    "passphrase.rs": 160,
    "protection.rs": 180,
}


def _read_firmware_source(root: Path) -> str:
    return "\n".join(
        path.read_text(errors="ignore")
        for path in (root / "apps/kaskold-hardware/src").rglob("*.rs")
    )


def _required_screen_module_errors(root: Path, screens_root: Path) -> list[str]:
    return [
        f"required firmware screen module is missing: {required.relative_to(root)}"
        for required in (screens_root / name for name in REQUIRED_SCREEN_MODULES)
        if not required.exists()
    ]


def _screen_facade_errors(root: Path, screens_facade: Path) -> list[str]:
    errors: list[str] = []
    if screens_facade.exists():
        screens_facade_source = screens_facade.read_text(errors="ignore")
        if len(screens_facade_source.splitlines()) > 80:
            errors.append("ui/screens.rs must remain a small stable module root")
        if re.search(r"\bimpl(?:<[^>]+>)?\s+BootDisplay", screens_facade_source):
            errors.append("ui/screens.rs must not contain screen implementations")
    return errors


def _screen_size_errors(root: Path, screen_production_files: list[Path]) -> list[str]:
    errors: list[str] = []
    for path in screen_production_files:
        line_count = len(path.read_text(errors="ignore").splitlines())
        if line_count > 600:
            errors.append(
                f"firmware screen module exceeds 600-line SRP limit: "
                f"{path.relative_to(root)} ({line_count} lines)"
            )
    return errors


def _shared_component_errors(root: Path, screens_root: Path) -> list[str]:
    errors: list[str] = []
    components_root = screens_root / "components"
    components_source = "\n".join(
        path.read_text(errors="ignore") for path in components_root.rglob("*.rs")
    ) if components_root.exists() else ""
    for helper in ("draw_level_settings", "update_level_bar", "draw_send_confirmation_layout", "draw_seed_qr_payload"):
        if helper not in components_source:
            errors.append(f"firmware shared screen component missing: {helper}")
    for path_text, required_calls in DELEGATED_SCREEN_METHODS.items():
        path = screens_root / path_text
        source = path.read_text(errors="ignore")
        for call in required_calls:
            if call not in source:
                errors.append(f"firmware screen bypasses shared rendering component: {path.relative_to(root)} -> {call}")
    return errors


def _screen_method_errors(root: Path, screen_production_files: list[Path]) -> list[str]:
    errors: list[str] = []
    screen_source = "\n".join(path.read_text(errors="ignore") for path in screen_production_files)
    screen_signature_pattern = re.compile(
        r"(?ms)^\s*pub\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(.*?)\{"
    )
    screen_methods = [name for name, _ in screen_signature_pattern.findall(screen_source)]
    duplicates = sorted({name for name in screen_methods if screen_methods.count(name) > 1})
    if duplicates:
        errors.append(f"firmware screen methods have duplicate owners: {duplicates}")
    firmware_source = _read_firmware_source(root)
    unused_methods = sorted(
        name for name in screen_methods
        if re.search(rf"\.{re.escape(name)}\s*\(", firmware_source) is None
    )
    if unused_methods:
        errors.append(f"firmware screen methods have no consumer: {unused_methods}")
    return errors


def _check_screen_modules(root: Path) -> list[str]:
    # Firmware screen rendering is grouped by user-facing responsibility while
    # preserving the ui::screens module and BootDisplay method contract.
    screens_facade = root / "apps/kaskold-hardware/src/ui/screens.rs"
    screens_root = root / "apps/kaskold-hardware/src/ui/screens"
    screen_production_files = list(screens_root.rglob("*.rs")) if screens_root.exists() else []
    return [
        *_required_screen_module_errors(root, screens_root),
        *_screen_facade_errors(root, screens_facade),
        *_screen_size_errors(root, screen_production_files),
        *_shared_component_errors(root, screens_root),
        *_screen_method_errors(root, screen_production_files),
    ]


def _check_redraw_state_coverage(root: Path, redraw_source: str) -> list[str]:
    errors: list[str] = []
    redraw_states = set(re.findall(
        r"(?:crate::runtime::input::)?AppState::([A-Za-z0-9_]+)",
        redraw_source,
    ))
    state_source = (root / "crates/kaskold-hardware-core/src/screen_flow/state.rs").read_text(
        errors="ignore"
    )
    state_match = re.search(r"(?ms)pub enum AppState\s*\{(?P<body>.*?)^\}", state_source)
    declared_states = set() if state_match is None else set(re.findall(
        r"(?m)^    ([A-Z][A-Za-z0-9_]*)\b", state_match.group("body")
    ))
    missing_states = declared_states - redraw_states
    unknown_states = redraw_states - declared_states
    if missing_states:
        errors.append(f"firmware redraw is missing AppState variants: {sorted(missing_states)}")
    if unknown_states:
        errors.append(f"firmware redraw references unknown AppState variants: {sorted(unknown_states)}")
    return errors


def _pin_ui_errors(ui_root: Path) -> list[str]:
    pin_ui = ui_root / "pin_ui.rs"
    pin_ui_tests = ui_root / "unit_tests/pin_ui_tests.rs"
    if pin_ui.exists() or pin_ui_tests.exists():
        return ["dormant PIN UI implementation or tests must not return"]
    return []


def _mnemonic_limit_errors(root: Path, mnemonic_root: Path) -> tuple[list[str], str]:
    errors: list[str] = []
    mnemonic_source = ""
    for name, limit in MNEMONIC_LIMITS.items():
        path = mnemonic_root / name
        if not path.exists():
            errors.append(f"required wallet mnemonic module is missing: {path.relative_to(root)}")
            continue
        source = path.read_text(errors="ignore")
        mnemonic_source += "\n" + source
        if len(source.splitlines()) > limit:
            errors.append(
                f"wallet mnemonic module exceeds SRP limit: {path.relative_to(root)} "
                f"({len(source.splitlines())} > {limit})"
            )
    return errors, mnemonic_source


def _mnemonic_facade_errors(mnemonic_root: Path, mnemonic_source: str) -> list[str]:
    errors: list[str] = []
    mnemonic_facade = (mnemonic_root / "mod.rs").read_text(errors="ignore") if mnemonic_root.exists() else ""
    for symbol in ("DiceCollector", "TouchEntropyCollector", "generate_from_entropy", "WordInput", "validate"):
        if symbol not in mnemonic_facade:
            errors.append(f"wallet mnemonic façade is missing export: {symbol}")
    if "TOUCH_ENTROPY_TARGET" in mnemonic_facade:
        errors.append("wallet mnemonic façade must keep TOUCH_ENTROPY_TARGET private to touch.rs")
    if "pub const fn target(&self) -> usize" not in mnemonic_source:
        errors.append("wallet touch-entropy collector must expose its target through target()")
    for obsolete_mnemonic_symbol in (
        "SetupState", "SetupWizard", "SetPin", "ConfirmPin",
        "serialize_mnemonic", "deserialize_mnemonic",
    ):
        if re.search(rf"\b{obsolete_mnemonic_symbol}\b", mnemonic_source):
            errors.append(
                f"wallet mnemonic domain retains dormant PIN/NVS surface: {obsolete_mnemonic_symbol}"
            )
    return errors


def _mnemonic_module_errors(root: Path, ui_root: Path) -> list[str]:
    legacy_mnemonic_module = ui_root / "setup_wizard.rs"
    mnemonic_root = root / "apps/kaskold-hardware/src/wallet/mnemonic"
    errors: list[str] = []
    if legacy_mnemonic_module.exists():
        errors.append("legacy monolithic mnemonic-setup module must not exist")
    actual_mnemonic = {path.name for path in mnemonic_root.glob("*.rs")}
    if actual_mnemonic != set(MNEMONIC_LIMITS):
        errors.append(
            f"wallet mnemonic module inventory changed: expected {sorted(MNEMONIC_LIMITS)}, "
            f"got {sorted(actual_mnemonic)}"
        )
    limit_errors, mnemonic_source = _mnemonic_limit_errors(root, mnemonic_root)
    return [
        *errors,
        *limit_errors,
        *_mnemonic_facade_errors(mnemonic_root, mnemonic_source),
    ]


def _seed_manager_errors(root: Path, ui_root: Path) -> list[str]:
    seed_legacy = ui_root / "seed_manager.rs"
    seed_root = root / "apps/kaskold-hardware/src/wallet/seed_manager"
    errors: list[str] = []
    if seed_legacy.exists():
        errors.append("legacy monolithic seed-manager module must not exist")
    actual_seed = {path.name for path in seed_root.glob("*.rs")}
    if actual_seed != set(SEED_LIMITS):
        errors.append(
            f"seed-manager module inventory changed: expected {sorted(SEED_LIMITS)}, "
            f"got {sorted(actual_seed)}"
        )
    for name, limit in SEED_LIMITS.items():
        path = seed_root / name
        if not path.exists():
            errors.append(f"required seed-manager module is missing: {path.relative_to(root)}")
            continue
        line_count = len(path.read_text(errors="ignore").splitlines())
        if line_count > limit:
            errors.append(
                f"seed-manager module exceeds SRP limit: {path.relative_to(root)} "
                f"({line_count} > {limit})"
            )
    seed_facade = (seed_root / "mod.rs").read_text(errors="ignore") if seed_root.exists() else ""
    for symbol in ("SeedSlot", "SeedManager", "MAX_SLOTS", "encode_seedqr", "decode_seedqr", "encode_compact_seedqr", "decode_compact_seedqr", "PassphraseInput"):
        if symbol not in seed_facade:
            errors.append(f"wallet seed-manager façade is missing export: {symbol}")
    return errors


def _stateless_nvs_errors(root: Path) -> list[str]:
    firmware_source = _read_firmware_source(root)
    if re.search(r"\bNVS\b", firmware_source, re.I):
        return ["stateless firmware retains dormant NVS references"]
    return []


def _icon_browser_errors(root: Path) -> list[str]:
    icon_browser = root / "apps/kaskold-hardware/src/ui/icon_browser.rs"
    if icon_browser.exists() and re.search(r"\bstruct\s+IconEntry\b", icon_browser.read_text(errors="ignore")):
        return ["unused IconEntry catalog type must not return"]
    return []


def _check_mnemonic_and_seed_ui(root: Path) -> list[str]:
    ui_root = root / "apps/kaskold-hardware/src/ui"
    return [
        *_pin_ui_errors(ui_root),
        *_mnemonic_module_errors(root, ui_root),
        *_seed_manager_errors(root, ui_root),
        *_stateless_nvs_errors(root),
        *_icon_browser_errors(root),
    ]


def _redraw_inventory_errors(actual_redraw_modules: set[Path], redraw_root: Path) -> list[str]:
    required_redraw_modules = {redraw_root / name for name in REQUIRED_REDRAW_MODULES}
    errors: list[str] = []
    if required_redraw_modules - actual_redraw_modules:
        errors.append(
            "required firmware redraw modules are missing: "
            f"{sorted(path.name for path in required_redraw_modules - actual_redraw_modules)}"
        )
    if actual_redraw_modules - required_redraw_modules:
        errors.append(
            "unregistered firmware redraw modules found: "
            f"{sorted(path.name for path in actual_redraw_modules - required_redraw_modules)}"
        )
    return errors


def _redraw_facade_errors(redraw_facade: Path) -> list[str]:
    errors: list[str] = []
    if redraw_facade.exists():
        redraw_facade_source = redraw_facade.read_text(errors="ignore")
        if len(redraw_facade_source.splitlines()) > 80:
            errors.append("ui/redraw.rs must remain a small stable façade")
        if "match ad.navigation.app.state" in redraw_facade_source:
            errors.append("ui/redraw.rs must delegate screen-family redraw implementations")
        api_match = re.search(r"(?ms)^pub fn redraw_screen\s*(.*?)\{", redraw_facade_source)
        api_signatures = [] if api_match is None else [
            " ".join(f"pub fn redraw_screen {api_match.group(1)}".split())
        ]
        api_digest = hashlib.sha256("\n".join(api_signatures).encode()).hexdigest()
        if len(api_signatures) != 1 or api_digest != "e0ebf2afa44b0343240f40f2be30ecba661624211c4b8a434d0faf2a773e11b2":
            errors.append(f"firmware redraw API changed: {len(api_signatures)} signatures / {api_digest}")
    return errors


def _redraw_dispatch_errors(root: Path, redraw_implementation_files: list[Path]) -> list[str]:
    errors: list[str] = []
    for module in redraw_implementation_files:
        dispatchers = module.read_text(errors="ignore").count("match ad.navigation.app.state")
        if dispatchers > 1:
            errors.append(
                f"firmware redraw module owns multiple state dispatchers: {module.relative_to(root)}"
            )
    return errors


def _redraw_delegation_errors(root: Path, actual_redraw_modules: set[Path]) -> list[str]:
    errors: list[str] = []
    for module in actual_redraw_modules:
        source = module.read_text(errors="ignore")
        delegates_state = (
            "match ad.navigation.app.state" in source
            or "redraw_state(ad.navigation.app.state" in source
            or "mod " in source
        )
        if not delegates_state:
            errors.append(
                f"firmware redraw module neither dispatches nor delegates: {module.relative_to(root)}"
            )
    return errors


def _redraw_size_errors(root: Path, redraw_implementation_files: list[Path]) -> list[str]:
    errors: list[str] = []
    for module in redraw_implementation_files:
        line_count = len(module.read_text(errors="ignore").splitlines())
        if line_count > 600:
            errors.append(
                f"firmware redraw module exceeds 600-line SRP limit: "
                f"{module.relative_to(root)} ({line_count} lines)"
            )
    return errors


def _check_redraw_modules(root: Path) -> list[str]:
    # Redraw routing is grouped by screen family while preserving ui::redraw::redraw_screen.
    redraw_facade = root / "apps/kaskold-hardware/src/ui/redraw.rs"
    redraw_root = root / "apps/kaskold-hardware/src/ui/redraw"
    actual_redraw_modules = set(redraw_root.glob("*.rs")) if redraw_root.exists() else set()
    redraw_implementation_files = sorted(redraw_root.rglob("*.rs"))
    redraw_source = "\n".join(path.read_text(errors="ignore") for path in redraw_implementation_files)
    return [
        *_redraw_inventory_errors(actual_redraw_modules, redraw_root),
        *_redraw_facade_errors(redraw_facade),
        *_redraw_size_errors(root, redraw_implementation_files),
        *_check_redraw_state_coverage(root, redraw_source),
        *_redraw_dispatch_errors(root, redraw_implementation_files),
        *_redraw_delegation_errors(root, actual_redraw_modules),
        *_check_mnemonic_and_seed_ui(root),
    ]


def check(root: Path) -> list[str]:
    return [
        *_check_screen_modules(root),
        *_check_redraw_modules(root),
    ]
