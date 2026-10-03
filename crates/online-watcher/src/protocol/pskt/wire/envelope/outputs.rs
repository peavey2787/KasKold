//! Consumer-limit validation of PSKT outputs.

use super::{
    optional_object_strict, optional_string_field, required_exact_u64, required_string,
    validate_canonical_hex, validate_exact_hex, validate_schema_object, validate_script_public_key,
    Value,
};

pub(crate) fn validate_outputs(
    outputs: &[Value],
    input_count: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    for (index, output) in outputs.iter().enumerate() {
        validate_output(output, index, input_count, limits)?;
    }
    Ok(())
}

pub(crate) fn validate_output(
    value: &Value,
    index: usize,
    input_count: usize,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    let output = value
        .as_object()
        .ok_or(format!("output[{index}] must be an object"))?;
    validate_schema_object(
        output,
        kaskold_protocol::wire::pskt_schema::Scope::Output,
        &format!("output[{index}]"),
    )?;
    validate_output_core(output, index, limits)?;
    validate_output_covenant_binding(output.get("covenantBinding"), index, input_count)
}

pub(crate) fn validate_output_core(
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

pub(crate) fn validate_output_covenant_binding(
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

pub(crate) fn validate_covenant_binding_shape(
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

pub(crate) fn validate_covenant_binding_values(
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
