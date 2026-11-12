//! Shared multisig descriptor/state installation and persistence helpers.

use crate::runtime::data::{AppData, MultisigDraft};
use kaskold_protocol::wire::multisig_descriptor::ParsedMultisigDescriptor;
use offline_signer::transaction::model::MultisigConfig;

/// A description from outside the device stays untrusted until its owner has looked at its
/// keys and kept it, because a kept description decides which outputs count as change.
pub(crate) fn review_descriptor(
    ad: &mut AppData,
    descriptor: &ParsedMultisigDescriptor,
    checkpoint: &mut (impl FnMut() + ?Sized),
) {
    ad.signing.multisig.begin(MultisigConfig::from_descriptor(descriptor), MultisigDraft::Reviewing { own: false });
    let _ = crate::runtime::interactions::tx::resolve_loaded_cosigner_index(ad, checkpoint);
    ad.signing.multisig.creating.build_script();
    let own = belongs_to_active_wallet(ad, &ad.signing.multisig.creating, checkpoint);
    ad.signing.multisig.draft = MultisigDraft::Reviewing { own };
}

pub(crate) fn can_keep_reviewed_descriptor(ad: &AppData) -> bool {
    ad.signing.multisig.draft == MultisigDraft::Reviewing { own: true }
}

/// An address from outside the device names no wallet it knows, so it is only shown.
pub(crate) fn show_foreign_address(ad: &mut AppData, address: &[u8]) -> bool {
    let room = ad.qr.outgoing.buffer.len().min(ad.export.kpub_data.len());
    if address.len() > room || !offline_signer::address::validate_kaspa_address(address) {
        return false;
    }
    ad.export.kpub_data[..address.len()].copy_from_slice(address);
    ad.export.kpub_len = address.len();
    ad.qr.outgoing.buffer[..address.len()].copy_from_slice(address);
    ad.qr.outgoing.length = address.len();
    ad.qr.outgoing.frame = 0;
    ad.qr.outgoing.frame_count = 0;
    ad.qr.presentation.large = false;
    ad.signing.multisig.draft = MultisigDraft::Assembling;
    true
}

/// Persist the currently edited config back to its matching stored wallet.
/// Wallet identity is centralized in `MultisigConfig::same_wallet_as`.
pub(crate) fn persist_creating_config(ad: &mut AppData) -> bool {
    let Some(slot) = stored_slot_of(ad, &ad.signing.multisig.creating) else {
        return false;
    };
    ad.signing.multisig.store.configs[slot] = Some(ad.signing.multisig.creating.clone());
    true
}

/// The store slot already holding the same wallet as `config`.
fn stored_slot_of(ad: &AppData, config: &MultisigConfig) -> Option<usize> {
    ad.signing
        .multisig
        .store
        .configs
        .iter()
        .position(|stored| stored.as_ref().is_some_and(|wallet| wallet.same_wallet_as(config)))
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
    if !config.family.is_hd45() {
        return false;
    }
    let active = usize::from(ad.wallet.seeds.seed_mgr.active);
    if active >= crate::wallet::seed_manager::MAX_SLOTS {
        return false;
    }
    let Some(slot) = ad.wallet.seeds.seed_mgr.slots.get(active) else {
        return false;
    };
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
    let Ok(parts) = parts else {
        return false;
    };
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
        let Some(config) = config else {
            continue;
        };
        if ad.signing.multisig.descriptor_persisted[index] && belongs_to_active_wallet(ad, config, checkpoint) {
            indices[count] = u8::try_from(index).unwrap_or(u8::MAX);
            count += 1;
        }
    }
    (indices, u8::try_from(count).unwrap_or(u8::MAX))
}

pub(crate) fn save_creating_descriptor(ad: &mut AppData) -> Option<usize> {
    let slot = stored_slot_of(ad, &ad.signing.multisig.creating).or_else(|| ad.signing.multisig.store.find_free())?;
    ad.signing.multisig.store.configs[slot] = Some(ad.signing.multisig.creating.clone());
    ad.signing.multisig.descriptor_persisted[slot] = true;
    Some(slot)
}

pub(crate) fn delete_descriptor(ad: &mut AppData, slot: usize) -> bool {
    if slot >= ad.signing.multisig.store.configs.len() || !ad.signing.multisig.descriptor_persisted[slot] {
        return false;
    }
    ad.signing.multisig.store.configs[slot] = None;
    ad.signing.multisig.descriptor_persisted[slot] = false;
    true
}
