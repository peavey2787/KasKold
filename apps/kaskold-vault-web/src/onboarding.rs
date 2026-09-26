//! M5-compatible software-Vault onboarding entropy boundary.
//!
//! The browser supplies only optional additive dice/touch transcripts. The
//! mandatory entropy pool and BIP39/passphrase handling remain inside Rust.

use wasm_bindgen::prelude::*;

use super::{json, vault_error, CreationResponse, VAULT};

#[wasm_bindgen]
pub fn kaskold_vault_creation_flow() -> String {
    vault_runtime::creation_flow_json()
}

#[wasm_bindgen]
pub fn kaskold_vault_create_with_entropy(
    word_count: u8,
    dice_text: &str,
    touch_transcript: &[u8],
    passphrase: &str,
) -> Result<String, JsValue> {
    let dice_rolls = parse_dice_rolls(dice_text)?;
    VAULT.with(|vault| {
        let created = vault
            .borrow_mut()
            .create_wallet_with_additive_entropy(
                word_count,
                &dice_rolls,
                touch_transcript,
                passphrase,
            )
            .map_err(vault_error)?;
        json(&CreationResponse {
            recovery_phrase: created.recovery_phrase.to_string(),
        })
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_add_create_with_entropy(
    word_count: u8,
    dice_text: &str,
    touch_transcript: &[u8],
    passphrase: &str,
) -> Result<String, JsValue> {
    let dice_rolls = parse_dice_rolls(dice_text)?;
    VAULT.with(|vault| {
        let created = vault
            .borrow_mut()
            .add_wallet_with_additive_entropy(
                word_count,
                &dice_rolls,
                touch_transcript,
                passphrase,
            )
            .map_err(vault_error)?;
        json(&CreationResponse {
            recovery_phrase: created.recovery_phrase.to_string(),
        })
    })
}

fn parse_dice_rolls(text: &str) -> Result<Vec<u8>, JsValue> {
    text.bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .map(|byte| match byte {
            b'1'..=b'6' => Ok(byte - b'0'),
            _ => Err(JsValue::from_str(
                "Dice rolls must contain only digits 1 through 6.",
            )),
        })
        .collect()
}
