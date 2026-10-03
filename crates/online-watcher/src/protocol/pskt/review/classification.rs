// KasKold Companion Web — organized PSKT subsystem
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

pub(crate) fn parse_spk_hex(s: &str) -> Result<(u16, Vec<u8>), String> {
    let bytes = s.as_bytes();
    if bytes.len() < 4 || !bytes.iter().all(u8::is_ascii_hexdigit) {
        return Err(format!(
            "scriptPublicKey must be ASCII hex and at least 4 bytes, got {}",
            bytes.len()
        ));
    }
    // The input is validated ASCII hex, so one decode yields the 2-byte BE
    // version followed by the script; only an odd length can still fail.
    let decoded = hex::decode(s).map_err(|e| format!("bad script hex: {}", e))?;
    let version = u16::from_be_bytes([decoded[0], decoded[1]]);
    Ok((version, decoded[2..].to_vec()))
}

pub(crate) fn classify_input_script(
    spk: &[u8],
    redeem: Option<&[u8]>,
) -> (String, Option<u8>, Option<u8>) {
    if is_p2sh_script(spk) {
        return classify_p2sh_redeem(redeem);
    }
    if is_p2pk_script(spk) {
        return ("p2pk".into(), None, None);
    }
    ("unknown".into(), None, None)
}

fn is_p2sh_script(spk: &[u8]) -> bool {
    spk.len() == 35 && spk[0] == 0xAA && spk[1] == 0x20 && spk[34] == 0x87
}

fn is_p2pk_script(spk: &[u8]) -> bool {
    spk.len() == 34 && spk[0] == 0x20 && spk[33] == 0xAC
}

fn classify_p2sh_redeem(redeem: Option<&[u8]>) -> (String, Option<u8>, Option<u8>) {
    let Some(script) = redeem else {
        return ("p2sh".into(), None, None);
    };
    if let Some((m, n)) = parse_multisig_redeem(script) {
        return ("p2sh-multisig".into(), Some(m), Some(n));
    }
    if script.first() == Some(&0x63) {
        return ("p2sh-covenant".into(), None, None);
    }
    ("p2sh".into(), None, None)
}

pub(crate) fn classify_output_script(spk: &[u8], network_prefix: &str) -> (String, Option<String>) {
    // P2SH
    if spk.len() == 35 && spk[0] == 0xAA && spk[1] == 0x20 && spk[34] == 0x87 {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&spk[2..34]);
        return (
            "p2sh".into(),
            Some(crate::account::address::encode_p2sh_address(
                &hash,
                network_prefix,
            )),
        );
    }
    // P2PK
    if spk.len() == 34 && spk[0] == 0x20 && spk[33] == 0xAC {
        let mut pk = [0u8; 32];
        pk.copy_from_slice(&spk[1..33]);
        return (
            "p2pk".into(),
            Some(crate::account::address::encode_p2pk_address(
                &pk,
                network_prefix,
            )),
        );
    }
    ("unknown".into(), None)
}

pub(crate) fn parse_multisig_redeem(rs: &[u8]) -> Option<(u8, u8)> {
    let m = multisig_threshold(rs)?;
    let (position, counted) = count_multisig_pubkeys(rs)?;
    validate_multisig_tail(rs, position, counted, m)
}

fn multisig_threshold(script: &[u8]) -> Option<u8> {
    if script.last() != Some(&0xAE) {
        return None;
    }
    decode_small_int(*script.first()?)
}

fn decode_small_int(opcode: u8) -> Option<u8> {
    match opcode {
        0x51..=0x60 => Some(opcode - 0x50),
        _ => None,
    }
}

fn count_multisig_pubkeys(script: &[u8]) -> Option<(usize, u8)> {
    let mut position = 1usize;
    let mut count = 0u8;
    while position < script.len().saturating_sub(2) {
        if script.get(position) != Some(&0x20) {
            return None;
        }
        position = position.checked_add(33)?;
        count = count.saturating_add(1);
    }
    Some((position, count))
}

fn validate_multisig_tail(script: &[u8], position: usize, counted: u8, m: u8) -> Option<(u8, u8)> {
    if position.checked_add(2)? != script.len() {
        return None;
    }
    let n = decode_small_int(*script.get(position)?)?;
    (counted == n && m <= n && n <= kaskold_protocol::SIGNER_CAPABILITIES.max_multisig_keys)
        .then_some((m, n))
}

pub(crate) fn find_pubkey_position_in_redeem(rs: &[u8], pk_hex_66: &str) -> Option<u8> {
    let bytes = pk_hex_66.as_bytes();
    if bytes.len() != 66
        || !bytes.iter().all(u8::is_ascii_hexdigit)
        || !(bytes.starts_with(b"02") || bytes.starts_with(b"03"))
    {
        return None;
    }
    // Strip SEC1 prefix (02/03) to get the 32-byte x-only key.
    let xonly = hex::decode(&bytes[2..]).ok()?;
    // Walk redeem: OP_M, then repeated [OP_DATA_32, <32>].
    let mut pos = 1usize;
    let mut idx: u8 = 0;
    while pos + 33 < rs.len() {
        if rs[pos] != 0x20 {
            return None;
        }
        if &rs[pos + 1..pos + 33] == xonly.as_slice() {
            return Some(idx);
        }
        pos += 33;
        idx = idx.saturating_add(1);
    }
    None
}
