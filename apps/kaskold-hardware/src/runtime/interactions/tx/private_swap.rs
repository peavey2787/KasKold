use crate::controllers::TouchInput;
use crate::{
    hw::display,
    runtime::interactions::feedback::{show_rejection, ErrorSound},
    runtime::{
        data::{AppData, PrivateSwapAction, PrivateSwapPhase},
        input::AppState,
    },
};

pub(super) fn handle(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    input: TouchInput,
) -> Option<bool> {
    match ad.navigation.app.state {
        AppState::PrivateSwapReview => Some(handle_review(ad, boot_display, delay, liveness, input)),
        AppState::PrivateSwapKeyResult => Some(handle_key(ad, boot_display, delay, input)),
        AppState::PrivateSwapResult => Some(handle_result(ad, boot_display, delay, input)),
        _ => None,
    }
}

fn handle_review(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    input: TouchInput,
) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        reset_to_home(ad);
        return true;
    }
    if !(178..=224).contains(&y) || !(55..=265).contains(&x) {
        return false;
    }

    boot_display.draw_saving_screen("Private Swap authorization...");
    let PrivateSwapPhase::Prepared(action) = ad.signing.private_swap.phase else {
        return false;
    };
    let result = match action {
        PrivateSwapAction::Bind => crate::services::private_swap::complete_binding(ad, liveness),
        PrivateSwapAction::PreSign => crate::services::private_swap::begin_presign(ad),
        PrivateSwapAction::Complete => crate::services::private_swap::complete_claim(ad, liveness),
    };
    match result {
        Ok(()) => {
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(PrivateSwapResult));
        }
        Err(error) => {
            ad.signing.private_swap.reset();
            show_rejection(boot_display, delay, error.message(), 1800, ErrorSound::Beep);
            crate::runtime::effects::home(ad);
        }
    }
    true
}

fn handle_key(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    input: TouchInput,
) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        reset_to_home(ad);
        return true;
    }
    if (178..=224).contains(&y) && (70..=250).contains(&x) {
        show_response(ad, boot_display, delay, crate::runtime::navigation::continuation!(PrivateSwapKeyResult));
        return true;
    }
    false
}

fn handle_result(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    input: TouchInput,
) -> bool {
    let TouchInput { x, y, is_back } = input;
    if is_back {
        reset_to_home(ad);
        return true;
    }
    if !(178..=224).contains(&y) || !(55..=265).contains(&x) {
        return false;
    }
    match ad.signing.private_swap.phase {
        PrivateSwapPhase::AwaitingReveal { nonce_shown: true } => {
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(ScanQR));
            return true;
        }
        PrivateSwapPhase::AwaitingReveal { nonce_shown: false } => {
            ad.signing.private_swap.phase = PrivateSwapPhase::AwaitingReveal { nonce_shown: true };
        }
        _ => {}
    }
    show_response(ad, boot_display, delay, crate::runtime::navigation::continuation!(PrivateSwapResult));
    true
}

/// A response is larger than one QR code, so it leaves through the frame sequence.
fn show_response(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    close_state: crate::runtime::navigation::ContinuationRoute,
) {
    let response = ad.signing.private_swap.response;
    let shown = &response[..ad.signing.private_swap.response_len];
    if crate::runtime::qr_presentation::present_payload(ad, shown, close_state).is_err() {
        show_rejection(boot_display, delay, "Not enough memory", 1800, ErrorSound::Beep);
    }
}

fn reset_to_home(ad: &mut AppData) {
    ad.signing.private_swap.reset();
    crate::runtime::effects::home(ad);
}
