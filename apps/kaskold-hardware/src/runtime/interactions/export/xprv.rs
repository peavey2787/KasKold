// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

//! Extended-private-key export menu.

use super::{context::ExportStorageContext, TouchInput};
use crate::runtime::interactions::feedback::{card_inserted, show_rejection, ErrorSound};
use crate::{
    hw::{display, touch},
    runtime::interactions::menu_selection::{handle_paged_menu_touch, PagedMenuAction},
    runtime::{data::AppData, input::AppState},
};

pub(super) fn handle_pure(
    ad: &mut AppData,
    list_zones: &[touch::TouchZone; 4],
    page_up_zone: &touch::TouchZone,
    page_down_zone: &touch::TouchZone,
    input: TouchInput,
) -> Option<bool> {
    let TouchInput { x, y, is_back } = input;
    match ad.navigation.app.state {
        AppState::ExportXprv => Some(discard_xprv(ad)),
        AppState::XprvExportMenu if is_back => Some(leave_menu(ad)),
        AppState::XprvExportMenu => match handle_paged_menu_touch(
            &mut ad.navigation.xprv_export_menu,
            list_zones,
            page_up_zone,
            page_down_zone,
            x,
            y,
        ) {
            PagedMenuAction::None => Some(false),
            PagedMenuAction::PageChanged => Some(true),
            // Showing/deriving the xprv and SD export still require the narrow
            // hardware fallback.
            PagedMenuAction::Selected(_) => None,
        },
        _ => None,
    }
}

pub(super) fn handle(context: ExportStorageContext<'_, '_, '_>) -> Option<bool> {
    match context.ad.navigation.app.state {
        AppState::ExportXprv => Some(discard_xprv(context.ad)),
        AppState::XprvExportMenu => Some(handle_menu(context)),
        _ => None,
    }
}

fn discard_xprv(ad: &mut AppData) -> bool {
    shared_signer::bytes::zeroize_bytes(&mut ad.export.xprv_data);
    ad.export.xprv_len = 0;
    crate::runtime::effects::return_to(ad, crate::runtime::navigation::ReturnScope::KeyExport);
    true
}

fn leave_menu(ad: &mut AppData) -> bool {
    ad.navigation.xprv_export_menu.reset();
    crate::runtime::effects::return_to(ad, crate::runtime::navigation::ReturnScope::KeyExport);
    true
}

fn handle_menu(context: ExportStorageContext<'_, '_, '_>) -> bool {
    let ExportStorageContext {
        ad,
        boot_display,
        delay,
        liveness,
        i2c,
        list_zones,
        page_up_zone,
        page_down_zone,
        input,
    } = context;
    let TouchInput { x, y, is_back } = input;
    if is_back {
        return leave_menu(ad);
    }

    match handle_paged_menu_touch(&mut ad.navigation.xprv_export_menu, list_zones, page_up_zone, page_down_zone, x, y) {
        PagedMenuAction::None => false,
        PagedMenuAction::PageChanged => true,
        PagedMenuAction::Selected(0) => {
            show_xprv(ad, boot_display, delay, liveness);
            true
        }
        PagedMenuAction::Selected(1) => {
            if !card_inserted(boot_display, delay, i2c) {
                return true;
            }
            let next = crate::runtime::interactions::sd::scan_auto_increment(i2c, delay, b"XP", b"KAS");
            let name = crate::runtime::interactions::sd::format_auto_name(b"XP", next, b"KAS");
            ad.wallet.seeds.pp_input.reset();
            for byte in name[..8].iter().copied().take_while(|byte| *byte != b' ') {
                ad.wallet.seeds.pp_input.push_char(byte);
            }
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(SdXprvFilename));
            true
        }
        PagedMenuAction::Selected(_) => false,
    }
}

fn show_xprv(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
) {
    if crate::runtime::interactions::feedback::physical_presentation_enabled() {
        boot_display.draw_saving_screen("Deriving xprv...");
    }
    let mut xprv = [0u8; offline_signer::derivation::xpub::XPRV_MAX_LEN];
    let result = crate::runtime::signing::serialize_active_xprv_with_checkpoint(ad, &mut xprv, liveness);

    match result {
        Ok(length) => {
            ad.export.xprv_len = length;
            ad.export.xprv_data[..length].copy_from_slice(&xprv[..length]);
            let _ = crate::runtime::effects::menu_select(ad, 0);
        }
        Err(_) => {
            show_rejection(boot_display, delay, "xprv derivation failed", 2_000, ErrorSound::Silent);
        }
    }
    shared_signer::bytes::zeroize_bytes(&mut xprv);
}
