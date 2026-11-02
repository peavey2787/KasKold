"""Companion/WASM business-ownership architecture contracts."""

from __future__ import annotations

from pathlib import Path
import re

_GLOBAL_THREAD_ADAPTER_PATHS = (
    "contracts/covenant/families/allowance/global.rs",
    "contracts/covenant/families/spending_limit/global.rs",
)
_REQUIRED_CORE_PATHS = (
    "serialization/input.rs",
    "transaction_builder/pskb/application.rs",
    "transaction_builder/covenant/global_thread.rs",
    "transaction_builder/covenant/payjoin.rs",
    "transaction_builder/covenant/private_swap.rs",
    "transaction_builder/covenant/sweep.rs",
    "transaction_builder/covenant/sweeps/owner.rs",
    "transaction_builder/covenant/sweeps/savings.rs",
    "transaction_builder/covenant/sweeps/timelocked.rs",
    "transaction_builder/covenant/vault/spend.rs",
    "transaction_builder/covenant/oracle_v1.rs",
    "transaction_builder/stealth.rs",
    "transaction_builder/zk/merkle.rs",
    "transaction_builder/zk/crowdfund.rs",
    "transaction_builder/zk/commit_reveal.rs",
    "contracts/covenant/oracle_v1.rs",
    "contracts/covenant/construction/mod.rs",
    "contracts/covenant/construction/additive.rs",
    "contracts/covenant/construction/allowance.rs",
    "contracts/covenant/construction/dms.rs",
    "contracts/covenant/construction/escrow.rs",
    "contracts/covenant/construction/payjoin.rs",
    "contracts/covenant/construction/private_swap.rs",
    "contracts/covenant/construction/savings.rs",
    "contracts/covenant/construction/spending_limit.rs",
    "contracts/oracle/genesis.rs",
    "contracts/shipping_escrow/construction.rs",
    "contracts/merkle/application.rs",
    "contracts/zk/crowdfund.rs",
    "contracts/commit_reveal/application.rs",
)
_DIRECT_RPC_ALLOWLIST = {Path("wallet/watcher.rs")}
_FORBIDDEN_WASM_FRAGMENTS = (
    ("network::submission", "transaction submission"),
    ("transaction_builder::selection", "UTXO selection"),
    ("PskbPlan {", "PSKB plan assembly"),
    ("PskbInputPlan::", "PSKB input assembly"),
    ("PskbOutputPlan::", "PSKB output assembly"),
    ("PskbGlobalPlan::", "PSKB global policy"),
    ("SweepInputPolicy::", "sweep input policy"),
    ("encode_pskt_value", "raw PSKB encoding"),
    ('"previousOutpoint"', "raw PSKB input schema"),
    ('"partialSigs"', "raw PSKB signature schema"),
    ('"covenantBinding"', "raw covenant-binding schema"),
)
_CHECKED_ARITHMETIC = re.compile(r"\.checked_(?:add|sub|mul)\s*\(")
_CONTRACT_BUSINESS_ROOTS = ("contracts/covenant/", "contracts/vault/", "contracts/oracle/", "contracts/zk/")
_CONTRACT_BOUNDARY_FRAGMENTS = (
    ("getrandom::getrandom", "covenant identity/RNG policy"),
    ("script_to_address(", "covenant address construction"),
    ("address_to_script_pubkey(", "covenant address validation/conversion"),
    ("serde_json::json!", "covenant response/business DTO construction"),
)
_WASM_ADAPTER_LIMITS = {
    Path("contracts/vault/genesis.rs"): 80,
    Path("contracts/vault/spend.rs"): 80,
    Path("contracts/covenant/global_thread/planning.rs"): 40,
    Path("contracts/oracle/publish/context.rs"): 10,
    Path("contracts/oracle/publish/plan.rs"): 10,
    Path("contracts/oracle/publish/request.rs"): 10,
    Path("privacy/stealth/spend.rs"): 80,
}


def _check_shared_pskb_planning(root: Path) -> list[str]:
    online_root = root / "crates/online-watcher/src"
    wasm_boundary_root = online_root / "wasm_api"
    shared_global_thread_adapter = wasm_boundary_root / "contracts/covenant/global_thread"
    core_global_thread = online_root / "transaction_builder/covenant/global_thread.rs"
    shared_global_thread_paths = [
        *shared_global_thread_adapter.glob("*.rs"),
        core_global_thread,
        online_root / "transaction_builder/pskb/thread_input.rs",
        online_root / "transaction_builder/pskb/thread_request.rs",
    ]
    return (
        _check_pskb_envelope_assembly(root, wasm_boundary_root)
        + _check_global_thread_adapters(root, wasm_boundary_root)
        + _check_shared_global_thread_adapter(root, shared_global_thread_adapter)
        + _check_core_global_thread_planner(core_global_thread)
        + _check_shared_global_thread_sizes(root, shared_global_thread_paths)
    )


def _check_pskb_envelope_assembly(root: Path, wasm_boundary_root: Path) -> list[str]:
    errors: list[str] = []
    for adapter_root in (wasm_boundary_root / "contracts", wasm_boundary_root / "privacy"):
        for adapter in adapter_root.rglob("*.rs"):
            source = adapter.read_text(errors="ignore")
            if 'extend_from_slice(b"PSKB")' in source or "serde_json::json!([pskt])" in source:
                errors.append(
                    f"PSKB envelope assembly bypasses the shared planner: {adapter.relative_to(root)}"
                )
    return errors


def _check_global_thread_adapters(root: Path, wasm_boundary_root: Path) -> list[str]:
    errors: list[str] = []
    for relative in _GLOBAL_THREAD_ADAPTER_PATHS:
        adapter = wasm_boundary_root / relative
        if adapter.exists():
            errors.extend(_check_global_thread_adapter(root, adapter))
    return errors


def _check_global_thread_adapter(root: Path, adapter: Path) -> list[str]:
    errors: list[str] = []
    source = adapter.read_text(errors="ignore")
    if len(source.splitlines()) > 180:
        errors.append(
            f"global-thread WASM adapter is too large: {adapter.relative_to(root)}"
        )
    for required_call in (
        "create_withdrawal",
        "create_topup",
    ):
        if required_call not in source:
            errors.append(
                f"global-thread adapter bypasses shared {required_call}: "
                f"{adapter.relative_to(root)}"
            )
    for forbidden_fragment in (
        '"previousOutpoint"',
        '"covenantBinding"',
        "encode_pskt_value",
    ):
        if forbidden_fragment in source:
            errors.append(
                f"global-thread adapter contains duplicated PSKB construction: "
                f"{adapter.relative_to(root)} contains {forbidden_fragment}"
            )
    return errors


def _check_shared_global_thread_adapter(root: Path, shared_global_thread_adapter: Path) -> list[str]:
    if not shared_global_thread_adapter.exists():
        return []
    shared_source = "\n".join(
        path.read_text(errors="ignore") for path in shared_global_thread_adapter.glob("*.rs")
    )
    return [
        f"global-thread business logic leaked into WASM boundary: {forbidden}"
        for forbidden in (
            "network::queries",
            "plan_global_thread_withdrawal",
            "plan_global_thread_topup",
            "fn decode_redeem_script",
            "fn decode_covenant_id",
            "fn parse_thread_utxo",
            "fn select_wallet_utxos",
        )
        if forbidden in shared_source
    ]


def _check_core_global_thread_planner(core_global_thread: Path) -> list[str]:
    if not core_global_thread.exists():
        return ["global-thread application planner must live under transaction_builder/covenant"]
    core_source = core_global_thread.read_text(errors="ignore")
    return [
        f"global-thread core planner lost {required_call}"
        for required_call in (
            "build_global_thread_withdrawal",
            "prepare_global_thread_topup_material",
            "build_global_thread_topup",
        )
        if required_call not in core_source
    ]


def _check_shared_global_thread_sizes(root: Path, shared_global_thread_paths: list[Path]) -> list[str]:
    errors: list[str] = []
    for shared_path in shared_global_thread_paths:
        if shared_path.exists() and len(shared_path.read_text(errors="ignore").splitlines()) > 300:
            errors.append(
                f"shared global-thread module is too large: {shared_path.relative_to(root)}"
            )
    return errors


def _check_wasm_business_boundary(root: Path) -> list[str]:
    """Keep transaction/covenant business policy out of browser bindings."""
    online_root = root / "crates/online-watcher/src"
    wasm_root = online_root / "wasm_api"
    errors = _check_required_core_paths(root, online_root)
    for path in wasm_root.rglob("*.rs"):
        if "unit_tests" in path.parts:
            continue
        errors.extend(_check_wasm_source_file(wasm_root, path))
    errors.extend(_check_wasm_adapter_limits(wasm_root))
    return errors


def _check_required_core_paths(root: Path, online_root: Path) -> list[str]:
    errors: list[str] = []
    for relative in _REQUIRED_CORE_PATHS:
        path = online_root / relative
        if not path.exists():
            errors.append(
                f"production WASM/domain extraction module is missing: {path.relative_to(root)}"
            )
    return errors


def _check_wasm_source_file(wasm_root: Path, path: Path) -> list[str]:
    errors: list[str] = []
    relative = path.relative_to(wasm_root)
    source = path.read_text(errors="ignore")
    if "network::queries" in source and relative not in _DIRECT_RPC_ALLOWLIST:
        errors.append(
            f"network-backed business acquisition leaked into WASM boundary: {relative}"
        )
    if _CHECKED_ARITHMETIC.search(source):
        errors.append(f"checked monetary/business arithmetic leaked into WASM boundary: {relative}")
    for fragment, description in _FORBIDDEN_WASM_FRAGMENTS:
        if fragment in source:
            errors.append(f"{description} leaked into WASM boundary: {relative}")
    if relative.as_posix().startswith(_CONTRACT_BUSINESS_ROOTS):
        errors.extend(_check_wasm_contract_source(relative, source))
    return errors


def _check_wasm_contract_source(relative: Path, source: str) -> list[str]:
    errors: list[str] = []
    code_source = "\n".join(
        line for line in source.splitlines()
        if not line.lstrip().startswith("//")
    )
    for fragment, description in _CONTRACT_BOUNDARY_FRAGMENTS:
        if fragment in code_source:
            errors.append(f"{description} leaked into WASM contract boundary: {relative}")
    if re.search(r"(?:build|create)_[A-Za-z0-9_]*(?:script|redeem)\s*\(", code_source):
        errors.append(f"covenant script construction leaked into WASM contract boundary: {relative}")
    return errors


def _check_wasm_adapter_limits(wasm_root: Path) -> list[str]:
    errors: list[str] = []
    for relative, limit in _WASM_ADAPTER_LIMITS.items():
        path = wasm_root / relative
        if path.exists() and len(path.read_text(errors="ignore").splitlines()) > limit:
            errors.append(f"WASM adapter is too large for a binding boundary: {relative}")
    return errors


def check(root: Path) -> list[str]:
    return [
        *_check_shared_pskb_planning(root),
        *_check_wasm_business_boundary(root),
    ]
