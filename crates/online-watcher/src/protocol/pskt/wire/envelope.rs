// KasKold Companion Web — PSKT / PSKB envelope encoding
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use serde_json::Value;

use super::super::error::PsktWireError;
use super::super::model::{PsktFormat, PSKB_MAGIC, PSKT_MAGIC};
use super::json::{decode_json_body, encode_json_body};

mod fields;
mod global;
mod inputs;
mod outputs;

use fields::*;
use global::*;
use inputs::*;
use outputs::*;

#[cfg(test)]
mod unit_tests;

/// Detect the outer PSKT/PSKB wire envelope without decoding the payload.
fn detect_format_hex(hex_str: &str) -> PsktFormat {
    let bytes = hex_str.as_bytes();
    let Some(prefix) = bytes.get(..8) else {
        return PsktFormat::Unknown;
    };
    if !prefix.iter().all(u8::is_ascii_hexdigit) {
        return PsktFormat::Unknown;
    }
    if prefix.eq_ignore_ascii_case(b"50534b42") {
        PsktFormat::Pskb
    } else if prefix.eq_ignore_ascii_case(b"50534b54") {
        PsktFormat::PsktSingle
    } else {
        PsktFormat::Unknown
    }
}

fn decode_wire(wire_hex: &str) -> Result<(PsktFormat, Value), PsktWireError> {
    validate_outer_hex(wire_hex)?;
    let format = decoded_format(wire_hex)?;
    let wire = hex::decode(wire_hex).map_err(|error| PsktWireError::OuterHex(error.to_string()))?;
    validate_wire_magic(&wire, format)?;
    let root = decode_json_body(&wire[4..])?;
    validate_consumer_limits(format, &root).map_err(PsktWireError::Json)?;
    Ok((format, root))
}

fn validate_outer_hex(wire_hex: &str) -> Result<(), PsktWireError> {
    if wire_hex.len() > kaskold_protocol::compat::MAX_PSKT_WIRE_HEX_CHARS {
        return Err(PsktWireError::OuterHex(
            "payload exceeds signer-compatible resource ceiling".to_string(),
        ));
    }
    let bytes = wire_hex.as_bytes();
    if !bytes.len().is_multiple_of(2)
        || !bytes
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
    {
        return Err(PsktWireError::OuterHex(
            "outer PSKT/PSKB must be even-length lowercase hexadecimal".to_string(),
        ));
    }
    Ok(())
}

fn decoded_format(wire_hex: &str) -> Result<PsktFormat, PsktWireError> {
    let format = detect_format_hex(wire_hex);
    if format == PsktFormat::Unknown {
        return Err(PsktWireError::UnknownFormat);
    }
    Ok(format)
}

fn validate_wire_magic(wire: &[u8], format: PsktFormat) -> Result<(), PsktWireError> {
    if wire.len() < 4 {
        return Err(PsktWireError::TooShort);
    }
    let expected_magic: &[u8; 4] = match format {
        PsktFormat::Pskb => PSKB_MAGIC,
        PsktFormat::PsktSingle => PSKT_MAGIC,
        PsktFormat::Unknown => return Err(PsktWireError::UnknownFormat),
    };
    if &wire[..4] != expected_magic {
        return Err(PsktWireError::MagicMismatch);
    }
    Ok(())
}

fn validate_consumer_limits(format: PsktFormat, root: &Value) -> Result<(), String> {
    let document = consumer_document(format, root)?;
    validate_schema_object(
        document,
        kaskold_protocol::wire::pskt_schema::Scope::TopLevel,
        "PSKT",
    )?;
    let limits = kaskold_protocol::SIGNER_CAPABILITIES;
    let inputs = required_array(document, "inputs")?;
    let outputs = required_array(document, "outputs")?;
    validate_collection_limits(inputs, outputs, limits)?;
    let global = required_object(document, "global")?;
    validate_global_fields(global, inputs.len(), outputs.len(), limits)?;
    validate_inputs(inputs, limits)?;
    validate_outputs(outputs, inputs.len(), limits)
}

fn consumer_document(
    format: PsktFormat,
    root: &Value,
) -> Result<&serde_json::Map<String, Value>, String> {
    let document = match format {
        PsktFormat::Pskb => {
            let entries = root
                .as_array()
                .ok_or("PSKB body must be an array".to_string())?;
            if entries.len() != 1 {
                return Err(format!(
                    "PSKB must wrap exactly 1 PSKT, got {}",
                    entries.len()
                ));
            }
            &entries[0]
        }
        PsktFormat::PsktSingle => root,
        PsktFormat::Unknown => return Err("unknown PSKT format".to_string()),
    };
    document
        .as_object()
        .ok_or("PSKT body must be an object".to_string())
}

fn validate_collection_limits(
    inputs: &[Value],
    outputs: &[Value],
    limits: kaskold_protocol::SignerCapabilities,
) -> Result<(), String> {
    if inputs.len() > usize::from(limits.max_inputs) {
        return Err(format!(
            "PSKT input count exceeds signer capability: {} > {}",
            inputs.len(),
            limits.max_inputs
        ));
    }
    if outputs.len() > usize::from(limits.max_outputs) {
        return Err(format!(
            "PSKT output count exceeds signer capability: {} > {}",
            outputs.len(),
            limits.max_outputs
        ));
    }
    Ok(())
}

pub(crate) fn format_wire_error(error: PsktWireError) -> String {
    match error {
        PsktWireError::UnknownFormat => "Not a PSKT/PSKB payload".to_string(),
        PsktWireError::OuterHex(message) => {
            format!("outer hex: {message}")
        }
        PsktWireError::TooShort => "payload too short".to_string(),
        PsktWireError::MagicMismatch => "wire magic does not match detected format".to_string(),
        PsktWireError::Json(message) => format!("JSON parse: {message}"),
    }
}

pub(crate) fn decode_root(wire_hex: &str) -> Result<(PsktFormat, Value), String> {
    decode_wire(wire_hex).map_err(format_wire_error)
}

fn validate_single_pskt(root: &Value, format: PsktFormat) -> Result<(), String> {
    match format {
        PsktFormat::Pskb => {
            let entries = root
                .as_array()
                .ok_or_else(|| "PSKB not array".to_string())?;
            if entries.len() != 1 {
                return Err(format!("PSKB must have 1 entry, got {}", entries.len()));
            }
            Ok(())
        }
        PsktFormat::PsktSingle => Ok(()),
        PsktFormat::Unknown => Err("Not a PSKT/PSKB payload".into()),
    }
}

pub(crate) fn pskt_from_root_mut(
    root: &mut Value,
    format: PsktFormat,
) -> Result<&mut Value, String> {
    validate_single_pskt(root, format)?;
    Ok(match format {
        PsktFormat::Pskb => &mut root.as_array_mut().expect("validated PSKB array")[0],
        PsktFormat::PsktSingle => root,
        PsktFormat::Unknown => unreachable!("validated format"),
    })
}

pub(crate) fn encode_root(format: PsktFormat, root: &Value) -> Result<String, String> {
    validate_consumer_limits(format, root)?;
    let body = encode_json_body(root)?;
    let magic: &[u8; 4] = match format {
        PsktFormat::Pskb => PSKB_MAGIC,
        PsktFormat::PsktSingle => PSKT_MAGIC,
        PsktFormat::Unknown => return Err("cannot encode unknown PSKT format".into()),
    };
    let mut wire = Vec::with_capacity(4 + body.len());
    wire.extend_from_slice(magic);
    wire.extend_from_slice(&body);
    Ok(hex::encode(wire))
}
