//! Private-swap claim transaction planning.

use crate::transaction_builder::pskb::{
    encode_prepared_sweep, prepare_selected_sweep, PreparedSweep, PskbGlobalPlan, SweepInputPolicy,
};

pub(crate) fn prepare_claim(
    covenant_address: &str,
    destination_address: &str,
    redeem_script_hex: &str,
    selected_utxo_json: &str,
    fee: u64,
) -> Result<(PreparedSweep, Vec<u8>), String> {
    let redeem = hex::decode(redeem_script_hex).map_err(|error| format!("Bad redeem: {error}"))?;
    let prepared = prepare_selected_sweep(
        selected_utxo_json,
        covenant_address,
        destination_address,
        fee,
        "Private Swap funding UTXO missing",
        "Private Swap balance too low",
    )?;
    if prepared.utxos.len() != 1 {
        return Err("Private Swap claim requires exactly one funding UTXO".to_string());
    }
    Ok((prepared, redeem))
}

pub(crate) fn build_claim(
    covenant_address: &str,
    destination_address: &str,
    redeem_script_hex: &str,
    selected_utxo_json: &str,
    fee: u64,
) -> Result<String, String> {
    let (prepared, redeem) = prepare_claim(
        covenant_address,
        destination_address,
        redeem_script_hex,
        selected_utxo_json,
        fee,
    )?;
    let global = PskbGlobalPlan::standard().with_branch("beneficiary");
    let mut policy =
        SweepInputPolicy::covenant(&redeem, 0, serde_json::json!({"privateSwapClaim": true}));
    policy.sig_op_count = 1;
    encode_prepared_sweep(&prepared, global, &policy)
}

pub(crate) fn insert_completed_signature_hex(
    pskb_hex: &str,
    claim_pubkey_x: &[u8; 32],
    signature: &[u8; 64],
) -> Result<String, String> {
    let mut public_key = [0x02; 33];
    public_key[1..].copy_from_slice(claim_pubkey_x);
    kaspa_portal::transaction::interchange::pskt::pipeline::attach_sole_signature(
        pskb_hex,
        &public_key,
        signature,
    )
}

#[cfg(test)]
mod unit_tests;
