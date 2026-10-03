//! Merge a signed compact KSPT back into its original PSKT.

use super::{
    find_pubkey_position, parse, verify_all_signatures, wire, Input, Map, Network, Signature,
    Transaction, Value,
};

pub(crate) fn validate_and_merge(
    original_pskt_hex: &str,
    signed_kspt: &[u8],
    network: Network,
) -> Result<String, String> {
    let unsigned = super::super::relay::encode_pskt(original_pskt_hex, network)?;
    let expected = parse(&unsigned)?;
    let signed = parse(signed_kspt)?;
    validate_same_transaction(&expected, &signed, network)?;
    verify_all_signatures(&expected)?;
    verify_all_signatures(&signed)?;
    merge_signatures(original_pskt_hex, &signed)
}

pub(crate) fn validate_same_transaction(
    expected: &Transaction,
    signed: &Transaction,
    network: Network,
) -> Result<(), String> {
    if signed.network != network.kspt_code() || expected.network != network.kspt_code() {
        return Err("signed KSPT network does not match requested network".to_string());
    }
    let mut expected_body = expected.clone();
    let mut signed_body = signed.clone();
    expected_body.flags = 0;
    signed_body.flags = 0;
    for input in &mut expected_body.inputs {
        input.signatures.clear();
    }
    for input in &mut signed_body.inputs {
        input.signatures.clear();
    }
    if expected_body != signed_body {
        return Err("signed KSPT transaction body does not match the wallet PSKT".to_string());
    }
    if signed
        .inputs
        .iter()
        .all(|input| input.signatures.is_empty())
    {
        return Err("signed KSPT contains no signatures".to_string());
    }
    Ok(())
}

pub(crate) fn merge_signatures(
    original_pskt_hex: &str,
    transaction: &Transaction,
) -> Result<String, String> {
    let (format, mut root) = wire::decode(original_pskt_hex)?;
    let document = wire::document_mut(&mut root, format)?
        .as_object_mut()
        .ok_or_else(|| "PSKT not object".to_string())?;
    let inputs = document
        .get_mut("inputs")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| "missing inputs".to_string())?;
    if inputs.len() != transaction.inputs.len() {
        return Err("signed KSPT input count does not match PSKT".to_string());
    }
    for (position, signed_input) in transaction.inputs.iter().enumerate() {
        merge_input(&mut inputs[position], signed_input, position)?;
    }
    wire::encode(format, &root)
}

pub(crate) fn merge_input(value: &mut Value, signed: &Input, index: usize) -> Result<(), String> {
    if signed.signatures.is_empty() {
        return Ok(());
    }
    let input = value
        .as_object_mut()
        .ok_or_else(|| format!("input[{index}] not object"))?;
    let redeem_hex = redeem_script_hex(input, index)?;
    let partials = partial_signatures_mut(input)?;
    merge_input_route(partials, redeem_hex.as_deref(), signed, index)
}

pub(crate) fn redeem_script_hex(
    input: &Map<String, Value>,
    index: usize,
) -> Result<Option<String>, String> {
    match input.get("redeemScript") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(format!(
            "input[{index}] redeemScript must be a hex string or null"
        )),
    }
}

pub(crate) fn merge_input_route(
    partials: &mut Map<String, Value>,
    redeem_hex: Option<&str>,
    signed: &Input,
    index: usize,
) -> Result<(), String> {
    let Some(redeem_hex) = redeem_hex else {
        return merge_p2pk(partials, signed, index);
    };
    let redeem = wire::decode_lower_hex(redeem_hex, &format!("input[{index}] redeemScript"))?;
    merge_redeem_signatures(partials, &redeem, &signed.signatures, index)
}

pub(crate) fn merge_redeem_signatures(
    partials: &mut Map<String, Value>,
    redeem: &[u8],
    signatures: &[Signature],
    index: usize,
) -> Result<(), String> {
    if super::super::relay_fields::parse_multisig_redeem(redeem).is_some() {
        return merge_multisig(partials, redeem, signatures, index);
    }
    merge_covenant(partials, redeem, signatures, index)
}

pub(crate) fn merge_p2pk(
    partials: &mut Map<String, Value>,
    signed: &Input,
    index: usize,
) -> Result<(), String> {
    if signed.script.len() != 34 || signed.script[0] != 0x20 || signed.script[33] != 0xac {
        return Err(format!("input[{index}] is not a standard P2PK input"));
    }
    if signed.signatures.len() != 1 {
        return Err(format!(
            "input[{index}] standard P2PK input must contain exactly one signature"
        ));
    }
    let signature = signed
        .signatures
        .first()
        .ok_or_else(|| format!("input[{index}] has no signature"))?;
    if signature.position != 0 {
        return Err(format!(
            "input[{index}] P2PK signature position must be zero"
        ));
    }
    require_sighash_all(signature, index)?;
    let public_key = format!("02{}", hex::encode(&signed.script[1..33]));
    insert_signature(partials, public_key, &signature.bytes, index)
}

pub(crate) fn merge_multisig(
    partials: &mut Map<String, Value>,
    redeem: &[u8],
    signatures: &[Signature],
    input_index: usize,
) -> Result<(), String> {
    for signature in signatures {
        require_sighash_all(signature, input_index)?;
        let key = multisig_xonly(redeem, signature.position).ok_or_else(|| {
            format!(
                "input[{input_index}] signature position {} is invalid",
                signature.position
            )
        })?;
        let public_key = format!("02{}", hex::encode(key));
        insert_signature(partials, public_key, &signature.bytes, input_index)?;
    }
    Ok(())
}

pub(crate) fn merge_covenant(
    partials: &mut Map<String, Value>,
    redeem: &[u8],
    signatures: &[Signature],
    input_index: usize,
) -> Result<(), String> {
    let resolution =
        shared_signer::covenant_branch::resolve_covenant_branches(redeem).map_err(|error| {
            format!("input[{input_index}] invalid covenant branch structure: {error:?}")
        })?;
    for signature in signatures {
        require_sighash_all(signature, input_index)?;
        let binding = resolution.key_at(signature.position).map_err(|_| {
            format!(
                "input[{input_index}] covenant signature position {} is invalid",
                signature.position
            )
        })?;
        let public_key = format!("02{}", hex::encode(binding.key));
        insert_signature(partials, public_key, &signature.bytes, input_index)?;
    }
    Ok(())
}

pub(crate) fn require_sighash_all(signature: &Signature, input_index: usize) -> Result<(), String> {
    if signature.sighash == 0x01 {
        Ok(())
    } else {
        Err(format!(
            "input[{input_index}] signed KSPT changed sighash type to 0x{:02x}",
            signature.sighash
        ))
    }
}

pub(crate) fn partial_signatures_mut(
    input: &mut Map<String, Value>,
) -> Result<&mut Map<String, Value>, String> {
    match input.get("partialSigs") {
        None | Some(Value::Null) => {
            input.insert("partialSigs".to_string(), Value::Object(Map::new()));
        }
        Some(Value::Object(_)) => {}
        Some(_) => return Err("partialSigs must be an object".to_string()),
    }
    input
        .get_mut("partialSigs")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "partialSigs normalization failed".to_string())
}

pub(crate) fn insert_signature(
    partials: &mut Map<String, Value>,
    public_key: String,
    signature: &[u8; 64],
    input_index: usize,
) -> Result<(), String> {
    let incoming = hex::encode(signature);
    if let Some(existing) = partials.get(&public_key) {
        let existing = existing
            .get("schnorr")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                format!("input[{input_index}] existing partial signature is malformed")
            })?;
        if existing != incoming {
            return Err(format!(
                "input[{input_index}] conflicting signature for public key {public_key}"
            ));
        }
        return Ok(());
    }
    if partials.len() >= usize::from(crate::SIGNER_CAPABILITIES.max_signatures_per_input) {
        return Err(format!(
            "input[{input_index}] partial signature capacity exceeded"
        ));
    }
    let mut value = Map::new();
    value.insert("schnorr".to_string(), Value::String(incoming));
    partials.insert(public_key, Value::Object(value));
    Ok(())
}

pub(crate) fn multisig_xonly(redeem: &[u8], position: u8) -> Option<[u8; 32]> {
    let public_key = (0u8..=u8::MAX).find_map(|candidate| {
        let start = 2usize.checked_add(usize::from(candidate).checked_mul(33)?)?;
        let key = redeem.get(start..start + 32)?;
        let mut prefixed = String::from("02");
        prefixed.push_str(&hex::encode(key));
        (find_pubkey_position(redeem, &prefixed) == Some(position)).then_some(key)
    })?;
    public_key.try_into().ok()
}

pub(crate) fn covenant_active_positions(
    branches: &shared_signer::covenant_branch::BranchResolution,
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Vec<u8> {
    (0..branches.len())
        .filter_map(|position| {
            let position = u8::try_from(position).ok()?;
            branches
                .key_at(position)
                .ok()?
                .matches_selectors(supplied_mask, supplied_true_mask)
                .then_some(position)
        })
        .collect()
}
