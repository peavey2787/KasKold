use blake2b_simd::Params;
use serde_json::{Map, Value};

use crate::{wire::kspt, Network};

use super::{
    compact::{Input, Output, Signature, Transaction},
    relay_fields::{collect_signatures, parse_ms45, InputFields},
    wire::{document, parse_derivation, parse_exact_u64, parse_spk},
};
mod build;
mod extensions;
pub(crate) use build::*;
pub(crate) use extensions::*;

pub(crate) fn encode_pskt(pskt_hex: &str, network: Network) -> Result<Vec<u8>, String> {
    let (format, root) = super::wire::decode(pskt_hex)?;
    let document = document(&root, format)?
        .as_object()
        .ok_or_else(|| "PSKT not object".to_string())?;
    super::schema_validate::validate_document(document)?;
    let global = document
        .get("global")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing global".to_string())?;
    let inputs = document
        .get("inputs")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing inputs".to_string())?;
    let outputs = document
        .get("outputs")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing outputs".to_string())?;
    let transaction = build_transaction(global, inputs, outputs, network)?;
    kspt::encode_vec(&transaction, kspt::SIGNER_LIMITS).map_err(|error| error.to_string())
}

pub(crate) fn verified_signature_counts(
    pskt_hex: &str,
    network: Network,
) -> Result<Vec<u8>, String> {
    let transaction = build_verified_transaction(pskt_hex, network)?;
    transaction
        .inputs
        .iter()
        .enumerate()
        .map(|(index, _)| verified_signature_count_u8(&transaction, index))
        .collect()
}

fn verified_signature_count_u8(transaction: &Transaction, index: usize) -> Result<u8, String> {
    let count = super::compact::verified_signature_count_for_input(transaction, index)?;
    u8::try_from(count).map_err(|_| format!("input[{index}] signature count exceeds u8"))
}

pub(crate) fn is_complete(pskt_hex: &str, network: Network) -> Result<bool, String> {
    let transaction = build_verified_transaction(pskt_hex, network)?;
    super::compact::verified_complete(&transaction)
}

/// Parse exactly once, reject legacy dual-routing covenant metadata, verify every
/// signature/branch invariant, and return the semantic transaction that was
/// authorized. Consumer finalizers must never reparse the source after this call.
pub(crate) fn verify_complete_transaction(
    pskt_hex: &str,
    network: Network,
) -> Result<Transaction, String> {
    let transaction = build_verified_transaction(pskt_hex, network)?;
    if !super::compact::verified_complete(&transaction)? {
        return Err("PSKT is not cryptographically complete".to_string());
    }
    Ok(transaction)
}

fn build_verified_transaction(pskt_hex: &str, network: Network) -> Result<Transaction, String> {
    let (format, root) = super::wire::decode(pskt_hex)?;
    let document = document(&root, format)?
        .as_object()
        .ok_or_else(|| "PSKT not object".to_string())?;
    super::schema_validate::validate_document(document)?;
    let (global, inputs, outputs) = document_parts(document)?;
    validate_counts(inputs, outputs)?;
    let mut transaction = build_base_transaction(global, inputs, outputs, network)?;
    apply_extensions(&mut transaction, inputs, outputs)?;
    super::specialized::bind_specialized_witnesses(global, inputs, &mut transaction)?;
    Ok(transaction)
}

/// The PSKT document's global map plus its input and output arrays.
type DocumentParts<'a> = (&'a Map<String, Value>, &'a [Value], &'a [Value]);

fn document_parts(document: &Map<String, Value>) -> Result<DocumentParts<'_>, String> {
    let global = document
        .get("global")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing global".to_string())?;
    let inputs = document
        .get("inputs")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing inputs".to_string())?;
    let outputs = document
        .get("outputs")
        .and_then(Value::as_array)
        .ok_or_else(|| "missing outputs".to_string())?;
    Ok((global, inputs, outputs))
}

fn build_transaction(
    global: &Map<String, Value>,
    inputs: &[Value],
    outputs: &[Value],
    network: Network,
) -> Result<Transaction, String> {
    validate_counts(inputs, outputs)?;
    let mut transaction = build_base_transaction(global, inputs, outputs, network)?;
    apply_extensions(&mut transaction, inputs, outputs)?;
    transaction.flags = if super::compact::verified_complete(&transaction)? {
        kspt::FLAG_SIGNED_OR_COMPLETE
    } else {
        0
    };
    Ok(transaction)
}

fn validate_counts(inputs: &[Value], outputs: &[Value]) -> Result<(), String> {
    if inputs.len() > usize::from(crate::SIGNER_CAPABILITIES.max_inputs) {
        return Err(format!(
            "too many inputs for signer capabilities: {} > {}",
            inputs.len(),
            crate::SIGNER_CAPABILITIES.max_inputs
        ));
    }
    if outputs.len() > usize::from(crate::SIGNER_CAPABILITIES.max_outputs) {
        return Err(format!(
            "too many outputs for signer capabilities: {} > {}",
            outputs.len(),
            crate::SIGNER_CAPABILITIES.max_outputs
        ));
    }
    Ok(())
}

fn build_base_transaction(
    global: &Map<String, Value>,
    inputs: &[Value],
    outputs: &[Value],
    network: Network,
) -> Result<Transaction, String> {
    Ok(Transaction {
        flags: 0,
        version: tx_version(global)?,
        locktime: optional_exact(global, "fallbackLockTime")?,
        subnetwork: decode_subnetwork(global)?,
        gas: optional_exact(global, "gas")?,
        payload: decode_payload(global)?,
        network: network.kspt_code(),
        inputs: build_inputs(inputs)?,
        outputs: build_outputs(outputs)?,
        stealth: find_stealth(inputs)?,
    })
}

fn apply_extensions(
    transaction: &mut Transaction,
    inputs: &[Value],
    outputs: &[Value],
) -> Result<(), String> {
    apply_ms45(transaction, inputs, outputs)?;
    apply_derivations(transaction, inputs, outputs)?;
    apply_covenants(transaction, inputs, outputs)
}

#[cfg(test)]
mod unit_tests;
