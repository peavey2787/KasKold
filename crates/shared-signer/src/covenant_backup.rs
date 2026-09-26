//! Covenant backup wire classification shared by hardware and software Vaults.
//!
//! M5 accepts raw `COVB` / `COVI` payloads and their ASCII-hex representation.
//! Keeping that byte contract here prevents platform shells from inventing
//! different recovery formats.

use crate::bytes::decode_hex_nibble;

pub const MIN_RAW_LEN: usize = 5;
pub const MAX_RAW_LEN: usize = 512;
pub const MIN_HEX_LEN: usize = MIN_RAW_LEN * 2;
pub const MAX_HEX_LEN: usize = MAX_RAW_LEN * 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CovenantBackupError {
    InvalidFormat,
    OutputTooSmall,
}

#[inline]
pub fn is_raw(input: &[u8]) -> bool {
    (MIN_RAW_LEN..=MAX_RAW_LEN).contains(&input.len())
        && (input.starts_with(b"COVB") || input.starts_with(b"COVI"))
}

#[inline]
pub fn is_hex(input: &[u8]) -> bool {
    if !(MIN_HEX_LEN..=MAX_HEX_LEN).contains(&input.len()) || !input.len().is_multiple_of(2) {
        return false;
    }
    let Some(prefix) = decode_four_byte_prefix(input) else {
        return false;
    };
    (prefix == *b"COVB" || prefix == *b"COVI")
        && input.iter().all(|byte| decode_hex_nibble(*byte).is_some())
}

/// Normalize raw/hex covenant backup payload into raw `COVB`/`COVI` bytes.
pub fn normalize(input: &[u8], output: &mut [u8]) -> Result<usize, CovenantBackupError> {
    if is_raw(input) {
        if output.len() < input.len() {
            return Err(CovenantBackupError::OutputTooSmall);
        }
        output[..input.len()].copy_from_slice(input);
        return Ok(input.len());
    }
    if !is_hex(input) {
        return Err(CovenantBackupError::InvalidFormat);
    }
    let len = input.len() / 2;
    if output.len() < len {
        return Err(CovenantBackupError::OutputTooSmall);
    }
    for (index, pair) in input.chunks_exact(2).enumerate() {
        let high = decode_hex_nibble(pair[0]).ok_or(CovenantBackupError::InvalidFormat)?;
        let low = decode_hex_nibble(pair[1]).ok_or(CovenantBackupError::InvalidFormat)?;
        output[index] = (high << 4) | low;
    }
    Ok(len)
}

fn decode_four_byte_prefix(input: &[u8]) -> Option<[u8; 4]> {
    if input.len() < 8 {
        return None;
    }
    let mut prefix = [0u8; 4];
    for index in 0..4 {
        let high = decode_hex_nibble(input[index * 2])?;
        let low = decode_hex_nibble(input[index * 2 + 1])?;
        prefix[index] = (high << 4) | low;
    }
    Some(prefix)
}
