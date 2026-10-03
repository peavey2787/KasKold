//! Strict parsing of Merkle inclusion proofs supplied with a claim.

use super::{Map, Value};

pub(crate) struct MerkleProofItem {
    pub(crate) sibling: [u8; 32],
    pub(crate) direction: u8,
}

pub(crate) fn parse_merkle_proof(
    value: Option<&Value>,
    index: usize,
) -> Result<Vec<MerkleProofItem>, String> {
    let array = value
        .and_then(Value::as_array)
        .ok_or_else(|| format!("input[{index}].proprietaries.merkleProof must be an array"))?;
    if array.len() > 15 {
        return Err(format!("input[{index}] merkle proof exceeds 15 levels"));
    }
    array
        .iter()
        .enumerate()
        .map(|(level, value)| parse_merkle_proof_item(value, index, level))
        .collect()
}

pub(crate) fn parse_merkle_proof_item(
    value: &Value,
    index: usize,
    level: usize,
) -> Result<MerkleProofItem, String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("input[{index}] merkle proof[{level}] must be an object"))?;
    validate_merkle_proof_item_shape(object, index, level)?;
    let sibling = parse_merkle_sibling(object, index, level)?;
    let direction = parse_merkle_direction(object, index, level)?;
    Ok(MerkleProofItem { sibling, direction })
}

pub(crate) fn validate_merkle_proof_item_shape(
    object: &Map<String, Value>,
    index: usize,
    level: usize,
) -> Result<(), String> {
    let valid =
        object.len() == 2 && object.contains_key("sibling") && object.contains_key("direction");
    if !valid {
        return Err(format!(
            "input[{index}] merkle proof[{level}] must contain only sibling and direction"
        ));
    }
    Ok(())
}

pub(crate) fn parse_merkle_sibling(
    object: &Map<String, Value>,
    index: usize,
    level: usize,
) -> Result<[u8; 32], String> {
    let sibling_text = object
        .get("sibling")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            format!("input[{index}] merkle proof[{level}].sibling must be lowercase hex")
        })?;
    if sibling_text.len() != 64 || !sibling_text.as_bytes().iter().all(lower_hex_byte) {
        return Err(format!(
            "input[{index}] merkle proof[{level}].sibling must be 32-byte lowercase hex"
        ));
    }
    let sibling_vec =
        hex::decode(sibling_text).map_err(|_| format!("input[{index}] invalid merkle sibling"))?;
    sibling_vec
        .as_slice()
        .try_into()
        .map_err(|_| format!("input[{index}] invalid merkle sibling length"))
}

pub(crate) fn lower_hex_byte(byte: &u8) -> bool {
    byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)
}

pub(crate) fn parse_merkle_direction(
    object: &Map<String, Value>,
    index: usize,
    level: usize,
) -> Result<u8, String> {
    let raw = merkle_direction_u64(object.get("direction"));
    let direction = raw
        .ok_or_else(|| format!("input[{index}] merkle proof[{level}].direction must be 0 or 1"))?;
    let direction =
        u8::try_from(direction).map_err(|_| format!("input[{index}] invalid merkle direction"))?;
    if direction > 1 {
        return Err(format!(
            "input[{index}] merkle proof[{level}].direction must be 0 or 1"
        ));
    }
    Ok(direction)
}

pub(crate) fn merkle_direction_u64(value: Option<&Value>) -> Option<u64> {
    match value {
        Some(Value::Number(number)) => number.as_u64(),
        Some(Value::String(text)) => {
            crate::wire::pskt_schema::parse_canonical_u64_bytes(text.as_bytes()).ok()
        }
        _ => None,
    }
}
