//! Public QR/import and privacy-protocol I/O adapters for the Web Vault.

use serde::Serialize;
use wasm_bindgen::prelude::*;

use super::{hex_encode, json, qr_svg, vault_error, VAULT};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PublicImportResponse {
    value: String,
    svg: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BinaryQrResponse {
    payload_hex: String,
    svg: String,
}

#[wasm_bindgen]
pub fn kaskold_vault_normalize_kpub(value: &str) -> Result<String, JsValue> {
    let value = vault_runtime::VaultRuntime::normalize_watch_kpub(value.as_bytes())
        .map_err(vault_error)?;
    public_import_json(value)
}

#[wasm_bindgen]
pub fn kaskold_vault_validate_address(value: &str) -> Result<String, JsValue> {
    let value = vault_runtime::VaultRuntime::validate_multisig_address(value).map_err(vault_error)?;
    public_import_json(value)
}

#[wasm_bindgen]
pub fn kaskold_vault_normalize_covenant_backup(data: &[u8]) -> Result<String, JsValue> {
    let payload = vault_runtime::VaultRuntime::normalize_covenant_backup(data).map_err(vault_error)?;
    covenant_import_json(payload)
}

fn public_import_json(value: String) -> Result<String, JsValue> {
    let svg = qr_svg(value.as_bytes())?;
    json(&PublicImportResponse { value, svg })
}

fn covenant_import_json(payload: Vec<u8>) -> Result<String, JsValue> {
    let svg = qr_svg(&payload)?;
    json(&PublicImportResponse {
        value: hex_encode(&payload),
        svg,
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_payload_qr_svg(payload: &[u8]) -> Result<String, JsValue> {
    qr_svg(payload)
}

#[wasm_bindgen]
pub fn kaskold_vault_stealth_scan(request: &[u8]) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let payload = vault
            .borrow()
            .stealth_scan_request(request)
            .map_err(vault_error)?;
        json(&BinaryQrResponse {
            payload_hex: hex_encode(&payload),
            svg: qr_svg(&payload)?,
        })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_privacy_pairing(request: &[u8]) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let payload = vault
            .borrow()
            .privacy_pairing_response(request)
            .map_err(vault_error)?;
        json(&BinaryQrResponse {
            payload_hex: hex_encode(&payload),
            svg: qr_svg(&payload)?,
        })
    })
}
