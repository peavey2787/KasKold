//! Direct KasKold privacy-pairing request handling.
//!
//! Private derivation is shared with software Vaults through
//! `offline_signer::privacy_pairing`; this adapter owns hardware liveness,
//! progress, and QR presentation only.

use crate::hw::display;
use crate::runtime::data::{AppData, OutgoingQrPurpose};
use crate::runtime::interactions::feedback::{show_rejection, ErrorSound};

pub(super) fn process(
    data: &[u8],
    len: usize,
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut (impl FnMut() + ?Sized),
) {
    let input = &data[..len.min(data.len())];
    if shared_signer::pairing::parse_request(input).is_err() {
        show_rejection(boot_display, delay, "Invalid pairing request", 1_500, ErrorSound::Silent);
        return;
    }
    let account = match crate::runtime::signing::derive_active_account_key_with_checkpoint(ad, liveness) {
        Ok(account) => account,
        Err(_) => {
            show_rejection(boot_display, delay, "HD wallet required", 1_500, ErrorSound::Silent);
            return;
        }
    };
    boot_display.draw_loading_screen("Pairing...");
    let response = match offline_signer::privacy_pairing::respond_with_progress(
        &account,
        input,
        |completed, total| {
            liveness();
            if total != 0 {
                boot_display.update_progress_bar(((completed * 90) / total).min(90) as u8);
            }
        },
    ) {
        Ok(response) => response,
        Err(_) => {
            show_rejection(boot_display, delay, "Pairing derivation failed", 1_500, ErrorSound::Silent);
            return;
        }
    };
    if ad.qr.outgoing.ensure_len(response.len()).is_err() {
        show_rejection(boot_display, delay, "Pairing response too large", 1_500, ErrorSound::Silent);
        return;
    }
    ad.qr.outgoing.clear();
    ad.qr.outgoing.buffer[..response.len()].copy_from_slice(&response);
    ad.qr.outgoing.length = response.len();
    ad.qr.outgoing.purpose = OutgoingQrPurpose::None;
    ad.qr.outgoing.frame = 0;
    ad.qr.outgoing.frame_count = 0;
    ad.qr.outgoing.manual_frames = false;
    ad.qr.presentation.large = false;
    ad.qr.presentation.mode = 0;
    ad.qr.outgoing.close_state = Some(crate::runtime::navigation::continuation!(MainMenu));
    crate::runtime::effects::route(ad, crate::runtime::navigation::route!(ShowQR));
}
