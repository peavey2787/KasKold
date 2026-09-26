use serde_json::{Map, Number, Value};

use crate::account::AddressBranch;

use crate::wire::pskt_envelope::{PSKB_MAGIC, PSKT_MAGIC};

/// The offline signer stores decoded JSON length in u16; enforce the same
/// ceiling before host-side hex/JSON allocations can grow without bound.
pub(crate) const MAX_PSKT_JSON_BYTES: usize = u16::MAX as usize;
pub(crate) const MAX_PSKT_WIRE_HEX_CHARS: usize = 2 * (4 + 2 * MAX_PSKT_JSON_BYTES);

pub(crate) fn decode_lower_hex(text: &str, field: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    if !bytes.len().is_multiple_of(2)
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(format!("{field} must be even-length lowercase hexadecimal"));
    }
    hex::decode(text).map_err(|error| format!("{field}: {error}"))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Format {
    Pskb,
    Pskt,
}

pub(crate) fn parse_strict_json(json: &[u8]) -> Result<Value, String> {
    // One executable grammar owns recursive duplicate-key rejection, nesting,
    // number syntax and string syntax for both Companion and Vault. Serde runs
    // only after that allocation-free validator has accepted the complete byte
    // stream, so its map duplicate semantics can never become security policy.
    crate::wire::pskt_schema::validate_canonical_json(json).map_err(|error| error.to_string())?;
    serde_json::from_slice(json)
        .map_err(|error| format!("JSON parse after canonical validation: {error}"))
}

pub(crate) fn decode(wire_hex: &str) -> Result<(Format, Value), String> {
    validate_outer_wire_hex(wire_hex)?;
    let wire = decode_lower_hex(wire_hex, "outer PSKT/PSKB hex")?;
    let format = decoded_wire_format(&wire)?;
    let root = decode_json_body_hex(&wire[4..])?;
    Ok((format, root))
}

fn validate_outer_wire_hex(wire_hex: &str) -> Result<(), String> {
    if wire_hex.len() > MAX_PSKT_WIRE_HEX_CHARS {
        return Err("PSKT/PSKB payload exceeds host resource ceiling".to_string());
    }
    if !wire_hex.is_ascii() || !wire_hex.len().is_multiple_of(2) {
        return Err("outer PSKT/PSKB must be even-length ASCII hex".to_string());
    }
    Ok(())
}

fn decoded_wire_format(wire: &[u8]) -> Result<Format, String> {
    if wire.len() < 4 {
        return Err("payload too short".to_string());
    }
    let magic = &wire[..4];
    if magic == PSKB_MAGIC {
        return Ok(Format::Pskb);
    }
    if magic == PSKT_MAGIC {
        return Ok(Format::Pskt);
    }
    Err("Not a PSKT/PSKB payload".to_string())
}

/// Decode the ASCII-hex JSON body using the exact canonical grammar shared by
/// the Companion and the offline signer boundary.
pub(crate) fn decode_json_body_hex(body_hex: &[u8]) -> Result<Value, String> {
    if !body_hex.len().is_multiple_of(2) || body_hex.len() > MAX_PSKT_JSON_BYTES * 2 {
        return Err("PSKT JSON hex body has invalid size".to_string());
    }
    let inner_text = core::str::from_utf8(body_hex)
        .map_err(|_| "inner PSKT JSON hex must be ASCII".to_string())?;
    let inner = decode_lower_hex(inner_text, "inner PSKT JSON hex")?;
    if inner.len() > MAX_PSKT_JSON_BYTES {
        return Err("PSKT JSON exceeds signer-compatible size".to_string());
    }
    parse_strict_json(&inner)
}

/// Encode a JSON value into the canonical ASCII-hex body. Values that would
/// require JSON escaping or otherwise violate the signer grammar are rejected.
pub(crate) fn encode_json_body_hex(root: &Value) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(root).map_err(|error| format!("JSON encode: {error}"))?;
    if json.len() > MAX_PSKT_JSON_BYTES {
        return Err("PSKT JSON exceeds signer-compatible size".to_string());
    }
    let reparsed = parse_strict_json(&json)?;
    if &reparsed != root {
        return Err("PSKT JSON value is not canonical under the signer grammar".to_string());
    }
    Ok(hex::encode(json).into_bytes())
}

pub(crate) fn encode(format: Format, root: &Value) -> Result<String, String> {
    let body = encode_json_body_hex(root)?;
    let magic = match format {
        Format::Pskb => PSKB_MAGIC,
        Format::Pskt => PSKT_MAGIC,
    };
    let mut wire = Vec::with_capacity(4 + body.len());
    wire.extend_from_slice(magic);
    wire.extend_from_slice(&body);
    Ok(hex::encode(wire))
}

pub(crate) fn document(root: &Value, format: Format) -> Result<&Value, String> {
    match format {
        Format::Pskt => Ok(root),
        Format::Pskb => {
            let entries = root
                .as_array()
                .ok_or_else(|| "PSKB not array".to_string())?;
            if entries.len() != 1 {
                return Err(format!("PSKB must have 1 entry, got {}", entries.len()));
            }
            Ok(&entries[0])
        }
    }
}

pub(crate) fn document_mut(root: &mut Value, format: Format) -> Result<&mut Value, String> {
    match format {
        Format::Pskt => Ok(root),
        Format::Pskb => {
            let entries = root
                .as_array_mut()
                .ok_or_else(|| "PSKB not array".to_string())?;
            if entries.len() != 1 {
                return Err(format!("PSKB must have 1 entry, got {}", entries.len()));
            }
            Ok(&mut entries[0])
        }
    }
}

pub(crate) fn parse_exact_u64(value: &Value, field: &str) -> Result<u64, String> {
    match value {
        Value::String(text) => parse_decimal(text, field),
        Value::Number(number) => parse_legacy_number(number, field),
        _ => Err(format!("{field} must be a decimal string")),
    }
}

fn parse_decimal(text: &str, field: &str) -> Result<u64, String> {
    crate::wire::pskt_schema::parse_canonical_u64_bytes(text.as_bytes()).map_err(
        |error| match error {
            crate::wire::pskt_schema::JsonNumberError::NonCanonical => {
                format!("{field} must be a canonical unsigned decimal string")
            }
            crate::wire::pskt_schema::JsonNumberError::Overflow => format!("{field} exceeds u64"),
            crate::wire::pskt_schema::JsonNumberError::LegacyUnsafeInteger => unreachable!(),
        },
    )
}

fn parse_legacy_number(number: &Number, field: &str) -> Result<u64, String> {
    let value = number
        .as_u64()
        .ok_or_else(|| format!("{field} must be an unsigned integer"))?;
    if !crate::wire::pskt_schema::legacy_json_integer_is_exact(value) {
        return Err(format!(
            "legacy numeric {field} exceeds JavaScript's exact integer range; encode it as a decimal string"
        ));
    }
    Ok(value)
}

pub(crate) fn parse_spk(value: &str) -> Result<(u16, Vec<u8>), String> {
    let bytes = value.as_bytes();
    if bytes.len() < 4
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(format!(
            "scriptPublicKey must be ASCII hex and at least 4 bytes, got {}",
            bytes.len()
        ));
    }
    let version_text = core::str::from_utf8(&bytes[..4])
        .map_err(|_| "scriptPublicKey version is not ASCII".to_string())?;
    let script_text = core::str::from_utf8(&bytes[4..])
        .map_err(|_| "scriptPublicKey body is not ASCII".to_string())?;
    let version = u16::from_str_radix(version_text, 16)
        .map_err(|error| format!("bad script version: {error}"))?;
    let script = decode_lower_hex(script_text, "scriptPublicKey body")?;
    Ok((version, script))
}

pub fn attach_input_derivation(
    pskt_hex: &str,
    input_index: usize,
    branch: AddressBranch,
    index: u32,
) -> Result<String, String> {
    attach_derivation(pskt_hex, "inputs", input_index, branch, index)
}

pub fn attach_output_derivation(
    pskt_hex: &str,
    output_index: usize,
    branch: AddressBranch,
    index: u32,
) -> Result<String, String> {
    attach_derivation(pskt_hex, "outputs", output_index, branch, index)
}

fn attach_derivation(
    pskt_hex: &str,
    section: &str,
    position: usize,
    branch: AddressBranch,
    index: u32,
) -> Result<String, String> {
    if index >= shared_signer::pairing::SOFT_INDEX_LIMIT {
        return Err("KasKold derivation index must be non-hardened".to_string());
    }
    let (format, mut root) = decode(pskt_hex)?;
    let doc = document_mut(&mut root, format)?
        .as_object_mut()
        .ok_or_else(|| "PSKT not object".to_string())?;
    let entries = doc
        .get_mut(section)
        .and_then(Value::as_array_mut)
        .ok_or_else(|| format!("missing {section}"))?;
    let entry = entries
        .get_mut(position)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{section}[{position}] not object"))?;
    let proprietaries = object_field_mut(entry, "proprietaries")?;
    let mut hint = Map::new();
    hint.insert("branch".to_string(), Value::from(branch.code()));
    hint.insert("index".to_string(), Value::String(index.to_string()));
    proprietaries.insert("kassignerDerivation".to_string(), Value::Object(hint));
    encode(format, &root)
}

fn object_field_mut<'a>(
    object: &'a mut Map<String, Value>,
    key: &str,
) -> Result<&'a mut Map<String, Value>, String> {
    match object.get(key) {
        None | Some(Value::Null) => {
            object.insert(key.to_string(), Value::Object(Map::new()));
        }
        Some(Value::Object(_)) => {}
        Some(_) => return Err(format!("{key} must be an object or null")),
    }
    object
        .get_mut(key)
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("{key} object normalization failed"))
}

pub(crate) fn parse_derivation(value: &Value) -> Result<Option<(u8, u32)>, String> {
    let map = value
        .as_object()
        .ok_or_else(|| "proprietaries must be an object".to_string())?;
    let Some(value) = map.get("kassignerDerivation") else {
        return Ok(None);
    };
    let hint = value
        .as_object()
        .ok_or_else(|| "kassignerDerivation must be an object".to_string())?;
    Ok(Some((
        parse_derivation_branch(hint)?,
        parse_derivation_index(hint)?,
    )))
}

fn parse_derivation_branch(hint: &Map<String, Value>) -> Result<u8, String> {
    let value = hint
        .get("branch")
        .ok_or_else(|| "kassignerDerivation.branch is missing".to_string())?;
    let branch = u8::try_from(parse_exact_u64(value, "kassignerDerivation.branch")?)
        .map_err(|_| "kassignerDerivation.branch must be 0 or 1".to_string())?;
    (branch <= 1)
        .then_some(branch)
        .ok_or_else(|| "kassignerDerivation.branch must be 0 or 1".to_string())
}

fn parse_derivation_index(hint: &Map<String, Value>) -> Result<u32, String> {
    let value = hint
        .get("index")
        .ok_or_else(|| "kassignerDerivation.index is missing".to_string())?;
    let index = u32::try_from(parse_exact_u64(value, "kassignerDerivation.index")?)
        .map_err(|_| "kassignerDerivation.index must be a non-hardened u32".to_string())?;
    (index < shared_signer::pairing::SOFT_INDEX_LIMIT)
        .then_some(index)
        .ok_or_else(|| "kassignerDerivation.index must be a non-hardened u32".to_string())
}

#[cfg(test)]
mod unit_tests;
