//! Canonical compact KSPT v1 wire grammar, owned by Kaspa Portal.
//!
//! KasKold contributes only the signer's device capacity; every consumer
//! passes [`SIGNER_LIMITS`] explicitly when encoding or decoding.

pub use kaspa_portal::transaction::interchange::kspt::wire::*;

/// Most inputs the signer accepts in one transaction.
pub const MAX_INPUTS: u32 = 32;
/// Largest transaction payload the signer accepts.
pub const MAX_PAYLOAD_SIZE: usize = 768;
/// The signer's device capacity, expressed as grammar limits.
pub const SIGNER_LIMITS: Limits = Limits::new(MAX_INPUTS, MAX_OUTPUTS, MAX_PAYLOAD_SIZE);
