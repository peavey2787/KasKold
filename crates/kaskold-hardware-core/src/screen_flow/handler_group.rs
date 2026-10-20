//! Maps application states to their owning touch-controller subsystem.

use super::state::AppState;

#[derive(Debug, Clone, Copy, PartialEq)]
/// Groups AppState variants by their touch handler module.
pub enum HandlerGroup {
    Menu,
    Stego,
    Sd,
    Seed,
    Export,
    Settings,
    Persistence,
    Tx,
    #[cfg(feature = "workflow-tests")]
    WorkflowTests,
}

impl AppState {
    /// Map this AppState to its responsible handler module.
    ///
    /// The match is exhaustive over every always-compiled screen: the root
    /// workspace builds this crate without screen-set features, so a new
    /// screen cannot compile there without an owner. Screens that exist only
    /// in feature builds are owned by [`AppState::feature_handler_group`]. The
    /// grouped table layout is kept because one variant per line would exceed
    /// the function-length limit without adding clarity.
    #[rustfmt::skip]
    pub fn handler_group(&self) -> HandlerGroup {
        use AppState::*;
        match self {
            // Menu screens
            MainMenu | SeedsMenu | SeedToolsMenu | ImportExportChoice
            | ImportMenu | SingleSigMenu | MultisigMenu | MultisigDescriptorsMenu | WalletBackupMethodsMenu
            | WalletDetails | AdvancedMenu | WalletAdvancedMenu | BackupRecoveryMenu
            | DiceRoll | TouchEntropy
            | ChooseWordCount { .. } | StorageSeedWordCountChoice { .. } | SeedEntropyUnavailable { .. } | StorageSeedDiceChoice | StorageSeedDiceCountChoice | StorageSeedTouchChoice | ShowQR | Rejected
            | CovBackupName
                => HandlerGroup::Menu,

            // JPEG steganographic wallet-backup workflow
            StegoModeSelect | StegoSecuritySelect | StegoResult | StegoJpegPick
            | StegoJpegDescChoice | StegoJpegDescFile | StegoJpegDesc
            | StegoJpegDescPreview | StegoJpegPpAsk | StegoJpegPpInfo
            | StegoJpegPpEntry | StegoPortablePassword | StegoPortablePasswordConfirm
            | StegoJpegConfirm | StegoImportPick
            | StegoImportDescChoice | StegoImportDescFile
            | StegoImportPass | StegoImportPortablePassword | StegoHintReveal | StegoHintPassphrase
                => HandlerGroup::Stego,

            // SD backup/restore
            SdFileList | SdDeleteConfirm | SdBackupWarning | SdSeedFilename | SdSeedExportPassphrase
            | SdXprvExportPassphrase | SdWalletBackupFileList | SdWalletBackupImportPassphrase
            | SdImportMenu | SdKsptFileList | SdKpubFileList
            | ShowQrPopup | SdKsptFilename | SdKpubFilename
            | SdSigFilename | SdXprvFilename | SdMsAddrFilename
            | SdMsAddrEncryptAsk
            | SdMsDescFilename | SdMsDescEncryptAsk
            | SdKsptEncryptAsk | SdKsptEncryptPass
            | SdOverwriteWarning | SdKpubEncryptAsk
            | ShowQrModeChoice
                => HandlerGroup::Sd,

            // Seed management
            Bip85Index { .. } | Bip85ShowWord { .. }
            | ImportPrivKey | ImportWord { .. } | RestoreWord { .. }
            | PassphraseChoice | PassphraseEntry | SeedList | AddWalletChoice | WalletNameEntry { .. } | ConfirmDeleteSeed
                => HandlerGroup::Seed,

            // Export/display
            SeedBackup { .. } | ShowAddress | ShowAddressQR | AddrIndexPicker
            | ExportSeedQR | ExportCompactSeedQR | SeedQrGrid { .. }
            | QrExportMenu | XprvExportMenu | SeedBackupMenu | WatchOnlyMenu | SigningKeysMenu | ExportPlainWordsQR
            | ExportKpub | ExportKpubPopup | KpubScannedPopup | ExportXprv | ExportChoice | ExportPrivKey
            | ExportPrivKeyIndex
                => HandlerGroup::Export,

            // Settings
            SettingsMenu | DisplaySettings | SdCardSettings | SdCardUnlockPassword | About
            | FactoryResetWarning | FactoryResetConfirm | FirmwareUpdateReady
            | AdvancedFeatures | AdvancedDuressWarning | AdvancedDuressEntry | AdvancedDuressConfirm
            | AdvancedSdStorageWarning
                => HandlerGroup::Settings,

            // Transaction / multisig / camera / message signing
            ScanQR | ReviewTx { .. } | InspectUtxoSummary | InspectUtxo { .. } | ConfirmTx | SignTxGuide | AntiKleptoRevealGuide
            | MultisigChooseMN | MultisigPickSeed { .. }
            | MultisigAddKey { .. } | MultisigShowAddress | MultisigShowAddressQR
            | MultisigSaveAddrAsk | MultisigSaveDescriptorAsk | MultisigDescriptor | MultisigDescriptorList { .. }
            | SignMsgChoice | SignMsgType | SignMsgScan | SignMsgFile | SignMsgPreview | SignMsgResult | SignMsgResultQr
            | CovenantSignReview | CovenantSignOpaqueWarning | CovenantSignOpaqueConfirm
            | CovenantKeyResult | CovenantKeyResultQr | CovenantSignResult | CovenantSignResultQr
            | PrivateSwapReview | PrivateSwapKeyResult | PrivateSwapKeyResultQr | PrivateSwapResult | PrivateSwapResultQr
            | CommitRevealType | CommitRevealPreview | CommitRevealResult | CommitRevealResultQr
            | DecryptSecretScan | DecryptSecretResult | DecryptSecretResultQr
                => HandlerGroup::Tx,

            // Persistent-wallet policy/setup/unlock states. Onboarding-owned
            // variants are intercepted by the authoritative onboarding facade
            // before this generic group is consulted.
            StorageModeChoice | StorageSeedSourceChoice | AdvancedRestoreMenu | RestoreWord12Detected
            | StorageRecoveryAcknowledgement | StorageFinalizeChoice | StorageProtectionChoice | StorageCredentialType
            | StoragePinEntry | StoragePinConfirm
            | StoragePasswordEntry | StoragePasswordConfirm | StorageUnlockPin | StorageUnlockPassword
            | StorageSdFailure => HandlerGroup::Persistence,

            #[cfg(any(feature = "developer-ui", feature = "provisioning-ui", feature = "m5stack"))]
            feature_screen => feature_screen.feature_handler_group(),
        }
    }

    /// Owner of a screen that only developer, provisioning, workflow-test or
    /// M5Stack builds declare; every such screen is named here. Always-compiled
    /// screens never reach this table: [`AppState::handler_group`] owns each of
    /// them before falling through, so the final arm only completes the match.
    #[cfg(any(
        feature = "developer-ui",
        feature = "provisioning-ui",
        feature = "m5stack"
    ))]
    #[rustfmt::skip]
    fn feature_handler_group(&self) -> HandlerGroup {
        use AppState::*;
        match self {
            #[cfg(feature = "developer-ui")]
            DeveloperMenu | NetworkMenu => HandlerGroup::Menu,
            #[cfg(feature = "provisioning-ui")]
            OwnerFirmwareMenu => HandlerGroup::Menu,
            #[cfg(feature = "workflow-tests")]
            WorkflowTestsMenu | WorkflowTestsCategory { .. } | WorkflowTestsResult => HandlerGroup::WorkflowTests,
            #[cfg(feature = "developer-ui")]
            DiagnosticInfo => HandlerGroup::Settings,
            #[cfg(feature = "provisioning-ui")]
            PopItPrompt | PopItExplain | PopItConfirm
            | OwnerKeyWarning | OwnerKeyConfirm | OwnerInstallWarning | OwnerInstallConfirm
                => HandlerGroup::Settings,
            #[cfg(feature = "m5stack")]
            AdvancedRtcEntry | AdvancedTimeLockWarning | AdvancedTimeLockEntry | AdvancedTimeLockConfirm
            | AdvancedWeeklyWarning | AdvancedWeeklyEntry | AdvancedWeeklyConfirm | AudioSettings
                => HandlerGroup::Settings,
            _ => HandlerGroup::Settings,
        }
    }
}
