use super::super::super::common::loaded_text::{looks_like_address, open_address, open_descriptor, open_kpub};
use super::super::super::{display, AppData};
use crate::runtime::{
    data::{EncryptedPayloadKind, TextFileKind},
    navigation::ContinuationRoute,
};
use kaskold_protocol::wire::multisig_descriptor::looks_like_descriptor;

pub(super) fn route(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    kind: EncryptedPayloadKind,
    failure_state: ContinuationRoute,
    plaintext: &[u8],
) {
    match kind {
        EncryptedPayloadKind::Text(TextFileKind::MultisigDescriptor) => {
            open_descriptor(ad, boot_display, delay, liveness, plaintext, Some(failure_state))
        }
        EncryptedPayloadKind::Text(TextFileKind::Kpub) => {
            open_kpub(ad, boot_display, delay, plaintext, Some(failure_state))
        }
        EncryptedPayloadKind::Text(TextFileKind::MultisigAddress) => {
            open_address(ad, boot_display, delay, plaintext, Some(failure_state))
        }
        EncryptedPayloadKind::Transaction => route_transaction_import(ad, boot_display, delay, liveness, plaintext),
    }
}

fn route_transaction_import(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    plaintext: &[u8],
) {
    let list = crate::runtime::navigation::continuation!(SdKsptFileList);
    if looks_like_descriptor(plaintext) {
        open_descriptor(ad, boot_display, delay, liveness, plaintext, Some(list));
    } else if looks_like_address(plaintext) {
        open_address(ad, boot_display, delay, plaintext, Some(list));
    } else if plaintext.starts_with(kaskold_protocol::wire::pskt_envelope::PSKT_MAGIC) {
        crate::runtime::interactions::tx::load_standard_transaction_with_checkpoint(plaintext, ad, liveness);
    } else {
        crate::runtime::interactions::tx::load_compact_transaction_with_checkpoint(plaintext, ad, liveness);
    }
}
