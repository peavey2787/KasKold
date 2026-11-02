#!/usr/bin/env python3
"""Rule helpers for the KasKold Vault Web custody and network-isolation gate."""
from __future__ import annotations

from pathlib import Path
from typing import NamedTuple
import re
import sys
import tomllib

ROOT = Path(__file__).resolve().parents[3]
APP = ROOT / "apps/kaskold-vault-web"

class _Sources(NamedTuple):
    rust: str
    html: str
    js: str
    main_js: str
    onboarding_js: str
    wallet_js: str
    settings_js: str


def _read(path: Path) -> str:
    return path.read_text(encoding="utf-8")


def _read_tree(directory: Path, pattern: str) -> str:
    return "\n".join(_read(path) for path in sorted(directory.rglob(pattern)))


def _report(errors: list[str]) -> int:
    for error in errors:
        print(f"ERROR: {error}", file=sys.stderr)
    return 1


def _require_each(errors: list[str], haystack: str, needles: tuple[str, ...], template: str) -> None:
    for needle in needles:
        if needle not in haystack:
            errors.append(template.format(needle))


def _forbid_each(errors: list[str], tokens: tuple[str, ...], template: str, *haystacks: str) -> None:
    for token in tokens:
        if any(token in haystack for haystack in haystacks):
            errors.append(template.format(token))


def _missing_source_errors() -> list[str]:
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
        ROOT / "tools/build/web/build_vault_runtime.py",
        ROOT / "tools/build/web/wasm_runtime.py",
    )
    return [
        f"missing Vault Web source: {path.relative_to(ROOT)}"
        for path in required
        if not path.is_file()
    ]


def _load_sources() -> _Sources:
    return _Sources(
        rust=_read_tree(APP / "src", "*.rs"),
        html=_read(APP / "web/index.html"),
        js=_read_tree(APP / "web/js", "vault_*.js"),
        main_js=_read(APP / "web/js/vault_main.js"),
        onboarding_js=_read(APP / "web/js/vault_onboarding.js"),
        wallet_js=_read(APP / "web/js/vault_wallet_workflows.js"),
        settings_js=_read(APP / "web/js/vault_settings.js"),
    )


def _check_cargo_manifest(errors: list[str]) -> None:
    manifest = tomllib.loads(_read(APP / "Cargo.toml"))
    deps = manifest.get("dependencies", {})
    if "vault-runtime" not in deps:
        errors.append("Vault Web must delegate custody/signing to vault-runtime")
    for forbidden in ("online-watcher", "kaskold-sdk"):
        if forbidden in deps:
            errors.append(f"Vault Web must not depend on online component {forbidden}")


def _check_rust_boundary(errors: list[str], rust: str) -> None:
    _require_each(
        errors,
        rust,
        (
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
        ),
        "Vault Web Rust boundary is missing {}",
    )
    _forbid_each(
        errors,
        ("online_watcher", "WebSocket", "fetch(", "broadcast"),
        "Vault Web Rust boundary contains online capability token {!r}",
        rust,
    )


def _check_html_csp(errors: list[str], html: str) -> None:
    if "connect-src 'self'" not in html:
        errors.append("Vault Web CSP must restrict network connections to same-origin build assets")
    for script_tag in re.findall(r"<script\b[^>]*>(.*?)</script>", html, flags=re.I | re.S):
        if script_tag.strip():
            errors.append("Vault Web must not use inline executable script under its strict CSP")


def _asset_urls(base: Path) -> set[str]:
    return {
        path.relative_to(base).as_posix()
        for path in base.rglob("*")
        if path.is_file() and path.suffix.lower() in {".js", ".css"}
    }


def _check_asset_collisions(errors: list[str]) -> None:
    # Companion and Vault are commonly served alternately on the same localhost
    # origin during development. Their authored static URLs must therefore be
    # product-unique or browsers can legally reuse a cached Companion asset on
    # the Vault page (and vice versa).
    collisions = sorted(_asset_urls(ROOT / "apps/kaskold-companion-web/web") & _asset_urls(APP / "web"))
    if collisions:
        errors.append(
            "Vault Web authored JS/CSS URLs collide with Companion Web cache keys: "
            + ", ".join(collisions)
        )


def _check_main_wiring(errors: list[str], main_js: str) -> None:
    if "import('../pkg/vault_web.js')" not in main_js:
        errors.append("Vault Web must dynamically load its generated runtime so missing pkg output is reported in the UI")
    if "document.querySelectorAll('.screen[id^=\"screen-\"]')" not in main_js:
        # Keep the screen registry derived from the DOM so adding an M5-parity screen cannot
        # leave the router stale and crash onboarding before its handlers are installed.
        errors.append("Vault Web screen registry must be derived from authored screen-* DOM sections")


def _check_handler_ids(errors: list[str], src: _Sources) -> None:
    authored_ids = set(re.findall(r'\bid="([^"]+)"', src.html))
    direct_handler_ids = {
        match.group(1)
        for source in (src.main_js, src.onboarding_js, src.wallet_js)
        for match in re.finditer(r"\$\('([^']+)'\)\.onclick", source)
    }
    missing_handler_ids = sorted(direct_handler_ids - authored_ids)
    if missing_handler_ids:
        errors.append(
            "Vault Web binds onclick handlers to missing authored DOM IDs: "
            + ", ".join(missing_handler_ids)
        )
    settings_bind_ids = set(re.findall(r"bindClick\('([^']+)'", src.settings_js))
    missing_settings_ids = sorted(settings_bind_ids - authored_ids)
    if missing_settings_ids:
        errors.append(
            "Vault Web Settings binds controls missing from the authored DOM: "
            + ", ".join(missing_settings_ids)
        )


def _check_shared_creation(errors: list[str], src: _Sources) -> None:
    _require_each(
        errors,
        src.rust,
        (
            "kaskold_vault_creation_flow",
            "kaskold_vault_create_with_entropy",
            "kaskold_vault_add_create_with_entropy",
        ),
        "Vault Web shared creation boundary is missing {}",
    )
    if "kaskold_vault_creation_flow" not in src.onboarding_js:
        errors.append("Vault Web onboarding must load dice/touch policy from the shared Rust creation flow")
    if "TOUCH_TARGET = 2048" in src.onboarding_js or 'data-dice-target="25"' in src.html:
        errors.append("Vault Web must not duplicate shared dice/touch creation policy in JavaScript/HTML")


def _check_add_wallet(errors: list[str], src: _Sources) -> None:
    if "kaskold_vault_add_create_12" in src.wallet_js or "kaskold_vault_add_create_24" in src.wallet_js:
        errors.append("Vault Web Add Wallet must not bypass the shared entropy/storage onboarding flow")
    if "openWalletPicker" not in src.wallet_js:
        errors.append("Vault Web Switch / Add Wallet must reuse the shared startup wallet picker")
    if 'id="wallet-list"' in src.html or "renderWalletList" in src.wallet_js:
        errors.append("Vault Web must not duplicate the startup wallet picker for Switch / Add Wallet")


def _check_signer_controls(errors: list[str], html: str) -> None:
    for required_id in (
        "onboarding-create", "onboarding-12", "onboarding-24", "show-restore",
        "recovery-material-submit", "begin-scan", "review-approve", "review-reject",
        "review-inspect", "lock-vault",
    ):
        if f'id="{required_id}"' not in html:
            errors.append(f"Vault Web UI is missing required signer control {required_id}")


def _check_backup_copy(errors: list[str], html: str, rust: str) -> None:
    if 'id="backup-kpub"' in html:
        errors.append("creation recovery-word screen must not redundantly render the kpub")
    stale_copy = "They are shown only for this explicit creation flow."
    if stale_copy in html:
        errors.append("Vault Web must not claim recovery words are creation-only")
    _require_each(
        errors,
        rust,
        (
            "kaskold_vault_backup_words",
            "kaskold_vault_seedqr_svg",
            "kaskold_vault_compact_seedqr_svg",
            "kaskold_vault_plain_seedqr_svg",
            "kaskold_vault_backup_xprv",
            "kaskold_vault_export_receive_key",
        ),
        "Vault Web Rust boundary is missing backup export {}",
    )


def _check_workflow_exports(errors: list[str], rust: str) -> None:
    _require_each(
        errors,
        rust,
        (
            "kaskold_vault_receive_address",
            "kaskold_vault_kip22_address",
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
        ),
        "Vault Web Rust boundary is missing workflow export {}",
    )


def _check_placeholder_copy(errors: list[str], html: str, js: str) -> None:
    haystack = (html + "\n" + js).lower()
    for forbidden_copy in (
        "data-unavailable",
        "not implemented",
        "is not available in this Web Vault build",
        "coming soon",
    ):
        if forbidden_copy.lower() in haystack:
            errors.append(f"Vault Web contains placeholder workflow copy {forbidden_copy!r}")


def _check_online_tokens(errors: list[str], js: str, html: str) -> None:
    _forbid_each(
        errors,
        (
            "WebSocket(", "new WebSocket", "XMLHttpRequest", "EventSource(",
            "resolvePublicNode", "broadcastTransaction", "online-watcher",
            "localStorage", "sessionStorage",
        ),
        "Vault Web must not contain online/persistent-secret capability {!r}",
        js,
        html,
    )


def _check_remote_urls(errors: list[str], js: str, html: str) -> None:
    # Network URLs in authored code would bypass the intended offline role. Same-origin
    # package imports are relative and therefore unaffected.
    for match in re.findall(r"https?://[^\s'\"<>]+", js + "\n" + html):
        errors.append(f"Vault Web authored UI must not reference remote URL {match}")


def _check_companion_watch_only(errors: list[str]) -> None:
    companion_manifest = tomllib.loads(_read(ROOT / "apps/kaskold-companion-web/Cargo.toml"))
    companion_deps = companion_manifest.get("dependencies", {})
    for forbidden in ("hot-wallet", "vault-runtime", "offline-signer"):
        if forbidden in companion_deps:
            errors.append(f"Companion Web must remain watch-only and cannot depend on {forbidden}")
