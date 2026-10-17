//! Covenant fee limits and thread-withdrawal pricing, all from Kaspa Portal.

use crate::wasm_api::utilities::common::js_error;
use crate::wasm_api::JsValue;
use kaspa_portal::transaction::mass::ThreadWithdrawFee;

/// Fee a one-thread covenant withdrawal must pay: KIP-9 storage mass or
/// compute mass at `fee_rate_sompi_per_gram`, never below `minimum_fee`.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn thread_withdraw_fee(
    thread_amount: u64,
    withdraw: u64,
    minimum_fee: u64,
    fee_rate_sompi_per_gram: u64,
) -> Result<u64, JsValue> {
    ThreadWithdrawFee {
        thread_amount,
        withdraw,
        minimum_fee,
        fee_rate_sompi_per_gram,
    }
    .calculate()
    .map_err(js_error)
}

/// Largest withdrawal, at most `cap`, that still leaves `minimum_return`
/// in the covenant after its priced fee; 0 when none fits.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn thread_maximum_withdraw(
    thread_amount: u64,
    cap: u64,
    minimum_fee: u64,
    fee_rate_sompi_per_gram: u64,
    minimum_return: u64,
) -> u64 {
    ThreadWithdrawFee {
        thread_amount,
        withdraw: 0,
        minimum_fee,
        fee_rate_sompi_per_gram,
    }
    .maximum_withdraw(cap, minimum_return)
}

/// The fee ceiling the Private Swap covenant script enforces.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn private_swap_max_fee_sompi() -> u64 {
    kaspa_portal::contract::covenant::PRIVATE_SWAP_MAX_FEE_SOMPI
}
