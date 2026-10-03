use crate::wasm_api::JsValue;

// ============================================================================
// Oracle (Model B) -- publish WASM boundary.
//
// The publish is the proof-bearing ROLL: it spends the singleton oracle UTXO via
// the committed-guest RISC0 succinct proof and RECREATES the oracle at the new
// price baked into the new redeem. Structurally it is your rollup_state_advance
// (ZK + txVersion=1 + recreate) with the covenant_id continuation tag from
// create_global_spending_limit_withdraw (the oracle must keep its covenant_id so
// consumers can locate it by OpCovInputIdx). Both halves are paths you already
// run on TN10; only the finalize dispatch branch (risc0OracleMb, in the
// companion PSKT subsystem path) is new, and it delegates to the byte-tested
// crate::contracts::oracle::script::build_oracle_mb_publish_sig_script.
//
// FEE MODEL (PoC/TN10): the fee is taken from the oracle value (single output =
// in - fee), matching the body's out >= in - max_fee. The conserve-value
// hardening (separate fee-payer input, out >= in) is the mainnet change, banked.
//
// PREREQUISITES: the oracle genesis UTXO exists and carries covenant_id G (bound
// by the tx_version=1 genesis funding). Read G from the oracle UTXO and pass it
// as covenant_id_hex. The 48-byte raw journal is the host's receipt journal:
//   price[0:8] LE | T (publish_time)[8:16] LE | set_root[16:48].
// ============================================================================

/// Oracle (Model B) PUBLISH: advance the oracle to the price proven in `journal`.
///
/// Spends the singleton oracle UTXO at `oracle_address` (revealing
/// `redeem_script_hex`, priced at the OLD price) and recreates the oracle UTXO at
/// the NEW price/T read from `journal`, tagged with the SAME covenant_id
/// (continuation). The keyless ROLL branch carries the RISC0 proof
/// (seal + claim + control_index + control_digests + journal); image_id /
/// control_id / set_root / hashfn are committed in the redeem and consumed by
/// OP_ZK_PRECOMPILE from there.
///
/// This route is currently fail-closed: the keyless branch is not exposed to
/// consumers until it has a typed zero-signature `VerifiedWitnessPlan`.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn create_oracle_mb_publish(request_json: &str) -> Result<String, JsValue> {
    let _ = request_json;
    Err(wasm_error!("Oracle MB publish is disabled until its zero-signature branch has a typed verified witness plan"))
}

#[cfg(test)]
mod unit_tests;
