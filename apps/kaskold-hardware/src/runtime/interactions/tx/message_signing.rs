// tx controller — message-signing workflow facade.
use crate::controllers::TouchInput;
mod export;
mod preview;
mod result;
mod service;
#[cfg(feature = "workflow-test-auto")]
mod workflow;

use crate::{
    hw::display,
    runtime::{data::AppData, input::AppState},
};

pub(super) fn handle(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    input: TouchInput,
) -> Option<bool> {
    let redraw = match ad.navigation.app.state {
        AppState::SignMsgPreview => preview::handle(ad, boot_display, delay, liveness, input),
        AppState::SignMsgResult => result::handle(ad, boot_display, delay, i2c, input),
        AppState::SignMsgResultQr => result::close_qr(ad),
        _ => return None,
    };
    Some(redraw)
}

#[cfg(feature = "workflow-test-auto")]
pub(crate) fn workflow_sign_preview(ad: &mut AppData) -> bool {
    workflow::sign_preview(ad)
}
