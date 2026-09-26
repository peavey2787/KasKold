//! Thin WASM application shell. All online wallet behavior lives in `online-watcher`.

use wasm_bindgen::{prelude::wasm_bindgen, JsValue as WasmJsValue};

pub use online_watcher::*;

#[wasm_bindgen(start)]
pub fn initialize_application() {
    console_error_panic_hook::set_once();
}

fn sdk_json<T: serde::Serialize>(value: &T) -> Result<String, WasmJsValue> {
    serde_json::to_string(value).map_err(|error| WasmJsValue::from_str(&error.to_string()))
}

fn protocol_error(error: kaskold_sdk::ProtocolError) -> WasmJsValue {
    WasmJsValue::from_str(&error.to_string())
}

fn sdk_error(error: kaskold_sdk::SdkError) -> WasmJsValue {
    WasmJsValue::from_str(&error.to_string())
}

#[wasm_bindgen]
pub fn kaskold_sdk_limits() -> Result<String, WasmJsValue> {
    sdk_json(&kaskold_sdk::limits())
}

// Companion is the reference consumer of the same friendly Rust SDK used by
// third-party wallets. Transaction construction/coin selection remains a
// Companion wallet-policy concern in online-watcher; only KasKold protocol
// preparation and response validation/merge cross this facade.
#[wasm_bindgen]
pub fn kaskold_sdk_prepare(pskt_hex: &str, network: &str) -> Result<String, WasmJsValue> {
    let network = kaskold_sdk::Network::parse(network).map_err(protocol_error)?;
    kaskold_sdk::prepare(pskt_hex, network)
        .map_err(sdk_error)
        .and_then(|value| sdk_json(&value))
}

#[wasm_bindgen]
pub fn kaskold_sdk_complete(
    original_pskt_hex: &str,
    response_hex: &str,
    network: &str,
) -> Result<String, WasmJsValue> {
    parse_sdk_network(network)
        .and_then(|network| prepare_sdk_request(original_pskt_hex, network))
        .and_then(|request| complete_sdk_request(&request, response_hex))
        .and_then(|value| sdk_json(&value))
}

fn parse_sdk_network(network: &str) -> Result<kaskold_sdk::Network, WasmJsValue> {
    kaskold_sdk::Network::parse(network).map_err(protocol_error)
}

fn prepare_sdk_request(
    pskt_hex: &str,
    network: kaskold_sdk::Network,
) -> Result<kaskold_sdk::SigningRequest, WasmJsValue> {
    kaskold_sdk::prepare(pskt_hex, network).map_err(sdk_error)
}

fn complete_sdk_request(
    request: &kaskold_sdk::SigningRequest,
    response_hex: &str,
) -> Result<kaskold_sdk::SignedPskt, WasmJsValue> {
    kaskold_sdk::complete(request, response_hex).map_err(sdk_error)
}
