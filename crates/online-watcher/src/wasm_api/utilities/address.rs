//! Kaspa addresses, network names and address prefixes, from Kaspa Portal.

use crate::account::address::network_prefix;
use crate::wasm_api::utilities::common::js_error;
use crate::wasm_api::JsValue;
use kaspa_portal::primitives::address::{
    address_to_script_pubkey, script_address, validate_kaspa_address, KaspaNetwork,
};

/// Whether `text` is a valid Kaspa address on any network.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn is_kaspa_address(text: &str) -> bool {
    validate_kaspa_address(text.as_bytes())
}

/// Script public key (hex, without the version) an address locks to.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn address_script_public_key(address: &str) -> Result<String, JsValue> {
    address_to_script_pubkey(address)
        .map(hex::encode)
        .map_err(js_error)
}

/// Address a standard P2PK or P2SH script public key (hex, without the
/// version) pays, or empty for any other script.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn script_public_key_address(script_hex: &str, network: &str) -> String {
    hex::decode(script_hex)
        .ok()
        .and_then(|script| script_address(&script, network_prefix(network).ok()?))
        .unwrap_or_default()
}

/// P2SH address of a redeem script (hex).
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn redeem_script_address(redeem_script_hex: &str, network: &str) -> Result<String, JsValue> {
    let script = hex::decode(redeem_script_hex).map_err(|error| js_error(error.to_string()))?;
    crate::protocol::script::p2sh::script_to_address(
        &script,
        network_prefix(network).map_err(js_error)?,
    )
    .map_err(js_error)
}

/// Address prefix with its separator (`kaspatest:`), or empty when the
/// network name is unknown.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn network_address_prefix(network: &str) -> String {
    KaspaNetwork::from_name(network)
        .and_then(KaspaNetwork::hrp)
        .map_or_else(String::new, |hrp| format!("{hrp}:"))
}

/// Network an address belongs to. A testnet address keeps the selected
/// testnet name (`testnet-12`) and defaults to `testnet-10`; anything
/// unrecognized is mainnet.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn address_network(address: &str, selected_network: &str) -> String {
    let network = address
        .split_once(':')
        .and_then(|(hrp, _)| KaspaNetwork::from_hrp(hrp));
    match network {
        Some(KaspaNetwork::Testnet) if selected_network.starts_with("testnet") => {
            selected_network.to_string()
        }
        Some(KaspaNetwork::Testnet) => "testnet-10".to_string(),
        Some(KaspaNetwork::Simnet) => "simnet".to_string(),
        Some(KaspaNetwork::Devnet) => "devnet".to_string(),
        _ => "mainnet".to_string(),
    }
}
