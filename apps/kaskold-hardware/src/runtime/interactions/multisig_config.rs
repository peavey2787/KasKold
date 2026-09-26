//! Shared multisig descriptor/state installation and persistence helpers.

use crate::runtime::data::AppData;
use offline_signer::transaction::model::MultisigConfig;
use kaskold_protocol::wire::multisig_descriptor::ParsedMultisigDescriptor;

pub(crate) fn config_from_descriptor(
    descriptor: &ParsedMultisigDescriptor,
    active: bool,
) -> MultisigConfig {
    let mut config = MultisigConfig::new();
    config.m = descriptor.threshold;
    config.n = descriptor.participant_count;
    config.v45 = descriptor.v45;
    config.cosigner_pubkeys = descriptor.public_keys;
    config.cosigner_chain_codes = descriptor.chain_codes;
    config.cosigner_depth = descriptor.depths;
    config.cosigner_parent_fp = descriptor.parent_fingerprints;
    config.cosigner_child_num = descriptor.child_numbers;
    config.active = active;
    config.build_script();
    config
}

pub(crate) fn install_descriptor(
    ad: &mut AppData,
    descriptor: &ParsedMultisigDescriptor,
    active: bool,
) {
    ad.signing.multisig.creating = config_from_descriptor(descriptor, active);
}

pub(crate) fn install_descriptor_and_resolve(
    ad: &mut AppData,
    descriptor: &ParsedMultisigDescriptor,
    active: bool,
    checkpoint: &mut (impl FnMut() + ?Sized),
) {
    install_descriptor(ad, descriptor, active);
    let _ = crate::runtime::interactions::tx::resolve_loaded_cosigner_index(ad, checkpoint);
    ad.signing.multisig.creating.build_script();
}

/// Persist the currently edited config back to its matching stored wallet.
/// Wallet identity is centralized in `MultisigConfig::same_wallet_as`.
pub(crate) fn persist_creating_config(ad: &mut AppData) -> bool {
    let creating = ad.signing.multisig.creating.clone();
    let Some(config) = ad.signing.multisig.store.configs.iter_mut().find(|config| {
        config.active && config.same_wallet_as(&creating)
    }) else { return false; };
    *config = creating;
    true
}

/// Return whether a persisted current-format descriptor actually contains the
/// currently active mnemonic wallet. Descriptor visibility is intentionally
/// scoped to the open wallet rather than any other wallet merely present on
/// the device.
pub(crate) fn belongs_to_active_wallet(
    ad: &AppData,
    config: &MultisigConfig,
    checkpoint: &mut (impl FnMut() + ?Sized),
) -> bool {
    if !config.active || !config.v45 {
        return false;
    }
    let active = usize::from(ad.wallet.seeds.seed_mgr.active);
    if active >= crate::wallet::seed_manager::MAX_SLOTS {
        return false;
    }
    let Some(slot) = ad.wallet.seeds.seed_mgr.slots.get(active) else { return false; };
    if !ad.wallet.seeds.seed_mgr.slot_visible(active) || !slot.is_mnemonic() {
        return false;
    }
    let Ok(mut seed) = crate::services::wallet_keys::derive_slot_seed_with_checkpoint(slot, checkpoint) else {
        return false;
    };
    checkpoint();
    let parts = offline_signer::derivation::xpub::derive_multisig_account_parts(&seed.bytes, 0);
    checkpoint();
    crate::runtime::signing::zeroize_seed(&mut seed.bytes);
    let Ok(parts) = parts else { return false; };
    let mut candidate = config.clone();
    candidate.resolve_cosigner_index(&parts)
}

pub(crate) fn visible_descriptor_indices(
    ad: &AppData,
    checkpoint: &mut (impl FnMut() + ?Sized),
) -> ([u8; offline_signer::transaction::model::MAX_MULTISIG_WALLETS], u8) {
    let mut indices = [u8::MAX; offline_signer::transaction::model::MAX_MULTISIG_WALLETS];
    let mut count = 0usize;
    for (index, config) in ad.signing.multisig.store.configs.iter().enumerate() {
        if ad.signing.multisig.descriptor_persisted[index]
            && belongs_to_active_wallet(ad, config, checkpoint)
        {
            indices[count] = index as u8;
            count += 1;
        }
    }
    (indices, count as u8)
}


pub(crate) fn save_creating_descriptor(ad: &mut AppData) -> Option<usize> {
    let creating = ad.signing.multisig.creating.clone();
    let existing = ad.signing.multisig.store.configs.iter().position(|config| {
        config.active && config.same_wallet_as(&creating)
    });
    let slot = existing.or_else(|| ad.signing.multisig.store.find_free())?;
    ad.signing.multisig.store.configs[slot] = creating;
    ad.signing.multisig.descriptor_persisted[slot] = true;
    Some(slot)
}

pub(crate) fn delete_descriptor(ad: &mut AppData, slot: usize) -> bool {
    if slot >= ad.signing.multisig.store.configs.len()
        || !ad.signing.multisig.descriptor_persisted[slot]
    {
        return false;
    }
    ad.signing.multisig.store.configs[slot] = MultisigConfig::new();
    ad.signing.multisig.descriptor_persisted[slot] = false;
    true
}
