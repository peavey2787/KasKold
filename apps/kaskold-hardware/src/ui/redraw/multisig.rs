// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//! Screen redraw — multisig states.

use super::display;
use crate::runtime::{data::AppData, input::AppState};

pub(super) fn redraw(ad: &mut AppData, boot_display: &mut display::BootDisplay<'_>) -> bool {
    redraw_state(ad.navigation.app.state, ad, boot_display)
}

fn redraw_state(
    state: AppState,
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
) -> bool {
    redraw_menu(state, ad, boot_display)
        || redraw_creation(state, ad, boot_display)
        || redraw_output(state, ad, boot_display)
        || redraw_descriptor(state, ad, boot_display)
}

fn redraw_menu(state: AppState, ad: &mut AppData, boot_display: &mut display::BootDisplay<'_>) -> bool {
    match state {
        AppState::MultisigMenu => {
            boot_display.update_menu_content("MULTISIG", &ad.navigation.multisig_menu);
        }
        AppState::MultisigDescriptorsMenu => {
            boot_display.update_menu_content(
                "DESCRIPTORS",
                &ad.navigation.multisig_descriptors_menu,
            );
        }
        AppState::MultisigDescriptorList { action } => {
            let visible = visible_descriptors(ad);
            boot_display.draw_multisig_descriptor_list(action, &visible);
        }
        _ => return false,
    }
    true
}

fn visible_descriptors(
    ad: &AppData,
) -> alloc::vec::Vec<offline_signer::transaction::model::MultisigConfig> {
    ad.signing
        .multisig
        .descriptor_indices
        .iter()
        .take(ad.signing.multisig.descriptor_count as usize)
        .filter_map(|index| ad.signing.multisig.store.configs.get(usize::from(*index)))
        .cloned()
        .collect()
}

fn redraw_creation(state: AppState, ad: &mut AppData, boot_display: &mut display::BootDisplay<'_>) -> bool {
    match state {
        AppState::MultisigChooseMN => boot_display.draw_multisig_choose_mn(
            ad.signing.multisig.threshold,
            ad.signing.multisig.participant_count,
        ),
        AppState::MultisigAddKey { key_idx } => boot_display.draw_multisig_add_key(
            key_idx,
            ad.signing.multisig.creating.n,
            can_choose_wallet(ad),
        ),
        AppState::MultisigPickSeed { key_idx } => boot_display.draw_multisig_pick_seed(
            key_idx,
            ad.signing.multisig.creating.n,
            &ad.wallet.seeds.seed_mgr,
            ad.signing.multisig.scroll,
        ),
        _ => return false,
    }
    true
}

fn can_choose_wallet(ad: &AppData) -> bool {
    ad.wallet.seeds.seed_mgr.find_free().is_some()
        || ad
            .wallet
            .seeds
            .seed_mgr
            .slots
            .iter()
            .enumerate()
            .any(|(index, _)| ad.wallet.seeds.seed_mgr.slot_visible(index))
}

fn redraw_output(state: AppState, ad: &mut AppData, boot_display: &mut display::BootDisplay<'_>) -> bool {
    match state {
        AppState::MultisigShowAddress => draw_address(ad, boot_display),
        AppState::MultisigShowAddressQR => draw_address_qr(ad, boot_display),
        _ => return false,
    }
    true
}

fn draw_address(ad: &AppData, boot_display: &mut display::BootDisplay<'_>) {
    let mut label_buf = [0u8; 8];
    let label_len = ad.signing.multisig.creating.label(&mut label_buf);
    let label = core::str::from_utf8(&label_buf[..label_len]).unwrap_or("?-of-?");
    let script_hash = offline_signer::transaction::sighash::blake2b_hash(
        &ad.signing.multisig.creating.script[..ad.signing.multisig.creating.script_len],
    );
    let mut addr_buf = [0u8; offline_signer::address::MAX_ADDR_LEN];
    let addr = offline_signer::address::encode_address_str_for_network(
        &script_hash,
        offline_signer::address::AddressType::P2sh,
        ad.wallet.seeds.seed_mgr.network().kaspa_network(),
        &mut addr_buf,
    );
    boot_display.draw_multisig_result(
        label,
        addr,
        ad.signing.multisig.creating.addr_index,
        ad.signing.multisig.creating.v45.then_some((
            ad.signing.multisig.creating.cosigner_index,
            ad.signing.multisig.creating.chain,
        )),
    );
}

fn draw_address_qr(ad: &AppData, boot_display: &mut display::BootDisplay<'_>) {
    if ad.signing.multisig.creating.active && ad.signing.multisig.creating.script_len > 0 {
        draw_live_address_qr(ad, boot_display);
    } else if ad.qr.outgoing.length > 0 {
        boot_display.draw_qr_fullscreen(&ad.qr.outgoing.buffer[..ad.qr.outgoing.length]);
    } else {
        boot_display.draw_error_back_screen("No address to display");
    }
}

fn draw_live_address_qr(ad: &AppData, boot_display: &mut display::BootDisplay<'_>) {
    let script_hash = offline_signer::transaction::sighash::blake2b_hash(
        &ad.signing.multisig.creating.script[..ad.signing.multisig.creating.script_len],
    );
    let mut addr_buf = [0u8; offline_signer::address::MAX_ADDR_LEN];
    let addr_len = offline_signer::address::encode_address_for_network(
        &script_hash,
        offline_signer::address::AddressType::P2sh,
        ad.wallet.seeds.seed_mgr.network().kaspa_network(),
        &mut addr_buf,
    );
    boot_display.draw_qr_fullscreen(&addr_buf[..addr_len]);
}

fn redraw_descriptor(state: AppState, ad: &mut AppData, boot_display: &mut display::BootDisplay<'_>) -> bool {
    match state {
        AppState::MultisigDescriptor => draw_descriptor(ad, boot_display),
        AppState::MultisigSaveDescriptorAsk => boot_display.draw_yes_no_ask(
            "SAVE MULTISIG?",
            "Save this descriptor",
            "to device storage?",
        ),
        AppState::MultisigSaveAddrAsk => boot_display.draw_yes_no_ask(
            "SAVE ADDRESS?",
            "Save the multisig address",
            "to SD card?",
        ),
        _ => return false,
    }
    true
}

fn draw_descriptor(ad: &AppData, boot_display: &mut display::BootDisplay<'_>) {
    let mut label_buf = [0u8; 8];
    let label_len = ad.signing.multisig.creating.label(&mut label_buf);
    let label = core::str::from_utf8(&label_buf[..label_len]).unwrap_or("?-of-?");
    let mut xonly = [[0u8; 32]; offline_signer::transaction::model::MAX_MULTISIG_KEYS];
    let count = ad.signing.multisig.creating.n as usize;
    for (index, key) in xonly.iter_mut().take(count).enumerate() {
        key.copy_from_slice(&ad.signing.multisig.creating.cosigner_pubkeys[index][1..33]);
    }
    boot_display.draw_multisig_descriptor(ad.signing.multisig.creating.n, &xonly[..count], label);
}
