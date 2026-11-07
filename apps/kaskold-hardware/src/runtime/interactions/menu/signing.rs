// KasKold — Air-gapped offline signing device for Kaspa
// Copyright (C) 2025-2026 KasSigner Project (kassigner@proton.me)
// License: GPL-3.0

use crate::controllers::TouchInput;
use crate::hw::{display, touch};
use crate::runtime::data::AppData;
use crate::runtime::input::AppState;

mod common;
mod feedback;
pub(super) mod multisig;

/// Route signing-menu choices that only mutate navigation/application state.
pub(super) fn handle_pure(ad: &mut AppData, list_zones: &[touch::TouchZone; 4], input: TouchInput) -> Option<bool> {
    multisig::handle_pure(ad, list_zones, input)
}

pub(super) fn handle(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    list_zones: &[touch::TouchZone; 4],
    input: TouchInput,
) -> Option<bool> {
    match ad.navigation.app.state {
        AppState::MultisigMenu | AppState::MultisigDescriptorsMenu => {
            Some(multisig::handle(ad, boot_display, delay, liveness, i2c, list_zones, input))
        }
        _ => None,
    }
}

pub(super) fn handle_feedback(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    list_zones: &[touch::TouchZone; 4],
    input: TouchInput,
) -> Option<bool> {
    feedback::handle(ad, boot_display, delay, liveness, list_zones, input)
}
