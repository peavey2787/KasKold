//! Which path a covenant spend took, read by Kaspa Portal.

use kaspa_portal::contract::covenant::spend_path::{spend_path, SpendPath};

/// `owner`, `heir` or `unknown` for a spend of `redeem_script_hex` whose
/// signature script is `signature_script_hex`.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn covenant_spend_path(signature_script_hex: &str, redeem_script_hex: &str) -> String {
    let (Ok(signature), Ok(redeem)) = (
        hex::decode(signature_script_hex),
        hex::decode(redeem_script_hex),
    ) else {
        return "unknown".into();
    };
    match spend_path(&signature, &redeem) {
        Some(SpendPath::Owner) => "owner",
        Some(SpendPath::Other) => "heir",
        None => "unknown",
    }
    .into()
}
