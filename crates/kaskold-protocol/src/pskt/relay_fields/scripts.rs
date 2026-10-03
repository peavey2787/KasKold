//! Redeem-script and MuSig-45 path parsing for relayed PSKT inputs.

use super::{compressed_xonly, Value};

pub(crate) fn parse_multisig_redeem(script: &[u8]) -> Option<(u8, u8)> {
    let threshold = script.first().copied().and_then(decode_small_int)?;
    let declared = multisig_declared_count(script)?;
    let count = multisig_key_count(script)?;
    (script.last() == Some(&0xae)
        && declared == count
        && threshold <= declared
        && declared <= crate::SIGNER_CAPABILITIES.max_multisig_keys)
        .then_some((threshold, declared))
}

pub(crate) fn multisig_declared_count(script: &[u8]) -> Option<u8> {
    let position = script.len().checked_sub(2)?;
    script.get(position).copied().and_then(decode_small_int)
}

pub(crate) fn multisig_key_count(script: &[u8]) -> Option<u8> {
    let end = script.len().checked_sub(2)?;
    let region = script.get(1..end)?;
    if !region.len().is_multiple_of(33)
        || !region
            .as_chunks::<33>()
            .0
            .iter()
            .all(|chunk| chunk[0] == 0x20)
    {
        return None;
    }
    u8::try_from(region.len() / 33).ok()
}

pub(crate) fn decode_small_int(opcode: u8) -> Option<u8> {
    (0x51..=0x60).contains(&opcode).then(|| opcode - 0x50)
}

pub(crate) fn find_pubkey_position(redeem: &[u8], public_key_hex: &str) -> Option<u8> {
    let xonly = compressed_xonly(public_key_hex)?;
    scan_pubkey_position(redeem, &xonly)
}

pub(crate) fn scan_pubkey_position(redeem: &[u8], xonly: &[u8]) -> Option<u8> {
    let mut position = 1usize;
    let mut index = 0u8;
    while position + 33 < redeem.len() {
        if redeem[position] != 0x20 {
            return None;
        }
        if redeem.get(position + 1..position + 33)? == xonly {
            return Some(index);
        }
        position += 33;
        index = index.saturating_add(1);
    }
    None
}

pub(crate) fn parse_ms45(value: &Value) -> Result<Option<(u32, u32, u32)>, String> {
    if value.is_null() {
        return Ok(None);
    }
    let map = value
        .as_object()
        .ok_or_else(|| "bip32Derivations must be an object or null".to_string())?;
    let mut parsed = None;
    for entry in map.values() {
        if let Some(candidate) = parse_ms45_entry(entry)? {
            merge_ms45_candidate(&mut parsed, candidate)?;
        }
    }
    Ok(parsed)
}

pub(crate) fn parse_ms45_entry(entry: &Value) -> Result<Option<(u32, u32, u32)>, String> {
    let path = entry
        .get("derivationPath")
        .and_then(Value::as_str)
        .ok_or_else(|| "bip32Derivations entry is missing derivationPath".to_string())?;
    let Some(tail) = path.strip_prefix("m/45'/111111'/0'/") else {
        return Ok(None);
    };
    let mut parts = tail.split('/');
    let cosigner = parse_ms45_part(&mut parts, "cosigner", None)?;
    let chain = parse_ms45_part(&mut parts, "chain", Some(1))?;
    let index = parse_ms45_part(&mut parts, "index", None)?;
    if parts.next().is_some() {
        return Err("MS45 path has trailing components".to_string());
    }
    Ok(Some((cosigner, chain, index)))
}

pub(crate) fn parse_ms45_part<'a, I>(
    parts: &mut I,
    label: &str,
    maximum: Option<u32>,
) -> Result<u32, String>
where
    I: Iterator<Item = &'a str>,
{
    let value = parts
        .next()
        .ok_or_else(|| format!("MS45 path missing {label}"))?;
    let parsed = parse_soft(value).ok_or_else(|| format!("MS45 {label} must be non-hardened"))?;
    if maximum.is_some_and(|limit| parsed > limit) {
        return Err("MS45 chain must be 0 or 1".to_string());
    }
    Ok(parsed)
}

pub(crate) fn merge_ms45_candidate(
    parsed: &mut Option<(u32, u32, u32)>,
    candidate: (u32, u32, u32),
) -> Result<(), String> {
    match parsed {
        Some(existing) if *existing != candidate => {
            Err("bip32Derivations contain conflicting MS45 paths".to_string())
        }
        Some(_) => Ok(()),
        None => {
            *parsed = Some(candidate);
            Ok(())
        }
    }
}

pub(crate) fn parse_soft(value: &str) -> Option<u32> {
    if value.ends_with('\'') {
        return None;
    }
    let parsed = value.parse::<u32>().ok()?;
    (parsed < 0x8000_0000).then_some(parsed)
}
