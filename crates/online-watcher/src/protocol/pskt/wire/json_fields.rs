use serde_json::{Map, Value};

/// Decode the required 20-byte PSKT subnetwork identifier.
pub(crate) fn decode_subnetwork_id(global: &Map<String, Value>) -> Result<[u8; 20], String> {
    let encoded = global
        .get("subnetworkId")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing subnetworkId".to_string())?;
    let bytes_text = encoded.as_bytes();
    if bytes_text.len() != 40
        || !bytes_text
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err("subnetworkId must be exactly 20 bytes of lowercase hexadecimal".to_string());
    }
    let bytes =
        hex::decode(encoded).map_err(|error| format!("invalid subnetworkId hex: {error}"))?;
    let id: [u8; 20] = bytes
        .try_into()
        .map_err(|bytes: Vec<u8>| format!("subnetworkId must be 20 bytes, got {}", bytes.len()))?;
    Ok(id)
}
