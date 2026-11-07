use super::AppData;
use crate::controllers::TouchInput;
use crate::runtime::input::AppState;

pub(super) fn handle(ad: &mut AppData, input: TouchInput) -> Option<bool> {
    let TouchInput { x, y, is_back } = input;
    match ad.navigation.app.state {
        AppState::ShowQR => {
            advance_or_close(ad);
            Some(true)
        }
        AppState::Rejected => {
            let ok = crate::ui::layout::ERROR_OK_ZONE.contains(x, y);
            if is_back || ok {
                dismiss_rejection(ad);
            }
            Some(true)
        }
        _ => None,
    }
}

/// A cancelled transaction is over; a refused scan goes back to the camera for another try.
fn dismiss_rejection(ad: &mut AppData) {
    if ad.navigation.history.peek() == Some(AppState::ConfirmTx) {
        let initial_counts = ad.signing.anti_klepto.initial_sig_counts;
        if ad.signing.anti_klepto.phase != crate::runtime::data::AntiKleptoPhase::Inactive {
            crate::runtime::signing::rollback_added_signatures(ad, &initial_counts);
        }
        ad.signing.anti_klepto.reset();
        crate::runtime::effects::home(ad);
    } else if !crate::runtime::effects::back(ad) {
        crate::runtime::effects::home(ad);
    }
}

fn advance_or_close(ad: &mut AppData) {
    use crate::runtime::data::AntiKleptoPhase;
    if ad.qr.outgoing.manual_frames && ad.qr.outgoing.frame_count > 1 {
        let next = ad.qr.outgoing.frame + 1;
        if next < ad.qr.outgoing.frame_count {
            ad.qr.outgoing.frame = next;
            return;
        }
    }
    match ad.signing.anti_klepto.phase {
        AntiKleptoPhase::AwaitingReveal => {
            ad.qr.outgoing.frame_count = 0;
            ad.qr.outgoing.frame = 0;
            crate::runtime::effects::route(ad, crate::runtime::navigation::route!(AntiKleptoRevealGuide));
            return;
        }
        AntiKleptoPhase::FinalResponse => {
            ad.signing.anti_klepto.reset();
            ad.qr.outgoing.frame_count = 0;
            crate::runtime::effects::home(ad);
            return;
        }
        _ => {}
    }
    if ad.qr.outgoing.length == 0 {
        crate::runtime::effects::home(ad);
        return;
    }
    if let Some(target) = ad.qr.outgoing.close_state {
        ad.qr.outgoing.clear();
        crate::runtime::effects::continue_to(ad, target);
        return;
    }
    crate::runtime::effects::route(ad, crate::runtime::navigation::route!(ShowQrPopup));
}
