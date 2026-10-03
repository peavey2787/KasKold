//! Strict typed field readers shared by the PSKT consumer validators.

use super::Value;

pub(crate) fn validate_schema_object(
    object: &serde_json::Map<String, Value>,
    scope: kaskold_protocol::wire::pskt_schema::Scope,
    label: &str,
) -> Result<(), String> {
    validate_required_schema_fields(object, scope, label)?;
    validate_present_schema_fields(object, scope, label)
}

pub(crate) fn validate_required_schema_fields(
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

pub(crate) fn validate_present_schema_fields(
    object: &serde_json::Map<String, Value>,
    scope: kaskold_protocol::wire::pskt_schema::Scope,
    label: &str,
) -> Result<(), String> {
    for (key, value) in object {
        validate_present_schema_field(scope, label, key, value)?;
    }
    Ok(())
}

pub(crate) fn validate_present_schema_field(
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

pub(crate) fn required_array<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a Vec<Value>, String> {
    object
        .get(key)
        .ok_or(format!("missing {key}"))?
        .as_array()
        .ok_or(format!("{key} must be an array"))
}

pub(crate) fn required_object<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<&'a serde_json::Map<String, Value>, String> {
    object
        .get(key)
        .ok_or(format!("missing {key}"))?
        .as_object()
        .ok_or(format!("{key} must be an object"))
}

pub(crate) fn required_string<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<&'a str, String> {
    object
        .get(key)
        .ok_or(format!("missing {field}"))?
        .as_str()
        .ok_or(format!("{field} must be a string"))
}

pub(crate) fn required_exact_u64(
    object: &serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<u64, String> {
    let value = object.get(key).ok_or(format!("missing {field}"))?;
    crate::protocol::pskt::exact_json::parse_exact_u64(value, field)
}

pub(crate) fn optional_exact_u64(
    object: &serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<Option<u64>, String> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => crate::protocol::pskt::exact_json::parse_exact_u64(value, field).map(Some),
    }
}

pub(crate) fn optional_bool(
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

pub(crate) fn optional_null_or_string(
    object: &serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<(), String> {
    match object.get(key) {
        None | Some(Value::Null) | Some(Value::String(_)) => Ok(()),
        Some(_) => Err(format!("{field} must be a string or null")),
    }
}

pub(crate) fn optional_object_strict<'a>(
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

pub(crate) fn optional_object_no_null<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
    field: &str,
) -> Result<Option<&'a serde_json::Map<String, Value>>, String> {
    optional_object_strict(object, key, field)
}

pub(crate) fn optional_string_field<'a>(
    object: &'a serde_json::Map<String, Value>,
    key: &str,
) -> Result<Option<&'a str>, String> {
    match object.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value)),
        Some(_) => Err(format!("{key} must be a string or null")),
    }
}

pub(crate) fn validate_compressed_pubkey(text: &str, field: &str) -> Result<(), String> {
    validate_exact_hex(text, field, 33)?;
    if !(text.starts_with("02") || text.starts_with("03")) {
        return Err(format!(
            "{field} must be a canonical compressed SEC1 public key"
        ));
    }
    Ok(())
}

pub(crate) fn validate_exact_hex(
    text: &str,
    field: &str,
    exact_bytes: usize,
) -> Result<(), String> {
    validate_canonical_hex(text, field, exact_bytes)?;
    if text.len() != exact_bytes.saturating_mul(2) {
        return Err(format!("{field} must be exactly {exact_bytes} bytes"));
    }
    Ok(())
}

pub(crate) fn validate_canonical_hex(
    text: &str,
    field: &str,
    max_bytes: usize,
) -> Result<(), String> {
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

pub(crate) fn validate_script_public_key(
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
