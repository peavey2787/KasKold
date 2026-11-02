from __future__ import annotations
from pathlib import Path
import hashlib, re

from .camera_contract import check_camera_controllers
from .firmware_controllers_sd import check_sd_controllers


def _check_export_controllers(root: Path) -> list[str]:
    ROOT = root
    errors: list[str] = []
    export_facade = ROOT / "apps/kaskold-hardware/src/runtime/interactions/export.rs"
    export_root = ROOT / "apps/kaskold-hardware/src/runtime/interactions/export"
    required_export_modules = (
        export_root / "address.rs",
        export_root / "derivation.rs",
        export_root / "kpub.rs",
        export_root / "private_key.rs",
        export_root / "seed_backup.rs",
        export_root / "seed_qr.rs",
        export_root / "watch_only.rs",
        export_root / "xprv.rs",
    )
    for required in required_export_modules:
        if not required.exists():
            errors.append(f"required firmware export module is missing: {required.relative_to(ROOT)}")
    if export_facade.exists():
        export_facade_source = export_facade.read_text(errors="ignore")
        if len(export_facade_source.splitlines()) > 120:
            errors.append("controllers/export.rs must remain a small stable façade")
        if "match ad.app.state" in export_facade_source:
            errors.append("controllers/export.rs must delegate workflow implementations")
    export_production_files = list(export_root.rglob("*.rs")) if export_root.exists() else []
    for path in export_production_files:
        line_count = len(path.read_text(errors="ignore").splitlines())
        if line_count > 300:
            errors.append(
                f"firmware export module exceeds 300-line SRP limit: "
                f"{path.relative_to(ROOT)} ({line_count} lines)"
            )
    export_source = "\n".join(
        path.read_text(errors="ignore") for path in [export_facade, *export_production_files]
        if path.exists()
    )
    export_signature_pattern = re.compile(
        r"(?ms)^\s*pub\s+fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*(.*?)\{"
    )
    export_functions = {
        name for name, _ in export_signature_pattern.findall(export_source)
    }
    if export_functions != {"handle_export_touch"}:
        errors.append(
            "firmware export façade must expose only handle_export_touch; "
            f"got {sorted(export_functions)}"
        )
    return errors
STATE_CONTROLLER_CONTRACTS = {
    "seed": {
        "modules": ("bip85.rs", "delete_seed.rs", "import/mod.rs", "passphrase.rs", "passphrase_choice.rs", "seed_list.rs", "wallet_name.rs"),
        "api_count": 1,
        "api_digest": "74d02759a6b4b64b202d0525d8cca161abe1b4f02731019963c6100d103504b7",
        "state_count": 9,
        "state_digest": "1b8879554a027becb6085e7d16851f7971301c1d8a3617e3ec771c81ae4e83dc",
    },
    "tx": {
        "modules": ("commit_results.rs", "commit_reveal.rs", "covenant_signing.rs", "message_signing.rs", "message_source/mod.rs", "multisig_output.rs", "multisig_setup.rs", "private_swap.rs", "transaction.rs"),
        "api_count": 1,
        "api_digest": "a22a29b83d1daef34a802353c435f0fa3f170b1783ae8461f2c093847ac3f9aa",
        "state_count": 26,
        "state_digest": "f1d679a363437d27b6f6b68bb842d2eb66c20ce27391d405a892994b3c618baf",
    },
    "stego": {
        "modules": ("context.rs", "export_confirm/mod.rs", "export_description.rs", "export_security.rs", "import_decrypt.rs", "import_finish.rs", "import_selection/mod.rs", "mode.rs", "photo_scan.rs"),
        "api_count": 1,
        "api_digest": "ceab23f1102d535acf13ec0f51232d882bc85dbdecdb5c33db0562ddaece2b0b",
        "state_count": 18,
        "state_digest": "76b3da3f2bd9b528ae979c8f315c677c605ce336446f212b65279c4c1dbf6e6c",
    },
    "menu": {
        "modules": ("context.rs", "cov_backup_name.rs", "primary.rs", "qr.rs", "seed_generation.rs", "signing.rs", "wallet_tools.rs"),
        "api_count": 1,
        "api_digest": "d7fb2213d77171c5030f2f9ca6c2c99b5d23089ebe1cf95844ac8b1676d378f4",
        "state_count": 15,
        "state_digest": "bf8760440963a66d8633f6978d5c0aa83b8a340f0f9a56218a689460429c1595",
    },
}
_STATE_COMPARE_PATTERN = r"ad\.navigation\.app\.state\s*(?:==|!=)\s*(?:crate::runtime::input::)?AppState::"


def _check_state_controllers(root: Path) -> list[str]:
    errors: list[str] = []
    for controller_name, contract in STATE_CONTROLLER_CONTRACTS.items():
        errors += _controller_state_errors(root, controller_name, contract)
    return errors


def _controller_state_errors(root: Path, name: str, contract: dict) -> list[str]:
    module_root = root / f"apps/kaskold-hardware/src/runtime/interactions/{name}"
    facade = root / f"apps/kaskold-hardware/src/runtime/interactions/{name}.rs"
    actual = _state_module_paths(module_root)
    implementation_files = sorted(module_root.rglob("*.rs"))
    errors = _module_inventory_errors(name, module_root, contract, actual)
    errors += _state_facade_errors(facade, name, contract)
    errors += _module_size_errors(root, name, implementation_files)
    errors += _handled_state_errors(name, implementation_files)
    errors += _dispatcher_count_errors(root, name, implementation_files)
    errors += _delegation_errors(root, name, actual)
    return errors


def _state_module_paths(module_root: Path) -> set[Path]:
    if not module_root.exists():
        return set()
    return set(module_root.glob("*.rs")) | set(module_root.glob("*/mod.rs"))


def _module_inventory_errors(name: str, module_root: Path, contract: dict, actual: set[Path]) -> list[str]:
    required = {module_root / module for module in contract["modules"]}
    errors: list[str] = []
    missing = required - actual
    extra = actual - required
    if missing:
        errors.append(
            f"required firmware {name} modules are missing: "
            f"{sorted(path.name for path in missing)}"
        )
    if extra:
        errors.append(
            f"unregistered firmware {name} modules found: "
            f"{sorted(path.name for path in extra)}"
        )
    return errors


def _state_facade_errors(facade: Path, name: str, contract: dict) -> list[str]:
    if not facade.exists():
        return []
    source = facade.read_text(errors="ignore")
    errors: list[str] = []
    if len(source.splitlines()) > 100:
        errors.append(f"controllers/{name}.rs must remain a small stable façade")
    if "match ad.navigation.app.state" in source:
        errors.append(f"controllers/{name}.rs must delegate state implementations")
    errors += _facade_api_errors(source, name, contract)
    return errors


def _facade_api_errors(source: str, name: str, contract: dict) -> list[str]:
    api_match = re.search(rf"(?ms)^pub fn handle_{name}_touch\s*(.*?)\{{", source)
    api_signatures = [] if api_match is None else [
        " ".join(f"pub fn handle_{name}_touch {api_match.group(1)}".split())
    ]
    api_digest = hashlib.sha256("\n".join(api_signatures).encode()).hexdigest()
    if len(api_signatures) == contract["api_count"] and api_digest == contract["api_digest"]:
        return []
    return [
        f"firmware {name} API changed: "
        f"{len(api_signatures)} signatures / {api_digest}"
    ]


def _module_size_errors(root: Path, name: str, files: list[Path]) -> list[str]:
    errors: list[str] = []
    for module in files:
        line_count = len(module.read_text(errors="ignore").splitlines())
        if line_count > 600:
            errors.append(
                f"firmware {name} module exceeds 600-line SRP limit: "
                f"{module.relative_to(root)} ({line_count} lines)"
            )
    return errors


def _handled_state_errors(name: str, files: list[Path]) -> list[str]:
    module_source = "\n".join(path.read_text(errors="ignore") for path in files)
    handled_states = set(re.findall(
        r"(?:crate::runtime::input::)?AppState::([A-Za-z0-9_]+)\s*=>",
        module_source,
    ))
    handled_states.update(re.findall(_STATE_COMPARE_PATTERN + r"([A-Za-z0-9_]+)", module_source))
    if handled_states:
        return []
    return [f"firmware {name} modules do not handle any AppState"]


def _dispatcher_count_errors(root: Path, name: str, files: list[Path]) -> list[str]:
    errors: list[str] = []
    for module in files:
        source = module.read_text(errors="ignore")
        dispatcher_count = source.count("match ad.navigation.app.state") + len(
            re.findall(_STATE_COMPARE_PATTERN, source)
        )
        if dispatcher_count > 1:
            errors.append(
                f"firmware {name} module owns multiple state dispatchers: "
                f"{module.relative_to(root)}"
            )
    return errors


def _has_state_dispatcher(source: str) -> bool:
    return "match ad.navigation.app.state" in source or re.search(_STATE_COMPARE_PATTERN, source) is not None


def _delegation_errors(root: Path, name: str, modules: set[Path]) -> list[str]:
    errors: list[str] = []
    for module in modules:
        source = module.read_text(errors="ignore")
        if (not _has_state_dispatcher(source) and "mod " not in source
                and "pub(super) fn" not in source and module.name != "context.rs"):
            errors.append(
                f"firmware {name} module neither dispatches nor delegates: "
                f"{module.relative_to(root)}"
            )
    return errors


def _check_pure_controller_boundary(root: Path) -> list[str]:
    errors: list[str] = []
    source_root = root / "apps/kaskold-hardware/src"
    controller = source_root / "controllers.rs"
    legacy_tree = source_root / "controllers"
    interactions = source_root / "runtime/interactions"
    if legacy_tree.exists():
        errors.append("legacy effectful src/controllers/ tree must stay retired")
    if not controller.exists():
        return [*errors, "pure firmware controllers.rs boundary is missing"]
    source = controller.read_text(errors="ignore")
    if len(source.splitlines()) > 120:
        errors.append("pure controllers.rs boundary exceeds 120-line SRP limit")
    for required in ("struct TouchInput", "enum InteractionDomain", "fn classify(", "HandlerGroup"):
        if required not in source:
            errors.append(f"pure controller boundary lost {required}")
    forbidden = (
        "crate::hw", "esp_hal", "crate::services", "crate::ui", "PersistentWallet",
        "BootDisplay", "I2c", "Delay", "Flash", "HMAC", "save_with_credential", "unlock_saved",
    )
    for token in forbidden:
        if token in source:
            errors.append(f"pure controller boundary imports/owns effectful capability: {token}")
    if not interactions.is_dir():
        errors.append("effectful runtime/interactions adapter tree is missing")
    touch_routes = (source_root / "runtime/event_loop/touch_routes.rs").read_text(errors="ignore")
    if "controllers::classify" not in touch_routes or "InteractionDomain::" not in touch_routes:
        errors.append("event-loop input dispatch must enter effectful adapters through the pure controller classifier")
    return errors

def check(root: Path) -> list[str]:
    return [
        *_check_pure_controller_boundary(root),
        *_check_export_controllers(root),
        *check_sd_controllers(root),
        *check_camera_controllers(root),
        *_check_state_controllers(root),
    ]
