//! Seed import workflow façade.
use super::{display, AppData};
use crate::controllers::TouchInput;

mod private_key;
mod restore;
mod word_entry;

pub(super) fn handle(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    input: TouchInput,
) -> Option<bool> {
    let redraw = match ad.navigation.app.state {
        crate::runtime::input::AppState::ImportPrivKey => private_key::handle(ad, boot_display, delay, input),
        crate::runtime::input::AppState::RestoreWord { word_idx } => {
            restore::handle(ad, boot_display, delay, input, word_idx)
        }
        _ => return None,
    };
    Some(redraw)
}
