//! Consumer-limit validation of PSKT inputs.

use super::{
    optional_bool, optional_exact_u64, optional_object_no_null, optional_object_strict,
    optional_string_field, required_exact_u64, required_object, required_string,
    validate_canonical_hex, validate_compressed_pubkey, validate_exact_hex, validate_schema_object,
    validate_script_public_key, Value,
};

pub(crate) fn validate_inputs(
    inputs: &[Value],
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    for (index, input) in inputs.iter().enumerate() {
        validate_input(input, index, limits)?;
    }
    Ok(())
}

pub(crate) fn validate_input(
    value: &Value,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    let input = value
        .as_object()
        .ok_or(format!("input[{index}] must be an object"))?;
    validate_schema_object(
        input,
        kaskold_protocol::wire::pskt_schema::Scope::Input,
        &format!("input[{index}]"),
    )?;
    validate_input_utxo(input, index, limits)?;
    validate_input_outpoint(input, index)?;
    validate_input_signing_fields(input, index, limits)?;
    validate_input_partial_signatures(input, index, limits)?;
    validate_input_derivations(input, index, limits)?;
    optional_object_strict(
        input,
        "proprietaries",
        &format!("input[{index}].proprietaries"),
    )?;
    Ok(())
}

pub(crate) fn validate_input_utxo(
    input: &serde_json::Map<String, Value>,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    let utxo = required_object(input, "utxoEntry")?;
    validate_schema_object(
        utxo,
        kaskold_protocol::wire::pskt_schema::Scope::InputUtxo,
        &format!("input[{index}].utxoEntry"),
    )?;
    validate_input_utxo_core(utxo, index, limits)?;
    validate_input_utxo_optionals(utxo, index)
}

pub(crate) fn validate_input_utxo_core(
    utxo: &serde_json::Map<String, Value>,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    required_exact_u64(utxo, "amount", &format!("input[{index}].utxoEntry.amount"))?;
    let spk = required_string(
        utxo,
        "scriptPublicKey",
        &format!("input[{index}].utxoEntry.scriptPublicKey"),
    )?;
    validate_script_public_key(
        spk,
        &format!("input[{index}].utxoEntry.scriptPublicKey"),
        usize::from(limits.max_script_bytes),
    )
}

pub(crate) fn validate_input_utxo_optionals(
    utxo: &serde_json::Map<String, Value>,
    index: usize,
) -> Result<(), String> {
    optional_exact_u64(
        utxo,
        "blockDaaScore",
        &format!("input[{index}].utxoEntry.blockDaaScore"),
    )?;
    optional_bool(
        utxo,
        "isCoinbase",
        &format!("input[{index}].utxoEntry.isCoinbase"),
    )?;
    validate_optional_covenant_id(utxo, index)
}

pub(crate) fn validate_optional_covenant_id(
    utxo: &serde_json::Map<String, Value>,
    index: usize,
) -> Result<(), String> {
    let Some(covenant_id) = optional_string_field(utxo, "covenantId")? else {
        return Ok(());
    };
    validate_exact_hex(
        covenant_id,
        &format!("input[{index}].utxoEntry.covenantId"),
        32,
    )
}

pub(crate) fn validate_input_outpoint(
    input: &serde_json::Map<String, Value>,
    index: usize,
) -> Result<(), String> {
    let outpoint = required_object(input, "previousOutpoint")?;
    validate_schema_object(
        outpoint,
        kaskold_protocol::wire::pskt_schema::Scope::InputOutpoint,
        &format!("input[{index}].previousOutpoint"),
    )?;
    let txid = required_string(
        outpoint,
        "transactionId",
        &format!("input[{index}].previousOutpoint.transactionId"),
    )?;
    validate_exact_hex(
        txid,
        &format!("input[{index}].previousOutpoint.transactionId"),
        32,
    )?;
    let previous_index = required_exact_u64(
        outpoint,
        "index",
        &format!("input[{index}].previousOutpoint.index"),
    )?;
    if previous_index > u64::from(u32::MAX) {
        return Err(format!("input[{index}].previousOutpoint.index exceeds u32"));
    }
    Ok(())
}

pub(crate) fn validate_input_signing_fields(
    input: &serde_json::Map<String, Value>,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    let sighash = required_exact_u64(input, "sighashType", &format!("input[{index}].sighashType"))?;
    if sighash != u64::from(kaskold_protocol::wire::pskt_schema::SIGHASH_ALL) {
        return Err(format!(
            "input[{index}].sighashType must be SIGHASH_ALL (1)"
        ));
    }
    optional_exact_u64(input, "sequence", &format!("input[{index}].sequence"))?;
    optional_exact_u64(input, "minTime", &format!("input[{index}].minTime"))?;
    validate_covenant_execution_field(input.get("covenantExecution"), index)?;
    validate_input_scripts(input, index, limits)?;
    validate_signature_counts(input, index, limits)
}

pub(crate) fn validate_input_scripts(
    input: &serde_json::Map<String, Value>,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    if let Some(redeem) = optional_string_field(input, "redeemScript")? {
        validate_canonical_hex(
            redeem,
            &format!("input[{index}].redeemScript"),
            usize::from(limits.max_redeem_script_bytes),
        )?;
    }
    if let Some(final_script) = optional_string_field(input, "finalScriptSig")? {
        validate_canonical_hex(
            final_script,
            &format!("input[{index}].finalScriptSig"),
            kaskold_protocol::compat::MAX_PSKT_JSON_BYTES,
        )?;
    }
    Ok(())
}

pub(crate) fn validate_signature_counts(
    input: &serde_json::Map<String, Value>,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    if let Some(value) = input.get("sigOpCount") {
        validate_sig_op_count(value, index, limits)?;
    }
    if let Some(value) = input.get("minimumSignatures") {
        validate_minimum_signatures(value, index, limits)?;
    }
    Ok(())
}

pub(crate) fn validate_sig_op_count(
    value: &Value,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    if value.is_null() {
        return Err(format!("input[{index}].sigOpCount must not be null"));
    }
    let count = crate::protocol::pskt::exact_json::parse_exact_u64(
        value,
        &format!("input[{index}].sigOpCount"),
    )?;
    if count > u64::from(limits.max_signatures_per_input) {
        return Err(format!(
            "input[{index}].sigOpCount exceeds signer capability: {count} > {}",
            limits.max_signatures_per_input
        ));
    }
    Ok(())
}

pub(crate) fn validate_minimum_signatures(
    value: &Value,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    if value.is_null() {
        return Err(format!("input[{index}].minimumSignatures must not be null"));
    }
    let count = crate::protocol::pskt::exact_json::parse_exact_u64(
        value,
        &format!("input[{index}].minimumSignatures"),
    )?;
    if count == 0 || count > u64::from(limits.max_signatures_per_input) {
        return Err(format!(
            "input[{index}].minimumSignatures must be within 1..={}",
            limits.max_signatures_per_input
        ));
    }
    Ok(())
}

pub(crate) fn validate_input_partial_signatures(
    input: &serde_json::Map<String, Value>,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    let Some(partials) =
        optional_object_no_null(input, "partialSigs", &format!("input[{index}].partialSigs"))?
    else {
        return Ok(());
    };
    if partials.len() > usize::from(limits.max_signatures_per_input) {
        return Err(format!(
            "input[{index}].partialSigs exceeds signer capability: {} > {}",
            partials.len(),
            limits.max_signatures_per_input
        ));
    }
    for (pubkey, signature) in partials {
        validate_partial_signature(pubkey, signature, index)?;
    }
    Ok(())
}

pub(crate) fn validate_partial_signature(
    pubkey: &str,
    value: &Value,
    index: usize,
) -> Result<(), String> {
    validate_compressed_pubkey(pubkey, &format!("input[{index}].partialSigs key"))?;
    let signature = value.as_object().ok_or(format!(
        "input[{index}].partialSigs[{pubkey}] must be an object"
    ))?;
    validate_schema_object(
        signature,
        kaskold_protocol::wire::pskt_schema::Scope::PartialSignature,
        &format!("input[{index}].partialSigs[{pubkey}]"),
    )?;
    if signature.len() != 1 || !signature.contains_key("schnorr") {
        return Err(format!(
            "input[{index}].partialSigs[{pubkey}] must contain only schnorr"
        ));
    }
    let schnorr = required_string(
        signature,
        "schnorr",
        &format!("input[{index}].partialSigs[{pubkey}].schnorr"),
    )?;
    validate_exact_hex(
        schnorr,
        &format!("input[{index}].partialSigs[{pubkey}].schnorr"),
        64,
    )
}

pub(crate) fn validate_input_derivations(
    input: &serde_json::Map<String, Value>,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    let Some(derivations) = optional_object_no_null(
        input,
        "bip32Derivations",
        &format!("input[{index}].bip32Derivations"),
    )?
    else {
        return Ok(());
    };
    if derivations.len() > usize::from(limits.max_signatures_per_input) {
        return Err(format!(
            "input[{index}].bip32Derivations exceeds signer capability"
        ));
    }
    for (pubkey, derivation) in derivations {
        validate_derivation_entry(pubkey, derivation, index)?;
    }
    Ok(())
}

pub(crate) fn validate_derivation_entry(
    pubkey: &str,
    derivation: &Value,
    index: usize,
) -> Result<(), String> {
    validate_compressed_pubkey(pubkey, &format!("input[{index}].bip32Derivations key"))?;
    if !derivation.is_null() && !derivation.is_object() {
        return Err(format!(
            "input[{index}].bip32Derivations[{pubkey}] must be an object or null"
        ));
    }
    Ok(())
}

pub(crate) fn validate_covenant_execution_field(
    value: Option<&Value>,
    index: usize,
) -> Result<(), String> {
    let Some(map) = covenant_execution_map(value, index)? else {
        return Ok(());
    };
    validate_schema_object(
        map,
        kaskold_protocol::wire::pskt_schema::Scope::CovenantExecution,
        &format!("input[{index}].covenantExecution"),
    )?;
    validate_covenant_execution_shape(map, index)?;
    validate_covenant_execution_values(map, index)
}

pub(crate) fn covenant_execution_map(
    value: Option<&Value>,
    index: usize,
) -> Result<Option<&serde_json::Map<String, Value>>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value.as_object().map(Some).ok_or(format!(
        "input[{index}].covenantExecution must be an object or null"
    ))
}

pub(crate) fn validate_covenant_execution_shape(
    map: &serde_json::Map<String, Value>,
    index: usize,
) -> Result<(), String> {
    if map.len() == 2 && map.contains_key("suppliedMask") && map.contains_key("suppliedTrueMask") {
        return Ok(());
    }
    Err(format!(
        "input[{index}].covenantExecution must contain only suppliedMask and suppliedTrueMask"
    ))
}

pub(crate) fn validate_covenant_execution_values(
    map: &serde_json::Map<String, Value>,
    index: usize,
) -> Result<(), String> {
    let mask = required_exact_u64(
        map,
        "suppliedMask",
        &format!("input[{index}].covenantExecution.suppliedMask"),
    )?;
    let truth = required_exact_u64(
        map,
        "suppliedTrueMask",
        &format!("input[{index}].covenantExecution.suppliedTrueMask"),
    )?;
    if mask <= u64::from(u16::MAX) && truth <= u64::from(u16::MAX) && (truth & !mask) == 0 {
        return Ok(());
    }
    Err(format!(
        "input[{index}].covenantExecution contains an invalid selector assignment"
    ))
}
