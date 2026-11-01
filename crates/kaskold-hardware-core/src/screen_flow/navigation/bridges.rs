//! The exact screen pairs allowed to cross between navigation owners.

use super::{AppState, NavigationOwner};

pub(super) fn add_wallet_restore_transport_bridge(
    from: NavigationOwner,
    to: NavigationOwner,
    from_state: AppState,
    to_state: AppState,
) -> bool {
    use AppState::*;
    use NavigationOwner::*;
    matches!(
        (from, to, from_state, to_state),
        (Onboarding, Seeds, StorageSeedSourceChoice, AddWalletChoice)
            | (
                Onboarding,
                Seeds,
                StorageSeedSourceChoice,
                RestoreWord { word_idx: 0 }
            )
            | (
                Seeds,
                Onboarding,
                RestoreWord { .. },
                StorageSeedSourceChoice
            )
            | (Seeds, Onboarding, PassphraseChoice, StorageSeedSourceChoice)
            | (
                Seeds,
                Onboarding,
                RestoreWord { word_idx: 11 },
                RestoreWord12Detected
            )
            | (
                Onboarding,
                Seeds,
                RestoreWord12Detected,
                RestoreWord { word_idx: 11 | 12 }
            )
            | (Onboarding, Seeds, RestoreWord12Detected, PassphraseChoice)
            | (Onboarding, Signing, StorageSeedSourceChoice, ScanQR)
            | (Signing, Onboarding, ScanQR, StorageSeedSourceChoice)
            | (
                Onboarding,
                Storage,
                StorageSeedSourceChoice,
                SdWalletBackupFileList
            )
            | (
                Storage,
                Onboarding,
                SdWalletBackupFileList,
                StorageSeedSourceChoice
            )
            | (Onboarding, Signing, AdvancedRestoreMenu, ScanQR)
            | (Signing, Onboarding, ScanQR, AdvancedRestoreMenu)
            | (Onboarding, Stego, AdvancedRestoreMenu, StegoImportPick)
            | (Stego, Onboarding, StegoImportPick, AdvancedRestoreMenu)
            | (Onboarding, Seeds, AdvancedRestoreMenu, ImportPrivKey)
            | (Seeds, Onboarding, ImportPrivKey, AdvancedRestoreMenu)
    )
}

pub(super) fn add_wallet_persistence_bridge(from_state: AppState, to_state: AppState) -> bool {
    use AppState::*;
    matches!(
        (from_state, to_state),
        (StorageRecoveryAcknowledgement, StorageProtectionChoice)
            | (StorageProtectionChoice, StorageRecoveryAcknowledgement)
            | (WalletNameEntry { purpose: 3 }, StorageFinalizeChoice)
            | (StorageFinalizeChoice, WalletNameEntry { purpose: 3 })
            | (StorageFinalizeChoice, PassphraseChoice)
            | (StorageFinalizeChoice, SeedList)
            | (StorageProtectionChoice, SeedList)
            | (StoragePinConfirm, SeedList)
            | (StoragePasswordConfirm, SeedList)
    )
}

pub(super) fn wallet_protection_persistence_bridge(
    from_state: AppState,
    to_state: AppState,
) -> bool {
    use AppState::*;
    matches!(
        (from_state, to_state),
        (WalletDetails, StorageCredentialType)
            | (StorageCredentialType, WalletDetails)
            | (StoragePinConfirm, WalletDetails)
            | (StoragePasswordConfirm, WalletDetails)
    )
}

pub(super) fn backup_recovery_stego_entry_allowed(
    from: NavigationOwner,
    to: NavigationOwner,
    from_state: AppState,
    to_state: AppState,
) -> bool {
    (from == NavigationOwner::Seeds
        && to == NavigationOwner::Stego
        && from_state == AppState::BackupRecoveryMenu
        && to_state == AppState::StegoModeSelect)
        || (from == NavigationOwner::Export
            && to == NavigationOwner::Stego
            && from_state == AppState::ImportMenu
            && to_state == AppState::StegoImportPick)
}

pub(super) fn seed_backup_return_allowed(
    from: NavigationOwner,
    from_state: AppState,
    to_state: AppState,
) -> bool {
    from == NavigationOwner::Onboarding
        && matches!(from_state, AppState::SeedBackup { .. })
        && matches!(
            to_state,
            AppState::WalletBackupMethodsMenu | AppState::SeedBackupMenu | AppState::SeedList
        )
}
