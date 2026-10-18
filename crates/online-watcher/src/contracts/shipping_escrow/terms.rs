//! Shipment-escrow terms read from a redeem script by Kaspa Portal.

use crate::account::address::{encode_p2pk_address, network_prefix};

pub(crate) fn terms_json(redeem_script_hex: &str, network: &str) -> Result<String, String> {
    let script = hex::decode(redeem_script_hex).map_err(|error| error.to_string())?;
    let terms = crate::contracts::shipping_escrow::script::ship_escrow_terms(&script)
        .ok_or("unrecognized ship-escrow redeem")?;
    let tranches = terms.tranches()?;
    let prefix = network_prefix(network)?;
    let address = |key: &[u8; 32]| encode_p2pk_address(key, prefix);
    Ok(serde_json::json!({
        "total_sompi": tranches.total.to_string(),
        "rem_sompi": tranches.remainder.to_string(),
        "fee_sompi": terms.fee_sompi.to_string(),
        "cltv1_deadline": terms.cltv1_deadline.to_string(),
        "cltv2_deadline": terms.cltv2_deadline.to_string(),
        "seller_addr": address(&terms.seller_pubkey),
        "deliverer_addr": address(&terms.deliverer_pubkey),
        "buyer_addr": address(&terms.buyer_pubkey),
    })
    .to_string())
}
