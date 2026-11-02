"""Helper checks for the PSKT and compact KSPT architecture contracts."""

from __future__ import annotations

import re
import sys as _portal_sys
from pathlib import Path

_portal_sys.path.insert(0, str(Path(__file__).resolve().parents[4] / "qa/checks"))

from architecture.core.common import rust_function_lengths  # noqa: E402


def display_path(path: Path, root: Path) -> str:
    try:
        return path.relative_to(root).as_posix()
    except ValueError:
        return path.as_posix()

PSKB_ADAPTER_FORBIDDEN = (
    "fn require_sweep_utxos",
    "fn sweep_amounts",
    "fn require_sweep_balance",
    "fn sweep_scripts",
    "struct SelectedUtxo",
    "fn selected_utxo_entry",
    "fn checked_total",
)
PSKB_ADAPTER_CALLS = (
    "prepare_sweep_from_utxos_core",
    "prepare_selected_sweep_core",
    "encode_prepared_sweep_core",
)
GLOBAL_ADAPTER_FORBIDDEN = (
    "fn withdrawal_policy(", "fn topup_policy(", "fn decode_redeem_script",
    "fn decode_covenant_id", "fn parse_thread_utxo", "fn parse_withdrawal_thread_utxos",
    "fn select_wallet_utxos", "plan_global_thread_withdrawal", "plan_global_thread_topup",
)
GLOBAL_APPLICATION_CALLS = (
    "build_global_thread_withdrawal", "prepare_global_thread_topup_material",
    "build_global_thread_topup",
)
BROWSER_PLANNER_FORBIDDEN = (
    "KIP9_MIN_CHANGE",
    "covenantDepositFee",
    "function selectedTotal",
    "function payloadAwareFee",
    "function prepareDepositAmount",
)
BROWSER_PLANNER_CALLS = ("estimate_covenant_fee", "covenant_type: covenantType")


def _leaked(source: str, forbidden: tuple[str, ...], template: str) -> list[str]:
    return [template.format(item) for item in forbidden if item in source]


def _unbypassed(source: str, required: tuple[str, ...], template: str) -> list[str]:
    return [template.format(item) for item in required if item not in source]


def covenant_dependency_errors(root: Path, dependencies: tuple[Path, ...]) -> list[str]:
    return [
        f"thin covenant boundary dependency missing: {required.relative_to(root)}"
        for required in dependencies
        if not required.is_file()
    ]


def check_pskb_adapter(wasm_pskb: Path) -> list[str]:
    if not wasm_pskb.is_file():
        return []
    source = wasm_pskb.read_text(errors="ignore")
    return _leaked(
        source, PSKB_ADAPTER_FORBIDDEN,
        "PSKB business logic leaked back into WASM adapter: {}",
    ) + _unbypassed(source, PSKB_ADAPTER_CALLS, "thin PSKB WASM adapter bypasses core {}")


def check_global_adapter(wasm_global: Path, online_root: Path) -> list[str]:
    if not wasm_global.is_file():
        return []
    source = wasm_global.read_text(errors="ignore")
    errors = _leaked(
        source, GLOBAL_ADAPTER_FORBIDDEN,
        "global-thread business logic leaked back into WASM adapter: {}",
    )
    application_source = (
        online_root / "transaction_builder/covenant/global_thread.rs"
    ).read_text(errors="ignore")
    if "transaction_builder::covenant::global_thread" not in source:
        errors.append("global-thread WASM adapter must remain a thin re-export of the browser-neutral application layer")
    errors.extend(_unbypassed(
        application_source, GLOBAL_APPLICATION_CALLS,
        "global-thread application layer bypasses core {}",
    ))
    return errors


def check_browser_planner(browser_planner: Path) -> list[str]:
    if not browser_planner.is_file():
        return []
    source = browser_planner.read_text(errors="ignore")
    return _leaked(
        source, BROWSER_PLANNER_FORBIDDEN,
        "covenant monetary policy leaked back into browser adapter: {}",
    ) + _unbypassed(
        source, BROWSER_PLANNER_CALLS,
        "covenant browser adapter bypasses typed core intent: {}",
    )


GLOBAL_THREAD_FORBIDDEN = (
    ".map(|utxo| utxo.amount).sum", "thread_amount + wallet_total",
    "total - withdrawal", "withdrawal - fee",
)
GLOBAL_THREAD_REQUIRED = (
    "checked_utxo_total(thread_utxos", "checked_utxo_total(wallet_utxos",
    ".checked_add(wallet_total)", "GlobalThreadPlanError::ArithmeticOverflow",
)
COVENANT_FORBIDDEN = (".sum::<u64>()", "total - plan.adjusted_send", "total - fee")
COVENANT_FEE_REQUIRED = (
    ".checked_mul(PER_P2PK_INPUT_BYTES)",
    ".checked_add(self.payload_bytes)",
    ".checked_mul(MIN_STANDARD_FEE_RATE_SOMPI_PER_GRAM)",
    "Result<u64, String>",
)
COVENANT_FEE_FORBIDDEN = (
    "input_count * (45 + 66 + 4)",
    "compute_mass * FEE_RATE",
)
STORAGE_MASS_REQUIRED = (
    ".checked_add(plurality)",
    ".checked_mul(plurality)",
    "checked_sum(ins.iter().map",
    ".checked_mul(STORAGE_MASS_C / mean_input)",
)
SHIPPING_FORBIDDEN = ("product_sompi + fee_sompi", "total - first_tranche")
SHIPPING_REQUIRED = (".checked_add(fee_sompi)", ".checked_sub(first_tranche)", "Result<Vec<u8>, String>")
PAYJOIN_FORBIDDEN = ("fee - covenant_fee", ".saturating_sub(")
PAYJOIN_REQUIRED = (
    ".checked_add(mixing_utxo.amount)",
    ".checked_mul(3)",
    ".checked_sub(covenant_fee)",
    ".checked_sub(mixing_fee)",
)
REVIEW_REQUIRED = (
    "checked_total(total_sompi, summary.amount_sompi",
    ".checked_sub(outputs.total_sompi)",
    "PSKT outputs exceed inputs",
)


def balance_errors(balance: str) -> list[str]:
    errors: list[str] = []
    if ".sum()" in balance or ".sum::<u64>()" in balance:
        errors.append("wallet balance aggregation must not use unchecked u64 sum")
    if "try_fold(0u64" not in balance or ".checked_add(entry.amount)" not in balance:
        errors.append("wallet balance aggregation must use checked_add with an explicit error")
    return errors


def global_thread_errors(global_thread: str) -> list[str]:
    return _leaked(
        global_thread, GLOBAL_THREAD_FORBIDDEN,
        "global-thread monetary arithmetic bypasses checked operations: {}",
    ) + _unbypassed(
        global_thread, GLOBAL_THREAD_REQUIRED,
        "global-thread checked arithmetic contract changed: {}",
    )


def covenant_errors(covenant: str) -> list[str]:
    errors = _leaked(
        covenant, COVENANT_FORBIDDEN,
        "covenant monetary arithmetic bypasses checked operations: {}",
    )
    if "selection::checked_total(&plan.selected)?" not in covenant:
        errors.append("covenant output planning must use the shared checked UTXO total")
    if ".checked_sub(plan.adjusted_send)" not in covenant or ".checked_sub(plan.fee)" not in covenant:
        errors.append("covenant output planning must return an explicit underflow error")
    return errors


def covenant_fee_errors(covenant_fee: str) -> list[str]:
    return _unbypassed(
        covenant_fee, COVENANT_FEE_REQUIRED,
        "covenant fee arithmetic must remain checked: {}",
    ) + _leaked(
        covenant_fee, COVENANT_FEE_FORBIDDEN,
        "covenant fee arithmetic bypasses checked operations: {}",
    )


def storage_mass_errors(amounts: str) -> list[str]:
    errors: list[str] = []
    if "pub fn storage_mass_estimate" not in amounts or "-> Result<u64, String>" not in amounts:
        errors.append("storage-mass transaction planning must return checked arithmetic errors")
    if "saturating_add" in amounts or "saturating_mul" in amounts:
        errors.append("storage-mass transaction planning must not silently saturate overflow arithmetic")
    if amounts.count(".saturating_sub(") != 1 or "harmonic_outputs.saturating_sub(input_mass)" not in amounts:
        errors.append(
            "storage-mass subtraction must only floor the final non-negative mass contribution"
        )
    errors.extend(_unbypassed(
        amounts, STORAGE_MASS_REQUIRED,
        "storage-mass checked arithmetic contract changed: {}",
    ))
    return errors


def shipping_escrow_errors(shipping_script: str) -> list[str]:
    return _leaked(
        shipping_script, SHIPPING_FORBIDDEN,
        "shipping-escrow script arithmetic bypasses checked operations: {}",
    ) + _unbypassed(
        shipping_script, SHIPPING_REQUIRED,
        "shipping-escrow script checked arithmetic contract changed: {}",
    )


def payjoin_errors(payjoin: str) -> list[str]:
    return _leaked(
        payjoin, PAYJOIN_FORBIDDEN,
        "PayJoin monetary arithmetic bypasses checked operations: {}",
    ) + _unbypassed(
        payjoin, PAYJOIN_REQUIRED,
        "PayJoin checked arithmetic contract changed: {}",
    )


def vault_errors(vault_spend: str) -> list[str]:
    errors: list[str] = []
    if "total - fee" in vault_spend:
        errors.append("vault spendable balance must not use raw subtraction")
    if ".checked_sub(fee)" not in vault_spend:
        errors.append("vault spendable balance must use checked_sub")
    if "prepared.spendable - amount_a" in vault_spend:
        errors.append("vault split must not use raw subtraction")
    if ".checked_sub(amount_a)" not in vault_spend:
        errors.append("vault split must use checked_sub")
    return errors


def review_errors(review_parser: str) -> list[str]:
    errors: list[str] = []
    if "saturating_add" in review_parser or "saturating_sub" in review_parser:
        errors.append("PSKT review monetary totals must not silently saturate")
    errors.extend(_unbypassed(
        review_parser, REVIEW_REQUIRED,
        "PSKT review checked arithmetic contract changed: {}",
    ))
    return errors


def missing_module_errors(root: Path, required: tuple[Path, ...]) -> list[str]:
    return [
        f"required compact KSPT module is missing: {display_path(path, root)}"
        for path in required
        if not path.exists()
    ]


def retired_module_errors(root: Path, forbidden: tuple[Path, ...]) -> list[str]:
    return [
        f"retired KSPT generation module remains: {display_path(path, root)}"
        for path in forbidden
        if path.exists()
    ]


def production_kspt_files(protocol_wire: Path, bridge_root: Path) -> list[Path]:
    return [
        path for base in (protocol_wire, bridge_root)
        for path in base.rglob("*.rs")
        if "unit_tests" not in path.parts
    ]


def production_kspt_source(files: list[Path]) -> str:
    return "\n".join(path.read_text(errors="ignore") for path in files)


def kspt_size_errors(root: Path, files: list[Path]) -> list[str]:
    errors: list[str] = []
    for path in files:
        line_count = len(path.read_text().splitlines())
        if line_count > 400:
            errors.append(
                f"compact KSPT module exceeds 400-line SRP limit: "
                f"{display_path(path, root)} ({line_count} lines)"
            )
        errors.extend(_function_size_errors(root, path))
    return errors


def _function_size_errors(root: Path, path: Path) -> list[str]:
    return [
        f"compact KSPT function exceeds 100-line SRP target: "
        f"{display_path(path, root)}::{function_name} ({function_lines} lines)"
        for function_name, function_lines in rust_function_lengths(path.read_text())
        if function_lines > 100
    ]


RETIRED_KSPT_API = (
    "GENERATION_V1", "GENERATION_V2", "parse_signed_pskt_v2",
    "serialize_signed_pskt_v2", "serialize_signed_pskt", "parse_pskt",
    "KsptV1", "KsptV2", "UnsignedEnvelopeFormat",
)
KSPT_TRAILER_MARKERS = (
    "NETWORK_MARKER", "MS45_INPUT_MARKER", "MS45_OUTPUT_MARKER", "STEALTH_MARKER",
    "COVENANT_MARKER", "INPUT_DERIVATION_MARKER", "OUTPUT_DERIVATION_MARKER",
)
KSPT_TRAILER_CALLS = (
    "write_network(writer, source.network())",
    "write_ms45_trailers(writer, source, inputs, outputs)",
    "write_stealth(writer, source.stealth())",
    "write_covenant_trailers(writer, source, outputs)",
    "write_derivation_trailers(writer, source, inputs, outputs)",
)


def retired_api_errors(source: str) -> list[str]:
    return _leaked(source, RETIRED_KSPT_API, "retired KSPT API remains: {}")


def wire_version_errors(wire_model: str) -> list[str]:
    if "KSPT_VERSION_CURRENT as KSPT_VERSION" in wire_model:
        return []
    return ["canonical compact KSPT version must be Kaspa Portal's v1 constant"]


def wire_marker_errors(wire_model: str) -> list[str]:
    return _unbypassed(
        wire_model, KSPT_TRAILER_MARKERS,
        "canonical KSPT wire model is missing trailer marker: {}",
    )


def wire_codec_errors(wire_decode: str, wire_encode: str) -> list[str]:
    errors: list[str] = []
    if "reader.u8()? != KSPT_VERSION" not in wire_decode:
        errors.append("canonical compact KSPT decoder must accept only the current version")
    if "write_network(writer, source.network())" not in wire_encode:
        errors.append("canonical compact KSPT encoder must own network trailer emission")
    return errors


def duplicate_grammar_errors(root: Path, protocol_wire: Path, production_files: list[Path]) -> list[str]:
    owner = protocol_wire / "model.rs"
    duplicates = []
    for path in production_files:
        if path == owner:
            continue
        text = path.read_text(errors="ignore")
        if 'b"KSPT"' in text or re.search(r"(?:KSPT|COMPACT_KSPT).*0x04", text):
            duplicates.append(display_path(path, root))
    if duplicates:
        return [f"KSPT magic/generation wire knowledge must have one owner: {duplicates}"]
    return []


def trailer_order_errors(wire_encode: str) -> list[str]:
    trailer_body = wire_encode.split("fn write_trailers", 1)[1].split("fn write_ms45_trailers", 1)[0]
    offsets = [trailer_body.find(token) for token in KSPT_TRAILER_CALLS]
    ms45_body = wire_encode.split("fn write_ms45_trailers", 1)[1].split("fn write_stealth", 1)[0]
    derivation_body = wire_encode.split("fn write_derivation_trailers", 1)[1].split("fn position", 1)[0]
    if any(offset < 0 for offset in offsets) or offsets != sorted(offsets) or not _grouped_ms45_and_derivation(
        ms45_body, derivation_body,
    ):
        return ["canonical compact KSPT encoder trailer order must remain N/I/O/S/C/A/D"]
    return []


def _grouped_ms45_and_derivation(ms45_body: str, derivation_body: str) -> bool:
    return (
        ms45_body.find("source.input_ms45") >= 0
        and ms45_body.find("source.output_ms45") > ms45_body.find("source.input_ms45")
        and derivation_body.find("source.input_derivation") >= 0
        and derivation_body.find("source.output_derivation") > derivation_body.find("source.input_derivation")
    )


def offline_facade_errors(root: Path) -> list[str]:
    errors: list[str] = []
    offline_facade = (root / "crates/offline-signer/src/lib.rs").read_text(errors="ignore")
    if "pub use kaspa_portal::transaction::" not in offline_facade or "interchange::{kspt" not in offline_facade:
        errors.append("offline signer must re-export Kaspa Portal's KSPT codec")
    if (root / "crates/offline-signer/src/transaction").exists():
        errors.append("offline signer must not carry a local transaction/KSPT implementation")
    return errors


PORTAL_BRANCH_PATTERN = re.compile(
    r'kaspa-portal = \{ git = "https://github\.com/peavey2787/kaspa-portal\.git", default-features = false'
)
FIRMWARE_CANONICAL_CONSTANTS = (
    "kspt::MAGIC",
    "kspt::KSPT_VERSION",
    "pskt_envelope::PSKB_MAGIC",
    "pskt_envelope::PSKT_MAGIC",
)
FIRMWARE_DUPLICATED_KNOWLEDGE = ('b"KSPT"', 'b"PSKB"', 'b"PSKT"', "input[4] == 0x04")
FIRMWARE_CORE_DEPENDENCY = (
    'kaskold-protocol = { version = "=2.0.0", path = "../kaskold-protocol", default-features = false }'
)
ANTI_KLEPTO_BINDINGS = (
    "left.network",
    "right.network",
    "left.has_derivation_hint",
    "left.derivation_branch",
    "left.derivation_index",
)


def manifest_errors(root: Path) -> list[str]:
    errors: list[str] = []
    protocol_manifest = (root / "crates/kaskold-protocol/Cargo.toml").read_text(errors="ignore")
    offline_manifest = (root / "crates/offline-signer/Cargo.toml").read_text(errors="ignore")
    if not PORTAL_BRANCH_PATTERN.search(offline_manifest):
        errors.append("offline signer must track kaspa-portal's default branch with std features disabled")
    if 'default-features = false' not in offline_manifest or 'kaskold-protocol' not in offline_manifest:
        errors.append("offline signer must consume kaskold-protocol with host features disabled")
    wire_facade = (root / "crates/kaskold-protocol/src/wire/kspt/mod.rs").read_text(errors="ignore")
    if "pub use kaspa_portal::transaction::interchange::kspt::wire::*;" not in wire_facade:
        errors.append("kaskold-protocol must take the canonical KSPT grammar from Kaspa Portal")
    if "online-watcher" in protocol_manifest or "offline-signer" in protocol_manifest:
        errors.append("kaskold-protocol must not depend upward on host or hardware consumers")
    return errors


def envelope_errors(envelope_source: str) -> list[str]:
    if "kspt::format::KSPT_VERSION_CURRENT" not in envelope_source:
        return ["transaction envelope detection must use the canonical KSPT version constant"]
    return []


def firmware_qr_errors(root: Path) -> list[str]:
    errors: list[str] = []
    firmware_parser_source = (root / "crates/kaskold-hardware-core/src/qr/classification.rs").read_text(errors="ignore")
    for canonical in FIRMWARE_CANONICAL_CONSTANTS:
        if canonical not in firmware_parser_source:
            errors.append(f"firmware QR classification must use canonical protocol constant: {canonical}")
    for duplicate in FIRMWARE_DUPLICATED_KNOWLEDGE:
        if duplicate in firmware_parser_source:
            errors.append(f"firmware QR classification must not duplicate public wire knowledge: {duplicate}")
    firmware_core_manifest = (root / "crates/kaskold-hardware-core/Cargo.toml").read_text(errors="ignore")
    if FIRMWARE_CORE_DEPENDENCY not in firmware_core_manifest:
        errors.append("kaskold-hardware-core must consume no_std kaskold-protocol constants directly")
    return errors


def fork_errors(root: Path, bridge_root: Path) -> list[str]:
    # The Companion relays, merges and verifies anti-klepto transcripts through
    # Kaspa Portal; it keeps no compact KSPT parser or host verifier of its own.
    forks = (
        bridge_root,
        root / "crates/online-watcher/src/protocol/pskt/anti_klepto.rs",
        root / "crates/online-watcher/src/protocol/transaction/signed_kspt.rs",
    )
    return [
        f"Companion forks Kaspa Portal KSPT handling: {display_path(fork, root)}"
        for fork in forks
        if fork.exists()
    ]


def anti_klepto_binding_errors(offline_root: Path) -> list[str]:
    offline_anti_klepto = (offline_root / "signing/anti_klepto/transaction_body.rs").read_text(errors="ignore")
    return [
        f"offline anti-klepto transaction comparison must bind v4 metadata: {required}"
        for required in ANTI_KLEPTO_BINDINGS
        if required not in offline_anti_klepto
    ]


def broadcast_errors(root: Path) -> list[str]:
    errors: list[str] = []
    facade = (root / "crates/online-watcher/src/facade.rs").read_text(errors="ignore")
    broadcast = facade.split("pub async fn broadcast(", 1)[-1].split("\n    }\n", 1)[0]
    # Signed KSPT is authorized once by the canonical verified pipeline; only
    # that typed result becomes the consensus transaction.
    submit_verified = facade.split("async fn submit_verified(", 1)[-1].split("\n    }\n", 1)[0]
    if (
        "kaskold_protocol::companion::verify_complete_kspt(&bytes)" not in broadcast
        or "self.submit_verified(&transaction" not in broadcast
        or ".to_consensus()" not in submit_verified
    ):
        errors.append("signed compact KSPT broadcast must delegate to the canonical verified pipeline")
    for duplicate in ('b"KSPT"', "COMPACT_KSPT_V4", "GENERATION_CURRENT: u8", "NETWORK_MARKER"):
        if duplicate in broadcast:
            errors.append(f"signed KSPT broadcast reintroduced KSPT wire grammar: {duplicate}")
    return errors


def kssn_errors(offline_root: Path) -> list[str]:
    errors: list[str] = []
    kssn_source = (offline_root / "kssn.rs").read_text(errors="ignore")
    if "KSSN_VERSION_CURRENT" not in kssn_source or "reader.u8()? != KSSN_VERSION_CURRENT" not in kssn_source:
        errors.append("KSSN parser must accept only the current protocol version")
    if "KSSN_VERSION_LEGACY" in kssn_source:
        errors.append("KSSN parser retains retired version-v1 compatibility")
    return errors
