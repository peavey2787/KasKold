use shared_signer::bytes::zeroize_bytes;

use crate::transaction::{
    model::{SigHashType, Transaction},
    sighash,
};

use super::super::{error::PsktError, validation::checked_redeem_bytes};
use super::{context::SigningContext, signature_state::set_single_signature};

const MAX_COVENANT_KEYS: usize = 8;

pub(super) struct CandidateKeys {
    pub(super) keys: [[u8; 32]; MAX_COVENANT_KEYS],
    /// Original resolver positions, preserved even when inactive branches are filtered out.
    pub(super) positions: [u8; MAX_COVENANT_KEYS],
    pub(super) len: usize,
}

impl CandidateKeys {
    const fn new() -> Self {
        Self {
            keys: [[0u8; 32]; MAX_COVENANT_KEYS],
            positions: [0u8; MAX_COVENANT_KEYS],
            len: 0,
        }
    }
}

/// Structural resolver used by parser-focused unit tests. Signing authorization must use
/// `candidate_keys_for_input`, which additionally binds candidates to covenantExecution.
#[cfg(test)]
pub(super) fn scan_candidate_keys(script: &[u8]) -> Result<CandidateKeys, PsktError> {
    let resolved = shared_signer::covenant_branch::resolve_covenant_branches(script)
        .map_err(|_| PsktError::InvalidModel)?;
    if resolved.len() > MAX_COVENANT_KEYS {
        return Err(PsktError::InvalidModel);
    }
    let mut candidates = CandidateKeys::new();
    for position in 0..resolved.len() {
        let position = u8::try_from(position).map_err(|_| PsktError::InvalidModel)?;
        let branch = resolved
            .key_at(position)
            .map_err(|_| PsktError::InvalidModel)?;
        candidates.keys[candidates.len] = branch.key;
        candidates.positions[candidates.len] = position;
        candidates.len += 1;
    }
    Ok(candidates)
}

fn scan_active_candidate_keys(
    script: &[u8],
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<CandidateKeys, PsktError> {
    let resolved = resolve_execution_branches(script, supplied_mask, supplied_true_mask)?;
    let candidates = collect_active_candidate_keys(&resolved, supplied_mask, supplied_true_mask)?;
    require_single_active_candidate(candidates)
}

fn resolve_execution_branches(
    script: &[u8],
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<shared_signer::covenant_branch::BranchResolution, PsktError> {
    if supplied_true_mask & !supplied_mask != 0 {
        return Err(PsktError::InvalidModel);
    }
    let resolved = shared_signer::covenant_branch::resolve_covenant_branches(script)
        .map_err(|_| PsktError::InvalidModel)?;
    if supplied_mask != resolved.selector_mask() {
        return Err(PsktError::InvalidModel);
    }
    // Signing and raw-KSPT broadcasting deliberately have different topology
    // boundaries. A Vault may sign a fully specified richer selector assignment
    // when it resolves to exactly one active signing key; after merge, the host
    // must still prove a typed family-specific VerifiedWitnessPlan before such a
    // witness can be materialized. Raw generic KSPT broadcast remains restricted
    // to the single-selector generic plan.
    if resolved.len() > MAX_COVENANT_KEYS {
        return Err(PsktError::InvalidModel);
    }
    Ok(resolved)
}

fn collect_active_candidate_keys(
    resolved: &shared_signer::covenant_branch::BranchResolution,
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<CandidateKeys, PsktError> {
    let mut candidates = CandidateKeys::new();
    for position in 0..resolved.len() {
        append_active_candidate(
            &mut candidates,
            resolved,
            position,
            supplied_mask,
            supplied_true_mask,
        )?;
    }
    Ok(candidates)
}

fn append_active_candidate(
    candidates: &mut CandidateKeys,
    resolved: &shared_signer::covenant_branch::BranchResolution,
    position: usize,
    supplied_mask: u16,
    supplied_true_mask: u16,
) -> Result<(), PsktError> {
    let encoded_position = u8::try_from(position).map_err(|_| PsktError::InvalidModel)?;
    let binding = resolved
        .key_at(encoded_position)
        .map_err(|_| PsktError::InvalidModel)?;
    if !binding.matches_selectors(supplied_mask, supplied_true_mask) {
        return Ok(());
    }
    // A branchless canonical CHECKSIG has no structural selector bits.
    // When covenantExecution is present and the supplied mask exactly
    // matches the resolver's zero selector mask, that is a complete
    // execution proof rather than an absent proof.
    candidates.keys[candidates.len] = binding.key;
    candidates.positions[candidates.len] = encoded_position;
    candidates.len += 1;
    Ok(())
}

fn require_single_active_candidate(candidates: CandidateKeys) -> Result<CandidateKeys, PsktError> {
    // Signing is intentionally fail-closed around signature cardinality: every
    // supplied selector assignment must resolve to exactly one active signing key.
    // Richer witness topology is authorized separately by the typed host finalizer.
    if candidates.len != 1 {
        return Err(PsktError::InvalidModel);
    }
    Ok(candidates)
}

pub(super) fn candidate_keys_for_input(
    tx: &Transaction,
    input_index: usize,
) -> Result<CandidateKeys, PsktError> {
    let input = tx.inputs.get(input_index).ok_or(PsktError::InvalidModel)?;
    if !input.covenant_execution_present {
        return Err(PsktError::InvalidModel);
    }
    checked_redeem_bytes(tx, input_index).and_then(|script| {
        scan_active_candidate_keys(
            script,
            input.covenant_execution_mask,
            input.covenant_execution_true_mask,
        )
    })
}

struct CovenantCandidate<'a> {
    position: u8,
    target: &'a [u8; 32],
}

pub(super) fn sign_covenant_input(
    tx: &mut Transaction,
    input_index: usize,
    context: &mut SigningContext,
    sighash_type: SigHashType,
    active_seed_index: Option<usize>,
    signing_entropy: Option<&[u8; 32]>,
) -> Result<usize, PsktError> {
    if tx.inputs[input_index].sig_count != 0 {
        return Ok(0);
    }
    let candidates = candidate_keys_for_input(tx, input_index)?;
    for candidate_index in 0..candidates.len {
        if sign_covenant_candidate(
            tx,
            input_index,
            context,
            sighash_type,
            active_seed_index,
            signing_entropy,
            CovenantCandidate {
                position: candidates.positions[candidate_index],
                target: &candidates.keys[candidate_index],
            },
        )? {
            return Ok(1);
        }
    }
    Ok(0)
}

fn sign_covenant_candidate(
    tx: &mut Transaction,
    input_index: usize,
    context: &mut SigningContext,
    sighash_type: SigHashType,
    active_seed_index: Option<usize>,
    signing_entropy: Option<&[u8; 32]>,
    candidate: CovenantCandidate<'_>,
) -> Result<bool, PsktError> {
    for seed_index in 0..context.seed_count() {
        if !seed_is_active(active_seed_index, seed_index) {
            continue;
        }
        let Some(mut material) = covenant_material(context, seed_index, candidate.target) else {
            continue;
        };
        let signature = sign_covenant_material(
            tx,
            input_index,
            &material.private_key,
            sighash_type,
            signing_entropy,
        )?;
        zeroize_bytes(&mut material.private_key);
        set_single_signature(
            &mut tx.inputs[input_index],
            signature,
            sighash_type.to_byte(),
            candidate.position,
            material.compressed_public_key,
        );
        return Ok(true);
    }
    Ok(false)
}

fn seed_is_active(active_seed_index: Option<usize>, seed_index: usize) -> bool {
    active_seed_index.is_none() || active_seed_index == Some(seed_index)
}

fn covenant_material(
    context: &mut SigningContext,
    seed_index: usize,
    target: &[u8; 32],
) -> Option<super::context::SigningKeyMaterial> {
    if context.account_xonly(seed_index) == Some(*target) {
        context.account_material(seed_index)
    } else {
        context.cached_address_material(seed_index, target)
    }
}

fn sign_covenant_material(
    tx: &Transaction,
    input_index: usize,
    private_key: &[u8; 32],
    sighash_type: SigHashType,
    signing_entropy: Option<&[u8; 32]>,
) -> Result<[u8; 64], PsktError> {
    let result = match signing_entropy {
        Some(entropy) => {
            sighash::sign_input_with_entropy(tx, input_index, private_key, sighash_type, entropy)
        }
        None => sighash::sign_input(tx, input_index, private_key, sighash_type),
    };
    result
        .map(|signature| signature.bytes)
        .map_err(|_| PsktError::SigningFailed)
}
