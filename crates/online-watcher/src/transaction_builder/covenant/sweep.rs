//! Browser-neutral covenant sweep planning.
//!
//! This is the authoritative application layer for one-source covenant sweeps.
//! WASM adapters may choose an exported operation and translate errors, but UTXO
//! acquisition, monetary validation, lock/branch policy and PSKB construction
//! stay here.

use super::{
    execution::{execution_for, SpendPath},
    redeem::require_redeem_for_address,
};
use crate::{
    network,
    transaction_builder::pskb::{
        encode_prepared_sweep, prepare_selected_sweep, prepare_sweep_from_utxos, PreparedSweep,
        PskbGlobalPlan, SweepInputPolicy,
    },
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SweepSourceKind {
    Automatic,
    Selected,
}

impl SweepSourceKind {
    #[must_use]
    pub(crate) fn choose<'a>(self, automatic: &'a str, selected: &'a str) -> &'a str {
        match self {
            Self::Automatic => automatic,
            Self::Selected => selected,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct CovenantSweepConfig<'a> {
    pub(crate) redeem_script: &'a [u8],
    pub(crate) input_sequence: u64,
    pub(crate) lock_time: u64,
    pub(crate) path: SpendPath,
}

#[derive(Clone, Copy)]
pub(crate) struct CovenantSweepSpec<'a> {
    pub(crate) covenant_address: &'a str,
    pub(crate) destination_address: &'a str,
    pub(crate) fee: u64,
    pub(crate) empty_error: &'a str,
    pub(crate) low_balance_error: &'a str,
    pub(crate) config: CovenantSweepConfig<'a>,
}

pub(crate) fn encode_covenant_sweep(
    prepared: &PreparedSweep,
    config: CovenantSweepConfig<'_>,
) -> Result<String, String> {
    let global = PskbGlobalPlan::standard().with_lock_time(config.lock_time);
    let policy = SweepInputPolicy::covenant(
        config.redeem_script,
        config.input_sequence,
        execution_for(config.redeem_script, config.path)?,
    );
    encode_prepared_sweep(prepared, global, &policy)
}

pub(crate) async fn prepare_automatic(
    websocket_url: &str,
    spec: CovenantSweepSpec<'_>,
) -> Result<PreparedSweep, String> {
    require_redeem_for_address(spec.covenant_address, spec.config.redeem_script)?;
    let utxos =
        network::queries::utxos::fetch_for_address(websocket_url, spec.covenant_address).await?;
    prepare_sweep_from_utxos(
        utxos,
        spec.covenant_address,
        spec.destination_address,
        spec.fee,
        spec.empty_error,
        spec.low_balance_error,
    )
}

pub(crate) fn prepare_selected(
    utxos_json: &str,
    spec: CovenantSweepSpec<'_>,
) -> Result<PreparedSweep, String> {
    require_redeem_for_address(spec.covenant_address, spec.config.redeem_script)?;
    prepare_selected_sweep(
        utxos_json,
        spec.covenant_address,
        spec.destination_address,
        spec.fee,
        spec.empty_error,
        spec.low_balance_error,
    )
}

pub(crate) async fn build_automatic(
    websocket_url: &str,
    spec: CovenantSweepSpec<'_>,
) -> Result<(PreparedSweep, String), String> {
    let prepared = prepare_automatic(websocket_url, spec).await?;
    let wire = encode_covenant_sweep(&prepared, spec.config)?;
    Ok((prepared, wire))
}

pub(crate) fn build_selected(
    utxos_json: &str,
    spec: CovenantSweepSpec<'_>,
) -> Result<(PreparedSweep, String), String> {
    let prepared = prepare_selected(utxos_json, spec)?;
    let wire = encode_covenant_sweep(&prepared, spec.config)?;
    Ok((prepared, wire))
}
