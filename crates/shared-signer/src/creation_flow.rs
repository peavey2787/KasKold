//! Cross-platform wallet-creation flow policy shared by hardware and software Vaults.
//!
//! Presentation shells own rendering and platform-specific persistence details, but
//! the sequence and optional-entropy policy live here so M5, Web, Android, and iOS
//! stay aligned.

pub const DICE_ROLL_TARGETS: [usize; 4] = [25, 50, 100, 200];
pub const TOUCH_ENTROPY_TARGET: usize = 2_048;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum CreationStage {
    WalletName = 0,
    WordCount = 1,
    DiceChoice = 2,
    DiceCount = 3,
    DiceRoll = 4,
    TouchChoice = 5,
    TouchEntropy = 6,
    PassphraseChoice = 7,
    PassphraseEntry = 8,
    SeedBackup = 9,
    RecoveryAcknowledgement = 10,
    StorageFinalize = 11,
    StorageProtection = 12,
    CredentialType = 13,
    CredentialEntry = 14,
    Complete = 15,
}

pub const CREATION_STAGE_ORDER: [CreationStage; 16] = [
    CreationStage::WalletName,
    CreationStage::WordCount,
    CreationStage::DiceChoice,
    CreationStage::DiceCount,
    CreationStage::DiceRoll,
    CreationStage::TouchChoice,
    CreationStage::TouchEntropy,
    CreationStage::PassphraseChoice,
    CreationStage::PassphraseEntry,
    CreationStage::SeedBackup,
    CreationStage::RecoveryAcknowledgement,
    CreationStage::StorageFinalize,
    CreationStage::StorageProtection,
    CreationStage::CredentialType,
    CreationStage::CredentialEntry,
    CreationStage::Complete,
];

const CREATION_STAGE_NAMES: [&str; 16] = [
    "WalletNameEntry",
    "StorageSeedWordCountChoice",
    "StorageSeedDiceChoice",
    "StorageSeedDiceCountChoice",
    "DiceRoll",
    "StorageSeedTouchChoice",
    "TouchEntropy",
    "PassphraseChoice",
    "PassphraseEntry",
    "SeedBackup",
    "StorageRecoveryAcknowledgement",
    "StorageFinalizeChoice",
    "StorageProtectionChoice",
    "StorageCredentialType",
    "StorageCredentialEntry",
    "Complete",
];

impl CreationStage {
    pub const fn name(self) -> &'static str {
        CREATION_STAGE_NAMES[self as usize]
    }
}

pub const fn first_stage() -> CreationStage {
    CreationStage::WalletName
}

pub const fn after_wallet_name() -> CreationStage {
    CreationStage::WordCount
}

pub const fn after_word_count() -> CreationStage {
    CreationStage::DiceChoice
}

pub const fn after_dice_choice(add_dice: bool) -> CreationStage {
    if add_dice {
        CreationStage::DiceCount
    } else {
        CreationStage::TouchChoice
    }
}

pub const fn after_dice_target() -> CreationStage {
    CreationStage::DiceRoll
}

pub const fn after_dice_rolls() -> CreationStage {
    CreationStage::TouchChoice
}

pub const fn after_touch_choice(add_touch: bool) -> CreationStage {
    if add_touch {
        CreationStage::TouchEntropy
    } else {
        CreationStage::PassphraseChoice
    }
}

pub const fn after_touch_entropy() -> CreationStage {
    CreationStage::PassphraseChoice
}

pub const fn after_passphrase_choice(use_passphrase: bool) -> CreationStage {
    if use_passphrase {
        CreationStage::PassphraseEntry
    } else {
        CreationStage::SeedBackup
    }
}

pub const fn after_passphrase_entry() -> CreationStage {
    CreationStage::SeedBackup
}

pub const fn after_seed_backup() -> CreationStage {
    CreationStage::RecoveryAcknowledgement
}

pub const fn after_recovery_acknowledgement() -> CreationStage {
    CreationStage::StorageFinalize
}

pub const fn after_storage_choice(save: bool) -> CreationStage {
    if save {
        CreationStage::StorageProtection
    } else {
        CreationStage::Complete
    }
}

pub const fn after_protection_choice(protect: bool) -> CreationStage {
    if protect {
        CreationStage::CredentialType
    } else {
        CreationStage::Complete
    }
}

pub const fn after_credential_type() -> CreationStage {
    CreationStage::CredentialEntry
}

pub const fn after_credential_entry() -> CreationStage {
    CreationStage::Complete
}

pub fn valid_dice_target(target: usize) -> bool {
    DICE_ROLL_TARGETS.contains(&target)
}
