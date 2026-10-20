//! Explicit first-wallet onboarding transition matrix.

use super::AppState;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnboardingRoute {
    Persistence,
    Generation,
    Dice,
    SeedEntry,
    RecoveryWords,
    Scan,
    Sd,
    Stego,
}

/// The onboarding sub-flow that owns `state`. Fixed persistence screens are
/// always onboarding-owned; the rest only while a new seed is being set up.
pub fn route_for(seed_onboarding: bool, state: AppState) -> Option<OnboardingRoute> {
    if is_fixed_persistence(state) {
        return Some(OnboardingRoute::Persistence);
    }
    if !seed_onboarding {
        return None;
    }
    generation_route(state).or_else(|| restore_route(state))
}

/// Persistence screens owned by onboarding whether or not a seed is being set up.
fn is_fixed_persistence(state: AppState) -> bool {
    use AppState::*;
    matches!(
        state,
        StorageModeChoice
            | StorageSeedSourceChoice
            | AdvancedRestoreMenu
            | RestoreWord12Detected
            | StorageFinalizeChoice
            | StorageProtectionChoice
            | StorageCredentialType
            | StoragePinEntry
            | StoragePinConfirm
            | StoragePasswordEntry
            | StoragePasswordConfirm
            | StorageSeedWordCountChoice { .. }
    )
}

/// Sub-flow of a seed-generation screen: entropy collection, the recovery
/// words it produced, and their acknowledgement.
fn generation_route(state: AppState) -> Option<OnboardingRoute> {
    use AppState::*;
    use OnboardingRoute::*;
    match state {
        StorageRecoveryAcknowledgement => Some(Persistence),
        SeedEntropyUnavailable { .. }
        | StorageSeedDiceChoice
        | StorageSeedDiceCountChoice
        | StorageSeedTouchChoice
        | TouchEntropy => Some(Generation),
        DiceRoll => Some(Dice),
        SeedBackup { .. } => Some(RecoveryWords),
        _ => None,
    }
}

/// Sub-flow of a seed-restore screen: typed entry or an import transport.
fn restore_route(state: AppState) -> Option<OnboardingRoute> {
    use AppState::*;
    use OnboardingRoute::*;
    match state {
        RestoreWord { .. }
        | ImportWord { .. }
        | PassphraseChoice
        | PassphraseEntry
        | ImportPrivKey
        | WalletNameEntry { purpose: 0 | 3 } => Some(SeedEntry),
        ScanQR => Some(Scan),
        SdWalletBackupFileList | SdWalletBackupImportPassphrase => Some(Sd),
        StegoImportPick
        | StegoImportDescChoice
        | StegoImportDescFile
        | StegoImportPass
        | StegoImportPortablePassword
        | StegoHintReveal
        | StegoHintPassphrase => Some(Stego),
        _ => None,
    }
}

pub fn owns_state(seed_onboarding: bool, state: AppState) -> bool {
    route_for(seed_onboarding, state).is_some()
}

pub fn transition_allowed(from: AppState, to: AppState) -> bool {
    from == to
        || source_transition_allowed(from, to)
        || entropy_transition_allowed(from, to)
        || seed_entry_transition_allowed(from, to)
        || restore_transport_transition_allowed(from, to)
        || recovery_transition_allowed(from, to)
        || credential_transition_allowed(from, to)
}

fn source_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        StorageModeChoice => matches!(to, StorageSeedSourceChoice | WalletNameEntry { purpose: 0 }),
        WalletNameEntry { purpose: 0 } => matches!(
            to,
            StorageModeChoice | StorageSeedWordCountChoice { action: 0 }
        ),
        WalletNameEntry { purpose: 3 } => matches!(to, PassphraseChoice | StorageFinalizeChoice),
        StorageSeedSourceChoice => matches!(
            to,
            StorageModeChoice
                | RestoreWord { word_idx: 0 }
                | ScanQR
                | SdWalletBackupFileList
                | AdvancedRestoreMenu
        ),
        AdvancedRestoreMenu => matches!(
            to,
            StorageSeedSourceChoice | ScanQR | ImportPrivKey | StegoImportPick
        ),
        RestoreWord12Detected => matches!(to, RestoreWord { word_idx: 11 | 12 } | PassphraseChoice),
        _ => false,
    }
}

fn entropy_transition_allowed(from: AppState, to: AppState) -> bool {
    word_count_transition_allowed(from, to)
        || dice_transition_allowed(from, to)
        || touch_transition_allowed(from, to)
}

fn word_count_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        StorageSeedWordCountChoice { action: 0 } => matches!(
            to,
            StorageModeChoice
                | StorageSeedDiceChoice
                | SeedEntropyUnavailable {
                    word_count: 12 | 24
                }
        ),
        SeedEntropyUnavailable { word_count } => entropy_retry_allowed(word_count, to),
        StorageSeedWordCountChoice { .. } => matches!(to, StorageModeChoice),
        _ => false,
    }
}

/// Exits from the "entropy unavailable" screen: back to the word count, on to
/// dice, or a retry for the same word count.
fn entropy_retry_allowed(word_count: u8, to: AppState) -> bool {
    use AppState::*;
    match to {
        StorageSeedWordCountChoice { action: 0 } | StorageSeedDiceChoice => true,
        SeedEntropyUnavailable {
            word_count: retry_count,
        } => retry_count == word_count,
        _ => false,
    }
}

fn dice_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        StorageSeedDiceChoice => matches!(
            to,
            StorageSeedWordCountChoice { action: 0 }
                | StorageSeedDiceCountChoice
                | StorageSeedTouchChoice
        ),
        StorageSeedDiceCountChoice => {
            matches!(to, StorageSeedDiceChoice | StorageModeChoice | DiceRoll)
        }
        DiceRoll => matches!(
            to,
            StorageSeedDiceCountChoice | StorageSeedTouchChoice | StorageModeChoice
        ),
        _ => false,
    }
}

fn touch_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        StorageSeedTouchChoice => matches!(
            to,
            StorageSeedWordCountChoice { action: 0 }
                | StorageModeChoice
                | TouchEntropy
                | PassphraseChoice
        ),
        TouchEntropy => matches!(
            to,
            StorageSeedTouchChoice
                | StorageSeedWordCountChoice { action: 0 }
                | StorageModeChoice
                | PassphraseChoice
        ),
        _ => false,
    }
}

fn seed_entry_transition_allowed(from: AppState, to: AppState) -> bool {
    restore_word_transition_allowed(from, to)
        || imported_word_transition_allowed(from, to)
        || passphrase_transition_allowed(from, to)
}

fn restore_word_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        RestoreWord { word_idx } => match to {
            RestoreWord { word_idx: next } => {
                next == word_idx.saturating_add(1) || next.saturating_add(1) == word_idx
            }
            RestoreWord12Detected if word_idx == 11 => true,
            StorageSeedSourceChoice | PassphraseChoice => true,
            _ => false,
        },
        _ => false,
    }
}

fn imported_word_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        ImportWord {
            word_idx: from_idx,
            word_count: from_count,
        } => match to {
            ImportWord {
                word_idx: to_idx,
                word_count: to_count,
            } => from_count == to_count && to_idx == from_idx.saturating_add(1),
            StorageSeedSourceChoice | PassphraseChoice => true,
            _ => false,
        },
        ImportPrivKey => matches!(
            to,
            AdvancedRestoreMenu | StorageRecoveryAcknowledgement | StorageFinalizeChoice | MainMenu
        ),
        _ => false,
    }
}

fn passphrase_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        PassphraseChoice => matches!(
            to,
            StorageSeedSourceChoice
                | StorageSeedWordCountChoice { action: 0 }
                | StorageSeedTouchChoice
                | PassphraseEntry
                | SeedBackup { word_idx: 0 }
                | StorageRecoveryAcknowledgement
                | StorageFinalizeChoice
                | WalletNameEntry { purpose: 3 }
        ),
        PassphraseEntry => matches!(
            to,
            PassphraseChoice
                | StorageSeedSourceChoice
                | StorageSeedWordCountChoice { action: 0 }
                | SeedBackup { word_idx: 0 }
                | StorageRecoveryAcknowledgement
                | StorageFinalizeChoice
                | WalletNameEntry { purpose: 3 }
        ),
        _ => false,
    }
}

fn restore_transport_transition_allowed(from: AppState, to: AppState) -> bool {
    scan_or_sd_transition_allowed(from, to)
        || stego_import_transition_allowed(from, to)
        || stego_unlock_transition_allowed(from, to)
}

fn scan_or_sd_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        ScanQR => matches!(
            to,
            StorageSeedSourceChoice | AdvancedRestoreMenu | PassphraseChoice
        ),
        SdWalletBackupFileList => {
            matches!(to, StorageSeedSourceChoice | SdWalletBackupImportPassphrase)
        }
        SdWalletBackupImportPassphrase => matches!(to, SdWalletBackupFileList | PassphraseChoice),
        _ => false,
    }
}

/// Picking the JPEG and its descriptor source.
fn stego_import_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        StegoImportPick => matches!(to, AdvancedRestoreMenu | StegoImportDescChoice),
        StegoImportDescChoice => {
            matches!(to, StegoImportPick | StegoImportDescFile | StegoImportPass)
        }
        StegoImportDescFile => matches!(to, StegoImportDescChoice | StegoImportPass),
        _ => false,
    }
}

/// Unlocking the JPEG payload: passphrase, portable password or hint.
fn stego_unlock_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        StegoImportPass => matches!(
            to,
            StegoImportDescChoice
                | StegoImportPortablePassword
                | StegoHintReveal
                | PassphraseChoice
                | StorageRecoveryAcknowledgement
        ),
        StegoImportPortablePassword => matches!(
            to,
            StegoImportDescChoice
                | StegoHintReveal
                | PassphraseChoice
                | StorageRecoveryAcknowledgement
        ),
        StegoHintReveal => matches!(
            to,
            StegoHintPassphrase | PassphraseChoice | StorageRecoveryAcknowledgement
        ),
        StegoHintPassphrase => matches!(to, PassphraseChoice | StorageRecoveryAcknowledgement),
        _ => false,
    }
}

fn recovery_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        SeedBackup { word_idx } => seed_backup_transition_allowed(word_idx, to),
        StorageRecoveryAcknowledgement => matches!(
            to,
            SeedBackup { .. }
                | StorageSeedSourceChoice
                | StorageFinalizeChoice
                | StorageProtectionChoice
        ),
        StorageFinalizeChoice => matches!(
            to,
            StorageRecoveryAcknowledgement
                | PassphraseChoice
                | WalletNameEntry { purpose: 3 }
                | AdvancedRestoreMenu
                | StorageProtectionChoice
                | MainMenu
        ),
        StorageProtectionChoice => {
            matches!(to, StorageFinalizeChoice | StorageCredentialType | MainMenu)
        }
        _ => false,
    }
}

/// Exits from recovery word `from_idx`: an adjacent word, back out from the
/// first word, or on to the acknowledgement.
fn seed_backup_transition_allowed(from_idx: u8, to: AppState) -> bool {
    use AppState::*;
    match to {
        SeedBackup { word_idx: to_idx } => {
            to_idx == from_idx.saturating_add(1) || from_idx == to_idx.saturating_add(1)
        }
        PassphraseChoice | StorageModeChoice => from_idx == 0,
        StorageRecoveryAcknowledgement => true,
        _ => false,
    }
}

fn credential_transition_allowed(from: AppState, to: AppState) -> bool {
    use AppState::*;
    match from {
        StorageCredentialType => matches!(
            to,
            StorageProtectionChoice | StoragePinEntry | StoragePasswordEntry
        ),
        StoragePinEntry => matches!(to, StorageCredentialType | StoragePinConfirm),
        StoragePinConfirm => matches!(to, StoragePinEntry | MainMenu),
        StoragePasswordEntry => matches!(to, StorageCredentialType | StoragePasswordConfirm),
        StoragePasswordConfirm => matches!(to, StoragePasswordEntry | MainMenu),
        _ => false,
    }
}
