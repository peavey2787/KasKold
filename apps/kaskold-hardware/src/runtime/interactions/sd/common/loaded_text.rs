//! Text files from a card open on the same screens whether they arrived plain or decrypted.

use super::shared::parse_descriptor;
use crate::hw::display::BootDisplay;
use crate::runtime::data::AppData;
use crate::runtime::interactions::feedback::{show_rejection, show_success, ErrorSound};
use crate::runtime::navigation::ContinuationRoute;

pub(in crate::runtime::interactions::sd) fn looks_like_address(text: &[u8]) -> bool {
    text.starts_with(b"kaspa:") || text.starts_with(b"kaspatest:")
}

pub(in crate::runtime::interactions::sd) fn open_descriptor(
    ad: &mut AppData,
    boot_display: &mut BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    text: &[u8],
    on_refusal: Option<ContinuationRoute>,
) {
    let Some(descriptor) = parse_descriptor(text) else {
        refuse(ad, boot_display, delay, "Bad descriptor", on_refusal);
        return;
    };
    crate::runtime::interactions::multisig_config::review_descriptor(ad, &descriptor, liveness);
    show_success(boot_display, delay, "Descriptor loaded!", 1_000);
    crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigDescriptor));
}

pub(in crate::runtime::interactions::sd) fn open_address(
    ad: &mut AppData,
    boot_display: &mut BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    text: &[u8],
    on_refusal: Option<ContinuationRoute>,
) {
    if !crate::runtime::interactions::multisig_config::show_foreign_address(ad, text) {
        refuse(ad, boot_display, delay, "Not a valid address", on_refusal);
        return;
    }
    show_success(boot_display, delay, "Address loaded!", 1_000);
    crate::runtime::effects::route(ad, crate::runtime::navigation::route!(MultisigShowAddress));
}

pub(in crate::runtime::interactions::sd) fn open_kpub(
    ad: &mut AppData,
    boot_display: &mut BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    text: &[u8],
    on_refusal: Option<ContinuationRoute>,
) {
    let mut canonical = [0u8; offline_signer::derivation::xpub::KPUB_MAX_LEN];
    let Ok(length) = offline_signer::derivation::xpub::normalize_kpub_text(text, &mut canonical) else {
        refuse(ad, boot_display, delay, "Not a valid kpub", on_refusal);
        return;
    };
    ad.export.kpub_data[..length].copy_from_slice(&canonical[..length]);
    ad.export.kpub_len = length;
    show_success(boot_display, delay, "Kpub loaded!", 1_000);
    crate::runtime::effects::route(ad, crate::runtime::navigation::route!(ExportKpub));
}

fn refuse(
    ad: &mut AppData,
    boot_display: &mut BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    message: &str,
    on_refusal: Option<ContinuationRoute>,
) {
    show_rejection(boot_display, delay, message, 2_000, ErrorSound::Silent);
    if let Some(route) = on_refusal {
        crate::runtime::effects::continue_to(ad, route);
    }
}
