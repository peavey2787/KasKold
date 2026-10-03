//! Kaspa SIGHASH_ALL digest of a compact KSPT transaction.

use super::{Params, Transaction};

pub(crate) fn signing_hash_state() -> blake2b_simd::State {
    Params::new()
        .hash_length(32)
        .key(b"TransactionSigningHash")
        .to_state()
}

pub(crate) fn finish_hash(state: blake2b_simd::State) -> [u8; 32] {
    let hash = state.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

pub(crate) fn hash_previous_outputs(transaction: &Transaction) -> [u8; 32] {
    let mut state = signing_hash_state();
    for input in &transaction.inputs {
        state.update(&input.tx_id);
        state.update(&input.index.to_le_bytes());
    }
    finish_hash(state)
}

pub(crate) fn hash_sequences(transaction: &Transaction) -> [u8; 32] {
    let mut state = signing_hash_state();
    for input in &transaction.inputs {
        state.update(&input.sequence.to_le_bytes());
    }
    finish_hash(state)
}

pub(crate) fn hash_sig_op_counts(transaction: &Transaction) -> [u8; 32] {
    let mut state = signing_hash_state();
    for input in &transaction.inputs {
        state.update(&[input.sig_op_count]);
    }
    finish_hash(state)
}

pub(crate) fn hash_outputs(transaction: &Transaction) -> [u8; 32] {
    let mut state = signing_hash_state();
    for output in &transaction.outputs {
        state.update(&output.amount.to_le_bytes());
        state.update(&output.script_version.to_le_bytes());
        state.update(&(output.script.len() as u64).to_le_bytes());
        state.update(&output.script);
        if transaction.version >= 1 {
            match output.covenant {
                Some((authorizing_input, id)) => {
                    state.update(&[1]);
                    state.update(&authorizing_input.to_le_bytes());
                    state.update(&id);
                }
                None => {
                    state.update(&[0]);
                }
            }
        }
    }
    finish_hash(state)
}

pub(crate) fn hash_payload(transaction: &Transaction) -> [u8; 32] {
    if transaction.subnetwork == [0u8; 20] && transaction.payload.is_empty() {
        return [0u8; 32];
    }
    let mut state = signing_hash_state();
    state.update(&(transaction.payload.len() as u64).to_le_bytes());
    state.update(&transaction.payload);
    finish_hash(state)
}

pub(crate) fn sighash_all(
    transaction: &Transaction,
    input_index: usize,
) -> Result<[u8; 32], String> {
    let input = transaction
        .inputs
        .get(input_index)
        .ok_or_else(|| "sighash input index out of range".to_string())?;
    let mut state = signing_hash_state();
    state.update(&transaction.version.to_le_bytes());
    state.update(&hash_previous_outputs(transaction));
    state.update(&hash_sequences(transaction));
    if transaction.version == 0 {
        state.update(&hash_sig_op_counts(transaction));
    }
    state.update(&input.tx_id);
    state.update(&input.index.to_le_bytes());
    state.update(&input.script_version.to_le_bytes());
    state.update(&(input.script.len() as u64).to_le_bytes());
    state.update(&input.script);
    state.update(&input.amount.to_le_bytes());
    state.update(&input.sequence.to_le_bytes());
    if transaction.version == 0 {
        state.update(&[input.sig_op_count]);
    }
    state.update(&hash_outputs(transaction));
    state.update(&transaction.locktime.to_le_bytes());
    state.update(&transaction.subnetwork);
    state.update(&transaction.gas.to_le_bytes());
    state.update(&hash_payload(transaction));
    state.update(&[0x01]);
    Ok(finish_hash(state))
}
