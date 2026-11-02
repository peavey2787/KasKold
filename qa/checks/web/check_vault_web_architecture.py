#!/usr/bin/env python3
"""Fail closed on KasKold Vault Web custody and network-isolation boundaries."""
from __future__ import annotations

from vault_web_architecture_rules import (
    _check_add_wallet,
    _check_asset_collisions,
    _check_backup_copy,
    _check_cargo_manifest,
    _check_companion_watch_only,
    _check_handler_ids,
    _check_html_csp,
    _check_main_wiring,
    _check_online_tokens,
    _check_placeholder_copy,
    _check_remote_urls,
    _check_rust_boundary,
    _check_shared_creation,
    _check_signer_controls,
    _check_workflow_exports,
    _missing_source_errors,
    _load_sources,
    _report,
    _require_each,
)
from vault_web_menu_rules import (
    _check_e2e_states,
    _check_m5_menu_mirror,
)

def main() -> int:
    errors = _missing_source_errors()
    if errors:
        return _report(errors)
    src = _load_sources()
    _check_cargo_manifest(errors)
    _check_rust_boundary(errors, src.rust)
    _check_html_csp(errors, src.html)
    _check_asset_collisions(errors)
    _require_each(
        errors,
        src.html,
        ("css/vault.css", "js/vault_main.js", "lib/vault_jsQR.js"),
        "Vault Web index must reference product-unique asset {}",
    )
    _check_main_wiring(errors, src.main_js)
    _check_handler_ids(errors, src)
    _check_shared_creation(errors, src)
    _check_add_wallet(errors, src)
    _check_signer_controls(errors, src.html)
    _check_m5_menu_mirror(errors, src.html, src.js)
    _check_e2e_states(errors, src.html)
    _check_backup_copy(errors, src.html, src.rust)
    _check_workflow_exports(errors, src.rust)
    _check_placeholder_copy(errors, src.html, src.js)
    _check_online_tokens(errors, src.js, src.html)
    _check_remote_urls(errors, src.js, src.html)
    _check_companion_watch_only(errors)
    if errors:
        return _report(errors)
    print("PASS: Vault Web owns software custody/signing and remains blockchain-network isolated")
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
