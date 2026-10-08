//! KasKold binding of Kaspa Portal's verify-once PSKT/KSPT pipeline.
//!
//! Portal owns parsing, authorization and finalization; KasKold contributes
//! only its signer capacity ([`SIGNER_LIMITS`]) and its network/branch types.

use kaspa_portal::{
    primitives::address::KaspaNetwork,
    transaction::interchange::{kspt::wire::Derivation, pskt::pipeline},
};

use crate::{wire::kspt::SIGNER_LIMITS, AddressBranch, Network};

fn portal_network(network: Network) -> Result<KaspaNetwork, String> {
    KaspaNetwork::from_wire(network.kspt_code())
        .ok_or_else(|| format!("network {network} has no KSPT code"))
}

const fn derivation(branch: AddressBranch, index: u32) -> Derivation {
    Derivation {
        branch: branch.code(),
        index,
    }
}

pub(crate) fn encode_pskt(pskt_hex: &str, network: Network) -> Result<Vec<u8>, String> {
    pipeline::encode_pskt(pskt_hex, portal_network(network)?, SIGNER_LIMITS)
}

pub(crate) fn merge_signed_kspt(
    original_pskt_hex: &str,
    signed_kspt: &[u8],
    network: Network,
) -> Result<String, String> {
    pipeline::merge_signed_kspt(
        original_pskt_hex,
        signed_kspt,
        portal_network(network)?,
        SIGNER_LIMITS,
    )
}

pub(crate) fn is_complete(pskt_hex: &str, network: Network) -> Result<bool, String> {
    pipeline::is_complete(pskt_hex, portal_network(network)?, SIGNER_LIMITS)
}

pub(crate) fn verified_signature_counts(
    pskt_hex: &str,
    network: Network,
) -> Result<Vec<u8>, String> {
    pipeline::verified_signature_counts(pskt_hex, portal_network(network)?, SIGNER_LIMITS)
}

pub(crate) fn finalize_json(pskt_hex: &str) -> Result<String, String> {
    pipeline::finalize_json(pskt_hex, SIGNER_LIMITS)
}

pub(crate) fn attach_input_derivation(
    pskt_hex: &str,
    input_index: usize,
    branch: AddressBranch,
    index: u32,
) -> Result<String, String> {
    pipeline::attach_input_derivation(pskt_hex, input_index, derivation(branch, index))
}

pub(crate) fn attach_output_derivation(
    pskt_hex: &str,
    output_index: usize,
    branch: AddressBranch,
    index: u32,
) -> Result<String, String> {
    pipeline::attach_output_derivation(pskt_hex, output_index, derivation(branch, index))
}

#[cfg(feature = "companion-compat")]
pub(crate) fn verify_complete_kspt(data: &[u8]) -> Result<pipeline::VerifiedTransaction, String> {
    pipeline::verify_complete_kspt(data, SIGNER_LIMITS)
}

#[cfg(feature = "companion-compat")]
pub(crate) fn verify_complete_pskt(
    pskt_hex: &str,
    network: Network,
) -> Result<pipeline::VerifiedTransaction, String> {
    pipeline::verify_complete_pskt(pskt_hex, portal_network(network)?, SIGNER_LIMITS)
}
