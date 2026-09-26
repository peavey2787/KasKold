// KasKold Companion Web — verified consensus transaction finalization
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use crate::protocol::transaction::consensus::{ConsensusInput, ConsensusOutput, InputEncoding};

#[derive(Debug)]
pub(crate) struct FinalizedConsensusTransaction {
    pub(crate) tx_version: u16,
    pub(crate) inputs: Vec<ConsensusInput>,
    pub(crate) outputs: Vec<ConsensusOutput>,
    pub(crate) locktime: u64,
    pub(crate) subnetwork_id: [u8; 20],
    pub(crate) gas: u64,
    pub(crate) payload: Vec<u8>,
    pub(crate) storage_mass: u64,
}

impl FinalizedConsensusTransaction {
    pub(crate) fn into_consensus_transaction(
        self,
    ) -> crate::protocol::transaction::consensus::ConsensusTransaction {
        crate::protocol::transaction::consensus::ConsensusTransaction {
            tx_version: self.tx_version,
            input_encoding: InputEncoding::Budgeted,
            inputs: self.inputs,
            outputs: self.outputs,
            locktime: self.locktime,
            subnetwork_id: self.subnetwork_id,
            gas: self.gas,
            payload: self.payload,
            storage_mass: self.storage_mass,
        }
    }
}

/// Authorize standard PSKT exactly once and materialize only the resulting typed
/// transaction/witness plan. The source JSON is never parsed again after
/// cryptographic authorization succeeds.
pub(crate) fn finalize_to_consensus(
    wire_hex: &str,
) -> Result<FinalizedConsensusTransaction, String> {
    let verified = kaskold_protocol::compat::verify_complete_pskt(
        wire_hex,
        kaskold_protocol::Network::Mainnet,
    )
    .map_err(|error| format!("PSKT cryptographic verification failed: {error}"))?;
    let transaction = crate::protocol::transaction::verified::materialize_verified_transaction(
        verified,
        InputEncoding::Budgeted,
    )?;
    Ok(FinalizedConsensusTransaction {
        tx_version: transaction.tx_version,
        inputs: transaction.inputs,
        outputs: transaction.outputs,
        locktime: transaction.locktime,
        subnetwork_id: transaction.subnetwork_id,
        gas: transaction.gas,
        payload: transaction.payload,
        storage_mass: transaction.storage_mass,
    })
}
