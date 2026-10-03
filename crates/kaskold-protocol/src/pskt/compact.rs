use blake2b_simd::Params;
use k256::schnorr::{Signature as K256Signature, VerifyingKey};
use serde_json::{Map, Value};

use super::{relay_fields::find_pubkey_position, wire};
use crate::{wire::kspt as kspt_wire, Network};
mod merge;
mod sighash;
mod verify;
pub(crate) use merge::*;
pub(crate) use sighash::*;
pub(crate) use verify::*;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Signature {
    pub(crate) position: u8,
    pub(crate) sighash: u8,
    pub(crate) bytes: [u8; 64],
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(clippy::enum_variant_names)] // every specialized route is a covenant claim
pub(crate) enum SpecializedRoute {
    PrivateSwapClaim,
    OracleV1Claim,
    CommitRevealClaim,
    MerkleClaim,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SpecializedWitness {
    pub(crate) route: SpecializedRoute,
    pub(crate) signature_script: Vec<u8>,
    pub(crate) supplied_mask: u16,
    pub(crate) supplied_true_mask: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Input {
    pub(crate) tx_id: [u8; 32],
    pub(crate) index: u32,
    pub(crate) amount: u64,
    pub(crate) sequence: u64,
    pub(crate) sig_op_count: u8,
    pub(crate) script_version: u16,
    pub(crate) script: Vec<u8>,
    /// Previous UTXO carries a covenant id; needed for exact KIP-9 storage mass.
    /// Compact KSPT v1 does not encode this metadata, so decoded KSPT defaults
    /// fail-conservatively to false while standard PSKT preserves it.
    pub(crate) has_covenant_id: bool,
    pub(crate) signatures: Vec<Signature>,
    pub(crate) redeem: Vec<u8>,
    pub(crate) derivation: Option<(u8, u32)>,
    pub(crate) ms45: Option<(u32, u32, u32)>,
    pub(crate) covenant_execution: Option<(u16, u16)>,
    /// Host-only typed witness plan for specialized PSKT covenant routes. This
    /// is deliberately not part of compact KSPT v1; raw KSPT therefore supports
    /// only the generic covenant witness grammar carried by covenantExecution.
    pub(crate) specialized_witness: Option<SpecializedWitness>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Output {
    pub(crate) amount: u64,
    pub(crate) script_version: u16,
    pub(crate) script: Vec<u8>,
    pub(crate) derivation: Option<(u8, u32)>,
    pub(crate) ms45: Option<(u32, u32, u32)>,
    pub(crate) covenant: Option<(u16, [u8; 32])>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Transaction {
    pub(crate) flags: u8,
    pub(crate) version: u16,
    pub(crate) locktime: u64,
    pub(crate) subnetwork: [u8; 20],
    pub(crate) gas: u64,
    pub(crate) payload: Vec<u8>,
    pub(crate) network: u8,
    pub(crate) inputs: Vec<Input>,
    pub(crate) outputs: Vec<Output>,
    pub(crate) stealth: Option<[u8; 32]>,
}

impl kspt_wire::EncodeSource for Transaction {
    fn global(&self) -> kspt_wire::Global<'_> {
        kspt_wire::Global {
            flags: self.flags,
            version: self.version,
            input_count: self.inputs.len() as u32,
            output_count: self.outputs.len() as u8,
            locktime: self.locktime,
            subnetwork_id: self.subnetwork,
            gas: self.gas,
            payload: &self.payload,
        }
    }

    fn input(&self, index: usize) -> kspt_wire::Input<'_> {
        let input = &self.inputs[index];
        kspt_wire::Input {
            previous_tx_id: input.tx_id,
            previous_index: input.index,
            amount: input.amount,
            sequence: input.sequence,
            sig_op_count: input.sig_op_count,
            script_version: input.script_version,
            script: &input.script,
        }
    }

    fn signature_count(&self, input: usize) -> usize {
        self.inputs[input].signatures.len()
    }
    fn signature(&self, input: usize, slot: usize) -> kspt_wire::Signature {
        let signature = &self.inputs[input].signatures[slot];
        kspt_wire::Signature {
            position: signature.position,
            sighash: signature.sighash,
            bytes: signature.bytes,
        }
    }
    fn redeem(&self, input: usize) -> &[u8] {
        &self.inputs[input].redeem
    }
    fn output(&self, index: usize) -> kspt_wire::Output<'_> {
        let output = &self.outputs[index];
        kspt_wire::Output {
            amount: output.amount,
            script_version: output.script_version,
            script: &output.script,
        }
    }
    fn network(&self) -> u8 {
        self.network
    }
    fn stealth(&self) -> Option<[u8; 32]> {
        self.stealth
    }
    fn input_derivation(&self, index: usize) -> Option<kspt_wire::Derivation> {
        self.inputs[index]
            .derivation
            .map(|(branch, index)| kspt_wire::Derivation { branch, index })
    }
    fn output_derivation(&self, index: usize) -> Option<kspt_wire::Derivation> {
        self.outputs[index]
            .derivation
            .map(|(branch, index)| kspt_wire::Derivation { branch, index })
    }
    fn input_ms45(&self, index: usize) -> Option<kspt_wire::Ms45Derivation> {
        self.inputs[index]
            .ms45
            .map(|(cosigner, chain, index)| kspt_wire::Ms45Derivation {
                cosigner,
                chain,
                index,
            })
    }
    fn covenant_execution(&self, index: usize) -> Option<kspt_wire::CovenantExecution> {
        self.inputs[index]
            .covenant_execution
            .map(
                |(supplied_mask, supplied_true_mask)| kspt_wire::CovenantExecution {
                    supplied_mask,
                    supplied_true_mask,
                },
            )
    }
    fn output_ms45(&self, index: usize) -> Option<kspt_wire::Ms45Derivation> {
        self.outputs[index]
            .ms45
            .map(|(cosigner, chain, index)| kspt_wire::Ms45Derivation {
                cosigner,
                chain,
                index,
            })
    }
    fn covenant(&self, index: usize) -> Option<kspt_wire::Covenant> {
        self.outputs[index]
            .covenant
            .map(|(authorizing_input, id)| kspt_wire::Covenant {
                authorizing_input,
                id,
            })
    }
}

mod parser;

pub(super) fn parse(data: &[u8]) -> Result<Transaction, String> {
    parser::parse(data)
}

#[cfg(test)]
pub(crate) fn test_sighash_all_for_pskt(
    pskt_hex: &str,
    network: Network,
    input_index: usize,
) -> Result<[u8; 32], String> {
    let compact = super::relay::encode_pskt(pskt_hex, network)?;
    let transaction = parse(&compact)?;
    sighash_all(&transaction, input_index)
}
