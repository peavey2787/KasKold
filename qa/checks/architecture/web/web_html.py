"""Companion static HTML composition and compatibility contracts."""

from __future__ import annotations

from collections import Counter
from pathlib import Path
import hashlib
import re

from tools.build.web.build_web_index import (
    OUTPUT,
    SOURCE_ROOT,
    manifest_entries,
    render,
)

EXPECTED_SCREEN_IDS = {
    "screen-welcome",
    "screen-dashboard",
    "screen-advanced",
    "screen-view-kpub",
    "screen-addresses",
    "screen-verify",
    "screen-utxos",
    "screen-history",
    "screen-portfolio",
    "screen-tokens",
    "screen-settings",
    "screen-kpub-manager",
    "screen-send",
    "screen-qr-display",
    "screen-scanner",
    "screen-receive",
    "screen-multisig",
    "screen-multisig-spend",
    "screen-multisig-descriptors",
    "screen-covenant",
    "screen-stealth",
    "screen-broadcast",
    "screen-pskt-review",
}
EXPECTED_ID_DIGEST = "193e28a0d3575732b12389a8b2cd92cf88a1ee71b7a026bcbb6351f37b749a36"
EXPECTED_ID_TAG_DIGEST = "b627a81d0f294090b3f218dd8085193e9b2b94d8a287f97c2fbe9d928db1c113"
EXPECTED_FRAGMENTS = (
    "document/open.html",
    "screens/system/welcome.html",
    "screens/system/dashboard.html",
    "screens/system/advanced.html",
    "screens/wallet/addresses.html",
    "screens/system/verify.html",
    "screens/wallet/utxos.html",
    "screens/wallet/history.html",
    "screens/wallet/portfolio.html",
    "screens/wallet/tokens.html",
    "screens/system/settings.html",
    "screens/wallet/kpub_manager.html",
    "screens/transactions/send.html",
    "screens/system/qr_display.html",
    "screens/system/scanner.html",
    "screens/wallet/receive.html",
    "screens/wallet/multisig.html",
    "screens/covenant/create/menu.html",
    "screens/covenant/create/kasfreeze.html",
    "screens/covenant/create/tagged_vault.html",
    "screens/covenant/create/form/open.html",
    "screens/covenant/create/form/basic/simple.html",
    "screens/covenant/create/form/savings/piggy.html",
    "screens/covenant/create/form/escrow/basic.html",
    "screens/covenant/create/form/escrow/shipment.html",
    "screens/covenant/create/form/savings/timelocked.html",
    "screens/covenant/create/form/automation/dead_mans_switch.html",
    "screens/covenant/create/form/limits/spending_limit.html",
    "screens/covenant/create/form/limits/allowance.html",
    "screens/covenant/create/form/advanced/payjoin.html",
    "screens/covenant/create/form/advanced/oracle_v1.html",
    "screens/covenant/create/form/advanced/crowdfund.html",
    "screens/covenant/create/form/advanced/commit_reveal.html",
    "screens/covenant/create/form/advanced/merkle_whitelist.html",
    "screens/covenant/create/private_swap.html",
    "screens/covenant/create/form/close.html",
    "screens/covenant/create/result.html",
    "screens/covenant/spend/owner_spend.html",
    "screens/covenant/spend/consolidate.html",
    "screens/covenant/spend/borrower_spend.html",
    "screens/covenant/spend/beneficiary_spend.html",
    "screens/covenant/recovery/timeout_refund.html",
    "screens/covenant/spend/payjoin_claim.html",
    "screens/covenant/spend/oracle_v1_claim.html",
    "screens/covenant/proofs/advanced_proofs.html",
    "screens/covenant/proofs/oracle_v1_attest.html",
    "screens/covenant/create/shipment.html",
    "screens/covenant/proofs/commit_reveal_spend.html",
    "screens/covenant/proofs/commit_reveal_verify.html",
    "screens/covenant/proofs/merkle_whitelist.html",
    "screens/covenant/proofs/rollup_balance.html",
    "screens/covenant/recovery/load_existing.html",
    "screens/privacy/stealth.html",
    "screens/transactions/broadcast.html",
    "screens/transactions/pskt_review.html",
    "document/close.html",
)


def _child_dirs(directory: Path) -> set[str]:
    if not directory.exists():
        return set()
    return {path.name for path in directory.iterdir() if path.is_dir()}


def _html_names(directory: Path) -> set[str]:
    if not directory.exists():
        return set()
    return {path.name for path in directory.glob("*.html")}


def _group_errors(actual: set[str], expected: set[str], message: str) -> list[str]:
    if actual == expected:
        return []
    return [f"{message}: expected {sorted(expected)}, got {sorted(actual)}"]


def _loose_html_errors(directory: Path, message: str) -> list[str]:
    names = sorted(path.name for path in directory.glob("*.html"))
    return [f"{message}: {names}"] if names else []


def _screen_layout_errors(screens_root: Path) -> list[str]:
    return [
        *_group_errors(
            _child_dirs(screens_root),
            {"system", "wallet", "transactions", "privacy", "covenant"},
            "web HTML screen groups changed",
        ),
        *_loose_html_errors(
            screens_root, "web HTML screen fragments must be grouped by domain"
        ),
    ]


def _covenant_layout_errors(screens_root: Path) -> list[str]:
    covenant_root = screens_root / "covenant"
    create_root = covenant_root / "create"
    form_root = create_root / "form"
    errors = _group_errors(
        _child_dirs(covenant_root),
        {"create", "spend", "recovery", "proofs"},
        "covenant HTML groups changed",
    )
    errors += _loose_html_errors(
        covenant_root, "covenant HTML fragments must be grouped by workflow"
    )
    errors += _group_errors(
        _child_dirs(form_root),
        {"advanced", "automation", "basic", "escrow", "limits", "savings"},
        "covenant creation form groups changed",
    )
    errors += _group_errors(
        _html_names(form_root),
        {"open.html", "close.html"},
        "covenant creation form shell changed",
    )
    if (create_root / "create.html").exists():
        errors.append("monolithic covenant creation HTML must not return")
    return errors


def _manifest_errors(entries: list[str]) -> list[str]:
    errors: list[str] = []
    if tuple(entries) != EXPECTED_FRAGMENTS:
        errors.append(
            "web HTML fragment inventory/order changed: expected "
            f"{list(EXPECTED_FRAGMENTS)}, got {entries}"
        )
    actual_fragments = {
        path.relative_to(SOURCE_ROOT).as_posix()
        for path in SOURCE_ROOT.rglob("*.html")
    }
    if actual_fragments != set(entries):
        errors.append(
            "web HTML source fragments differ from the manifest: "
            f"unlisted={sorted(actual_fragments - set(entries))}, "
            f"missing={sorted(set(entries) - actual_fragments)}"
        )
    return errors


def _fragment_errors(entry: str) -> list[str]:
    fragment_source = (SOURCE_ROOT / entry).read_text(errors="ignore")
    errors: list[str] = []
    line_count = len(fragment_source.splitlines())
    if line_count > 600:
        errors.append(
            f"web HTML source fragment exceeds 600 lines: {entry} ({line_count})"
        )
    if re.search(r"\sstyle=[\"']", fragment_source):
        errors.append(f"authored HTML contains inline styling: {entry}")
    if re.search(r"<style\b", fragment_source, re.I):
        errors.append(f"authored HTML contains an inline style block: {entry}")
    if "data:image" in fragment_source:
        errors.append(f"authored HTML contains an embedded image asset: {entry}")
    return errors


def check_composition(root: Path) -> tuple[list[str], str | None]:
    screens_root = SOURCE_ROOT / "screens"
    errors = _screen_layout_errors(screens_root) + _covenant_layout_errors(screens_root)
    try:
        entries = manifest_entries()
    except ValueError as error:
        return [str(error)], None
    errors += _manifest_errors(entries)
    for entry in entries:
        errors += _fragment_errors(entry)
    generated = render(entries)
    if not OUTPUT.is_file() or OUTPUT.read_text() != generated:
        errors.append(
            "web/index.html is stale; run tools/build/web/build_web_index.py"
        )
    return errors, generated


def _runtime_builder_errors(root: Path) -> list[str]:
    build_script = root / "tools/build/web/build_companion_runtime.py"
    if not build_script.is_file():
        return [
            "Canonical Companion runtime builder is missing: tools/build/web/build_companion_runtime.py"
        ]
    errors: list[str] = []
    build_source = build_script.read_text(errors="ignore")
    shared = (root / "tools/build/web/wasm_runtime.py").read_text(errors="ignore")
    if shared.find("runtime.authored_generators") > shared.find("_compile_wasm(runtime, toolchain"):
        errors.append(
            "Shared WASM runtime builder must generate authored assets before the WASM cargo build"
        )
    for builder in (
        "tools/build/web/build_web_index.py",
        "tools/build/web/build_app_css.py",
    ):
        if builder not in build_source:
            errors.append(
                f"Canonical Companion runtime builder must run {builder} before the WASM cargo build"
            )
    return errors


def _dom_contract_errors(source: str) -> list[str]:
    errors: list[str] = []
    id_tags = [
        (match.group(3), match.group(1).lower())
        for match in re.finditer(
            r'<([A-Za-z][A-Za-z0-9:-]*)\b([^>]*?\bid=["\']([^"\']+)["\'][^>]*)>',
            source,
            re.S,
        )
    ]
    ids = [name for name, _ in id_tags]
    duplicate_ids = sorted(name for name, count in Counter(ids).items() if count > 1)
    if duplicate_ids:
        errors.append(f"web/index.html contains duplicate DOM IDs: {duplicate_ids}")

    id_digest = hashlib.sha256("\n".join(sorted(ids)).encode()).hexdigest()
    # Contract covers the watch-only Companion shell.
    if len(ids) != 670 or id_digest != EXPECTED_ID_DIGEST:
        errors.append(
            f"web shell DOM ID contract changed: {len(ids)} IDs / {id_digest}"
        )

    id_tag_digest = hashlib.sha256(
        "\n".join(f"{name}:{tag}" for name, tag in sorted(id_tags)).encode()
    ).hexdigest()
    if id_tag_digest != EXPECTED_ID_TAG_DIGEST:
        errors.append(f"web shell DOM element-type contract changed: {id_tag_digest}")

    screen_ids = set(re.findall(
        r'<section\b[^>]*\bid=["\']([^"\']+)["\'][^>]*'
        r'\bclass=["\'][^"\']*\bscreen\b',
        source,
        re.I,
    ))
    if screen_ids != EXPECTED_SCREEN_IDS:
        errors.append(
            f"web screen inventory changed: expected {sorted(EXPECTED_SCREEN_IDS)}, "
            f"got {sorted(screen_ids)}"
        )
    return errors


def _resource_contract_errors(source: str) -> list[str]:
    errors: list[str] = []
    if len(re.findall(r'href=["\']css/app\.css(?:\?[^"\']*)?["\']', source)) != 1:
        errors.append("web/index.html must load css/app.css exactly once")
    if len(re.findall(
        r'<script\b[^>]*\btype=["\']module["\'][^>]*'
        r'\bsrc=["\']js/main\.js["\'][^>]*></script>',
        source,
        re.I | re.S,
    )) != 1:
        errors.append("web/index.html must load js/main.js once as a module")
    if len(re.findall(
        r'<script\b[^>]*\bsrc=["\']lib/jsQR\.js["\'][^>]*></script>',
        source,
        re.I | re.S,
    )) != 1:
        errors.append("web/index.html must load the local jsQR compatibility library once")
    inline_handlers = sorted(set(re.findall(r"\s(on[A-Za-z]+)\s*=", source)))
    if inline_handlers:
        errors.append(f"web/index.html contains inline event handlers: {inline_handlers}")
    return errors


def check(root: Path) -> list[str]:
    errors, generated = check_composition(root)
    errors += _runtime_builder_errors(root)
    index_path = root / "apps/kaskold-companion-web/web/index.html"
    if not index_path.is_file():
        return [*errors, "Companion web/index.html is missing"]
    source = index_path.read_text(errors="ignore")
    if generated is not None and source != generated:
        return errors
    return [*errors, *_dom_contract_errors(source), *_resource_contract_errors(source)]
