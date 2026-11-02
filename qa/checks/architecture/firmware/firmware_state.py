from __future__ import annotations

from pathlib import Path
import re

from ..core.common import rust_code_only, rust_use_paths
from .firmware_settings_state import check as check_settings_state_groups
from .firmware_state_fields import LEGACY_DOMAIN_FIELDS, nested_struct_expectations


def _expected_firmware_state_fields() -> dict[str, tuple[str, ...]]:
    return {
        "runtime": ("needs_redraw", "idle_ticks", "display_asleep", "home_reached", "qr_brightness_override", "reauth_return_to"),
        "presentation": (),
        "navigation": (
            "multisig_menu", "multisig_descriptors_menu",
            "xprv_export_menu", "settings_menu",
            "production", "sd_import_menu",
        ),
        "wallet": ("seeds", "keys", "addresses"),
        "export": (
            "export_key_hex", "kpub_data", "kpub_len", "kpub_progress", "kpub_seed_derivation", "kpub_worker_generation", "multisig_seed_derivation", "multisig_worker_generation",
            "xprv_data", "xprv_len",
        ),
        "storage": ("persistence", "browser", "export_file", "confirmation", "text_files"),
        "qr": ("outgoing", "presentation", "scan"),
        "signing": ("transaction", "multisig", "message", "commit_reveal", "covenant", "private_swap", "anti_klepto"),
        "stego": ("session", "export_flow", "hint", "import"),
        "settings": ("brightness", "screen_dim_timeout", "volume", "previous_volume"),
        "pop_it": ("return_state", "owner_authority_enrolled", "error"),
    }


def _state_module_path(root: Path, name: str) -> Path:
    base = root / "apps/kaskold-hardware/src/runtime/data"
    direct = base / f"{name}.rs"
    return direct if direct.exists() else base / name / "mod.rs"


def _check_state_facade(root: Path, expected_fields: dict[str, tuple[str, ...]]) -> list[str]:
    errors: list[str] = []
    firmware_data_facade = root / "apps/kaskold-hardware/src/runtime/data.rs"
    firmware_data_root = root / "apps/kaskold-hardware/src/runtime/data"
    {
        name: "".join(part.title() for part in name.split("_")) + "State"
        for name in expected_fields
    }
    for name in expected_fields:
        required = _state_module_path(root, name)
        if not required.exists():
            errors.append(f"required firmware state module is missing: {required.relative_to(root)}")

    if firmware_data_facade.exists():
        data_source = firmware_data_facade.read_text(errors="ignore")
        placement_path = firmware_data_root / "placement" / "mod.rs"
        placement_source = placement_path.read_text(errors="ignore") if placement_path.exists() else ""
        if len(data_source.splitlines()) > 120:
            errors.append("runtime/data.rs must remain a small AppData aggregate")
        app_data_match = re.search(
            r"(?ms)pub struct AppData\s*\{(?P<body>.*?)^\}", data_source
        )
        if not app_data_match:
            errors.append("runtime/data.rs must define the stable AppData root")
        else:
            root_fields = tuple(re.findall(
                r"(?m)^\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:",
                app_data_match.group("body"),
            ))
            if root_fields != tuple(expected_fields):
                errors.append(
                    f"AppData state groups changed: expected {tuple(expected_fields)}, "
                    f"got {root_fields}"
                )
        if "pub(crate) fn try_initialize(" not in placement_source:
            errors.append("AppData must retain its stable one-shot initialization boundary")

    return errors


def _check_navigation_authority_boundary(root: Path) -> list[str]:
    errors: list[str] = []
    path = root / "apps/kaskold-hardware/src/runtime/data/navigation.rs"
    source = path.read_text(errors="ignore") if path.exists() else ""
    match = re.search(r"(?ms)pub struct NavigationState\s*\{(?P<body>.*?)^\}", source)
    if not match:
        return ["NavigationState authority boundary is missing"]
    body = match.group("body")
    for field in ("owner", "committed_state", "app"):
        if not re.search(rf"(?m)^\s*pub\(crate\)\s+{field}\s*:", body):
            errors.append(
                f"NavigationState.{field} must remain crate-private to the navigation authority"
            )
        if re.search(rf"(?m)^\s*pub\s+{field}\s*:", body):
            errors.append(
                f"NavigationState.{field} must not be a public state-mutation surface"
            )

    firmware_root = root / "apps/kaskold-hardware/src"
    navigation_root = firmware_root / "runtime/navigation"
    assignment = re.compile(r"\.navigation\.app\.state\s*=\s*(?!=)")
    for rust_path in firmware_root.rglob("*.rs"):
        if rust_path == navigation_root.with_suffix(".rs") or navigation_root in rust_path.parents:
            continue
        if assignment.search(rust_code_only(rust_path.read_text(errors="ignore"))):
            errors.append(
                "raw navigation state assignment bypasses the navigation authority: "
                f"{rust_path.relative_to(root)}"
            )
    return errors


def _check_state_modules(root: Path, expected_fields: dict[str, tuple[str, ...]]) -> tuple[list[str], set[str]]:
    errors: list[str] = []
    expected_types = {
        name: "".join(part.title() for part in name.split("_")) + "State"
        for name in expected_fields
    }
    all_flat_state_fields: set[str] = set()
    for name, module_fields in expected_fields.items():
        path = _state_module_path(root, name)
        if not path.exists():
            continue
        source = path.read_text(errors="ignore")
        if len(source.splitlines()) > 140:
            errors.append(
                f"firmware state module exceeds 140-line SRP limit: "
                f"{path.relative_to(root)} ({len(source.splitlines())} lines)"
            )
        struct_name = expected_types[name]
        match = re.search(
            rf"(?ms)pub struct {struct_name}\s*\{{(?P<body>.*?)^\}}", source
        )
        if not match:
            errors.append(f"state module does not define {struct_name}: {path.relative_to(root)}")
            continue
        actual_fields = tuple(re.findall(
            r"(?m)^\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", match.group("body")
        ))
        if actual_fields != module_fields:
            errors.append(
                f"firmware {name} state ownership changed: expected {module_fields}, "
                f"got {actual_fields}"
            )
        overlap = all_flat_state_fields.intersection(actual_fields)
        if overlap:
            errors.append(f"firmware state fields have multiple owners: {sorted(overlap)}")
        all_flat_state_fields.update(actual_fields)
        if name == "wallet":
            placement = (root / "apps/kaskold-hardware/src/runtime/data/placement/mod.rs").read_text(errors="ignore")
            placement_contracts = ("unsafe fn place_wallet(", "place_wallet_seeds", "place_wallet_keys", "place_wallet_addresses")
            if not all(contract in placement for contract in placement_contracts):
                errors.append("wallet state lacks focused stack-safe in-place placement")
        else:
            constructor_contracts = (
                "pub(super) fn new() -> Self",
                "pub(super) fn try_new() -> Result<Self, ()>",
                "pub(super) unsafe fn initialize_in_place(",
            )
            if not any(contract in source for contract in constructor_contracts):
                errors.append(f"state module lacks focused constructor: {path.relative_to(root)}")

    return errors, all_flat_state_fields


def _check_state_usage(root: Path, all_flat_state_fields: set[str]) -> list[str]:
    errors: list[str] = []
    firmware_data_facade = root / "apps/kaskold-hardware/src/runtime/data.rs"
    firmware_data_root = root / "apps/kaskold-hardware/src/runtime/data"
    firmware_source_root = root / "apps/kaskold-hardware/src"
    legacy_flat_access = re.compile(
        r"\bad\.({})\b".format("|".join(sorted(map(re.escape, all_flat_state_fields), key=len, reverse=True)))
    ) if all_flat_state_fields else None
    if legacy_flat_access:
        for path in firmware_source_root.rglob("*.rs"):
            if path == firmware_data_facade or firmware_data_root in path.parents:
                continue
            source = path.read_text(errors="ignore")
            match = legacy_flat_access.search(source)
            if match:
                errors.append(
                    f"flat AppData field access bypasses state ownership: "
                    f"{path.relative_to(root)} uses ad.{match.group(1)}"
                )

    firmware_source = "\n".join(
        path.read_text(errors="ignore") for path in firmware_source_root.rglob("*.rs")
    )
    if "StegoEmbed" in firmware_source:
        errors.append("firmware retains unreachable legacy StegoEmbed state")

    return errors



def _check_nested_domain_state(root: Path) -> list[str]:
    data_root = root / "apps/kaskold-hardware/src/runtime/data"
    errors = _check_nested_struct_fields(nested_struct_expectations(data_root))
    firmware_source = "\n".join(path.read_text(errors="ignore") for path in (root / "apps/kaskold-hardware/src").rglob("*.rs"))
    errors.extend(_check_legacy_domain_state(firmware_source))
    return errors


def _check_nested_struct_fields(expectations: dict[Path, dict[str, tuple[str, ...]]]) -> list[str]:
    errors: list[str] = []
    for path, structs in expectations.items():
        source = path.read_text(errors="ignore") if path.exists() else ""
        for name, expected in structs.items():
            errors.extend(_struct_field_errors(name, expected, source))
    return errors


def _struct_field_errors(name: str, expected: tuple[str, ...], source: str) -> list[str]:
    match = re.search(rf"(?ms)pub struct {name}\s*\{{(?P<body>.*?)^\}}", source)
    if not match:
        return [f"nested firmware state is missing: {name}"]
    actual = tuple(re.findall(r"(?m)^\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", match.group("body")))
    if actual != expected:
        return [f"nested firmware state {name} changed: expected {expected}, got {actual}"]
    return []


def _check_legacy_domain_state(firmware_source: str) -> list[str]:
    errors: list[str] = []
    for prefix, label, fields in LEGACY_DOMAIN_FIELDS:
        for field in fields:
            if re.search(rf"\b{prefix}\.{field}\b", firmware_source):
                errors.append(f"{label} bypasses domain owner: {field}")
    return errors



def _check_camera_session_identity(root: Path) -> list[str]:
    """CoreS3 has one fixed GC0308 path; reject legacy runtime sensor selection."""
    errors: list[str] = []
    firmware_root = root / "apps/kaskold-hardware/src"
    combined = "\n".join(path.read_text(errors="ignore") for path in firmware_root.rglob("*.rs"))
    for forbidden in (
        "SENSOR_OV2640",
        "is_ov2640_sensor",
        "set_ov2640_sensor",
        "sensor_is_ov2640",
        "OV2640",
        "OV5640",
    ):
        if forbidden in combined:
            errors.append(f"firmware retains retired camera-sensor selection: {forbidden}")
    return errors


FIRMWARE_ROOT = Path("apps/kaskold-hardware/src")
DATA_ROOT = FIRMWARE_ROOT / "runtime/data"


STALE_WALLET_DOMAIN_PATHS = ("ui::seed_manager", "ui::setup_wizard")


def _local_rust_path(path: str) -> str:
    normalized = path.strip().lstrip(":")
    return normalized.removeprefix("crate::")


def _stale_wallet_domain_paths(source: str) -> set[str]:
    stale: set[str] = set()
    for imported in rust_use_paths(source):
        local = _local_rust_path(imported)
        if any(local == prefix or local.startswith(f"{prefix}::") for prefix in STALE_WALLET_DOMAIN_PATHS):
            stale.add(local)
    code = rust_code_only(source)
    pattern = re.compile(r"\b(?:crate::)?ui::(?:seed_manager|setup_wizard)(?:::[A-Za-z_][A-Za-z0-9_]*)*")
    stale.update(_local_rust_path(match.group(0)) for match in pattern.finditer(code))
    return stale


def _check_wallet_domain_parser_contract() -> list[str]:
    errors: list[str] = []
    stale_probes = (
        "use crate::ui::seed_manager;",
        "use crate::ui::{seed_manager, setup_wizard};",
        "use crate::{runtime::data::AppData, ui::{display, seed_manager::MAX_SLOTS}};",
        "fn probe() { crate::ui::setup_wizard::generate(); }",
    )
    for probe in stale_probes:
        if not _stale_wallet_domain_paths(probe):
            errors.append(f"wallet-domain stale-path parser missed regression probe: {probe}")
    valid_probe = "use crate::{ui::display, wallet::{mnemonic, seed_manager::MAX_SLOTS}};"
    if _stale_wallet_domain_paths(valid_probe):
        errors.append("wallet-domain stale-path parser rejects current wallet imports")
    return errors


def _check_wallet_domain_boundary(root: Path) -> list[str]:
    errors = _check_wallet_domain_parser_contract()
    ui_root = root / FIRMWARE_ROOT / "ui"
    for forbidden in ("seed_manager", "setup_wizard"):
        if (ui_root / forbidden).exists() or (ui_root / f"{forbidden}.rs").exists():
            errors.append(f"wallet-domain code must not live under ui/: {forbidden}")
    for path in (root / FIRMWARE_ROOT).rglob("*.rs"):
        stale = _stale_wallet_domain_paths(path.read_text(errors="ignore"))
        if stale:
            joined = ", ".join(sorted(stale))
            errors.append(
                f"wallet-domain dependency points through removed ui/ modules: "
                f"{path.relative_to(root)} ({joined})"
            )
    return errors


def _public_state_fields(path: Path) -> tuple[str, ...]:
    source = path.read_text(errors="ignore")
    return tuple(re.findall(r"(?m)^\s*pub\s+([A-Za-z_][A-Za-z0-9_]*)\s*:", source))


def _is_write_only_occurrence(line: str, field: str) -> bool:
    marker = f".{field}"
    offset = line.find(marker)
    if offset < 0:
        return False
    before = line[:offset]
    after = line[offset + len(marker):]
    if re.match(r"\s*(?:=(?!=)|\+=|-=|\*=|/=|%=)", after):
        return True
    if re.match(r"\s*\.(?:fill|clear|reset|zeroize|copy_from_slice|clone_from)\s*\(", after):
        return True
    if re.search(r"&mut\s+[^,;)]*$", before):
        return True
    if re.search(r"(?:zeroize_bytes|zeroize_u16|volatile_clear)\s*\([^)]*&mut\s+[^)]*$", before):
        return True
    return False


def _check_write_only_state(root: Path) -> list[str]:
    errors: list[str] = []
    firmware_root = root / FIRMWARE_ROOT
    sources = {
        path: path.read_text(errors="ignore")
        for path in firmware_root.rglob("*.rs")
        if DATA_ROOT not in path.relative_to(root).parents
    }
    for data_path in (root / DATA_ROOT).glob("*.rs"):
        for field in _public_state_fields(data_path):
            occurrences = [
                line
                for source in sources.values()
                for line in source.splitlines()
                if f".{field}" in line
            ]
            if occurrences and all(_is_write_only_occurrence(line, field) for line in occurrences):
                errors.append(
                    f"runtime state field is write-only: {data_path.relative_to(root)}::{field}"
                )
    return errors


def check(root: Path) -> list[str]:
    expected_fields = _expected_firmware_state_fields()
    module_errors, all_flat_state_fields = _check_state_modules(root, expected_fields)
    all_flat_state_fields -= set(expected_fields)
    return [
        *_check_state_facade(root, expected_fields),
        *_check_navigation_authority_boundary(root),
        *module_errors,
        *_check_state_usage(root, all_flat_state_fields),
        *_check_nested_domain_state(root),
        *check_settings_state_groups(root),
        *_check_camera_session_identity(root),
        *_check_wallet_domain_boundary(root),
        *_check_write_only_state(root),
    ]
