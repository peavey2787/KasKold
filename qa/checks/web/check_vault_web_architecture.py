#!/usr/bin/env python3
"""Fail closed on KasKold Vault Web custody and network-isolation boundaries."""
from __future__ import annotations

from pathlib import Path
import re
import json
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[3]
APP = ROOT / "apps/kaskold-vault-web"

# Vault Web mirrors the hardware navigation model wherever the feature has a
# meaningful software implementation. These Settings surfaces are deliberately
# physical-M5-only and must not be emulated in software Vaults.
SOFTWARE_VAULT_OMITTED_MENUS = {"AdvancedMenu", "OwnerFirmwareMenu"}
SOFTWARE_VAULT_OMITTED_SETTINGS_ACTIONS = {
    "settings.display",
    "settings.audio",
    "settings.advanced",
}
SOFTWARE_VAULT_LABEL_OVERRIDES = {
    ("ConfirmTx", "tx.confirm"): "Sign",
}

SOFTWARE_VAULT_OMITTED_STATES = {
    "AdvancedMenu",
    "AdvancedRtcEntry",
    "AdvancedSdStorageWarning",
    "AudioSettings",
    "DisplaySettings",
    "FactoryResetConfirm",
    "FactoryResetWarning",
    "OwnerFirmwareMenu",
    "OwnerInstallConfirm",
    "OwnerInstallWarning",
    "OwnerKeyConfirm",
    "OwnerKeyWarning",
    "PopItConfirm",
    "PopItExplain",
    "PopItPrompt",
}


def main() -> int:
    errors: list[str] = []
    required = (
        APP / "Cargo.toml",
        APP / "Cargo.lock",
        APP / "src/lib.rs",
        APP / "web/index.html",
        APP / "web/js/vault_main.js",
        APP / "web/js/vault_wallet_workflows.js",
        APP / "web/js/vault_advanced_workflows.js",
        APP / "web/js/vault_settings.js",
        APP / "web/js/vault_onboarding.js",
        APP / "web/js/vault_transaction_workflows.js",
        APP / "web/js/vault_covenant_workflows.js",
        APP / "web/js/vault_private_swap_workflows.js",
        APP / "web/js/vault_legacy_workflows.js",
        ROOT / "tools/build/web/build_vault_runtime.py",
    )
    for path in required:
        if not path.is_file():
            errors.append(f"missing Vault Web source: {path.relative_to(ROOT)}")
    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1

    manifest = tomllib.loads((APP / "Cargo.toml").read_text(encoding="utf-8"))
    deps = manifest.get("dependencies", {})
    if "vault-runtime" not in deps:
        errors.append("Vault Web must delegate custody/signing to vault-runtime")
    for forbidden in ("online-watcher", "kaskold-sdk"):
        if forbidden in deps:
            errors.append(f"Vault Web must not depend on online component {forbidden}")

    rust = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted((APP / "src").rglob("*.rs"))
    )
    for required_symbol in (
        "kaskold_vault_create_12",
        "kaskold_vault_create_24",
        "kaskold_vault_restore",
        "kaskold_vault_export_kpub",
        "kaskold_vault_begin_scan",
        "kaskold_vault_accept_frame",
        "kaskold_vault_review",
        "kaskold_vault_approve",
        "kaskold_vault_reject",
        "kaskold_vault_lock",
    ):
        if required_symbol not in rust:
            errors.append(f"Vault Web Rust boundary is missing {required_symbol}")
    for forbidden in ("online_watcher", "WebSocket", "fetch(", "broadcast"):
        if forbidden in rust:
            errors.append(f"Vault Web Rust boundary contains online capability token {forbidden!r}")

    html = (APP / "web/index.html").read_text(encoding="utf-8")
    js = "\n".join(
        path.read_text(encoding="utf-8")
        for path in sorted((APP / "web/js").rglob("vault_*.js"))
    )
    main_js = (APP / "web/js/vault_main.js").read_text(encoding="utf-8")
    onboarding_js = (APP / "web/js/vault_onboarding.js").read_text(encoding="utf-8")
    wallet_js = (APP / "web/js/vault_wallet_workflows.js").read_text(encoding="utf-8")
    settings_js = (APP / "web/js/vault_settings.js").read_text(encoding="utf-8")
    if "connect-src 'self'" not in html:
        errors.append("Vault Web CSP must restrict network connections to same-origin build assets")
    for script_tag in re.findall(r"<script\b[^>]*>(.*?)</script>", html, flags=re.I | re.S):
        if script_tag.strip():
            errors.append("Vault Web must not use inline executable script under its strict CSP")

    # Companion and Vault are commonly served alternately on the same localhost
    # origin during development. Their authored static URLs must therefore be
    # product-unique or browsers can legally reuse a cached Companion asset on
    # the Vault page (and vice versa).
    companion_web = ROOT / "apps/kaskold-companion-web/web"
    companion_assets = {
        path.relative_to(companion_web).as_posix()
        for path in companion_web.rglob("*")
        if path.is_file() and path.suffix.lower() in {".js", ".css"}
    }
    vault_web = APP / "web"
    vault_assets = {
        path.relative_to(vault_web).as_posix()
        for path in vault_web.rglob("*")
        if path.is_file() and path.suffix.lower() in {".js", ".css"}
    }
    collisions = sorted(companion_assets & vault_assets)
    if collisions:
        errors.append(
            "Vault Web authored JS/CSS URLs collide with Companion Web cache keys: "
            + ", ".join(collisions)
        )
    for required_asset in ("css/vault.css", "js/vault_main.js", "lib/vault_jsQR.js"):
        if required_asset not in html:
            errors.append(f"Vault Web index must reference product-unique asset {required_asset}")
    if "import('../pkg/vault_web.js')" not in main_js:
        errors.append("Vault Web must dynamically load its generated runtime so missing pkg output is reported in the UI")
    if "document.querySelectorAll('.screen[id^=\"screen-\"]')" not in main_js:
        # Keep the screen registry derived from the DOM so adding an M5-parity screen cannot
        # leave the router stale and crash onboarding before its handlers are installed.
        errors.append("Vault Web screen registry must be derived from authored screen-* DOM sections")

    static_handler_sources = (main_js, onboarding_js, wallet_js)
    authored_ids = set(re.findall(r'\bid="([^"]+)"', html))
    direct_handler_ids = {
        match.group(1)
        for source in static_handler_sources
        for match in re.finditer(r"\$\('([^']+)'\)\.onclick", source)
    }
    missing_handler_ids = sorted(direct_handler_ids - authored_ids)
    if missing_handler_ids:
        errors.append(
            "Vault Web binds onclick handlers to missing authored DOM IDs: "
            + ", ".join(missing_handler_ids)
        )
    settings_bind_ids = set(re.findall(r"bindClick\('([^']+)'", settings_js))
    missing_settings_ids = sorted(settings_bind_ids - authored_ids)
    if missing_settings_ids:
        errors.append(
            "Vault Web Settings binds controls missing from the authored DOM: "
            + ", ".join(missing_settings_ids)
        )

    for required_export in (
        "kaskold_vault_creation_flow",
        "kaskold_vault_create_with_entropy",
        "kaskold_vault_add_create_with_entropy",
    ):
        if required_export not in rust:
            errors.append(f"Vault Web shared creation boundary is missing {required_export}")
    if "kaskold_vault_creation_flow" not in onboarding_js:
        errors.append("Vault Web onboarding must load dice/touch policy from the shared Rust creation flow")
    if "TOUCH_TARGET = 2048" in onboarding_js or 'data-dice-target="25"' in html:
        errors.append("Vault Web must not duplicate shared dice/touch creation policy in JavaScript/HTML")
    if "kaskold_vault_add_create_12" in wallet_js or "kaskold_vault_add_create_24" in wallet_js:
        errors.append("Vault Web Add Wallet must not bypass the shared entropy/storage onboarding flow")
    if "openWalletPicker" not in wallet_js:
        errors.append("Vault Web Switch / Add Wallet must reuse the shared startup wallet picker")
    if 'id="wallet-list"' in html or "renderWalletList" in wallet_js:
        errors.append("Vault Web must not duplicate the startup wallet picker for Switch / Add Wallet")
    for required_id in (
        "onboarding-create", "onboarding-12", "onboarding-24", "show-restore",
        "recovery-material-submit", "begin-scan", "review-approve", "review-reject",
        "review-inspect", "lock-vault",
    ):
        if f'id="{required_id}"' not in html:
            errors.append(f"Vault Web UI is missing required signer control {required_id}")

    # Software Vault navigation mirrors the authoritative M5 production menu
    # graph instead of maintaining a second hand-written menu contract.
    workflow_graph = json.loads(
        (ROOT / "qa/config/workflow/production_ui_graph.json").read_text(encoding="utf-8")
    )
    menus = {menu["state"]: menu for menu in workflow_graph["menus"]}
    mirrored = tuple(
        menu["state"]
        for menu in workflow_graph["menus"]
        if menu["state"] not in SOFTWARE_VAULT_OMITTED_MENUS
    )
    for menu_name in mirrored:
        section_match = re.search(
            rf'<section\b[^>]*data-workflow-menu="{re.escape(menu_name)}"[^>]*>(.*?)</section>',
            html,
            flags=re.I | re.S,
        )
        if not section_match:
            errors.append(f"Vault Web is missing mirrored M5 menu {menu_name}")
            continue
        menu_html = section_match.group(1)
        actual = []
        for button in re.finditer(
            r'<button\b([^>]*)data-workflow-action="([^"]+)"([^>]*)>(.*?)</button>',
            menu_html,
            flags=re.I | re.S,
        ):
            attrs = button.group(1) + button.group(3)
            action = button.group(2)
            label = re.sub(r"<[^>]+>", "", button.group(4))
            actual.append((action, re.sub(r"\s+", " ", label).strip()))

            # Every retained M5 menu action must be wired to an executable Web
            # handler. Nested icon/span markup is allowed on the home grid.
            id_match = re.search(r'\bid="([^"]+)"', attrs, flags=re.I)
            if not id_match:
                errors.append(f"Vault Web workflow action {action} has no stable control id")
                continue
            control_id = id_match.group(1)
            direct = f"$('" + control_id + "').onclick"
            delegated = "click('" + control_id + "')"
            guarded = "bindClick('" + control_id + "'"
            if direct not in js and delegated not in js and guarded not in js:
                errors.append(
                    f"Vault Web workflow action {action} ({control_id}) has no bound handler"
                )

        expected_items = menus[menu_name]["items"]
        if menu_name == "SettingsMenu":
            expected_items = [
                item for item in expected_items
                if item["action"] not in SOFTWARE_VAULT_OMITTED_SETTINGS_ACTIONS
            ]
        expected = [
            (
                item["action"],
                SOFTWARE_VAULT_LABEL_OVERRIDES.get((menu_name, item["action"]), item["label"]),
            )
            for item in expected_items
        ]
        if actual != expected:
            errors.append(
                f"Vault Web menu {menu_name} drifted from its M5/software projection: "
                f"expected {expected!r}, got {actual!r}"
            )

    e2e = json.loads(
        (ROOT / "qa/config/workflow/production_e2e_scenarios.json").read_text(encoding="utf-8")
    )
    rendered_states: set[str] = set()
    for value in re.findall(r'data-workflow-state="([^"]+)"', html):
        rendered_states.update(value.split())
    missing_render_states = sorted(
        set(e2e["workflow_e2e_physical_render_states"])
        - rendered_states
        - SOFTWARE_VAULT_OMITTED_STATES
    )
    if missing_render_states:
        errors.append(
            "Vault Web is missing authoritative M5 physical workflow states: "
            + ", ".join(missing_render_states)
        )

    connected_states: set[str] = set()
    for scenario in e2e["scenarios"]:
        if scenario.get("level") in {"connected", "qa"} or scenario.get("id", "").startswith("qa-"):
            connected_states.update(scenario.get("states", []))
    missing_connected_states = sorted(
        connected_states - rendered_states - SOFTWARE_VAULT_OMITTED_STATES
    )
    if missing_connected_states:
        errors.append(
            "Vault Web is missing authoritative connected M5 E2E states/equivalents: "
            + ", ".join(missing_connected_states)
        )

    if 'id="backup-kpub"' in html:
        errors.append("creation recovery-word screen must not redundantly render the kpub")
    stale_copy = "They are shown only for this explicit creation flow."
    if stale_copy in html:
        errors.append("Vault Web must not claim recovery words are creation-only")
    for required_backup_export in (
        "kaskold_vault_backup_words",
        "kaskold_vault_seedqr_svg",
        "kaskold_vault_compact_seedqr_svg",
        "kaskold_vault_plain_seedqr_svg",
        "kaskold_vault_backup_xprv",
        "kaskold_vault_export_receive_key",
    ):
        if required_backup_export not in rust:
            errors.append(f"Vault Web Rust boundary is missing backup export {required_backup_export}")
    for required_workflow_export in (
        "kaskold_vault_receive_address",
        "kaskold_vault_open_transaction_file",
        "kaskold_vault_normalize_kpub",
        "kaskold_vault_validate_address",
        "kaskold_vault_normalize_covenant_backup",
        "kaskold_vault_import_raw_key",
        "kaskold_vault_import_xprv",
        "kaskold_vault_multisig_kpub",
        "kaskold_vault_create_multisig",
        "kaskold_vault_import_multisig",
        "kaskold_vault_bip85",
        "kaskold_vault_sign_message",
        "kaskold_vault_commit_secret",
        "kaskold_vault_decrypt_secret",
        "kaskold_vault_portable_backup",
        "kaskold_vault_restore_portable",
        "kaskold_vault_stego_backup",
        "kaskold_vault_restore_stego",
        "kaskold_vault_create_with_entropy",
        "kaskold_vault_add_create_with_entropy",
        "kaskold_vault_anti_klepto_awaiting_reveal",
        "kaskold_vault_begin_anti_klepto_reveal",
        "kaskold_vault_finalize_anti_klepto_reveal",
        "kaskold_vault_covenant_prepare",
        "kaskold_vault_covenant_confirm",
        "kaskold_vault_covenant_reveal",
        "kaskold_vault_private_swap_prepare",
        "kaskold_vault_private_swap_confirm",
        "kaskold_vault_private_swap_reveal",
        "kaskold_vault_signed_transaction",
        "kaskold_vault_set_wallet_name",
    ):
        if required_workflow_export not in rust:
            errors.append(f"Vault Web Rust boundary is missing workflow export {required_workflow_export}")

    for forbidden_copy in (
        "data-unavailable",
        "not implemented",
        "is not available in this Web Vault build",
        "coming soon",
    ):
        if forbidden_copy.lower() in (html + "\n" + js).lower():
            errors.append(f"Vault Web contains placeholder workflow copy {forbidden_copy!r}")

    for forbidden in (
        "WebSocket(", "new WebSocket", "XMLHttpRequest", "EventSource(",
        "resolvePublicNode", "broadcastTransaction", "online-watcher",
        "localStorage", "sessionStorage",
    ):
        if forbidden in js or forbidden in html:
            errors.append(f"Vault Web must not contain online/persistent-secret capability {forbidden!r}")
    # Network URLs in authored code would bypass the intended offline role. Same-origin
    # package imports are relative and therefore unaffected.
    for match in re.findall(r"https?://[^\s'\"<>]+", js + "\n" + html):
        errors.append(f"Vault Web authored UI must not reference remote URL {match}")

    companion_manifest = tomllib.loads((ROOT / "apps/kaskold-companion-web/Cargo.toml").read_text(encoding="utf-8"))
    companion_deps = companion_manifest.get("dependencies", {})
    for forbidden in ("hot-wallet", "vault-runtime", "offline-signer"):
        if forbidden in companion_deps:
            errors.append(f"Companion Web must remain watch-only and cannot depend on {forbidden}")

    if errors:
        for error in errors:
            print(f"ERROR: {error}", file=sys.stderr)
        return 1
    print("PASS: Vault Web owns software custody/signing and remains blockchain-network isolated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
