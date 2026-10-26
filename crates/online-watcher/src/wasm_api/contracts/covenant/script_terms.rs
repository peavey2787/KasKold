use super::JsValue;

/// Verify a covenant invite or stored record against its redeem script.
/// Returns JSON of the terms read from the script (plus type, address and
/// redeem_script_hex); errors when the type is unsupported, the script is not
/// that family's canonical script, the address does not match the script, or
/// an oracle-v1 statement does not match the script's commitment.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn covenant_script_terms(
    covenant_type: &str,
    redeem_script_hex: &str,
    address: &str,
    network: &str,
    attestation_statement: &str,
) -> Result<String, JsValue> {
    crate::contracts::covenant::script_terms::verified_terms_json(
        covenant_type,
        redeem_script_hex,
        address,
        network,
        attestation_statement,
    )
    .map_err(|error| wasm_error!(&error))
}
