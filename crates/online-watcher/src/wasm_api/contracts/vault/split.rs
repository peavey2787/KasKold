use crate::wasm_api::JsValue;

use super::{
    genesis::{build_vault_genesis_pskb, VaultGenesisKind},
    spend::{build_vault_spend_pskb, VaultSpendKind},
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn split_vault_genesis_pskb(
    wallet_json: &str,
    owner_pubkey_hex: &str,
    send_amount: u64,
    fee: u64,
    network: &str,
    ws_url: &str,
) -> Result<String, JsValue> {
    build_vault_genesis_pskb(
        VaultGenesisKind::Split,
        wallet_json,
        owner_pubkey_hex,
        send_amount,
        fee,
        network,
        ws_url,
    )
    .await
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn split_vault_spend_pskb(
    covenant_address: &str,
    owner_pubkey_hex: &str,
    covenant_id_hex: &str,
    fee: u64,
    network: &str,
    ws_url: &str,
) -> Result<String, JsValue> {
    build_vault_spend_pskb(
        VaultSpendKind::Split,
        covenant_address,
        owner_pubkey_hex,
        covenant_id_hex,
        fee,
        network,
        ws_url,
    )
    .await
}
