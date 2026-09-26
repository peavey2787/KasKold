use crate::hw::{display, touch};
use crate::runtime::data::AppData;
use crate::runtime::input::AppState;

use super::common::selected_item;

pub(super) fn handle_pure(
    ad: &mut AppData,
    list_zones: &[touch::TouchZone; 4],
    x: u16,
    y: u16,
    is_back: bool,
) -> Option<bool> {
    if !matches!(ad.navigation.app.state, AppState::MultisigMenu) {
        return None;
    }
    if is_back {
        return Some(route_back(ad));
    }
    match selected_item(&ad.navigation.multisig_menu, list_zones, x, y) {
        Some(0) => Some(start_multisig_creation(ad)),
        Some(1) => Some(crate::runtime::effects::menu_select(ad, 1)),
        Some(2) => None,
        Some(_) | None => Some(false),
    }
}

pub(super) fn handle(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    sd_card_type: &Option<crate::services::storage_device::SdCardType>,
    list_zones: &[touch::TouchZone; 4],
    x: u16,
    y: u16,
    is_back: bool,
) -> bool {
    match ad.navigation.app.state {
        AppState::MultisigMenu => handle_root_menu(
            ad,
            boot_display,
            delay,
            liveness,
            list_zones,
            x,
            y,
            is_back,
        ),
        AppState::MultisigDescriptorsMenu => handle_descriptors_menu(
            ad,
            boot_display,
            delay,
            liveness,
            i2c,
            sd_card_type,
            list_zones,
            x,
            y,
            is_back,
        ),
        _ => false,
    }
}

fn handle_root_menu(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    list_zones: &[touch::TouchZone; 4],
    x: u16,
    y: u16,
    is_back: bool,
) -> bool {
    if is_back {
        return route_back(ad);
    }
    match selected_item(&ad.navigation.multisig_menu, list_zones, x, y) {
        Some(0) => start_multisig_creation(ad),
        Some(1) => crate::runtime::effects::menu_select(ad, 1),
        Some(2) => export_multisig_kpub(ad, boot_display, delay, liveness),
        _ => false,
    }
}

fn handle_descriptors_menu(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    sd_card_type: &Option<crate::services::storage_device::SdCardType>,
    list_zones: &[touch::TouchZone; 4],
    x: u16,
    y: u16,
    is_back: bool,
) -> bool {
    if is_back {
        ad.navigation.multisig_descriptors_menu.reset();
        crate::runtime::effects::back(ad);
        return true;
    }
    let Some(item) = selected_item(&ad.navigation.multisig_descriptors_menu, list_zones, x, y) else {
        return false;
    };
    match item {
        0 | 1 | 3 => {
            let action = match item {
                0 => 0,
                1 => 1,
                _ => 2,
            };
            let (indices, count) = crate::runtime::interactions::multisig_config::visible_descriptor_indices(
                ad,
                liveness,
            );
            ad.signing.multisig.descriptor_indices = indices;
            ad.signing.multisig.descriptor_count = count;
            crate::runtime::effects::route(
                ad,
                crate::runtime::navigation::route!(MultisigDescriptorList { action }),
            );
            true
        }
        2 => {
            crate::runtime::interactions::sd::imports::import_menu::scan_multisig_descriptor_files(
                ad,
                boot_display,
                delay,
                i2c,
                sd_card_type.is_some(),
            );
            true
        }
        _ => false,
    }
}

pub(super) fn handle_without_storage(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    list_zones: &[touch::TouchZone; 4],
    x: u16,
    y: u16,
    is_back: bool,
) -> bool {
    if !matches!(ad.navigation.app.state, AppState::MultisigMenu) {
        return false;
    }
    handle_root_menu(ad, boot_display, delay, liveness, list_zones, x, y, is_back)
}

fn route_back(ad: &mut AppData) -> bool {
    ad.navigation.multisig_menu.reset();
    crate::runtime::effects::back(ad);
    true
}

fn start_multisig_creation(ad: &mut AppData) -> bool {
    ad.signing.multisig.threshold = 2;
    ad.signing.multisig.participant_count = 3;
    ad.signing.multisig.creating = offline_signer::transaction::model::MultisigConfig::new();
    ad.signing.multisig.descriptor_save_prompted = false;
    let _ = crate::runtime::effects::menu_select(ad, 0);
    true
}

fn export_multisig_kpub(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
) -> bool {
    let _ = crate::runtime::interactions::export::prepare_multisig_kpub_qr(
        ad,
        boot_display,
        delay,
        liveness,
    );
    true
}
