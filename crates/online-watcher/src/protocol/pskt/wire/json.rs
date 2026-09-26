// KasKold Companion Web — PSKT JSON body encoding
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use serde_json::Value;

use crate::protocol::pskt::error::PsktWireError;

/// Decode through the protocol crate's canonical restricted JSON grammar so
/// Companion and Vault cannot disagree on duplicate keys, escapes, number
/// syntax, nesting, Unicode, or signer-compatible size limits.
pub(crate) fn decode_json_body(body_hex: &[u8]) -> Result<Value, PsktWireError> {
    kaskold_protocol::compat::decode_pskt_json_body(body_hex).map_err(PsktWireError::Json)
}

/// Encode only values that the offline signer can parse canonically.
pub(crate) fn encode_json_body(root: &Value) -> Result<Vec<u8>, String> {
    kaskold_protocol::compat::encode_pskt_json_body(root)
}
