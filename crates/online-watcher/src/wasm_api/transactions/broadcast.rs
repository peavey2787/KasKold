use crate::wasm_api::JsValue;
use crate::{wasm_api::utilities::common::js_error, WatchWallet};

/// Broadcast a signed KSPT hex to the network → return TX ID
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn broadcast_signed(signed_hex: &str, ws_url: &str) -> Result<String, JsValue> {
    WatchWallet::new()
        .broadcast(signed_hex, ws_url)
        .await
        .map_err(js_error)
}

#[cfg(test)]
mod unit_tests;
