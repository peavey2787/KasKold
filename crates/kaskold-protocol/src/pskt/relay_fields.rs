use serde_json::{Map, Value};

use super::wire::{parse_exact_u64, parse_spk};

pub(crate) struct InputFields {
    pub previous_tx_id: [u8; 32],
    pub previous_index: u32,
    pub amount: u64,
    pub sequence: u64,
    pub sig_op_count: u8,
    pub script_version: u16,
    pub script_public_key: Vec<u8>,
    pub has_covenant_id: bool,
    pub redeem_script: Option<Vec<u8>>,
    pub partial_signatures: Map<String, Value>,
    pub covenant_execution: Option<(u16, u16)>,
}

impl InputFields {
    pub(crate) fn parse(value: &Value) -> Result<Self, String> {
        let object = value
            .as_object()
            .ok_or_else(|| "input not object".to_string())?;
        let (amount, script_version, script_public_key, has_covenant_id) =
            parse_utxo_fields(object)?;
        let (previous_tx_id, previous_index) = parse_outpoint(object)?;
        let sequence = parse_sequence(object)?;
        let sig_op_count = parse_sig_op_count(object)?;
        let redeem_script = parse_redeem_script(object)?;
        let covenant_execution = parse_covenant_execution(object)?;
        validate_sighash(object)?;
        let partial_signatures = parse_partial_signatures(object)?;
        Ok(Self {
            previous_tx_id,
            previous_index,
            amount,
            sequence,
            sig_op_count,
            script_version,
            script_public_key,
            has_covenant_id,
            redeem_script,
            partial_signatures,
            covenant_execution,
        })
    }
}

fn validate_sighash(object: &Map<String, Value>) -> Result<(), String> {
    let sighash = parse_exact_u64(
        object
            .get("sighashType")
            .ok_or_else(|| "missing sighashType".to_string())?,
        "sighashType",
    )?;
    if sighash != u64::from(crate::wire::pskt_schema::SIGHASH_ALL) {
        return Err(format!("unsupported sighashType: {sighash}"));
    }
    Ok(())
}

fn parse_partial_signatures(object: &Map<String, Value>) -> Result<Map<String, Value>, String> {
    let partial_signatures = match object.get("partialSigs") {
        None => Map::new(),
        Some(Value::Object(values)) => values.clone(),
        Some(_) => return Err("partialSigs must be an object".to_string()),
    };
    if partial_signatures.len() > usize::from(crate::SIGNER_CAPABILITIES.max_signatures_per_input) {
        return Err("too many partial signatures for signer capabilities".to_string());
    }
    Ok(partial_signatures)
}

fn parse_utxo_fields(object: &Map<String, Value>) -> Result<(u64, u16, Vec<u8>, bool), String> {
    let utxo = object
        .get("utxoEntry")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing utxoEntry".to_string())?;
    let amount = parse_utxo_amount(utxo)?;
    let (script_version, script_public_key) = parse_utxo_script(utxo)?;
    let has_covenant_id = parse_covenant_id_presence(utxo)?;
    Ok((amount, script_version, script_public_key, has_covenant_id))
}

fn parse_utxo_amount(utxo: &Map<String, Value>) -> Result<u64, String> {
    parse_exact_u64(
        utxo.get("amount")
            .ok_or_else(|| "missing amount".to_string())?,
        "amount",
    )
}

fn parse_utxo_script(utxo: &Map<String, Value>) -> Result<(u16, Vec<u8>), String> {
    let spk = utxo
        .get("scriptPublicKey")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing scriptPublicKey".to_string())?;
    let parsed = parse_spk(spk)?;
    if parsed.1.len() > 512 {
        return Err(format!(
            "spk too long for compact KSPT ({})",
            parsed.1.len()
        ));
    }
    Ok(parsed)
}

fn parse_covenant_id_presence(utxo: &Map<String, Value>) -> Result<bool, String> {
    let Some(value) = utxo.get("covenantId") else {
        return Ok(false);
    };
    let Some(text) = covenant_id_text(value)? else {
        return Ok(false);
    };
    validate_covenant_id(text)?;
    Ok(true)
}

fn covenant_id_text(value: &Value) -> Result<Option<&str>, String> {
    if value.is_null() {
        return Ok(None);
    }
    value
        .as_str()
        .map(Some)
        .ok_or_else(|| "utxoEntry.covenantId must be a hex string or null".to_string())
}

fn validate_covenant_id(text: &str) -> Result<(), String> {
    let bytes = super::wire::decode_lower_hex(text, "utxoEntry.covenantId")?;
    if bytes.len() != 32 {
        return Err("utxoEntry.covenantId must be 32 bytes".to_string());
    }
    Ok(())
}

fn parse_sequence(object: &Map<String, Value>) -> Result<u64, String> {
    object.get("sequence").map_or_else(
        || super::schema_validate::default_u64(crate::wire::pskt_schema::Scope::Input, "sequence"),
        |value| parse_exact_u64(value, "sequence"),
    )
}

fn parse_sig_op_count(object: &Map<String, Value>) -> Result<u8, String> {
    match object.get("sigOpCount") {
        None => u8::try_from(super::schema_validate::default_u64(
            crate::wire::pskt_schema::Scope::Input,
            "sigOpCount",
        )?)
        .map_err(|_| "shared sigOpCount default exceeds u8".to_string()),
        Some(value) => {
            let value = parse_exact_u64(value, "sigOpCount")?;
            let value = u8::try_from(value).map_err(|_| "sigOpCount exceeds u8".to_string())?;
            if value > crate::SIGNER_CAPABILITIES.max_signatures_per_input {
                return Err("sigOpCount exceeds signer capabilities".to_string());
            }
            Ok(value)
        }
    }
}

fn parse_covenant_execution(object: &Map<String, Value>) -> Result<Option<(u16, u16)>, String> {
    let Some(value) = object.get("covenantExecution") else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let map = covenant_execution_object(value)?;
    let mask = covenant_execution_u16(map, "suppliedMask")?;
    let truth = covenant_execution_u16(map, "suppliedTrueMask")?;
    if truth & !mask != 0 {
        return Err("covenantExecution true-mask contains unsupplied decisions".to_string());
    }
    Ok(Some((mask, truth)))
}

fn covenant_execution_object(value: &Value) -> Result<&Map<String, Value>, String> {
    let map = value
        .as_object()
        .ok_or_else(|| "covenantExecution must be an object or null".to_string())?;
    if map.len() != 2 || !map.contains_key("suppliedMask") || !map.contains_key("suppliedTrueMask")
    {
        return Err(
            "covenantExecution must contain only suppliedMask and suppliedTrueMask".to_string(),
        );
    }
    Ok(map)
}

fn covenant_execution_u16(map: &Map<String, Value>, key: &str) -> Result<u16, String> {
    let label = format!("covenantExecution.{key}");
    let value = parse_exact_u64(
        map.get(key).ok_or_else(|| format!("missing {label}"))?,
        &label,
    )?;
    u16::try_from(value).map_err(|_| format!("{label} exceeds u16"))
}

fn parse_redeem_script(object: &Map<String, Value>) -> Result<Option<Vec<u8>>, String> {
    let redeem = match object.get("redeemScript") {
        None | Some(Value::Null) => return Ok(None),
        Some(Value::String(value)) => super::wire::decode_lower_hex(value, "redeemScript")?,
        Some(_) => return Err("redeemScript must be a hex string or null".to_string()),
    };
    if redeem.len() > usize::from(crate::SIGNER_CAPABILITIES.max_redeem_script_bytes) {
        return Err("redeemScript exceeds signer capabilities".to_string());
    }
    Ok(Some(redeem))
}

fn parse_outpoint(object: &Map<String, Value>) -> Result<([u8; 32], u32), String> {
    let outpoint = object
        .get("previousOutpoint")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing previousOutpoint".to_string())?;
    let transaction_id = outpoint
        .get("transactionId")
        .and_then(Value::as_str)
        .ok_or_else(|| "missing transactionId".to_string())?;
    let bytes = super::wire::decode_lower_hex(transaction_id, "transactionId")?;
    let previous_tx_id = bytes
        .as_slice()
        .try_into()
        .map_err(|_| "tx_id not 32 bytes".to_string())?;
    let index = outpoint
        .get("index")
        .ok_or_else(|| "missing index".to_string())?;
    let previous_index = u32::try_from(parse_exact_u64(index, "previousOutpoint.index")?)
        .map_err(|_| "index exceeds u32".to_string())?;
    Ok((previous_tx_id, previous_index))
}

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

fn ensure_signature_capacity(fields: &InputFields) -> Result<(), String> {
    if fields.partial_signatures.len()
        > usize::from(crate::SIGNER_CAPABILITIES.max_signatures_per_input)
    {
        return Err("too many partial signatures for signer capabilities".to_string());
    }
    Ok(())
}

fn collect_multisig_signatures(
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

fn collect_covenant_signatures(
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

fn covenant_signature_position(
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

fn sort_unique_signatures(
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

fn collect_p2pk_signature(fields: &InputFields) -> Result<Vec<Signature>, String> {
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

fn validate_p2pk_script(fields: &InputFields) -> Result<(), String> {
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

fn compressed_xonly(public_key_hex: &str) -> Option<[u8; 32]> {
    let bytes = public_key_hex.as_bytes();
    if bytes.len() != 66
        || !bytes.iter().all(u8::is_ascii_hexdigit)
        || !(bytes.starts_with(b"02") || bytes.starts_with(b"03"))
    {
        return None;
    }
    let decoded = super::wire::decode_lower_hex(
        core::str::from_utf8(&bytes[2..]).ok()?,
        "compressed public key",
    )
    .ok()?;
    decoded.as_slice().try_into().ok()
}

fn decode_signature(value: &Value) -> Result<[u8; 64], String> {
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
    let bytes = super::wire::decode_lower_hex(text, "signature")?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| "Schnorr signature must be 64 bytes".to_string())
}

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

fn multisig_declared_count(script: &[u8]) -> Option<u8> {
    let position = script.len().checked_sub(2)?;
    script.get(position).copied().and_then(decode_small_int)
}

fn multisig_key_count(script: &[u8]) -> Option<u8> {
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

fn decode_small_int(opcode: u8) -> Option<u8> {
    (0x51..=0x60).contains(&opcode).then(|| opcode - 0x50)
}

pub(crate) fn find_pubkey_position(redeem: &[u8], public_key_hex: &str) -> Option<u8> {
    let xonly = compressed_xonly(public_key_hex)?;
    scan_pubkey_position(redeem, &xonly)
}

fn scan_pubkey_position(redeem: &[u8], xonly: &[u8]) -> Option<u8> {
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

fn parse_ms45_entry(entry: &Value) -> Result<Option<(u32, u32, u32)>, String> {
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

fn parse_ms45_part<'a, I>(parts: &mut I, label: &str, maximum: Option<u32>) -> Result<u32, String>
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

fn merge_ms45_candidate(
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

fn parse_soft(value: &str) -> Option<u32> {
    if value.ends_with('\'') {
        return None;
    }
    let parsed = value.parse::<u32>().ok()?;
    (parsed < 0x8000_0000).then_some(parsed)
}
