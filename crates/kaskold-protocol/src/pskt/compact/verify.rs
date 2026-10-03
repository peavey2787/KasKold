//! Cryptographic completeness and BIP340 verification of compact KSPT signatures.

use super::{
    covenant_active_positions, multisig_xonly, require_sighash_all, sighash_all, Input,
    K256Signature, Signature, Transaction, VerifyingKey,
};

pub(crate) fn verified_complete(transaction: &Transaction) -> Result<bool, String> {
    if transaction.inputs.is_empty() {
        return Ok(false);
    }
    for (index, input) in transaction.inputs.iter().enumerate() {
        let Some(required) = required_signature_count(index, input)? else {
            return Ok(false);
        };
        if input.signatures.len() != required {
            return Ok(false);
        }
        verify_input_signatures(transaction, index)?;
    }
    Ok(true)
}

pub(crate) fn required_signature_count(
    index: usize,
    input: &Input,
) -> Result<Option<usize>, String> {
    if input.redeem.is_empty() {
        return Ok(canonical_p2pk(input).then_some(1));
    }
    if let Some((threshold, _)) = super::super::relay_fields::parse_multisig_redeem(&input.redeem) {
        return Ok(Some(usize::from(threshold)));
    }
    if input.specialized_witness.is_some() {
        return Ok((input.signatures.len() == 1).then_some(1));
    }
    generic_covenant_required(index, input)
}

pub(crate) fn canonical_p2pk(input: &Input) -> bool {
    input.script.len() == 34
        && input.script.first() == Some(&0x20)
        && input.script.get(33) == Some(&0xac)
}

pub(crate) fn generic_covenant_required(
    index: usize,
    input: &Input,
) -> Result<Option<usize>, String> {
    let Some((mask, truth)) = input.covenant_execution else {
        return Ok(None);
    };
    let branches = shared_signer::covenant_branch::resolve_covenant_branches(&input.redeem)
        .map_err(|error| format!("input[{index}] invalid covenant branch structure: {error:?}"))?;
    if mask != branches.selector_mask() || truth & !mask != 0 || branches.selector_mask() != 0b1 {
        return Ok(None);
    }
    let active = covenant_active_positions(&branches, mask, truth);
    if active.len() != 1 || input.signatures.len() != 1 {
        return Ok(None);
    }
    let Some(signature) = input.signatures.first() else {
        return Ok(None);
    };
    if signature.position != active[0] {
        return Ok(None);
    }
    let Ok(binding) = branches.key_at(active[0]) else {
        return Ok(None);
    };
    Ok((binding.decision_mask & 0b1 != 0).then_some(1))
}

pub(crate) fn verify_all_signatures(transaction: &Transaction) -> Result<(), String> {
    for index in 0..transaction.inputs.len() {
        verify_input_signatures(transaction, index)?;
    }
    Ok(())
}

pub(crate) fn verified_signature_count_for_input(
    transaction: &Transaction,
    input_index: usize,
) -> Result<usize, String> {
    let input = transaction
        .inputs
        .get(input_index)
        .ok_or_else(|| "signature input index out of range".to_string())?;
    verify_input_signatures(transaction, input_index)?;
    if input.redeem.is_empty()
        || super::super::relay_fields::parse_multisig_redeem(&input.redeem).is_some()
    {
        return Ok(input.signatures.len());
    }
    if input.specialized_witness.is_some() {
        return specialized_signature_count(input, input_index);
    }
    generic_covenant_signature_count(input, input_index)
}

pub(crate) fn specialized_signature_count(
    input: &Input,
    input_index: usize,
) -> Result<usize, String> {
    if input.signatures.len() != 1 {
        return Err(format!(
            "input[{input_index}] typed specialized covenant requires exactly one verified signature"
        ));
    }
    Ok(1)
}

pub(crate) fn generic_covenant_signature_count(
    input: &Input,
    input_index: usize,
) -> Result<usize, String> {
    let (mask, truth) = input
        .covenant_execution
        .ok_or_else(|| format!("input[{input_index}] covenant is missing covenantExecution"))?;
    let branches = shared_signer::covenant_branch::resolve_covenant_branches(&input.redeem)
        .map_err(|error| {
            format!("input[{input_index}] invalid covenant branch structure: {error:?}")
        })?;
    validate_generic_covenant_count_binding(input, input_index, &branches, mask, truth)?;
    Ok(input.signatures.len())
}

pub(crate) fn validate_generic_covenant_count_binding(
    input: &Input,
    input_index: usize,
    branches: &shared_signer::covenant_branch::BranchResolution,
    mask: u16,
    truth: u16,
) -> Result<(), String> {
    if truth & !mask != 0 || mask != branches.selector_mask() {
        return Err(format!(
            "input[{input_index}] covenantExecution is not a complete selector assignment"
        ));
    }
    if branches.selector_mask() != 0b1 {
        return Err(format!(
            "input[{input_index}] generic covenant selector topology requires a typed specialized witness plan"
        ));
    }
    let active = covenant_active_positions(branches, mask, truth);
    if active.len() != 1 {
        return Err(format!(
            "input[{input_index}] generic covenant must have exactly one active signer"
        ));
    }
    if input
        .signatures
        .iter()
        .any(|signature| signature.position != active[0])
    {
        return Err(format!(
            "input[{input_index}] contains a signature outside the active covenant branch"
        ));
    }
    Ok(())
}

pub(crate) fn verify_input_signatures(
    transaction: &Transaction,
    input_index: usize,
) -> Result<(), String> {
    let input = transaction
        .inputs
        .get(input_index)
        .ok_or_else(|| "signature input index out of range".to_string())?;
    if input.signatures.is_empty() {
        return Ok(());
    }
    let message = sighash_all(transaction, input_index)?;
    let mut seen = [false; 256];
    for signature in &input.signatures {
        verify_one_input_signature(input, input_index, signature, &message, &mut seen)?;
    }
    Ok(())
}

pub(crate) fn verify_one_input_signature(
    input: &Input,
    input_index: usize,
    signature: &Signature,
    message: &[u8; 32],
    seen: &mut [bool; 256],
) -> Result<(), String> {
    require_sighash_all(signature, input_index)?;
    let slot = usize::from(signature.position);
    if seen[slot] {
        return Err(format!(
            "input[{input_index}] contains duplicate signature position {}",
            signature.position
        ));
    }
    seen[slot] = true;
    let public_key = bound_public_key(input, input_index, signature)?;
    verify_bip340(&public_key, message, &signature.bytes).map_err(|error| {
        format!(
            "input[{input_index}] signature position {}: {error}",
            signature.position
        )
    })
}

pub(crate) fn bound_public_key(
    input: &Input,
    input_index: usize,
    signature: &Signature,
) -> Result<[u8; 32], String> {
    if input.redeem.is_empty() {
        return p2pk_bound_key(input, input_index, signature.position);
    }
    if super::super::relay_fields::parse_multisig_redeem(&input.redeem).is_some() {
        return multisig_xonly(&input.redeem, signature.position).ok_or_else(|| {
            format!(
                "input[{input_index}] signature position {} has no bound public key",
                signature.position
            )
        });
    }
    covenant_bound_key(input, input_index, signature.position)
}

pub(crate) fn p2pk_bound_key(
    input: &Input,
    input_index: usize,
    position: u8,
) -> Result<[u8; 32], String> {
    if position != 0 || !canonical_p2pk(input) {
        return Err(format!(
            "input[{input_index}] signature is not bound to a canonical P2PK key"
        ));
    }
    input.script[1..33].try_into().map_err(|_| {
        format!("input[{input_index}] signature position {position} has no bound public key")
    })
}

pub(crate) fn covenant_bound_key(
    input: &Input,
    input_index: usize,
    position: u8,
) -> Result<[u8; 32], String> {
    let branches = shared_signer::covenant_branch::resolve_covenant_branches(&input.redeem)
        .map_err(|error| {
            format!("input[{input_index}] invalid covenant branch structure: {error:?}")
        })?;
    let binding = branches.key_at(position).map_err(|_| {
        format!(
            "input[{input_index}] covenant signature position {position} has no branch-bound key"
        )
    })?;
    if let Some((mask, truth)) = input.covenant_execution {
        if !binding.matches_selectors(mask, truth) {
            return Err(format!(
                "input[{input_index}] covenant signature position {position} is outside the proven execution branch"
            ));
        }
    }
    Ok(binding.key)
}

pub(crate) fn verify_bip340(
    public_key: &[u8; 32],
    message: &[u8; 32],
    signature: &[u8; 64],
) -> Result<(), String> {
    let key = VerifyingKey::from_bytes(public_key)
        .map_err(|_| "invalid Schnorr public key".to_string())?;
    let signature = K256Signature::try_from(signature.as_slice())
        .map_err(|_| "invalid Schnorr signature encoding".to_string())?;
    key.verify_raw(message, &signature)
        .map_err(|_| "Schnorr signature verification failed".to_string())
}
