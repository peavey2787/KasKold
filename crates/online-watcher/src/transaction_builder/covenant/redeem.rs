//! A covenant spend's redeem script, held to the address it claims to unlock.

pub(crate) fn decode_redeem_script(redeem_script_hex: &str) -> Result<Vec<u8>, String> {
    hex::decode(redeem_script_hex).map_err(|error| format!("Bad redeem hex: {error}"))
}

/// A redeem script that does not hash to the covenant address can never spend it.
pub(crate) fn require_redeem_on_prefix(
    covenant_address: &str,
    redeem_script: &[u8],
    prefix: &str,
) -> Result<(), String> {
    if crate::protocol::script::p2sh::script_to_address(redeem_script, prefix)? != covenant_address
    {
        return Err("Covenant address does not match the supplied redeem script".to_string());
    }
    Ok(())
}

pub(crate) fn require_redeem_for_address(
    covenant_address: &str,
    redeem_script: &[u8],
) -> Result<(), String> {
    crate::account::address::address_to_script_pubkey(covenant_address)?;
    let prefix = covenant_address.split(':').next().unwrap_or_default();
    require_redeem_on_prefix(covenant_address, redeem_script, prefix)
}
