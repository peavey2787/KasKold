//! Portable session signing policy for the Web Vault.
//!
//! Policy parsing/evaluation lives in `vault-runtime` so every software Vault
//! enforces the same rules. The Web shell supplies only the host UTC value and
//! never claims hardware-RTC tamper resistance.

use wasm_bindgen::prelude::*;

use super::{vault_error, VAULT};

#[wasm_bindgen]
pub fn kaskold_vault_set_session_signing_policy(
    not_before_utc: &str,
    weekly_windows: &str,
) -> Result<(), JsValue> {
    VAULT.with(|vault| {
        vault
            .borrow_mut()
            .set_session_signing_policy(not_before_utc, weekly_windows)
            .map_err(vault_error)
    })
}

#[wasm_bindgen]
pub fn kaskold_vault_clear_session_signing_policy() {
    VAULT.with(|vault| vault.borrow_mut().clear_session_signing_policy());
}

#[wasm_bindgen]
pub fn kaskold_vault_check_session_signing_policy(now_unix: f64) -> Result<(), JsValue> {
    let now_unix = validate_host_time(now_unix).map_err(JsValue::from_str)?;
    VAULT.with(|vault| {
        vault
            .borrow()
            .check_session_signing_policy(now_unix)
            .map_err(vault_error)
    })
}

pub(crate) fn validate_host_time(now_unix: f64) -> Result<u64, &'static str> {
    if !now_unix.is_finite() || now_unix < 0.0 || now_unix > u64::MAX as f64 {
        return Err("Current UTC time is invalid.");
    }
    Ok(now_unix as u64)
}
