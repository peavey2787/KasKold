use super::{AppData, BootDisplay, SdCardType};
use crate::runtime::input::AppState;

pub(super) fn redraw(ad: &AppData, display: &mut BootDisplay<'_>) -> bool {
    match ad.navigation.app.state {
        AppState::SdCardUnlockPassword => {
            display.draw_storage_secret_entry(&ad.wallet.seeds.pp_input, "SD PASSWORD", false, false);
            true
        }
        AppState::SdCardSettings => {
            draw_settings(display);
            true
        }
        _ => false,
    }
}

fn draw_settings(display: &mut BootDisplay<'_>) {
    let card = crate::services::storage_device::known_card();
    let description = match card {
        Some(SdCardType::SdV2Hc) => "SDHC (High Capacity)",
        Some(SdCardType::SdV2Sc) => "SD v2 (Standard)",
        Some(SdCardType::SdV1) => "SD v1",
        _ => "Unknown",
    };
    let card_locked = card.is_some() && crate::services::storage_device::card_is_known_locked();
    display.draw_sdcard_settings(card.is_some(), card_locked, description);
}
