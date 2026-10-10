//! KasKold multisig policy over Kaspa Portal's multisig wallet model.

/// Reject descriptors the KasKold signer cannot co-sign.
pub(crate) fn require_signer_descriptor(descriptor_text: &str) -> Result<(), String> {
    kaskold_protocol::wire::multisig_descriptor::parse_multisig_descriptor(
        descriptor_text.as_bytes(),
    )
    .map(|_| ())
    .map_err(|error| error.message().to_string())
}

#[cfg(test)]
mod unit_tests;
