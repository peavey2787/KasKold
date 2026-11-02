//! Beneficiary spend planning.

use super::super::{
    execution::SpendPath,
    redeem::decode_redeem_script,
    sweep::{self, CovenantSweepSpec, SweepSourceKind},
};
use crate::transaction_builder::pskb::PreparedSweep;

pub(crate) struct BeneficiarySweepPlan {
    pub(crate) redeem_script: Vec<u8>,
    pub(crate) displayed_locktime: u64,
    pub(crate) input_sequence: u64,
    pub(crate) transaction_locktime: u64,
}

impl BeneficiarySweepPlan {
    pub(crate) fn new(redeem_script_hex: &str, displayed_locktime: u64) -> Result<Self, String> {
        let redeem_script = decode_redeem_script(redeem_script_hex)?;
        let input_sequence =
            crate::protocol::script::extract_csv_sequence(&redeem_script)?.unwrap_or(0);
        Ok(Self {
            redeem_script,
            displayed_locktime,
            input_sequence,
            transaction_locktime: if input_sequence > 0 {
                0
            } else {
                displayed_locktime
            },
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
                input_sequence: self.input_sequence,
                lock_time: self.transaction_locktime,
                path: SpendPath::Key(1),
            },
        )
    }
}

pub(crate) async fn build_beneficiary_automatic(
    covenant_address: &str,
    destination_address: &str,
    redeem_script_hex: &str,
    locktime_daa: u64,
    fee: u64,
    websocket_url: &str,
) -> Result<(PreparedSweep, String, u64), String> {
    let plan = BeneficiarySweepPlan::new(redeem_script_hex, locktime_daa)?;
    let result = sweep::build_automatic(
        websocket_url,
        plan.spec(
            covenant_address,
            destination_address,
            fee,
            SweepSourceKind::Automatic,
        ),
    )
    .await?;
    Ok((result.0, result.1, plan.displayed_locktime))
}

pub(crate) fn build_beneficiary_selected(
    covenant_address: &str,
    destination_address: &str,
    redeem_script_hex: &str,
    locktime_daa: u64,
    utxos_json: &str,
    fee: u64,
) -> Result<(PreparedSweep, String, u64), String> {
    let plan = BeneficiarySweepPlan::new(redeem_script_hex, locktime_daa)?;
    let result = sweep::build_selected(
        utxos_json,
        plan.spec(
            covenant_address,
            destination_address,
            fee,
            SweepSourceKind::Selected,
        ),
    )?;
    Ok((result.0, result.1, plan.displayed_locktime))
}
