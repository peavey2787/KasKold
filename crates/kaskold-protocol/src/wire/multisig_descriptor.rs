//! KasKold binding of Kaspa Portal's canonical multisig descriptor grammar.
//!
//! Portal owns the `multi(...)`, `multi_hd(...)` and `multi_hd45(...)` syntax,
//! canonical kpub decoding, duplicate detection and HD45 cosigner ordering.
//! KasKold fixes the participant capacity to what its signer can co-sign, so
//! hardware, hot-wallet and Companion parsing share one bounded model.

use kaspa_portal::wallet::multisig::grammar;

pub use grammar::{MultisigDescriptorError, MultisigDescriptorKind};

/// Most cosigners a KasKold signer co-signs for.
pub const MAX_DESCRIPTOR_PARTICIPANTS: usize =
    crate::SIGNER_CAPABILITIES.max_multisig_keys as usize;

pub type ParsedMultisigDescriptor = grammar::ParsedMultisigDescriptor<MAX_DESCRIPTOR_PARTICIPANTS>;

/// Parse a descriptor the KasKold signer can co-sign.
pub fn parse_multisig_descriptor(
    data: &[u8],
) -> Result<ParsedMultisigDescriptor, MultisigDescriptorError> {
    grammar::parse_multisig_descriptor(data)
}
