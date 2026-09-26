use crate::{
    entropy::fill_random, mnemonic12_phrase, mnemonic24_phrase, CreatedWallet, HotWallet,
    HotWalletError, RecoveryMaterial, MAX_BIP39_PASSPHRASE_LEN,
};
use offline_signer::{derivation::bip39::Seed, transaction::model::MultisigStore, OfflineSigner};
use shared_signer::seed_entropy::{mix_additive_dice, mix_additive_touch_transcript};
use zeroize::{Zeroize, Zeroizing};

impl HotWallet {
    /// Create a 12-word BIP39 wallet from CSPRNG entropy generated inside Rust.
    pub fn create_12() -> Result<CreatedWallet, HotWalletError> {
        Self::create_mnemonic(12, &[], &[], "")
    }

    /// Create a 24-word BIP39 wallet from CSPRNG entropy generated inside Rust.
    pub fn create_24() -> Result<CreatedWallet, HotWalletError> {
        Self::create_mnemonic(24, &[], &[], "")
    }

    /// Create a BIP39 wallet from mandatory Rust CSPRNG entropy plus optional
    /// additive user entropy. Dice/touch can strengthen the pool but can never
    /// replace the independently collected CSPRNG entropy.
    pub fn create_with_additive_entropy(
        word_count: u8,
        dice_rolls: &[u8],
        touch_transcript: &[u8],
        passphrase: &str,
    ) -> Result<CreatedWallet, HotWalletError> {
        Self::create_mnemonic(word_count, dice_rolls, touch_transcript, passphrase)
    }

    fn create_mnemonic(
        word_count: u8,
        dice_rolls: &[u8],
        touch_transcript: &[u8],
        passphrase: &str,
    ) -> Result<CreatedWallet, HotWalletError> {
        validate_creation_request(word_count, passphrase)?;
        let mut pool = [0u8; 32];
        fill_random(&mut pool)?;
        if mix_optional_entropy(&mut pool, dice_rolls, touch_transcript).is_err() {
            pool.zeroize();
            return Err(HotWalletError::InvalidToolInput);
        }
        let result = create_from_pool(word_count, &pool, passphrase);
        pool.zeroize();
        result
    }
}

fn validate_creation_request(word_count: u8, passphrase: &str) -> Result<(), HotWalletError> {
    if !matches!(word_count, 12 | 24) {
        return Err(HotWalletError::InvalidMnemonicWordCount);
    }
    if passphrase.len() > MAX_BIP39_PASSPHRASE_LEN {
        return Err(HotWalletError::Bip39PassphraseTooLong);
    }
    Ok(())
}

fn mix_optional_entropy(
    pool: &mut [u8; 32],
    dice_rolls: &[u8],
    touch_transcript: &[u8],
) -> Result<(), HotWalletError> {
    if !dice_rolls.is_empty() && !mix_additive_dice(pool, dice_rolls) {
        return Err(HotWalletError::InvalidToolInput);
    }
    if !touch_transcript.is_empty() && !mix_additive_touch_transcript(pool, touch_transcript) {
        return Err(HotWalletError::InvalidToolInput);
    }
    Ok(())
}

fn create_from_pool(
    word_count: u8,
    pool: &[u8; 32],
    passphrase: &str,
) -> Result<CreatedWallet, HotWalletError> {
    match word_count {
        12 => create_12_from_pool(pool, passphrase),
        24 => create_24_from_pool(pool, passphrase),
        _ => Err(HotWalletError::InvalidMnemonicWordCount),
    }
}

fn create_12_from_pool(pool: &[u8; 32], passphrase: &str) -> Result<CreatedWallet, HotWalletError> {
    let signer = OfflineSigner::new();
    let mut source = [0u8; 16];
    source.copy_from_slice(&pool[..16]);
    let mut mnemonic = signer.generate_wallet_12(&source);
    source.zeroize();
    let phrase = mnemonic12_phrase(&mnemonic);
    let seed = signer.restore_wallet_12(&mnemonic, passphrase)?;
    let recovery = RecoveryMaterial::from_12(&mnemonic, passphrase)?;
    mnemonic.zeroize();
    Ok(created_wallet(seed, recovery, phrase))
}

fn create_24_from_pool(pool: &[u8; 32], passphrase: &str) -> Result<CreatedWallet, HotWalletError> {
    let signer = OfflineSigner::new();
    let mut mnemonic = signer.generate_wallet_24(pool);
    let phrase = mnemonic24_phrase(&mnemonic);
    let seed = signer.restore_wallet_24(&mnemonic, passphrase)?;
    let recovery = RecoveryMaterial::from_24(&mnemonic, passphrase)?;
    mnemonic.zeroize();
    Ok(created_wallet(seed, recovery, phrase))
}

fn created_wallet(seed: Seed, recovery: RecoveryMaterial, phrase: String) -> CreatedWallet {
    CreatedWallet {
        wallet: HotWallet {
            seed,
            account_key: None,
            account_parent_fingerprint: [0u8; 4],
            raw_key: None,
            recovery: Some(recovery),
            multisig_store: MultisigStore::new(),
        },
        recovery_phrase: Zeroizing::new(phrase),
    }
}
