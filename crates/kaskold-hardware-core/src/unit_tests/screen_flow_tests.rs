use crate::screen_flow::onboarding::{owns_state, route_for, transition_allowed, OnboardingRoute};
use crate::screen_flow::{is_scan_state, AppState, HandlerGroup};
use std::collections::BTreeSet;
use std::vec::Vec;
use AppState::*;

const MAX_WORDS: u8 = 24;

/// Every onboarding screen plus the neighbours a bad matrix entry could leak
/// into, with each parameterised screen expanded over its realistic range.
fn universe() -> Vec<AppState> {
    let mut states = std::vec![
        StorageModeChoice,
        StorageSeedSourceChoice,
        StorageSeedDiceChoice,
        StorageSeedDiceCountChoice,
        StorageSeedTouchChoice,
        AdvancedRestoreMenu,
        RestoreWord12Detected,
        StorageRecoveryAcknowledgement,
        StorageFinalizeChoice,
        StorageProtectionChoice,
        StorageCredentialType,
        StoragePinEntry,
        StoragePinConfirm,
        StoragePasswordEntry,
        StoragePasswordConfirm,
        StorageUnlockPin,
        StorageUnlockPassword,
        StorageSdFailure,
        MainMenu,
        ScanQR,
        SeedsMenu,
        SettingsMenu,
        SeedList,
        AddWalletChoice,
        DiceRoll,
        TouchEntropy,
        PassphraseChoice,
        PassphraseEntry,
        ImportPrivKey,
        ExportPrivKey,
        SdWalletBackupFileList,
        SdWalletBackupImportPassphrase,
        SdFileList,
        StegoImportPick,
        StegoImportDescChoice,
        StegoImportDescFile,
        StegoImportPass,
        StegoImportPortablePassword,
        StegoHintReveal,
        StegoHintPassphrase,
        StegoModeSelect,
        ChooseWordCount { action: 0 },
        ShowQR,
    ];
    for small in 0..4 {
        states.push(StorageSeedWordCountChoice { action: small });
        states.push(WalletNameEntry { purpose: small });
    }
    for word_count in [12, 24] {
        states.push(SeedEntropyUnavailable { word_count });
        states.extend((0..word_count).map(|word_idx| ImportWord {
            word_idx,
            word_count,
        }));
    }
    for word_idx in 0..=MAX_WORDS {
        states.push(RestoreWord { word_idx });
        states.push(SeedBackup { word_idx });
    }
    states
}

fn reachable_from(start: AppState) -> Vec<AppState> {
    let universe = universe();
    let mut seen = std::vec![start];
    let mut frontier = std::vec![start];
    while let Some(from) = frontier.pop() {
        for &to in &universe {
            if !seen.contains(&to) && transition_allowed(from, to) {
                seen.push(to);
                frontier.push(to);
            }
        }
    }
    seen
}

#[test]
fn persistence_screens_are_onboarding_owned_whatever_the_intent() {
    for state in [
        StorageModeChoice,
        StorageSeedSourceChoice,
        AdvancedRestoreMenu,
        RestoreWord12Detected,
        StorageFinalizeChoice,
        StorageProtectionChoice,
        StorageCredentialType,
        StoragePinEntry,
        StoragePinConfirm,
        StoragePasswordEntry,
        StoragePasswordConfirm,
        StorageSeedWordCountChoice { action: 2 },
    ] {
        for seed_onboarding in [false, true] {
            assert_eq!(
                route_for(seed_onboarding, state),
                Some(OnboardingRoute::Persistence),
                "{state:?}"
            );
        }
    }
}

#[test]
fn seed_screens_are_onboarding_owned_only_while_a_seed_is_set_up() {
    let seed_only = [
        (StorageRecoveryAcknowledgement, OnboardingRoute::Persistence),
        (
            SeedEntropyUnavailable { word_count: 24 },
            OnboardingRoute::Generation,
        ),
        (StorageSeedDiceChoice, OnboardingRoute::Generation),
        (TouchEntropy, OnboardingRoute::Generation),
        (DiceRoll, OnboardingRoute::Dice),
        (RestoreWord { word_idx: 3 }, OnboardingRoute::SeedEntry),
        (
            ImportWord {
                word_idx: 1,
                word_count: 12,
            },
            OnboardingRoute::SeedEntry,
        ),
        (WalletNameEntry { purpose: 3 }, OnboardingRoute::SeedEntry),
        (SeedBackup { word_idx: 5 }, OnboardingRoute::RecoveryWords),
        (ScanQR, OnboardingRoute::Scan),
        (SdWalletBackupImportPassphrase, OnboardingRoute::Sd),
        (StegoHintPassphrase, OnboardingRoute::Stego),
    ];
    for (state, route) in seed_only {
        assert_eq!(route_for(true, state), Some(route), "{state:?}");
        assert_eq!(route_for(false, state), None, "{state:?}");
    }
    for state in [
        MainMenu,
        StorageUnlockPin,
        StorageSdFailure,
        WalletNameEntry { purpose: 1 },
        SettingsMenu,
    ] {
        assert!(!owns_state(true, state), "{state:?}");
    }
}

#[test]
fn onboarding_can_only_leave_into_the_main_menu() {
    let reached = reachable_from(StorageModeChoice);
    let escaped: Vec<_> = reached
        .iter()
        .filter(|state| **state != MainMenu && !owns_state(true, **state))
        .collect();
    assert!(escaped.is_empty(), "onboarding leaked into {escaped:?}");
    for required in [
        MainMenu,
        StoragePinConfirm,
        StoragePasswordConfirm,
        RestoreWord12Detected,
        RestoreWord { word_idx: 23 },
        SeedBackup {
            word_idx: MAX_WORDS,
        },
        DiceRoll,
        TouchEntropy,
        StegoHintPassphrase,
        SdWalletBackupImportPassphrase,
        ImportPrivKey,
        SeedEntropyUnavailable { word_count: 12 },
        WalletNameEntry { purpose: 3 },
    ] {
        assert!(
            reached.contains(&required),
            "{required:?} unreachable from welcome"
        );
    }
}

#[test]
fn only_completion_screens_may_commit_the_main_menu() {
    let completing: BTreeSet<_> = universe()
        .into_iter()
        .filter(|from| *from != MainMenu && transition_allowed(*from, MainMenu))
        .map(|from| std::format!("{from:?}"))
        .collect();
    let expected: BTreeSet<_> = [
        "ImportPrivKey",
        "StorageFinalizeChoice",
        "StoragePasswordConfirm",
        "StoragePinConfirm",
        "StorageProtectionChoice",
    ]
    .into_iter()
    .map(std::string::String::from)
    .collect();
    assert_eq!(completing, expected);
}

#[test]
fn word_screens_step_one_word_at_a_time() {
    assert!(transition_allowed(
        RestoreWord { word_idx: 5 },
        RestoreWord { word_idx: 6 }
    ));
    assert!(transition_allowed(
        RestoreWord { word_idx: 5 },
        RestoreWord { word_idx: 4 }
    ));
    assert!(!transition_allowed(
        RestoreWord { word_idx: 5 },
        RestoreWord { word_idx: 7 }
    ));
    for word_idx in 0..MAX_WORDS {
        let detected = transition_allowed(RestoreWord { word_idx }, RestoreWord12Detected);
        assert_eq!(detected, word_idx == 11, "word {word_idx}");
    }
    let import = |word_idx, word_count| ImportWord {
        word_idx,
        word_count,
    };
    assert!(transition_allowed(import(3, 12), import(4, 12)));
    assert!(!transition_allowed(import(3, 12), import(2, 12)));
    assert!(!transition_allowed(import(3, 12), import(4, 24)));
    assert!(transition_allowed(
        SeedBackup { word_idx: 4 },
        SeedBackup { word_idx: 3 }
    ));
    assert!(!transition_allowed(
        SeedBackup { word_idx: 4 },
        SeedBackup { word_idx: 6 }
    ));
}

#[test]
fn recovery_words_only_back_out_from_the_first_word() {
    for word_idx in 0..=MAX_WORDS {
        let first = word_idx == 0;
        assert_eq!(
            transition_allowed(SeedBackup { word_idx }, PassphraseChoice),
            first
        );
        assert_eq!(
            transition_allowed(SeedBackup { word_idx }, StorageModeChoice),
            first
        );
        assert!(transition_allowed(
            SeedBackup { word_idx },
            StorageRecoveryAcknowledgement
        ));
    }
}

#[test]
fn entropy_is_gathered_before_the_passphrase_and_retries_keep_the_word_count() {
    let generate = StorageSeedWordCountChoice { action: 0 };
    let name_new_wallet = WalletNameEntry { purpose: 0 };
    assert!(transition_allowed(StorageModeChoice, name_new_wallet));
    assert!(transition_allowed(name_new_wallet, generate));
    assert!(!transition_allowed(name_new_wallet, StorageSeedDiceChoice));
    assert!(!transition_allowed(generate, PassphraseChoice));
    assert!(transition_allowed(generate, StorageSeedDiceChoice));
    assert!(transition_allowed(
        StorageSeedDiceChoice,
        StorageSeedTouchChoice
    ));
    assert!(transition_allowed(StorageSeedTouchChoice, PassphraseChoice));
    assert!(!transition_allowed(StorageSeedDiceChoice, PassphraseChoice));
    let unavailable = |word_count| SeedEntropyUnavailable { word_count };
    assert!(transition_allowed(unavailable(12), unavailable(12)));
    assert!(!transition_allowed(unavailable(12), unavailable(24)));
    assert!(transition_allowed(generate, unavailable(24)));
    assert!(!transition_allowed(generate, unavailable(18)));
    assert!(!transition_allowed(
        StorageSeedWordCountChoice { action: 1 },
        StorageSeedDiceChoice
    ));
}

#[test]
fn every_screen_family_has_one_touch_owner() {
    let owners = [
        (MainMenu, HandlerGroup::Menu),
        (StorageSeedWordCountChoice { action: 0 }, HandlerGroup::Menu),
        (
            SeedEntropyUnavailable { word_count: 12 },
            HandlerGroup::Menu,
        ),
        (StegoImportPick, HandlerGroup::Stego),
        (SdWalletBackupFileList, HandlerGroup::Sd),
        (RestoreWord { word_idx: 0 }, HandlerGroup::Seed),
        (SeedBackup { word_idx: 0 }, HandlerGroup::Export),
        (SettingsMenu, HandlerGroup::Settings),
        (ScanQR, HandlerGroup::Tx),
        (StorageUnlockPin, HandlerGroup::Persistence),
    ];
    for (state, group) in owners {
        assert_eq!(state.handler_group(), group, "{state:?}");
    }
}

#[cfg(all(feature = "m5stack", feature = "workflow-tests"))]
#[test]
fn feature_gated_screens_have_touch_owners() {
    let owners = [
        (OwnerFirmwareMenu, HandlerGroup::Menu),
        (PopItPrompt, HandlerGroup::Settings),
        (DeveloperMenu, HandlerGroup::Menu),
        (DiagnosticInfo, HandlerGroup::Settings),
        (
            WorkflowTestsCategory { category: 1 },
            HandlerGroup::WorkflowTests,
        ),
        (AudioSettings, HandlerGroup::Settings),
        (AdvancedWeeklyEntry, HandlerGroup::Settings),
    ];
    for (state, group) in owners {
        assert_eq!(state.handler_group(), group, "{state:?}");
    }
}

#[test]
fn only_scanner_facing_screens_present_or_read_qr_codes() {
    assert!(ShowQR.shows_scannable_qr());
    assert!(ExportXprv.shows_scannable_qr());
    assert!(!SeedQrGrid {
        pan_x: 0,
        pan_y: 0,
        compact: false
    }
    .shows_scannable_qr());
    assert!(!QrExportMenu.shows_scannable_qr());
    for state in [ScanQR, DecryptSecretScan, SignMsgScan] {
        assert!(is_scan_state(state), "{state:?}");
    }
    assert!(!is_scan_state(ShowQR));
}
