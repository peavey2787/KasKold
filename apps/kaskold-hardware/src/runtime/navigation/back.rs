//! Centralized hierarchy/menu Back reducer support.
//!
//! Stage 2 separates domain cleanup (`prepare`) from route selection (`target`).
//! The navigation kernel owns the actual Back commit and uses bounded history
//! whenever a shared screen has a caller-dependent return route.

use crate::runtime::{data::AppData, input::AppState};

pub(super) fn prepare(ad: &mut AppData) {
    use AppState::*;
    match ad.navigation.app.state {
        SeedList => ad.wallet.seeds.seed_list_scroll = 0,
        ConfirmDeleteSeed => ad.wallet.seeds.pending_delete_slot = 0xFF,
        MultisigChooseMN => ad.signing.multisig.creating.n = 0,
        MultisigAddKey { key_idx: 0 } => ad.signing.multisig.creating.n = 0,
        PassphraseEntry => ad.wallet.seeds.pp_input.reset(),
        _ => {}
    }
}

pub(super) fn target(ad: &AppData) -> Option<AppState> {
    let target = seed_or_multisig_target(ad).or_else(|| production_target(ad)).or_else(|| platform_target(ad));
    #[cfg(feature = "workflow-tests")]
    let target = target.or_else(|| workflow_tests_target(ad));
    target
}

fn seed_or_multisig_target(ad: &AppData) -> Option<AppState> {
    use AppState::*;
    Some(match ad.navigation.app.state {
        SeedList => SeedsMenu,
        ConfirmDeleteSeed => SeedList,
        MultisigChooseMN => MultisigMenu,
        MultisigAddKey { key_idx: 0 } => MultisigChooseMN,
        MultisigAddKey { key_idx } => MultisigAddKey { key_idx: key_idx - 1 },
        MultisigPickSeed { key_idx } => MultisigAddKey { key_idx },
        MultisigDescriptorList { .. } => MultisigDescriptorsMenu,
        MultisigSaveDescriptorAsk => MultisigShowAddress,
        MultisigDescriptor if ad.signing.multisig.draft == crate::runtime::data::MultisigDraft::Kept => {
            MultisigShowAddress
        }
        MultisigDescriptor => MultisigDescriptorsMenu,
        _ => return None,
    })
}

fn production_target(ad: &AppData) -> Option<AppState> {
    use AppState::*;
    Some(match ad.navigation.app.state {
        DisplaySettings => SettingsMenu,
        WalletBackupMethodsMenu | WalletDetails => SeedsMenu,
        AdvancedMenu => SettingsMenu,
        FirmwareUpdateReady | FactoryResetWarning | FactoryResetConfirm => AdvancedMenu,
        SettingsMenu | SeedsMenu => MainMenu,
        // Caller-dependent shared screens intentionally fall through to the
        // bounded navigation history: WalletAdvancedMenu, BackupRecoveryMenu,
        // MultisigMenu, SdCardSettings, SdCardUnlockPassword, AdvancedFeatures, About, SdImportMenu.
        _ => return None,
    })
}

fn platform_target(ad: &AppData) -> Option<AppState> {
    use AppState::*;
    Some(match ad.navigation.app.state {
        #[cfg(feature = "m5stack")]
        AudioSettings => SettingsMenu,
        PassphraseEntry => PassphraseChoice,
        _ => return None,
    })
}

#[cfg(feature = "workflow-tests")]
fn workflow_tests_target(ad: &AppData) -> Option<AppState> {
    use AppState::*;
    Some(match ad.navigation.app.state {
        WorkflowTestsMenu => ad.navigation.workflow_tests_return.state(),
        WorkflowTestsCategory { .. } => WorkflowTestsMenu,
        WorkflowTestsResult if ad.workflow_tests.result.ran_all => WorkflowTestsMenu,
        WorkflowTestsResult => WorkflowTestsCategory { category: ad.workflow_tests.selected_category },
        _ => return None,
    })
}
