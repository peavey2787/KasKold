use crate::screen_flow::navigation::{
    self, owner_for_intent, recovery_owner, safe_recovery, transition_allowed,
    DeviceStorageIntent as Intent, NavigationOwner as Owner,
};
use crate::screen_flow::AppState::{self, *};

fn allowed(from: Owner, to: Owner, from_state: AppState, to_state: AppState) -> bool {
    transition_allowed(from, to, from_state, to_state)
}

#[test]
fn only_seed_setup_intents_are_seed_onboarding() {
    assert!(Intent::StartFresh.is_seed_onboarding());
    assert!(Intent::CreateInternal.is_seed_onboarding());
    assert!(!Intent::None.is_seed_onboarding());
    assert!(!Intent::EnableSd.is_seed_onboarding());
}

#[test]
fn onboarding_cannot_reach_the_main_menu_through_a_transition() {
    for from_state in [
        StorageModeChoice,
        StorageFinalizeChoice,
        StorageProtectionChoice,
        StoragePinConfirm,
    ] {
        assert!(
            !allowed(Owner::Onboarding, Owner::Main, from_state, MainMenu),
            "{from_state:?}"
        );
    }
    assert!(allowed(Owner::Seeds, Owner::Main, SeedsMenu, MainMenu));
}

#[test]
fn same_owner_moves_are_free_except_inside_onboarding() {
    assert!(allowed(
        Owner::Settings,
        Owner::Settings,
        SettingsMenu,
        About
    ));
    assert!(!allowed(
        Owner::Onboarding,
        Owner::Onboarding,
        StorageModeChoice,
        StoragePinConfirm
    ));
}

#[test]
fn home_reaches_only_its_published_routes() {
    assert!(allowed(
        Owner::Main,
        Owner::Settings,
        MainMenu,
        SettingsMenu
    ));
    assert!(allowed(Owner::Main, Owner::Seeds, MainMenu, SeedsMenu));
    assert!(allowed(Owner::Main, Owner::Signing, MainMenu, ScanQR));
    assert!(!allowed(
        Owner::Main,
        Owner::Settings,
        MainMenu,
        FactoryResetConfirm
    ));
    assert!(!allowed(
        Owner::Main,
        Owner::Multisig,
        MainMenu,
        MultisigMenu
    ));
}

#[test]
fn home_may_always_enter_storage_unlock() {
    for to_state in [StorageModeChoice, StorageUnlockPin, StorageUnlockPassword] {
        assert!(
            allowed(Owner::Main, Owner::Onboarding, MainMenu, to_state),
            "{to_state:?}"
        );
    }
}

#[test]
fn owner_crossings_follow_the_matrix() {
    assert!(allowed(
        Owner::Settings,
        Owner::Signing,
        SettingsMenu,
        ScanQR
    ));
    assert!(!allowed(
        Owner::Stego,
        Owner::Signing,
        StegoImportPick,
        ScanQR
    ));
    assert!(!allowed(
        Owner::Main,
        Owner::Storage,
        SeedsMenu,
        SdWalletBackupFileList
    ));
    assert!(!allowed(
        Owner::Onboarding,
        Owner::Settings,
        StorageModeChoice,
        SettingsMenu
    ));
}

#[test]
fn seeds_reach_stego_only_through_the_backup_entry() {
    assert!(allowed(
        Owner::Seeds,
        Owner::Stego,
        BackupRecoveryMenu,
        StegoModeSelect
    ));
    assert!(!allowed(
        Owner::Seeds,
        Owner::Stego,
        SeedsMenu,
        StegoModeSelect
    ));
}

#[test]
fn add_wallet_bridges_are_exact() {
    assert!(allowed(
        Owner::Onboarding,
        Owner::Seeds,
        StorageSeedSourceChoice,
        AddWalletChoice
    ));
    assert!(allowed(
        Owner::Onboarding,
        Owner::Signing,
        StorageSeedSourceChoice,
        ScanQR
    ));
    assert!(allowed(
        Owner::Seeds,
        Owner::Onboarding,
        RestoreWord { word_idx: 11 },
        RestoreWord12Detected
    ));
    assert!(!allowed(
        Owner::Onboarding,
        Owner::Seeds,
        RestoreWord12Detected,
        RestoreWord { word_idx: 5 }
    ));
    assert!(!allowed(
        Owner::Onboarding,
        Owner::Seeds,
        StorageSeedSourceChoice,
        RestoreWord { word_idx: 1 }
    ));
    assert!(allowed(
        Owner::Seeds,
        Owner::Onboarding,
        WalletNameEntry { purpose: 3 },
        StorageFinalizeChoice
    ));
    assert!(allowed(
        Owner::Onboarding,
        Owner::Seeds,
        StorageFinalizeChoice,
        WalletNameEntry { purpose: 3 }
    ));
    assert!(allowed(
        Owner::Onboarding,
        Owner::Seeds,
        StorageFinalizeChoice,
        SeedList
    ));
    assert!(!allowed(
        Owner::Onboarding,
        Owner::Seeds,
        StorageFinalizeChoice,
        WalletNameEntry { purpose: 0 }
    ));
}

#[test]
fn wallet_protection_returns_to_wallet_details_only_from_its_screens() {
    assert!(allowed(
        Owner::Seeds,
        Owner::Onboarding,
        WalletDetails,
        StorageCredentialType
    ));
    assert!(allowed(
        Owner::Onboarding,
        Owner::Seeds,
        StoragePinConfirm,
        WalletDetails
    ));
    assert!(!allowed(
        Owner::Onboarding,
        Owner::Seeds,
        StorageModeChoice,
        WalletDetails
    ));
}

#[test]
fn sd_enable_keeps_recovery_screens_with_settings() {
    for state in [StorageRecoveryAcknowledgement, StorageSdFailure] {
        assert_eq!(
            owner_for_intent(Intent::EnableSd, state, Owner::Main),
            Owner::Settings
        );
    }
    assert_eq!(
        owner_for_intent(
            Intent::StartFresh,
            StorageRecoveryAcknowledgement,
            Owner::Main
        ),
        Owner::Onboarding
    );
}

#[test]
fn a_stale_onboarding_intent_cannot_steal_a_qr_result() {
    for owner in [
        Owner::Signing,
        Owner::Export,
        Owner::Storage,
        Owner::Multisig,
    ] {
        assert_eq!(owner_for_intent(Intent::StartFresh, ShowQR, owner), owner);
    }
    assert_eq!(
        owner_for_intent(Intent::StartFresh, ShowQR, Owner::Settings),
        Owner::Signing
    );
    assert_eq!(
        owner_for_intent(Intent::None, ShowQR, Owner::Onboarding),
        Owner::Signing
    );
}

#[test]
fn fixed_screens_have_one_owner() {
    assert_eq!(
        owner_for_intent(Intent::None, MainMenu, Owner::Settings),
        Owner::Main
    );
    assert_eq!(
        owner_for_intent(Intent::None, StorageUnlockPin, Owner::Main),
        Owner::Storage
    );
    assert_eq!(
        owner_for_intent(Intent::None, SeedList, Owner::Main),
        Owner::Seeds
    );
    assert_eq!(
        owner_for_intent(Intent::None, MultisigMenu, Owner::Main),
        Owner::Multisig
    );
    assert_eq!(
        owner_for_intent(Intent::None, SettingsMenu, Owner::Main),
        Owner::Settings
    );
    assert_eq!(
        owner_for_intent(Intent::None, ShowQrPopup, Owner::Export),
        Owner::Signing
    );
}

#[test]
fn sd_failure_stays_with_settings_only_when_settings_started_it() {
    assert_eq!(
        owner_for_intent(Intent::None, StorageSdFailure, Owner::Settings),
        Owner::Settings
    );
    assert_ne!(
        owner_for_intent(Intent::None, StorageSdFailure, Owner::Main),
        Owner::Settings
    );
}

#[test]
fn recovery_lands_on_each_owners_safe_screen() {
    let expected = [
        (Owner::Onboarding, StorageModeChoice, Owner::Onboarding),
        (Owner::Settings, SettingsMenu, Owner::Settings),
        (Owner::Seeds, SeedsMenu, Owner::Seeds),
        (Owner::Multisig, MultisigMenu, Owner::Multisig),
        (Owner::Signing, MainMenu, Owner::Main),
        (Owner::Export, MainMenu, Owner::Main),
        (Owner::Storage, MainMenu, Owner::Main),
        (Owner::Stego, MainMenu, Owner::Main),
        (Owner::Main, MainMenu, Owner::Main),
    ];
    for (owner, screen, next_owner) in expected {
        assert_eq!(safe_recovery(owner), screen);
        assert_eq!(recovery_owner(owner), next_owner);
    }
}

#[test]
fn secret_entry_screens_own_their_tap_feedback() {
    for state in [
        AppState::ChooseWordCount { action: 0 },
        AppState::StorageSeedWordCountChoice { action: 0 },
        AppState::SeedEntropyUnavailable { word_count: 12 },
        AppState::StoragePinEntry,
        AppState::StoragePinConfirm,
        AppState::StoragePasswordEntry,
        AppState::StoragePasswordConfirm,
        AppState::StorageUnlockPin,
        AppState::StorageUnlockPassword,
    ] {
        assert!(!navigation::tap_uses_router_click(state), "{state:?}");
    }
    assert!(navigation::tap_uses_router_click(AppState::MainMenu));
    assert!(navigation::tap_uses_router_click(AppState::SettingsMenu));
}

#[test]
fn home_shortcut_needs_home_reached_and_never_escapes_unlock() {
    assert!(navigation::home_shortcut_allowed(
        true,
        AppState::SettingsMenu
    ));
    assert!(!navigation::home_shortcut_allowed(
        false,
        AppState::SettingsMenu
    ));
    assert!(!navigation::home_shortcut_allowed(true, AppState::MainMenu));
    assert!(!navigation::home_shortcut_allowed(
        true,
        AppState::StorageUnlockPin
    ));
    assert!(!navigation::home_shortcut_allowed(
        true,
        AppState::StorageUnlockPassword
    ));
}
