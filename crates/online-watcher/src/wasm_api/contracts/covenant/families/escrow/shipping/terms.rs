//! Shipment-escrow terms read from a redeem script by Kaspa Portal.

use crate::account::address::encode_p2pk_address;
use crate::wasm_api::utilities::common::network_to_prefix;
use crate::wasm_api::JsValue;

/// JSON `{total_sompi, rem_sompi, fee_sompi, cltv1_deadline, cltv2_deadline,
/// seller_addr, deliverer_addr, buyer_addr}` for a shipment-escrow redeem script.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn ship_escrow_terms(redeem_script_hex: &str, network: &str) -> Result<String, JsValue> {
    ship_escrow_terms_json(redeem_script_hex, network).map_err(|error| wasm_error!(&error))
}

pub(crate) fn ship_escrow_terms_json(
    redeem_script_hex: &str,
    network: &str,
) -> Result<String, String> {
    let script = hex::decode(redeem_script_hex).map_err(|error| error.to_string())?;
    let terms = kaspa_portal::contract::shipping_escrow::ship_escrow_terms(&script)
        .ok_or("unrecognized ship-escrow redeem")?;
    let tranches = terms.tranches()?;
    let prefix = network_to_prefix(network);
    let address = |key: &[u8; 32]| encode_p2pk_address(key, prefix);
    Ok(serde_json::json!({
        "total_sompi": tranches.total.to_string(),
        "rem_sompi": tranches.remainder.to_string(),
        "fee_sompi": terms.fee_sompi.to_string(),
        "cltv1_deadline": terms.cltv1_deadline.to_string(),
        "cltv2_deadline": terms.cltv2_deadline.to_string(),
        "seller_addr": address(&terms.seller_pubkey),
        "deliverer_addr": address(&terms.deliverer_pubkey),
        "buyer_addr": address(&terms.buyer_pubkey),
    })
    .to_string())
}
