//! Oracle-v1 claim transaction planning.

use super::{
    execution::{execution_for, SpendPath},
    redeem::require_redeem_for_address,
};
use crate::transaction_builder::pskb::{
    encode_prepared_sweep, prepare_sweep_from_utxos, PreparedSweep, PskbGlobalPlan,
    SweepInputPolicy,
};

pub(crate) struct OracleClaimRequest<'a> {
    pub covenant_address: &'a str,
    pub destination_address: &'a str,
    pub redeem_script_hex: &'a str,
    pub oracle_pubkey_hex: &'a str,
    pub oracle_signature_hex: &'a str,
    pub message_commitment_hex: &'a str,
    pub fee: u64,
    pub websocket_url: &'a str,
}

pub(crate) async fn build_claim(
    request: OracleClaimRequest<'_>,
) -> Result<(PreparedSweep, String), String> {
    let redeem = crate::contracts::covenant::oracle_v1::checked_redeem_and_attestation(
        request.redeem_script_hex,
        request.oracle_pubkey_hex,
        request.oracle_signature_hex,
        request.message_commitment_hex,
    )?;
    require_redeem_for_address(request.covenant_address, &redeem)?;
    let utxos = crate::network::queries::utxos::fetch_for_address(
        request.websocket_url,
        request.covenant_address,
    )
    .await?;
    let prepared = prepare_sweep_from_utxos(
        utxos,
        request.covenant_address,
        request.destination_address,
        request.fee,
        "No UTXOs at oracle covenant address",
        "Oracle covenant balance too low to cover fee",
    )?;
    let wire = encode_claim(&prepared, &redeem, request.oracle_signature_hex)?;
    Ok((prepared, wire))
}

/// The claim's witness route: the attestation, then the beneficiary's signature.
pub(crate) fn encode_claim(
    prepared: &PreparedSweep,
    redeem: &[u8],
    oracle_signature_hex: &str,
) -> Result<String, String> {
    let mut policy =
        SweepInputPolicy::covenant(redeem, 0, execution_for(redeem, SpendPath::Key(1))?);
    policy.proprietaries = serde_json::json!({
        "oracleV1Claim": true,
        "oracleV1Signature": oracle_signature_hex,
    });
    policy.sig_op_count = crate::contracts::covenant::script::ORACLE_V1_SIG_OP_COUNT;
    encode_prepared_sweep(prepared, PskbGlobalPlan::standard(), &policy)
}
