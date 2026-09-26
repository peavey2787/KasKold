//! Network-free WebAssembly shell for KasKold Vault Web.
//!
//! The browser UI owns only presentation/camera state. Wallet creation,
//! restoration, transaction review, and signing remain inside `vault-runtime`.

use std::cell::RefCell;

use serde::Serialize;
use vault_runtime::{ScanResult, VaultReview, VaultRuntime};
use wasm_bindgen::prelude::*;

mod covenant;
mod onboarding;
mod private_swap;
mod public_io;
mod qr;
mod security;
mod workflows;

pub use covenant::*;
pub use onboarding::*;
pub use private_swap::*;
pub use public_io::*;
pub use security::*;
pub use workflows::*;

use qr::{qr_svg, render_profiled_response_frames, render_response_frames, ResponseFrame};

thread_local! {
    static VAULT: RefCell<VaultRuntime> = RefCell::new(VaultRuntime::new());
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CreationResponse {
    recovery_phrase: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct KpubResponse {
    kpub: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SecretTextResponse {
    value: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressResponse {
    state: &'static str,
    received: u8,
    total: u8,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewInputResponse {
    index: usize,
    outpoint: String,
    amount: String,
    script_type: &'static str,
    address: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewOutputResponse {
    index: usize,
    amount: String,
    ownership: &'static str,
    address: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewResponse {
    state: &'static str,
    network: &'static str,
    input_count: usize,
    output_count: usize,
    input_total: String,
    output_total: String,
    external_total: String,
    change_total: String,
    own_receive_total: String,
    fee: String,
    inputs: Vec<ReviewInputResponse>,
    outputs: Vec<ReviewOutputResponse>,
}

#[wasm_bindgen(start)]
pub fn initialize_application() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn kaskold_vault_create_12() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let created = vault.borrow_mut().create_wallet_12().map_err(vault_error)?;
        json(&CreationResponse {
            recovery_phrase: created.recovery_phrase.to_string(),
        })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_create_24() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let created = vault.borrow_mut().create_wallet_24().map_err(vault_error)?;
        json(&CreationResponse {
            recovery_phrase: created.recovery_phrase.to_string(),
        })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_restore(recovery_phrase: &str, passphrase: &str) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let kpub = vault
            .borrow_mut()
            .restore_wallet(recovery_phrase, passphrase)
            .map_err(vault_error)?;
        json(&KpubResponse { kpub })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_lock() {
    VAULT.with(|vault| vault.borrow_mut().lock_wallet());
}

#[wasm_bindgen]
pub fn kaskold_vault_is_unlocked() -> bool {
    VAULT.with(|vault| vault.borrow().is_unlocked())
}

#[wasm_bindgen]
pub fn kaskold_vault_export_kpub() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let kpub = vault.borrow().export_public_account().map_err(vault_error)?;
        json(&KpubResponse { kpub })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_begin_scan() -> Result<(), JsValue> {
    VAULT.with(|vault| vault.borrow_mut().begin_scan().map_err(vault_error))
}

#[wasm_bindgen]
pub fn kaskold_vault_accept_frame(frame: &[u8]) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let result = vault.borrow_mut().accept_qr_frame(frame).map_err(vault_error)?;
        scan_result_json(result)
    })
}

fn scan_result_json(result: ScanResult) -> Result<String, JsValue> {
    if let ScanResult::Progress(progress) = &result {
        return json(&ProgressResponse {
            state: "progress",
            received: progress.received,
            total: progress.total,
        });
    }
    let ScanResult::Ready(review) = result else {
        unreachable!("scan result is exhaustively covered")
    };
    json(&review_response(review))
}

#[wasm_bindgen]
pub fn kaskold_vault_open_transaction_file(wire: &[u8]) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let review = vault
            .borrow_mut()
            .load_transaction_file(wire)
            .map_err(vault_error)?;
        json(&review_response(review))
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_review() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let review = vault.borrow().review_transaction().map_err(vault_error)?;
        json(&review_response(review))
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_approve(now_unix: f64) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let mut vault = vault.borrow_mut();
        let now_unix = security::validate_host_time(now_unix).map_err(JsValue::from_str)?;
        let rendered = approve_response_frames(&mut vault, now_unix)?;
        json(&rendered)
    })
}

fn approve_response_frames(vault: &mut VaultRuntime, now_unix: u64) -> Result<Vec<ResponseFrame>, JsValue> {
    let frames = vault.approve(now_unix).map_err(vault_error)?;
    render_response_frames(frames)
}

#[wasm_bindgen]
pub fn kaskold_vault_anti_klepto_awaiting_reveal() -> bool {
    VAULT.with(|vault| vault.borrow().anti_klepto_awaiting_reveal())
}

#[wasm_bindgen]
pub fn kaskold_vault_begin_anti_klepto_reveal() -> Result<(), JsValue> {
    VAULT.with(|vault| {
        vault
            .borrow_mut()
            .begin_anti_klepto_reveal()
            .map_err(vault_error)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_finalize_anti_klepto_reveal(
    reveal: &[u8],
    now_unix: f64,
) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let mut vault = vault.borrow_mut();
        let frames = vault
            .finalize_anti_klepto_reveal(
                reveal,
                security::validate_host_time(now_unix).map_err(JsValue::from_str)?,
            )
            .map_err(vault_error)?;
        let rendered = render_response_frames(frames)?;
        json(&rendered)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_signed_transaction() -> Result<Vec<u8>, JsValue> {
    VAULT.with(|vault| {
        vault
            .borrow()
            .signed_response_wire()
            .map(|wire| wire.to_vec())
            .map_err(vault_error)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_signed_response_frames(payload_limit: u32) -> Result<String, JsValue> {
    let payload_limit = usize::try_from(payload_limit)
        .map_err(|_| JsValue::from_str("Invalid QR density."))?;
    VAULT.with(|vault| {
        let vault = vault.borrow();
        let wire = vault.signed_response_wire().map_err(vault_error)?;
        let rendered = render_profiled_response_frames(wire, payload_limit)?;
        json(&rendered)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_reject() {
    VAULT.with(|vault| vault.borrow_mut().reject());
}

#[wasm_bindgen]
pub fn kaskold_vault_kpub_qr_svg() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let kpub = vault.borrow().export_public_account().map_err(vault_error)?;
        qr_svg(kpub.as_bytes())
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_backup_words() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let phrase = vault
            .borrow()
            .backup_recovery_phrase()
            .map_err(vault_error)?;
        json(&SecretTextResponse {
            value: phrase.to_string(),
        })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_seedqr_svg() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let payload = vault.borrow().backup_seedqr().map_err(vault_error)?;
        qr_svg(&payload)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_compact_seedqr_svg() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let payload = vault
            .borrow()
            .backup_compact_seedqr()
            .map_err(vault_error)?;
        qr_svg(&payload)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_plain_seedqr_svg() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let phrase = vault
            .borrow()
            .backup_recovery_phrase()
            .map_err(vault_error)?;
        if phrase.split_whitespace().count() != 12 {
            return Err(JsValue::from_str(
                "Plain-text SeedQR is available only for 12-word wallets.",
            ));
        }
        qr_svg(phrase.as_bytes())
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_backup_xprv() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let xprv = vault.borrow().backup_account_xprv().map_err(vault_error)?;
        json(&SecretTextResponse {
            value: xprv.to_string(),
        })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_export_receive_key(address_index: u32) -> Result<String, JsValue> {
    let address_index = u16::try_from(address_index)
        .map_err(|_| JsValue::from_str("Address index must be between 0 and 65535."))?;
    VAULT.with(|vault| {
        let key = vault
            .borrow()
            .backup_receive_private_key_hex(address_index)
            .map_err(vault_error)?;
        json(&SecretTextResponse {
            value: key.to_string(),
        })
    })
}

fn review_response(review: VaultReview) -> ReviewResponse {
    ReviewResponse {
        state: "review",
        network: review.network,
        input_count: review.input_count,
        output_count: review.output_count,
        input_total: review.input_total.to_string(),
        output_total: review.output_total.to_string(),
        external_total: review.external_total.to_string(),
        change_total: review.change_total.to_string(),
        own_receive_total: review.own_receive_total.to_string(),
        fee: review.fee.to_string(),
        inputs: review
            .inputs
            .into_iter()
            .map(|input| ReviewInputResponse {
                index: input.index,
                outpoint: input.outpoint,
                amount: input.amount.to_string(),
                script_type: input.script_type,
                address: input.address,
            })
            .collect(),
        outputs: review
            .outputs
            .into_iter()
            .map(|output| ReviewOutputResponse {
                index: output.index,
                amount: output.amount.to_string(),
                ownership: output.ownership,
                address: output.address,
            })
            .collect(),
    }
}

fn json<T: Serialize>(value: &T) -> Result<String, JsValue> {
    serde_json::to_string(value).map_err(|error| JsValue::from_str(&error.to_string()))
}

fn vault_error(error: impl core::fmt::Debug) -> JsValue {
    JsValue::from_str(&format!("KasKold Vault error: {error:?}"))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(char::from(HEX[usize::from(byte >> 4)]));
        out.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    out
}


#[cfg(test)]
#[path = "unit_tests/mod.rs"]
mod unit_tests;
