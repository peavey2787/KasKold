//! Finds the photos on the card for the backup and restore pickers.

use super::display;
use crate::runtime::data::AppData;
use crate::runtime::interactions::feedback::{show_rejection, ErrorSound};
use crate::services::storage_files;

pub(super) struct PhotoList<'list> {
    pub(super) names: &'list mut [[u8; 11]; 8],
    pub(super) display_names: &'list mut [[u8; 32]; 8],
    pub(super) display_lens: &'list mut [u8; 8],
    pub(super) count: &'list mut u8,
}

/// Fills `list` and says whether a person has anything to pick from.
pub(super) fn scan(
    list: PhotoList<'_>,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
) -> bool {
    boot_display.draw_loading_screen("Scanning SD...");
    boot_display.update_progress_bar(50);
    crate::services::timing::pause(delay, 50);
    let result =
        storage_files::scan_jpeg_files(list.names, list.display_names, list.display_lens, list.count, delay, i2c);
    if matches!(result, Ok(count) if count > 0) {
        return true;
    }
    show_rejection(boot_display, delay, "No .JPG files on SD", 2_000, ErrorSound::Beep);
    false
}

/// Opens the restore picker only on photos that exist.
pub(crate) fn open_restore_picker(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    card_present: bool,
) {
    if !card_present {
        show_rejection(
            boot_display,
            delay,
            crate::runtime::interactions::feedback::NO_CARD_MESSAGE,
            1_500,
            ErrorSound::Silent,
        );
        return;
    }
    let import = &mut ad.stego.import;
    let list = PhotoList {
        names: &mut import.jpeg_names,
        display_names: &mut import.jpeg_display,
        display_lens: &mut import.jpeg_display_lens,
        count: &mut import.jpeg_count,
    };
    if scan(list, boot_display, delay, i2c) {
        ad.stego.import.jpeg_selected = 0;
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(StegoImportPick));
    }
}
