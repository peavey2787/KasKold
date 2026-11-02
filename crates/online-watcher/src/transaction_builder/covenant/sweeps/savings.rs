//! Time-locked savings spend planning.

use super::super::{
    execution::SpendPath,
    redeem::decode_redeem_script,
    sweep::{self, CovenantSweepSpec, SweepSourceKind},
};
use crate::transaction_builder::pskb::PreparedSweep;

pub(crate) struct SavingsClaimPlan {
    pub(crate) redeem_script: Vec<u8>,
    pub(crate) locktime_daa: u64,
    pub(crate) key_position: u8,
}

impl SavingsClaimPlan {
    /// `key_position` names the signing wallet: 0 the primary, 1 the recovery key.
    pub(crate) fn new(
        redeem_script_hex: &str,
        locktime_daa: u64,
        key_position: u8,
    ) -> Result<Self, String> {
        Ok(Self {
            redeem_script: decode_redeem_script(redeem_script_hex)?,
            locktime_daa,
            key_position,
        })
    }

    pub(crate) fn spec<'a>(
        &'a self,
        covenant_address: &'a str,
        destination_address: &'a str,
        fee: u64,
        source: SweepSourceKind,
    ) -> CovenantSweepSpec<'a> {
        super::timelocked_claim_spec(
            covenant_address,
            destination_address,
            fee,
            source,
            super::TimelockedClaimConfig {
                redeem_script: &self.redeem_script,
                input_sequence: 0,
                lock_time: self.locktime_daa,
                path: SpendPath::Key(self.key_position),
            },
        )
    }
}

pub(crate) async fn build_automatic(
    covenant_address: &str,
    destination_address: &str,
    redeem_script_hex: &str,
    locktime_daa: u64,
    key_position: u8,
    fee: u64,
    websocket_url: &str,
) -> Result<(PreparedSweep, String), String> {
    let plan = SavingsClaimPlan::new(redeem_script_hex, locktime_daa, key_position)?;
    sweep::build_automatic(
        websocket_url,
        plan.spec(
            covenant_address,
            destination_address,
            fee,
            SweepSourceKind::Automatic,
        ),
    )
    .await
}

pub(crate) fn build_selected(
    covenant_address: &str,
    destination_address: &str,
    redeem_script_hex: &str,
    locktime_daa: u64,
    key_position: u8,
    utxos_json: &str,
    fee: u64,
) -> Result<(PreparedSweep, String), String> {
    let plan = SavingsClaimPlan::new(redeem_script_hex, locktime_daa, key_position)?;
    sweep::build_selected(
        utxos_json,
        plan.spec(
            covenant_address,
            destination_address,
            fee,
            SweepSourceKind::Selected,
        ),
    )
}
