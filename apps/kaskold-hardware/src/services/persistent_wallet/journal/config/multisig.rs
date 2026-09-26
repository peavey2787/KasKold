//! Persistent current-format multisig descriptor codec.

use super::{MULTISIG_DESCRIPTOR_BYTES, MULTISIG_DESCRIPTOR_ENTRY_SIZE};
use offline_signer::transaction::model::{MultisigConfig, MultisigStore, MAX_MULTISIG_KEYS};

pub(super) fn encode_multisig_store(
    store: &MultisigStore,
    out: &mut [u8; MULTISIG_DESCRIPTOR_BYTES],
) {
    out.fill(0);
    for (slot, config) in store.configs.iter().enumerate() {
        if !encodable(config) {
            continue;
        }
        encode_slot(config, slot * MULTISIG_DESCRIPTOR_ENTRY_SIZE, out);
    }
}

fn encodable(config: &MultisigConfig) -> bool {
    config.active && config.v45 && config.n > 0 && usize::from(config.n) <= MAX_MULTISIG_KEYS
}

fn encode_slot(
    config: &MultisigConfig,
    base: usize,
    out: &mut [u8; MULTISIG_DESCRIPTOR_BYTES],
) {
    out[base] = 1;
    out[base + 1] = config.m;
    out[base + 2] = config.n;
    out[base + 3] = 1;
    out[base + 4] = config.cosigner_index;
    out[base + 5] = config.chain;
    out[base + 6..base + 10].copy_from_slice(&config.addr_index.to_le_bytes());
    for index in 0..usize::from(config.n) {
        encode_cosigner(config, index, base + 10 + index * 74, out);
    }
}

fn encode_cosigner(
    config: &MultisigConfig,
    index: usize,
    entry: usize,
    out: &mut [u8; MULTISIG_DESCRIPTOR_BYTES],
) {
    out[entry] = config.cosigner_depth[index];
    out[entry + 1..entry + 5].copy_from_slice(&config.cosigner_parent_fp[index]);
    out[entry + 5..entry + 9].copy_from_slice(&config.cosigner_child_num[index]);
    out[entry + 9..entry + 41].copy_from_slice(&config.cosigner_chain_codes[index]);
    out[entry + 41..entry + 74].copy_from_slice(&config.cosigner_pubkeys[index]);
}

pub(super) fn decode_multisig_store(
    input: &[u8; MULTISIG_DESCRIPTOR_BYTES],
) -> Option<MultisigStore> {
    let mut store = MultisigStore::new();
    for slot in 0..store.configs.len() {
        let base = slot * MULTISIG_DESCRIPTOR_ENTRY_SIZE;
        if let Some(config) = decode_multisig_slot(input, base)? {
            store.configs[slot] = config;
        }
    }
    Some(store)
}

fn decode_multisig_slot(
    input: &[u8; MULTISIG_DESCRIPTOR_BYTES],
    base: usize,
) -> Option<Option<MultisigConfig>> {
    if input[base] == 0 {
        return empty_multisig_slot(input, base).then_some(None);
    }
    if input[base] != 1 {
        return None;
    }

    let m = input[base + 1];
    let n = input[base + 2];
    if !multisig_header_valid(input, base, m, n) {
        return None;
    }

    let mut config = MultisigConfig::new();
    config.m = m;
    config.n = n;
    config.v45 = true;
    config.cosigner_index = input[base + 4];
    config.chain = input[base + 5];
    config.addr_index = u32::from_le_bytes(input[base + 6..base + 10].try_into().ok()?);
    if !decode_multisig_cosigners(input, base, &mut config) {
        return None;
    }
    let tail = base + 10 + usize::from(n) * 74;
    if !multisig_tail_clear(input, tail, base + MULTISIG_DESCRIPTOR_ENTRY_SIZE) {
        return None;
    }
    config.active = true;
    (config.build_script() != 0).then_some(Some(config))
}

fn empty_multisig_slot(input: &[u8; MULTISIG_DESCRIPTOR_BYTES], base: usize) -> bool {
    input[base + 1..base + MULTISIG_DESCRIPTOR_ENTRY_SIZE]
        .iter()
        .all(|byte| *byte == 0)
}

fn multisig_header_valid(
    input: &[u8; MULTISIG_DESCRIPTOR_BYTES],
    base: usize,
    m: u8,
    n: u8,
) -> bool {
    input[base + 3] == 1
        && n > 0
        && usize::from(n) <= MAX_MULTISIG_KEYS
        && m > 0
        && m <= n
        && input[base + 4] < n
        && input[base + 5] <= 1
}

fn decode_multisig_cosigners(
    input: &[u8; MULTISIG_DESCRIPTOR_BYTES],
    base: usize,
    config: &mut MultisigConfig,
) -> bool {
    for index in 0..usize::from(config.n) {
        let entry = base + 10 + index * 74;
        let parts = decode_multisig_kpub_parts(input, entry);
        if !config.set_cosigner(index, &parts) {
            return false;
        }
    }
    true
}

fn decode_multisig_kpub_parts(
    input: &[u8; MULTISIG_DESCRIPTOR_BYTES],
    entry: usize,
) -> offline_signer::derivation::xpub::KpubParts {
    let mut parts = offline_signer::derivation::xpub::KpubParts {
        depth: input[entry],
        parent_fp: [0; 4],
        child_num: [0; 4],
        chain_code: [0; 32],
        pubkey: [0; 33],
    };
    parts.parent_fp.copy_from_slice(&input[entry + 1..entry + 5]);
    parts.child_num.copy_from_slice(&input[entry + 5..entry + 9]);
    parts.chain_code.copy_from_slice(&input[entry + 9..entry + 41]);
    parts.pubkey.copy_from_slice(&input[entry + 41..entry + 74]);
    parts
}

fn multisig_tail_clear(
    input: &[u8; MULTISIG_DESCRIPTOR_BYTES],
    start: usize,
    end: usize,
) -> bool {
    input[start..end].iter().all(|byte| *byte == 0)
}
