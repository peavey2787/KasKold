use crate::wasm_api::JsValue;

/// Oracle Model-B genesis covenant.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn covenant_oracle_mb(request_json: &str) -> Result<String, JsValue> {
    build_oracle_genesis_json(request_json).map_err(|error| wasm_error!(&error))
}

pub(crate) fn build_oracle_genesis_json(request_json: &str) -> Result<String, String> {
    crate::contracts::oracle::genesis::build_json(request_json)
}
