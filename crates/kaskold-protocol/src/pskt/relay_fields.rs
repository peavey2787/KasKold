use serde_json::{Map, Value};

use super::wire::{parse_exact_u64, parse_spk};
mod scripts;
mod signatures;
pub(crate) use scripts::*;
pub(crate) use signatures::*;

pub(crate) struct InputFields {
    pub previous_tx_id: [u8; 32],
    pub previous_index: u32,
    pub amount: u64,
    pub sequence: u64,
    pub sig_op_count: u8,
    pub script_version: u16,
    pub script_public_key: Vec<u8>,
    pub has_covenant_id: bool,
    pub redeem_script: Option<Vec<u8>>,
    pub partial_signatures: Map<String, Value>,
    pub covenant_execution: Option<(u16, u16)>,
}

impl InputFields {
    pub(crate) fn parse(value: &Value) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or_else(|| "input not object".to_string())?;
        let (amount, script_version, script_public_key, has_covenant_id) =
            parse_utxo_fields(object)?;
        let (previous_tx_id, previous_index) = parse_outpoint(object)?;
        let sequence = parse_sequence(object)?;
        let sig_op_count = parse_sig_op_count(object)?;
        let redeem_script = parse_redeem_script(object)?;
        let covenant_execution = parse_covenant_execution(object)?;
        validate_sighash(object)?;
        let partial_signatures = parse_partial_signatures(object)?;
        Ok(Self {
            previous_tx_id,
            previous_index,
            amount,
            sequence,
            sig_op_count,
            script_version,
            script_public_key,
            has_covenant_id,
            redeem_script,
            partial_signatures,
            covenant_execution,
        })
    }
}

fn validate_sighash(object: &Map<String, Value>) -> Result<(), String> {
    let sighash = parse_exact_u64(
        object
            .get("sighashType")
            .ok_or_else(|| "missing sighashType".to_string())?,
        "sighashType",
    )?;
    if sighash != u64::from(crate::wire::pskt_schema::SIGHASH_ALL) {
        return Err(format!("unsupported sighashType: {sighash}"));
    }
    Ok(())
}

fn parse_partial_signatures(object: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    let partial_signatures = match object.get("partialSigs") {
        None => Map::new(),
        Some(Value::Object(values)) => values.clone(),
        Some(_) => return Err("partialSigs must be an object".to_string()),
    };
    if partial_signatures.len() > usize::from(crate::SIGNER_CAPABILITIES.max_signatures_per_input) {
        return Err("too many partial signatures for signer capabilities".to_string());
    }
    Ok(partial_signatures)
}

fn parse_utxo_fields(object: &Map<String, Value>) -> Result<(u64, u16, Vec<u8>, bool), String> {
    let utxo = object
        .get("utxoEntry")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing utxoEntry".to_string())?;
    let amount = parse_utxo_amount(utxo)?;
    let (script_version, script_public_key) = parse_utxo_script(utxo)?;
    let has_covenant_id = parse_covenant_id_presence(utxo)?;
    Ok((amount, script_version, script_public_key, has_covenant_id))
}

fn parse_utxo_amount(utxo: &Map<String, Value>) -> Result<u64, String> {
    parse_exact_u64(
        utxo.get("amount")
            .ok_or_else(|| "missing amount".to_string())?,
        "amount",
    )
}

fn parse_utxo_script(utxo: &Map<String, Value>) -> Result<(u16, Vec<u8>), String> {
    let spk = utxo
        .get("scriptPublicKey")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing scriptPublicKey".to_string())?;
    let parsed = parse_spk(spk)?;
    if parsed.1.len() > 512 {
        return Err(format!(
            "spk too long for compact KSPT ({})",
            parsed.1.len()
        ));
    }
    Ok(parsed)
}

fn parse_covenant_id_presence(utxo: &Map<String, Value>) -> Result<bool, String> {
    let Some(value) = utxo.get("covenantId") else {
        return Ok(false);
    };
    let Some(text) = covenant_id_text(value)? else {
        return Ok(false);
    };
    validate_covenant_id(text)?;
    Ok(true)
}

fn covenant_id_text(value: &Value) -> Result<Option<&str>, String> {
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(Some)
        .ok_or_else(|| "utxoEntry.covenantId must be a hex string or null".to_string())
}

fn validate_covenant_id(text: &str) -> Result<(), String> {
    let bytes = super::wire::decode_lower_hex(text, "utxoEntry.covenantId")?;
    if bytes.len() != 32 {
        return Err("utxoEntry.covenantId must be 32 bytes".to_string());
    }
    Ok(())
}

fn parse_sequence(object: &Map<String, Value>) -> Result<u64, String> {
    object.get("sequence").map_or_else(
        || super::schema_validate::default_u64(crate::wire::pskt_schema::Scope::Input, "sequence"),
        |value| parse_exact_u64(value, "sequence"),
    )
}

fn parse_sig_op_count(object: &Map<String, Value>) -> Result<u8, String> {
    match object.get("sigOpCount") {
        None => u8::try_from(super::schema_validate::default_u64(
            crate::wire::pskt_schema::Scope::Input,
            "sigOpCount",
        )?)
        .map_err(|_| "shared sigOpCount default exceeds u8".to_string()),
        Some(value) => {
            let value = parse_exact_u64(value, "sigOpCount")?;
            let value = u8::try_from(value).map_err(|_| "sigOpCount exceeds u8".to_string())?;
            if value > crate::SIGNER_CAPABILITIES.max_signatures_per_input {
                return Err("sigOpCount exceeds signer capabilities".to_string());
            }
            Ok(value)
        }
    }
}

fn parse_covenant_execution(object: &Map<String, Value>) -> Result<Option<(u16, u16)>, String> {
    let Some(value) = object.get("covenantExecution") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let map = covenant_execution_object(value)?;
    let mask = covenant_execution_u16(map, "suppliedMask")?;
    let truth = covenant_execution_u16(map, "suppliedTrueMask")?;
    if truth & !mask != 0 {
        return Err("covenantExecution true-mask contains unsupplied decisions".to_string());
    }
    Ok(Some((mask, truth)))
}

fn covenant_execution_object(value: &Value) -> Result<&Map<String, Value>, String> {
    let map = value
        .as_object()
        .ok_or_else(|| "covenantExecution must be an object or null".to_string())?;
    if map.len() != 2 || !map.contains_key("suppliedMask") || !map.contains_key("suppliedTrueMask")
    {
        return Err(
            "covenantExecution must contain only suppliedMask and suppliedTrueMask".to_string(),
        );
    }
    Ok(map)
}

fn covenant_execution_u16(map: &Map<String, Value>, key: &str) -> Result<u16, String> {
    let label = format!("covenantExecution.{key}");
    let value = parse_exact_u64(
        map.get(key).ok_or_else(|| format!("missing {label}"))?,
        &label,
    )?;
    u16::try_from(value).map_err(|_| format!("{label} exceeds u16"))
}

fn parse_redeem_script(object: &Map<String, Value>) -> Result<Option<Vec<u8>>, String> {
    let redeem = match object.get("redeemScript") {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(value)) => super::wire::decode_lower_hex(value, "redeemScript")?,
        Some(_) => return Err("redeemScript must be a hex string or null".to_string()),
    };
    if redeem.len() > usize::from(crate::SIGNER_CAPABILITIES.max_redeem_script_bytes) {
        return Err("redeemScript exceeds signer capabilities".to_string());
    }
    Ok(Some(redeem))
}

fn parse_outpoint(object: &Map<String, Value>) -> Result<([u8; 32], u32), String> {
    let outpoint = object
        .get("previousOutpoint")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing previousOutpoint".to_string())?;
    let transaction_id = outpoint
        .get("transactionId")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing transactionId".to_string())?;
    let bytes = super::wire::decode_lower_hex(transaction_id, "transactionId")?;
    let previous_tx_id = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "tx_id not 32 bytes".to_string())?;
    let index = outpoint
        .get("index")
        .ok_or_else(|| "missing index".to_string())?;
    let previous_index = u32::try_from(parse_exact_u64(index, "previousOutpoint.index")?)
        .map_err(|_| "index exceeds u32".to_string())?;
    Ok((previous_tx_id, previous_index))
}
