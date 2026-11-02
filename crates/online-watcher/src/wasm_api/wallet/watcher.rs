use crate::network;
use crate::wasm_api::utilities::common::{js_error, serialize_json};
use crate::wasm_api::JsValue;

/// Query node for current fee rates → return JSON
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn get_fee_estimate(ws_url: &str) -> Result<String, JsValue> {
    let estimate = network::queries::fees::get(ws_url)
        .await
        .map_err(js_error)?;
    serialize_json(&estimate)
}

/// Fetch UTXOs for a single address (for multisig balance check) → JSON array
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn fetch_utxos_for_address_js(address: &str, ws_url: &str) -> Result<String, JsValue> {
    let utxos = network::queries::utxos::fetch_for_address(ws_url, address)
        .await
        .map_err(js_error)?;
    serialize_json(&utxos)
}

/// Get the current virtual DAA score from the node.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn get_virtual_daa_score(ws_url: &str) -> Result<String, JsValue> {
    let daa = network::queries::chain::virtual_daa_score(ws_url)
        .await
        .map_err(js_error)?;
    Ok(daa.to_string())
}

/// Build a NotifyVirtualChainChanged subscribe request (raw bytes).
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn build_vcc_subscribe_request(request_id: u64) -> Result<Vec<u8>, JsValue> {
    network::codec::requests::subscription::block_added(request_id)
        .map_err(|error| js_error(error.to_string()))
}
