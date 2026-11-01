use crate::runtime::input::AppState;

pub(super) fn destination(state: AppState, index: u8) -> Option<AppState> {
    root_settings(state, index)
        .or_else(|| wallet_a(state, index))
        .or_else(|| wallet_b(state, index))
        .or_else(|| wallet_advanced(state, index))
        .or_else(|| signing(state, index))
        .or_else(|| export(state, index))
        .or_else(|| restore(state, index))
        .or_else(|| imports(state, index))
}

fn root_settings(state: AppState, index: u8) -> Option<AppState> {
    use AppState::*;
    Some(match (state, index) {
        (MainMenu, 0) => SeedsMenu,
        (MainMenu, 1) => ScanQR,
        (MainMenu, 2) => SeedsMenu,
        (MainMenu, 3) => SettingsMenu,
        (SettingsMenu, 0) => DisplaySettings,
        #[cfg(feature = "m5stack")]
        (SettingsMenu, 1) => AudioSettings,
        (SettingsMenu, 2) => AdvancedFeatures,
        (SettingsMenu, 3) => SdCardSettings,
        (SettingsMenu, 4) => AdvancedMenu,
        (SettingsMenu, 5) => About,
        _ => return None,
    })
}

fn wallet_a(state: AppState, index: u8) -> Option<AppState> {
    use AppState::*;
    Some(match (state, index) {
        (AdvancedMenu, 0) => FirmwareUpdateReady,
        (AdvancedMenu, 1) => FactoryResetWarning,
        #[cfg(feature = "provisioning-ui")]
        (AdvancedMenu, 2) => OwnerFirmwareMenu,
        #[cfg(feature = "provisioning-ui")]
        (AdvancedMenu, 3) => PopItPrompt,
        #[cfg(feature = "provisioning-ui")]
        (OwnerFirmwareMenu, 0) => OwnerKeyWarning,
        #[cfg(feature = "provisioning-ui")]
        (OwnerFirmwareMenu, 1) => OwnerInstallWarning,
        (SeedsMenu, 0) => ShowAddress,
        (SeedsMenu, 1) => WalletBackupMethodsMenu,
        (SeedsMenu, 2) => SdImportMenu,
        (SeedsMenu, 3) => WalletDetails,
        (SeedsMenu, 4) => SeedList,
        (SeedsMenu, 5) => MultisigMenu,
        (SeedsMenu, 6) => WalletAdvancedMenu,
        _ => return None,
    })
}

fn wallet_b(state: AppState, index: u8) -> Option<AppState> {
    use AppState::*;
    Some(match (state, index) {
        (WalletBackupMethodsMenu, 0) => SeedBackup { word_idx: 0 },
        (WalletBackupMethodsMenu, 1) => ExportSeedQR,
        (WalletBackupMethodsMenu, 2) => SdBackupWarning,
        (WalletBackupMethodsMenu, 3) => BackupRecoveryMenu,
        (BackupRecoveryMenu, 0) => ExportCompactSeedQR,
        (BackupRecoveryMenu, 1) => ExportPlainWordsQR,
        (BackupRecoveryMenu, 2) => StegoModeSelect,
        (BackupRecoveryMenu, 3) => XprvExportMenu,
        (BackupRecoveryMenu, 4) => ExportPrivKeyIndex,
        _ => return None,
    })
}

fn wallet_advanced(state: AppState, index: u8) -> Option<AppState> {
    use AppState::*;
    Some(match (state, index) {
        (WalletAdvancedMenu, 0) => ChooseWordCount { action: 4 },
        (WalletAdvancedMenu, 1) => SignMsgChoice,
        (WalletAdvancedMenu, 2) => CommitRevealType,
        (WalletAdvancedMenu, 3) => DecryptSecretScan,
        (WalletAdvancedMenu, 4) => ShowAddress,
        (WalletAdvancedMenu, 5) => SdKpubFilename,
        _ => return None,
    })
}

fn signing(state: AppState, index: u8) -> Option<AppState> {
    use AppState::*;
    let destinations: &[AppState] = match state {
        MultisigMenu => &[MultisigChooseMN, MultisigDescriptorsMenu, MultisigMenu],
        MultisigDescriptorsMenu => &[
            MultisigDescriptorList { action: 0 },
            MultisigDescriptorList { action: 1 },
            SdKpubFileList,
            MultisigDescriptorList { action: 2 },
        ],
        ConfirmTx => &[ConfirmTx, Rejected, ReviewTx { page: 0 }],
        _ => return None,
    };
    destinations.get(usize::from(index)).copied()
}

fn export(state: AppState, index: u8) -> Option<AppState> {
    use AppState::*;
    Some(match (state, index) {
        (XprvExportMenu, 0) => ExportXprv,
        (XprvExportMenu, 1) => SdXprvFilename,
        _ => return None,
    })
}

fn restore(state: AppState, index: u8) -> Option<AppState> {
    use AppState::*;
    Some(match (state, index) {
        (StorageSeedSourceChoice, 0) => RestoreWord { word_idx: 0 },
        (StorageSeedSourceChoice, 1) => ScanQR,
        (StorageSeedSourceChoice, 2) => SdWalletBackupFileList,
        (StorageSeedSourceChoice, 3) => AdvancedRestoreMenu,
        (AdvancedRestoreMenu, 0) => ScanQR,
        (AdvancedRestoreMenu, 1) => ScanQR,
        (AdvancedRestoreMenu, 2) => StegoImportPick,
        (AdvancedRestoreMenu, 3) => ImportPrivKey,
        _ => return None,
    })
}

fn imports(state: AppState, index: u8) -> Option<AppState> {
    use AppState::*;
    Some(match (state, index) {
        (SdImportMenu, 0) => SdWalletBackupFileList,
        (SdImportMenu, 1) => SdKsptFileList,
        (SdImportMenu, 2) => SdKpubFileList,
        (SdImportMenu, 3) => SdKpubFileList,
        (SdImportMenu, 4) => SdFileList,
        (SdImportMenu, 5) => ImportPrivKey,
        _ => return None,
    })
}
