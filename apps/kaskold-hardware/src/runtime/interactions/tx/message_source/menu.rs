use super::super::{display, AppData};
use crate::controllers::TouchInput;
use crate::runtime::interactions::text_files::{self, TextFileScanWorkflow};

pub(super) fn handle(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    input: TouchInput,
) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        crate::runtime::effects::return_to(ad, crate::runtime::navigation::ReturnScope::SigningTool);
    } else if (40..280).contains(&x) && (68..112).contains(&y) {
        ad.wallet.seeds.pp_input.reset();
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(SignMsgType));
    } else if (40..280).contains(&x) && (114..158).contains(&y) {
        ad.signing.message.payload_len = 0;
        crate::runtime::effects::route(ad, crate::runtime::navigation::route!(SignMsgScan));
    } else if (40..280).contains(&x) && (160..204).contains(&y) {
        let maximum_bytes = u32::try_from(ad.signing.message.payload.len()).unwrap_or(u32::MAX);
        return text_files::scan(
            ad,
            boot_display,
            delay,
            i2c,
            TextFileScanWorkflow {
                maximum_bytes,
                next_state: crate::runtime::navigation::continuation!(SignMsgFile),
                empty_message: "No .TXT files on SD",
            },
        );
    } else {
        return false;
    }
    true
}
