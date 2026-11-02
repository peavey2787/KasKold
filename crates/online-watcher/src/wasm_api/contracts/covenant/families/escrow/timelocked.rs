use crate::wasm_api::JsValue;
#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn create_covenant_beneficiary_spend(
    covenant_address: &str,
    dest_address: &str,
    redeem_script_hex: &str,
    locktime_daa: u64,
    fee: u64,
    ws_url: &str,
) -> Result<String, JsValue> {
    let (prepared, wire, displayed_locktime) =
        crate::transaction_builder::covenant::sweeps::timelocked::build_beneficiary_automatic(
            covenant_address,
            dest_address,
            redeem_script_hex,
            locktime_daa,
            fee,
            ws_url,
        )
        .await
        .map_err(|error| wasm_error!(&error))?;
    super::super::logging::log_prepared_sweep_with_detail(
        "Beneficiary-spend PSKB",
        &prepared,
        fee,
        &wire,
        ("locktime", displayed_locktime),
    );
    Ok(wire)
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn create_covenant_beneficiary_spend_selected(
    covenant_address: &str,
    dest_address: &str,
    redeem_script_hex: &str,
    locktime_daa: u64,
    utxos_json: &str,
    fee: u64,
) -> Result<String, JsValue> {
    let (prepared, wire, displayed_locktime) =
        crate::transaction_builder::covenant::sweeps::timelocked::build_beneficiary_selected(
            covenant_address,
            dest_address,
            redeem_script_hex,
            locktime_daa,
            utxos_json,
            fee,
        )
        .map_err(|error| wasm_error!(&error))?;
    super::super::logging::log_prepared_sweep_with_detail(
        "Beneficiary-spend (selected) PSKB",
        &prepared,
        fee,
        &wire,
        ("locktime", displayed_locktime),
    );
    Ok(wire)
}

#[cfg(test)]
pub(crate) use crate::transaction_builder::covenant::sweeps::timelocked::BeneficiarySweepPlan;

#[cfg(test)]
mod unit_tests;
