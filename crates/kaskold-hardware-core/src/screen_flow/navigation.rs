//! Navigation ownership and cross-workflow transition policy.

use super::{onboarding, AppState, HandlerGroup};

mod bridges;
use bridges::{
    add_wallet_persistence_bridge, add_wallet_restore_transport_bridge,
    backup_recovery_stego_entry_allowed, seed_backup_return_allowed,
    wallet_protection_persistence_bridge,
};

/// Why the device is currently walking a storage workflow.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeviceStorageIntent {
    None,
    StartFresh,
    CreateInternal,
    EnableSd,
}

impl DeviceStorageIntent {
    pub const fn is_seed_onboarding(self) -> bool {
        matches!(self, Self::StartFresh | Self::CreateInternal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavigationOwner {
    Onboarding,
    Main,
    Seeds,
    Settings,
    Signing,
    Export,
    Storage,
    Stego,
    Multisig,
    #[cfg(feature = "workflow-tests")]
    WorkflowTests,
}

pub fn owner_for_intent(
    intent: DeviceStorageIntent,
    state: AppState,
    current: NavigationOwner,
) -> NavigationOwner {
    if intent == DeviceStorageIntent::EnableSd && is_sd_enable_state(state) {
        return NavigationOwner::Settings;
    }
    // QR presentation belongs to the workflow that produced the payload. A
    // stale seed-onboarding storage intent must not steal a later Signing,
    // Export, Storage, or Multisig result screen.
    if is_qr_state(state) {
        return qr_owner(current);
    }
    if onboarding::owns_state(intent.is_seed_onboarding(), state) {
        return NavigationOwner::Onboarding;
    }
    if state == AppState::StorageSdFailure && current == NavigationOwner::Settings {
        return NavigationOwner::Settings;
    }
    fixed_owner(state).unwrap_or_else(|| owner_from_handler(state.handler_group(), current))
}

pub fn transition_allowed(
    from: NavigationOwner,
    to: NavigationOwner,
    from_state: AppState,
    to_state: AppState,
) -> bool {
    if special_transition_allowed(from, to, from_state, to_state) {
        return true;
    }
    if from == to {
        return from != NavigationOwner::Onboarding
            || onboarding::transition_allowed(from_state, to_state);
    }
    if from_state == AppState::MainMenu
        && matches!(
            to_state,
            AppState::StorageModeChoice
                | AppState::StorageUnlockPin
                | AppState::StorageUnlockPassword
        )
    {
        return true;
    }
    if from_state == AppState::MainMenu {
        return home_route_allowed(to_state) && owner_transition_allowed(from, to);
    }
    if to_state == AppState::MainMenu {
        return from != NavigationOwner::Onboarding;
    }
    owner_transition_allowed(from, to)
}

fn home_route_allowed(state: AppState) -> bool {
    matches!(
        state,
        AppState::ShowAddress
            | AppState::ScanQR
            | AppState::SeedsMenu
            | AppState::SettingsMenu
            | AppState::ExportKpub
    )
}

fn special_transition_allowed(
    from: NavigationOwner,
    to: NavigationOwner,
    from_state: AppState,
    to_state: AppState,
) -> bool {
    // Add Wallet reuses the hardened persistence credential screens without
    // becoming first-device onboarding. Keep only the exact bridges required
    // by recovery acknowledgement -> protection -> credential -> WALLETS.
    if add_wallet_restore_transport_bridge(from, to, from_state, to_state) {
        return true;
    }
    if add_wallet_persistence_bridge(from_state, to_state) {
        return true;
    }
    if wallet_protection_persistence_bridge(from_state, to_state) {
        return true;
    }
    // Advanced Backup is Seeds-owned, while the steganographic export flow is
    // Stego-owned. Permit only this explicit production entry edge; do not
    // broaden Seeds -> Stego transitions generally.
    if backup_recovery_stego_entry_allowed(from, to, from_state, to_state) {
        return true;
    }
    // Recovery-word viewing reuses the SeedBackup pager, but ordinary Wallet/
    // export callers are not onboarding. Permit only its explicit bounded
    // return destinations so Back cannot fall through to onboarding recovery.
    seed_backup_return_allowed(from, from_state, to_state)
}

pub fn safe_recovery(owner: NavigationOwner) -> AppState {
    use NavigationOwner::*;
    match owner {
        Onboarding => AppState::StorageModeChoice,
        Settings => AppState::SettingsMenu,
        Seeds => AppState::SeedsMenu,
        Multisig => AppState::MultisigMenu,
        #[cfg(feature = "workflow-tests")]
        WorkflowTests => AppState::WorkflowTestsMenu,
        Main | Signing | Export | Storage | Stego => AppState::MainMenu,
    }
}

pub fn recovery_owner(owner: NavigationOwner) -> NavigationOwner {
    use NavigationOwner::*;
    match owner {
        Onboarding => Onboarding,
        Settings => Settings,
        Seeds => Seeds,
        Multisig => Multisig,
        #[cfg(feature = "workflow-tests")]
        WorkflowTests => WorkflowTests,
        _ => Main,
    }
}

fn fixed_owner(state: AppState) -> Option<NavigationOwner> {
    use AppState::*;
    use NavigationOwner::*;
    if matches!(state, StorageUnlockPin | StorageUnlockPassword) {
        return Some(Storage);
    }
    if state == MainMenu {
        return Some(Main);
    }
    if matches!(
        state,
        SeedsMenu
            | SeedList
            | AddWalletChoice
            | WalletNameEntry { .. }
            | ConfirmDeleteSeed
            | WalletBackupMethodsMenu
            | WalletDetails
            | WalletAdvancedMenu
            | BackupRecoveryMenu
    ) {
        return Some(Seeds);
    }
    #[cfg(feature = "workflow-tests")]
    if matches!(
        state,
        WorkflowTestsMenu | WorkflowTestsCategory { .. } | WorkflowTestsResult
    ) {
        return Some(WorkflowTests);
    }
    if is_multisig_menu_state(state) {
        return Some(Multisig);
    }
    if is_settings_state(state) {
        return Some(Settings);
    }
    if state == ShowQrPopup {
        return Some(Signing);
    }
    None
}

/// The owners each workflow may hand over to.
const OWNER_TRANSITIONS: &[(NavigationOwner, &[NavigationOwner])] = {
    use NavigationOwner::*;
    &[
        (Onboarding, &[Onboarding, Main]),
        (
            Settings,
            &[Main, Seeds, Signing, Export, Storage, Stego, Multisig],
        ),
        (Main, &[Seeds, Settings, Signing, Export]),
        (
            Seeds,
            &[
                Main, Settings, Signing, Export, Storage, Multisig, Onboarding,
            ],
        ),
        (Signing, &[Main, Seeds, Export, Storage, Multisig]),
        (Export, &[Main, Seeds, Signing, Storage, Multisig]),
        (Storage, &[Main, Seeds, Signing, Export, Multisig, Stego]),
        (Stego, &[Main, Seeds, Storage, Export]),
        (Multisig, &[Main, Seeds, Signing, Export, Storage]),
        #[cfg(feature = "workflow-tests")]
        (Settings, &[WorkflowTests]),
        #[cfg(feature = "workflow-tests")]
        (WorkflowTests, &[Settings]),
    ]
};

fn owner_transition_allowed(from: NavigationOwner, to: NavigationOwner) -> bool {
    OWNER_TRANSITIONS
        .iter()
        .any(|(owner, targets)| *owner == from && targets.contains(&to))
}

fn is_qr_state(state: AppState) -> bool {
    matches!(state, AppState::ShowQR | AppState::ShowQrModeChoice)
}

fn qr_owner(current: NavigationOwner) -> NavigationOwner {
    use NavigationOwner::*;
    if matches!(current, Signing | Export | Storage | Multisig) {
        current
    } else {
        Signing
    }
}

fn owner_from_handler(group: HandlerGroup, current: NavigationOwner) -> NavigationOwner {
    use NavigationOwner::*;
    match group {
        HandlerGroup::Settings => Settings,
        HandlerGroup::Persistence => Storage,
        HandlerGroup::Seed => Seeds,
        HandlerGroup::Export => Export,
        HandlerGroup::Sd => Storage,
        HandlerGroup::Stego => Stego,
        HandlerGroup::Tx => Signing,
        #[cfg(feature = "workflow-tests")]
        HandlerGroup::WorkflowTests => WorkflowTests,
        HandlerGroup::Menu => menu_handler_owner(current),
    }
}

fn menu_handler_owner(current: NavigationOwner) -> NavigationOwner {
    use NavigationOwner::*;
    if matches!(
        current,
        Onboarding | Main | Seeds | Settings | Signing | Export | Multisig
    ) {
        current
    } else {
        Main
    }
}

fn is_sd_enable_state(state: AppState) -> bool {
    matches!(
        state,
        AppState::StorageRecoveryAcknowledgement | AppState::StorageSdFailure
    )
}

fn is_multisig_menu_state(state: AppState) -> bool {
    use AppState::*;
    matches!(
        state,
        MultisigMenu
            | MultisigDescriptorsMenu
            | MultisigDescriptorList { .. }
            | MultisigChooseMN
            | MultisigPickSeed { .. }
            | MultisigAddKey { .. }
            | MultisigShowAddress
            | MultisigShowAddressQR
            | MultisigDescriptor
            | MultisigSaveAddrAsk
            | MultisigSaveDescriptorAsk
    )
}

fn is_settings_state(state: AppState) -> bool {
    use AppState::*;
    if matches!(
        state,
        SettingsMenu
            | AdvancedMenu
            | FactoryResetWarning
            | FactoryResetConfirm
            | DisplaySettings
            | SdCardSettings
            | SdCardUnlockPassword
            | AdvancedFeatures
            | AdvancedDuressWarning
            | AdvancedDuressEntry
            | AdvancedDuressConfirm
            | AdvancedSdStorageWarning
            | FirmwareUpdateReady
            | About
    ) {
        return true;
    }
    #[cfg(feature = "provisioning-ui")]
    if matches!(
        state,
        PopItPrompt
            | PopItExplain
            | PopItConfirm
            | OwnerFirmwareMenu
            | OwnerKeyWarning
            | OwnerKeyConfirm
            | OwnerInstallWarning
            | OwnerInstallConfirm
    ) {
        return true;
    }
    #[cfg(feature = "developer-ui")]
    if matches!(state, DeveloperMenu | NetworkMenu | DiagnosticInfo) {
        return true;
    }
    #[cfg(feature = "m5stack")]
    if matches!(
        state,
        AudioSettings
            | AdvancedRtcEntry
            | AdvancedTimeLockWarning
            | AdvancedTimeLockEntry
            | AdvancedTimeLockConfirm
            | AdvancedWeeklyWarning
            | AdvancedWeeklyEntry
            | AdvancedWeeklyConfirm
    ) {
        return true;
    }
    false
}

/// Whether the event loop should emit its ordinary navigation click after
/// routing and reconciling a tap. Secret-entry screens own feedback locally so accepted
/// credential keys may click once while the persisted global mute/volume policy remains authoritative.
pub fn tap_uses_router_click(state: AppState) -> bool {
    !matches!(
        state,
        AppState::ChooseWordCount { .. }
            | AppState::StorageSeedWordCountChoice { .. }
            | AppState::SeedEntropyUnavailable { .. }
            | AppState::StoragePinEntry
            | AppState::StoragePinConfirm
            | AppState::StoragePasswordEntry
            | AppState::StoragePasswordConfirm
            | AppState::StorageUnlockPin
            | AppState::StorageUnlockPassword
    )
}

/// Home is a post-onboarding shortcut only, and never escapes a storage unlock.
pub fn home_shortcut_allowed(home_reached: bool, state: AppState) -> bool {
    home_reached
        && state != AppState::MainMenu
        && !matches!(
            state,
            AppState::StorageUnlockPin | AppState::StorageUnlockPassword
        )
}

#[cfg(test)]
mod unit_tests;
