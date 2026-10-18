//! Companion compatibility facade over protocol-owned Kaspa address primitives.

#[cfg(test)]
pub(super) use kaskold_protocol::encode_address;
pub use kaskold_protocol::encode_p2pk_address;

pub fn decode_address(addr: &str) -> Result<(u8, [u8; 32]), String> {
    kaskold_protocol::compat::decode_address(addr)
}

pub fn address_to_script_pubkey(addr: &str) -> Result<Vec<u8>, String> {
    kaskold_protocol::compat::address_to_script_pubkey(addr)
}

/// Map a Kaspa network label to the canonical address prefix for internal Companion callers.
/// Unrecognized labels are rejected rather than defaulting to mainnet.
pub(crate) fn network_prefix(network: &str) -> Result<&'static str, String> {
    use kaspa_portal::primitives::address::KaspaNetwork;
    KaspaNetwork::from_name(network)
        .and_then(KaspaNetwork::hrp)
        .ok_or_else(|| format!("unsupported Kaspa network: {network:?}"))
}
