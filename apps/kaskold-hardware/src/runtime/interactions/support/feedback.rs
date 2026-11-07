//! Shared transient controller feedback.

use crate::hw::display::BootDisplay;
use crate::services::audio as sound;

pub(crate) const NO_CARD_MESSAGE: &str = "Insert an SD card";

/// Look for a card now, so one put in after power-on is found. Without one,
/// say so and return false.
pub(crate) fn card_inserted(
    display: &mut BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
) -> bool {
    if crate::services::storage_device::detect_card(i2c, delay).is_some() {
        return true;
    }
    show_rejection(display, delay, NO_CARD_MESSAGE, 1_500, ErrorSound::Silent);
    false
}

/// Small mutable accumulator used by controller façades that delegate one
/// touch event across several focused handlers.
#[derive(Default)]
pub(crate) struct RedrawFlag(bool);

impl RedrawFlag {
    pub(crate) fn set(&mut self, value: bool) {
        self.0 = value;
    }

    pub(crate) fn value(self) -> bool {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ErrorSound {
    Silent,
    Beep,
}

/// Normal controller E2E intentionally isolates physical presentation; HIL and
/// production retain the actual LCD/audio/hold behavior.
pub(crate) const fn physical_presentation_enabled() -> bool {
    !cfg!(all(feature = "workflow-test-auto", not(feature = "workflow-runtime-auto")))
}

pub(crate) fn render_rejection(display: &mut BootDisplay<'_>, message: &str, sound_policy: ErrorSound) {
    if !physical_presentation_enabled() {
        return;
    }
    display.draw_transient_error_screen(message);
    if sound_policy == ErrorSound::Beep {
        sound::error();
    }
}

pub(crate) fn show_rejection(
    display: &mut BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    message: &str,
    hold_millis: u32,
    sound_policy: ErrorSound,
) {
    if !physical_presentation_enabled() {
        return;
    }
    render_rejection(display, message, sound_policy);
    crate::services::timing::pause(delay, hold_millis);
}

pub(crate) fn show_success(
    display: &mut BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    message: &str,
    hold_millis: u32,
) {
    if !physical_presentation_enabled() {
        return;
    }
    display.draw_success_screen(message);
    sound::success();
    crate::services::timing::pause(delay, hold_millis);
}
