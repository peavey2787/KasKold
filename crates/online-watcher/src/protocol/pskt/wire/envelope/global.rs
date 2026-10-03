//! Consumer-limit validation of the PSKT `global` map.

use super::{
    optional_bool, optional_exact_u64, optional_null_or_string, optional_object_strict,
    optional_string_field, required_exact_u64, validate_canonical_hex, validate_exact_hex,
    validate_schema_object, Value,
};

pub(crate) fn validate_global_fields(
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
    validate_global_route_fields(global)
}

pub(crate) fn validate_global_versions(
    global: &serde_json::Map<String, Value>,
) -> Result<(), String> {
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

pub(crate) fn validate_declared_counts(
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

pub(crate) fn validate_global_optionals(
    global: &serde_json::Map<String, Value>,
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    validate_global_numeric_and_bool_optionals(global)?;
    validate_global_object_optionals(global)?;
    validate_global_payload(global, limits)
}

pub(crate) fn validate_global_numeric_and_bool_optionals(
    global: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    optional_exact_u64(global, "fallbackLockTime", "global.fallbackLockTime")?;
    optional_exact_u64(global, "gas", "global.gas")?;
    optional_bool(global, "inputsModifiable", "global.inputsModifiable")?;
    optional_bool(global, "outputsModifiable", "global.outputsModifiable")?;
    Ok(())
}

pub(crate) fn validate_global_object_optionals(
    global: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    optional_object_strict(global, "xpubs", "global.xpubs")?;
    optional_object_strict(global, "proprietaries", "global.proprietaries")?;
    optional_null_or_string(global, "id", "global.id")?;
    Ok(())
}

pub(crate) fn validate_global_payload(
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

pub(crate) fn validate_global_route_fields(
    global: &serde_json::Map<String, Value>,
) -> Result<(), String> {
    if let Some(subnetwork) = optional_string_field(global, "subnetworkId")? {
        validate_exact_hex(subnetwork, "global.subnetworkId", 20)?;
    }
    if let Some(branch) = optional_string_field(global, "covenantBranch")? {
        validate_covenant_branch(branch)?;
    }
    Ok(())
}

pub(crate) fn validate_covenant_branch(branch: &str) -> Result<(), String> {
    if matches!(
        branch,
        "owner" | "owner-time" | "beneficiary" | "savings" | "oracle-v1-claim"
    ) {
        return Ok(());
    }
    Err(format!("unsupported global.covenantBranch: {branch}"))
}
