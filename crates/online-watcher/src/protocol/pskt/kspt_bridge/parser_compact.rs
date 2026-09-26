#[cfg(test)]
use crate::protocol::pskt::model::KsptSigRecord;

#[cfg(test)]
use super::parse_compact_kspt_transaction;

#[cfg(test)]
pub(crate) fn parse_compact_kspt_signatures(
    data: &[u8],
) -> Result<Vec<Vec<KsptSigRecord>>, String> {
    parse_compact_kspt_transaction(data).map(|transaction| {
        transaction
            .inputs
            .into_iter()
            .map(|input| {
                input
                    .signatures
                    .into_iter()
                    .map(|signature| KsptSigRecord {
                        pubkey_pos: signature.pubkey_pos,
                        sighash_type: signature.sighash_type,
                        sig: signature.signature,
                    })
                    .collect()
            })
            .collect()
    })
}

fn standard_multisig_key_count(script: &[u8]) -> Option<usize> {
    crate::protocol::pskt::review::parse_multisig_redeem(script)
        .map(|(_, key_count)| usize::from(key_count))
}

fn standard_multisig_xonly(script: &[u8], position: u8) -> Option<[u8; 32]> {
    let key_count = standard_multisig_key_count(script)?;
    let position = usize::from(position);
    if position >= key_count {
        return None;
    }
    let start = 2usize.checked_add(position.checked_mul(33)?)?;
    script.get(start..start.checked_add(32)?)?.try_into().ok()
}

/// Resolve a signature position with the same strict structural covenant resolver
/// used by signing and protocol verification. There is deliberately no fallback
/// raw-byte scan: malformed or ambiguous covenant scripts fail closed.
pub(crate) fn xonly_at_position(script: &[u8], position: u8) -> Option<[u8; 32]> {
    if let Some(key) = standard_multisig_xonly(script, position) {
        return Some(key);
    }
    shared_signer::covenant_branch::resolve_covenant_branches(script)
        .ok()?
        .key_at(position)
        .ok()
        .map(|binding| binding.key)
}
