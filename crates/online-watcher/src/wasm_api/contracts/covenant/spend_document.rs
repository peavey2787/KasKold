//! Covenant spends the Companion shapes itself.

use crate::wasm_api::JsValue;

/// Encode a covenant spend named by its script path; see
/// [`crate::transaction_builder::covenant::spend_document`].
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn covenant_spend_document(request_json: &str) -> Result<String, JsValue> {
    crate::transaction_builder::covenant::spend_document::encode_json(request_json)
        .map_err(|error| wasm_error!(&error))
}
