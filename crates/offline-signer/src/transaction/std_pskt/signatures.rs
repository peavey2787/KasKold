//! PSKT signature merge and UI status helpers.

use crate::transaction::model::{Transaction, MAX_SIGS_PER_INPUT};

use super::PskError;

/// After `sign_transaction_multisig` or `sign_transaction_multi_addr`
/// has populated `inp.sigs[]` with new signatures tagged by
/// `pubkey_compressed`, promote them into `inp.incoming_partial_sigs[]`
/// ready for PSKT emission.
///
/// Merging rules:
///   - Existing entries in `incoming_partial_sigs` (from a PSKT that
///     arrived partially signed) are preserved.
///   - Each new entry from `sigs[]` with `present=true` and a non-zero
///     `pubkey_compressed` is inserted — unless a matching pubkey
///     already exists. An identical signature is idempotent; different bytes
///     for the same public key are a hard conflict and abort the merge.
///   - After insertion, the slot array is sorted by pubkey byte order
///     so emission matches `kaspa-wallet-pskt`'s BTreeMap iteration.
///
/// If the fixed signature capacity is already full, the merge fails instead
/// of silently dropping a signature. The complete retained set is sorted by
/// public key only after a successful merge.
///
/// Does not mutate `sigs[]` — KSPT emission on the same tx still works
/// if the caller picks that path instead. Designed to be idempotent:
/// calling this twice is a no-op on the second call.
fn incoming_signature_for_pubkey(
    input: &crate::transaction::model::TransactionInput,
    pubkey: [u8; 33],
) -> Option<[u8; 64]> {
    input.incoming_partial_sigs[..input.incoming_partial_sigs_count as usize]
        .iter()
        .find(|entry| entry.pubkey == pubkey)
        .map(|entry| entry.signature)
}

fn preflight_ksp_signatures(
    input: &crate::transaction::model::TransactionInput,
) -> Result<(), PskError> {
    let mut additions = 0usize;
    for slot_index in 0..input.sig_count as usize {
        if preflight_slot_adds_signature(input, slot_index)? {
            additions += 1;
        }
    }
    let existing = input.incoming_partial_sigs_count as usize;
    if existing.saturating_add(additions) > MAX_SIGS_PER_INPUT {
        return Err(PskError::TooManyPartialSigs);
    }
    Ok(())
}

fn preflight_slot_adds_signature(
    input: &crate::transaction::model::TransactionInput,
    slot_index: usize,
) -> Result<bool, PskError> {
    let slot = &input.sigs[slot_index];
    if !slot.present || slot.pubkey_compressed == [0u8; 33] {
        return Ok(false);
    }
    if let Some(existing) = incoming_signature_for_pubkey(input, slot.pubkey_compressed) {
        if existing != slot.signature {
            return Err(PskError::SignatureConflict);
        }
        return Ok(false);
    }
    Ok(prior_signature_for_pubkey(input, slot_index, slot)?.is_none())
}

fn prior_signature_for_pubkey(
    input: &crate::transaction::model::TransactionInput,
    slot_index: usize,
    slot: &crate::transaction::model::InputSig,
) -> Result<Option<[u8; 64]>, PskError> {
    let mut found = None;
    for prior in &input.sigs[..slot_index] {
        if !prior.present
            || prior.pubkey_compressed == [0u8; 33]
            || prior.pubkey_compressed != slot.pubkey_compressed
        {
            continue;
        }
        if prior.signature != slot.signature {
            return Err(PskError::SignatureConflict);
        }
        found = Some(prior.signature);
    }
    Ok(found)
}

fn append_ksp_signatures(
    input: &mut crate::transaction::model::TransactionInput,
) -> Result<(), PskError> {
    for slot_index in 0..input.sig_count as usize {
        let slot = input.sigs[slot_index].clone();
        if !slot.present || slot.pubkey_compressed == [0u8; 33] {
            continue;
        }
        if let Some(existing) = incoming_signature_for_pubkey(input, slot.pubkey_compressed) {
            if existing != slot.signature {
                return Err(PskError::SignatureConflict);
            }
            continue;
        }
        let next = input.incoming_partial_sigs_count as usize;
        if next >= MAX_SIGS_PER_INPUT {
            return Err(PskError::TooManyPartialSigs);
        }
        input.incoming_partial_sigs[next].pubkey = slot.pubkey_compressed;
        input.incoming_partial_sigs[next].signature = slot.signature;
        input.incoming_partial_sigs[next].present = true;
        input.incoming_partial_sigs_count = (next + 1) as u8;
    }
    Ok(())
}

fn sort_incoming_signatures(input: &mut crate::transaction::model::TransactionInput) {
    let count = input.incoming_partial_sigs_count as usize;
    input.incoming_partial_sigs[..count].sort_unstable_by_key(|entry| entry.pubkey);
}

pub fn move_ksp_sigs_to_pskt(tx: &mut Transaction) -> Result<(), PskError> {
    // Preflight every input before mutating any of them so a late conflict or
    // capacity error cannot leave the transaction half-merged.
    for input_index in 0..tx.num_inputs {
        preflight_ksp_signatures(&tx.inputs[input_index])?;
    }
    for input_index in 0..tx.num_inputs {
        let input = &mut tx.inputs[input_index];
        let base = input.incoming_partial_sigs_count;
        append_ksp_signatures(input)?;
        if input.incoming_partial_sigs_count != base {
            sort_incoming_signatures(input);
        }
    }
    Ok(())
}

/// Return `(cryptographically_valid, required)` for incoming standard-PSKT
/// partial signatures. Invalid, misbound, duplicate, or wrong-sighash entries
/// contribute zero; structural parsing never promotes them to valid progress.
pub fn pskt_signature_status(tx: &Transaction) -> (u32, u32) {
    crate::transaction::signature_verification::pskt_status(tx)
}
