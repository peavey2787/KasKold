from __future__ import annotations

from pathlib import Path

import hashlib
import re

from architecture.core.common import relative_posix
from .web_contracts import check_feature_contracts
from .web_infrastructure import check_scoped_context, check_shared_browser_infrastructure
from .web_wasm_api import WASM_API_RELATIVE, check_wasm_api


def _check_module_layout(root: Path) -> tuple[list[str], list[str], str]:
    web_js_root = root / "apps/kaskold-companion-web/web/js"
    actual_web_js_modules = {
        relative_posix(path, web_js_root)
        for path in web_js_root.rglob("*.js")
    } if web_js_root.exists() else set()
    wasm_errors, wasm_import_names = check_wasm_api(root, web_js_root)
    all_web_js_source = "\n".join(
        path.read_text(errors="ignore")
        for path in sorted(web_js_root.rglob("*.js"))
    ) if web_js_root.exists() else ""
    errors = [
        *_check_required_modules(actual_web_js_modules),
        *_check_main_entry(web_js_root),
        *check_scoped_context(web_js_root),
        *_check_module_files(web_js_root),
        *check_shared_browser_infrastructure(web_js_root),
        *wasm_errors,
        *_check_forbidden_browser_sources(root, web_js_root, all_web_js_source),
    ]
    return errors, wasm_import_names, all_web_js_source


def _check_required_modules(actual_web_js_modules: set[str]) -> list[str]:
    errors: list[str] = []
    required_web_js_files = {
        "main.js",
        "app/bootstrap.js",
        "app/shell_controls.js",
        "app/state/index.js",
        "app/state/core/wallet_state.js",
        "app/state/core/wallet_session.js",
        "app/state/core/network_state.js",
        "app/state/core/navigation_state.js",
        "app/state/core/transaction_state.js",
        "app/state/core/ui_state.js",
        "app/state/covenants/covenant_state.js",
        "app/state/covenants/commit_reveal_state.js",
        "app/state/covenants/covenant_recovery_state.js",
        "app/state/covenants/covenant_watcher_state.js",
        "app/state/features/scanner_state.js",
        "app/state/features/oracle_state.js",
        "app/state/features/stealth_state.js",
        "app/events/index.js",
        "features/covenants/model/script_terms.js",
        "wasm/api.js",
    }
    missing_web_js_files = required_web_js_files - actual_web_js_modules
    if missing_web_js_files:
        errors.append(f"required browser modules missing: {sorted(missing_web_js_files)}")
    if "app/initializers.js" in actual_web_js_modules:
        errors.append("legacy app/initializers.js must not return")
    if "app/browser_contract.js" in actual_web_js_modules:
        errors.append("legacy window compatibility module must not return")
    return errors


def _check_main_entry(web_js_root: Path) -> list[str]:
    errors: list[str] = []
    web_main = web_js_root / "main.js"
    if web_main.exists():
        main_lines = len(web_main.read_text().splitlines())
        if main_lines > 20:
            errors.append(f"web/js/main.js exceeds entry-point limit: {main_lines} lines")
        main_source = web_main.read_text(errors="ignore")
        if main_source.count("await import('./app/bootstrap.js')") != 1 or main_source.count("await startApplication()") != 1:
            errors.append("web/js/main.js must dynamically load and start the application exactly once")
        if "pkg/companion_web.js" in main_source:
            errors.append("web/js/main.js must not import the generated WASM package directly")
        if "bindShellControls" not in main_source or "./app/shell_controls.js" not in main_source:
            errors.append("web/js/main.js must bind dependency-free shell controls before application startup")
    return errors


def _check_module_files(web_js_root: Path) -> list[str]:
    errors: list[str] = []
    for module_path in sorted(web_js_root.rglob("*.js")) if web_js_root.exists() else []:
        relative = module_path.relative_to(web_js_root)
        module_source = module_path.read_text(errors="ignore")
        line_count = len(module_source.splitlines())
        if relative != Path("wasm/api.js") and line_count > 600:
            errors.append(f"web module exceeds SRP size limit: {relative} ({line_count} lines)")
        if re.search(r"\sstyle=[\"']", module_source):
            errors.append(f"static inline JavaScript styling is forbidden: {relative}")
        if ".style.cssText" in module_source:
            errors.append(f"JavaScript cssText styling is forbidden: {relative}")
    return errors


def _check_forbidden_browser_sources(root: Path, web_js_root: Path, all_web_js_source: str) -> list[str]:
    return [
        *_check_direct_wasm_package_imports(root, web_js_root),
        *_check_obsolete_heartbeat_surface(all_web_js_source),
        *_check_covenant_script_terms(web_js_root, all_web_js_source),
        *_check_broadcast_preimage_qr(web_js_root),
        *_check_obsolete_commit_hash_surfaces(root, all_web_js_source),
    ]


def _check_direct_wasm_package_imports(root: Path, web_js_root: Path) -> list[str]:
    errors: list[str] = []
    wasm_api_path = web_js_root / WASM_API_RELATIVE
    for path in sorted(web_js_root.rglob("*.js")) if web_js_root.exists() else []:
        if path != wasm_api_path and "pkg/companion_web.js" in path.read_text(errors="ignore"):
            errors.append(f"direct WASM package import outside wasm/api.js: {path.relative_to(root)}")
    return errors


def _check_obsolete_heartbeat_surface(all_web_js_source: str) -> list[str]:
    if "create_oracle_mb_heartbeat_roll" in all_web_js_source:
        return ["obsolete standalone Oracle heartbeat-roll browser surface must not return"]
    return []


def _check_covenant_script_terms(web_js_root: Path, all_web_js_source: str) -> list[str]:
    errors: list[str] = []
    # Covenant terms are read from redeem scripts in WASM only; the browser
    # must not grow its own script parser again.
    script_terms_source = (
        web_js_root / "features/covenants/model/script_terms.js"
    ).read_text(errors="ignore") if web_js_root.exists() else ""
    if "covenant_script_terms(" not in script_terms_source:
        errors.append("covenant script terms must come from the WASM covenant_script_terms export")
    for forbidden in ("walkScriptPushes", "parseEscrowScript", "parseAllowanceScript", "parsePiggyScript",
                      "let lastPush", "op >= 0x01 && op <= 0x4b"):
        if forbidden in all_web_js_source:
            errors.append(f"browser covenant script parser must not return: {forbidden}")
    return errors


def _check_broadcast_preimage_qr(web_js_root: Path) -> list[str]:
    broadcast_source = (
        web_js_root / "features/transactions/send/broadcast.js"
    ).read_text(errors="ignore")
    if "Preimage QR share (commented out" in broadcast_source or "generate_qr_svg_text(preimageJson)" in broadcast_source:
        return ["disabled preimage-QR broadcast implementation must not return"]
    return []


def _check_obsolete_commit_hash_surfaces(root: Path, all_web_js_source: str) -> list[str]:
    errors: list[str] = []
    obsolete_commit_hash_patterns = {
        "handleCrHash": r"\bhandleCrHash\b",
        "btn-cov-cr-hash": r"[\"']btn-cov-cr-hash[\"']",
        "cov-cr-preimage": r"[\"']cov-cr-preimage[\"']",
    }
    for name, pattern in obsolete_commit_hash_patterns.items():
        if re.search(pattern, all_web_js_source):
            errors.append(f"dead manual commit-hash browser surface must not return: {name}")
    web_html_source = "\n".join(
        path.read_text(errors="ignore")
        for path in (root / "apps/kaskold-companion-web/web/html").rglob("*.html")
    )
    if 'id="cov-cr-preimage"' in web_html_source:
        errors.append("dead commit-reveal preimage fallback field must not return")
    return errors


def _check_browser_contracts(root: Path, all_web_js_source: str) -> list[str]:
    web_js_root = root / "apps/kaskold-companion-web/web/js"
    event_root = web_js_root / "app/events"
    return [
        *_check_browser_globals(root, web_js_root, all_web_js_source),
        *_check_storage_contract(all_web_js_source),
        *_check_required_web_checks(root),
        *_check_event_layout(event_root),
        *_check_event_index(event_root),
    ]


def _check_browser_globals(root: Path, web_js_root: Path, all_web_js_source: str) -> list[str]:
    errors: list[str] = []
    # Application code must not expose or consume a window.* compatibility API.
    for path in sorted(web_js_root.rglob("*.js")) if web_js_root.exists() else []:
        source = path.read_text(errors="ignore")
        if re.search(r"\bwindow\.[A-Za-z_$][A-Za-z0-9_$]*", source):
            errors.append(f"application-defined browser global access is forbidden: {path.relative_to(root)}")
        if "Object.defineProperty(window" in source:
            errors.append(f"application-defined browser global proxy is forbidden: {path.relative_to(root)}")
    if "readApplicationCapability" in all_web_js_source or "writeApplicationCapability" in all_web_js_source:
        errors.append("generic browser capability bypass functions must not return")
    return errors


def _check_storage_contract(all_web_js_source: str) -> list[str]:
    storage_keys = set(
        f"{store}:{key}"
        for store, key in re.findall(
            r"\b(localStorage|sessionStorage)\.(?:getItem|setItem|removeItem)\(\s*['\"]([^'\"]+)['\"]",
            all_web_js_source,
        )
    )
    storage_digest = hashlib.sha256("\n".join(sorted(storage_keys)).encode()).hexdigest()
    if len(storage_keys) != 4 or storage_digest != "1f23f49540caa51369fce050881cc1cb7ae16ff5cd99c319487a3aa7a325079a":
        return [f"browser storage contract changed: {len(storage_keys)} keys / {storage_digest}"]
    return []


def _check_required_web_checks(root: Path) -> list[str]:
    required_web_checks = (
        root / "qa/checks/web/check_web_dom_contract.py",
        root / "qa/checks/web/covenants/check_web_covenant_interactions.mjs",
        root / "qa/checks/web/check_web_javascript.py",
        root / "qa/checks/web/runtime/check_web_critical_paths.mjs",
    )
    missing_web_checks = [path.name for path in required_web_checks if not path.exists()]
    if missing_web_checks:
        return [f"required browser regression checks missing: {missing_web_checks}"]
    return []


def _check_event_layout(event_root: Path) -> list[str]:
    errors: list[str] = []
    expected_event_groups = {"system", "wallet", "transactions", "contracts"}
    actual_event_groups = {
        path.name for path in event_root.iterdir() if path.is_dir()
    } if event_root.exists() else set()
    if actual_event_groups != expected_event_groups:
        errors.append(
            f"web event groups changed: expected {sorted(expected_event_groups)}, "
            f"got {sorted(actual_event_groups)}"
        )
    direct_event_modules = sorted(
        path.name for path in event_root.glob("*.js") if path.name != "index.js"
    ) if event_root.exists() else []
    if direct_event_modules:
        errors.append(
            f"web event handlers must be grouped by domain, found direct modules: {direct_event_modules}"
        )
    required_event_modules = {
        "index.js",
        "system/core.js",
        "wallet/wallet_settings.js",
        "transactions/transactions.js",
        "transactions/stealth.js",
        "contracts/oracle.js",
        "contracts/covenant_creation.js",
        "contracts/covenant_actions.js",
        "contracts/covenant_specialized.js",
        "contracts/vault_recovery_events.js",
        "contracts/covenant_loading.js",
    }
    actual_event_modules = {
        path.relative_to(event_root).as_posix() for path in event_root.rglob("*.js")
    } if event_root.exists() else set()
    missing_event_modules = required_event_modules - actual_event_modules
    if missing_event_modules:
        errors.append(f"required grouped web event modules missing: {sorted(missing_event_modules)}")
    return errors


def _check_event_index(event_root: Path) -> list[str]:
    errors: list[str] = []
    expected_event_binders = {
        "bindCoreEvents", "bindOracleEvents", "bindStealthEvents",
        "bindCovenantCreationEvents", "bindCovenantActionsEvents",
        "bindCovenantSpecializedEvents", "bindVaultRecoveryEvents",
        "bindCovenantLoadingEvents", "bindTransactionsEvents",
        "bindWalletSettingsEvents", "bindPortfolioEvents",
    }
    event_index = event_root / "index.js"
    if event_index.exists():
        event_source = event_index.read_text(errors="ignore")
        actual_event_binders = set(re.findall(r"\b(bind[A-Za-z0-9]+Events)\(\);", event_source))
        if actual_event_binders != expected_event_binders:
            errors.append(
                f"web event binder inventory changed: expected {sorted(expected_event_binders)}, "
                f"got {sorted(actual_event_binders)}"
            )
        if "createFeatureContext" in event_source:
            errors.append("event composition must use direct imports rather than application capabilities")
    return errors


def _check_recovery_compatibility_exception(root: Path) -> list[str]:
    errors: list[str] = []
    web_root = root / "apps/kaskold-companion-web/web/js"
    historical_modules = sorted(
        path.relative_to(web_root).as_posix()
        for path in web_root.rglob("historical*.js")
    )
    if historical_modules:
        errors.append(
            "historical browser recovery modules must stay absent: "
            + ", ".join(historical_modules)
        )
    regression_test = root / "qa/checks/web/runtime/check_web_critical_paths.mjs"
    if not regression_test.exists():
        errors.append("current recovery regression test is missing")
    else:
        test_source = regression_test.read_text(errors="ignore")
        for symbol in ("readOptionalDate",):
            if symbol not in test_source:
                errors.append(f"current recovery regression lacks coverage for: {symbol}")
    return errors


def check(root: Path) -> list[str]:
    layout_errors, wasm_import_names, all_web_js_source = _check_module_layout(root)
    return [
        *layout_errors,
        *check_feature_contracts(root, wasm_import_names),
        *_check_browser_contracts(root, all_web_js_source),
        *_check_recovery_compatibility_exception(root),
    ]
