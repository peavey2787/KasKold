use blake2b_simd::Params;
use serde_json::{Map, Value};

use crate::{wire::kspt, Network};

use super::{
    compact::{Input, Output, Signature, Transaction},
    relay_fields::{collect_signatures, parse_ms45, InputFields},
    wire::{document, parse_derivation, parse_exact_u64, parse_spk},
};

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
    kspt::encode_vec(&transaction).map_err(|error| error.to_string())
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

fn tx_version(global: &Map<String, Value>) -> Result<u16, String> {
    let value = global
        .get("txVersion")
        .ok_or_else(|| "missing txVersion".to_string())?;
    let value = parse_exact_u64(value, "txVersion")?;
    let version = u16::try_from(value).map_err(|_| "txVersion exceeds u16".to_string())?;
    if !crate::wire::pskt_schema::supported_tx_version(version) {
        return Err(format!("unsupported transaction version: {version}"));
    }
    Ok(version)
}

fn optional_exact(global: &Map<String, Value>, key: &str) -> Result<u64, String> {
    match global.get(key) {
        None | Some(Value::Null) => {
            super::schema_validate::default_u64(crate::wire::pskt_schema::Scope::Global, key)
        }
        Some(value) => parse_exact_u64(value, key),
    }
}

fn decode_subnetwork(global: &Map<String, Value>) -> Result<[u8; 20], String> {
    match global.get("subnetworkId") {
        None | Some(Value::Null) => match crate::wire::pskt_schema::default_rule(
            crate::wire::pskt_schema::Scope::Global,
            b"subnetworkId",
        ) {
            crate::wire::pskt_schema::DefaultRule::NativeSubnetwork => Ok([0u8; 20]),
            _ => Err("subnetworkId has no shared native-subnetwork default".to_string()),
        },
        Some(Value::String(value)) => {
            let bytes = super::wire::decode_lower_hex(value, "subnetworkId")?;
            bytes
                .as_slice()
                .try_into()
                .map_err(|_| "subnetworkId must be 20 bytes".to_string())
        }
        _ => Err("subnetworkId must be a hex string".to_string()),
    }
}

fn decode_payload(global: &Map<String, Value>) -> Result<Vec<u8>, String> {
    let payload = match global.get("txPayload") {
        None | Some(Value::Null) => match crate::wire::pskt_schema::default_rule(
            crate::wire::pskt_schema::Scope::Global,
            b"txPayload",
        ) {
            crate::wire::pskt_schema::DefaultRule::EmptyBytes => Vec::new(),
            _ => return Err("txPayload has no shared empty-byte default".to_string()),
        },
        Some(Value::String(value)) => super::wire::decode_lower_hex(value, "txPayload")?,
        Some(_) => return Err("txPayload must be a hex string or null".to_string()),
    };
    if payload.len() > usize::from(crate::SIGNER_CAPABILITIES.max_payload_bytes) {
        return Err(format!(
            "transaction payload exceeds signer capabilities: {} > {}",
            payload.len(),
            crate::SIGNER_CAPABILITIES.max_payload_bytes
        ));
    }
    Ok(payload)
}

fn build_inputs(values: &[Value]) -> Result<Vec<Input>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            build_input(value).map_err(|error| format!("input[{index}]: {error}"))
        })
        .collect()
}

fn build_input(value: &Value) -> Result<Input, String> {
    let fields = InputFields::parse(value)?;
    let signatures = collect_signatures(&fields)?
        .into_iter()
        .map(|entry| Signature {
            position: entry.position,
            sighash: 0x01,
            bytes: entry.bytes,
        })
        .collect();
    Ok(Input {
        tx_id: fields.previous_tx_id,
        index: fields.previous_index,
        amount: fields.amount,
        sequence: fields.sequence,
        sig_op_count: fields.sig_op_count,
        script_version: fields.script_version,
        script: fields.script_public_key,
        has_covenant_id: fields.has_covenant_id,
        signatures,
        redeem: fields.redeem_script.unwrap_or_default(),
        derivation: None,
        ms45: None,
        covenant_execution: fields.covenant_execution,
        specialized_witness: None,
    })
}

fn build_outputs(values: &[Value]) -> Result<Vec<Output>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            build_output(value).map_err(|error| format!("output[{index}]: {error}"))
        })
        .collect()
}

fn build_output(value: &Value) -> Result<Output, String> {
    let object = value
        .as_object()
        .ok_or_else(|| "output not object".to_string())?;
    let amount = parse_exact_u64(
        object
            .get("amount")
            .ok_or_else(|| "missing amount".to_string())?,
        "amount",
    )?;
    let spk = object
        .get("scriptPublicKey")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing scriptPublicKey".to_string())?;
    let (script_version, script) = parse_spk(spk)?;
    if script.len() > kspt::MAX_SCRIPT_SIZE {
        return Err(format!(
            "script too long for compact KSPT ({})",
            script.len()
        ));
    }
    Ok(Output {
        amount,
        script_version,
        script,
        derivation: None,
        ms45: None,
        covenant: None,
    })
}

fn apply_derivations(
    transaction: &mut Transaction,
    inputs: &[Value],
    outputs: &[Value],
) -> Result<(), String> {
    for (position, value) in inputs.iter().enumerate() {
        if let Some(proprietaries) = value.get("proprietaries") {
            if let Some(hint) = parse_derivation(proprietaries)
                .map_err(|error| format!("input[{position}] {error}"))?
            {
                transaction.inputs[position].derivation = Some(hint);
            }
        }
    }
    for (position, value) in outputs.iter().enumerate() {
        if let Some(proprietaries) = value.get("proprietaries") {
            if let Some(hint) = parse_derivation(proprietaries)
                .map_err(|error| format!("output[{position}] {error}"))?
            {
                transaction.outputs[position].derivation = Some(hint);
            }
        }
    }
    Ok(())
}

fn apply_ms45(
    transaction: &mut Transaction,
    inputs: &[Value],
    outputs: &[Value],
) -> Result<(), String> {
    for (position, value) in inputs.iter().enumerate() {
        if let Some(field) = value.get("bip32Derivations") {
            transaction.inputs[position].ms45 =
                parse_ms45(field).map_err(|error| format!("input[{position}] {error}"))?;
        }
    }
    for (position, value) in outputs.iter().enumerate() {
        if let Some(field) = value.get("bip32Derivations") {
            transaction.outputs[position].ms45 =
                parse_ms45(field).map_err(|error| format!("output[{position}] {error}"))?;
        }
    }
    Ok(())
}

fn find_stealth(inputs: &[Value]) -> Result<Option<[u8; 32]>, String> {
    let mut found = None;
    for (position, input) in inputs.iter().enumerate() {
        let Some(proprietaries) = input.get("proprietaries") else {
            continue;
        };
        let proprietaries = proprietaries
            .as_object()
            .ok_or_else(|| format!("input[{position}] proprietaries must be an object"))?;
        let Some(value) = proprietaries.get("stealthTweak") else {
            continue;
        };
        let text = value
            .as_str()
            .ok_or_else(|| format!("input[{position}] stealthTweak must be a hex string"))?;
        let bytes =
            super::wire::decode_lower_hex(text, &format!("input[{position}] stealthTweak"))?;
        let tweak: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| format!("input[{position}] stealthTweak must be 32 bytes"))?;
        match found {
            None => found = Some(tweak),
            Some(existing) if existing == tweak => {}
            Some(_) => return Err("inputs contain conflicting stealth tweaks".to_string()),
        }
    }
    Ok(found)
}

fn apply_covenants(
    transaction: &mut Transaction,
    inputs: &[Value],
    outputs: &[Value],
) -> Result<(), String> {
    apply_explicit_covenants(transaction, outputs)?;
    if transaction
        .outputs
        .first()
        .is_some_and(|output| output.covenant.is_some())
    {
        return Ok(());
    }
    if !persistent_vault_requested(inputs)? {
        return Ok(());
    }
    apply_persistent_vault_covenant(transaction, inputs)
}

fn persistent_vault_requested(inputs: &[Value]) -> Result<bool, String> {
    let mut persistent = false;
    for (position, input) in inputs.iter().enumerate() {
        persistent |= input_persistent_vault(input, position)?;
    }
    Ok(persistent)
}

fn input_persistent_vault(input: &Value, position: usize) -> Result<bool, String> {
    let Some(proprietaries) = input.get("proprietaries") else {
        return Ok(false);
    };
    let proprietaries = proprietaries
        .as_object()
        .ok_or_else(|| format!("input[{position}] proprietaries must be an object"))?;
    let Some(value) = proprietaries.get("persistentVault") else {
        return Ok(false);
    };
    value
        .as_bool()
        .ok_or_else(|| format!("input[{position}] persistentVault must be boolean"))
}

fn apply_persistent_vault_covenant(
    transaction: &mut Transaction,
    inputs: &[Value],
) -> Result<(), String> {
    let (tx_id, prev_index) = first_outpoint(inputs)?
        .ok_or_else(|| "persistent covenant requires an authorizing input".to_string())?;
    let Some(output) = transaction.outputs.first() else {
        return Ok(());
    };
    let id = covenant_id(
        &tx_id,
        prev_index,
        0,
        output.amount,
        output.script_version,
        &output.script,
    );
    transaction.outputs[0].covenant = Some((0, id));
    Ok(())
}

fn apply_explicit_covenants(
    transaction: &mut Transaction,
    outputs: &[Value],
) -> Result<(), String> {
    for (position, value) in outputs.iter().enumerate() {
        let Some(binding) =
            parse_explicit_covenant_binding(value, position, transaction.inputs.len())?
        else {
            continue;
        };
        transaction.outputs[position].covenant = Some(binding);
    }
    Ok(())
}

fn parse_explicit_covenant_binding(
    value: &Value,
    position: usize,
    input_count: usize,
) -> Result<Option<(u16, [u8; 32])>, String> {
    let binding = match value.get("covenantBinding") {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::Object(binding)) => binding,
        Some(_) => {
            return Err(format!(
                "output[{position}] covenantBinding must be an object or null"
            ))
        }
    };
    let authorizing = parse_authorizing_input(binding, position, input_count)?;
    let id = parse_explicit_covenant_id(binding, position)?;
    Ok(Some((authorizing, id)))
}

fn parse_authorizing_input(
    binding: &serde_json::Map<String, Value>,
    position: usize,
    input_count: usize,
) -> Result<u16, String> {
    let value = binding.get("authorizingInput").ok_or_else(|| {
        format!("output[{position}] covenant binding is missing authorizingInput")
    })?;
    let authorizing = parse_exact_u64(value, "authorizingInput")?;
    let authorizing = u16::try_from(authorizing)
        .map_err(|_| format!("output[{position}] covenant authorizing input exceeds u16"))?;
    if usize::from(authorizing) >= input_count {
        return Err(format!(
            "output[{position}] covenant authorizing input is out of range"
        ));
    }
    Ok(authorizing)
}

fn parse_explicit_covenant_id(
    binding: &serde_json::Map<String, Value>,
    position: usize,
) -> Result<[u8; 32], String> {
    let text = binding
        .get("covenantId")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("output[{position}] covenant binding is missing covenantId"))?;
    let bytes = super::wire::decode_lower_hex(text, &format!("output[{position}] covenant id"))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| format!("output[{position}] covenant id must be 32 bytes"))
}

pub(crate) fn first_outpoint(inputs: &[Value]) -> Result<Option<([u8; 32], u32)>, String> {
    let Some(input) = inputs.first() else {
        return Ok(None);
    };
    let input = input
        .as_object()
        .ok_or_else(|| "input[0] must be an object".to_string())?;
    let outpoint = input
        .get("previousOutpoint")
        .and_then(Value::as_object)
        .ok_or_else(|| "input[0] previousOutpoint must be an object".to_string())?;
    let transaction_id = outpoint
        .get("transactionId")
        .and_then(Value::as_str)
        .ok_or_else(|| "input[0] transactionId must be a hex string".to_string())?;
    let bytes = super::wire::decode_lower_hex(transaction_id, "input[0] transactionId")?;
    let id = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "input[0] transactionId must be 32 bytes".to_string())?;
    let index_value = outpoint
        .get("index")
        .ok_or_else(|| "input[0] previousOutpoint.index is missing".to_string())?;
    let index = u32::try_from(parse_exact_u64(index_value, "previousOutpoint.index")?)
        .map_err(|_| "input[0] previousOutpoint.index must be a u32".to_string())?;
    Ok(Some((id, index)))
}

fn covenant_id(
    prev_tx_id: &[u8; 32],
    prev_index: u32,
    output_index: u32,
    value: u64,
    version: u16,
    script: &[u8],
) -> [u8; 32] {
    let hash = Params::new()
        .hash_length(32)
        .key(b"CovenantID")
        .to_state()
        .update(prev_tx_id)
        .update(&prev_index.to_le_bytes())
        .update(&1u64.to_le_bytes())
        .update(&output_index.to_le_bytes())
        .update(&value.to_le_bytes())
        .update(&version.to_le_bytes())
        .update(&(script.len() as u64).to_le_bytes())
        .update(script)
        .finalize();
    let mut output = [0u8; 32];
    output.copy_from_slice(hash.as_bytes());
    output
}

#[cfg(test)]
mod unit_tests;
