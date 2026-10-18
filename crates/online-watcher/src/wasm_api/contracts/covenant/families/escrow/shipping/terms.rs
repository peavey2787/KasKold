//! Thin WASM adapter for shipment-escrow terms read by Kaspa Portal.

use crate::wasm_api::JsValue;

#[cfg(test)]
pub(crate) use crate::contracts::shipping_escrow::terms::terms_json as ship_escrow_terms_json;

/// JSON `{total_sompi, rem_sompi, fee_sompi, cltv1_deadline, cltv2_deadline,
/// seller_addr, deliverer_addr, buyer_addr}` for a shipment-escrow redeem script.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn ship_escrow_terms(redeem_script_hex: &str, network: &str) -> Result<String, JsValue> {
    crate::contracts::shipping_escrow::terms::terms_json(redeem_script_hex, network)
        .map_err(|error| wasm_error!(&error))
}
