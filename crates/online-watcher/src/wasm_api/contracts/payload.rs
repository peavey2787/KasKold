use crate::wasm_api::utilities::common::js_error;
use crate::wasm_api::JsValue;

/// Build the plaintext covenant payload blob: [version:1][type:1][params...]
/// version = 0x01, type = covenant type byte. Caller provides params as hex.
/// Returns hex of the assembled plaintext (ready for AES-GCM encryption in JS).
pub(crate) fn build_covenant_payload_string(
    covenant_type: u8,
    params_hex: &str,
) -> Result<String, String> {
    let params = hex::decode(params_hex).map_err(|error| format!("Bad params hex: {error}"))?;
    let mut blob = Vec::with_capacity(2 + params.len());
    blob.push(0x01);
    blob.push(covenant_type);
    blob.extend_from_slice(&params);
    Ok(hex::encode(&blob))
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn build_covenant_payload(covenant_type: u8, params_hex: &str) -> Result<String, JsValue> {
    build_covenant_payload_string(covenant_type, params_hex).map_err(js_error)
}

/// Parse a decrypted covenant payload blob: [version:1][type:1][params...]
/// Returns JSON: { "version": 1, "covenant_type": N, "params_hex": "..." }
pub(crate) fn parse_covenant_payload_string(plaintext_hex: &str) -> Result<String, String> {
    let blob = hex::decode(plaintext_hex).map_err(|error| format!("Bad plaintext hex: {error}"))?;
    if blob.len() < 2 {
        return Err("Payload too short".to_string());
    }
    let result = serde_json::json!({
        "version": blob[0],
        "covenant_type": blob[1],
        "params_hex": hex::encode(&blob[2..]),
    });
    serde_json::to_string(&result).map_err(|error| error.to_string())
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn parse_covenant_payload(plaintext_hex: &str) -> Result<String, JsValue> {
    parse_covenant_payload_string(plaintext_hex).map_err(js_error)
}
