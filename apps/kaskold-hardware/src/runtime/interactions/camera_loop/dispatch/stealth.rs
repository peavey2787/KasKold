// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use crate::runtime::interactions::feedback::{show_rejection, ErrorSound};
use super::super::{AppData, display, sound};

fn display_response(response: &[u8], ad: &mut AppData) {
    if crate::runtime::qr_presentation::present_payload(
        ad,
        response,
        crate::runtime::navigation::continuation!(MainMenu),
    )
    .is_err()
    {
        crate::runtime::effects::home(ad);
        crate::runtime::effects::redraw(ad);
    }
}

pub(super) fn process(
    data: &[u8],
    len: usize,
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut (impl FnMut() + ?Sized),
) {
    let input = &data[..len.min(data.len())];
    let count = match offline_signer::stealth::validate_request(input) {
        Ok(count) => count,
        Err(error) => {
            log!("   → STLH: {}", error.message());
            sound::error();
            return;
        }
    };
    if ad.wallet.seeds.seed_mgr.active_slot().is_none() {
        show_rejection(boot_display, delay, "Load seed first", 1500, ErrorSound::Beep);
        crate::runtime::effects::redraw(ad);
        return;
    }

    log!("   → STLH: {} R values to scan", count);
    boot_display.draw_loading_screen("Stealth scanning...");
    let account_key = match crate::runtime::signing::derive_active_account_key_with_checkpoint(ad, liveness) {
        Ok(key) => key,
        Err(message) => {
            show_rejection(boot_display, delay, message, 1500, ErrorSound::Beep);
            crate::runtime::effects::redraw(ad);
            return;
        }
    };
    boot_display.update_progress_bar(30);
    let response = match offline_signer::stealth::scan_request_with_progress(
        &account_key,
        input,
        |completed, total| {
            boot_display.update_progress_bar(30 + (completed * 60 / total) as u8);
        },
    ) {
        Ok(response) => response,
        Err(error) => {
            show_rejection(boot_display, delay, error.message(), 1_500, ErrorSound::Beep);
            crate::runtime::effects::redraw(ad);
            return;
        }
    };
    boot_display.update_progress_bar(100);
    display_response(&response, ad);
}

#[cfg(feature = "workflow-test-auto")]
pub(super) fn workflow_validate_request(
    data: &[u8],
    length: usize,
) -> Result<usize, &'static str> {
    offline_signer::stealth::validate_request(&data[..length.min(data.len())])
        .map_err(offline_signer::stealth::StealthError::message)
}
