// Authoritative production menu/action inventory. Included by ../ui_graph.rs.

pub(crate) const MAIN_MENU_ITEMS: [UiMenuItemSpec; 4] = [
    ui_menu!(MainMenu, 0, "Connect", "home.connect_companion", SeedsMenu, "seed_loaded", ConnectCompanion),
    ui_menu!(MainMenu, 1, "Scan QR", "home.scan_qr", ScanQR, "always"),
    ui_menu!(MainMenu, 2, "Wallet", "home.wallet", SeedsMenu, "always"),
    ui_menu!(MainMenu, 3, "Settings", "home.settings", SettingsMenu, "always"),
];
pub(crate) const MAIN_MENU_LABELS: [&str; 4] = [MAIN_MENU_ITEMS[0].label, MAIN_MENU_ITEMS[1].label, MAIN_MENU_ITEMS[2].label, MAIN_MENU_ITEMS[3].label];

pub(crate) const ADVANCED_MENU_BASE_ITEMS: [UiMenuItemSpec; 2] = [
    ui_menu!(AdvancedMenu, 0, "Firmware Update", "settings.advanced.firmware_update", FirmwareUpdateReady, "always"),
    ui_menu!(AdvancedMenu, 1, "Factory Reset", "settings.advanced.factory_reset", FactoryResetWarning, "always"),
];
#[cfg(feature = "provisioning-ui")]
pub(crate) const ADVANCED_MENU_PROVISIONING_ITEMS: [UiMenuItemSpec; 2] = [
    ui_menu!(AdvancedMenu, 2, "Owner Firmware", "settings.advanced.owner_firmware", OwnerFirmwareMenu, "always"),
    ui_menu!(AdvancedMenu, 3, "Pop It!", "settings.advanced.pop_it", PopItPrompt, "secure_boot_disabled"),
];
#[cfg(feature = "provisioning-ui")]
pub(crate) const ADVANCED_MENU_ITEMS: [UiMenuItemSpec; 4] = [
    ADVANCED_MENU_BASE_ITEMS[0], ADVANCED_MENU_BASE_ITEMS[1],
    ADVANCED_MENU_PROVISIONING_ITEMS[0], ADVANCED_MENU_PROVISIONING_ITEMS[1],
];
#[cfg(not(feature = "provisioning-ui"))]
pub(crate) const ADVANCED_MENU_ITEMS: [UiMenuItemSpec; 2] = ADVANCED_MENU_BASE_ITEMS;
#[cfg(feature = "provisioning-ui")]
pub(crate) const ADVANCED_MENU_LABELS: [&str; 4] = [ADVANCED_MENU_ITEMS[0].label, ADVANCED_MENU_ITEMS[1].label, ADVANCED_MENU_ITEMS[2].label, ADVANCED_MENU_ITEMS[3].label];
#[cfg(not(feature = "provisioning-ui"))]
pub(crate) const ADVANCED_MENU_LABELS: [&str; 2] = [ADVANCED_MENU_ITEMS[0].label, ADVANCED_MENU_ITEMS[1].label];

#[cfg(feature = "provisioning-ui")]
pub(crate) const OWNER_FIRMWARE_MENU_ITEMS: [UiMenuItemSpec; 2] = [
    ui_menu!(OwnerFirmwareMenu, 0, "Enroll Owner Key", "settings.owner_firmware.enroll", OwnerKeyWarning, "secure_boot_disabled"),
    ui_menu!(OwnerFirmwareMenu, 1, "Install from SD", "settings.owner_firmware.install", OwnerInstallWarning, "m5stack"),
];
#[cfg(feature = "provisioning-ui")]
pub(crate) const OWNER_FIRMWARE_MENU_LABELS: [&str; 2] = [OWNER_FIRMWARE_MENU_ITEMS[0].label, OWNER_FIRMWARE_MENU_ITEMS[1].label];

pub(crate) const SEEDS_MENU_ITEMS: [UiMenuItemSpec; 7] = [
    ui_menu!(SeedsMenu, 0, "Receive", "wallet.receive", ShowAddress, "always"),
    ui_menu!(SeedsMenu, 1, "Backup", "wallet.backup", WalletBackupMethodsMenu, "always"),
    ui_menu!(SeedsMenu, 2, "Recovery", "wallet.recovery", SdImportMenu, "always"),
    ui_menu!(SeedsMenu, 3, "Wallet Details", "wallet.details", WalletDetails, "always"),
    ui_menu!(SeedsMenu, 4, "Switch / Add Wallet", "wallet.switch_add", SeedList, "always"),
    ui_menu!(SeedsMenu, 5, "Multisig", "wallet.multisig", MultisigMenu, "always"),
    ui_menu!(SeedsMenu, 6, "Advanced", "wallet.advanced", WalletAdvancedMenu, "always"),
];
pub(crate) const SEEDS_MENU_LABELS: [&str; 7] = [SEEDS_MENU_ITEMS[0].label, SEEDS_MENU_ITEMS[1].label, SEEDS_MENU_ITEMS[2].label, SEEDS_MENU_ITEMS[3].label, SEEDS_MENU_ITEMS[4].label, SEEDS_MENU_ITEMS[5].label, SEEDS_MENU_ITEMS[6].label];

pub(crate) const WALLET_BACKUP_METHODS_MENU_ITEMS: [UiMenuItemSpec; 4] = [
    ui_menu!(WalletBackupMethodsMenu, 0, "View Words", "wallet.backup.view_words", SeedBackup, "mnemonic_wallet"),
    ui_menu!(WalletBackupMethodsMenu, 1, "SeedQR Backup", "wallet.backup.seedqr", ExportSeedQR, "mnemonic_wallet"),
    ui_menu!(WalletBackupMethodsMenu, 2, "Encrypted SD Card", "wallet.backup.sd", SdBackupWarning, "mnemonic_wallet"),
    ui_menu!(WalletBackupMethodsMenu, 3, "Advanced", "wallet.backup.advanced", BackupRecoveryMenu, "always"),
];
pub(crate) const WALLET_BACKUP_METHODS_MENU_LABELS: [&str; 4] = [WALLET_BACKUP_METHODS_MENU_ITEMS[0].label, WALLET_BACKUP_METHODS_MENU_ITEMS[1].label, WALLET_BACKUP_METHODS_MENU_ITEMS[2].label, WALLET_BACKUP_METHODS_MENU_ITEMS[3].label];

pub(crate) const WALLET_ADVANCED_MENU_ITEMS: [UiMenuItemSpec; 6] = [
    ui_menu!(WalletAdvancedMenu, 0, "BIP85 Child Wallet", "wallet.advanced.bip85", ChooseWordCount, "seed_loaded"),
    ui_menu!(WalletAdvancedMenu, 1, "Sign Message", "wallet.advanced.sign_message", SignMsgChoice, "seed_loaded"),
    ui_menu!(WalletAdvancedMenu, 2, "Commit Secret", "wallet.advanced.commit_secret", CommitRevealType, "seed_loaded"),
    ui_menu!(WalletAdvancedMenu, 3, "Decrypt Secret", "wallet.advanced.decrypt_secret", DecryptSecretScan, "seed_loaded"),
    ui_menu!(WalletAdvancedMenu, 4, "Hidden Key Addr", "wallet.advanced.kip22_address", ShowAddress, "mnemonic_wallet"),
    ui_menu!(WalletAdvancedMenu, 5, "kpub to SD", "wallet.advanced.kpub_sd", SdKpubFilename, "sd_present_and_seed_loaded"),
];
pub(crate) const WALLET_ADVANCED_MENU_LABELS: [&str; 6] = [
    WALLET_ADVANCED_MENU_ITEMS[0].label,
    WALLET_ADVANCED_MENU_ITEMS[1].label,
    WALLET_ADVANCED_MENU_ITEMS[2].label,
    WALLET_ADVANCED_MENU_ITEMS[3].label,
    WALLET_ADVANCED_MENU_ITEMS[4].label,
    WALLET_ADVANCED_MENU_ITEMS[5].label,
];

pub(crate) const BACKUP_RECOVERY_MENU_ITEMS: [UiMenuItemSpec; 5] = [
    ui_menu!(BackupRecoveryMenu, 0, "Compact SeedQR", "wallet.backup.advanced.compact_seedqr", ExportCompactSeedQR, "mnemonic_wallet"),
    ui_menu!(BackupRecoveryMenu, 1, "Plain-text SeedQR", "wallet.backup.advanced.plain_seedqr", ExportPlainWordsQR, "mnemonic_wallet"),
    ui_menu!(BackupRecoveryMenu, 2, "Steganographic", "wallet.backup.advanced.stego", StegoModeSelect, "seed_loaded"),
    ui_menu!(BackupRecoveryMenu, 3, "XPrv Backup", "wallet.backup.advanced.xprv", XprvExportMenu, "seed_loaded"),
    ui_menu!(BackupRecoveryMenu, 4, "Export Key", "wallet.backup.advanced.export_key", ExportPrivKeyIndex, "seed_loaded"),
];
pub(crate) const BACKUP_RECOVERY_MENU_LABELS: [&str; 5] = [BACKUP_RECOVERY_MENU_ITEMS[0].label, BACKUP_RECOVERY_MENU_ITEMS[1].label, BACKUP_RECOVERY_MENU_ITEMS[2].label, BACKUP_RECOVERY_MENU_ITEMS[3].label, BACKUP_RECOVERY_MENU_ITEMS[4].label];

pub(crate) const STORAGE_SEED_SOURCE_CHOICE_ITEMS: [UiMenuItemSpec; 4] = [
    ui_menu!(StorageSeedSourceChoice, 0, "Words", "onboarding.restore.words", RestoreWord, "always"),
    ui_menu!(StorageSeedSourceChoice, 1, "SeedQR", "onboarding.restore.seedqr", ScanQR, "camera_available"),
    ui_menu!(StorageSeedSourceChoice, 2, "SD", "onboarding.restore.sd", SdWalletBackupFileList, "sd_present"),
    ui_menu!(StorageSeedSourceChoice, 3, "Advanced", "onboarding.restore.advanced", AdvancedRestoreMenu, "always"),
];
pub(crate) const STORAGE_SEED_SOURCE_CHOICE_LABELS: [&str; 4] = [STORAGE_SEED_SOURCE_CHOICE_ITEMS[0].label, STORAGE_SEED_SOURCE_CHOICE_ITEMS[1].label, STORAGE_SEED_SOURCE_CHOICE_ITEMS[2].label, STORAGE_SEED_SOURCE_CHOICE_ITEMS[3].label];

pub(crate) const ADVANCED_RESTORE_MENU_ITEMS: [UiMenuItemSpec; 4] = [
    ui_menu!(AdvancedRestoreMenu, 0, "Compact SeedQR", "onboarding.advanced_restore.compact_seedqr", ScanQR, "camera_available"),
    ui_menu!(AdvancedRestoreMenu, 1, "Plain-text SeedQR", "onboarding.advanced_restore.plain_seedqr", ScanQR, "camera_available"),
    ui_menu!(AdvancedRestoreMenu, 2, "Steganographic", "onboarding.advanced_restore.stego", StegoImportPick, "sd_present"),
    ui_menu!(AdvancedRestoreMenu, 3, "Raw Private Key", "onboarding.advanced_restore.raw_key", ImportPrivKey, "always"),
];
pub(crate) const ADVANCED_RESTORE_MENU_LABELS: [&str; 4] = [ADVANCED_RESTORE_MENU_ITEMS[0].label, ADVANCED_RESTORE_MENU_ITEMS[1].label, ADVANCED_RESTORE_MENU_ITEMS[2].label, ADVANCED_RESTORE_MENU_ITEMS[3].label];

pub(crate) const SETTINGS_MENU_ITEMS: [UiMenuItemSpec; 6] = [
    ui_menu!(SettingsMenu, 0, "Display", "settings.display", DisplaySettings, "always"),
    ui_menu!(SettingsMenu, 1, "Audio", "settings.audio", AudioSettings, "m5stack"),
    ui_menu!(SettingsMenu, 2, "Security", "settings.security", AdvancedFeatures, "always"),
    ui_menu!(SettingsMenu, 3, "Storage", "settings.storage", SdCardSettings, "always"),
    ui_menu!(SettingsMenu, 4, "Advanced", "settings.advanced", AdvancedMenu, "always"),
    ui_menu!(SettingsMenu, 5, "About", "settings.about", About, "always"),
];
#[cfg(all(feature = "m5stack", not(feature = "developer-ui")))]
pub(crate) const SETTINGS_MENU_LABELS: [&str; 6] = [SETTINGS_MENU_ITEMS[0].label, SETTINGS_MENU_ITEMS[1].label, SETTINGS_MENU_ITEMS[2].label, SETTINGS_MENU_ITEMS[3].label, SETTINGS_MENU_ITEMS[4].label, SETTINGS_MENU_ITEMS[5].label];

pub(crate) const MULTISIG_MENU_ITEMS: [UiMenuItemSpec; 3] = [
    ui_menu!(MultisigMenu, 0, "Create Multisig", "multisig.create", MultisigChooseMN, "always"),
    ui_menu!(MultisigMenu, 1, "Descriptors", "multisig.descriptors", MultisigDescriptorsMenu, "mnemonic_wallet"),
    ui_menu!(MultisigMenu, 2, "kpub Multisig QR", "multisig.kpub", MultisigMenu, "mnemonic_wallet", DeriveMultisigKpub),
];
pub(crate) const MULTISIG_MENU_LABELS: [&str; 3] = [MULTISIG_MENU_ITEMS[0].label, MULTISIG_MENU_ITEMS[1].label, MULTISIG_MENU_ITEMS[2].label];

pub(crate) const MULTISIG_DESCRIPTORS_MENU_ITEMS: [UiMenuItemSpec; 4] = [
    ui_menu!(MultisigDescriptorsMenu, 0, "View", "multisig.descriptors.view", MultisigDescriptorList, "mnemonic_wallet"),
    ui_menu!(MultisigDescriptorsMenu, 1, "Backup to SD", "multisig.descriptors.backup", MultisigDescriptorList, "mnemonic_wallet"),
    ui_menu!(MultisigDescriptorsMenu, 2, "Restore from SD", "multisig.descriptors.restore", SdKpubFileList, "sd_present"),
    ui_menu!(MultisigDescriptorsMenu, 3, "Delete", "multisig.descriptors.delete", MultisigDescriptorList, "mnemonic_wallet"),
];
pub(crate) const MULTISIG_DESCRIPTORS_MENU_LABELS: [&str; 4] = [
    MULTISIG_DESCRIPTORS_MENU_ITEMS[0].label,
    MULTISIG_DESCRIPTORS_MENU_ITEMS[1].label,
    MULTISIG_DESCRIPTORS_MENU_ITEMS[2].label,
    MULTISIG_DESCRIPTORS_MENU_ITEMS[3].label,
];

pub(crate) const XPRV_EXPORT_MENU_ITEMS: [UiMenuItemSpec; 2] = [
    ui_menu!(XprvExportMenu, 0, "Show as QR", "export.xprv.qr", ExportXprv, "seed_loaded"),
    ui_menu!(XprvExportMenu, 1, "Encrypt to SD", "export.xprv.sd", SdXprvFilename, "sd_present_and_seed_loaded"),
];
pub(crate) const XPRV_EXPORT_MENU_LABELS: [&str; 2] = [XPRV_EXPORT_MENU_ITEMS[0].label, XPRV_EXPORT_MENU_ITEMS[1].label];

pub(crate) const SD_IMPORT_MENU_ITEMS: [UiMenuItemSpec; 6] = [
    ui_menu!(SdImportMenu, 0, "Seed / XPrv Backup", "sd.import.wallet_backup", SdWalletBackupFileList, "sd_present"),
    ui_menu!(SdImportMenu, 1, "Transaction", "sd.import.transaction", SdKsptFileList, "sd_present"),
    ui_menu!(SdImportMenu, 2, "kpub (Watch-Only)", "sd.import.kpub", SdKpubFileList, "sd_present"),
    ui_menu!(SdImportMenu, 3, "Multisig Address", "sd.import.multisig_address", SdKpubFileList, "sd_present"),
    ui_menu!(SdImportMenu, 4, "Covenant Restore", "sd.import.covenant", SdFileList, "sd_present"),
    ui_menu!(SdImportMenu, 5, "Import Raw Key", "recovery.import_raw_key", ImportPrivKey, "always"),
];
pub(crate) const SD_IMPORT_MENU_LABELS: [&str; 6] = [SD_IMPORT_MENU_ITEMS[0].label, SD_IMPORT_MENU_ITEMS[1].label, SD_IMPORT_MENU_ITEMS[2].label, SD_IMPORT_MENU_ITEMS[3].label, SD_IMPORT_MENU_ITEMS[4].label, SD_IMPORT_MENU_ITEMS[5].label];

pub(crate) const CONFIRM_TX_ITEMS: [UiMenuItemSpec; 3] = [
    ui_menu!(ConfirmTx, 0, "Confirm", "tx.confirm", ConfirmTx, "review_complete", SignTransaction),
    ui_menu!(ConfirmTx, 1, "Cancel", "tx.cancel", Rejected, "always"),
    ui_menu!(ConfirmTx, 2, "Inspect", "tx.inspect", ReviewTx, "review_available"),
];
pub(crate) const CONFIRM_TX_LABELS: [&str; 3] = [CONFIRM_TX_ITEMS[0].label, CONFIRM_TX_ITEMS[1].label, CONFIRM_TX_ITEMS[2].label];

pub(crate) const PRODUCTION_MENUS: &[UiMenuSpec] = &[
    UiMenuSpec { state: "MainMenu", back: "-", items: &MAIN_MENU_ITEMS },
    UiMenuSpec { state: "AdvancedMenu", back: "SettingsMenu", items: &ADVANCED_MENU_ITEMS },
    #[cfg(feature = "provisioning-ui")]
    UiMenuSpec { state: "OwnerFirmwareMenu", back: "AdvancedMenu", items: &OWNER_FIRMWARE_MENU_ITEMS },
    UiMenuSpec { state: "SeedsMenu", back: "MainMenu", items: &SEEDS_MENU_ITEMS },
    UiMenuSpec { state: "WalletBackupMethodsMenu", back: "SeedsMenu", items: &WALLET_BACKUP_METHODS_MENU_ITEMS },
    UiMenuSpec { state: "WalletAdvancedMenu", back: "history", items: &WALLET_ADVANCED_MENU_ITEMS },
    UiMenuSpec { state: "BackupRecoveryMenu", back: "history", items: &BACKUP_RECOVERY_MENU_ITEMS },
    UiMenuSpec { state: "StorageSeedSourceChoice", back: "StorageModeChoice", items: &STORAGE_SEED_SOURCE_CHOICE_ITEMS },
    UiMenuSpec { state: "AdvancedRestoreMenu", back: "StorageSeedSourceChoice", items: &ADVANCED_RESTORE_MENU_ITEMS },
    UiMenuSpec { state: "SettingsMenu", back: "MainMenu", items: &SETTINGS_MENU_ITEMS },
    UiMenuSpec { state: "MultisigMenu", back: "history", items: &MULTISIG_MENU_ITEMS },
    UiMenuSpec { state: "MultisigDescriptorsMenu", back: "MultisigMenu", items: &MULTISIG_DESCRIPTORS_MENU_ITEMS },
    UiMenuSpec { state: "XprvExportMenu", back: "history", items: &XPRV_EXPORT_MENU_ITEMS },
    UiMenuSpec { state: "SdImportMenu", back: "history", items: &SD_IMPORT_MENU_ITEMS },
    UiMenuSpec { state: "ConfirmTx", back: "ReviewTx", items: &CONFIRM_TX_ITEMS },
];

pub(crate) const WALLET_MENU_LABELS: &[&str] = &SEEDS_MENU_LABELS;
pub(crate) const WALLET_BACKUP_METHODS_LABELS: &[&str] = &WALLET_BACKUP_METHODS_MENU_LABELS;
pub(crate) const WALLET_ADVANCED_LABELS: &[&str] = &WALLET_ADVANCED_MENU_LABELS;
pub(crate) const BACKUP_RECOVERY_LABELS: &[&str] = &BACKUP_RECOVERY_MENU_LABELS;
pub(crate) const RESTORE_LABELS: &[&str] = &STORAGE_SEED_SOURCE_CHOICE_LABELS;
pub(crate) const ADVANCED_RESTORE_LABELS: &[&str] = &ADVANCED_RESTORE_MENU_LABELS;
#[cfg(all(feature = "m5stack", not(feature = "developer-ui")))]
pub(crate) const M5STACK_SETTINGS_LABELS: &[&str] = &SETTINGS_MENU_LABELS;
pub(crate) const MULTISIG_LABELS: &[&str] = &MULTISIG_MENU_LABELS;
pub(crate) const MULTISIG_DESCRIPTOR_LABELS: &[&str] = &MULTISIG_DESCRIPTORS_MENU_LABELS;
pub(crate) const XPRV_EXPORT_LABELS: &[&str] = &XPRV_EXPORT_MENU_LABELS;
pub(crate) const SD_IMPORT_LABELS: &[&str] = &SD_IMPORT_MENU_LABELS;

