//! Thin WASM adapters for commit-reveal contracts and spends.

use crate::wasm_api::JsValue;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn covenant_commit_reveal(
    owner_pubkey_hex: &str,
    committed_hash_hex: &str,
    locktime_daa: u64,
    network: &str,
) -> Result<String, JsValue> {
    build_commit_reveal_json(owner_pubkey_hex, committed_hash_hex, locktime_daa, network)
        .map_err(|error| wasm_error!(&error))
}

pub(super) fn build_commit_reveal_json(
    owner_pubkey_hex: &str,
    committed_hash_hex: &str,
    locktime_daa: u64,
    network: &str,
) -> Result<String, String> {
    crate::contracts::commit_reveal::application::build_json(
        owner_pubkey_hex,
        committed_hash_hex,
        locktime_daa,
        network,
    )
}

#[cfg(test)]
pub(super) use crate::transaction_builder::zk::commit_reveal::parse as parse_commit_reveal_spend;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn create_commit_reveal_spend(request_json: &str) -> Result<String, JsValue> {
    crate::transaction_builder::zk::commit_reveal::build(request_json)
        .await
        .map_err(|error| wasm_error!(&error))
}

/// JSON `{preimage_hex, committed_hash, computed_hash, matches}` for a
/// commit-reveal reveal signature script, read and hashed by Kaspa Portal.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn read_commit_reveal_reveal(signature_script_hex: &str) -> Result<String, JsValue> {
    read_reveal_json(signature_script_hex).map_err(|error| wasm_error!(&error))
}

pub(super) fn read_reveal_json(signature_script_hex: &str) -> Result<String, String> {
    let script = hex::decode(signature_script_hex).map_err(|error| error.to_string())?;
    let reveal = kaspa_portal::contract::commit_reveal::read_commit_reveal_reveal(&script)
        .ok_or("not a commit-reveal reveal signature script")?;
    Ok(serde_json::json!({
        "preimage_hex": hex::encode(&reveal.preimage),
        "committed_hash": hex::encode(reveal.fields.committed_hash),
        "computed_hash": hex::encode(kaspa_portal::primitives::hash::blake2b_256(&reveal.preimage)),
        "matches": reveal.matches(),
    })
    .to_string())
}

#[cfg(test)]
mod unit_tests;
