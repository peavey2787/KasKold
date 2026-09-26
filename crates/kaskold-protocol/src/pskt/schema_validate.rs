//! Serde adapter for the executable no-std PSKT schema.
//!
//! This module contains no field policy of its own: requiredness, nullability,
//! broad JSON type and defaults come from `wire::pskt_schema`. Semantic bounds
//! (for example 32-byte txids or u16 covenant selectors) are applied after this
//! common shape validation.

use serde_json::{Map, Value};

use crate::wire::pskt_schema::{self, DefaultRule, NullRule, Scope, ValueKind, EXTENSION_GRAMMAR};

pub(super) fn default_u64(scope: Scope, field: &str) -> Result<u64, String> {
    match pskt_schema::default_rule(scope, field.as_bytes()) {
        DefaultRule::Zero => Ok(0),
        DefaultRule::One => Ok(1),
        other => Err(format!(
            "{field} has no unsigned-integer default ({other:?})"
        )),
    }
}

pub(super) fn validate_document(document: &Map<String, Value>) -> Result<(), String> {
    validate_object(document, Scope::TopLevel, "PSKT")?;
    let global = required_object(document, "global", "PSKT.global")?;
    validate_object(global, Scope::Global, "global")?;
    let inputs = required_array(document, "inputs", "PSKT.inputs")?;
    let outputs = required_array(document, "outputs", "PSKT.outputs")?;
    validate_inputs(inputs)?;
    validate_global_semantics(global, inputs.len(), outputs.len())?;
    validate_outputs(outputs)
}

fn validate_inputs(inputs: &[Value]) -> Result<(), String> {
    for (index, value) in inputs.iter().enumerate() {
        validate_input(value, index)?;
    }
    Ok(())
}

fn validate_outputs(outputs: &[Value]) -> Result<(), String> {
    for (index, value) in outputs.iter().enumerate() {
        validate_output(value, index)?;
    }
    Ok(())
}

fn validate_input(value: &Value, index: usize) -> Result<(), String> {
    let label = format!("input[{index}]");
    let input = value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))?;
    validate_object(input, Scope::Input, &label)?;
    validate_input_nested_objects(input, &label)?;
    validate_input_semantics(input, &label)?;
    validate_input_partials(input, &label)
}

fn validate_input_nested_objects(input: &Map<String, Value>, label: &str) -> Result<(), String> {
    let utxo = required_object(input, "utxoEntry", &format!("{label}.utxoEntry"))?;
    validate_object(utxo, Scope::InputUtxo, &format!("{label}.utxoEntry"))?;
    let outpoint = required_object(
        input,
        "previousOutpoint",
        &format!("{label}.previousOutpoint"),
    )?;
    validate_object(
        outpoint,
        Scope::InputOutpoint,
        &format!("{label}.previousOutpoint"),
    )?;
    if let Some(execution) = optional_object(
        input.get("covenantExecution"),
        &format!("{label}.covenantExecution"),
    )? {
        validate_object(
            execution,
            Scope::CovenantExecution,
            &format!("{label}.covenantExecution"),
        )?;
    }
    Ok(())
}

fn validate_input_semantics(input: &Map<String, Value>, label: &str) -> Result<(), String> {
    if let Some(value) = input.get("sighashType") {
        let sighash = parse_exact_unsigned(value, &format!("{label}.sighashType"))?;
        if sighash != u64::from(pskt_schema::SIGHASH_ALL) {
            return Err(format!("{label}.sighashType is unsupported: {sighash}"));
        }
    }
    if let Some(value) = input.get("minimumSignatures") {
        validate_minimum_signatures(value, label)?;
    }
    Ok(())
}

fn validate_minimum_signatures(value: &Value, label: &str) -> Result<(), String> {
    let count = parse_exact_unsigned(value, &format!("{label}.minimumSignatures"))?;
    if count == 0 || count > u64::from(pskt_schema::MAX_SIGNATURES_PER_INPUT) {
        return Err(format!(
            "{label}.minimumSignatures must be within 1..={}",
            pskt_schema::MAX_SIGNATURES_PER_INPUT
        ));
    }
    Ok(())
}

fn validate_input_partials(input: &Map<String, Value>, label: &str) -> Result<(), String> {
    let Some(partials) =
        optional_object(input.get("partialSigs"), &format!("{label}.partialSigs"))?
    else {
        return Ok(());
    };
    if partials.len() > usize::from(pskt_schema::MAX_SIGNATURES_PER_INPUT) {
        return Err(format!(
            "{label}.partialSigs exceeds the five-signature consumer limit"
        ));
    }
    for (key, value) in partials {
        validate_partial_signature_object(value, label, key)?;
    }
    Ok(())
}

fn validate_partial_signature_object(value: &Value, label: &str, key: &str) -> Result<(), String> {
    let signature = value
        .as_object()
        .ok_or_else(|| format!("{label}.partialSigs[{key}] must be an object"))?;
    validate_object(
        signature,
        Scope::PartialSignature,
        &format!("{label}.partialSigs[{key}]"),
    )
}

fn validate_output(value: &Value, index: usize) -> Result<(), String> {
    let label = format!("output[{index}]");
    let output = value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))?;
    validate_object(output, Scope::Output, &label)?;
    if let Some(binding) = optional_object(
        output.get("covenantBinding"),
        &format!("{label}.covenantBinding"),
    )? {
        validate_object(
            binding,
            Scope::CovenantBinding,
            &format!("{label}.covenantBinding"),
        )?;
    }
    Ok(())
}

fn validate_object(object: &Map<String, Value>, scope: Scope, label: &str) -> Result<(), String> {
    validate_required_fields(object, scope, label)?;
    validate_object_fields(object, scope, label)
}

fn validate_required_fields(
    object: &Map<String, Value>,
    scope: Scope,
    label: &str,
) -> Result<(), String> {
    for spec in pskt_schema::fields(scope) {
        if spec.required && !object.contains_key(spec.name) {
            return Err(format!("missing {label}.{}", spec.name));
        }
    }
    Ok(())
}

fn validate_object_fields(
    object: &Map<String, Value>,
    scope: Scope,
    label: &str,
) -> Result<(), String> {
    for (key, value) in object {
        validate_object_field(scope, label, key, value)?;
    }
    Ok(())
}

fn validate_object_field(
    scope: Scope,
    label: &str,
    key: &str,
    value: &Value,
) -> Result<(), String> {
    let Some(spec) = pskt_schema::field_spec(scope, key.as_bytes()) else {
        if EXTENSION_GRAMMAR.preserve_unknown_fields {
            return Ok(());
        }
        return Err(format!("unknown {label} field: {key}"));
    };
    if value.is_null() {
        return validate_null_field(spec, label, key);
    }
    validate_kind(value, spec.kind, &format!("{label}.{key}"))
}

fn validate_null_field(
    spec: &pskt_schema::FieldSpec,
    label: &str,
    key: &str,
) -> Result<(), String> {
    if spec.null_rule == NullRule::Forbidden {
        return Err(format!("{label}.{key} must not be null"));
    }
    if spec.default == DefaultRule::None && spec.required {
        return Err(format!("{label}.{key} has no null default"));
    }
    Ok(())
}

fn validate_global_semantics(
    global: &Map<String, Value>,
    input_len: usize,
    output_len: usize,
) -> Result<(), String> {
    validate_pskt_version(global)?;
    validate_transaction_version(global)?;
    validate_declared_counts(global, input_len, output_len)
}

fn validate_pskt_version(global: &Map<String, Value>) -> Result<(), String> {
    let version = parse_exact_unsigned(
        global
            .get("version")
            .ok_or_else(|| "missing global.version".to_string())?,
        "global.version",
    )?;
    if version != pskt_schema::PSKT_VERSION {
        return Err(format!("unsupported PSKT version: {version}"));
    }
    Ok(())
}

fn validate_transaction_version(global: &Map<String, Value>) -> Result<(), String> {
    let tx_version = parse_exact_unsigned(
        global
            .get("txVersion")
            .ok_or_else(|| "missing global.txVersion".to_string())?,
        "global.txVersion",
    )?;
    if !pskt_schema::supported_tx_version_u64(tx_version) {
        return Err(format!("unsupported transaction version: {tx_version}"));
    }
    Ok(())
}

fn validate_declared_counts(
    global: &Map<String, Value>,
    input_len: usize,
    output_len: usize,
) -> Result<(), String> {
    let declared_inputs = parse_exact_unsigned(
        global
            .get("inputCount")
            .ok_or_else(|| "missing global.inputCount".to_string())?,
        "global.inputCount",
    )?;
    let declared_outputs = parse_exact_unsigned(
        global
            .get("outputCount")
            .ok_or_else(|| "missing global.outputCount".to_string())?,
        "global.outputCount",
    )?;
    if declared_inputs != input_len as u64 || declared_outputs != output_len as u64 {
        return Err("declared PSKT input/output counts do not match arrays".to_string());
    }
    Ok(())
}

fn parse_exact_unsigned(value: &Value, label: &str) -> Result<u64, String> {
    match value {
        Value::String(text) => pskt_schema::parse_canonical_u64_bytes(text.as_bytes())
            .map_err(|_| format!("{label} must be a canonical unsigned decimal string")),
        Value::Number(number) => {
            let integer = number
                .as_u64()
                .ok_or_else(|| format!("{label} must be an unsigned integer"))?;
            if pskt_schema::legacy_json_integer_is_exact(integer) {
                Ok(integer)
            } else {
                Err(format!(
                    "legacy numeric {label} exceeds JavaScript's exact integer range; encode it as a decimal string"
                ))
            }
        }
        _ => Err(format!(
            "{label} must be a decimal string or exact legacy integer"
        )),
    }
}

fn validate_kind(value: &Value, kind: ValueKind, label: &str) -> Result<(), String> {
    match kind {
        ValueKind::Object if value.is_object() => Ok(()),
        ValueKind::Array if value.is_array() => Ok(()),
        ValueKind::Boolean if value.is_boolean() => Ok(()),
        ValueKind::String if value.is_string() => Ok(()),
        ValueKind::ExactUnsigned => validate_exact_unsigned(value, label),
        ValueKind::HexString => value
            .as_str()
            .ok_or_else(|| format!("{label} must be a lowercase hexadecimal string"))
            .and_then(|text| validate_hex(text, label)),
        ValueKind::ScriptPublicKey => value
            .as_str()
            .ok_or_else(|| format!("{label} must be a scriptPublicKey string"))
            .and_then(|text| validate_script_public_key_shape(text, label)),
        _ => Err(format!("{label} has the wrong JSON type")),
    }
}

fn validate_exact_unsigned(value: &Value, label: &str) -> Result<(), String> {
    parse_exact_unsigned(value, label).map(|_| ())
}

fn validate_hex(text: &str, label: &str) -> Result<(), String> {
    let bytes = text.as_bytes();
    if bytes.len().is_multiple_of(2)
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        Ok(())
    } else {
        Err(format!("{label} must be even-length lowercase hexadecimal"))
    }
}

fn validate_script_public_key_shape(text: &str, label: &str) -> Result<(), String> {
    if text.len() < 4 {
        return Err(format!(
            "{label} must contain a four-hex-digit script version"
        ));
    }
    validate_hex(text, label)
}

fn required_object<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a Map<String, Value>, String> {
    object
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{label} must be an object"))
}

fn required_array<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    label: &str,
) -> Result<&'a Vec<Value>, String> {
    object
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{label} must be an array"))
}

fn optional_object<'a>(
    value: Option<&'a Value>,
    label: &str,
) -> Result<Option<&'a Map<String, Value>>, String> {
    match value {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(object)) => Ok(Some(object)),
        Some(_) => Err(format!("{label} must be an object or null")),
    }
}

#[cfg(test)]
mod unit_tests;
