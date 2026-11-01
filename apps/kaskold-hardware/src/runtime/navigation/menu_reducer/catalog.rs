use super::super::ui_graph::{self, UiMenuItemSpec};
use crate::runtime::input::AppState;

pub(super) fn item_for(state: AppState, index: u8) -> Option<&'static UiMenuItemSpec> {
    primary(state)
        .or_else(|| wallet(state))
        .or_else(|| restore_and_signing(state))
        .and_then(|items| items.get(usize::from(index)))
}

fn primary(state: AppState) -> Option<&'static [UiMenuItemSpec]> {
    Some(match state {
        AppState::MainMenu => &ui_graph::MAIN_MENU_ITEMS,
        AppState::AdvancedMenu => &ui_graph::ADVANCED_MENU_ITEMS,
        #[cfg(feature = "provisioning-ui")]
        AppState::OwnerFirmwareMenu => &ui_graph::OWNER_FIRMWARE_MENU_ITEMS,
        AppState::SettingsMenu => &ui_graph::SETTINGS_MENU_ITEMS,
        AppState::SeedsMenu => &ui_graph::SEEDS_MENU_ITEMS,
        AppState::WalletBackupMethodsMenu => &ui_graph::WALLET_BACKUP_METHODS_MENU_ITEMS,
        AppState::WalletAdvancedMenu => &ui_graph::WALLET_ADVANCED_MENU_ITEMS,
        _ => return None,
    })
}

fn wallet(state: AppState) -> Option<&'static [UiMenuItemSpec]> {
    Some(match state {
        AppState::BackupRecoveryMenu => &ui_graph::BACKUP_RECOVERY_MENU_ITEMS,
        AppState::XprvExportMenu => &ui_graph::XPRV_EXPORT_MENU_ITEMS,
        AppState::MultisigMenu => &ui_graph::MULTISIG_MENU_ITEMS,
        AppState::MultisigDescriptorsMenu => &ui_graph::MULTISIG_DESCRIPTORS_MENU_ITEMS,
        AppState::ConfirmTx => &ui_graph::CONFIRM_TX_ITEMS,
        _ => return None,
    })
}

fn restore_and_signing(state: AppState) -> Option<&'static [UiMenuItemSpec]> {
    Some(match state {
        AppState::StorageSeedSourceChoice => &ui_graph::STORAGE_SEED_SOURCE_CHOICE_ITEMS,
        AppState::AdvancedRestoreMenu => &ui_graph::ADVANCED_RESTORE_MENU_ITEMS,
        AppState::SdImportMenu => &ui_graph::SD_IMPORT_MENU_ITEMS,
        _ => return None,
    })
}
