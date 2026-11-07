use crate::controllers::TouchInput;
use crate::hw::{display, touch};
use crate::runtime::{data::AppData, input::AppState};

pub(super) fn handle(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    list_zones: &[touch::TouchZone; 4],
    input: TouchInput,
) -> Option<bool> {
    match ad.navigation.app.state {
        AppState::MultisigMenu => {
            Some(super::multisig::handle_without_storage(ad, boot_display, delay, liveness, list_zones, input))
        }
        _ => None,
    }
}
