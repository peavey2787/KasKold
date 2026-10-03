//! Thin WASM boundary for shipping-escrow deposits.

use crate::wasm_api::JsValue;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn create_covenant_borrower_spend() -> Result<String, JsValue> {
    Err(wasm_error!("Shipping borrower-spend is disabled until its zero-signature covenant branch has a typed verified witness plan"))
}
