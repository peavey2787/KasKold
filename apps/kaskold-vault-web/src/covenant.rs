//! Universal covenant-signing Web bindings.
//!
//! These are presentation adapters around `vault-runtime`; covenant protocol
//! parsing, mnemonic-derived keys, binding validation, and anti-klepto signing
//! never execute in JavaScript.

use serde::Serialize;
use vault_runtime::{CovenantMode, CovenantReview};
use wasm_bindgen::prelude::*;

use super::{hex_encode, json, qr_svg, vault_error, VAULT};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CovenantReviewResponse {
    mode: &'static str,
    scheme: &'static str,
    key_id_hex: String,
    pubkey_hex: String,
    commitment_hex: String,
    script_hash_hex: String,
    context: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CovenantResponse {
    kind: &'static str,
    payload_hex: String,
    svg: String,
    awaiting_reveal: bool,
}

#[wasm_bindgen]
pub fn kaskold_vault_covenant_prepare(wire: &[u8]) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let review = vault
            .borrow_mut()
            .covenant_prepare(wire)
            .map_err(vault_error)?;
        json(&review_response(review))
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_covenant_confirm() -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let bytes = vault.borrow_mut().covenant_confirm().map_err(vault_error)?;
        response_json(&bytes)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_covenant_reveal(wire: &[u8]) -> Result<String, JsValue> {
    VAULT.with(|vault| {
        let bytes = vault
            .borrow_mut()
            .covenant_finalize_reveal(wire)
            .map_err(vault_error)?;
        response_json(&bytes)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_covenant_cancel() {
    VAULT.with(|vault| vault.borrow_mut().covenant_cancel());
}

fn review_response(review: CovenantReview) -> CovenantReviewResponse {
    CovenantReviewResponse {
        mode: mode_label(review.mode),
        scheme: scheme_label(review.scheme),
        key_id_hex: hex_encode(&review.key_id),
        pubkey_hex: hex_encode(&review.pubkey_x),
        commitment_hex: hex_encode(&review.commitment),
        script_hash_hex: hex_encode(&review.script_hash),
        context: review.context,
    }
}

pub(crate) fn response_json(bytes: &[u8]) -> Result<String, JsValue> {
    let parsed = shared_signer::covenant_sign::parse_response(bytes)
        .map_err(|_| JsValue::from_str("Vault produced an invalid covenant response."))?;
    let kind = match parsed.kind {
        shared_signer::covenant_sign::ResponseKind::KeyInfo => "KeyInfo",
        shared_signer::covenant_sign::ResponseKind::NonceCommitment => "NonceCommitment",
        shared_signer::covenant_sign::ResponseKind::Signature => "Signature",
        shared_signer::covenant_sign::ResponseKind::Binding => "Binding",
    };
    json(&CovenantResponse {
        kind,
        payload_hex: hex_encode(bytes),
        svg: qr_svg(bytes)?,
        awaiting_reveal: matches!(
            parsed.kind,
            shared_signer::covenant_sign::ResponseKind::NonceCommitment
        ),
    })
}

pub(crate) const fn mode_label(mode: CovenantMode) -> &'static str {
    match mode {
        CovenantMode::None => "None",
        CovenantMode::KeyInfo => "KeyInfo",
        CovenantMode::BindKnown => "BindKnown",
        CovenantMode::BindOpaque => "BindOpaque",
        CovenantMode::Known => "Known",
        CovenantMode::Opaque => "Opaque",
    }
}

pub(crate) const fn scheme_label(scheme: shared_signer::covenant_sign::KnownScheme) -> &'static str {
    match scheme {
        shared_signer::covenant_sign::KnownScheme::None => "Opaque",
        shared_signer::covenant_sign::KnownScheme::Sha256Preimage => "SHA256 Preimage",
        shared_signer::covenant_sign::KnownScheme::OracleV1 => "Oracle v1",
    }
}
