use crate::wasm_api::JsValue;
// ─── Commit-Reveal Covenant (MEV Resistance) ───

/// Compute BLAKE2B hash of a preimage (for creating the commitment).
/// Returns hex string of the 32-byte hash.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn commit_hash(preimage_hex: &str) -> Result<String, JsValue> {
    let preimage =
        hex::decode(preimage_hex).map_err(|e| wasm_error!(&format!("Bad preimage hex: {}", e)))?;
    let hash = kaspa_portal::primitives::hash::blake2b_256(&preimage);
    Ok(hex::encode(hash))
}
