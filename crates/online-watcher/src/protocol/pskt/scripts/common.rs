// KasKold Companion Web — organized PSKT subsystem
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use serde_json::{Map, Value};

pub(crate) fn strip_optional_covenant_salt(redeem: &[u8]) -> &[u8] {
    if redeem.first() == Some(&0x08)
        && redeem.get(9) == Some(&0x75)
        && matches!(
            redeem.get(10).copied(),
            Some(0x63) | Some(0xb9) | Some(0x20)
        )
    {
        &redeem[10..]
    } else {
        redeem
    }
}

pub(crate) fn first_schnorr_signature<'a>(
    partial_signatures: &'a Map<String, Value>,
    missing_signature: String,
    missing_variant: String,
    bad_length_prefix: Option<&str>,
    decode_prefix: &str,
) -> Result<(&'a str, Vec<u8>), String> {
    let (public_key, signature_value) =
        only_partial_signature(partial_signatures, missing_signature)?;
    let signature_hex = schnorr_hex(signature_value, missing_variant)?;
    let mut signature = decode_schnorr_hex(signature_hex, bad_length_prefix, decode_prefix)?;
    signature.push(0x01); // SIGHASH_ALL
    Ok((public_key.as_str(), signature))
}

fn only_partial_signature(
    partial_signatures: &Map<String, Value>,
    missing_signature: String,
) -> Result<(&String, &Value), String> {
    if partial_signatures.is_empty() {
        return Err(missing_signature);
    }
    if partial_signatures.len() != 1 {
        return Err(format!(
            "single-signature witness requires exactly one partial signature, got {}",
            partial_signatures.len()
        ));
    }
    partial_signatures
        .iter()
        .next()
        .ok_or_else(|| "partial signature map became empty".to_string())
}

fn schnorr_hex(signature_value: &Value, missing_variant: String) -> Result<&str, String> {
    let signature_object = signature_value
        .as_object()
        .ok_or_else(|| "partial signature value must be an object".to_string())?;
    if signature_object.len() != 1 || !signature_object.contains_key("schnorr") {
        return Err("partial signature object must contain only the schnorr field".to_string());
    }
    signature_object
        .get("schnorr")
        .and_then(Value::as_str)
        .ok_or(missing_variant)
}

fn decode_schnorr_hex(
    signature_hex: &str,
    bad_length_prefix: Option<&str>,
    decode_prefix: &str,
) -> Result<Vec<u8>, String> {
    if signature_hex.len() != 128 {
        let prefix = bad_length_prefix.unwrap_or("bad Schnorr signature length");
        return Err(format!("{}: {}", prefix, signature_hex.len()));
    }
    if !signature_hex
        .as_bytes()
        .iter()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(format!(
            "{}: signature must be lowercase hexadecimal",
            decode_prefix
        ));
    }
    hex::decode(signature_hex).map_err(|error| format!("{}: {}", decode_prefix, error))
}

pub(crate) fn push_data_item(ss: &mut Vec<u8>, data: &[u8]) -> Result<(), String> {
    let len = data.len();
    if len == 0 {
        ss.push(0x00); // OP_0 = empty
    } else if len <= 75 {
        ss.push(len as u8);
    } else if len <= 255 {
        ss.push(0x4C);
        ss.push(len as u8);
    } else if len <= 65535 {
        ss.push(0x4D);
        ss.extend_from_slice(&(len as u16).to_le_bytes());
    } else {
        return Err("data item too large".into());
    }
    ss.extend_from_slice(data);
    Ok(())
}

pub(crate) fn push_data_sigscript(buf: &mut Vec<u8>, data: &[u8]) {
    if data.len() <= 75 {
        buf.push(data.len() as u8);
    } else if data.len() <= 255 {
        buf.push(0x4C); // OP_PUSHDATA1
        buf.push(data.len() as u8);
    } else if data.len() <= 65535 {
        buf.push(0x4D); // OP_PUSHDATA2
        buf.extend_from_slice(&(data.len() as u16).to_le_bytes());
    } else {
        buf.push(0x4E); // OP_PUSHDATA4
        buf.extend_from_slice(&(data.len() as u32).to_le_bytes());
    }
    buf.extend_from_slice(data);
}

pub(crate) fn push_int_sigscript(buf: &mut Vec<u8>, value: u64) {
    if value == 0 {
        buf.push(0x00);
    } else if value <= 16 {
        buf.push(0x50 + value as u8);
    } else {
        let encoded = value.to_le_bytes();
        let used = encoded
            .iter()
            .rposition(|byte| *byte != 0)
            .map(|index| index + 1)
            .unwrap_or(1);
        let mut bytes = encoded[..used].to_vec();
        if bytes.last().is_some_and(|b| b & 0x80 != 0) {
            bytes.push(0x00);
        }
        buf.push(bytes.len() as u8);
        buf.extend_from_slice(&bytes);
    }
}

pub fn push_redeem_script(buf: &mut Vec<u8>, redeem: &[u8]) -> Result<(), String> {
    push_data_item(buf, redeem).map_err(|_| "redeem script too large".to_string())
}
