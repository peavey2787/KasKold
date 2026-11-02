use crate::runtime::interactions::feedback::{show_rejection, ErrorSound};
// SD controller workflow: kpub.
use super::super::common::context::{SdFileListContext, SdIoContext};
use super::super::common::loaded_text::{open_address, open_descriptor, open_kpub};
use super::super::{
    display, run_encryption_prompt, run_filename_workflow, run_sd_file_list_context, sdcard, AppData,
    EncryptionPayload, EncryptionPromptWorkflow, FileListWorkflow, FilenameWorkflow, PromptDestination,
};
use crate::runtime::{data::TextFileKind, input::AppState};

pub(crate) fn handle_sd_kpub_file_list(context: SdFileListContext<'_, '_, '_>) -> bool {
    let back_state = if matches!(context.ad.storage.browser.text_import_kind, Some(TextFileKind::MultisigDescriptor)) {
        crate::runtime::navigation::continuation!(MultisigDescriptorsMenu)
    } else {
        crate::runtime::navigation::continuation!(SdImportMenu)
    };
    run_sd_file_list_context(
        context,
        FileListWorkflow { allow_delete: true, current_state: AppState::SdKpubFileList, back_state },
        load_selected_text,
    )
}

fn load_selected_text(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
) {
    let _ = &mut *i2c;
    boot_display.draw_loading_screen(import_label(ad.storage.browser.text_import_kind));
    let Ok(mut buffer) = crate::services::memory::zeroed_bytes(1024) else {
        show_rejection(boot_display, delay, "Not enough memory", 2000, ErrorSound::Silent);
        return;
    };
    match read_selected_file(ad, i2c, delay, &mut buffer) {
        Ok(length) => handle_import_payload(ad, boot_display, delay, liveness, &buffer[..length]),
        Err(error) => {
            log!("[SD-TXT] Read failed: {}", error);
            show_rejection(boot_display, delay, "SD read error", 2000, ErrorSound::Silent);
        }
    }
}

fn import_label(kind: Option<TextFileKind>) -> &'static str {
    match kind {
        Some(TextFileKind::Kpub) => "Reading kpub...",
        Some(TextFileKind::MultisigAddress) => "Reading address...",
        Some(TextFileKind::MultisigDescriptor) => "Reading descriptor...",
        None => "Reading file...",
    }
}

fn read_selected_file(
    ad: &AppData,
    i2c: &mut esp_hal::i2c::master::I2c<'_, esp_hal::Blocking>,
    delay: &mut esp_hal::delay::Delay,
    buffer: &mut [u8],
) -> Result<usize, &'static str> {
    let _ = &mut *i2c;
    sdcard::with_sd_card!(i2c, delay, |card| {
        let fat32 = sdcard::mount_fat32(card)?;
        let (entry, _, _) = sdcard::find_file_in_root(card, &fat32, &ad.storage.browser.selected_file)?;
        sdcard::read_file(card, &fat32, &entry, buffer)
    })
}

fn handle_import_payload(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    liveness: &mut dyn FnMut(),
    payload: &[u8],
) {
    let Some(kind) = ad.storage.browser.text_import_kind else {
        show_rejection(boot_display, delay, "Unsupported text import", 2000, ErrorSound::Silent);
        return;
    };
    if is_encrypted(payload) {
        stage_encrypted_import(ad, payload, kind);
        return;
    }
    match kind {
        TextFileKind::Kpub => open_kpub(ad, boot_display, delay, payload, None),
        TextFileKind::MultisigAddress => open_address(ad, boot_display, delay, payload, None),
        TextFileKind::MultisigDescriptor => open_descriptor(ad, boot_display, delay, liveness, payload, None),
    }
}

fn is_encrypted(payload: &[u8]) -> bool {
    payload.starts_with(&offline_signer::crypto::container_framing::TRANSPORT_CURRENT_MAGIC)
}

fn stage_encrypted_import(ad: &mut AppData, payload: &[u8], kind: TextFileKind) {
    if payload.len() > ad.qr.outgoing.buffer.len() {
        return;
    }
    ad.qr.outgoing.buffer[..payload.len()].copy_from_slice(payload);
    ad.qr.outgoing.length = payload.len();
    ad.storage.export_file.encrypted_operation = crate::runtime::data::EncryptedFileOperation::Import {
        kind: crate::runtime::data::EncryptedPayloadKind::Text(kind),
        back_state: crate::runtime::navigation::continuation!(SdKpubFileList),
    };
    ad.wallet.seeds.pp_input.reset();
    crate::runtime::effects::route(ad, crate::runtime::navigation::route!(SdKsptEncryptPass));
}

pub(crate) fn handle_sd_kpub_filename(ctx: SdIoContext<'_, '_, '_>) -> bool {
    run_filename_workflow(
        ctx,
        FilenameWorkflow {
            extension: *b"TXT",
            back_state: crate::runtime::navigation::continuation!(WalletAdvancedMenu),
            filename_state: crate::runtime::input::AppState::SdKpubFilename,
            next_state: crate::runtime::navigation::continuation!(SdKpubEncryptAsk),
        },
    )
}

pub(crate) fn handle_sd_kpub_encrypt_ask(ctx: SdIoContext<'_, '_, '_>) -> bool {
    run_encryption_prompt(
        ctx,
        EncryptionPromptWorkflow {
            back_state: crate::runtime::navigation::continuation!(SdKpubFilename),
            payload: EncryptionPayload::KpubExport { kind: TextFileKind::Kpub },
            password_back_state: crate::runtime::navigation::continuation!(SdKpubEncryptAsk),
            encrypted_success_state: crate::runtime::navigation::continuation!(WalletAdvancedMenu),
            plain_destination: PromptDestination::Route(crate::runtime::navigation::continuation!(WalletAdvancedMenu)),
            progress_message: "Saving kpub...",
            success_message: "kpub saved!",
        },
    )
}

#[cfg(feature = "workflow-test-auto")]
pub(crate) fn workflow_import_text_payload(
    ad: &mut AppData,
    boot_display: &mut display::BootDisplay<'_>,
    delay: &mut esp_hal::delay::Delay,
    kind: TextFileKind,
    payload: &[u8],
) {
    ad.storage.browser.text_import_kind = Some(kind);
    handle_import_payload(ad, boot_display, delay, &mut || {}, payload);
}
