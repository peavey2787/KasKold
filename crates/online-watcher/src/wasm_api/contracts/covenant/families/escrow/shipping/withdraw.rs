//! Thin WASM boundary for shipping-escrow withdrawals.

use crate::wasm_api::JsValue;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn create_covenant_borrower_withdraw() -> Result<String, JsValue> {
    Err(wasm_error!("Shipping borrower-withdraw is disabled until its zero-signature covenant branch has a typed verified witness plan"))
}
