use crate::wasm_api::JsValue;

/// Derive the 32-byte AES-256 key used for encrypting covenant payloads.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn derive_covenant_payload_key(kpub_text: &str) -> Result<String, JsValue> {
    let parts = account_parts(kpub_text)?;
    let mut input = Vec::with_capacity(32 + 20);
    input.extend_from_slice(&parts.chain_code);
    input.extend_from_slice(b"covenant-payload-key");
    Ok(hex::encode(kaspa_portal::primitives::hash::blake2b_256(
        &input,
    )))
}

/// Extract the account-level x-only public key from canonical `kpub1:` text.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn parse_kpub(kpub_text: &str) -> Result<String, JsValue> {
    let parts = account_parts(kpub_text)?;
    let key = &parts.pubkey;
    if !matches!(key[0], 0x02 | 0x03) {
        return Err(wasm_error!(
            "Account key has an invalid compressed-key prefix"
        ));
    }
    let result = serde_json::json!({
        "account_pubkey": hex::encode(&key[1..]),
    });
    serde_json::to_string(&result).map_err(|error| wasm_error!(&error.to_string()))
}

fn account_parts(kpub_text: &str) -> Result<kaspa_portal::wallet::key::xpub::KpubParts, JsValue> {
    kaspa_portal::wallet::key::xpub::parse_kpub_parts(kpub_text.trim().as_bytes())
        .ok_or_else(|| wasm_error!("Invalid kpub"))
}
