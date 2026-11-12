"""Architecture contracts for PSKT and compact KSPT protocol subsystems."""

from __future__ import annotations

import sys as _portal_sys
from pathlib import Path as _PortalPath

_portal_sys.path.insert(0, str(_PortalPath(__file__).resolve().parents[4] / "qa/checks"))
from portal_source import kaskold_source, portal_text  # noqa: E402

from pathlib import Path
import re

from architecture.protocols.compact_protocols_checks import (
    anti_klepto_binding_errors,
    balance_errors,
    broadcast_errors,
    check_browser_planner,
    check_global_adapter,
    check_pskb_adapter,
    covenant_dependency_errors,
    covenant_errors,
    covenant_fee_errors,
    duplicate_grammar_errors,
    envelope_errors,
    firmware_qr_errors,
    fork_errors,
    global_thread_errors,
    kspt_size_errors,
    kssn_errors,
    manifest_errors,
    missing_module_errors,
    offline_facade_errors,
    payjoin_errors,
    production_kspt_files,
    production_kspt_source,
    retired_api_errors,
    retired_module_errors,
    review_errors,
    shipping_escrow_errors,
    storage_mass_errors,
    trailer_order_errors,
    vault_errors,
    wire_codec_errors,
    wire_marker_errors,
    wire_version_errors,
)

def check_pskt(root: Path) -> list[str]:
    ROOT = root
    errors: list[str] = []
    # PSKT parsing, encoding, review and relay are Kaspa Portal's; the
    # Companion keeps no PSKT protocol subsystem of its own.
    for fork in ("crates/online-watcher/src/protocol/pskt.rs", "crates/online-watcher/src/protocol/pskt"):
        if (ROOT / fork).exists():
            errors.append(f"Companion forks Kaspa Portal's PSKT protocol: {fork}")
    # The PSKT JSON body codec has exactly one owner, kaskold-protocol; the
    # Companion PSKT/PSKB layers delegate to it instead of serializing JSON bodies.
    protocol_wire = kaskold_source("crates/kaskold-protocol/src/pskt/wire.rs").read_text(errors="ignore")
    if len(re.findall(r"serde_json::from_slice", protocol_wire)) != 1:
        errors.append("kaskold-protocol must contain exactly one PSKT JSON body decoder")
    if len(re.findall(r"serde_json::to_vec", protocol_wire)) != 1:
        errors.append("kaskold-protocol must contain exactly one PSKT JSON body encoder")
    relay_root = kaskold_source("crates/kaskold-protocol/src/pskt")
    relay_source = chr(10).join(
        path.read_text(errors="ignore")
        for path in (relay_root / "relay.rs", *sorted((relay_root / "relay").glob("*.rs")))
    )
    subnetwork_source = re.split(
        r"\n(?:pub(?:\([^)]*\))? )?fn ",
        relay_source.split("fn decode_subnetwork", 1)[-1],
        maxsplit=1,
    )[0]
    if (
        "fn decode_subnetwork" not in relay_source
        or "subnetworkId must be 20 bytes" not in subnetwork_source
        or "schema::default_rule(" not in subnetwork_source
        or "unwrap_or_default" in subnetwork_source
    ):
        errors.append("PSKT subnetwork decoding must use the canonical schema default and require 20 bytes")
    canonical_pskb_source = portal_text("transaction/interchange/pskt/pskb/encoder.rs")
    if '.entry("subnetworkId".to_string())' not in canonical_pskb_source:
        errors.append("canonical PSKB encoding must explicitly supply the native subnetwork")
    pskb_builder_root = ROOT / "crates/online-watcher/src/transaction_builder/pskb"
    if (pskb_builder_root / "encoder.rs").exists():
        errors.append("transaction-builder PSKB encoding must delegate to Kaspa Portal's canonical PSKB encoder")
    for path in pskb_builder_root.rglob("*.rs"):
        if '.entry("subnetworkId".to_string())' in path.read_text(errors="ignore"):
            errors.append("transaction-builder PSKB encoding must not duplicate native-subnetwork insertion")

    errors.extend(check_monetary_arithmetic(root))
    errors.extend(check_thin_covenant_boundaries(root))
    return errors

def check_thin_covenant_boundaries(root: Path) -> list[str]:
    online_root = root / "crates/online-watcher/src"
    wasm_pskb = online_root / "wasm_api/protocol/pskb_planning.rs"
    wasm_global = online_root / "wasm_api/contracts/covenant/global_thread/planning.rs"
    core_preparation = online_root / "transaction_builder/pskb/preparation.rs"
    core_thread_input = online_root / "transaction_builder/pskb/thread_input.rs"
    core_thread_request = online_root / "transaction_builder/pskb/thread_request.rs"
    core_fee = kaskold_source("crates/online-watcher/src/transaction_builder/covenant/fee.rs")
    browser_planner = root / "apps/kaskold-companion-web/web/js/features/transactions/send/compose/planners/covenant.js"
    dependencies = (core_preparation, core_thread_input, core_thread_request, core_fee, browser_planner)
    return [
        *covenant_dependency_errors(root, dependencies),
        *check_pskb_adapter(wasm_pskb),
        *check_global_adapter(wasm_global, online_root),
        *check_browser_planner(browser_planner),
    ]


def check_monetary_arithmetic(root: Path) -> list[str]:
    """Keep wallet/transaction values on explicit checked-error paths."""
    online_root = root / "crates/online-watcher/src"
    balance = kaskold_source("crates/online-watcher/src/account/balance.rs").read_text(
        errors="ignore"
    )
    # The global-thread planner lives in Portal; KasKold only adapts its requests.
    global_thread = portal_text("transaction/builder/pskb/global_thread.rs")
    covenant = kaskold_source("crates/online-watcher/src/transaction_builder/covenant/builder.rs").read_text(errors="ignore")
    # The covenant fee shape lives in Portal's mass model.
    covenant_fee = (
        kaskold_source("crates/online-watcher/src/transaction_builder/covenant/fee.rs").read_text(errors="ignore")
        + portal_text("transaction/mass/covenant.rs")
    )
    amounts = kaskold_source("crates/online-watcher/src/transaction_builder/planning/amounts.rs").read_text(errors="ignore")
    shipping_script = kaskold_source(
        "crates/online-watcher/src/contracts/shipping_escrow/script.rs"
    ).read_text(errors="ignore")
    payjoin = (
        online_root / "transaction_builder/covenant/payjoin.rs"
    ).read_text(errors="ignore")
    vault_spend = (
        online_root / "transaction_builder/covenant/vault/spend.rs"
    ).read_text(errors="ignore")
    review_parser = kaskold_source(
        "crates/online-watcher/src/protocol/pskt/review/parser.rs"
    ).read_text(errors="ignore")
    return [
        *balance_errors(balance),
        *global_thread_errors(global_thread),
        *covenant_errors(covenant),
        *covenant_fee_errors(covenant_fee),
        *storage_mass_errors(amounts),
        *shipping_escrow_errors(shipping_script),
        *payjoin_errors(payjoin),
        *vault_errors(vault_spend),
        *review_errors(review_parser),
    ]


def check_kspt(root: Path) -> list[str]:
    offline_root = kaskold_source("crates/offline-signer/src/transaction/kspt")
    bridge_root = root / "crates/online-watcher/src/protocol/pskt/kspt_bridge"
    # Kaspa Portal owns the canonical grammar; kaskold-protocol re-exports it.
    protocol_wire = offline_root / "wire"
    protocol_facade = root / "crates/kaskold-protocol/src/wire/kspt/mod.rs"
    required_offline = (
        offline_root / "mod.rs",
        offline_root / "codec/partial_signed.rs",
        offline_root / "codec/sink.rs",
        offline_root / "codec/source.rs",
        offline_root / "kssn.rs",
        offline_root / "signing/mod.rs",
        offline_root / "validation.rs",
    )
    required_protocol = (
        protocol_facade,
        protocol_wire / "mod.rs",
        protocol_wire / "model.rs",
        protocol_wire / "io.rs",
        protocol_wire / "decode.rs",
        protocol_wire / "encode.rs",
        protocol_wire / "error.rs",
    )
    forbidden_files = (
        offline_root / "codec/unsigned.rs",
        offline_root / "codec/signed_v1.rs",
        bridge_root / "parser_v1.rs",
        bridge_root / "parser_v2.rs",
        bridge_root / "finalizer.rs",
    )
    production_files = production_kspt_files(protocol_wire, bridge_root)
    source = production_kspt_source(production_files)
    wire_model = (protocol_wire / "model.rs").read_text(errors="ignore")
    wire_decode = (protocol_wire / "decode.rs").read_text(errors="ignore")
    wire_encode = (protocol_wire / "encode.rs").read_text(errors="ignore")
    envelope_source = kaskold_source("crates/offline-signer/src/transaction/std_pskt/envelope.rs").read_text(errors="ignore")
    return [
        *missing_module_errors(root, (*required_offline, *required_protocol)),
        *retired_module_errors(root, forbidden_files),
        *kspt_size_errors(root, production_files),
        *retired_api_errors(source),
        *wire_version_errors(wire_model),
        *duplicate_grammar_errors(root, protocol_wire, production_files),
        *wire_marker_errors(wire_model),
        *wire_codec_errors(wire_decode, wire_encode),
        *trailer_order_errors(wire_encode),
        *offline_facade_errors(root),
        *manifest_errors(root),
        *envelope_errors(envelope_source),
        *firmware_qr_errors(root),
        *fork_errors(root, bridge_root),
        *anti_klepto_binding_errors(offline_root),
        *broadcast_errors(root),
        *kssn_errors(offline_root),
    ]
