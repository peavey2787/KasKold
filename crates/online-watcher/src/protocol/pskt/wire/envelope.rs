// KasKold Companion Web — PSKT / PSKB envelope encoding
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use serde_json::Value;

use super::super::error::PsktWireError;
use super::super::model::{PsktFormat, PSKB_MAGIC, PSKT_MAGIC};
use super::json::{decode_json_body, encode_json_body};

#[cfg(test)]
mod unit_tests;

#[derive(Clone, Copy)]
pub(crate) enum ErrorStyle {
    Standard,
    Review,
}

#[derive(Clone, Copy)]
enum PskbShapeStyle {
    Standard,
    Review,
}

/// Detect the outer PSKT/PSKB wire envelope without decoding the payload.
pub fn detect_format_hex(hex_str: &str) -> PsktFormat {
    let bytes = hex_str.as_bytes();
    let Some(prefix) = bytes.get(..8) else {
        return PsktFormat::Unknown;
    };
    if !prefix.iter().all(u8::is_ascii_hexdigit) {
        return PsktFormat::Unknown;
    }
    if prefix.eq_ignore_ascii_case(b"50534b42") {
        PsktFormat::Pskb
    } else if prefix.eq_ignore_ascii_case(b"50534b54") {
        PsktFormat::PsktSingle
    } else {
        PsktFormat::Unknown
    }
}

fn decode_wire(wire_hex: &str) -> Result<(PsktFormat, Value), PsktWireError> {
    validate_outer_hex(wire_hex)?;
    let format = decoded_format(wire_hex)?;
    let wire = hex::decode(wire_hex).map_err(|error| PsktWireError::OuterHex(error.to_string()))?;
    validate_wire_magic(&wire, format)?;
    let root = decode_json_body(&wire[4..])?;
    validate_consumer_limits(format, &root).map_err(PsktWireError::Json)?;
    Ok((format, root))
}

fn validate_outer_hex(wire_hex: &str) -> Result<(), PsktWireError> {
    if wire_hex.len() > kaskold_protocol::compat::MAX_PSKT_WIRE_HEX_CHARS {
        return Err(PsktWireError::OuterHex(
            "payload exceeds signer-compatible resource ceiling".to_string(),
        ));
    }
    let bytes = wire_hex.as_bytes();
    if !bytes.len().is_multiple_of(2)
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(PsktWireError::OuterHex(
            "outer PSKT/PSKB must be even-length lowercase hexadecimal".to_string(),
        ));
    }
    Ok(())
}

fn decoded_format(wire_hex: &str) -> Result<PsktFormat, PsktWireError> {
    let format = detect_format_hex(wire_hex);
    if format == PsktFormat::Unknown {
        return Err(PsktWireError::UnknownFormat);
    }
    Ok(format)
}

fn validate_wire_magic(wire: &[u8], format: PsktFormat) -> Result<(), PsktWireError> {
    if wire.len() < 4 {
        return Err(PsktWireError::TooShort);
    }
    let expected_magic: &[u8; 4] = match format {
        PsktFormat::Pskb => PSKB_MAGIC,
        PsktFormat::PsktSingle => PSKT_MAGIC,
        PsktFormat::Unknown => return Err(PsktWireError::UnknownFormat),
    };
    if &wire[..4] != expected_magic {
        return Err(PsktWireError::MagicMismatch);
    }
    Ok(())
}

fn validate_consumer_limits(format: PsktFormat, root: &Value) -> Result<(), String> {
    let document = consumer_document(format, root)?;
    validate_schema_object(
        document,
        kaskold_protocol::wire::pskt_schema::Scope::TopLevel,
        "PSKT",
    )?;
    let limits = kaskold_protocol::SIGNER_CAPABILITIES;
    let inputs = required_array(document, "inputs")?;
    let outputs = required_array(document, "outputs")?;
    validate_collection_limits(inputs, outputs, limits)?;
    let global = required_object(document, "global")?;
    validate_global_fields(global, inputs.len(), outputs.len(), limits)?;
    validate_inputs(inputs, limits)?;
    validate_outputs(outputs, inputs.len(), limits)
}

fn consumer_document(
    format: PsktFormat,
    root: &Value,
) -> Result<&serde_json::Map<String, Value>, String> {
    let document = match format {
        PsktFormat::Pskb => {
            let entries = root
                .as_array()
                .ok_or_else(|| "PSKB body must be an array".to_string())?;
            if entries.len() != 1 {
                return Err(format!(
                    "PSKB must wrap exactly 1 PSKT, got {}",
                    entries.len()
                ));
            }
            &entries[0]
        }
        PsktFormat::PsktSingle => root,
        PsktFormat::Unknown => return Err("unknown PSKT format".to_string()),
    };
    document
        .as_object()
        .ok_or_else(|| "PSKT body must be an object".to_string())
}

fn validate_collection_limits(
    inputs: &[Value],
    outputs: &[Value],
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    if inputs.len() > usize::from(limits.max_inputs) {
        return Err(format!(
            "PSKT input count exceeds signer capability: {} > {}",
            inputs.len(),
            limits.max_inputs
        ));
    }
    if outputs.len() > usize::from(limits.max_outputs) {
        return Err(format!(
            "PSKT output count exceeds signer capability: {} > {}",
            outputs.len(),
            limits.max_outputs
        ));
    }
    Ok(())
}

fn validate_global_fields(
    global: &serde_json::Map<String, Value>,
    input_count: usize,
    output_count: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    validate_schema_object(
        global,
        kaskold_protocol::wire::pskt_schema::Scope::Global,
        "global",
    )?;
    validate_global_versions(global)?;
    validate_declared_counts(global, input_count, output_count)?;
    validate_global_optionals(global, limits)?;
    validate_global_route_fields(global, limits)
}

fn validate_global_versions(global: &serde_json::Map<String, Value>) -> Result<(), String> {
    let version = required_exact_u64(global, "version", "global.version")?;
    if version != kaskold_protocol::wire::pskt_schema::PSKT_VERSION {
        return Err(format!("unsupported PSKT version: {version}"));
    }
    let tx_version = required_exact_u64(global, "txVersion", "global.txVersion")?;
    if !kaskold_protocol::wire::pskt_schema::supported_tx_version_u64(tx_version) {
        return Err(format!("unsupported transaction version: {tx_version}"));
    }
    Ok(())
}

fn validate_declared_counts(
    global: &serde_json::Map<String, Value>,
    input_count: usize,
    output_count: usize,
) -> Result<(), String> {
    let declared_inputs = required_exact_u64(global, "inputCount", "global.inputCount")?;
    let declared_outputs = required_exact_u64(global, "outputCount", "global.outputCount")?;
    if declared_inputs != input_count as u64 || declared_outputs != output_count as u64 {
        return Err("declared PSKT input/output counts do not match arrays".to_string());
    }
    Ok(())
}

fn validate_global_optionals(
    global: &serde_json::Map<String, Value>,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    validate_global_numeric_and_bool_optionals(global)?;
    validate_global_object_optionals(global)?;
    validate_global_payload(global, limits)
}

fn validate_global_numeric_and_bool_optionals(
    global: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    optional_exact_u64(global, "fallbackLockTime", "global.fallbackLockTime")?;
    optional_exact_u64(global, "gas", "global.gas")?;
    optional_bool(global, "inputsModifiable", "global.inputsModifiable")?;
    optional_bool(global, "outputsModifiable", "global.outputsModifiable")?;
    Ok(())
}

fn validate_global_object_optionals(global: &serde_json::Map<String, Value>) -> Result<(), String> {
    optional_object_strict(global, "xpubs", "global.xpubs")?;
    optional_object_strict(global, "proprietaries", "global.proprietaries")?;
    optional_null_or_string(global, "id", "global.id")?;
    Ok(())
}

fn validate_global_payload(
    global: &serde_json::Map<String, Value>,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    let Some(payload) = optional_string_field(global, "txPayload")? else {
        return Ok(());
    };
    validate_canonical_hex(
        payload,
        "global.txPayload",
        usize::from(limits.max_payload_bytes),
    )
}

fn validate_global_route_fields(
    global: &serde_json::Map<String, Value>,
    _limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    if let Some(subnetwork) = optional_string_field(global, "subnetworkId")? {
        validate_exact_hex(subnetwork, "global.subnetworkId", 20)?;
    }
    if let Some(branch) = optional_string_field(global, "covenantBranch")? {
        validate_covenant_branch(branch)?;
    }
    Ok(())
}

fn validate_covenant_branch(branch: &str) -> Result<(), String> {
    if matches!(
        branch,
        "owner" | "owner-time" | "beneficiary" | "savings" | "oracle-v1-claim"
    ) {
        return Ok(());
    }
    Err(format!("unsupported global.covenantBranch: {branch}"))
}

fn validate_inputs(
    inputs: &[Value],
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    for (index, input) in inputs.iter().enumerate() {
        validate_input(input, index, inputs.len(), limits)?;
    }
    Ok(())
}

fn validate_outputs(
    outputs: &[Value],
    input_count: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    for (index, output) in outputs.iter().enumerate() {
        validate_output(output, index, input_count, limits)?;
    }
    Ok(())
}

fn validate_input(
    value: &Value,
    index: usize,
    _input_count: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    let input = value
        .as_object()
        .ok_or_else(|| format!("input[{index}] must be an object"))?;
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

fn validate_input_utxo(
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

fn validate_input_utxo_core(
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

fn validate_input_utxo_optionals(
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

fn validate_optional_covenant_id(
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

fn validate_input_outpoint(
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

fn validate_input_signing_fields(
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

fn validate_input_scripts(
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

fn validate_signature_counts(
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

fn validate_sig_op_count(
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

fn validate_minimum_signatures(
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

fn validate_input_partial_signatures(
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

fn validate_partial_signature(pubkey: &str, value: &Value, index: usize) -> Result<(), String> {
    validate_compressed_pubkey(pubkey, &format!("input[{index}].partialSigs key"))?;
    let signature = value
        .as_object()
        .ok_or_else(|| format!("input[{index}].partialSigs[{pubkey}] must be an object"))?;
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

fn validate_input_derivations(
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

fn validate_derivation_entry(pubkey: &str, derivation: &Value, index: usize) -> Result<(), String> {
    validate_compressed_pubkey(pubkey, &format!("input[{index}].bip32Derivations key"))?;
    if !derivation.is_null() && !derivation.is_object() {
        return Err(format!(
            "input[{index}].bip32Derivations[{pubkey}] must be an object or null"
        ));
    }
    Ok(())
}

fn validate_covenant_execution_field(value: Option<&Value>, index: usize) -> Result<(), String> {
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

fn covenant_execution_map(
    value: Option<&Value>,
    index: usize,
) -> Result<Option<&serde_json::Map<String, Value>>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_object()
        .map(Some)
        .ok_or_else(|| format!("input[{index}].covenantExecution must be an object or null"))
}

fn validate_covenant_execution_shape(
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

fn validate_covenant_execution_values(
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

fn validate_output(
    value: &Value,
    index: usize,
    input_count: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    let output = value
        .as_object()
        .ok_or_else(|| format!("output[{index}] must be an object"))?;
    validate_schema_object(
        output,
        kaskold_protocol::wire::pskt_schema::Scope::Output,
        &format!("output[{index}]"),
    )?;
    validate_output_core(output, index, limits)?;
    validate_output_covenant_binding(output.get("covenantBinding"), index, input_count)
}

fn validate_output_core(
    output: &serde_json::Map<String, Value>,
    index: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    required_exact_u64(output, "amount", &format!("output[{index}].amount"))?;
    let spk = required_string(
        output,
        "scriptPublicKey",
        &format!("output[{index}].scriptPublicKey"),
    )?;
    validate_script_public_key(
        spk,
        &format!("output[{index}].scriptPublicKey"),
        usize::from(limits.max_script_bytes),
    )?;
    if let Some(redeem) = optional_string_field(output, "redeemScript")? {
        validate_canonical_hex(
            redeem,
            &format!("output[{index}].redeemScript"),
            usize::from(limits.max_redeem_script_bytes),
        )?;
    }
    optional_object_strict(
        output,
        "bip32Derivations",
        &format!("output[{index}].bip32Derivations"),
    )?;
    optional_object_strict(
        output,
        "proprietaries",
        &format!("output[{index}].proprietaries"),
    )?;
    Ok(())
}

fn validate_output_covenant_binding(
    value: Option<&Value>,
    index: usize,
    input_count: usize,
) -> Result<(), String> {
    let Some(value) = value else {
        return Ok(());
    };
    let binding = match value {
        Value::Null => return Ok(()),
        Value::Object(binding) => binding,
        _ => {
            return Err(format!(
                "output[{index}].covenantBinding must be an object or null"
            ))
        }
    };
    validate_schema_object(
        binding,
        kaskold_protocol::wire::pskt_schema::Scope::CovenantBinding,
        &format!("output[{index}].covenantBinding"),
    )?;
    validate_covenant_binding_shape(binding, index)?;
    validate_covenant_binding_values(binding, index, input_count)
}

fn validate_covenant_binding_shape(
    binding: &serde_json::Map<String, Value>,
    index: usize,
) -> Result<(), String> {
    if binding.len() != 2
        || !binding.contains_key("authorizingInput")
        || !binding.contains_key("covenantId")
    {
        return Err(format!(
            "output[{index}].covenantBinding must contain exactly authorizingInput and covenantId"
        ));
    }
    Ok(())
}

fn validate_covenant_binding_values(
    binding: &serde_json::Map<String, Value>,
    index: usize,
    input_count: usize,
) -> Result<(), String> {
    let authorizing = required_exact_u64(
        binding,
        "authorizingInput",
        &format!("output[{index}].covenantBinding.authorizingInput"),
    )?;
    if authorizing >= input_count as u64 || authorizing > u64::from(u16::MAX) {
        return Err(format!(
            "output[{index}].covenantBinding.authorizingInput is out of range"
        ));
    }
    let covenant_id = required_string(
        binding,
        "covenantId",
        &format!("output[{index}].covenantBinding.covenantId"),
    )?;
    validate_exact_hex(
        covenant_id,
        &format!("output[{index}].covenantBinding.covenantId"),
        32,
    )
}

fn validate_schema_object(
    object: &serde_json::Map<String, Value>,
    scope: kaskold_protocol::wire::pskt_schema::Scope,
    label: &str,
) -> Result<(), String> {
    validate_required_schema_fields(object, scope, label)?;
    validate_present_schema_fields(object, scope, label)
}

fn validate_required_schema_fields(
    object: &serde_json::Map<String, Value>,
    scope: kaskold_protocol::wire::pskt_schema::Scope,
    label: &str,
) -> Result<(), String> {
    use kaskold_protocol::wire::pskt_schema::fields;

    for spec in fields(scope) {
        if spec.required && !object.contains_key(spec.name) {
            return Err(format!("missing {label}.{}", spec.name));
        }
    }
    Ok(())
}

fn validate_present_schema_fields(
    object: &serde_json::Map<String, Value>,
    scope: kaskold_protocol::wire::pskt_schema::Scope,
    label: &str,
) -> Result<(), String> {
    for (key, value) in object {
        validate_present_schema_field(scope, label, key, value)?;
    }
    Ok(())
}

fn validate_present_schema_field(
    scope: kaskold_protocol::wire::pskt_schema::Scope,
    label: &str,
    key: &str,
    value: &Value,
) -> Result<(), String> {
    use kaskold_protocol::wire::pskt_schema::{field_spec, NullRule, EXTENSION_GRAMMAR};

    let Some(spec) = field_spec(scope, key.as_bytes()) else {
        return if EXTENSION_GRAMMAR.preserve_unknown_fields {
            Ok(())
        } else {
            Err(format!("unknown {label} field: {key}"))
        };
    };
    if value.is_null() && spec.null_rule == NullRule::Forbidden {
        Err(format!("{label}.{key} must not be null"))
    } else {
        Ok(())
    }
}

fn required_array<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a Vec<Value>, String> {
    object
        .get(key)
        .ok_or_else(|| format!("missing {key}"))?
        .as_array()
        .ok_or_else(|| format!("{key} must be an array"))
}

fn required_object<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    object
        .get(key)
        .ok_or_else(|| format!("missing {key}"))?
        .as_object()
        .ok_or_else(|| format!("{key} must be an object"))
}

fn required_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .ok_or_else(|| format!("missing {field}"))?
        .as_str()
        .ok_or_else(|| format!("{field} must be a string"))
}

fn required_exact_u64(
    object: &serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<u64, String> {
    let value = object.get(key).ok_or_else(|| format!("missing {field}"))?;
    crate::protocol::pskt::exact_json::parse_exact_u64(value, field)
}

fn optional_exact_u64(
    object: &serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<Option<u64>, String> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => crate::protocol::pskt::exact_json::parse_exact_u64(value, field).map(Some),
    }
}

fn optional_bool(
    object: &serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<Option<bool>, String> {
    match object.get(key) {
        None => Ok(None),
        Some(Value::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("{field} must be a boolean")),
    }
}

fn optional_null_or_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<(), String> {
    match object.get(key) {
        None | Some(Value::Null) | Some(Value::String(_)) => Ok(()),
        Some(_) => Err(format!("{field} must be a string or null")),
    }
}

fn optional_object_strict<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<Option<&'a serde_json::Map<String, Value>>, String> {
    match object.get(key) {
        None => Ok(None),
        Some(Value::Object(value)) => Ok(Some(value)),
        Some(_) => Err(format!("{field} must be an object")),
    }
}

fn optional_object_no_null<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<Option<&'a serde_json::Map<String, Value>>, String> {
    optional_object_strict(object, key, field)
}

fn optional_string_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<&'a str>, String> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => Err(format!("{key} must be a string or null")),
    }
}

fn validate_compressed_pubkey(text: &str, field: &str) -> Result<(), String> {
    validate_exact_hex(text, field, 33)?;
    if !(text.starts_with("02") || text.starts_with("03")) {
        return Err(format!(
            "{field} must be a canonical compressed SEC1 public key"
        ));
    }
    Ok(())
}

fn validate_exact_hex(text: &str, field: &str, exact_bytes: usize) -> Result<(), String> {
    validate_canonical_hex(text, field, exact_bytes)?;
    if text.len() != exact_bytes.saturating_mul(2) {
        return Err(format!("{field} must be exactly {exact_bytes} bytes"));
    }
    Ok(())
}

fn validate_canonical_hex(text: &str, field: &str, max_bytes: usize) -> Result<(), String> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(2)
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(format!("{field} must be even-length lowercase hexadecimal"));
    }
    let byte_len = bytes.len() / 2;
    if byte_len > max_bytes {
        return Err(format!(
            "{field} exceeds signer capability: {byte_len} > {max_bytes} bytes"
        ));
    }
    Ok(())
}

fn validate_script_public_key(
    text: &str,
    field: &str,
    max_script_bytes: usize,
) -> Result<(), String> {
    validate_canonical_hex(text, field, max_script_bytes.saturating_add(2))?;
    if text.len() < 4 {
        return Err(format!("{field} must contain a 2-byte script version"));
    }
    let script_bytes = text.len() / 2 - 2;
    if script_bytes > max_script_bytes {
        return Err(format!(
            "{field} script exceeds signer capability: {script_bytes} > {max_script_bytes} bytes"
        ));
    }
    Ok(())
}

fn decode_root_with_style(
    wire_hex: &str,
    style: ErrorStyle,
) -> Result<(PsktFormat, Value), String> {
    decode_wire(wire_hex).map_err(|error| format_wire_error(error, style))
}

pub(crate) fn format_wire_error(error: PsktWireError, style: ErrorStyle) -> String {
    match (style, error) {
        (_, PsktWireError::UnknownFormat) => "Not a PSKT/PSKB payload".to_string(),
        (ErrorStyle::Standard, PsktWireError::OuterHex(message)) => {
            format!("outer hex: {message}")
        }
        (ErrorStyle::Review, PsktWireError::OuterHex(message)) => {
            format!("Bad outer hex: {message}")
        }
        (ErrorStyle::Standard, PsktWireError::TooShort) => "payload too short".to_string(),
        (ErrorStyle::Review, PsktWireError::TooShort) => "Payload too short".to_string(),
        (_, PsktWireError::MagicMismatch) => {
            "wire magic does not match detected format".to_string()
        }
        #[cfg(test)]
        (ErrorStyle::Standard, PsktWireError::InnerHex(message)) => {
            format!("inner hex: {message}")
        }
        #[cfg(test)]
        (ErrorStyle::Review, PsktWireError::InnerHex(message)) => {
            format!("Bad inner hex: {message}")
        }
        (_, PsktWireError::Json(message)) => format!("JSON parse: {message}"),
    }
}

pub(crate) fn decode_root(wire_hex: &str) -> Result<(PsktFormat, Value), String> {
    decode_root_with_style(wire_hex, ErrorStyle::Standard)
}

pub(crate) fn decode_root_for_review(wire_hex: &str) -> Result<(PsktFormat, Value), String> {
    decode_root_with_style(wire_hex, ErrorStyle::Review)
}

fn validate_single_pskt(
    root: &Value,
    format: PsktFormat,
    style: PskbShapeStyle,
) -> Result<(), String> {
    match format {
        PsktFormat::Pskb => {
            let entries = root.as_array().ok_or_else(|| match style {
                PskbShapeStyle::Standard => "PSKB not array".to_string(),
                PskbShapeStyle::Review => "PSKB body is not an array".to_string(),
            })?;
            if entries.len() != 1 {
                return Err(match style {
                    PskbShapeStyle::Standard => {
                        format!("PSKB must have 1 entry, got {}", entries.len())
                    }
                    PskbShapeStyle::Review => {
                        format!("PSKB must wrap exactly 1 PSKT, got {}", entries.len())
                    }
                });
            }
            Ok(())
        }
        PsktFormat::PsktSingle => Ok(()),
        PsktFormat::Unknown => Err("Not a PSKT/PSKB payload".into()),
    }
}

fn validated_pskt(
    root: &Value,
    format: PsktFormat,
    style: PskbShapeStyle,
) -> Result<&Value, String> {
    validate_single_pskt(root, format, style)?;
    Ok(match format {
        PsktFormat::Pskb => &root.as_array().expect("validated PSKB array")[0],
        PsktFormat::PsktSingle => root,
        PsktFormat::Unknown => unreachable!("validated format"),
    })
}

#[cfg(test)]
pub(crate) fn pskt_from_root(root: &Value, format: PsktFormat) -> Result<&Value, String> {
    validated_pskt(root, format, PskbShapeStyle::Standard)
}

pub(crate) fn pskt_from_root_for_review(
    root: &Value,
    format: PsktFormat,
) -> Result<&Value, String> {
    validated_pskt(root, format, PskbShapeStyle::Review)
}

pub(crate) fn pskt_from_root_mut(
    root: &mut Value,
    format: PsktFormat,
) -> Result<&mut Value, String> {
    validate_single_pskt(root, format, PskbShapeStyle::Standard)?;
    Ok(match format {
        PsktFormat::Pskb => &mut root.as_array_mut().expect("validated PSKB array")[0],
        PsktFormat::PsktSingle => root,
        PsktFormat::Unknown => unreachable!("validated format"),
    })
}

pub(crate) fn first_pskt_from_pskb_mut(root: &mut Value) -> Result<&mut Value, String> {
    pskb_entries_mut(root)?
        .first_mut()
        .ok_or_else(|| "empty PSKB".to_string())
}

fn pskb_entries_mut(root: &mut Value) -> Result<&mut Vec<Value>, String> {
    root.as_array_mut()
        .ok_or_else(|| "PSKB not array".to_string())
}

pub(crate) fn encode_root(format: PsktFormat, root: &Value) -> Result<String, String> {
    validate_consumer_limits(format, root)?;
    let body = encode_json_body(root)?;
    let magic: &[u8; 4] = match format {
        PsktFormat::Pskb => PSKB_MAGIC,
        PsktFormat::PsktSingle => PSKT_MAGIC,
        PsktFormat::Unknown => return Err("cannot encode unknown PSKT format".into()),
    };
    let mut wire = Vec::with_capacity(4 + body.len());
    wire.extend_from_slice(magic);
    wire.extend_from_slice(&body);
    Ok(hex::encode(wire))
}
