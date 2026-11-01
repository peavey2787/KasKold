//! Lossless JSON representation for consensus `u64` values at browser boundaries.
//!
//! Payloads use canonical decimal strings because JavaScript `Number` cannot
//! represent every `u64`. A JSON number is rejected rather than accepted as a
//! value that may already have been rounded by JavaScript.

use serde::{Deserialize, Deserializer, Serializer};

pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&value.to_string())
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let text = String::deserialize(deserializer)?;
    parse_canonical_decimal(&text).map_err(serde::de::Error::custom)
}

fn parse_canonical_decimal(text: &str) -> Result<u64, &'static str> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("expected an unsigned decimal string");
    }
    if text.len() > 1 && text.as_bytes()[0] == b'0' {
        return Err("decimal string must be canonical (no leading zeroes)");
    }
    text.parse::<u64>()
        .map_err(|_| "decimal string exceeds u64")
}
