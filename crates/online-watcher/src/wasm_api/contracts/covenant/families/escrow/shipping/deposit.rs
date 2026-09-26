//! Thin WASM boundary for shipping-escrow deposits.

use crate::wasm_api::JsValue;

#[cfg(test)]
pub(super) use crate::transaction_builder::covenant::shipping::deposit::build_deposit;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn create_covenant_borrower_spend(
    borrower_wallet_json: &str,
    covenant_address: &str,
    redeem_script_hex: &str,
    add_amount_sompi: u64,
    fee: u64,
    ws_url: &str,
) -> Result<String, JsValue> {
    let _ = (
        borrower_wallet_json,
        covenant_address,
        redeem_script_hex,
        add_amount_sompi,
        fee,
        ws_url,
    );
    Err(wasm_error!("Shipping borrower-spend is disabled until its zero-signature covenant branch has a typed verified witness plan"))
}

#[cfg(test)]
pub(super) fn log_summary(
    summary: &crate::transaction_builder::covenant::shipping::deposit::DepositSummary,
    fee: u64,
) {
    crate::infrastructure::log_info(format!(
        "[Companion] Covenant borrower-spend PSKB: 1 covenant + {} funding inputs, covenant_out={}, change={}, fee={}, wire {} chars",
        summary.funding_count,
        summary.covenant_output,
        summary.change,
        fee,
        summary.wire.len()
    ));
}
