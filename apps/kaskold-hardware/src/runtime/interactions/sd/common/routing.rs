use crate::runtime::input::AppState;

use super::super::{
    backup::xprv,
    exports::{kpub, kspt_export, multisig, qr, signature},
    imports::{file_browser, import_menu, kspt_import},
};
use super::{
    context::{SdActionContext, SdFileListContext, SdImportMenuContext, SdIoContext, SdListContext, SdTouchContext},
    overwrite,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SdRouteGroup {
    Signature,
    File,
    SeedBackup,
    Xprv,
    Kspt,
    Kpub,
    Multisig,
    Action,
    WalletBackupList,
    FileList,
    ImportMenu,
}

/// Handle touch events for live SD import/export and device-storage screens.
#[inline(never)]
pub fn handle_sd_touch(context: SdTouchContext<'_, '_, '_>) -> Option<bool> {
    let state = context.ad.navigation.app.state;
    let group = route_group(state)?;
    let SdTouchContext {
        ad,
        boot_display,
        delay,
        liveness,
        i2c,
        backup_device,
        list_zones,
        page_up_zone,
        page_down_zone,
        input,
    } = context;
    Some(match group {
        SdRouteGroup::Signature => {
            handle_signature_io(SdIoContext { ad, boot_display, delay, liveness, i2c, backup_device, input })
        }
        SdRouteGroup::File => {
            handle_file_io(SdIoContext { ad, boot_display, delay, liveness, i2c, backup_device, input })
        }
        SdRouteGroup::SeedBackup => {
            handle_seed_backup_io(SdIoContext { ad, boot_display, delay, liveness, i2c, backup_device, input })
        }
        SdRouteGroup::Xprv => {
            handle_xprv_io(SdIoContext { ad, boot_display, delay, liveness, i2c, backup_device, input })
        }
        SdRouteGroup::Kspt => {
            handle_kspt_io(SdIoContext { ad, boot_display, delay, liveness, i2c, backup_device, input })
        }
        SdRouteGroup::Kpub => {
            handle_kpub_io(SdIoContext { ad, boot_display, delay, liveness, i2c, backup_device, input })
        }
        SdRouteGroup::Multisig => {
            handle_multisig_io(SdIoContext { ad, boot_display, delay, liveness, i2c, backup_device, input })
        }
        SdRouteGroup::Action => qr::handle_show_qr_mode_choice(SdActionContext { ad, input }),
        SdRouteGroup::WalletBackupList => {
            super::super::backup::import::handle_wallet_backup_file_list(SdListContext { ad, list_zones, input })
        }
        SdRouteGroup::FileList => {
            handle_file_list(SdFileListContext { ad, boot_display, delay, liveness, i2c, list_zones, input })
        }
        SdRouteGroup::ImportMenu => import_menu::handle_sd_import_menu(SdImportMenuContext {
            ad,
            boot_display,
            delay,
            i2c,
            list_zones,
            page_up_zone,
            page_down_zone,
            input,
        }),
    })
}

fn route_group(state: AppState) -> Option<SdRouteGroup> {
    if state == AppState::SdSigFilename {
        Some(SdRouteGroup::Signature)
    } else if matches!(state, AppState::SdDeleteConfirm | AppState::ShowQrPopup | AppState::SdOverwriteWarning) {
        Some(SdRouteGroup::File)
    } else if matches!(
        state,
        AppState::SdBackupWarning
            | AppState::SdSeedFilename
            | AppState::SdSeedExportPassphrase
            | AppState::SdWalletBackupImportPassphrase
    ) {
        Some(SdRouteGroup::SeedBackup)
    } else if matches!(state, AppState::SdXprvFilename | AppState::SdXprvExportPassphrase) {
        Some(SdRouteGroup::Xprv)
    } else if matches!(state, AppState::SdKsptFilename | AppState::SdKsptEncryptAsk | AppState::SdKsptEncryptPass) {
        Some(SdRouteGroup::Kspt)
    } else if matches!(state, AppState::SdKpubFilename | AppState::SdKpubEncryptAsk) {
        Some(SdRouteGroup::Kpub)
    } else if matches!(
        state,
        AppState::SdMsAddrFilename
            | AppState::SdMsAddrEncryptAsk
            | AppState::SdMsDescFilename
            | AppState::SdMsDescEncryptAsk
    ) {
        Some(SdRouteGroup::Multisig)
    } else if state == AppState::ShowQrModeChoice {
        Some(SdRouteGroup::Action)
    } else if state == AppState::SdWalletBackupFileList {
        Some(SdRouteGroup::WalletBackupList)
    } else if matches!(state, AppState::SdFileList | AppState::SdKsptFileList | AppState::SdKpubFileList) {
        Some(SdRouteGroup::FileList)
    } else if state == AppState::SdImportMenu {
        Some(SdRouteGroup::ImportMenu)
    } else {
        None
    }
}

fn handle_signature_io(context: SdIoContext<'_, '_, '_>) -> bool {
    match context.ad.navigation.app.state {
        AppState::SdSigFilename => signature::handle_sd_sig_filename(context),
        _ => false,
    }
}

fn handle_file_io(context: SdIoContext<'_, '_, '_>) -> bool {
    match context.ad.navigation.app.state {
        AppState::SdDeleteConfirm => file_browser::handle_sd_delete_confirm(context),
        AppState::ShowQrPopup => qr::handle_show_qr_popup(context),
        AppState::SdOverwriteWarning => overwrite::handle_sd_overwrite_warning(context),
        _ => false,
    }
}

fn handle_seed_backup_io(context: SdIoContext<'_, '_, '_>) -> bool {
    match context.ad.navigation.app.state {
        AppState::SdBackupWarning => super::super::backup::seed::handle_seed_backup_warning(context),
        AppState::SdSeedFilename => super::super::backup::seed::handle_seed_backup_filename(context),
        AppState::SdSeedExportPassphrase => super::super::backup::seed::handle_seed_backup_export_passphrase(context),
        AppState::SdWalletBackupImportPassphrase => {
            super::super::backup::import::handle_wallet_backup_import_passphrase(context)
        }
        _ => false,
    }
}

fn handle_xprv_io(context: SdIoContext<'_, '_, '_>) -> bool {
    match context.ad.navigation.app.state {
        AppState::SdXprvFilename => xprv::handle_sd_xprv_filename(context),
        AppState::SdXprvExportPassphrase => xprv::handle_sd_xprv_export_passphrase(context),
        _ => false,
    }
}

fn handle_kspt_io(context: SdIoContext<'_, '_, '_>) -> bool {
    match context.ad.navigation.app.state {
        AppState::SdKsptFilename => kspt_export::handle_sd_kspt_filename(context),
        AppState::SdKsptEncryptAsk => kspt_export::handle_sd_kspt_encrypt_ask(context),
        AppState::SdKsptEncryptPass => kspt_export::handle_sd_kspt_encrypt_pass(context),
        _ => false,
    }
}

fn handle_kpub_io(context: SdIoContext<'_, '_, '_>) -> bool {
    match context.ad.navigation.app.state {
        AppState::SdKpubFilename => kpub::handle_sd_kpub_filename(context),
        AppState::SdKpubEncryptAsk => kpub::handle_sd_kpub_encrypt_ask(context),
        _ => false,
    }
}

fn handle_multisig_io(context: SdIoContext<'_, '_, '_>) -> bool {
    match context.ad.navigation.app.state {
        AppState::SdMsAddrFilename => multisig::handle_sd_ms_addr_filename(context),
        AppState::SdMsAddrEncryptAsk => multisig::handle_sd_ms_addr_encrypt_ask(context),
        AppState::SdMsDescFilename => multisig::handle_sd_ms_desc_filename(context),
        AppState::SdMsDescEncryptAsk => multisig::handle_sd_ms_desc_encrypt_ask(context),
        _ => false,
    }
}

fn handle_file_list(context: SdFileListContext<'_, '_, '_>) -> bool {
    match context.ad.navigation.app.state {
        AppState::SdFileList => file_browser::handle_sd_file_list(context),
        AppState::SdKsptFileList => kspt_import::handle_sd_kspt_file_list(context),
        AppState::SdKpubFileList => kpub::handle_sd_kpub_file_list(context),
        _ => false,
    }
}
