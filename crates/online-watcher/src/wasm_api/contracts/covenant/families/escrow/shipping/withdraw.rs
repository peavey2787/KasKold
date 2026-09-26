//! Thin WASM boundary for shipping-escrow withdrawals.

use crate::wasm_api::JsValue;

#[cfg(test)]
pub(super) use crate::transaction_builder::covenant::shipping::withdraw::build_withdrawal;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn create_covenant_borrower_withdraw(
    borrower_wallet_json: &str,
    covenant_address: &str,
    redeem_script_hex: &str,
    withdraw_sompi: u64,
    fee: u64,
    ws_url: &str,
) -> Result<String, JsValue> {
    let _ = (
        borrower_wallet_json,
        covenant_address,
        redeem_script_hex,
        withdraw_sompi,
        fee,
        ws_url,
    );
    Err(wasm_error!("Shipping borrower-withdraw is disabled until its zero-signature covenant branch has a typed verified witness plan"))
}

#[cfg(test)]
pub(super) fn log_summary(
    summary: &crate::transaction_builder::covenant::shipping::withdraw::WithdrawalSummary,
    fee: u64,
) {
    crate::infrastructure::log_info(format!(
        "[Companion] Covenant borrower-withdraw PSKB: 1 covenant + {} funding inputs, return={}, withdraw={}, fee={}, wire {} chars",
        summary.funding_count,
        summary.covenant_return,
        summary.borrower_receive,
        fee,
        summary.wire.len()
    ));
}
