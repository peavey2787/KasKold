use crate::creation_flow::{
    after_credential_entry, after_credential_type, after_dice_choice, after_dice_rolls,
    after_dice_target, after_passphrase_choice, after_passphrase_entry, after_protection_choice,
    after_recovery_acknowledgement, after_seed_backup, after_storage_choice, after_touch_choice,
    after_touch_entropy, after_wallet_name, after_word_count, first_stage, valid_dice_target,
    CreationStage, DICE_ROLL_TARGETS, TOUCH_ENTROPY_TARGET,
};

#[test]
fn creation_flow_matches_hardware_onboarding_order() {
    assert_eq!(first_stage(), CreationStage::WalletName);
    assert_eq!(after_wallet_name(), CreationStage::WordCount);
    assert_eq!(after_word_count(), CreationStage::DiceChoice);
    assert_eq!(after_dice_choice(false), CreationStage::TouchChoice);
    assert_eq!(after_dice_choice(true), CreationStage::DiceCount);
    assert_eq!(after_dice_target(), CreationStage::DiceRoll);
    assert_eq!(after_dice_rolls(), CreationStage::TouchChoice);
    assert_eq!(after_touch_choice(false), CreationStage::PassphraseChoice);
    assert_eq!(after_touch_choice(true), CreationStage::TouchEntropy);
    assert_eq!(after_touch_entropy(), CreationStage::PassphraseChoice);
    assert_eq!(after_passphrase_choice(false), CreationStage::SeedBackup);
    assert_eq!(
        after_passphrase_choice(true),
        CreationStage::PassphraseEntry
    );
    assert_eq!(after_passphrase_entry(), CreationStage::SeedBackup);
    assert_eq!(after_seed_backup(), CreationStage::RecoveryAcknowledgement);
    assert_eq!(
        after_recovery_acknowledgement(),
        CreationStage::StorageFinalize
    );
    assert_eq!(after_storage_choice(false), CreationStage::Complete);
    assert_eq!(after_storage_choice(true), CreationStage::StorageProtection);
    assert_eq!(after_protection_choice(false), CreationStage::Complete);
    assert_eq!(after_protection_choice(true), CreationStage::CredentialType);
    assert_eq!(after_credential_type(), CreationStage::CredentialEntry);
    assert_eq!(after_credential_entry(), CreationStage::Complete);
}

#[test]
fn creation_entropy_options_match_hardware_policy() {
    assert_eq!(DICE_ROLL_TARGETS, [25, 50, 100, 200]);
    assert_eq!(TOUCH_ENTROPY_TARGET, 2_048);
    for target in DICE_ROLL_TARGETS {
        assert!(valid_dice_target(target));
    }
    assert!(!valid_dice_target(0));
    assert!(!valid_dice_target(24));
    assert!(!valid_dice_target(201));
}
