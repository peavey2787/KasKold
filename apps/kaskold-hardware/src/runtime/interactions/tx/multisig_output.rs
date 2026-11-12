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
// tx controller — multisig output workflow.
use super::{AppData, RedrawFlag};
use crate::controllers::TouchInput;
use crate::runtime::data::MultisigDraft;
use crate::runtime::input::AppState;
use offline_signer::transaction::model::MultisigFamily;

fn draft_kept(ad: &AppData) -> bool {
    ad.signing.multisig.draft == MultisigDraft::Kept
}

fn build_multisig_descriptor(ad: &mut AppData) {
    let config = &ad.signing.multisig.creating;
    ad.qr.outgoing.length = config.write_descriptor(&mut ad.qr.outgoing.buffer).unwrap_or(0);
}

pub(super) fn handle(
    ad: &mut AppData,
    boot_display: &mut crate::hw::display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    mut persistent_wallet: Option<&mut crate::services::persistent_wallet::PersistentWallet<'_>>,
    input: TouchInput,
) -> Option<bool> {
    let redraw = match ad.navigation.app.state {
        AppState::MultisigShowAddress => handle_address(ad, input),
        AppState::MultisigShowAddressQR => handle_address_qr(ad),
        AppState::MultisigSaveDescriptorAsk => {
            handle_descriptor_save_prompt(ad, boot_display, delay, persistent_wallet.as_deref_mut(), input)
        }
        AppState::MultisigSaveAddrAsk => handle_save_prompt(ad, i2c, delay, input),
        AppState::MultisigDescriptor => {
            handle_descriptor(ad, boot_display, delay, i2c, persistent_wallet.as_deref_mut(), input)
        }
        AppState::MultisigDescriptorList { action } => {
            handle_descriptor_list(ad, boot_display, delay, i2c, persistent_wallet, action, input)
        }
        _ => return None,
    };
    let mut flag = RedrawFlag::default();
    flag.set(redraw);
    Some(flag.value())
}

fn handle_address(ad: &mut AppData, input: TouchInput) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        let route = if draft_kept(ad) {
            crate::runtime::navigation::route!(MultisigMenu)
        } else {
            crate::runtime::navigation::route!(SdImportMenu)
        };
        crate::runtime::effects::route(ad, route);
        return true;
    }
    if y < 195 {
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigShowAddressQR));
        return true;
    }
    if !draft_kept(ad) {
        return false;
    }
    let hd45 = ad.signing.multisig.creating.family.is_hd45();
    match x {
        62..=110 if hd45 => next_cosigner(ad),
        118..=158 if hd45 => toggle_chain(ad),
        166..=250 if hd45 => open_index_picker(ad),
        ..=90 => change_address_index(ad, -1),
        230.. => change_address_index(ad, 1),
        _ => open_index_picker(ad),
    }
    true
}

fn next_cosigner(ad: &mut AppData) {
    let count = ad.signing.multisig.creating.n;
    let MultisigFamily::Hd45 { cosigner_index, .. } = &mut ad.signing.multisig.creating.family else {
        return;
    };
    if count == 0 {
        return;
    }
    *cosigner_index = (*cosigner_index + 1) % count;
    rebuild_and_persist(ad);
}

fn toggle_chain(ad: &mut AppData) {
    let MultisigFamily::Hd45 { chain, .. } = &mut ad.signing.multisig.creating.family else {
        return;
    };
    *chain ^= 1;
    rebuild_and_persist(ad);
}

fn change_address_index(ad: &mut AppData, direction: i32) {
    let current = ad.signing.multisig.creating.addr_index;
    let next =
        if direction < 0 { current.saturating_sub(1) } else { current.saturating_add(1).min(u32::from(u16::MAX)) };
    if next == current {
        return;
    }
    ad.signing.multisig.creating.addr_index = next;
    rebuild_and_persist(ad);
}

fn open_index_picker(ad: &mut AppData) {
    ad.wallet.addresses.input_len = 0;
    ad.signing.multisig.picking_key = 255;
    crate::runtime::effects::route(ad, crate::runtime::navigation::route!(AddrIndexPicker));
}

fn rebuild_and_persist(ad: &mut AppData) {
    ad.signing.multisig.creating.build_script();
    crate::runtime::interactions::multisig_config::persist_creating_config(ad);
}

fn handle_address_qr(ad: &mut AppData) -> bool {
    let route = if draft_kept(ad) {
        if ad.signing.multisig.descriptor_save_prompted {
            crate::runtime::navigation::route!(MultisigSaveAddrAsk)
        } else {
            crate::runtime::navigation::route!(MultisigSaveDescriptorAsk)
        }
    } else {
        crate::runtime::navigation::route!(SdImportMenu)
    };
    crate::runtime::effects::route(ad, route);
    true
}

fn handle_descriptor_save_prompt(
    ad: &mut AppData,
    boot_display: &mut crate::hw::display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    persistent_wallet: Option<&mut crate::services::persistent_wallet::PersistentWallet<'_>>,
    input: TouchInput,
) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigShowAddress));
        return true;
    }
    if !(140..=185).contains(&y) {
        return false;
    }
    if (30..=155).contains(&x) {
        save_descriptor(ad, boot_display, delay, persistent_wallet);
        ad.signing.multisig.descriptor_save_prompted = true;
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigSaveAddrAsk));
        return true;
    }
    if (165..=290).contains(&x) {
        ad.signing.multisig.descriptor_save_prompted = true;
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigSaveAddrAsk));
        return true;
    }
    false
}

fn save_descriptor(
    ad: &mut AppData,
    boot_display: &mut crate::hw::display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    persistent_wallet: Option<&mut crate::services::persistent_wallet::PersistentWallet<'_>>,
) -> bool {
    let saved = crate::runtime::interactions::multisig_config::save_creating_descriptor(ad)
        .and(persistent_wallet)
        .is_some_and(|wallet| wallet.persist_multisig_store(ad).is_ok());
    if saved {
        crate::runtime::interactions::feedback::show_success(boot_display, delay, "Descriptor saved", 1_000);
    } else {
        crate::runtime::interactions::feedback::show_rejection(
            boot_display,
            delay,
            "Could not save descriptor",
            1_000,
            crate::runtime::interactions::feedback::ErrorSound::Beep,
        );
    }
    saved
}

fn keep_reviewed_descriptor(
    ad: &mut AppData,
    boot_display: &mut crate::hw::display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    persistent_wallet: Option<&mut crate::services::persistent_wallet::PersistentWallet<'_>>,
) {
    ad.signing.multisig.descriptor_save_prompted = true;
    if save_descriptor(ad, boot_display, delay, persistent_wallet) {
        ad.signing.multisig.draft = MultisigDraft::Kept;
    }
}

fn handle_save_prompt(
    ad: &mut AppData,
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    delay: &mut esp_hal::delay::Delay,
    input: TouchInput,
) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigShowAddress));
        return true;
    }
    if (30..=155).contains(&x) && (140..=185).contains(&y) {
        prepare_address_export(ad);
        prepare_filename(ad, i2c, delay, b"MS", crate::runtime::navigation::route!(SdMsAddrFilename));
        return true;
    }
    if (165..=290).contains(&x) && (140..=185).contains(&y) {
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigDescriptor));
        return true;
    }
    false
}

fn prepare_address_export(ad: &mut AppData) {
    let mut address = [0u8; offline_signer::address::MAX_ADDR_LEN];
    let length = ad.signing.multisig.creating.address(ad.wallet.seeds.seed_mgr.network().kaspa_network(), &mut address);
    ad.export.kpub_data[..length].copy_from_slice(&address[..length]);
    ad.export.kpub_len = length;
}

fn handle_descriptor(
    ad: &mut AppData,
    boot_display: &mut crate::hw::display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    persistent_wallet: Option<&mut crate::services::persistent_wallet::PersistentWallet<'_>>,
    input: TouchInput,
) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        if draft_kept(ad) {
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigShowAddress));
        } else {
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigDescriptorsMenu));
        }
        return true;
    }
    if !(190..=230).contains(&y) {
        return false;
    }
    if (170..=310).contains(&x) {
        if crate::runtime::interactions::multisig_config::can_keep_reviewed_descriptor(ad) {
            keep_reviewed_descriptor(ad, boot_display, delay, persistent_wallet);
        } else {
            build_multisig_descriptor(ad);
            prepare_filename(ad, i2c, delay, b"MD", crate::runtime::navigation::route!(SdMsDescFilename));
        }
        return true;
    }
    if (10..=150).contains(&x) {
        build_multisig_descriptor(ad);
        ad.qr.outgoing.frame_count = 0;
        ad.qr.outgoing.frame = 0;
        ad.qr.outgoing.manual_frames = false;
        ad.qr.outgoing.close_state = Some(crate::runtime::navigation::continuation!(MultisigDescriptor));
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(ShowQR));
        return true;
    }
    false
}

fn handle_descriptor_list(
    ad: &mut AppData,
    boot_display: &mut crate::hw::display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    persistent_wallet: Option<&mut crate::services::persistent_wallet::PersistentWallet<'_>>,
    action: u8,
    input: TouchInput,
) -> bool {
    let TouchInput { y, is_back, .. } = input;
    if is_back {
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigDescriptorsMenu));
        return true;
    }
    if ad.signing.multisig.descriptor_count == 0 {
        return false;
    }
    let row = if (68..=122).contains(&y) {
        0usize
    } else if (138..=192).contains(&y) {
        1usize
    } else {
        return false;
    };
    if row >= usize::from(ad.signing.multisig.descriptor_count) {
        return false;
    }
    let slot = usize::from(ad.signing.multisig.descriptor_indices[row]);
    let Some(config) = ad.signing.multisig.store.configs.get(slot).and_then(Option::as_ref).cloned() else {
        return false;
    };
    match action {
        0 => {
            ad.signing.multisig.begin(config, MultisigDraft::Kept);
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigDescriptor));
        }
        1 => {
            ad.signing.multisig.begin(config, MultisigDraft::Kept);
            build_multisig_descriptor(ad);
            prepare_filename(ad, i2c, delay, b"MD", crate::runtime::navigation::route!(SdMsDescFilename));
        }
        2 => {
            if !crate::runtime::interactions::multisig_config::delete_descriptor(ad, slot) {
                return false;
            }
            let persisted = persistent_wallet.is_some_and(|wallet| wallet.persist_multisig_store(ad).is_ok());
            if persisted {
                crate::runtime::interactions::feedback::show_success(boot_display, delay, "Descriptor deleted", 1_000);
            } else {
                crate::runtime::interactions::feedback::show_rejection(
                    boot_display,
                    delay,
                    "Delete was not persisted",
                    1_000,
                    crate::runtime::interactions::feedback::ErrorSound::Beep,
                );
            }
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigDescriptorsMenu));
        }
        _ => return false,
    }
    true
}

fn prepare_filename(
    ad: &mut AppData,
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    delay: &mut esp_hal::delay::Delay,
    prefix: &[u8; 2],
    next_route: crate::runtime::navigation::UiRoute,
) {
    let next = crate::runtime::interactions::sd::scan_auto_increment(i2c, delay, prefix, b"TXT");
    let name = crate::runtime::interactions::sd::format_auto_name(prefix, next, b"TXT");
    ad.storage.export_file.filename = name;
    ad.wallet.seeds.pp_input.reset();
    for &byte in name.iter().take(8).filter(|byte| **byte != b' ') {
        ad.wallet.seeds.pp_input.push_char(byte);
    }
    crate::runtime::effects::route(ad, next_route);
}
