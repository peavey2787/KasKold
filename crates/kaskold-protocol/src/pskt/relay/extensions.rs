//! KSPT trailer extensions: derivations, MuSig-45 paths, stealth and covenant bindings.

use super::{parse_derivation, parse_exact_u64, parse_ms45, Params, Transaction, Value};

pub(crate) fn apply_derivations(
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

pub(crate) fn apply_ms45(
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

pub(crate) fn find_stealth(inputs: &[Value]) -> Result<Option<[u8; 32]>, String> {
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
            super::super::wire::decode_lower_hex(text, &format!("input[{position}] stealthTweak"))?;
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

pub(crate) fn apply_covenants(
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

pub(crate) fn persistent_vault_requested(inputs: &[Value]) -> Result<bool, String> {
    let mut persistent = false;
    for (position, input) in inputs.iter().enumerate() {
        persistent |= input_persistent_vault(input, position)?;
    }
    Ok(persistent)
}

pub(crate) fn input_persistent_vault(input: &Value, position: usize) -> Result<bool, String> {
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

pub(crate) fn apply_persistent_vault_covenant(
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

pub(crate) fn apply_explicit_covenants(
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

pub(crate) fn parse_explicit_covenant_binding(
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

pub(crate) fn parse_authorizing_input(
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

pub(crate) fn parse_explicit_covenant_id(
    binding: &serde_json::Map<String, Value>,
    position: usize,
) -> Result<[u8; 32], String> {
    let text = binding
        .get("covenantId")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("output[{position}] covenant binding is missing covenantId"))?;
    let bytes =
        super::super::wire::decode_lower_hex(text, &format!("output[{position}] covenant id"))?;
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
    let bytes = super::super::wire::decode_lower_hex(transaction_id, "input[0] transactionId")?;
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

pub(crate) fn covenant_id(
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
