//! Private Swap v2 Web presentation adapter.
//!
//! All key derivation, binding validation, adaptor signing, anti-klepto reveal
//! verification, and signature completion remain inside Rust custody.

use serde::Serialize;
use shared_signer::covenant_sign::private_swap::{self as wire, ResponseKind};
use vault_runtime::{PrivateSwapMode, PrivateSwapPrepareResult, PrivateSwapReview};
use wasm_bindgen::prelude::*;

use super::{hex_encode, json, qr_svg, vault_error, VAULT};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PrivateSwapReviewResponse {
    state: &'static str,
    mode: &'static str,
    key_id_hex: String,
    claim_pubkey_hex: String,
    adaptor_point_hex: String,
    script_hash_hex: String,
    sighash_hex: String,
    input_amount: String,
    output_amount: String,
    fee: String,
    refund_locktime_daa: String,
    destination_hash_hex: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PrivateSwapResponsePayload {
    state: &'static str,
    kind: &'static str,
    payload_hex: String,
    svg: String,
    awaiting_reveal: bool,
}

#[wasm_bindgen]
pub fn kaskold_vault_private_swap_prepare(wire_bytes: &[u8]) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let result = vault
            .borrow_mut()
            .private_swap_prepare(wire_bytes)
            .map_err(vault_error)?;
        match result {
            PrivateSwapPrepareResult::Review(review) => json(&review_response(*review)),
            PrivateSwapPrepareResult::Response(response) => response_json(&response),
        }
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_private_swap_confirm() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let response = vault
            .borrow_mut()
            .private_swap_confirm()
            .map_err(vault_error)?;
        response_json(&response)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_private_swap_reveal(wire_bytes: &[u8]) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let response = vault
            .borrow_mut()
            .private_swap_reveal(wire_bytes)
            .map_err(vault_error)?;
        response_json(&response)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_private_swap_cancel() {
    VAULT.with(|vault| vault.borrow_mut().private_swap_cancel());
}

fn review_response(review: PrivateSwapReview) -> PrivateSwapReviewResponse {
    PrivateSwapReviewResponse {
        state: "review",
        mode: mode_label(review.mode),
        key_id_hex: hex_encode(&review.key_id),
        claim_pubkey_hex: hex_encode(&review.claim_pubkey),
        adaptor_point_hex: hex_encode(&review.adaptor_point),
        script_hash_hex: hex_encode(&review.script_hash),
        sighash_hex: hex_encode(&review.sighash),
        input_amount: review.input_amount.to_string(),
        output_amount: review.output_amount.to_string(),
        fee: review.fee.to_string(),
        refund_locktime_daa: review.refund_locktime_daa.to_string(),
        destination_hash_hex: hex_encode(&review.destination_hash),
    }
}

pub(crate) fn response_json(bytes: &[u8]) -> Result<String, JsValue> {
    let parsed = wire::parse_response(bytes)
        .map_err(|_| JsValue::from_str("Vault produced an invalid Private Swap response."))?;
    json(&PrivateSwapResponsePayload {
        state: "response",
        kind: response_kind_label(parsed.kind),
        payload_hex: hex_encode(bytes),
        svg: qr_svg(bytes)?,
        awaiting_reveal: matches!(parsed.kind, ResponseKind::Nonce),
    })
}

pub(crate) const fn mode_label(mode: PrivateSwapMode) -> &'static str {
    match mode {
        PrivateSwapMode::None => "None",
        PrivateSwapMode::KeyInfo => "KeyInfo",
        PrivateSwapMode::Bind => "Bind",
        PrivateSwapMode::PreSign => "PreSign",
        PrivateSwapMode::Complete => "Complete",
    }
}

pub(crate) const fn response_kind_label(kind: ResponseKind) -> &'static str {
    match kind {
        ResponseKind::KeyInfo => "KeyInfo",
        ResponseKind::Binding => "Binding",
        ResponseKind::Nonce => "Nonce",
        ResponseKind::PreSignature => "PreSignature",
        ResponseKind::Completed => "Completed",
    }
}
