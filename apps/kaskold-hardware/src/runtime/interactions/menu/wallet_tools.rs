//! Wallet Advanced items that own the display or the SD bus.

use crate::hw::{display, touch};
use crate::runtime::interactions::{
    feedback::{show_rejection, ErrorSound},
    menu_selection::selected_visible_item,
    DeviceIo, TouchInput,
};
use crate::runtime::{data::AppData, input::AppState};

const HIDDEN_KEY_ADDRESS: u8 = 4;
const KPUB_TO_SD: u8 = 5;

/// The hardware-free menu layer hands these two items over to [`handle`].
pub(super) fn owns_item(item: u8) -> bool {
    matches!(item, HIDDEN_KEY_ADDRESS | KPUB_TO_SD)
}

pub(super) fn handle(
    ad: &mut AppData,
    io: &mut DeviceIo<'_, '_, '_>,
    list_zones: &[touch::TouchZone; 4],
    input: TouchInput,
) -> Option<bool> {
    if ad.navigation.app.state != AppState::WalletAdvancedMenu {
        return None;
    }
    let DeviceIo { boot_display, delay, i2c, liveness } = io;
    match selected_visible_item(&ad.navigation.production.wallet_advanced_menu, list_zones, input.x, input.y) {
        Some(HIDDEN_KEY_ADDRESS) => open_hidden_key_address(ad, boot_display, delay, liveness),
        Some(KPUB_TO_SD) => {
            crate::runtime::interactions::export::export_kpub_to_sd(ad, boot_display, delay, liveness, i2c)
        }
        _ => return Some(false),
    }
    Some(true)
}

/// Show the KIP-22 receive address at the index last picked in Receive. The
/// address hides its public key until the coins are spent.
pub(crate) fn open_hidden_key_address(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
) {
    if !ad.wallet.seeds.seed_loaded {
        show_rejection(boot_display, delay, "Load a seed first", 1500, ErrorSound::Silent);
        return;
    }
    boot_display.draw_saving_screen("Deriving address...");
    let address = match crate::runtime::signing::derive_active_kip22_address(ad, liveness) {
        Ok(address) => address,
        Err(message) => {
            show_rejection(boot_display, delay, message, 1500, ErrorSound::Silent);
            return;
        }
    };
    let length = address.len().min(ad.qr.scan.address.len());
    ad.qr.scan.address[..length].copy_from_slice(&address.as_bytes()[..length]);
    ad.qr.scan.address_length = length;
    ad.qr.scan.address_valid = true;
    ad.qr.scan.address_hidden_key = true;
    let _ = crate::runtime::effects::menu_select(ad, HIDDEN_KEY_ADDRESS);
}
