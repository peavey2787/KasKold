use crate::hw::{display, touch};
use crate::runtime::{data::AppData, input::AppState};

pub(super) fn handle(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    list_zones: &[touch::TouchZone; 4],
    x: u16,
    y: u16,
    is_back: bool,
) -> Option<bool> {
    match ad.navigation.app.state {
        AppState::SingleSigMenu => Some(super::single_sig::handle(
            ad,
            boot_display,
            delay,
            liveness,
            list_zones,
            x,
            y,
            is_back,
        )),
        AppState::MultisigMenu => Some(super::multisig::handle_without_storage(
            ad,
            boot_display,
            delay,
            liveness,
            list_zones,
            x,
            y,
            is_back,
        )),
        _ => None,
    }
}
