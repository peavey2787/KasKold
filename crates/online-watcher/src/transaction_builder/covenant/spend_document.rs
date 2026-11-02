//! A covenant spend the Companion shapes itself: its script path, its coins and
//! its outputs. Encoding happens here so a document always names a path the
//! script can run and carries the signature count that path checks.

use serde::Deserialize;

use super::{
    execution::{execution_named, signatures_required},
    redeem::{decode_redeem_script, require_redeem_for_address},
};
use crate::{
    account::address::address_to_script_pubkey,
    account::utxo::UtxoEntry,
    transaction_builder::pskb::{
        encode_wire, selected_utxo_entry, CovenantInputPolicy, PskbGlobalPlan, PskbInputPlan,
        PskbOutputPlan, PskbPlan, SelectedUtxo,
    },
};

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpendOutput {
    address: String,
    #[serde(with = "crate::serialization::decimal_u64")]
    amount: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SpendDocument {
    covenant_address: String,
    redeem_script_hex: String,
    path: String,
    tx_version: u16,
    #[serde(with = "crate::serialization::decimal_u64")]
    lock_time: u64,
    payload_hex: String,
    utxos: Vec<SelectedUtxo>,
    outputs: Vec<SpendOutput>,
}

fn funded_inputs(document: &mut SpendDocument) -> Result<Vec<UtxoEntry>, String> {
    if document.utxos.is_empty() {
        return Err("No UTXOs at covenant address".to_string());
    }
    std::mem::take(&mut document.utxos)
        .into_iter()
        .map(selected_utxo_entry)
        .collect()
}

fn encoded_outputs(document: &SpendDocument) -> Result<Vec<PskbOutputPlan>, String> {
    if document.outputs.is_empty() {
        return Err("A covenant spend needs an output".to_string());
    }
    document
        .outputs
        .iter()
        .map(|output| {
            let script = address_to_script_pubkey(&output.address)?;
            Ok(PskbOutputPlan::plain(output.amount, &script))
        })
        .collect()
}

fn sum(mut amounts: impl Iterator<Item = u64>) -> Result<u64, String> {
    amounts
        .try_fold(0u64, u64::checked_add)
        .ok_or_else(|| "Covenant spend total overflows".to_string())
}

fn require_covered_outputs(utxos: &[UtxoEntry], document: &SpendDocument) -> Result<(), String> {
    let spent = sum(document.outputs.iter().map(|output| output.amount))?;
    if spent > sum(utxos.iter().map(|utxo| utxo.amount))? {
        return Err("Covenant spend outputs exceed its inputs".to_string());
    }
    Ok(())
}

pub(crate) fn encode_json(request_json: &str) -> Result<String, String> {
    let mut document: SpendDocument = serde_json::from_str(request_json)
        .map_err(|error| format!("Bad covenant spend JSON: {error}"))?;
    let redeem = decode_redeem_script(&document.redeem_script_hex)?;
    require_redeem_for_address(&document.covenant_address, &redeem)?;
    let execution = execution_named(&redeem, &document.path)?;
    let covenant_script = address_to_script_pubkey(&document.covenant_address)?;
    let utxos = funded_inputs(&mut document)?;
    require_covered_outputs(&utxos, &document)?;
    let payload = hex::decode(&document.payload_hex)
        .map_err(|error| format!("Bad covenant spend payload: {error}"))?;
    let inputs = utxos
        .into_iter()
        .map(|utxo| {
            PskbInputPlan::covenant(
                utxo,
                &covenant_script,
                &redeem,
                CovenantInputPolicy {
                    sequence: 0,
                    // One signature operation buys the compute budget every
                    // covenant path needs, the keyless ones included.
                    sig_op_count: 1,
                    minimum_signatures: signatures_required(&redeem, &execution),
                    proprietaries: serde_json::json!({}),
                    min_time: Some(0),
                    execution: Some(execution),
                },
            )
        })
        .collect();
    encode_wire(&PskbPlan {
        global: PskbGlobalPlan {
            tx_version: document.tx_version,
            fallback_lock_time: (document.lock_time > 0).then_some(document.lock_time),
            proprietaries: serde_json::json!({}),
            transaction_payload: (!payload.is_empty()).then_some(payload),
        },
        inputs,
        outputs: encoded_outputs(&document)?,
    })
}
