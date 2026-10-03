//! Partial-signature collection for relayed PSKT inputs.

use super::{find_pubkey_position, parse_multisig_redeem, InputFields, Value};

pub(crate) struct Signature {
    pub position: u8,
    pub bytes: [u8; 64],
}

pub(crate) fn collect_signatures(fields: &InputFields) -> Result<Vec<Signature>, String> {
    if fields.partial_signatures.is_empty() {
        return Ok(Vec::new());
    }
    ensure_signature_capacity(fields)?;
    match fields.redeem_script.as_deref() {
        Some(redeem) if parse_multisig_redeem(redeem).is_some() => {
            collect_multisig_signatures(fields, redeem)
        }
        Some(redeem) => collect_covenant_signatures(fields, redeem),
        None => collect_p2pk_signature(fields),
    }
}

pub(crate) fn ensure_signature_capacity(fields: &InputFields) -> Result<(), String> {
    if fields.partial_signatures.len()
        > usize::from(crate::SIGNER_CAPABILITIES.max_signatures_per_input)
    {
        return Err("too many partial signatures for signer capabilities".to_string());
    }
    Ok(())
}

pub(crate) fn collect_multisig_signatures(
    fields: &InputFields,
    redeem: &[u8],
) -> Result<Vec<Signature>, String> {
    let mut signatures = Vec::with_capacity(fields.partial_signatures.len());
    for (public_key, value) in &fields.partial_signatures {
        let position = find_pubkey_position(redeem, public_key).ok_or_else(|| {
            format!("partial-signature pubkey is not in redeem script: {public_key}")
        })?;
        signatures.push(Signature {
            position,
            bytes: decode_signature(value)?,
        });
    }
    sort_unique_signatures(signatures, "duplicate partial-signature position")
}

pub(crate) fn collect_covenant_signatures(
    fields: &InputFields,
    redeem: &[u8],
) -> Result<Vec<Signature>, String> {
    let resolution = shared_signer::covenant_branch::resolve_covenant_branches(redeem)
        .map_err(|error| format!("invalid covenant branch structure: {error:?}"))?;
    let mut signatures = Vec::with_capacity(fields.partial_signatures.len());
    for (public_key, value) in &fields.partial_signatures {
        let key = compressed_xonly(public_key).ok_or_else(|| {
            format!("invalid covenant partial-signature public key: {public_key}")
        })?;
        let position = covenant_signature_position(fields, &resolution, public_key, &key)?;
        signatures.push(Signature {
            position,
            bytes: decode_signature(value)?,
        });
    }
    sort_unique_signatures(signatures, "duplicate covenant partial-signature position")
}

pub(crate) fn covenant_signature_position(
    fields: &InputFields,
    resolution: &shared_signer::covenant_branch::BranchResolution,
    public_key: &str,
    key: &[u8; 32],
) -> Result<u8, String> {
    let result = match fields.covenant_execution {
        Some((mask, truth)) => resolution.position_for_key_with_selectors(key, mask, truth),
        None => resolution.unique_position_for_key(key),
    };
    result.map_err(|error| match error {
        shared_signer::covenant_branch::BranchResolveError::KeyNotFound => {
            format!("covenant partial-signature pubkey is not branch-bound: {public_key}")
        }
        shared_signer::covenant_branch::BranchResolveError::AmbiguousKey => format!(
            "covenant partial-signature pubkey is branch-ambiguous without execution selectors: {public_key}"
        ),
        other => format!("invalid covenant branch binding: {other:?}"),
    })
}

pub(crate) fn sort_unique_signatures(
    mut signatures: Vec<Signature>,
    duplicate_error: &str,
) -> Result<Vec<Signature>, String> {
    signatures.sort_by_key(|entry| entry.position);
    if signatures
        .windows(2)
        .any(|pair| pair[0].position == pair[1].position)
    {
        return Err(duplicate_error.to_string());
    }
    Ok(signatures)
}

pub(crate) fn collect_p2pk_signature(fields: &InputFields) -> Result<Vec<Signature>, String> {
    if fields.partial_signatures.len() != 1 {
        return Err("standard P2PK input must contain exactly one partial signature".to_string());
    }
    validate_p2pk_script(fields)?;
    let (public_key, value) = fields.partial_signatures.iter().next().ok_or_else(|| {
        "standard P2PK input is missing its required partial signature".to_string()
    })?;
    let key = compressed_xonly(public_key)
        .ok_or_else(|| "P2PK partial-signature public key is malformed".to_string())?;
    if fields.script_public_key.get(1..33) != Some(key.as_slice()) {
        return Err("P2PK partial-signature public key does not match scriptPublicKey".to_string());
    }
    Ok(vec![Signature {
        position: 0,
        bytes: decode_signature(value)?,
    }])
}

pub(crate) fn validate_p2pk_script(fields: &InputFields) -> Result<(), String> {
    let valid = fields.script_public_key.len() == 34
        && fields.script_public_key.first() == Some(&0x20)
        && fields.script_public_key.get(33) == Some(&0xac);
    if !valid {
        return Err(
            "partial signature is present for a non-P2PK input without redeemScript".to_string(),
        );
    }
    Ok(())
}

pub(crate) fn compressed_xonly(public_key_hex: &str) -> Option<[u8; 32]> {
    let bytes = public_key_hex.as_bytes();
    if bytes.len() != 66
        || !bytes.iter().all(u8::is_ascii_hexdigit)
        || !(bytes.starts_with(b"02") || bytes.starts_with(b"03"))
    {
        return None;
    }
    let decoded = super::super::wire::decode_lower_hex(
        core::str::from_utf8(&bytes[2..]).ok()?,
        "compressed public key",
    )
    .ok()?;
    decoded.as_slice().try_into().ok()
}

pub(crate) fn decode_signature(value: &Value) -> Result<[u8; 64], String> {
    let object = value
        .as_object()
        .ok_or_else(|| "partial signature value must be an object".to_string())?;
    if object.len() != 1 || !object.contains_key("schnorr") {
        return Err("partial signature object must contain only the schnorr field".to_string());
    }
    let text = object
        .get("schnorr")
        .and_then(Value::as_str)
        .ok_or_else(|| "partial sig missing schnorr variant".to_string())?;
    let bytes = super::super::wire::decode_lower_hex(text, "signature")?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| "Schnorr signature must be 64 bytes".to_string())
}
