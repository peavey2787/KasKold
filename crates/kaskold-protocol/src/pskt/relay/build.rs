//! Typed transaction fields read from a PSKT document for KSPT relay.

use super::{
    collect_signatures, kspt, parse_exact_u64, parse_spk, Input, InputFields, Map, Output,
    Signature, Value,
};

pub(crate) fn tx_version(global: &Map<String, Value>) -> Result<u16, String> {
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

pub(crate) fn optional_exact(global: &Map<String, Value>, key: &str) -> Result<u64, String> {
    match global.get(key) {
        None | Some(Value::Null) => {
            super::super::schema_validate::default_u64(crate::wire::pskt_schema::Scope::Global, key)
        }
        Some(value) => parse_exact_u64(value, key),
    }
}

pub(crate) fn decode_subnetwork(global: &Map<String, Value>) -> Result<[u8; 20], String> {
    match global.get("subnetworkId") {
        None | Some(Value::Null) => match crate::wire::pskt_schema::default_rule(
            crate::wire::pskt_schema::Scope::Global,
            b"subnetworkId",
        ) {
            crate::wire::pskt_schema::DefaultRule::NativeSubnetwork => Ok([0u8; 20]),
            _ => Err("subnetworkId has no shared native-subnetwork default".to_string()),
        },
        Some(Value::String(value)) => {
            let bytes = super::super::wire::decode_lower_hex(value, "subnetworkId")?;
            bytes
                .as_slice()
                .try_into()
                .map_err(|_| "subnetworkId must be 20 bytes".to_string())
        }
        _ => Err("subnetworkId must be a hex string".to_string()),
    }
}

pub(crate) fn decode_payload(global: &Map<String, Value>) -> Result<Vec<u8>, String> {
    let payload = match global.get("txPayload") {
        None | Some(Value::Null) => match crate::wire::pskt_schema::default_rule(
            crate::wire::pskt_schema::Scope::Global,
            b"txPayload",
        ) {
            crate::wire::pskt_schema::DefaultRule::EmptyBytes => Vec::new(),
            _ => return Err("txPayload has no shared empty-byte default".to_string()),
        },
        Some(Value::String(value)) => super::super::wire::decode_lower_hex(value, "txPayload")?,
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

pub(crate) fn build_inputs(values: &[Value]) -> Result<Vec<Input>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            build_input(value).map_err(|error| format!("input[{index}]: {error}"))
        })
        .collect()
}

pub(crate) fn build_input(value: &Value) -> Result<Input, String> {
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

pub(crate) fn build_outputs(values: &[Value]) -> Result<Vec<Output>, String> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            build_output(value).map_err(|error| format!("output[{index}]: {error}"))
        })
        .collect()
}

pub(crate) fn build_output(value: &Value) -> Result<Output, String> {
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
