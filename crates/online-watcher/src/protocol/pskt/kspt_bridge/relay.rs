// Companion delegates the reusable PSKT -> KSPT relay to the official protocol crate.

pub fn relay_pskb_as_kspt_hex_for_network(wire_hex: &str, network: &str) -> Result<String, String> {
    let network = kaskold_protocol::Network::parse(network).map_err(|error| error.to_string())?;
    kaskold_protocol::encode_pskt_hex(wire_hex, network).map_err(|error| error.to_string())
}
