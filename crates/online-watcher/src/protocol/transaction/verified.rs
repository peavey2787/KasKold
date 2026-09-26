//! Consensus materialization from the exact typed transaction that passed
//! cryptographic authorization.
//!
//! This module is intentionally source-format agnostic: it never sees PSKT JSON
//! or KSPT bytes. That prevents a second parser from interpreting authorization
//! inputs differently after signatures and covenant execution have been verified.

use kaskold_protocol::compat::VerifiedTransaction;

use super::consensus::{ConsensusInput, ConsensusOutput, ConsensusTransaction, InputEncoding};

pub(crate) fn materialize_verified_transaction(
    verified: VerifiedTransaction,
    input_encoding: InputEncoding,
) -> Result<ConsensusTransaction, String> {
    let input_cells = verified
        .inputs()
        .iter()
        .map(|input| {
            (
                input.amount(),
                crate::transaction_builder::planning::amounts::utxo_plurality(
                    input.script_public_key().len(),
                    input.has_covenant_id(),
                ),
            )
        })
        .collect::<Vec<_>>();
    let output_cells = verified
        .outputs()
        .iter()
        .map(|output| {
            (
                output.amount(),
                crate::transaction_builder::planning::amounts::utxo_plurality(
                    output.script_public_key().len(),
                    output.covenant().is_some(),
                ),
            )
        })
        .collect::<Vec<_>>();
    let storage_mass = crate::transaction_builder::planning::amounts::storage_mass_estimate(
        &input_cells,
        &output_cells,
    )?;

    let mut inputs = Vec::new();
    inputs
        .try_reserve(verified.inputs().len())
        .map_err(|_| "verified input count exceeds available memory".to_string())?;
    for input in verified.inputs() {
        inputs.push(ConsensusInput {
            prev_tx_id: *input.previous_tx_id(),
            prev_index: input.previous_index(),
            sig_script: input.witness().materialize_signature_script()?,
            sequence: input.sequence(),
            sig_op_count: input.sig_op_count(),
        });
    }

    let mut outputs = Vec::new();
    outputs
        .try_reserve(verified.outputs().len())
        .map_err(|_| "verified output count exceeds available memory".to_string())?;
    for output in verified.outputs() {
        outputs.push(ConsensusOutput {
            value: output.amount(),
            spk_version: output.script_version(),
            spk_script: output.script_public_key().to_vec(),
            covenant: output.covenant(),
        });
    }

    Ok(ConsensusTransaction {
        tx_version: verified.version(),
        input_encoding,
        inputs,
        outputs,
        locktime: verified.locktime(),
        subnetwork_id: *verified.subnetwork_id(),
        gas: verified.gas(),
        payload: verified.payload().to_vec(),
        storage_mass,
    })
}
