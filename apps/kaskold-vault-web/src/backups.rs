//! Portable and steganographic wallet backups for the web vault.

use vault_runtime::VaultRuntime;
use wasm_bindgen::prelude::*;

use super::workflows::wallet_response;
use super::{json, vault_error, VAULT};

#[wasm_bindgen]
pub fn kaskold_vault_portable_backup(password: &str) -> Result<Vec<u8>, JsValue> {
    VAULT.with(|vault| {
        vault
            .borrow()
            .portable_backup(password)
            .map_err(vault_error)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_portable_xprv_backup(password: &str) -> Result<Vec<u8>, JsValue> {
    VAULT.with(|vault| {
        vault
            .borrow()
            .portable_xprv_backup(password)
            .map_err(vault_error)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_portable_backup_is_current(data: &[u8]) -> bool {
    VaultRuntime::portable_backup_is_current(data)
}

#[wasm_bindgen]
pub fn kaskold_vault_restore_portable(data: &[u8], password: &str) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let wallet = vault
            .borrow_mut()
            .add_portable_backup(data, password)
            .map_err(vault_error)?;
        json(&wallet_response(wallet))
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_stego_backup(jpeg: &[u8], password: &str) -> Result<Vec<u8>, JsValue> {
    VAULT.with(|vault| {
        vault
            .borrow()
            .stego_backup(jpeg, password)
            .map_err(vault_error)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_restore_stego(jpeg: &[u8], password: &str) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let wallet = vault
            .borrow_mut()
            .add_stego_backup(jpeg, password)
            .map_err(vault_error)?;
        json(&wallet_response(wallet))
    })
}
