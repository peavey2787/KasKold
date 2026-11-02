pub(crate) mod application;
mod preparation;
mod thread_input;
mod thread_policy;
mod thread_request;

pub use kaspa_portal::transaction::builder::{
    CovenantExecution, CovenantInputPolicy, GlobalThreadPolicy, GlobalThreadWithdrawalRequest,
    PskbApi, PskbGlobalPlan, PskbInputPlan, PskbOutputPlan, PskbPlan, SweepInputPolicy,
};
pub(crate) use preparation::{
    encode_prepared_sweep, prepare_selected_sweep, prepare_sweep_from_utxos, selected_utxo_entry,
    PreparedSweep, SelectedUtxo,
};

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use thread_policy::topup_policy_for;
pub(crate) use thread_policy::{withdrawal_policy_for, GlobalThreadFamily};

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use thread_input::select_wallet_utxos;
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use thread_request::build_topup as build_global_thread_topup;
pub(crate) use thread_request::{
    build_withdrawal as build_global_thread_withdrawal, PreparedWithdrawal, WithdrawalBuildRequest,
};
#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use thread_request::{
    prepare_topup_material as prepare_global_thread_topup_material, PreparedTopup,
};

/// Encode a typed plan as the canonical PSKB wire.
pub fn encode_wire(plan: &PskbPlan) -> Result<String, String> {
    PskbApi.encode(plan).map_err(|error| error.to_string())
}
