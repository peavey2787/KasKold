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

#[derive(Clone, Copy)]
pub(crate) enum ErrorStyle {
    Standard,
    Review,
}

#[derive(Clone, Copy)]
enum PskbShapeStyle {
    Standard,
    Review,
}

/// Detect the outer PSKT/PSKB wire envelope without decoding the payload.
pub fn detect_format_hex(hex_str: &str) -> PsktFormat {
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

fn decode_root_with_style(
    wire_hex: &str,
    style: ErrorStyle,
) -> Result<(PsktFormat, Value), String> {
    decode_wire(wire_hex).map_err(|error| format_wire_error(error, style))
}

pub(crate) fn format_wire_error(error: PsktWireError, style: ErrorStyle) -> String {
    match (style, error) {
        (_, PsktWireError::UnknownFormat) => "Not a PSKT/PSKB payload".to_string(),
        (ErrorStyle::Standard, PsktWireError::OuterHex(message)) => {
            format!("outer hex: {message}")
        }
        (ErrorStyle::Review, PsktWireError::OuterHex(message)) => {
            format!("Bad outer hex: {message}")
        }
        (ErrorStyle::Standard, PsktWireError::TooShort) => "payload too short".to_string(),
        (ErrorStyle::Review, PsktWireError::TooShort) => "Payload too short".to_string(),
        (_, PsktWireError::MagicMismatch) => {
            "wire magic does not match detected format".to_string()
        }
        #[cfg(test)]
        (ErrorStyle::Standard, PsktWireError::InnerHex(message)) => {
            format!("inner hex: {message}")
        }
        #[cfg(test)]
        (ErrorStyle::Review, PsktWireError::InnerHex(message)) => {
            format!("Bad inner hex: {message}")
        }
        (_, PsktWireError::Json(message)) => format!("JSON parse: {message}"),
    }
}

pub(crate) fn decode_root(wire_hex: &str) -> Result<(PsktFormat, Value), String> {
    decode_root_with_style(wire_hex, ErrorStyle::Standard)
}

pub(crate) fn decode_root_for_review(wire_hex: &str) -> Result<(PsktFormat, Value), String> {
    decode_root_with_style(wire_hex, ErrorStyle::Review)
}

fn validate_single_pskt(
    root: &Value,
    format: PsktFormat,
    style: PskbShapeStyle,
) -> Result<(), String> {
    match format {
        PsktFormat::Pskb => {
            let entries = root.as_array().ok_or_else(|| match style {
                PskbShapeStyle::Standard => "PSKB not array".to_string(),
                PskbShapeStyle::Review => "PSKB body is not an array".to_string(),
            })?;
            if entries.len() != 1 {
                return Err(match style {
                    PskbShapeStyle::Standard => {
                        format!("PSKB must have 1 entry, got {}", entries.len())
                    }
                    PskbShapeStyle::Review => {
                        format!("PSKB must wrap exactly 1 PSKT, got {}", entries.len())
                    }
                });
            }
            Ok(())
        }
        PsktFormat::PsktSingle => Ok(()),
        PsktFormat::Unknown => Err("Not a PSKT/PSKB payload".into()),
    }
}

fn validated_pskt(
    root: &Value,
    format: PsktFormat,
    style: PskbShapeStyle,
) -> Result<&Value, String> {
    validate_single_pskt(root, format, style)?;
    Ok(match format {
        PsktFormat::Pskb => &root.as_array().expect("validated PSKB array")[0],
        PsktFormat::PsktSingle => root,
        PsktFormat::Unknown => unreachable!("validated format"),
    })
}

pub(crate) fn pskt_from_root_for_review(
    root: &Value,
    format: PsktFormat,
) -> Result<&Value, String> {
    validated_pskt(root, format, PskbShapeStyle::Review)
}

pub(crate) fn pskt_from_root_mut(
    root: &mut Value,
    format: PsktFormat,
) -> Result<&mut Value, String> {
    validate_single_pskt(root, format, PskbShapeStyle::Standard)?;
    Ok(match format {
        PsktFormat::Pskb => &mut root.as_array_mut().expect("validated PSKB array")[0],
        PsktFormat::PsktSingle => root,
        PsktFormat::Unknown => unreachable!("validated format"),
    })
}

pub(crate) fn first_pskt_from_pskb_mut(root: &mut Value) -> Result<&mut Value, String> {
    pskb_entries_mut(root)?
        .first_mut()
        .ok_or("empty PSKB".to_string())
}

fn pskb_entries_mut(root: &mut Value) -> Result<&mut Vec<Value>, String> {
    root.as_array_mut().ok_or("PSKB not array".to_string())
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
