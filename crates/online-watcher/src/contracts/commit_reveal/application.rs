//! Browser-neutral commit-reveal covenant construction.

pub(crate) fn build_json(
    owner_pubkey_hex: &str,
    committed_hash_hex: &str,
    locktime_daa: u64,
    network: &str,
) -> Result<String, String> {
    let owner_public_key = crate::serialization::input::decode_pubkey32(owner_pubkey_hex)?;
    let committed_hash =
        crate::serialization::input::decode_named_32(committed_hash_hex, "committed hash")?;
    let script = crate::contracts::commit_reveal::script::build_commit_reveal_script(
        &owner_public_key,
        &committed_hash,
        locktime_daa,
    );
    let address = crate::protocol::script::p2sh::script_to_address(
        &script,
        crate::account::address::network_prefix(network)?,
    )?;
    Ok(serde_json::json!({
        "address": address,
        "redeem_script_hex": hex::encode(&script),
        "committed_hash": committed_hash_hex,
        "locktime_daa": locktime_daa.to_string(),
    })
    .to_string())
}

/// JSON `{preimage_hex, committed_hash, computed_hash, matches}` for a
/// commit-reveal reveal signature script, read and hashed by Kaspa Portal.
pub(crate) fn read_reveal_json(signature_script_hex: &str) -> Result<String, String> {
    let script = hex::decode(signature_script_hex).map_err(|error| error.to_string())?;
    let reveal = kaspa_portal::contract::commit_reveal::read_commit_reveal_reveal(&script)
        .ok_or("not a commit-reveal reveal signature script")?;
    Ok(serde_json::json!({
        "preimage_hex": hex::encode(&reveal.preimage),
        "committed_hash": hex::encode(reveal.fields.committed_hash),
        "computed_hash": hex::encode(kaspa_portal::primitives::hash::blake2b_256(&reveal.preimage)),
        "matches": reveal.matches(),
    })
    .to_string())
}
